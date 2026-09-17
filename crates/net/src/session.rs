//! Session runners. `Runner` drives a GGRS P2P session; `LocalRunner` steps the sim directly (solo).

use ggrs::{
    Config, DesyncDetection, GgrsError, GgrsEvent, GgrsRequest, InputStatus, P2PSession,
    PlayerType, PredictRepeatLast, SessionBuilder, SessionState,
};
use hg_sim::config::MAX_PLAYERS;
use hg_sim::{PlayerInput, State, World};
use std::time::Duration;
use thiserror::Error;

use crate::input::{NetInput, PeerSlot};
use crate::socket::{BridgeSocket, SharedQueues, SocketStats};

/// GGRS type configuration.
pub struct HgConfig;

impl Config for HgConfig {
    type Input = NetInput;
    type InputPredictor = PredictRepeatLast;
    type State = State;
    type Address = PeerSlot;
}

/// Tunables (docs/design-docs/netcode-rollback.md §GGRS 설정).
#[derive(Clone, Copy, Debug)]
pub struct NetSettings {
    /// Frames of input delay.
    pub input_delay: usize,
    /// Max frames predicted ahead.
    pub max_prediction: usize,
    /// Checksum exchange interval (frames).
    pub desync_interval: u32,
    /// Disconnect after this silence.
    pub disconnect_timeout: Duration,
    /// Emit interruption after this silence.
    pub disconnect_notify: Duration,
}

impl Default for NetSettings {
    fn default() -> Self {
        Self {
            input_delay: 2,
            max_prediction: 8,
            desync_interval: 30,
            disconnect_timeout: Duration::from_secs(3),
            disconnect_notify: Duration::from_secs(1),
        }
    }
}

/// Outcome of one `tick`.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TickStatus {
    /// Simulation advanced.
    Advanced = 0,
    /// Still synchronizing with peers.
    Waiting = 1,
    /// Skipped to let slower peers catch up.
    Skipped = 2,
}

/// Session notifications surfaced to the UI. `slot` is the remote player handle.
#[allow(missing_docs)] // field names are self-describing
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetEvent {
    /// Handshake progress with a peer.
    Synchronizing { slot: u8, count: u32, total: u32 },
    /// Peer synchronized.
    Synchronized { slot: u8 },
    /// Connection shaky.
    Interrupted { slot: u8 },
    /// Connection recovered.
    Resumed { slot: u8 },
    /// Peer dropped for good.
    Disconnected { slot: u8 },
    /// Checksums diverged at `frame`.
    Desync { frame: i32 },
}

/// Construction errors.
#[derive(Debug, Error)]
pub enum SessionError {
    /// Invalid player setup.
    #[error("invalid session setup: {0}")]
    Setup(String),
}

impl From<GgrsError> for SessionError {
    fn from(e: GgrsError) -> Self {
        Self::Setup(e.to_string())
    }
}

/// Simulation host shared by both runners: owns the world, state and last-seen checksum.
struct SimHost {
    world: World,
    state: Box<State>,
}

impl SimHost {
    fn new(num_players: u8, seed: u32, map_id: u8) -> Self {
        let world = World::new(map_id);
        let state = world.initial_state(num_players, seed);
        Self { world, state }
    }
}

/// Networked runner.
pub struct Runner {
    session: P2PSession<HgConfig>,
    queues: SharedQueues,
    host: SimHost,
    local_handle: usize,
    skip_frames: u32,
    events: Vec<NetEvent>,
    /// Frame → checksum of the latest save, for diagnostics/netsim.
    pub checksum_log: Vec<(i32, u64)>,
    record_checksums: bool,
}

impl Runner {
    /// Build a P2P session. Handles are `0..num_players`; `local_handle` is ours, every other
    /// handle `h` is reachable at `PeerSlot(h)`.
    ///
    /// # Errors
    /// Invalid handle/count combinations.
    pub fn new(
        num_players: u8,
        local_handle: u8,
        seed: u32,
        map_id: u8,
        settings: NetSettings,
    ) -> Result<Self, SessionError> {
        if num_players < 2 || usize::from(num_players) > MAX_PLAYERS || local_handle >= num_players
        {
            return Err(SessionError::Setup(format!(
                "players={num_players} local={local_handle}"
            )));
        }
        let (socket, queues) = BridgeSocket::new();
        let mut builder = SessionBuilder::<HgConfig>::new()
            .with_num_players(usize::from(num_players))?
            .with_fps(60)?
            .with_input_delay(settings.input_delay)
            .with_max_prediction_window(settings.max_prediction)
            .with_desync_detection_mode(DesyncDetection::On {
                interval: settings.desync_interval,
            })
            .with_disconnect_timeout(settings.disconnect_timeout)
            .with_disconnect_notify_delay(settings.disconnect_notify);
        for h in 0..num_players {
            let kind = if h == local_handle {
                PlayerType::Local
            } else {
                PlayerType::Remote(PeerSlot(h))
            };
            builder = builder.add_player(kind, usize::from(h))?;
        }
        let session = builder.start_p2p_session(socket)?;
        Ok(Self {
            session,
            queues,
            host: SimHost::new(num_players, seed, map_id),
            local_handle: usize::from(local_handle),
            skip_frames: 0,
            events: Vec::new(),
            checksum_log: Vec::new(),
            record_checksums: false,
        })
    }

    /// Keep a `(frame, checksum)` log of saves (tests/netsim).
    pub fn record_checksums(&mut self, on: bool) {
        self.record_checksums = on;
    }

    /// Feed a received datagram from peer `slot`.
    pub fn push_packet(&mut self, slot: u8, bytes: &[u8]) {
        self.queues
            .borrow_mut()
            .push_incoming(PeerSlot(slot), bytes);
    }

    /// Take datagrams to send.
    pub fn take_outbox(&mut self) -> Vec<(PeerSlot, Vec<u8>)> {
        std::mem::take(&mut self.queues.borrow_mut().outbox)
    }

    /// Advance the session by at most one frame.
    pub fn tick(&mut self, local: PlayerInput) -> TickStatus {
        self.session.poll_remote_clients();
        self.drain_ggrs_events();

        if self.session.current_state() != SessionState::Running {
            return TickStatus::Waiting;
        }
        if self.skip_frames > 0 {
            self.skip_frames -= 1;
            return TickStatus::Skipped;
        }
        let input = NetInput(local.0 | PlayerInput::PRESENT);
        if self
            .session
            .add_local_input(self.local_handle, input)
            .is_err()
        {
            return TickStatus::Waiting;
        }
        match self.session.advance_frame() {
            Ok(requests) => {
                self.handle_requests(requests);
                TickStatus::Advanced
            }
            Err(GgrsError::PredictionThreshold) => TickStatus::Skipped,
            Err(_) => TickStatus::Waiting,
        }
    }

    fn drain_ggrs_events(&mut self) {
        for ev in self.session.events() {
            let mapped = match ev {
                GgrsEvent::Synchronizing { addr, total, count } => Some(NetEvent::Synchronizing {
                    slot: addr.0,
                    count,
                    total,
                }),
                GgrsEvent::Synchronized { addr } => Some(NetEvent::Synchronized { slot: addr.0 }),
                GgrsEvent::NetworkInterrupted { addr, .. } => {
                    Some(NetEvent::Interrupted { slot: addr.0 })
                }
                GgrsEvent::NetworkResumed { addr } => Some(NetEvent::Resumed { slot: addr.0 }),
                GgrsEvent::Disconnected { addr } => Some(NetEvent::Disconnected { slot: addr.0 }),
                GgrsEvent::DesyncDetected { frame, .. } => Some(NetEvent::Desync { frame }),
                GgrsEvent::WaitRecommendation { skip_frames } => {
                    self.skip_frames = self.skip_frames.saturating_add(skip_frames);
                    None
                }
            };
            if let Some(m) = mapped {
                self.events.push(m);
            }
        }
    }

    fn handle_requests(&mut self, requests: Vec<GgrsRequest<HgConfig>>) {
        for req in requests {
            match req {
                GgrsRequest::SaveGameState { cell, frame } => {
                    let sum = self.host.state.checksum();
                    cell.save(frame, Some(*self.host.state), Some(u128::from(sum)));
                    if self.record_checksums {
                        self.checksum_log.push((frame, sum));
                    }
                }
                GgrsRequest::LoadGameState { cell, .. } => {
                    if let Some(s) = cell.load() {
                        *self.host.state = s;
                    }
                }
                GgrsRequest::AdvanceFrame { inputs } => {
                    let mut frame_inputs = [PlayerInput::default(); MAX_PLAYERS];
                    for (slot, (input, status)) in frame_inputs.iter_mut().zip(inputs) {
                        *slot = if status == InputStatus::Disconnected {
                            PlayerInput::default()
                        } else {
                            input.into()
                        };
                    }
                    self.host.world.step(&mut self.host.state, &frame_inputs);
                }
            }
        }
    }

    /// Drain UI notifications.
    pub fn take_events(&mut self) -> Vec<NetEvent> {
        std::mem::take(&mut self.events)
    }

    /// Current (possibly predicted) state.
    pub fn state(&self) -> &State {
        &self.host.state
    }

    /// World (map) in use.
    pub fn world(&self) -> &World {
        &self.host.world
    }

    /// Latest frame all peers agreed on.
    pub fn confirmed_frame(&self) -> i32 {
        self.session.confirmed_frame()
    }

    /// Round-trip ping to a remote handle in ms, if known.
    pub fn ping_ms(&self, slot: u8) -> Option<u128> {
        self.session
            .network_stats(usize::from(slot))
            .ok()
            .map(|s| s.ping)
    }

    /// Local handle.
    pub fn local_handle(&self) -> usize {
        self.local_handle
    }

    /// Socket drop counters.
    pub fn socket_stats(&self) -> SocketStats {
        self.queues.borrow().stats
    }
}

/// Single-player runner: no GGRS, same sim.
pub struct LocalRunner {
    host: SimHost,
}

impl LocalRunner {
    /// New solo game.
    pub fn new(seed: u32, map_id: u8) -> Self {
        Self {
            host: SimHost::new(1, seed, map_id),
        }
    }

    /// Step one frame.
    pub fn tick(&mut self, local: PlayerInput) -> TickStatus {
        let mut inputs = [PlayerInput::default(); MAX_PLAYERS];
        inputs[0] = PlayerInput(local.0 | PlayerInput::PRESENT);
        self.host.world.step(&mut self.host.state, &inputs);
        TickStatus::Advanced
    }

    /// Current state.
    pub fn state(&self) -> &State {
        &self.host.state
    }

    /// World (map).
    pub fn world(&self) -> &World {
        &self.host.world
    }
}

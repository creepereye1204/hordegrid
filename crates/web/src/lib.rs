//! Browser boundary. One `Game` object; per-frame boundary calls are O(packets), bulk data is a
//! pointer into wasm memory (docs/design-docs/wasm-boundary.md).

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// wasm-bindgen exports take `&mut self`/primitive args by design
#![allow(clippy::needless_pass_by_value, clippy::new_without_default)]

pub mod view;

use hg_net::{LocalRunner, NetEvent, NetSettings, Runner, TickStatus};
use hg_sim::config::{EVENT_RING, MAP_H, MAP_W};
use hg_sim::{PlayerInput, State, World};
use wasm_bindgen::prelude::*;

enum Mode {
    Solo(Box<LocalRunner>),
    Net(Box<Runner>),
}

/// A running game session.
#[wasm_bindgen]
pub struct Game {
    mode: Mode,
    view: Vec<i32>,
    last_event_frame: u32,
    local_handle: i32,
}

#[wasm_bindgen]
impl Game {
    /// Start a session. `num_players == 1` runs solo without networking.
    /// `game_mode` is `hg_sim::GameMode` as `u8` (0 = survival, 1 = hide & seek).
    ///
    /// # Errors
    /// Invalid player count or handle.
    #[wasm_bindgen(constructor)]
    pub fn new(
        num_players: u8,
        local_handle: u8,
        seed: u32,
        map_id: u8,
        game_mode: u8,
    ) -> Result<Game, JsError> {
        let mode = if num_players <= 1 {
            Mode::Solo(Box::new(LocalRunner::new(seed, map_id, game_mode)))
        } else {
            let runner = Runner::new(
                num_players,
                local_handle,
                seed,
                map_id,
                game_mode,
                NetSettings::default(),
            )
            .map_err(|e| JsError::new(&e.to_string()))?;
            Mode::Net(Box::new(runner))
        };
        Ok(Game {
            mode,
            view: Vec::with_capacity(view::CAPACITY),
            last_event_frame: 0,
            local_handle: i32::from(local_handle),
        })
    }

    /// Feed a datagram received from remote player `slot`.
    pub fn push_packet(&mut self, slot: u8, bytes: &[u8]) {
        if let Mode::Net(r) = &mut self.mode {
            r.push_packet(slot, bytes);
        }
    }

    /// Advance one fixed step with the local input bits. Returns [`TickStatus`] as u8.
    pub fn tick(&mut self, input_bits: u16) -> u8 {
        let input = PlayerInput(input_bits);
        let status = match &mut self.mode {
            Mode::Solo(r) => r.tick(input),
            Mode::Net(r) => r.tick(input),
        };
        status as u8
    }

    /// Datagrams to send, framed as repeated `[slot u8][len u16 LE][bytes]`.
    pub fn take_outbox(&mut self) -> Vec<u8> {
        let Mode::Net(r) = &mut self.mode else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for (slot, bytes) in r.take_outbox() {
            let Ok(len) = u16::try_from(bytes.len()) else {
                continue;
            };
            out.push(slot.0);
            out.extend_from_slice(&len.to_le_bytes());
            out.extend_from_slice(&bytes);
        }
        out
    }

    /// Rebuild the render view; returns its length in `i32`s.
    pub fn render(&mut self) -> usize {
        let mut v = std::mem::take(&mut self.view);
        view::fill(&mut v, self.state(), self.local_handle);
        self.view = v;
        self.view.len()
    }

    /// Pointer to the render view (valid until the next `render`; rebuild JS views every frame).
    pub fn render_ptr(&self) -> *const i32 {
        self.view.as_ptr()
    }

    /// New feedback events since the last call as `[frame, kind, a, x, y]*`.
    /// Events from frames re-simulated by a rollback are not repeated.
    pub fn take_events(&mut self) -> Vec<i32> {
        let s = self.state();
        let ring = &s.events;
        let mut out = Vec::new();
        let mut newest = self.last_event_frame;
        for k in 0..EVENT_RING {
            let ev = ring.items[(ring.head as usize + k) % EVENT_RING];
            if ev.kind != 0 && ev.frame > self.last_event_frame && ev.frame <= s.header.frame {
                out.extend_from_slice(&[
                    ev.frame as i32,
                    i32::from(ev.kind),
                    i32::from(ev.a),
                    ev.x.0,
                    ev.y.0,
                ]);
                newest = newest.max(ev.frame);
            }
        }
        self.last_event_frame = newest.max(self.last_event_frame);
        out
    }

    /// Session notifications as `[code, slot, value]*`:
    /// 1 synchronizing(value=count), 2 synchronized, 3 interrupted, 4 resumed, 5 disconnected, 6 desync(value=frame).
    pub fn take_net_events(&mut self) -> Vec<i32> {
        let Mode::Net(r) = &mut self.mode else {
            return Vec::new();
        };
        r.take_events()
            .into_iter()
            .flat_map(|e| match e {
                NetEvent::Synchronizing { slot, count, .. } => [1, i32::from(slot), count as i32],
                NetEvent::Synchronized { slot } => [2, i32::from(slot), 0],
                NetEvent::Interrupted { slot } => [3, i32::from(slot), 0],
                NetEvent::Resumed { slot } => [4, i32::from(slot), 0],
                NetEvent::Disconnected { slot } => [5, i32::from(slot), 0],
                NetEvent::Desync { frame } => [6, -1, frame],
            })
            .collect()
    }

    /// Ping to a remote handle in ms, or -1.
    pub fn ping_ms(&self, slot: u8) -> i32 {
        match &self.mode {
            Mode::Net(r) => r
                .ping_ms(slot)
                .map_or(-1, |p| p.min(i32::MAX as u128) as i32),
            Mode::Solo(_) => -1,
        }
    }

    /// Map width in tiles.
    pub fn map_width(&self) -> usize {
        MAP_W
    }

    /// Map height in tiles.
    pub fn map_height(&self) -> usize {
        MAP_H
    }

    /// Tiles row-major: 1 wall, 0 floor.
    pub fn map_tiles(&self) -> Vec<u8> {
        self.world().map.tiles().collect()
    }

    /// Hex checksum of the current state (debug overlay / desync reports).
    pub fn checksum_hex(&self) -> String {
        format!("{:016x}", self.state().checksum())
    }

    /// Raw state bytes for desync dumps.
    pub fn state_dump(&self) -> Vec<u8> {
        self.state().as_bytes().to_vec()
    }
}

impl Game {
    fn state(&self) -> &State {
        match &self.mode {
            Mode::Solo(r) => r.state(),
            Mode::Net(r) => r.state(),
        }
    }

    fn world(&self) -> &World {
        match &self.mode {
            Mode::Solo(r) => r.world(),
            Mode::Net(r) => r.world(),
        }
    }
}

// Keep TickStatus discriminants in sync with web/src/core/wasm.ts
const _: () = {
    assert!(TickStatus::Advanced as u8 == 0);
    assert!(TickStatus::Waiting as u8 == 1);
    assert!(TickStatus::Skipped as u8 == 2);
};

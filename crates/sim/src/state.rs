//! Rollback-able world state. Flat, `Pod`, no padding — enforced by `derive(Pod)` at compile time.

use bytemuck::{Pod, Zeroable};

use crate::config::{EVENT_RING, MAX_ENEMIES, MAX_PLAYERS, MAX_SHOTS, TILES};
use crate::fx::Fixed;
use crate::rng::Rng;

/// Player lifecycle. Stored as `u8`.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Life {
    /// Slot unused.
    Empty = 0,
    /// Playing.
    Alive = 1,
    /// HP 0, can be revived.
    Downed = 2,
    /// Waiting for next wave.
    Dead = 3,
    /// Disconnected and removed.
    Gone = 4,
}

impl Life {
    /// Decode from storage; unknown values map to `Empty`.
    pub const fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Alive,
            2 => Self::Downed,
            3 => Self::Dead,
            4 => Self::Gone,
            _ => Self::Empty,
        }
    }
}

/// Players, structure-of-arrays.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Players {
    /// Position x.
    pub x: [Fixed; MAX_PLAYERS],
    /// Position y.
    pub y: [Fixed; MAX_PLAYERS],
    /// Hit points.
    pub hp: [i32; MAX_PLAYERS],
    /// Kills credited.
    pub kills: [u32; MAX_PLAYERS],
    /// [`Life`] as u8.
    pub life: [u8; MAX_PLAYERS],
    /// Facing direction 1..=8.
    pub facing: [u8; MAX_PLAYERS],
    /// Equipped weapon index.
    pub weapon: [u8; MAX_PLAYERS],
    /// Hurt flash frames (render only, but deterministic).
    pub hurt_flash: [u8; MAX_PLAYERS],
    /// Frames until next shot.
    pub cooldown: [u16; MAX_PLAYERS],
    /// Previous frame input (edge detection).
    pub prev_input: [u16; MAX_PLAYERS],
    /// Downed countdown.
    pub down_timer: [u16; MAX_PLAYERS],
    /// Revive progress.
    pub revive: [u16; MAX_PLAYERS],
    /// Consecutive frames without `present`.
    pub absent: [u16; MAX_PLAYERS],
    /// Revives performed.
    pub revives_done: [u16; MAX_PLAYERS],
}

/// Enemies, structure-of-arrays.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Enemies {
    /// Position x.
    pub x: [Fixed; MAX_ENEMIES],
    /// Position y.
    pub y: [Fixed; MAX_ENEMIES],
    /// Hit points.
    pub hp: [i32; MAX_ENEMIES],
    /// [`crate::config::EnemyKind`] as u8.
    pub kind: [u8; MAX_ENEMIES],
    /// 1 if slot is in use.
    pub alive: [u8; MAX_ENEMIES],
    /// Frames until next attack.
    pub cooldown: [u8; MAX_ENEMIES],
    /// Frames of stagger remaining.
    pub stagger: [u8; MAX_ENEMIES],
}

/// Projectiles, structure-of-arrays.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Shots {
    /// Position x.
    pub x: [Fixed; MAX_SHOTS],
    /// Position y.
    pub y: [Fixed; MAX_SHOTS],
    /// Velocity x.
    pub vx: [Fixed; MAX_SHOTS],
    /// Velocity y.
    pub vy: [Fixed; MAX_SHOTS],
    /// Frames left.
    pub ttl: [u8; MAX_SHOTS],
    /// Owner player index.
    pub owner: [u8; MAX_SHOTS],
    /// Damage.
    pub damage: [u8; MAX_SHOTS],
    /// Weapon kind (render).
    pub kind: [u8; MAX_SHOTS],
}

/// Wave phase.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WavePhase {
    /// Countdown before spawning.
    Intermission = 0,
    /// Spawning and fighting.
    Active = 1,
    /// Everyone is down.
    GameOver = 2,
}

/// Wave / scoring state.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Wave {
    /// Current wave number (1-based once started).
    pub number: u32,
    /// [`WavePhase`] as u32.
    pub phase: u32,
    /// Phase timer (frames).
    pub timer: u32,
    /// Enemies left to spawn this wave.
    pub to_spawn: u32,
    /// Team score.
    pub team_score: u32,
    /// Current combo.
    pub combo: u32,
    /// Frames until combo resets.
    pub combo_timer: u32,
    /// Bitmask of unlocked weapons.
    pub unlocked: u32,
    /// Enemies alive (cached count).
    pub enemies_alive: u32,
    /// Best combo reached.
    pub best_combo: u32,
}

/// Feedback event kinds.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    /// Enemy hit.
    Hit = 1,
    /// Enemy killed.
    Kill = 2,
    /// Player took damage.
    PlayerHurt = 3,
    /// Player downed.
    Down = 4,
    /// Player revived.
    Revive = 5,
    /// Weapon unlocked (a = weapon).
    Unlock = 6,
    /// Wave started (a = wave number, saturating).
    WaveStart = 7,
    /// Game over.
    GameOver = 8,
    /// Shot fired (a = weapon).
    Fire = 9,
}

/// One feedback event.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct Event {
    /// Frame it happened.
    pub frame: u32,
    /// [`EventKind`] as u8.
    pub kind: u8,
    /// Payload a (player index, weapon, …).
    pub a: u8,
    /// Payload b.
    pub b: u8,
    /// Reserved (keeps layout padding-free).
    pub reserved: u8,
    /// Position x.
    pub x: Fixed,
    /// Position y.
    pub y: Fixed,
}

/// Ring buffer of recent events.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct EventRing {
    /// Storage.
    pub items: [Event; EVENT_RING],
    /// Next write index.
    pub head: u32,
    /// Total pushed (monotonic).
    pub total: u32,
}

impl EventRing {
    /// Append an event, overwriting the oldest.
    pub fn push(&mut self, ev: Event) {
        let i = self.head as usize % EVENT_RING;
        self.items[i] = ev;
        self.head = ((self.head as usize + 1) % EVENT_RING) as u32;
        self.total = self.total.wrapping_add(1);
    }
}

/// Flow field: per tile, direction (1..=8) toward the nearest alive player; 0 = unreachable/target.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct FlowField {
    /// Directions.
    pub dir: [u8; TILES],
}

/// Session constants mixed into state so every snapshot is self-describing.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Header {
    /// Frames simulated.
    pub frame: u32,
    /// Players in the session.
    pub num_players: u32,
    /// Seed used at init.
    pub seed: u32,
    /// Map id.
    pub map_id: u32,
}

/// The whole rollback-able world.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct State {
    /// Session header.
    pub header: Header,
    /// PRNG.
    pub rng: Rng,
    /// Players.
    pub players: Players,
    /// Enemies.
    pub enemies: Enemies,
    /// Projectiles.
    pub shots: Shots,
    /// Wave and score.
    pub wave: Wave,
    /// Feedback events.
    pub events: EventRing,
    /// Pathing.
    pub flow: FlowField,
}

impl State {
    /// Byte view used for checksums and desync dumps.
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }

    /// Checksum of the full state (xxh3-64).
    pub fn checksum(&self) -> u64 {
        xxhash_rust::xxh3::xxh3_64(self.as_bytes())
    }

    /// Current wave phase.
    pub fn phase(&self) -> WavePhase {
        match self.wave.phase {
            1 => WavePhase::Active,
            2 => WavePhase::GameOver,
            _ => WavePhase::Intermission,
        }
    }

    /// Player life.
    pub fn life(&self, i: usize) -> Life {
        Life::from_u8(self.players.life[i])
    }

    pub(crate) fn emit(&mut self, kind: EventKind, a: u8, x: Fixed, y: Fixed) {
        let frame = self.header.frame;
        self.events.push(Event {
            frame,
            kind: kind as u8,
            a,
            b: 0,
            reserved: 0,
            x,
            y,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_measuring_state_then_within_budget() {
        let size = core::mem::size_of::<State>();
        assert!(size <= 64 * 1024, "State is {size} bytes, budget 64KiB");
    }
}

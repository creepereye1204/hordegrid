//! Compile-time capacities and tuning tables (Flyweight data — shared, immutable, outside `State`).
//! Numbers mirror docs/product-specs/core-gameplay.md and weapons-and-enemies.md.

use crate::fx::Fixed;

/// Maximum players in a session.
pub const MAX_PLAYERS: usize = 4;
/// Enemy slot capacity.
pub const MAX_ENEMIES: usize = 384;
/// Projectile slot capacity.
pub const MAX_SHOTS: usize = 256;
/// Event ring capacity (render/sound feedback).
pub const EVENT_RING: usize = 64;
/// Map width in tiles.
pub const MAP_W: usize = 40;
/// Map height in tiles.
pub const MAP_H: usize = 30;
/// Tile count.
pub const TILES: usize = MAP_W * MAP_H;

/// Visible half-extent used for "spawn outside every camera" (tiles).
pub const VIEW_HALF_W: i32 = 12;
/// Visible half-extent (tiles).
pub const VIEW_HALF_H: i32 = 8;

/// Frames per second of the fixed step.
pub const FPS: u32 = 60;

// ---- players ----
/// Player collision half-size.
pub const PLAYER_RADIUS: Fixed = Fixed::ratio(35, 100);
/// Tiles per frame.
pub const PLAYER_SPEED: Fixed = Fixed::ratio(9, 100);
/// Crawl speed while downed.
pub const DOWNED_SPEED: Fixed = Fixed::ratio(2, 100);
/// Starting HP.
pub const PLAYER_HP: i32 = 100;
/// HP after a teammate revive.
pub const REVIVE_HP: i32 = 40;
/// HP when a dead player returns at the next wave.
pub const RESPAWN_HP: i32 = 50;
/// Frames a downed player survives.
pub const DOWNED_FRAMES: u16 = 10 * 60;
/// Frames a teammate must stay adjacent to revive.
pub const REVIVE_FRAMES: u16 = 2 * 60;
/// Revive distance (tiles).
pub const REVIVE_RANGE: Fixed = Fixed::ONE;
/// Frames without `present` before a player is removed.
pub const ABSENT_FRAMES: u16 = 3 * 60;
/// Frames of delay when switching weapons.
pub const WEAPON_SWITCH_FRAMES: u16 = 8;

// ---- combo / score ----
/// Frames a combo survives without a kill.
pub const COMBO_FRAMES: u32 = 2 * 60;
/// Combo cap for the multiplier.
pub const COMBO_CAP: u32 = 50;

// ---- waves ----
/// Frames between waves.
pub const INTERMISSION_FRAMES: u32 = 4 * 60;
/// Minimum frames between two spawns (late waves).
pub const SPAWN_INTERVAL: u32 = 16;
/// Player-count multiplier ×100 (≈ n^0.7), indexed by present players.
pub const PLAYER_COUNT_MUL_X100: [u32; MAX_PLAYERS + 1] = [100, 100, 162, 216, 264];

/// Weapon identifiers. Stored as `u8` in state.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeaponKind {
    /// Starting weapon. Infinite ammo, never reloads.
    Pistol = 0,
    /// Fast, low damage, jitter.
    Smg = 1,
    /// Short-range pellet fan.
    Shotgun = 2,
    /// Slow, splash damage on impact (docs/product-specs/weapons-and-enemies.md #6).
    Rocket = 3,
}

/// Number of weapon kinds.
pub const WEAPON_COUNT: usize = 4;

/// Ammo value meaning "never runs out, never reloads" (the starting pistol).
pub const INFINITE_AMMO: u16 = u16::MAX;

/// Frames a reload takes, same for every weapon (kept simple on purpose).
pub const RELOAD_FRAMES: u16 = 60;

/// Immutable weapon tuning.
#[derive(Clone, Copy, Debug)]
pub struct WeaponStats {
    /// Team score needed to unlock.
    pub unlock_score: u32,
    /// Damage per projectile.
    pub damage: u8,
    /// Frames between shots.
    pub cooldown: u16,
    /// Projectile speed (tiles/frame).
    pub speed: Fixed,
    /// Projectile lifetime (frames).
    pub ttl: u8,
    /// Projectiles per shot.
    pub pellets: u8,
    /// Perpendicular spread per pellet step / max jitter (tiles/frame).
    pub spread: Fixed,
    /// Ammo pool capacity. [`INFINITE_AMMO`] = never depletes, never reloads.
    pub max_ammo: u16,
    /// Splash radius on impact; [`Fixed::ZERO`] = single-target only.
    pub splash_radius: Fixed,
}

/// Weapon table indexed by [`WeaponKind`].
pub const WEAPONS: [WeaponStats; WEAPON_COUNT] = [
    WeaponStats {
        unlock_score: 0,
        damage: 12,
        cooldown: 18,
        speed: Fixed::ratio(60, 100),
        ttl: 30,
        pellets: 1,
        spread: Fixed::ZERO,
        max_ammo: INFINITE_AMMO,
        splash_radius: Fixed::ZERO,
    },
    WeaponStats {
        unlock_score: 1_500,
        damage: 7,
        cooldown: 5,
        speed: Fixed::ratio(70, 100),
        ttl: 28,
        pellets: 1,
        spread: Fixed::ratio(4, 100),
        max_ammo: 240,
        splash_radius: Fixed::ZERO,
    },
    WeaponStats {
        unlock_score: 4_000,
        damage: 9,
        cooldown: 40,
        speed: Fixed::ratio(55, 100),
        ttl: 10,
        pellets: 5,
        spread: Fixed::ratio(8, 100),
        max_ammo: 60,
        splash_radius: Fixed::ZERO,
    },
    WeaponStats {
        unlock_score: 16_000,
        damage: 60,
        cooldown: 50,
        speed: Fixed::ratio(45, 100),
        ttl: 40,
        pellets: 1,
        spread: Fixed::ZERO,
        max_ammo: 20,
        splash_radius: Fixed::from_int(2),
    },
];

/// Enemy identifiers.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnemyKind {
    /// Slow, sturdy.
    Walker = 0,
    /// Fast, fragile.
    Runner = 1,
}

/// Immutable enemy tuning.
#[derive(Clone, Copy, Debug)]
pub struct EnemyStats {
    /// Hit points.
    pub hp: i32,
    /// Base speed (tiles/frame).
    pub speed: Fixed,
    /// Contact damage.
    pub damage: i32,
    /// Frames between attacks.
    pub attack_cooldown: u8,
    /// Collision half-size.
    pub radius: Fixed,
    /// Base score.
    pub score: u32,
    /// Separation strength (Runners pack tighter).
    pub separation: Fixed,
}

/// Enemy table indexed by [`EnemyKind`].
pub const ENEMIES: [EnemyStats; 2] = [
    EnemyStats {
        hp: 30,
        speed: Fixed::ratio(35, 1000),
        damage: 8,
        attack_cooldown: 30,
        radius: Fixed::ratio(35, 100),
        score: 10,
        separation: Fixed::ratio(3, 100),
    },
    EnemyStats {
        hp: 18,
        speed: Fixed::ratio(70, 1000),
        damage: 5,
        attack_cooldown: 20,
        radius: Fixed::ratio(30, 100),
        score: 15,
        separation: Fixed::ratio(1, 100),
    },
];

/// Projectile collision half-size.
pub const SHOT_RADIUS: Fixed = Fixed::ratio(1, 10);
/// Knockback applied on hit (tiles).
pub const KNOCKBACK: Fixed = Fixed::ratio(5, 100);
/// Frames an enemy is staggered after a hit.
pub const STAGGER_FRAMES: u8 = 6;

// Tunnelling guard: per-substep movement must stay below half a tile.
const _: () = assert!(PLAYER_SPEED.0 < Fixed::HALF.0);
const _: () = assert!(ENEMIES[1].speed.0 < Fixed::HALF.0);
const _: () = assert!(
    WEAPONS[1].speed.0 / 2 < Fixed::HALF.0,
    "shots move in 2 substeps"
);

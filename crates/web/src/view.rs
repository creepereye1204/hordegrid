//! `RenderView`: a flat `i32` buffer the renderer reads without per-entity boundary calls.
//! Layout is the contract with `web/src/core/view-layout.ts` (docs/generated/render-view.md).

use hg_sim::config::{MAX_ENEMIES, MAX_PLAYERS, MAX_SHOTS};
use hg_sim::input::dir_from_delta;
use hg_sim::State;

/// Header length.
pub const HEADER: usize = 17;
/// Header field indices.
pub mod h {
    /// Frame.
    pub const FRAME: usize = 0;
    /// Player records (always `MAX_PLAYERS`).
    pub const N_PLAYERS: usize = 1;
    /// Enemy records.
    pub const N_ENEMIES: usize = 2;
    /// Shot records.
    pub const N_SHOTS: usize = 3;
    /// Wave number (survival only).
    pub const WAVE: usize = 4;
    /// Phase: survival `WavePhase`, or hide & seek `HideSeekPhase` when `MODE == 1`.
    pub const PHASE: usize = 5;
    /// Timer for whichever phase `PHASE` names.
    pub const TIMER: usize = 6;
    /// Team score (survival only).
    pub const SCORE: usize = 7;
    /// Combo (survival only).
    pub const COMBO: usize = 8;
    /// Combo timer (survival only).
    pub const COMBO_TIMER: usize = 9;
    /// Unlocked weapon mask (survival only).
    pub const UNLOCKED: usize = 10;
    /// Local player handle (-1 spectator).
    pub const LOCAL: usize = 11;
    /// Session player count.
    pub const NUM_PLAYERS: usize = 12;
    /// Best combo (survival only).
    pub const BEST_COMBO: usize = 13;
    /// Enemies left to spawn (survival only).
    pub const TO_SPAWN: usize = 14;
    /// Session mode: 0 survival, 1 hide & seek.
    pub const MODE: usize = 15;
    /// Hide & seek winner: 0 undecided, 1 hiders, 2 seeker. Always 0 in survival.
    pub const HS_WINNER: usize = 16;
}
/// Player record stride: x, y, hp, life, facing, weapon, `hurt_flash`, `down_timer`, revive,
/// kills, hide-seek `role`, hide-seek `found` (last two unused/0 in survival).
pub const PLAYER_STRIDE: usize = 12;
/// Enemy record stride: slot, x, y, kind, `hp_permille`, stagger. `slot` keys interpolation.
pub const ENEMY_STRIDE: usize = 6;
/// Shot record stride: x, y, kind, dir.
pub const SHOT_STRIDE: usize = 4;
/// Buffer capacity (never reallocates during play).
pub const CAPACITY: usize =
    HEADER + MAX_PLAYERS * PLAYER_STRIDE + MAX_ENEMIES * ENEMY_STRIDE + MAX_SHOTS * SHOT_STRIDE;

/// Rebuild `out` from `s`. `out` keeps its capacity.
pub fn fill(out: &mut Vec<i32>, s: &State, local_handle: i32) {
    out.clear();
    out.resize(HEADER, 0);
    let hide_seek = s.mode() == hg_sim::GameMode::HideSeek;
    out[h::FRAME] = s.header.frame as i32;
    out[h::N_PLAYERS] = MAX_PLAYERS as i32;
    out[h::WAVE] = s.wave.number as i32;
    out[h::PHASE] = if hide_seek {
        s.hide_seek.phase as i32
    } else {
        s.wave.phase as i32
    };
    out[h::TIMER] = if hide_seek {
        s.hide_seek.timer as i32
    } else {
        s.wave.timer as i32
    };
    out[h::SCORE] = s.wave.team_score as i32;
    out[h::COMBO] = s.wave.combo as i32;
    out[h::COMBO_TIMER] = s.wave.combo_timer as i32;
    out[h::UNLOCKED] = s.wave.unlocked as i32;
    out[h::LOCAL] = local_handle;
    out[h::NUM_PLAYERS] = s.header.num_players as i32;
    out[h::BEST_COMBO] = s.wave.best_combo as i32;
    out[h::TO_SPAWN] = s.wave.to_spawn as i32;
    out[h::MODE] = s.header.mode as i32;
    out[h::HS_WINNER] = s.hide_seek.winner as i32;

    let p = &s.players;
    for i in 0..MAX_PLAYERS {
        out.extend_from_slice(&[
            p.x[i].0,
            p.y[i].0,
            p.hp[i],
            i32::from(p.life[i]),
            i32::from(p.facing[i]),
            i32::from(p.weapon[i]),
            i32::from(p.hurt_flash[i]),
            i32::from(p.down_timer[i]),
            i32::from(p.revive[i]),
            p.kills[i] as i32,
            i32::from(s.hide_seek.role[i]),
            i32::from(s.hide_seek.found[i]),
        ]);
    }

    let e = &s.enemies;
    let mut n_enemies = 0;
    for i in (0..MAX_ENEMIES).filter(|&i| e.alive[i] != 0) {
        let max_hp = hg_sim::config::ENEMIES
            [usize::from(e.kind[i]).min(hg_sim::config::ENEMIES.len() - 1)]
        .hp
        .max(1);
        out.extend_from_slice(&[
            e.x[i].0,
            e.y[i].0,
            i32::from(e.kind[i]),
            e.hp[i] * 1000 / max_hp,
            i32::from(e.stagger[i]),
        ]);
        n_enemies += 1;
    }
    out[h::N_ENEMIES] = n_enemies;

    let sh = &s.shots;
    let mut n_shots = 0;
    for i in (0..MAX_SHOTS).filter(|&i| sh.ttl[i] != 0) {
        out.extend_from_slice(&[
            sh.x[i].0,
            sh.y[i].0,
            i32::from(sh.kind[i]),
            i32::from(dir_from_delta(sh.vx[i], sh.vy[i])),
        ]);
        n_shots += 1;
    }
    out[h::N_SHOTS] = n_shots;
}

#[cfg(test)]
mod tests {
    use super::*;
    use hg_sim::World;

    #[test]
    fn when_filling_then_counts_and_length_agree() {
        let world = World::new(0);
        let s = world.initial_state(2, 1, 0);
        let mut v = Vec::with_capacity(CAPACITY);
        fill(&mut v, &s, 0);
        let n_e = v[h::N_ENEMIES] as usize;
        let n_s = v[h::N_SHOTS] as usize;
        assert_eq!(
            v.len(),
            HEADER + MAX_PLAYERS * PLAYER_STRIDE + n_e * ENEMY_STRIDE + n_s * SHOT_STRIDE
        );
        assert!(v.capacity() >= CAPACITY);
    }
}

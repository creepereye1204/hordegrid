//! Test bots shared by integration tests.
#![allow(dead_code)]

use hg_sim::config::{MAX_ENEMIES, MAX_PLAYERS};
use hg_sim::input::dir_from_delta;
use hg_sim::rng::Rng;
use hg_sim::state::Life;
use hg_sim::{Fixed, PlayerInput, State};

/// Random wanderer (exercises odd input combos).
pub fn random_inputs(
    rng: &mut Rng,
    held: &mut [u16; MAX_PLAYERS],
    players: usize,
) -> [PlayerInput; MAX_PLAYERS] {
    let mut out = [PlayerInput::default(); MAX_PLAYERS];
    for i in 0..players {
        if rng.below(20) == 0 {
            let dir = rng.below(9) as u16;
            let fire = if rng.below(4) != 0 {
                PlayerInput::FIRE
            } else {
                0
            };
            let strafe = if rng.below(3) == 0 {
                PlayerInput::STRAFE
            } else {
                0
            };
            let next = if rng.below(30) == 0 {
                PlayerInput::WEAPON_NEXT
            } else {
                0
            };
            held[i] = dir | fire | strafe | next | PlayerInput::PRESENT;
        }
        out[i] = PlayerInput(held[i]);
    }
    out
}

/// Kiting bot: aims at the nearest enemy, backs away when close, revives downed teammates.
pub fn kiting_inputs(s: &State, players: usize) -> [PlayerInput; MAX_PLAYERS] {
    let mut out = [PlayerInput::default(); MAX_PLAYERS];
    let frame = s.header.frame;
    for (i, slot) in out.iter_mut().enumerate().take(players) {
        let (px, py) = (s.players.x[i], s.players.y[i]);
        let mut bits = PlayerInput::PRESENT | PlayerInput::FIRE;
        if let Some(d) = (0..MAX_PLAYERS).find(|&d| d != i && s.life(d) == Life::Downed) {
            bits |= u16::from(dir_from_delta(s.players.x[d] - px, s.players.y[d] - py));
            *slot = PlayerInput(bits);
            continue;
        }
        let nearest = (0..MAX_ENEMIES)
            .filter(|&e| s.enemies.alive[e] != 0)
            .min_by_key(|&e| (Fixed::len_sq(s.enemies.x[e] - px, s.enemies.y[e] - py), e));
        if let Some(e) = nearest {
            let (dx, dy) = (s.enemies.x[e] - px, s.enemies.y[e] - py);
            let toward = u16::from(dir_from_delta(dx, dy));
            let close = Fixed::len_sq(dx, dy) < Fixed::from_int(4).sq();
            if frame.is_multiple_of(4) || !close {
                bits |= toward; // turn to aim (and approach when far)
            } else {
                let away = (toward + 3) % 8 + 1;
                bits |= away | PlayerInput::STRAFE;
            }
            if frame.is_multiple_of(240) {
                bits |= PlayerInput::WEAPON_NEXT;
            }
        }
        *slot = PlayerInput(bits);
    }
    out
}

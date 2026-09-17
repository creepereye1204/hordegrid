#![allow(
    clippy::float_arithmetic,
    clippy::cast_precision_loss,
    clippy::needless_range_loop
)]
//! Gameplay acceptance checks from docs/product-specs/core-gameplay.md.

mod common;

use common::kiting_inputs;
use hg_sim::config::{MAX_ENEMIES, MAX_PLAYERS, VIEW_HALF_H, VIEW_HALF_W};
use hg_sim::state::{Life, WavePhase};
use hg_sim::{Fixed, World};

/// AC3: no enemy stays motionless for 5 s unless it is attacking (in contact) or staggered.
#[test]
fn when_playing_ten_minutes_then_no_enemy_is_stuck() {
    let mut world = World::new(0);
    let mut s = world.initial_state(2, 2024);
    let mut last_pos = [(Fixed::ZERO, Fixed::ZERO); MAX_ENEMIES];
    let mut still = [0u32; MAX_ENEMIES];
    for _ in 0..60 * 60 * 10 {
        let inputs = kiting_inputs(&s, 2);
        world.step(&mut s, &inputs);
        if s.phase() == WavePhase::GameOver {
            break;
        }
        // enemies intentionally idle when nobody is standing (all downed)
        let anyone_alive = (0..MAX_PLAYERS).any(|p| s.life(p) == Life::Alive);
        for e in 0..MAX_ENEMIES {
            if s.enemies.alive[e] == 0 || !anyone_alive {
                still[e] = 0;
                last_pos[e] = (Fixed::ZERO, Fixed::ZERO);
                continue;
            }
            let pos = (s.enemies.x[e], s.enemies.y[e]);
            let near_player = (0..MAX_PLAYERS).any(|p| {
                s.life(p) != Life::Empty
                    && Fixed::len_sq(s.players.x[p] - pos.0, s.players.y[p] - pos.1)
                        < Fixed::ONE.sq()
            });
            if pos == last_pos[e] && !near_player && s.enemies.stagger[e] == 0 {
                still[e] += 1;
                if still[e] >= 300 {
                    let (tx, ty) = (pos.0.floor_int(), pos.1.floor_int());
                    let flow = s.flow.dir[ty as usize * hg_sim::config::MAP_W + tx as usize];
                    let others: Vec<_> = (0..MAX_ENEMIES)
                        .filter(|&o| {
                            o != e
                                && s.enemies.alive[o] != 0
                                && Fixed::len_sq(s.enemies.x[o] - pos.0, s.enemies.y[o] - pos.1)
                                    < Fixed::ONE.sq()
                        })
                        .map(|o| {
                            (
                                o,
                                s.enemies.x[o].0 as f64 / 65536.0,
                                s.enemies.y[o].0 as f64 / 65536.0,
                            )
                        })
                        .collect();
                    let players: Vec<_> = (0..MAX_PLAYERS)
                        .map(|p| {
                            (
                                s.players.life[p],
                                s.players.x[p].0 as f64 / 65536.0,
                                s.players.y[p].0 as f64 / 65536.0,
                            )
                        })
                        .collect();
                    panic!("enemy {e} stuck at ({:.3},{:.3}) tile ({tx},{ty}) flow {flow} neighbours {others:?} players {players:?} frame {}", pos.0.0 as f64/65536.0, pos.1.0 as f64/65536.0, s.header.frame);
                }
            } else {
                still[e] = 0;
            }
            last_pos[e] = pos;
        }
    }
}

/// AC4: spawns happen outside every player's camera whenever such a door exists.
#[test]
fn when_enemy_spawns_then_outside_all_cameras_or_no_hidden_door_existed() {
    let mut world = World::new(0);
    let mut s = world.initial_state(4, 77);
    let mut prev_alive = [0u8; MAX_ENEMIES];
    for _ in 0..60 * 60 * 4 {
        let inputs = kiting_inputs(&s, 4);
        world.step(&mut s, &inputs);
        for e in 0..MAX_ENEMIES {
            if prev_alive[e] == 0 && s.enemies.alive[e] != 0 {
                let (tx, ty) = (s.enemies.x[e].floor_int(), s.enemies.y[e].floor_int());
                let visible = |px: i32, py: i32| {
                    (tx - px).abs() <= VIEW_HALF_W && (ty - py).abs() <= VIEW_HALF_H
                };
                let seen = (0..MAX_PLAYERS)
                    .filter(|&p| matches!(s.life(p), Life::Alive | Life::Downed))
                    .any(|p| visible(s.players.x[p].floor_int(), s.players.y[p].floor_int()));
                if seen {
                    let map = &world.map;
                    let hidden_door_existed =
                        map.spawns[..map.spawn_count].iter().any(|&(dx, dy)| {
                            !(0..MAX_PLAYERS)
                                .filter(|&p| matches!(s.life(p), Life::Alive | Life::Downed))
                                .any(|p| {
                                    let (px, py) =
                                        (s.players.x[p].floor_int(), s.players.y[p].floor_int());
                                    (i32::from(dx) - px).abs() <= VIEW_HALF_W
                                        && (i32::from(dy) - py).abs() <= VIEW_HALF_H
                                })
                        });
                    assert!(
                        !hidden_door_existed,
                        "spawned in view at ({tx},{ty}) although a hidden door existed"
                    );
                }
            }
        }
        prev_alive = s.enemies.alive;
    }
}

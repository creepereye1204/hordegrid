//! The pure step function: `State_{n+1} = step(State_n, inputs)`.
//! System order is part of the determinism contract — do not reorder without a desync-replay check.

use crate::config::{
    EnemyKind, ABSENT_FRAMES, COMBO_CAP, COMBO_FRAMES, DOWNED_FRAMES, DOWNED_SPEED, ENEMIES,
    INTERMISSION_FRAMES, KNOCKBACK, MAX_ENEMIES, MAX_PLAYERS, MAX_SHOTS, PLAYER_COUNT_MUL_X100,
    PLAYER_HP, PLAYER_RADIUS, PLAYER_SPEED, RESPAWN_HP, REVIVE_FRAMES, REVIVE_HP, REVIVE_RANGE,
    SHOT_RADIUS, SPAWN_INTERVAL, STAGGER_FRAMES, VIEW_HALF_H, VIEW_HALF_W, WEAPONS, WEAPON_COUNT,
    WEAPON_SWITCH_FRAMES,
};
use crate::flow::{self, FlowScratch};
use crate::fx::Fixed;
use crate::grid::EnemyGrid;
use crate::input::{dir_from_delta, PlayerInput, DIR8};
use crate::map::MapData;
use crate::physics::move_box;
use crate::state::{EventKind, GameMode, Life, State, WavePhase};

/// Tile offsets for DIR8 indices (0 = none).
const NEIGHBOUR: [(i32, i32); 9] = [
    (0, 0),
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];

/// Frames between flow-field rebuilds.
const FLOW_INTERVAL: u32 = 8;
/// Max neighbours sampled for separation (bounds worst-case cost).
const SEPARATION_SAMPLES: u32 = 6;

/// Static map + scratch buffers. Owns nothing that must roll back.
pub struct World {
    /// Map in use.
    pub map: MapData,
    flow: FlowScratch,
    grid: EnemyGrid,
}

impl World {
    /// Create a world for `map_id`.
    pub fn new(map_id: u8) -> Self {
        Self {
            map: MapData::by_id(map_id),
            flow: FlowScratch::default(),
            grid: EnemyGrid::default(),
        }
    }

    /// Fresh initial state. Boxed: `State` is tens of KiB. `mode` is [`GameMode`] as `u8`.
    pub fn initial_state(&self, num_players: u8, seed: u32, mode: u8) -> Box<State> {
        let mut s: Box<State> = bytemuck::zeroed_box();
        let n = usize::from(num_players.clamp(1, MAX_PLAYERS as u8));
        s.header.num_players = n as u32;
        s.header.seed = seed;
        s.header.mode = u32::from(mode);
        s.rng = crate::rng::Rng::from_seed(seed);
        for i in 0..n {
            let (tx, ty) = self.map.starts[i];
            s.players.x[i] = Fixed::from_int(i32::from(tx)) + Fixed::HALF;
            s.players.y[i] = Fixed::from_int(i32::from(ty)) + Fixed::HALF;
            s.players.hp[i] = PLAYER_HP;
            s.players.life[i] = Life::Alive as u8;
            s.players.facing[i] = 5;
        }
        match s.mode() {
            GameMode::Survival => {
                s.wave.phase = WavePhase::Intermission as u32;
                s.wave.timer = INTERMISSION_FRAMES / 2;
                s.wave.unlocked = 1; // pistol
                flow::rebuild(&mut s, &self.map, &mut FlowScratch::default());
            }
            GameMode::HideSeek => crate::hide_seek::init(&mut s),
        }
        s
    }

    /// Advance one frame.
    pub fn step(&mut self, s: &mut State, inputs: &[PlayerInput; MAX_PLAYERS]) {
        s.header.frame = s.header.frame.wrapping_add(1);
        let inputs = inputs.map(PlayerInput::sanitized);

        if s.mode() == GameMode::HideSeek {
            crate::hide_seek::step(s, &self.map, inputs);
            return;
        }
        if s.phase() == WavePhase::GameOver {
            return;
        }

        update_presence(s, inputs);
        update_players(s, &self.map, inputs);
        update_revives(s);
        if s.header.frame.is_multiple_of(FLOW_INTERVAL) {
            flow::rebuild(s, &self.map, &mut self.flow);
        }
        self.grid.rebuild(&s.enemies);
        update_enemies(s, &self.map, &self.grid);
        self.grid.rebuild(&s.enemies);
        update_shots(s, &self.map, &self.grid);
        update_combo(s);
        update_waves(s, &self.map);
        check_game_over(s);

        s.players.prev_input = inputs.map(|i| i.0);
    }
}

fn present_count(s: &State) -> u32 {
    (0..MAX_PLAYERS)
        .filter(|&i| !matches!(s.life(i), Life::Empty | Life::Gone))
        .count() as u32
}

fn update_presence(s: &mut State, inputs: [PlayerInput; MAX_PLAYERS]) {
    for (i, input) in inputs.iter().enumerate() {
        if matches!(s.life(i), Life::Empty | Life::Gone) {
            continue;
        }
        if input.has(PlayerInput::PRESENT) {
            s.players.absent[i] = 0;
        } else {
            s.players.absent[i] = s.players.absent[i].saturating_add(1);
            if s.players.absent[i] >= ABSENT_FRAMES {
                s.players.life[i] = Life::Gone as u8;
            }
        }
    }
}

fn update_players(s: &mut State, map: &MapData, inputs: [PlayerInput; MAX_PLAYERS]) {
    for (i, &input) in inputs.iter().enumerate() {
        let prev = PlayerInput(s.players.prev_input[i]);
        let p = &mut s.players;
        p.hurt_flash[i] = p.hurt_flash[i].saturating_sub(1);
        p.cooldown[i] = p.cooldown[i].saturating_sub(1);
        match s.life(i) {
            Life::Alive => {
                let dir = input.dir();
                if dir != 0 && !input.has(PlayerInput::STRAFE) {
                    s.players.facing[i] = dir;
                }
                let (ux, uy) = DIR8[usize::from(dir)];
                let (nx, ny) = move_box(
                    map,
                    s.players.x[i],
                    s.players.y[i],
                    ux * PLAYER_SPEED,
                    uy * PLAYER_SPEED,
                    PLAYER_RADIUS,
                );
                s.players.x[i] = nx;
                s.players.y[i] = ny;

                if input.pressed(prev, PlayerInput::WEAPON_NEXT) {
                    cycle_weapon(s, i, 1);
                } else if input.pressed(prev, PlayerInput::WEAPON_PREV) {
                    cycle_weapon(s, i, WEAPON_COUNT - 1);
                }
                if input.has(PlayerInput::FIRE) && s.players.cooldown[i] == 0 {
                    fire(s, i);
                }
            }
            Life::Downed => {
                let (ux, uy) = DIR8[usize::from(input.dir())];
                let (nx, ny) = move_box(
                    map,
                    s.players.x[i],
                    s.players.y[i],
                    ux * DOWNED_SPEED,
                    uy * DOWNED_SPEED,
                    PLAYER_RADIUS,
                );
                s.players.x[i] = nx;
                s.players.y[i] = ny;
                s.players.down_timer[i] = s.players.down_timer[i].saturating_sub(1);
                if s.players.down_timer[i] == 0 {
                    s.players.life[i] = Life::Dead as u8;
                }
            }
            Life::Empty | Life::Dead | Life::Gone => {}
        }
    }
}

fn cycle_weapon(s: &mut State, i: usize, step: usize) {
    let mut w = usize::from(s.players.weapon[i]);
    for _ in 0..WEAPON_COUNT {
        w = (w + step) % WEAPON_COUNT;
        if s.wave.unlocked & (1 << w) != 0 {
            break;
        }
    }
    if usize::from(s.players.weapon[i]) != w {
        s.players.weapon[i] = w as u8;
        s.players.cooldown[i] = s.players.cooldown[i].max(WEAPON_SWITCH_FRAMES);
    }
}

#[allow(clippy::many_single_char_names)] // x/y/w/i/k are conventional here
fn fire(s: &mut State, i: usize) {
    let w = usize::from(s.players.weapon[i]).min(WEAPON_COUNT - 1);
    let stats = WEAPONS[w];
    let facing = usize::from(s.players.facing[i].clamp(1, 8));
    let (ux, uy) = DIR8[facing];
    let (px, py) = (-uy, ux); // perpendicular
    let pellets = i32::from(stats.pellets);
    for k in 0..pellets {
        let Some(slot) = (0..MAX_SHOTS).find(|&j| s.shots.ttl[j] == 0) else {
            break;
        };
        let lateral = if pellets > 1 {
            stats.spread.mul_int(k - pellets / 2)
        } else if stats.spread != Fixed::ZERO {
            Fixed(s.rng.range_i32(-stats.spread.0, stats.spread.0))
        } else {
            Fixed::ZERO
        };
        let sh = &mut s.shots;
        sh.x[slot] = s.players.x[i] + ux * Fixed::HALF;
        sh.y[slot] = s.players.y[i] + uy * Fixed::HALF;
        sh.vx[slot] = ux * stats.speed + px * lateral;
        sh.vy[slot] = uy * stats.speed + py * lateral;
        sh.ttl[slot] = stats.ttl;
        sh.owner[slot] = i as u8;
        sh.damage[slot] = stats.damage;
        sh.kind[slot] = w as u8;
    }
    s.players.cooldown[i] = stats.cooldown;
    let (x, y) = (s.players.x[i], s.players.y[i]);
    s.emit(EventKind::Fire, w as u8, x, y);
}

fn update_revives(s: &mut State) {
    for d in 0..MAX_PLAYERS {
        if s.life(d) != Life::Downed {
            continue;
        }
        let helped = (0..MAX_PLAYERS).any(|h| {
            h != d
                && s.life(h) == Life::Alive
                && Fixed::len_sq(
                    s.players.x[h] - s.players.x[d],
                    s.players.y[h] - s.players.y[d],
                ) <= REVIVE_RANGE.sq()
        });
        if helped {
            s.players.revive[d] += 1;
            if s.players.revive[d] >= REVIVE_FRAMES {
                s.players.life[d] = Life::Alive as u8;
                s.players.hp[d] = REVIVE_HP;
                s.players.revive[d] = 0;
                if let Some(h) = (0..MAX_PLAYERS).find(|&h| h != d && s.life(h) == Life::Alive) {
                    s.players.revives_done[h] = s.players.revives_done[h].saturating_add(1);
                }
                let (x, y) = (s.players.x[d], s.players.y[d]);
                s.emit(EventKind::Revive, d as u8, x, y);
            }
        } else {
            s.players.revive[d] = 0;
        }
    }
}

fn nearest_alive_player(s: &State, x: Fixed, y: Fixed) -> Option<(usize, i64)> {
    (0..MAX_PLAYERS)
        .filter(|&i| s.life(i) == Life::Alive)
        .map(|i| (i, Fixed::len_sq(s.players.x[i] - x, s.players.y[i] - y)))
        .min_by_key(|&(i, d)| (d, i))
}

fn damage_player(s: &mut State, p: usize, amount: i32) {
    s.players.hp[p] -= amount;
    s.players.hurt_flash[p] = 8;
    let (x, y) = (s.players.x[p], s.players.y[p]);
    s.emit(EventKind::PlayerHurt, p as u8, x, y);
    if s.players.hp[p] <= 0 {
        s.players.hp[p] = 0;
        if present_count(s) <= 1 {
            s.players.life[p] = Life::Dead as u8; // solo: no one can revive
        } else {
            s.players.life[p] = Life::Downed as u8;
            s.players.down_timer[p] = DOWNED_FRAMES;
            s.players.revive[p] = 0;
        }
        s.emit(EventKind::Down, p as u8, x, y);
    }
}

fn update_enemies(s: &mut State, map: &MapData, grid: &EnemyGrid) {
    let speed_mul = Fixed::ONE + Fixed::ratio(2, 100).mul_int(s.wave.number.min(20) as i32);
    for e in 0..MAX_ENEMIES {
        if s.enemies.alive[e] == 0 {
            continue;
        }
        let stats = ENEMIES[usize::from(s.enemies.kind[e]).min(ENEMIES.len() - 1)];
        s.enemies.cooldown[e] = s.enemies.cooldown[e].saturating_sub(1);
        if s.enemies.stagger[e] > 0 {
            s.enemies.stagger[e] -= 1;
            continue;
        }
        let (ex, ey) = (s.enemies.x[e], s.enemies.y[e]);
        let Some((target, dist_sq)) = nearest_alive_player(s, ex, ey) else {
            continue;
        };

        // attack if in contact
        let reach = stats.radius + PLAYER_RADIUS + Fixed::ratio(5, 100);
        if dist_sq <= reach.sq() {
            if s.enemies.cooldown[e] == 0 {
                s.enemies.cooldown[e] = stats.attack_cooldown;
                damage_player(s, target, stats.damage);
            }
            continue;
        }

        // steer: flow field, or direct chase when sharing the target's tile / no path
        let (tx, ty) = (ex.floor_int(), ey.floor_int());
        let flow_dir = if map.solid(tx, ty) {
            0
        } else {
            s.flow.dir[ty as usize * crate::config::MAP_W + tx as usize]
        };
        // target point: the player once on its tile (flow 0), else the centre of the next flow tile.
        // Never chase in a straight line from further away — that walks into walls.
        // Steering at a tile centre (not along the quantized direction) re-centres bodies in
        // corridors so their boxes don't snag wall corners.
        let (goal_x, goal_y) = if flow_dir == 0 {
            (s.players.x[target], s.players.y[target])
        } else {
            let (ox, oy) = NEIGHBOUR[usize::from(flow_dir)];
            (
                Fixed::from_int(tx + ox) + Fixed::HALF,
                Fixed::from_int(ty + oy) + Fixed::HALF,
            )
        };
        let speed = stats.speed * speed_mul;
        let (mut vx, mut vy) = scale_to(goal_x - ex, goal_y - ey, speed);

        // separation (Boids-lite), bounded samples
        let mut sampled = 0;
        let min_d = stats.radius.mul_int(2);
        grid.for_each_near(tx, ty, |o| {
            if o == e {
                return true;
            }
            let (dx, dy) = (ex - s.enemies.x[o], ey - s.enemies.y[o]);
            if Fixed::len_sq(dx, dy) < min_d.sq() {
                // normalize by L1 norm to avoid sqrt; exact zero overlap pushes along slot order
                let l1 = dx.abs() + dy.abs();
                if l1 == Fixed::ZERO {
                    vx += if e < o {
                        -stats.separation
                    } else {
                        stats.separation
                    };
                } else {
                    vx += (dx / l1) * stats.separation;
                    vy += (dy / l1) * stats.separation;
                }
                sampled += 1;
            }
            sampled < SEPARATION_SAMPLES
        });

        let max_step = Fixed::ratio(45, 100);
        vx = vx.clamp(-max_step, max_step);
        vy = vy.clamp(-max_step, max_step);
        let (nx, ny) = move_box(map, ex, ey, vx, vy, stats.radius);
        s.enemies.x[e] = nx;
        s.enemies.y[e] = ny;
    }
}

/// Vector `(dx, dy)` rescaled to length `len` (integer sqrt; zero stays zero).
fn scale_to(dx: Fixed, dy: Fixed, len: Fixed) -> (Fixed, Fixed) {
    let mag = crate::fx::isqrt_u64(Fixed::len_sq(dx, dy) as u64) as i64; // Q16.16
    if mag == 0 {
        return (Fixed::ZERO, Fixed::ZERO);
    }
    let k = i64::from(len.0);
    (
        Fixed((i64::from(dx.0) * k / mag) as i32),
        Fixed((i64::from(dy.0) * k / mag) as i32),
    )
}

fn update_shots(s: &mut State, map: &MapData, grid: &EnemyGrid) {
    for j in 0..MAX_SHOTS {
        if s.shots.ttl[j] == 0 {
            continue;
        }
        s.shots.ttl[j] -= 1;
        // two substeps keep per-substep motion under half a tile
        for _ in 0..2 {
            let x = s.shots.x[j] + Fixed(s.shots.vx[j].0 / 2);
            let y = s.shots.y[j] + Fixed(s.shots.vy[j].0 / 2);
            s.shots.x[j] = x;
            s.shots.y[j] = y;
            if map.solid(x.floor_int(), y.floor_int()) {
                s.shots.ttl[j] = 0;
                break;
            }
            let mut hit = None;
            grid.for_each_near(x.floor_int(), y.floor_int(), |e| {
                if s.enemies.alive[e] == 0 {
                    return true;
                }
                let r = ENEMIES[usize::from(s.enemies.kind[e]).min(ENEMIES.len() - 1)].radius
                    + SHOT_RADIUS;
                if Fixed::len_sq(s.enemies.x[e] - x, s.enemies.y[e] - y) <= r.sq() {
                    hit = Some(e);
                    return false;
                }
                true
            });
            if let Some(e) = hit {
                hit_enemy(s, map, e, j);
                s.shots.ttl[j] = 0;
                break;
            }
        }
    }
}

fn hit_enemy(s: &mut State, map: &MapData, e: usize, shot: usize) {
    let owner = usize::from(s.shots.owner[shot]).min(MAX_PLAYERS - 1);
    let kind = usize::from(s.enemies.kind[e]).min(ENEMIES.len() - 1);
    let stats = ENEMIES[kind];
    s.enemies.hp[e] -= i32::from(s.shots.damage[shot]);
    let (x, y) = (s.enemies.x[e], s.enemies.y[e]);
    if s.enemies.hp[e] <= 0 {
        s.enemies.alive[e] = 0;
        s.wave.enemies_alive = s.wave.enemies_alive.saturating_sub(1);
        s.players.kills[owner] = s.players.kills[owner].saturating_add(1);
        s.wave.combo = s.wave.combo.saturating_add(1);
        s.wave.best_combo = s.wave.best_combo.max(s.wave.combo);
        s.wave.combo_timer = COMBO_FRAMES;
        let mult_x10 = 10 + s.wave.combo.min(COMBO_CAP);
        s.wave.team_score = s
            .wave
            .team_score
            .saturating_add(stats.score * mult_x10 / 10);
        s.emit(EventKind::Kill, kind as u8, x, y);
        unlock_weapons(s);
    } else {
        let dir = dir_from_delta(s.shots.vx[shot], s.shots.vy[shot]);
        let (ux, uy) = DIR8[usize::from(dir)];
        let (nx, ny) = move_box(map, x, y, ux * KNOCKBACK, uy * KNOCKBACK, stats.radius);
        s.enemies.x[e] = nx;
        s.enemies.y[e] = ny;
        s.enemies.stagger[e] = STAGGER_FRAMES;
        s.emit(EventKind::Hit, kind as u8, x, y);
    }
}

fn unlock_weapons(s: &mut State) {
    for (w, stats) in WEAPONS.iter().enumerate() {
        let bit = 1u32 << w;
        if s.wave.unlocked & bit == 0 && s.wave.team_score >= stats.unlock_score {
            s.wave.unlocked |= bit;
            s.emit(EventKind::Unlock, w as u8, Fixed::ZERO, Fixed::ZERO);
        }
    }
}

fn update_combo(s: &mut State) {
    if s.wave.combo_timer > 0 {
        s.wave.combo_timer -= 1;
        if s.wave.combo_timer == 0 {
            s.wave.combo = 0;
        }
    }
}

fn visible_to_any(s: &State, tx: i32, ty: i32) -> bool {
    (0..MAX_PLAYERS)
        .filter(|&i| matches!(s.life(i), Life::Alive | Life::Downed))
        .any(|i| {
            let (px, py) = (s.players.x[i].floor_int(), s.players.y[i].floor_int());
            (tx - px).abs() <= VIEW_HALF_W && (ty - py).abs() <= VIEW_HALF_H
        })
}

fn update_waves(s: &mut State, map: &MapData) {
    match s.phase() {
        WavePhase::Intermission => {
            s.wave.timer = s.wave.timer.saturating_sub(1);
            if s.wave.timer == 0 {
                start_wave(s, map);
            }
        }
        WavePhase::Active => {
            if s.wave.to_spawn > 0 {
                s.wave.timer = s.wave.timer.saturating_sub(1);
                if s.wave.timer == 0 {
                    // early waves trickle in; later waves flood (floor at SPAWN_INTERVAL)
                    s.wave.timer = 48u32
                        .saturating_sub(s.wave.number.saturating_mul(4))
                        .max(SPAWN_INTERVAL);
                    spawn_one(s, map);
                }
            } else if s.wave.enemies_alive == 0 {
                s.wave.phase = WavePhase::Intermission as u32;
                s.wave.timer = INTERMISSION_FRAMES;
            }
        }
        WavePhase::GameOver => {}
    }
}

fn start_wave(s: &mut State, map: &MapData) {
    s.wave.number = s.wave.number.saturating_add(1);
    s.wave.phase = WavePhase::Active as u32;
    s.wave.timer = 1;
    let players = present_count(s).min(MAX_PLAYERS as u32) as usize;
    let base = 8 + s.wave.number.saturating_mul(4);
    s.wave.to_spawn = base.saturating_mul(PLAYER_COUNT_MUL_X100[players]) / 100;
    // dead players return with partial HP
    for i in 0..MAX_PLAYERS {
        if s.life(i) == Life::Dead {
            let (tx, ty) = map.starts[i];
            s.players.life[i] = Life::Alive as u8;
            s.players.hp[i] = RESPAWN_HP;
            s.players.x[i] = Fixed::from_int(i32::from(tx)) + Fixed::HALF;
            s.players.y[i] = Fixed::from_int(i32::from(ty)) + Fixed::HALF;
        }
    }
    let n = s.wave.number.min(255) as u8;
    s.emit(EventKind::WaveStart, n, Fixed::ZERO, Fixed::ZERO);
}

fn spawn_one(s: &mut State, map: &MapData) {
    if map.spawn_count == 0 {
        s.wave.to_spawn = 0;
        return;
    }
    let Some(slot) = (0..MAX_ENEMIES).find(|&e| s.enemies.alive[e] == 0) else {
        return;
    }; // full: retry next interval

    // candidates outside every camera; if none, the door farthest from all players
    let offset = s.rng.below(map.spawn_count as u32) as usize;
    let door = (0..map.spawn_count)
        .map(|k| (k + offset) % map.spawn_count)
        .find(|&k| !visible_to_any(s, i32::from(map.spawns[k].0), i32::from(map.spawns[k].1)))
        .unwrap_or_else(|| {
            (0..map.spawn_count)
                .max_by_key(|&k| {
                    let (dx, dy) = (i32::from(map.spawns[k].0), i32::from(map.spawns[k].1));
                    let nearest = (0..MAX_PLAYERS)
                        .filter(|&i| s.life(i) == Life::Alive)
                        .map(|i| {
                            let (px, py) = (s.players.x[i].floor_int(), s.players.y[i].floor_int());
                            (dx - px).abs() + (dy - py).abs()
                        })
                        .min()
                        .unwrap_or(0);
                    (nearest, core::cmp::Reverse(k))
                })
                .unwrap_or(0)
        });

    let w = s.wave.number;
    let walker_w = 10u32.saturating_sub(w).max(3);
    let runner_w = if w >= 3 { w.min(8) } else { 0 };
    let kind = if s.rng.below(walker_w + runner_w) < walker_w {
        EnemyKind::Walker
    } else {
        EnemyKind::Runner
    };
    let stats = ENEMIES[kind as usize];

    let (tx, ty) = map.spawns[door];
    s.enemies.x[slot] = Fixed::from_int(i32::from(tx)) + Fixed::HALF;
    s.enemies.y[slot] = Fixed::from_int(i32::from(ty)) + Fixed::HALF;
    s.enemies.hp[slot] = stats.hp;
    s.enemies.kind[slot] = kind as u8;
    s.enemies.alive[slot] = 1;
    s.enemies.cooldown[slot] = stats.attack_cooldown;
    s.enemies.stagger[slot] = 0;
    s.wave.enemies_alive += 1;
    s.wave.to_spawn -= 1;
}

fn check_game_over(s: &mut State) {
    let anyone_standing = (0..MAX_PLAYERS).any(|i| matches!(s.life(i), Life::Alive | Life::Downed));
    if !anyone_standing {
        s.wave.phase = WavePhase::GameOver as u32;
        s.emit(EventKind::GameOver, 0, Fixed::ZERO, Fixed::ZERO);
    }
}

//! Hide & seek mode: one seeker finds hiders by sight. Reuses the survival map and player
//! slots but none of its weapons/wave/HP logic. Dispatched from `World::step` when
//! `state.mode() == GameMode::HideSeek` (docs/exec-plans/active/0003-hide-and-seek-mode.md).

use crate::config::{MAX_PLAYERS, PLAYER_RADIUS, PLAYER_SPEED};
use crate::fx::Fixed;
use crate::input::{PlayerInput, DIR8};
use crate::map::MapData;
use crate::physics::move_box;
use crate::state::{EventKind, HideSeekPhase, Life, State};

/// Prep countdown before the seeker can move or see (frames). Gives hiders time to scatter.
pub const PREP_FRAMES: u32 = 5 * 60;
/// Round length once seeking starts (frames).
pub const ROUND_FRAMES: u32 = 90 * 60;
/// Seeker vision distance (tiles).
pub const VISION_RANGE: Fixed = Fixed::from_int(8);
/// cos(half the vision cone angle). ~60° half-angle: a wide but not omniscient cone.
pub const VISION_COS_HALF: Fixed = Fixed::ratio(50, 100);
/// Consecutive frames a hider must stay in sight before being caught.
pub const SPOT_FRAMES: u16 = 45;

/// Set up roles/timers for a fresh round. Player positions are already placed at map starts
/// by `World::initial_state`.
pub fn init(s: &mut State) {
    let n = s.header.num_players as usize;
    for i in 0..n {
        s.hide_seek.role[i] = u8::from(i == 0); // player 0 seeks first round
    }
    s.hide_seek.phase = HideSeekPhase::Hiding as u32;
    s.hide_seek.timer = PREP_FRAMES;
}

/// Advance one frame. Movement-only: no weapons, HP, waves or enemies.
pub fn step(s: &mut State, map: &MapData, inputs: [PlayerInput; MAX_PLAYERS]) {
    let n = s.header.num_players as usize;
    let seeking = s.hide_seek.phase == HideSeekPhase::Seeking as u32;
    let over = s.hide_seek.phase == HideSeekPhase::RoundOver as u32;

    for (i, input) in inputs.iter().enumerate().take(n) {
        if s.life(i) != Life::Alive {
            continue;
        }
        let is_seeker = s.hide_seek.role[i] == 1;
        if over || (is_seeker && !seeking) {
            continue; // seeker frozen during prep; nobody moves once the round is decided
        }
        let dir = input.dir();
        if dir != 0 {
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
    }

    if over {
        return;
    }

    s.hide_seek.timer = s.hide_seek.timer.saturating_sub(1);
    if !seeking {
        if s.hide_seek.timer == 0 {
            s.hide_seek.phase = HideSeekPhase::Seeking as u32;
            s.hide_seek.timer = ROUND_FRAMES;
        }
        return;
    }

    let Some(seeker) = (0..n).find(|&i| s.hide_seek.role[i] == 1 && s.life(i) == Life::Alive)
    else {
        return;
    };
    let (sx, sy) = (s.players.x[seeker], s.players.y[seeker]);
    let facing = usize::from(s.players.facing[seeker].clamp(1, 8));
    let (fdx, fdy) = DIR8[facing];

    let mut all_found = true;
    for i in 0..n {
        if i == seeker || s.life(i) != Life::Alive {
            continue;
        }
        if s.hide_seek.found[i] == 1 {
            continue;
        }
        let seen = can_see(map, sx, sy, fdx, fdy, s.players.x[i], s.players.y[i]);
        if seen {
            s.hide_seek.spot_frames[i] = s.hide_seek.spot_frames[i].saturating_add(1);
            if s.hide_seek.spot_frames[i] >= SPOT_FRAMES {
                s.hide_seek.found[i] = 1;
                let (x, y) = (s.players.x[i], s.players.y[i]);
                s.emit(EventKind::Found, i as u8, x, y);
            }
        } else {
            s.hide_seek.spot_frames[i] = 0;
        }
        // Checked after the possible catch above so the win check sees this frame's result,
        // not last frame's — otherwise the last capture always lags a frame behind.
        if s.hide_seek.found[i] == 0 {
            all_found = false;
        }
    }

    if all_found {
        end_round(s, 2); // seeker wins
    } else if s.hide_seek.timer == 0 {
        end_round(s, 1); // hiders win
    }
}

fn end_round(s: &mut State, winner: u32) {
    s.hide_seek.phase = HideSeekPhase::RoundOver as u32;
    s.hide_seek.winner = winner;
    s.emit(EventKind::RoundEnd, winner as u8, Fixed::ZERO, Fixed::ZERO);
}

/// Whether a hider at `(hx, hy)` is inside the seeker's vision cone with a clear line of sight.
/// Pure integer math (`i128` for the cone comparison): no floats, deterministic across builds.
fn can_see(
    map: &MapData,
    sx: Fixed,
    sy: Fixed,
    fdx: Fixed,
    fdy: Fixed,
    hx: Fixed,
    hy: Fixed,
) -> bool {
    let (dx, dy) = (hx - sx, hy - sy);
    let dist_sq = Fixed::len_sq(dx, dy);
    if dist_sq > VISION_RANGE.sq() {
        return false;
    }
    if dist_sq == 0 {
        return true;
    }
    // `fdx/fdy` is a unit vector, so `dot == |d| * cos(theta)` in the same Q16.16 "tiles" scale
    // as `dx/dy` — comparing its square against `cos_half^2 * dist_sq` avoids a sqrt entirely.
    let dot = fdx * dx + fdy * dy;
    if dot.0 <= 0 {
        return false;
    }
    let dot_sq = dot.sq();
    let cos_sq = VISION_COS_HALF.sq();
    let rhs = ((cos_sq as i128 * dist_sq as i128) >> (Fixed::FRAC_BITS * 2)) as i64;
    if dot_sq < rhs {
        return false;
    }
    line_clear(
        map,
        sx.floor_int(),
        sy.floor_int(),
        hx.floor_int(),
        hy.floor_int(),
    )
}

/// Bresenham line: true if no wall tile lies strictly between (exclusive) the two endpoints.
fn line_clear(map: &MapData, mut x0: i32, mut y0: i32, x1: i32, y1: i32) -> bool {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut first = true;
    loop {
        if x0 == x1 && y0 == y1 {
            return true;
        }
        if !first && map.solid(x0, y0) {
            return false;
        }
        first = false;
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::step::World;

    fn stepped(
        mode: u8,
        n: u8,
        frames: u32,
        inputs: impl Fn(u32) -> [PlayerInput; MAX_PLAYERS],
    ) -> Box<State> {
        let world = World::new(0);
        let mut s = world.initial_state(n, 1, mode);
        let mut w = World::new(0);
        for f in 0..frames {
            w.step(&mut s, &inputs(f));
        }
        s
    }

    #[test]
    fn when_seeker_never_sees_hider_then_hiders_win_at_timeout() {
        // Nobody moves; players start far apart on the yard map. Round should time out.
        let s = stepped(1, 2, PREP_FRAMES + ROUND_FRAMES, |_| {
            [PlayerInput::default(); MAX_PLAYERS]
        });
        assert_eq!(s.hide_seek.phase, HideSeekPhase::RoundOver as u32);
        assert_eq!(s.hide_seek.winner, 1, "hiders should win by timeout");
    }

    #[test]
    fn when_seeker_faces_and_holds_sight_on_adjacent_hider_then_found() {
        let mut s = {
            let world = World::new(0);
            world.initial_state(2, 1, 1)
        };
        // Place hider directly south of the seeker, well within range, no wall between.
        s.players.x[1] = s.players.x[0];
        s.players.y[1] = s.players.y[0] + Fixed::from_int(2);
        s.players.facing[0] = 5; // south, matches DIR8 index for (0,+1)

        let mut w = World::new(0);
        // Run past the prep phase first (seeker frozen/blind during Hiding).
        for _ in 0..PREP_FRAMES {
            w.step(&mut s, &[PlayerInput::default(); MAX_PLAYERS]);
        }
        assert_eq!(s.hide_seek.phase, HideSeekPhase::Seeking as u32);
        for _ in 0..u32::from(SPOT_FRAMES) {
            w.step(&mut s, &[PlayerInput::default(); MAX_PLAYERS]);
        }
        assert_eq!(
            s.hide_seek.found[1], 1,
            "hider in plain sight should be caught"
        );
        assert_eq!(
            s.hide_seek.winner, 2,
            "seeker should win once everyone is found"
        );
    }

    #[test]
    fn when_wall_blocks_line_of_sight_then_never_found() {
        let mut s = {
            let world = World::new(0);
            world.initial_state(2, 1, 1)
        };
        // Row y=4 of the yard map is "#...#....": a wall column sits at tile x=4, with open
        // floor on both sides. Put the seeker left of it, the hider on the far side.
        s.players.x[0] = Fixed::from_int(2) + Fixed::HALF;
        s.players.y[0] = Fixed::from_int(4) + Fixed::HALF;
        s.players.x[1] = Fixed::from_int(6) + Fixed::HALF;
        s.players.y[1] = Fixed::from_int(4) + Fixed::HALF;
        s.players.facing[0] = 3; // east

        let mut w = World::new(0);
        for _ in 0..PREP_FRAMES {
            w.step(&mut s, &[PlayerInput::default(); MAX_PLAYERS]);
        }
        for _ in 0..u32::from(SPOT_FRAMES) * 2 {
            w.step(&mut s, &[PlayerInput::default(); MAX_PLAYERS]);
        }
        assert_eq!(
            s.hide_seek.found[1], 0,
            "wall should block the seeker's sight"
        );
    }

    #[test]
    fn when_same_seed_and_inputs_then_checksums_match_every_frame() {
        // GGRS rolls back by re-simulating from a saved State; that's only safe if hide_seek::step
        // is exactly as deterministic as the survival path (docs/design-docs/core-beliefs.md #2).
        fn run(seed: u32, frames: u32) -> Vec<u64> {
            let world = World::new(0);
            let mut s = world.initial_state(2, seed, 1);
            let mut w = World::new(0);
            let mut rng = crate::rng::Rng::from_seed(seed ^ 0xBEEF);
            let mut sums = Vec::with_capacity(frames as usize);
            for _ in 0..frames {
                let mut inputs = [PlayerInput::default(); MAX_PLAYERS];
                for p in inputs.iter_mut().take(2) {
                    let bits = (rng.next_u32() % 9) as u16;
                    *p = PlayerInput(bits | PlayerInput::PRESENT);
                }
                w.step(&mut s, &inputs);
                sums.push(s.checksum());
            }
            sums
        }
        let a = run(42, PREP_FRAMES + ROUND_FRAMES);
        let b = run(42, PREP_FRAMES + ROUND_FRAMES);
        assert_eq!(a, b);
    }
}

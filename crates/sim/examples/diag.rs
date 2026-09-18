//! Headless diagnostic: prints wave/enemy/player stats while bots play.
#![allow(clippy::float_arithmetic, clippy::cast_precision_loss)]
#[path = "../tests/common/mod.rs"]
mod common;
use hg_sim::rng::Rng;
use hg_sim::World;

fn main() {
    let players: u8 = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(2);
    let mut world = World::new(0);
    let mut s = world.initial_state(players, 42, 0);
    let mut rng = Rng::from_seed(0x2A ^ 0xABCD);
    let mut held = [0u16; 4];
    for f in 0..60 * 60 * 3u32 {
        let _ = (&mut rng, &mut held);
        let inputs = common::kiting_inputs(&s, usize::from(players));
        world.step(&mut s, &inputs);
        if f == 60 * 60 {
            dump(&s);
        }
        if f % 600 == 0 {
            let alive_e = s.enemies.alive.iter().filter(|&&a| a != 0).count();
            println!(
                "t={:>4}s wave={} phase={} to_spawn={} alive_e={} cached={} score={} kills={:?} life={:?} hp={:?} pos=({:.1},{:.1})",
                f / 60, s.wave.number, s.wave.phase, s.wave.to_spawn, alive_e, s.wave.enemies_alive, s.wave.team_score,
                &s.players.kills[..2], &s.players.life[..2], &s.players.hp[..2],
                s.players.x[0].0 >> 16, s.players.y[0].0 >> 16
            );
        }
    }
}

#[allow(dead_code)]
fn dump(s: &hg_sim::State) {
    for e in 0..hg_sim::config::MAX_ENEMIES {
        if s.enemies.alive[e] != 0 {
            let (tx, ty) = (s.enemies.x[e].floor_int(), s.enemies.y[e].floor_int());
            let fd = if tx >= 0 && ty >= 0 {
                s.flow.dir[ty as usize * 40 + tx as usize]
            } else {
                99
            };
            println!(
                "enemy {e} kind {} at ({:.2},{:.2}) tile ({tx},{ty}) flow {fd} stagger {}",
                s.enemies.kind[e],
                s.enemies.x[e].0 as f64 / 65536.0,
                s.enemies.y[e].0 as f64 / 65536.0,
                s.enemies.stagger[e]
            );
        }
    }
    println!(
        "p0 ({:.2},{:.2}) p1 ({:.2},{:.2})",
        s.players.x[0].0 as f64 / 65536.0,
        s.players.y[0].0 as f64 / 65536.0,
        s.players.x[1].0 as f64 / 65536.0,
        s.players.y[1].0 as f64 / 65536.0
    );
}

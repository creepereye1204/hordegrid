//! Worst-case step cost: every enemy and shot slot in use (docs/QUALITY_SCORE.md#성능-예산).
#![allow(
    clippy::float_arithmetic,
    clippy::cast_precision_loss,
    clippy::disallowed_types
)]

use hg_sim::config::{MAX_ENEMIES, MAX_SHOTS};
use hg_sim::state::WavePhase;
use hg_sim::{Fixed, PlayerInput, World};
use std::time::Instant;

fn main() {
    let mut world = World::new(0);
    let base = world.initial_state(4, 1, 0);
    let mut full = base.clone();
    let mut k = 0;
    for e in 0..MAX_ENEMIES {
        // scatter on floor tiles
        loop {
            k += 1;
            let (tx, ty) = ((k * 7) % 38 + 1, (k * 13) % 28 + 1);
            if !world.map.solid(tx, ty) {
                full.enemies.x[e] = Fixed::from_int(tx) + Fixed::HALF;
                full.enemies.y[e] = Fixed::from_int(ty) + Fixed::HALF;
                break;
            }
        }
        full.enemies.hp[e] = 100_000;
        full.enemies.alive[e] = 1;
    }
    full.wave.enemies_alive = MAX_ENEMIES as u32;
    full.wave.phase = WavePhase::Active as u32;
    let fire = [PlayerInput(PlayerInput::PRESENT | PlayerInput::FIRE | 3); 4];
    let mut samples = Vec::new();
    for _ in 0..200 {
        let mut s = full.clone();
        for j in 0..MAX_SHOTS {
            s.shots.ttl[j] = 20;
            s.shots.x[j] = s.enemies.x[j % MAX_ENEMIES];
            s.shots.y[j] = s.enemies.y[j % MAX_ENEMIES] - Fixed::ONE;
            s.shots.vy[j] = Fixed::ratio(3, 10);
            s.shots.damage[j] = 1;
        }
        for _ in 0..9 {
            let t = Instant::now();
            world.step(&mut s, &fire);
            samples.push(t.elapsed().as_secs_f64() * 1e3);
        }
    }
    samples.sort_by(f64::total_cmp);
    let p = |q: f64| samples[((samples.len() as f64 - 1.0) * q) as usize];
    println!(
        "step_full_horde native ms: p50 {:.3} p99 {:.3} max {:.3}",
        p(0.5),
        p(0.99),
        p(1.0)
    );
    println!("rollback_9_steps p99 ≈ {:.3} ms", p(0.99) * 9.0);
    println!(
        "State size: {} bytes",
        core::mem::size_of::<hg_sim::State>()
    );
}

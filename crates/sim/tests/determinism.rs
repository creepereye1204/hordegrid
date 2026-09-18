//! Replay & snapshot determinism (docs/design-docs/determinism.md §검증).

mod common;

use common::{kiting_inputs, random_inputs};
use hg_sim::config::MAX_PLAYERS;
use hg_sim::rng::Rng;
use hg_sim::state::{Life, WavePhase};
use hg_sim::{PlayerInput, State, World};

fn run(players: u8, seed: u32, frames: u32) -> (Vec<u64>, Box<State>) {
    let mut world = World::new(0);
    let mut state = world.initial_state(players, seed, 0);
    let mut rng = Rng::from_seed(seed ^ 0xABCD);
    let mut held = [0u16; MAX_PLAYERS];
    let mut sums = Vec::with_capacity(frames as usize);
    for _ in 0..frames {
        let inputs = random_inputs(&mut rng, &mut held, usize::from(players));
        world.step(&mut state, &inputs);
        sums.push(state.checksum());
    }
    (sums, state)
}

#[test]
fn when_same_seed_and_inputs_then_identical_checksums_every_frame() {
    let (a, _) = run(4, 1234, 6_000);
    let (b, _) = run(4, 1234, 6_000);
    assert_eq!(a, b);
}

#[test]
fn when_different_seed_then_states_diverge() {
    let (a, _) = run(2, 1, 1_000);
    let (b, _) = run(2, 2, 1_000);
    assert_ne!(a.last(), b.last());
}

#[test]
fn when_restoring_snapshot_then_replay_matches() {
    let seed = 99;
    let mut world = World::new(0);
    let mut state = world.initial_state(3, seed, 0);
    let mut rng = Rng::from_seed(7);
    let mut held = [0u16; MAX_PLAYERS];
    let mut log = Vec::new();
    for _ in 0..1_500 {
        let inputs = random_inputs(&mut rng, &mut held, 3);
        log.push(inputs);
        world.step(&mut state, &inputs);
    }
    let snapshot: State = *state;
    let mut forward = Vec::new();
    for _ in 0..600 {
        let inputs = random_inputs(&mut rng, &mut held, 3);
        log.push(inputs);
        world.step(&mut state, &inputs);
        forward.push(state.checksum());
    }
    // "rollback": a fresh World (fresh scratch buffers) replays from the snapshot
    let mut world2 = World::new(0);
    let mut restored = Box::new(snapshot);
    let replayed: Vec<u64> = log[1_500..]
        .iter()
        .map(|inputs| {
            world2.step(&mut restored, inputs);
            restored.checksum()
        })
        .collect();
    assert_eq!(forward, replayed);
}

#[test]
fn when_kiting_bots_play_then_game_progresses() {
    let mut world = World::new(0);
    let mut state = world.initial_state(2, 42, 0);
    for _ in 0..60 * 60 * 3 {
        let inputs = kiting_inputs(&state, 2);
        world.step(&mut state, &inputs);
    }
    let kills: u32 = state.players.kills.iter().sum();
    assert!(
        state.wave.number >= 3,
        "wave {} kills {kills} phase {}",
        state.wave.number,
        state.wave.phase
    );
    assert!(kills > 20, "kills {kills}");
    assert!(
        state.wave.unlocked & 0b10 != 0,
        "smg unlocked at score {}",
        state.wave.team_score
    );
}

#[test]
fn when_player_absent_long_enough_then_gone() {
    let mut world = World::new(0);
    let mut state = world.initial_state(2, 5, 0);
    let present = PlayerInput(PlayerInput::PRESENT);
    for _ in 0..200 {
        world.step(
            &mut state,
            &[
                present,
                PlayerInput::default(),
                PlayerInput::default(),
                PlayerInput::default(),
            ],
        );
    }
    assert_eq!(state.life(0), Life::Alive);
    assert_eq!(state.life(1), Life::Gone);
}

#[test]
fn when_solo_player_idles_then_game_over_eventually() {
    let mut world = World::new(0);
    let mut state = world.initial_state(1, 3, 0);
    let idle = [
        PlayerInput(PlayerInput::PRESENT),
        PlayerInput::default(),
        PlayerInput::default(),
        PlayerInput::default(),
    ];
    for _ in 0..60 * 60 * 3 {
        world.step(&mut state, &idle);
        if state.phase() == WavePhase::GameOver {
            break;
        }
    }
    assert_eq!(state.phase(), WavePhase::GameOver);
}

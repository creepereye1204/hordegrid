//! GGRS `SyncTestSession`: forces a rollback of `check_distance` frames every frame and compares checksums.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ggrs::{GgrsRequest, PlayerType, SessionBuilder};
use hg_net::session::HgConfig;
use hg_net::NetInput;
use hg_sim::config::MAX_PLAYERS;
use hg_sim::rng::Rng;
use hg_sim::{PlayerInput, World};

#[test]
fn when_rolling_back_every_frame_then_checksums_match() {
    let players = 3usize;
    let mut builder = SessionBuilder::<HgConfig>::new()
        .with_num_players(players)
        .unwrap()
        .with_check_distance(7);
    for h in 0..players {
        builder = builder.add_player(PlayerType::Local, h).unwrap();
    }
    let mut session = builder.start_synctest_session().unwrap();
    let mut world = World::new(0);
    let mut state = world.initial_state(players as u8, 555, 0);
    let mut rng = Rng::from_seed(9);
    let mut held = [0u16; MAX_PLAYERS];

    for _ in 0..1_200 {
        for (h, bits) in held.iter_mut().enumerate().take(players) {
            if rng.below(12) == 0 {
                *bits = rng.below(9) as u16 | PlayerInput::FIRE | PlayerInput::PRESENT;
            }
            session.add_local_input(h, NetInput(*bits)).unwrap();
        }
        let requests = session
            .advance_frame()
            .unwrap_or_else(|e| panic!("synctest failed: {e}"));
        for req in requests {
            match req {
                GgrsRequest::SaveGameState { cell, frame } => {
                    cell.save(frame, Some(*state), Some(u128::from(state.checksum())));
                }
                GgrsRequest::LoadGameState { cell, .. } => *state = cell.load().unwrap(),
                GgrsRequest::AdvanceFrame { inputs } => {
                    let mut f = [PlayerInput::default(); MAX_PLAYERS];
                    for (slot, (i, _)) in f.iter_mut().zip(inputs) {
                        *slot = i.into();
                    }
                    world.step(&mut state, &f);
                }
            }
        }
    }
    assert!(state.header.frame >= 1_200);
}

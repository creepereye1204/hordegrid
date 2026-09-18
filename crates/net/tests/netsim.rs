//! In-process network simulator: N GGRS peers over a lossy, laggy virtual network.
//! Verifies that confirmed-frame checksums agree across peers (docs/RELIABILITY.md §네트워크 시뮬레이터).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::disallowed_types
)] // test harness measures real time

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use hg_net::{NetEvent, NetSettings, Runner, TickStatus};
use hg_sim::rng::Rng;
use hg_sim::PlayerInput;

#[derive(Clone, Copy)]
struct Link {
    base_ms: u64,
    jitter_ms: u64,
    loss_per_mille: u32,
    dup_per_mille: u32,
}

struct InFlight {
    deliver_at: Instant,
    to: usize,
    from: u8,
    bytes: Vec<u8>,
}

struct Outcome {
    frames: Vec<i32>,
    desyncs: usize,
    compared_frames: usize,
}

fn simulate(players: u8, link: Link, target_frames: i32, seed: u32, mode: u8) -> Outcome {
    let mut peers: Vec<Runner> = (0..players)
        .map(|h| {
            let mut r =
                Runner::new(players, h, 0x00C0_FFEE, 0, mode, NetSettings::default()).unwrap();
            r.record_checksums(true);
            r
        })
        .collect();
    let mut net_rng = Rng::from_seed(seed);
    let mut input_rng: Vec<Rng> = (0..players)
        .map(|h| Rng::from_seed(seed.wrapping_add(u32::from(h) * 7919)))
        .collect();
    let mut held = vec![0u16; usize::from(players)];
    let mut flight: Vec<InFlight> = Vec::new();
    let mut desyncs = 0;
    let start = Instant::now();
    let step = Duration::from_micros(16_667);
    let mut next_tick = start;

    while peers.iter().map(Runner::confirmed_frame).min().unwrap_or(0) < target_frames {
        assert!(
            start.elapsed() < Duration::from_secs(60),
            "netsim timed out"
        );
        let now = Instant::now();

        // deliver
        let (ready, pending): (Vec<_>, Vec<_>) =
            flight.into_iter().partition(|p| p.deliver_at <= now);
        flight = pending;
        for p in ready {
            peers[p.to].push_packet(p.from, &p.bytes);
        }

        if now >= next_tick {
            next_tick += step;
            for (h, peer) in peers.iter_mut().enumerate() {
                let rng = &mut input_rng[h];
                if rng.below(15) == 0 {
                    held[h] = rng.below(9) as u16
                        | if rng.below(3) == 0 {
                            0
                        } else {
                            PlayerInput::FIRE
                        };
                }
                let _status: TickStatus = peer.tick(PlayerInput(held[h]));
                for ev in peer.take_events() {
                    if matches!(ev, NetEvent::Desync { .. }) {
                        desyncs += 1;
                    }
                }
                for (slot, bytes) in peer.take_outbox() {
                    if net_rng.below(1000) < link.loss_per_mille {
                        continue;
                    }
                    let copies = if net_rng.below(1000) < link.dup_per_mille {
                        2
                    } else {
                        1
                    };
                    for _ in 0..copies {
                        let jitter = if link.jitter_ms == 0 {
                            0
                        } else {
                            u64::from(net_rng.below(link.jitter_ms as u32 * 2 + 1))
                        };
                        let delay = (link.base_ms / 2 + jitter).saturating_sub(link.jitter_ms); // one-way
                        flight.push(InFlight {
                            deliver_at: now + Duration::from_millis(delay),
                            to: usize::from(slot.0),
                            from: h as u8,
                            bytes: bytes.clone(),
                        });
                    }
                }
            }
        }
        std::thread::sleep(Duration::from_micros(500));
    }

    // Compare the LAST saved checksum per frame (re-saves after rollback overwrite predictions)
    let confirmed = peers.iter().map(Runner::confirmed_frame).min().unwrap_or(0);
    let logs: Vec<BTreeMap<i32, u64>> = peers
        .iter()
        .map(|p| p.checksum_log.iter().copied().collect())
        .collect();
    let mut compared = 0;
    for frame in 1..confirmed {
        let sums: Vec<_> = logs.iter().filter_map(|l| l.get(&frame)).collect();
        if sums.len() == logs.len() {
            assert!(
                sums.windows(2).all(|w| w[0] == w[1]),
                "checksum mismatch at frame {frame}: {sums:?}"
            );
            compared += 1;
        }
    }
    Outcome {
        frames: peers.iter().map(Runner::confirmed_frame).collect(),
        desyncs,
        compared_frames: compared,
    }
}

#[test]
fn when_two_peers_on_lan_then_confirmed_states_match() {
    let out = simulate(
        2,
        Link {
            base_ms: 2,
            jitter_ms: 0,
            loss_per_mille: 0,
            dup_per_mille: 0,
        },
        240,
        1,
        0,
    );
    assert_eq!(out.desyncs, 0);
    assert!(
        out.compared_frames > 200,
        "compared {} frames {:?}",
        out.compared_frames,
        out.frames
    );
}

#[test]
fn when_four_peers_on_mobile_network_then_confirmed_states_match() {
    let out = simulate(
        4,
        Link {
            base_ms: 150,
            jitter_ms: 30,
            loss_per_mille: 30,
            dup_per_mille: 10,
        },
        240,
        2,
        0,
    );
    assert_eq!(out.desyncs, 0);
    assert!(
        out.compared_frames > 150,
        "compared {} frames {:?}",
        out.compared_frames,
        out.frames
    );
}

#[test]
#[ignore = "long: run with `cargo test -p hg-net -- --ignored`"]
fn when_hostile_network_then_still_consistent() {
    let out = simulate(
        4,
        Link {
            base_ms: 250,
            jitter_ms: 80,
            loss_per_mille: 100,
            dup_per_mille: 20,
        },
        900,
        3,
        0,
    );
    assert_eq!(out.desyncs, 0);
}

#[test]
fn when_two_peers_play_hide_and_seek_then_confirmed_states_match() {
    // Same rollback harness, hg_sim::GameMode::HideSeek this time — the netcode layer only ever
    // rolls back opaque State bytes, but hide_seek adds a new struct to that state, so this
    // closes the gap the survival-only tests above don't cover.
    let out = simulate(
        2,
        Link {
            base_ms: 150,
            jitter_ms: 30,
            loss_per_mille: 30,
            dup_per_mille: 10,
        },
        400, // past the 300-frame prep window, well into Seeking
        4,
        1,
    );
    assert_eq!(out.desyncs, 0);
    assert!(
        out.compared_frames > 300,
        "compared {} frames {:?}",
        out.compared_frames,
        out.frames
    );
}

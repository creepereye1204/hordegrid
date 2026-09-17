//! Multi-source BFS flow field toward the nearest alive player (docs/design-docs/performance.md §1).

use crate::config::{MAP_H, MAP_W, MAX_PLAYERS, TILES};
use crate::map::MapData;
use crate::state::{Life, State};

/// Neighbour offsets aligned with `DIR8` indices 1..=8.
const OFFS: [(i32, i32); 8] = [
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];

/// Reusable scratch buffers (not part of `State`; fully rebuilt each use).
pub struct FlowScratch {
    queue: Box<[u16; TILES]>,
    dist: Box<[u16; TILES]>,
}

impl Default for FlowScratch {
    fn default() -> Self {
        Self {
            queue: Box::new([0; TILES]),
            dist: Box::new([u16::MAX; TILES]),
        }
    }
}

/// Recompute `state.flow` from current player positions.
/// Cost: one BFS over walkable tiles, O(TILES).
pub fn rebuild(state: &mut State, map: &MapData, scratch: &mut FlowScratch) {
    let FlowScratch { queue, dist } = scratch;
    dist.fill(u16::MAX);
    state.flow.dir.fill(0);
    let (mut head, mut tail) = (0usize, 0usize);

    for i in 0..MAX_PLAYERS {
        if state.life(i) != Life::Alive {
            continue;
        }
        let tx = state.players.x[i].floor_int();
        let ty = state.players.y[i].floor_int();
        if map.solid(tx, ty) {
            continue;
        }
        let idx = ty as usize * MAP_W + tx as usize;
        if dist[idx] == u16::MAX {
            dist[idx] = 0;
            queue[tail] = idx as u16;
            tail += 1;
        }
    }

    while head < tail {
        let cur = usize::from(queue[head]);
        head += 1;
        let (cx, cy) = ((cur % MAP_W) as i32, (cur / MAP_W) as i32);
        for (d, &(ox, oy)) in OFFS.iter().enumerate() {
            let (nx, ny) = (cx + ox, cy + oy);
            if map.solid(nx, ny) {
                continue;
            }
            // diagonal only if both orthogonal neighbours are open (no corner cutting)
            if ox != 0 && oy != 0 && (map.solid(cx + ox, cy) || map.solid(cx, cy + oy)) {
                continue;
            }
            let n = ny as usize * MAP_W + nx as usize;
            if dist[n] != u16::MAX {
                continue;
            }
            dist[n] = dist[cur] + 1;
            // neighbour steps back toward `cur`: opposite of d
            state.flow.dir[n] = ((d + 4) % 8 + 1) as u8;
            queue[tail] = n as u16;
            tail += 1;
        }
    }
    debug_assert!(tail <= MAP_W * MAP_H);
}

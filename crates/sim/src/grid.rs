//! Uniform-grid broadphase rebuilt every step with a counting sort (docs/design-docs/performance.md §2).
//! O(N) build, zero allocation after construction, deterministic in-cell order (slot order).

use crate::config::{MAP_H, MAP_W, MAX_ENEMIES, TILES};
use crate::state::Enemies;

/// Cell = one tile.
pub struct EnemyGrid {
    start: Box<[u16; TILES + 1]>,
    items: Box<[u16; MAX_ENEMIES]>,
    cell_of: Box<[u16; MAX_ENEMIES]>,
}

const NONE: u16 = u16::MAX;

impl Default for EnemyGrid {
    fn default() -> Self {
        Self {
            start: Box::new([0; TILES + 1]),
            items: Box::new([0; MAX_ENEMIES]),
            cell_of: Box::new([NONE; MAX_ENEMIES]),
        }
    }
}

fn cell_index(tx: i32, ty: i32) -> Option<usize> {
    (tx >= 0 && ty >= 0 && (tx as usize) < MAP_W && (ty as usize) < MAP_H)
        .then(|| ty as usize * MAP_W + tx as usize)
}

impl EnemyGrid {
    /// Rebuild from live enemies.
    pub fn rebuild(&mut self, e: &Enemies) {
        let start = &mut *self.start;
        start.fill(0);
        for i in 0..MAX_ENEMIES {
            self.cell_of[i] = NONE;
            if e.alive[i] == 0 {
                continue;
            }
            if let Some(c) = cell_index(e.x[i].floor_int(), e.y[i].floor_int()) {
                self.cell_of[i] = c as u16;
                start[c + 1] += 1;
            }
        }
        for c in 0..TILES {
            start[c + 1] += start[c];
        }
        // fill using a running cursor copy of `start`
        let mut cursor = [0u16; TILES];
        cursor.copy_from_slice(&start[..TILES]);
        for i in 0..MAX_ENEMIES {
            let c = self.cell_of[i];
            if c != NONE {
                let slot = &mut cursor[usize::from(c)];
                self.items[usize::from(*slot)] = i as u16;
                *slot += 1;
            }
        }
    }

    /// Visit enemy slots in the 3×3 cells around tile `(tx, ty)`, in deterministic order.
    /// Stop early when `f` returns `false`.
    pub fn for_each_near(&self, tx: i32, ty: i32, mut f: impl FnMut(usize) -> bool) {
        for oy in -1..=1 {
            for ox in -1..=1 {
                let Some(c) = cell_index(tx + ox, ty + oy) else {
                    continue;
                };
                let (a, b) = (usize::from(self.start[c]), usize::from(self.start[c + 1]));
                for &slot in &self.items[a..b] {
                    if !f(usize::from(slot)) {
                        return;
                    }
                }
            }
        }
    }
}

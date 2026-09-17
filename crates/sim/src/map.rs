//! Static map data (not rolled back).

use crate::config::{MAP_H, MAP_W, TILES};

/// Maximum spawn doors.
pub const MAX_SPAWNS: usize = 8;

/// Immutable tile map shared by all peers.
#[derive(Clone, Debug)]
pub struct MapData {
    solid: [bool; TILES],
    /// Spawn door tiles `(x, y)`.
    pub spawns: [(u8, u8); MAX_SPAWNS],
    /// Number of valid entries in `spawns`.
    pub spawn_count: usize,
    /// Player start tiles.
    pub starts: [(u8, u8); 4],
}

/// Legend: `#` wall, `.` floor, `S` enemy spawn (floor), `1`-`4` player start (floor).
const YARD: [&str; MAP_H] = [
    "########################################",
    "#S.................##.................S#",
    "#......................................#",
    "#...####..........................####.#",
    "#...#................................#.#",
    "#...#.......######......######.......#.#",
    "#...........#....................#.....#",
    "#...........#....................#.....#",
    "#......................................#",
    "#.....##..........................##...#",
    "#.....##..........................##...#",
    "#......................................#",
    "#...............#......#...............#",
    "#...............#......#...............#",
    "S.................1..2.................S",
    "#.................3..4.................#",
    "#...............#......#...............#",
    "#...............#......#...............#",
    "#......................................#",
    "#.....##..........................##...#",
    "#.....##..........................##...#",
    "#......................................#",
    "#...........#....................#.....#",
    "#...........#....................#.....#",
    "#...#.......######......######.......#.#",
    "#...#................................#.#",
    "#...####..........................####.#",
    "#......................................#",
    "#S.................##.................S#",
    "########################################",
];

impl MapData {
    /// Build a map by id. Unknown ids fall back to the yard.
    pub fn by_id(_id: u8) -> Self {
        Self::parse(&YARD)
    }

    fn parse(rows: &[&str; MAP_H]) -> Self {
        let mut map = Self {
            solid: [true; TILES],
            spawns: [(0, 0); MAX_SPAWNS],
            spawn_count: 0,
            starts: [(1, 1); 4],
        };
        for (y, row) in rows.iter().enumerate() {
            for (x, ch) in row.bytes().enumerate().take(MAP_W) {
                let i = y * MAP_W + x;
                map.solid[i] = ch == b'#';
                match ch {
                    b'S' if map.spawn_count < MAX_SPAWNS => {
                        map.spawns[map.spawn_count] = (x as u8, y as u8);
                        map.spawn_count += 1;
                    }
                    b'1'..=b'4' => map.starts[usize::from(ch - b'1')] = (x as u8, y as u8),
                    _ => {}
                }
            }
        }
        map
    }

    /// Whether tile `(x, y)` blocks movement. Out-of-bounds is solid.
    pub fn solid(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 || x >= MAP_W as i32 || y >= MAP_H as i32 {
            return true;
        }
        self.solid[y as usize * MAP_W + x as usize]
    }

    /// Raw tiles for rendering: 1 = wall, 0 = floor.
    pub fn tiles(&self) -> impl Iterator<Item = u8> + '_ {
        self.solid.iter().map(|&s| u8::from(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_parsing_yard_then_rows_have_exact_width() {
        for row in YARD {
            assert_eq!(row.len(), MAP_W, "{row}");
        }
    }

    #[test]
    fn when_parsing_yard_then_spawns_and_starts_are_floor() {
        let m = MapData::by_id(0);
        assert!(m.spawn_count >= 4);
        for &(x, y) in &m.spawns[..m.spawn_count] {
            assert!(!m.solid(i32::from(x), i32::from(y)));
        }
        for &(x, y) in &m.starts {
            assert!(!m.solid(i32::from(x), i32::from(y)));
        }
    }
}

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
    kinds: [u8; TILES],
}

/// Number of built-in maps; ids `0..MAP_COUNT`.
pub const MAP_COUNT: u8 = 3;

/// Tile kinds exported to the renderer (`map_tiles`). Movement only cares about [`MapData::solid`].
pub mod kind {
    /// Walkable ground.
    pub const FLOOR: u8 = 0;
    /// Solid wall (autotiled by the renderer).
    pub const WALL: u8 = 1;
    /// Walkable path / rubble (cosmetic).
    pub const PATH: u8 = 2;
    /// Solid tree, statue or tombstone.
    pub const TREE: u8 = 3;
    /// Solid crate, barrel or other small prop.
    pub const CRATE: u8 = 4;
    /// Solid fence.
    pub const FENCE: u8 = 5;
    /// Solid house block.
    pub const HOUSE: u8 = 6;
}

/// Legend: `#` wall, `.` floor, `,` path, `T` tree/statue, `B` crate, `F` fence, `H` house,
/// `S` enemy spawn (floor), `1`-`4` player start (floor).
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

/// Converted from Kenney "Tiny Dungeon" `sampleMap.tmx` (CC0) by `tools/map-import/tmx2ascii.py`.
const CRYPT: [&str; MAP_H] = [
    "####################S###################",
    "#S....................................S#",
    "#......................................#",
    "#......................................#",
    "#......................................#",
    "#...#.##############################...#",
    "#...#.##############################...#",
    "#...#.##############################...#",
    "#...#....###########################...#",
    "#...#.###############.######.......#...#",
    "#...#.###############.######.T.T.T.#...#",
    "#...#.###############.######.T.T.T.#...#",
    "#...#.##....#########.....##.......#...#",
    "#...#.##.........####..,B.##.......#...#",
    "#.....T..,.......####...,.##.......#...#",
    "S....,T.......12.......................S",
    "#....T####....34...........#########...#",
    "#....T####.......########..#########...#",
    "#...######...############FF#########...#",
    "#...######...############..#########...#",
    "#...######...######........#####...B...#",
    "#...#####......####..,.....#####.B.....#",
    "#...##.......,......,......#####.......#",
    "#...##...................,....B........#",
    "#...##.................................#",
    "#......................................#",
    "#......................................#",
    "#......................................#",
    "#S....................................S#",
    "####################S###################",
];

/// Traced from the Kenney "Tiny Town" sample scene (CC0): houses, fenced yard, castle with two gates.
const TOWN: [&str; MAP_H] = [
    "TTTTTTTTTTTTSTTTTTTTTTTTTTTTSTTTTTTTTTTT",
    "TTT................,,..................T",
    "TT.................,,............B..B..T",
    "TTTHHHHH.HHHH......,,.HHHHH............T",
    "T..HHHHH.HHHH......,,.HHHHH...HHHHH....T",
    "TT.HHHHH.HHHH......,,.HHHHH...HHHHH..T.T",
    "TT.HHHHH.HHHH..T...,,.HHHHH.T.HHHHH....T",
    "S..................,,..................S",
    "T.T.....,,,,,,,,,,,,,,,,,,,,,,,,.......T",
    "TT......,.....FFFF.,,..........,.......T",
    "TT...T..,..........,,.T...B....,....T..T",
    "T.......,...T....B.,,...T......,.......T",
    "T.......,..........,,..........,.......T",
    "T,,,,,,,,,,,,,,,,,,12,,,,,,,,,,,,,,,,,,T",
    "T,,,,,,,,,,,,,,,,,,34,,,,,,,,,,,,,,,,,,T",
    "T..................,,..........,....T..T",
    "T.FFFFFF.HHHHHH....,,..........,.......T",
    "T.F......HHHHHH....,,.B..#####,,,#####.T",
    "T.F.B.B..HHHHHH..T.,,....#####,,,#####.T",
    "T.F......HHHHHH....,,....##....B....##.T",
    "T.F................,,....##....B....##.S",
    "T.FFFF.............,,....##.........##.T",
    "S.............T....,,....##.........##.T",
    "T..................,,....#####,,,#####.T",
    "T.....FFFFFFFFFF...,,....#####,,,#####TT",
    "T..................,,..................T",
    "T...T..............,,..T..............TT",
    "T.........T........,,.....T...........TT",
    "T..................,,........T..T..TTTTT",
    "TTTTTTTTTTTTSTTTTTTTSTTTTTTTTTTTTTTTTTTT",
];

impl MapData {
    /// Build a map by id: 0 yard, 1 crypt, 2 town. Unknown ids fall back to the yard.
    pub fn by_id(id: u8) -> Self {
        Self::parse(match id {
            1 => &CRYPT,
            2 => &TOWN,
            _ => &YARD,
        })
    }

    fn parse(rows: &[&str; MAP_H]) -> Self {
        let mut map = Self {
            solid: [true; TILES],
            spawns: [(0, 0); MAX_SPAWNS],
            spawn_count: 0,
            starts: [(1, 1); 4],
            kinds: [kind::WALL; TILES],
        };
        for (y, row) in rows.iter().enumerate() {
            for (x, ch) in row.bytes().enumerate().take(MAP_W) {
                let i = y * MAP_W + x;
                let k = match ch {
                    b'#' => kind::WALL,
                    b',' => kind::PATH,
                    b'T' => kind::TREE,
                    b'B' => kind::CRATE,
                    b'F' => kind::FENCE,
                    b'H' => kind::HOUSE,
                    _ => kind::FLOOR,
                };
                map.kinds[i] = k;
                map.solid[i] = !matches!(k, kind::FLOOR | kind::PATH);
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

    /// Tile kinds for rendering, see [`kind`].
    pub fn tiles(&self) -> impl Iterator<Item = u8> + '_ {
        self.kinds.iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_parsing_any_map_then_rows_have_exact_width_and_known_chars() {
        for rows in [&YARD, &CRYPT, &TOWN] {
            for row in rows {
                assert_eq!(row.len(), MAP_W, "{row}");
                assert!(row.bytes().all(|c| b"#.,TBFHS1234".contains(&c)), "{row}");
            }
        }
    }

    #[test]
    fn when_parsing_any_map_then_spawns_and_starts_are_floor() {
        for id in 0..MAP_COUNT {
            let m = MapData::by_id(id);
            assert!(m.spawn_count >= 4, "map {id}");
            for &(x, y) in &m.spawns[..m.spawn_count] {
                assert!(!m.solid(i32::from(x), i32::from(y)), "map {id}");
            }
            for &(x, y) in &m.starts {
                assert!(!m.solid(i32::from(x), i32::from(y)), "map {id}");
            }
        }
    }

    #[test]
    fn when_flooding_from_start_then_every_walkable_tile_and_spawn_is_reached() {
        for id in 0..MAP_COUNT {
            let map = MapData::by_id(id);
            let mut seen = [false; TILES];
            let mut stack = [0usize; TILES];
            let (sx, sy) = map.starts[0];
            let start = usize::from(sy) * MAP_W + usize::from(sx);
            seen[start] = true;
            stack[0] = start;
            let mut top = 1;
            while top > 0 {
                top -= 1;
                let cur = stack[top];
                let (cx, cy) = ((cur % MAP_W) as i32, (cur / MAP_W) as i32);
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (nx, ny) = (cx + dx, cy + dy);
                    if !map.solid(nx, ny) {
                        let next = ny as usize * MAP_W + nx as usize;
                        if !seen[next] {
                            seen[next] = true;
                            stack[top] = next;
                            top += 1;
                        }
                    }
                }
            }
            for (idx, (&solid, &reached)) in map.solid.iter().zip(&seen).enumerate() {
                let (tx, ty) = (idx % MAP_W, idx / MAP_W);
                assert!(solid || reached, "map {id}: tile ({tx}, {ty}) unreachable");
            }
        }
    }
}

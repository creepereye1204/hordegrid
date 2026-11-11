# map-import

Turns an open-licensed Tiled map into a hordegrid ASCII layout (`crates/sim/src/map.rs`).

```sh
python3 -I tools/map-import/tmx2ascii.py path/to/kenney_tiny-dungeon/Tiled/sampleMap.tmx
```

- Input: Kenney *Tiny Dungeon* `sampleMap.tmx` (CC0, get it from kenney.nl or opengameart.org). The tile-id → meaning tables
  at the top of the script are specific to that tileset; another tileset needs its own table.
- Output: 30 rows × 40 chars on stdout, pasted into `map.rs` as `CRYPT`. The 32×20 sample is centred, gets an open margin,
  an outer wall, 8 spawn doors and 4 starts; floor unreachable from start 1 is filled in.
- Only CC0 / permissive sources. No layouts from the reference game (docs/SECURITY.md#ip-지식재산). Credit the source in
  `CREDITS.md`.
- `cargo test -p hg-sim map` checks every map: exact width, known chars, spawns/starts on floor, everything reachable.

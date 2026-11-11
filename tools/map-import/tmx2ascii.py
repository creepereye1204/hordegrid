"""Convert Kenney "Tiny Dungeon" sampleMap.tmx (CC0) into a hordegrid ASCII map.

Usage: python3 -I tools/map-import/tmx2ascii.py path/to/sampleMap.tmx

Prints MAP_H rows of MAP_W chars (legend in crates/sim/src/map.rs). The 32x20 sample is
centred in a 40x30 arena with an open margin, an outer wall and 8 spawn doors. Floor that
the players cannot reach from start 1 is filled in, so every walkable tile is reachable.
"""
import sys
import xml.etree.ElementTree as ET
from collections import deque

W, H = 40, 30
FLIP_MASK = 0x1FFFFFFF

# Tiny Dungeon tile index -> meaning. Everything not listed in FLOOR is wall.
FLOOR = {30, 31, 32, 36, 37, 38, 41, 42, 48, 49, 50, 51, 52, 53}
RUBBLE = {41, 42}
# Objects layer: props that block movement.
PROP_TOMB = {64, 65, 66}          # tombstones / statues  -> 'T'
PROP_CRATE = {72, 74, 75, 82, 89}  # table, anvil, shelf, barrel, chest -> 'B'
PROP_FENCE = {77}                  # fence -> 'F'


def layers(path):
    root = ET.parse(path).getroot()
    w = int(root.get('width'))
    out = {}
    for layer in root.findall('layer'):
        cells = [int(v) for v in layer.find('data').text.replace('\n', '').split(',') if v.strip()]
        out[layer.get('name')] = [[(g & FLIP_MASK) - 1 for g in cells[r * w:(r + 1) * w]] for r in range(len(cells) // w)]
    return out


def main(path):
    ls = layers(path)
    dungeon, objects = ls['Dungeon'], ls['Objects']
    sh, sw = len(dungeon), len(dungeon[0])
    ox, oy = (W - sw) // 2, (H - sh) // 2
    grid = [['.'] * W for _ in range(H)]
    for y in range(H):
        for x in range(W):
            if x in (0, W - 1) or y in (0, H - 1):
                grid[y][x] = '#'
    for y in range(sh):
        for x in range(sw):
            t, o = dungeon[y][x], objects[y][x]
            c = '.' if t in FLOOR else '#'
            if c == '.' and t in RUBBLE:
                c = ','
            if c != '#':
                if o in PROP_TOMB:
                    c = 'T'
                elif o in PROP_CRATE:
                    c = 'B'
                elif o in PROP_FENCE:
                    c = 'F'
            grid[oy + y][ox + x] = c
    # Spawn doors on the outer wall: corners-ish and mid edges.
    for x, y in [(1, 1), (W - 2, 1), (1, H - 2), (W - 2, H - 2), (0, H // 2), (W - 1, H // 2), (W // 2, 0), (W // 2, H - 1)]:
        grid[y][x] = 'S'
    # Player starts: the open hall in the sample's lower middle.
    starts = [(ox + 10, oy + 10), (ox + 11, oy + 10), (ox + 10, oy + 11), (ox + 11, oy + 11)]
    for i, (x, y) in enumerate(starts):
        assert grid[y][x] in '.,', (x, y, grid[y][x])
        grid[y][x] = str(i + 1)
    # Fill unreachable floor (4-neighbour flood from start 1).
    seen = {starts[0]}
    q = deque([starts[0]])
    while q:
        x, y = q.popleft()
        for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            nx, ny = x + dx, y + dy
            if 0 <= nx < W and 0 <= ny < H and (nx, ny) not in seen and grid[ny][nx] not in '#TBF':
                seen.add((nx, ny))
                q.append((nx, ny))
    for y in range(H):
        for x in range(W):
            if grid[y][x] not in '#TBF' and (x, y) not in seen:
                grid[y][x] = '#'
    # Props with no reachable neighbour are buried in a filled pocket.
    for y in range(1, H - 1):
        for x in range(1, W - 1):
            if grid[y][x] in 'TBF' and not any((x + dx, y + dy) in seen for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))):
                grid[y][x] = '#'
    for row in grid:
        print(''.join(row))


if __name__ == '__main__':
    main(sys.argv[1])

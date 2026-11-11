// Paints the static map layer from CC0 Kenney tilesheets (credited in CREDITS.md):
// "Tiny Dungeon" for the yard and crypt, "Tiny Town" for the town. Both sheets are 12x11
// tiles of 16px with no spacing. Walls, fences and houses are autotiled from the tile-kind
// grid the sim exports, so a map is just an ASCII layout (crates/sim/src/map.rs).
import dungeonUrl from '../assets/sprites/dungeon-sheet.png';
import townUrl from '../assets/sprites/town-sheet.png';

/** Mirrors `hg_sim::map::kind`. */
export const K = { FLOOR: 0, WALL: 1, PATH: 2, TREE: 3, CRATE: 4, FENCE: 5, HOUSE: 6 } as const;

/** Map ids and their display names (ids mirror `MapData::by_id`). */
export const MAPS = [
  { id: 0, name: '마당' },
  { id: 1, name: '지하묘지' },
  { id: 2, name: '마을' },
] as const;
export type MapId = 0 | 1 | 2;
export const isMapId = (v: unknown): v is MapId => v === 0 || v === 1 || v === 2;

const SRC = 16;
const COLS = 12;

function img(src: string): HTMLImageElement {
  const el = new Image();
  el.src = src;
  return el;
}

export const sheets = { dungeon: img(dungeonUrl), town: img(townUrl) };

/** Stable per-tile noise in 0..99 so variants never flicker between rebuilds. */
function noise(x: number, y: number): number {
  let h = (x * 374761393 + y * 668265263) | 0;
  h = Math.imul(h ^ (h >>> 13), 1274126177);
  return ((h ^ (h >>> 16)) >>> 0) % 100;
}

type Painter = (g: CanvasRenderingContext2D, tiles: Uint8Array, w: number, h: number, t: number) => void;

function at(tiles: Uint8Array, w: number, h: number, x: number, y: number): number {
  if (x < 0 || y < 0 || x >= w || y >= h) return K.WALL;
  return tiles[y * w + x]!;
}

function blit(g: CanvasRenderingContext2D, sheet: HTMLImageElement, idx: number, x: number, y: number, t: number): void {
  g.drawImage(sheet, (idx % COLS) * SRC, Math.floor(idx / COLS) * SRC, SRC, SRC, x * t, y * t, t, t);
}

// ------------------------------------------------------------------ dungeon (Tiny Dungeon)

const D_SAND = [48, 48, 48, 48, 49] as const;
/** Sand with a shadow along its top edge, for tiles right under a wall face. */
const D_SAND_SHADOW = 50;
const D_RIM_DARK = '#3f2631';
const D_RIM_LIGHT = '#c0cbdc';

const paintDungeon: Painter = (g, tiles, w, h, t) => {
  const s = sheets.dungeon;
  const px = t / SRC;
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const k = at(tiles, w, h, x, y);
      const n = noise(x, y);
      if (k !== K.WALL) {
        const shaded = at(tiles, w, h, x, y - 1) === K.WALL;
        blit(g, s, k === K.PATH ? 42 : shaded ? D_SAND_SHADOW : D_SAND[n % D_SAND.length]!, x, y, t);
      }
      switch (k) {
        case K.WALL: {
          const open = (dx: number, dy: number): boolean => at(tiles, w, h, x + dx, y + dy) !== K.WALL;
          if (open(0, 1)) {
            // Front face: brick, with the odd window / banner for variety.
            blit(g, s, n < 6 ? 28 : n < 9 ? 29 : 40, x, y, t);
          } else {
            blit(g, s, n < 4 ? 12 : n < 7 ? 24 : 0, x, y, t);
            // Stone rim on every side that drops to a walkable tile (dark/light/dark, 2px each).
            const band = (ix: number, iy: number, iw: number, ih: number, c: string): void => {
              g.fillStyle = c;
              g.fillRect(x * t + ix * px, y * t + iy * px, iw * px, ih * px);
            };
            const rim = (side: 'n' | 's' | 'w' | 'e'): void => {
              for (const [off, c] of [[0, D_RIM_DARK], [2, D_RIM_LIGHT], [4, D_RIM_DARK]] as const) {
                if (side === 'n') band(0, off, SRC, 2, c);
                if (side === 's') band(0, SRC - off - 2, SRC, 2, c);
                if (side === 'w') band(off, 0, 2, SRC, c);
                if (side === 'e') band(SRC - off - 2, 0, 2, SRC, c);
              }
            };
            if (open(0, -1)) rim('n');
            if (open(0, 2) && !open(0, 1)) rim('s');
            if (open(-1, 0)) rim('w');
            if (open(1, 0)) rim('e');
          }
          break;
        }
        case K.TREE:
          blit(g, s, n % 2 ? 65 : 64, x, y, t);
          break;
        case K.CRATE:
          blit(g, s, [82, 82, 89, 75][n % 4]!, x, y, t);
          break;
        case K.FENCE:
          blit(g, s, 77, x, y, t);
          break;
        case K.HOUSE:
          blit(g, s, 40, x, y, t);
          break;
        default:
      }
    }
  }
};

// ------------------------------------------------------------------ town (Tiny Town)

const T_GRASS = [0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 2, 0, 0, 0, 0] as const;
const T_TREES = [16, 28, 5, 16, 28, 15, 27] as const;
const T_PROPS = [94, 95, 83, 106] as const;

/** Picks column 0/1/2 of a 3-wide strip from which horizontal neighbours continue the run. */
function edge3(left: boolean, right: boolean): number {
  if (!left && right) return 0;
  if (left && !right) return 2;
  return 1;
}

const paintTown: Painter = (g, tiles, w, h, t) => {
  const s = sheets.town;
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const k = at(tiles, w, h, x, y);
      const n = noise(x, y);
      const same = (dx: number, dy: number): boolean => at(tiles, w, h, x + dx, y + dy) === k;
      blit(g, s, k === K.PATH ? 43 : T_GRASS[n % T_GRASS.length]!, x, y, t);
      switch (k) {
        case K.WALL: {
          // Castle stone: 3x3 slice on 96..122, picked from which sides continue.
          const row = !same(0, -1) ? 96 : !same(0, 1) ? 120 : 108;
          blit(g, s, row + edge3(same(-1, 0), same(1, 0)), x, y, t);
          break;
        }
        case K.TREE:
          blit(g, s, T_TREES[n % T_TREES.length]!, x, y, t);
          break;
        case K.CRATE:
          blit(g, s, T_PROPS[n % T_PROPS.length]!, x, y, t);
          break;
        case K.FENCE: {
          const hz = same(-1, 0) || same(1, 0);
          const vt = same(0, -1) || same(0, 1);
          if (hz || !vt) blit(g, s, 44 + edge3(same(-1, 0), same(1, 0)), x, y, t);
          else blit(g, s, !same(0, -1) ? 47 : !same(0, 1) ? 71 : 59, x, y, t);
          break;
        }
        case K.HOUSE: {
          // Find this house's horizontal extent on its row and its vertical extent in its column.
          let x0 = x;
          while (at(tiles, w, h, x0 - 1, y) === K.HOUSE) x0--;
          let x1 = x;
          while (at(tiles, w, h, x1 + 1, y) === K.HOUSE) x1++;
          let y0 = y;
          while (at(tiles, w, h, x, y0 - 1) === K.HOUSE) y0--;
          const red = noise(x0, y0) % 2 === 0;
          const mid = (x0 + x1) >> 1;
          const col = edge3(x > x0, x < x1);
          if (!same(0, 1)) {
            // Front wall: door in the middle, windows elsewhere.
            const wood = red;
            if (x === mid) blit(g, s, wood ? 86 : 90, x, y, t);
            else blit(g, s, (x - x0) % 2 ? (wood ? 84 : 88) : (wood ? 73 : 77), x, y, t);
          } else if (y === y0) {
            blit(g, s, x === mid ? (red ? 67 : 63) : (red ? 52 : 48) + col, x, y, t);
          } else {
            blit(g, s, (red ? 64 : 60) + col, x, y, t);
          }
          break;
        }
        default:
      }
    }
  }
};

const PAINTERS: Record<MapId, Painter> = { 0: paintDungeon, 1: paintDungeon, 2: paintTown };

/** Background behind the map (what shows past the map edge). */
export const MAP_BG: Record<MapId, string> = { 0: '#3f2631', 1: '#3f2631', 2: '#2b4a2f' };

export function paintMap(g: CanvasRenderingContext2D, mapId: MapId, tiles: Uint8Array, w: number, h: number, t: number): void {
  PAINTERS[mapId](g, tiles, w, h, t);
}

export function loadSheets(): Promise<void> {
  return Promise.all(
    Object.values(sheets).map(
      (el) =>
        new Promise<void>((resolve) => {
          if (el.complete) resolve();
          else {
            el.onload = () => resolve();
            el.onerror = () => resolve();
          }
        }),
    ),
  ).then(() => undefined);
}

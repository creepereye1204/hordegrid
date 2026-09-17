import {
  DOWNED_FRAMES, E, ENEMY_STRIDE, enemiesOffset, FIXED_ONE, H, HEADER, Life, MAX_ENEMIES, MAX_PLAYERS, P,
  PLAYER_STRIDE, REVIVE_FRAMES, S, SHOT_STRIDE, shotsOffset,
} from '../core/view-layout';
import type { Effects } from './effects';
import { C } from './palette';

const VIEW_TILES_W = 24;
const VIEW_TILES_H = 16;
const DIR_VEC: ReadonlyArray<readonly [number, number]> = [
  [0, 0], [0, -1], [0.707, -0.707], [1, 0], [0.707, 0.707], [0, 1], [-0.707, 0.707], [-1, 0], [-0.707, -0.707],
];

/**
 * Canvas2D renderer. Reads snapshots of the render view; never writes sim state (core-beliefs #5).
 * Interpolates between the last two fixed steps, keyed by player index / enemy slot.
 */
export class Renderer {
  private readonly ctx: CanvasRenderingContext2D;
  private mapW = 0;
  private mapH = 0;
  private tiles: Uint8Array = new Uint8Array();
  private floorCache: HTMLCanvasElement | null = null;
  private cachedTile = 0;
  private tile = 24;
  private cssW = 0;
  private cssH = 0;
  private prev: Int32Array | null = null;
  private cur: Int32Array | null = null;
  private readonly prevEnemy = new Float64Array(MAX_ENEMIES * 2);
  private readonly prevEnemyFrame = new Int32Array(MAX_ENEMIES).fill(-1);

  constructor(private readonly canvas: HTMLCanvasElement) {
    const ctx = canvas.getContext('2d', { alpha: false });
    if (!ctx) throw new Error('Canvas2D unavailable');
    this.ctx = ctx;
  }

  setMap(w: number, h: number, tiles: Uint8Array): void {
    this.mapW = w;
    this.mapH = h;
    this.tiles = tiles;
    this.floorCache = null;
    this.prev = null;
    this.cur = null;
    this.prevEnemyFrame.fill(-1);
  }

  /** Feed the view after each simulated step (copied, the wasm view is transient). */
  pushStep(view: Int32Array): void {
    if (this.cur) {
      const n = this.cur[H.N_ENEMIES] ?? 0;
      const frame = this.cur[H.FRAME] ?? 0;
      for (let k = 0; k < n; k++) {
        const o = enemiesOffset + k * ENEMY_STRIDE;
        const slot = this.cur[o + E.SLOT]!;
        this.prevEnemy[slot * 2] = this.cur[o + E.X]!;
        this.prevEnemy[slot * 2 + 1] = this.cur[o + E.Y]!;
        this.prevEnemyFrame[slot] = frame;
      }
    }
    this.prev = this.cur;
    this.cur = view.slice();
  }

  get latest(): Int32Array | null {
    return this.cur;
  }

  resize(): void {
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    const rect = this.canvas.getBoundingClientRect();
    this.cssW = rect.width;
    this.cssH = rect.height;
    this.canvas.width = Math.round(rect.width * dpr);
    this.canvas.height = Math.round(rect.height * dpr);
    this.ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    // same world area on every device: PCs and phones see equally far (mobile-play.md#화면)
    this.tile = Math.max(12, Math.floor(Math.min(this.cssW / VIEW_TILES_W, this.cssH / VIEW_TILES_H)));
  }

  draw(alpha: number, effects: Effects): void {
    const v = this.cur;
    const ctx = this.ctx;
    ctx.fillStyle = C.bg;
    ctx.fillRect(0, 0, this.cssW, this.cssH);
    if (!v) return;
    const t = this.tile;
    const prev = this.prev;
    const local = Math.max(0, Math.min(MAX_PLAYERS - 1, v[H.LOCAL] ?? 0));

    const lerp = (a: number, b: number): number => (Math.abs(b - a) > FIXED_ONE ? b : a + (b - a) * alpha) / FIXED_ONE;
    const playerPos = (i: number): [number, number] => {
      const o = HEADER + i * PLAYER_STRIDE;
      const px = prev ? prev[o + P.X]! : v[o + P.X]!;
      const py = prev ? prev[o + P.Y]! : v[o + P.Y]!;
      return [lerp(px, v[o + P.X]!), lerp(py, v[o + P.Y]!)];
    };

    // camera
    const [cx, cy] = playerPos(local);
    const viewW = this.cssW / t;
    const viewH = this.cssH / t;
    let ox = clampCam(cx - viewW / 2, this.mapW, viewW);
    let oy = clampCam(cy - viewH / 2, this.mapH, viewH);
    if (effects.shake > 0.01) {
      ox += (Math.random() - 0.5) * effects.shake;
      oy += (Math.random() - 0.5) * effects.shake;
    }

    this.drawFloor(ox, oy);

    // shots
    const nE = v[H.N_ENEMIES] ?? 0;
    const nS = v[H.N_SHOTS] ?? 0;
    ctx.fillStyle = C.shot;
    const so = shotsOffset(nE);
    for (let k = 0; k < nS; k++) {
      const o = so + k * SHOT_STRIDE;
      const x = (v[o + S.X]! / FIXED_ONE - ox) * t;
      const y = (v[o + S.Y]! / FIXED_ONE - oy) * t;
      const [dx, dy] = DIR_VEC[v[o + S.DIR]!] ?? [0, 0];
      ctx.fillRect(x - 2 - dx * t * 0.12, y - 2 - dy * t * 0.12, 4, 4);
    }

    // enemies
    const frame = v[H.FRAME]!;
    for (let k = 0; k < nE; k++) {
      const o = enemiesOffset + k * ENEMY_STRIDE;
      const slot = v[o + E.SLOT]!;
      const hasPrev = this.prevEnemyFrame[slot] === frame - 1;
      const x = lerp(hasPrev ? this.prevEnemy[slot * 2]! : v[o + E.X]!, v[o + E.X]!);
      const y = lerp(hasPrev ? this.prevEnemy[slot * 2 + 1]! : v[o + E.Y]!, v[o + E.Y]!);
      const kind = v[o + E.KIND]!;
      const r = (kind === 1 ? 0.3 : 0.36) * t;
      const sx = (x - ox) * t;
      const sy = (y - oy) * t;
      if (sx < -t || sy < -t || sx > this.cssW + t || sy > this.cssH + t) continue;
      ctx.fillStyle = v[o + E.STAGGER]! > 3 ? '#ffffff' : kind === 1 ? C.runner : C.walker;
      ctx.fillRect(sx - r, sy - r, r * 2, r * 2);
      ctx.fillStyle = '#00000055';
      ctx.fillRect(sx - r, sy + r * 0.35, r * 2, r * 0.65);
      const hp = v[o + E.HP_PERMILLE]!;
      if (hp < 1000) {
        ctx.fillStyle = C.danger;
        ctx.fillRect(sx - r, sy - r - 4, ((r * 2) * hp) / 1000, 2);
      }
    }

    // players
    const numPlayers = v[H.NUM_PLAYERS] ?? 1;
    for (let i = 0; i < numPlayers; i++) {
      const o = HEADER + i * PLAYER_STRIDE;
      const life = v[o + P.LIFE]!;
      if (life !== Life.Alive && life !== Life.Downed) continue;
      const [x, y] = playerPos(i);
      const sx = (x - ox) * t;
      const sy = (y - oy) * t;
      const r = 0.36 * t;
      const color = C.players[i] ?? C.fg;
      if (life === Life.Downed) {
        const pulse = 0.5 + 0.5 * Math.sin(frame / 6);
        ctx.strokeStyle = C.danger;
        ctx.globalAlpha = 0.4 + 0.6 * pulse;
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.arc(sx, sy, r * 1.6, 0, Math.PI * 2 * (v[o + P.DOWN_TIMER]! / DOWNED_FRAMES));
        ctx.stroke();
        ctx.globalAlpha = 1;
        const revive = v[o + P.REVIVE]!;
        if (revive > 0) {
          ctx.strokeStyle = C.players[3];
          ctx.lineWidth = 4;
          ctx.beginPath();
          ctx.arc(sx, sy, r * 2, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * (revive / REVIVE_FRAMES));
          ctx.stroke();
        }
      }
      ctx.fillStyle = v[o + P.HURT]! > 0 ? '#ffffff' : color;
      ctx.globalAlpha = life === Life.Downed ? 0.55 : 1;
      ctx.beginPath();
      ctx.arc(sx, sy, r, 0, Math.PI * 2);
      ctx.fill();
      ctx.globalAlpha = 1;
      // facing
      const [fx, fy] = DIR_VEC[v[o + P.FACING]!] ?? [0, 1];
      ctx.strokeStyle = '#0e0f13';
      ctx.lineWidth = Math.max(2, t * 0.1);
      ctx.beginPath();
      ctx.moveTo(sx, sy);
      ctx.lineTo(sx + fx * r * 1.25, sy + fy * r * 1.25);
      ctx.stroke();
      // ring for local player + number (never color-only identification)
      if (i === local) {
        ctx.strokeStyle = color;
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.arc(sx, sy, r + 4, 0, Math.PI * 2);
        ctx.stroke();
      }
      ctx.fillStyle = C.fg;
      ctx.font = `bold ${Math.max(10, Math.round(t * 0.42))}px system-ui, sans-serif`;
      ctx.textAlign = 'center';
      ctx.fillText(String(i + 1), sx, sy - r - 6);
      // hp
      const hp = Math.max(0, v[o + P.HP]!);
      ctx.fillStyle = '#00000088';
      ctx.fillRect(sx - r, sy + r + 3, r * 2, 3);
      ctx.fillStyle = hp > 30 ? C.players[3] : C.danger;
      ctx.fillRect(sx - r, sy + r + 3, (r * 2 * hp) / 100, 3);
    }

    effects.draw(ctx, t, ox, oy);
    this.drawOffscreenArrows(v, local, ox, oy, viewW, viewH);
  }

  private drawFloor(ox: number, oy: number): void {
    const t = this.tile;
    if (!this.floorCache || this.cachedTile !== t) {
      const c = document.createElement('canvas');
      c.width = this.mapW * t;
      c.height = this.mapH * t;
      const g = c.getContext('2d');
      if (!g) return;
      for (let y = 0; y < this.mapH; y++) {
        for (let x = 0; x < this.mapW; x++) {
          const wall = this.tiles[y * this.mapW + x] === 1;
          g.fillStyle = wall ? C.wall : (x + y) % 2 ? C.floor : C.floorAlt;
          g.fillRect(x * t, y * t, t, t);
          if (wall) {
            g.fillStyle = C.wallTop;
            g.fillRect(x * t, y * t, t, Math.max(2, t * 0.18));
          }
        }
      }
      this.floorCache = c;
      this.cachedTile = t;
    }
    this.ctx.drawImage(this.floorCache, Math.round(-ox * t), Math.round(-oy * t));
  }

  private drawOffscreenArrows(v: Int32Array, local: number, ox: number, oy: number, viewW: number, viewH: number): void {
    const ctx = this.ctx;
    const t = this.tile;
    const numPlayers = v[H.NUM_PLAYERS] ?? 1;
    for (let i = 0; i < numPlayers; i++) {
      if (i === local) continue;
      const o = HEADER + i * PLAYER_STRIDE;
      const life = v[o + P.LIFE]!;
      if (life !== Life.Alive && life !== Life.Downed) continue;
      const x = v[o + P.X]! / FIXED_ONE - ox;
      const y = v[o + P.Y]! / FIXED_ONE - oy;
      if (x >= 0 && y >= 0 && x <= viewW && y <= viewH) continue;
      const ex = Math.min(Math.max(x, 0.6), viewW - 0.6) * t;
      const ey = Math.min(Math.max(y, 0.6), viewH - 0.6) * t;
      ctx.fillStyle = life === Life.Downed ? C.danger : (C.players[i] ?? C.fg);
      ctx.beginPath();
      ctx.arc(ex, ey, 7, 0, Math.PI * 2);
      ctx.fill();
      ctx.fillStyle = '#0e0f13';
      ctx.font = 'bold 10px system-ui, sans-serif';
      ctx.textAlign = 'center';
      ctx.fillText(String(i + 1), ex, ey + 3.5);
    }
  }
}

function clampCam(o: number, mapSize: number, view: number): number {
  if (view >= mapSize) return (mapSize - view) / 2;
  return Math.min(Math.max(o, 0), mapSize - view);
}

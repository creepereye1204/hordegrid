/**
 * Dev-only stand-in for the wasm module (used when src/wasm-pkg is missing).
 * Not deterministic, not networked — it only feeds the renderer/UI so they can be built and
 * screenshot-tested without a Rust toolchain. The real rules live in crates/sim.
 */
import { ENEMY_STRIDE, H, HEADER, MAX_PLAYERS, PLAYER_STRIDE, SHOT_STRIDE } from './view-layout';

const W = 40;
const HGT = 30;
const ONE = 65536;
const memory = new WebAssembly.Memory({ initial: 2 });

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
}

export default function init(): Promise<InitOutput> {
  return Promise.resolve({ memory });
}

const DIRS: ReadonlyArray<readonly [number, number]> = [
  [0, 0], [0, -1], [0.7071, -0.7071], [1, 0], [0.7071, 0.7071], [0, 1], [-0.7071, 0.7071], [-1, 0], [-0.7071, -0.7071],
];

function solid(x: number, y: number): boolean {
  if (x < 1 || y < 1 || x >= W - 1 || y >= HGT - 1) return true;
  return (x % 10 === 5 && y % 8 > 2 && y % 8 < 6);
}

export class Game {
  private frame = 0;
  private px: number[];
  private py: number[];
  private facing = [5, 5, 5, 5];
  private enemies: { x: number; y: number; hp: number }[] = [];
  private shots: { x: number; y: number; vx: number; vy: number; ttl: number }[] = [];
  private score = 0;
  private events: number[] = [];

  constructor(private readonly players: number, private readonly local: number, _seed: number, _map: number) {
    this.px = [18.5, 21.5, 18.5, 21.5];
    this.py = [14.5, 14.5, 15.5, 15.5];
  }

  free(): void {}
  push_packet(): void {}
  take_outbox(): Uint8Array { return new Uint8Array(); }
  take_net_events(): Int32Array { return new Int32Array(); }
  ping_ms(): number { return -1; }
  map_width(): number { return W; }
  map_height(): number { return HGT; }
  map_tiles(): Uint8Array {
    const t = new Uint8Array(W * HGT);
    for (let y = 0; y < HGT; y++) for (let x = 0; x < W; x++) t[y * W + x] = solid(x, y) ? 1 : 0;
    return t;
  }
  checksum_hex(): string { return 'mock'; }
  state_dump(): Uint8Array { return new Uint8Array(); }

  tick(bits: number): number {
    this.frame++;
    const dir = bits & 15;
    const i = this.local;
    if (dir > 0 && dir <= 8 && !(bits & 64)) this.facing[i] = dir;
    const [dx, dy] = DIRS[dir <= 8 ? dir : 0]!;
    const nx = this.px[i]! + dx * 0.09;
    const ny = this.py[i]! + dy * 0.09;
    if (!solid(Math.floor(nx), Math.floor(ny))) { this.px[i] = nx; this.py[i] = ny; }
    if (bits & 16 && this.frame % 12 === 0) {
      const [fx, fy] = DIRS[this.facing[i]!]!;
      this.shots.push({ x: this.px[i]!, y: this.py[i]!, vx: fx * 0.6, vy: fy * 0.6, ttl: 30 });
      this.events.push(this.frame, 9, 0, 0, 0);
    }
    if (this.frame % 40 === 0 && this.enemies.length < 60) {
      this.enemies.push({ x: 2.5 + (this.frame % 3) * 17, y: 2.5, hp: 3 });
    }
    for (const e of this.enemies) {
      const ang = Math.atan2(this.py[i]! - e.y, this.px[i]! - e.x);
      e.x += Math.cos(ang) * 0.03;
      e.y += Math.sin(ang) * 0.03;
    }
    for (const s of this.shots) {
      s.x += s.vx; s.y += s.vy; s.ttl--;
      for (const e of this.enemies) {
        if (Math.hypot(e.x - s.x, e.y - s.y) < 0.45) {
          e.hp--; s.ttl = 0;
          if (e.hp <= 0) { this.score += 10; this.events.push(this.frame, 2, 0, Math.round(e.x * ONE), Math.round(e.y * ONE)); }
        }
      }
    }
    this.enemies = this.enemies.filter((e) => e.hp > 0);
    this.shots = this.shots.filter((s) => s.ttl > 0);
    return 0;
  }

  take_events(): Int32Array {
    const out = Int32Array.from(this.events);
    this.events = [];
    return out;
  }

  render(): number {
    const v = new Int32Array(memory.buffer, 0, HEADER + MAX_PLAYERS * PLAYER_STRIDE + this.enemies.length * ENEMY_STRIDE + this.shots.length * SHOT_STRIDE);
    v.fill(0);
    v[H.FRAME] = this.frame;
    v[H.N_PLAYERS] = MAX_PLAYERS;
    v[H.N_ENEMIES] = this.enemies.length;
    v[H.N_SHOTS] = this.shots.length;
    v[H.WAVE] = 1;
    v[H.PHASE] = 1;
    v[H.SCORE] = this.score;
    v[H.COMBO] = this.frame % 300 < 150 ? 7 : 0;
    v[H.UNLOCKED] = 1;
    v[H.LOCAL] = this.local;
    v[H.NUM_PLAYERS] = this.players;
    let o = HEADER;
    for (let p = 0; p < MAX_PLAYERS; p++, o += PLAYER_STRIDE) {
      if (p >= this.players) continue;
      v[o] = Math.round(this.px[p]! * ONE);
      v[o + 1] = Math.round(this.py[p]! * ONE);
      v[o + 2] = p === 1 ? 35 : 100;
      v[o + 3] = p === 2 ? 2 : 1;
      v[o + 4] = this.facing[p]!;
      v[o + 7] = 400;
      v[o + 8] = p === 2 ? 60 : 0;
    }
    this.enemies.forEach((e, k) => {
      v.set([k, Math.round(e.x * ONE), Math.round(e.y * ONE), k % 2, Math.round((e.hp / 3) * 1000), 0], o);
      o += ENEMY_STRIDE;
    });
    for (const s of this.shots) {
      v.set([Math.round(s.x * ONE), Math.round(s.y * ONE), 0, 3], o);
      o += SHOT_STRIDE;
    }
    return v.length;
  }

  render_ptr(): number { return 0; }
}

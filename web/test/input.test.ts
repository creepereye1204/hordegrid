import { describe, expect, it } from 'vitest';
import { formatCode, normalizeCode, sanitizeName, seedFromIds } from '../src/core/ids';
import { iterOutbox } from '../src/core/wasm-framing';
import { Bits, dirFromVector, dirWithHysteresis, emptyInput, encode } from '../src/input/encode';

describe('direction quantization (mirrors crates/sim dir_from_delta)', () => {
  it('maps unit vectors to DIR8', () => {
    const d = Math.SQRT1_2;
    const cases: Array<[number, number, number]> = [
      [0, -1, 1], [d, -d, 2], [1, 0, 3], [d, d, 4], [0, 1, 5], [-d, d, 6], [-1, 0, 7], [-d, -d, 8], [0, 0, 0],
    ];
    for (const [x, y, dir] of cases) expect(dirFromVector(x, y)).toBe(dir);
  });

  it('holds direction near a sector boundary (hysteresis)', () => {
    const angle = (deg: number): [number, number] => [Math.sin((deg * Math.PI) / 180), -Math.cos((deg * Math.PI) / 180)];
    const [x1, y1] = angle(24); // just past N/NE boundary (22.5°)
    expect(dirWithHysteresis(x1, y1, 1, 0.2)).toBe(1);
    const [x2, y2] = angle(30);
    expect(dirWithHysteresis(x2, y2, 1, 0.2)).toBe(2);
    expect(dirWithHysteresis(0.1, 0.05, 3, 0.2)).toBe(0);
  });

  it('encodes flags without touching reserved/present bits', () => {
    const f = { ...emptyInput(), fire: true, strafe: true, weaponNext: true };
    const bits = encode(f, 4);
    expect(bits).toBe(4 | Bits.FIRE | Bits.STRAFE | Bits.WEAPON_NEXT);
    expect(bits & 0xfe00).toBe(0);
  });
});

describe('ids', () => {
  it('normalizes invite codes', () => {
    expect(normalizeCode('4821 0937')).toBe('48210937');
    expect(normalizeCode('4821-0937')).toBe('48210937');
    expect(normalizeCode('1234')).toBeNull();
    expect(formatCode('48210937')).toBe('4821 0937');
  });

  it('seed is order-independent', () => {
    expect(seedFromIds(['b', 'a', 'c'], 7)).toBe(seedFromIds(['c', 'b', 'a'], 7));
    expect(seedFromIds(['a', 'b'], 7)).not.toBe(seedFromIds(['a', 'b'], 8));
  });

  it('sanitizes names', () => {
    expect(sanitizeName(`a${String.fromCharCode(0)}b`)).toBe('ab');
    expect(sanitizeName(42)).toBe('???');
  });
});

describe('outbox framing', () => {
  it('splits [slot][len LE][bytes] records and stops on truncation', () => {
    const buf = new Uint8Array([1, 2, 0, 9, 9, 3, 1, 0, 7, 2, 5, 0, 1]);
    expect([...iterOutbox(buf)].map(([s, b]) => [s, [...b]])).toEqual([[1, [9, 9]], [3, [7]]]);
  });
});

// Input bit layout: docs/generated/net-protocol.md (mirrors crates/sim/src/input.rs).
export const Bits = {
  FIRE: 1 << 4,
  PLACE: 1 << 5,
  STRAFE: 1 << 6,
  WEAPON_NEXT: 1 << 7,
  WEAPON_PREV: 1 << 8,
  RELOAD: 1 << 9,
} as const;

export interface InputFrame {
  /** -1..1 each; (0,0) = no movement */
  moveX: number;
  moveY: number;
  fire: boolean;
  place: boolean;
  strafe: boolean;
  weaponNext: boolean;
  weaponPrev: boolean;
  reload: boolean;
}

export const emptyInput = (): InputFrame => ({
  moveX: 0, moveY: 0, fire: false, place: false, strafe: false, weaponNext: false, weaponPrev: false, reload: false,
});

const TAN_67_5 = 2.4142;

/** Quantize a vector to DIR8 (1=N … 8=NW, screen y down). `deadzone` on magnitude. */
export function dirFromVector(x: number, y: number, deadzone = 0): number {
  if (Math.hypot(x, y) <= deadzone || (x === 0 && y === 0)) return 0;
  const ax = Math.abs(x);
  const ay = Math.abs(y);
  if (ax > ay * TAN_67_5) return x >= 0 ? 3 : 7;
  if (ay > ax * TAN_67_5) return y >= 0 ? 5 : 1;
  if (x >= 0) return y < 0 ? 2 : 4;
  return y >= 0 ? 6 : 8;
}

/**
 * Stick quantization with hysteresis: keep the previous direction while the angle stays within
 * ±(22.5° + margin) of it, so a thumb resting on a sector boundary doesn't jitter.
 */
export function dirWithHysteresis(x: number, y: number, prev: number, deadzone: number, marginDeg = 5): number {
  const mag = Math.hypot(x, y);
  if (mag <= deadzone) return 0;
  if (prev >= 1 && prev <= 8) {
    const angle = Math.atan2(x, -y); // 0 = north, clockwise
    const center = ((prev - 1) * Math.PI) / 4;
    let diff = Math.abs(angle - center) % (2 * Math.PI);
    if (diff > Math.PI) diff = 2 * Math.PI - diff;
    if (diff <= ((22.5 + marginDeg) * Math.PI) / 180) return prev;
  }
  return dirFromVector(x, y);
}

export function encode(f: InputFrame, dir: number): number {
  let bits = dir & 0xf;
  if (f.fire) bits |= Bits.FIRE;
  if (f.place) bits |= Bits.PLACE;
  if (f.strafe) bits |= Bits.STRAFE;
  if (f.weaponNext) bits |= Bits.WEAPON_NEXT;
  if (f.weaponPrev) bits |= Bits.WEAPON_PREV;
  if (f.reload) bits |= Bits.RELOAD;
  return bits;
}

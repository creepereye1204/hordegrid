import type { InputFrame } from './encode';

/** Standard-mapping gamepad (docs/FRONTEND.md#입력). */
export class GamepadInput {
  connected(): boolean {
    return [...(navigator.getGamepads?.() ?? [])].some((p) => p?.connected);
  }

  apply(f: InputFrame): void {
    const pad = [...(navigator.getGamepads?.() ?? [])].find((p) => p?.connected);
    if (!pad) return;
    const [lx = 0, ly = 0] = pad.axes;
    if (Math.hypot(lx, ly) > 0.3) {
      f.moveX += lx;
      f.moveY += ly;
    }
    const pressed = (i: number): boolean => pad.buttons[i]?.pressed ?? false;
    f.fire ||= pressed(7) || pressed(0);
    f.place ||= pressed(3);
    f.strafe ||= pressed(6);
    f.weaponNext ||= pressed(5);
    f.weaponPrev ||= pressed(4);
  }
}

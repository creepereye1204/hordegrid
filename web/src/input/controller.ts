import { dirFromVector, dirWithHysteresis, emptyInput, encode, type InputFrame } from './encode';
import { GamepadInput } from './gamepad';
import { KeyboardInput } from './keyboard';
import type { TouchInput } from './touch';

/** Merges all devices into one 16-bit command per fixed step. */
export class InputController {
  private readonly keyboard = new KeyboardInput();
  private readonly gamepad = new GamepadInput();
  private lastDir = 0;
  private strafeFacingSet = false;

  constructor(private readonly touch: TouchInput) {}

  attach(): void {
    this.keyboard.attach();
    this.touch.attach();
  }

  sample(): number {
    const f: InputFrame = emptyInput();
    this.keyboard.apply(f);
    this.gamepad.apply(f);
    this.touch.apply(f);

    const touchOnly = this.touch.isVisible && !this.keyboard.active && !this.gamepad.connected();
    let dir: number;
    if (touchOnly) {
      const s = this.touch.stick;
      dir = dirWithHysteresis(f.moveX, f.moveY, this.lastDir, s.deadzone);
    } else {
      dir = dirFromVector(f.moveX, f.moveY);
    }

    // touch: the first frame of firing turns toward the stick before strafe locks facing
    if (touchOnly && f.fire && !this.strafeFacingSet) {
      f.strafe = false;
      this.strafeFacingSet = true;
    } else if (!f.fire) {
      this.strafeFacingSet = false;
    }

    this.lastDir = dir;
    return encode(f, dir);
  }

  /** Show touch controls on coarse pointers unless a gamepad is in use. */
  refreshTouchVisibility(inGame: boolean): void {
    const coarse = window.matchMedia('(pointer: coarse)').matches;
    this.touch.setVisible(inGame && coarse && !this.gamepad.connected());
  }
}

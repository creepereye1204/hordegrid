import type { InputFrame } from './encode';

const MAP: Record<string, keyof Pick<InputFrame, 'fire' | 'place' | 'strafe' | 'weaponNext' | 'weaponPrev' | 'reload'> | 'up' | 'down' | 'left' | 'right'> = {
  KeyW: 'up', ArrowUp: 'up',
  KeyS: 'down', ArrowDown: 'down',
  KeyA: 'left', ArrowLeft: 'left',
  KeyD: 'right', ArrowRight: 'right',
  Space: 'fire', KeyJ: 'fire',
  KeyF: 'place', KeyK: 'place',
  ShiftLeft: 'strafe', ShiftRight: 'strafe',
  KeyE: 'weaponNext', KeyQ: 'weaponPrev',
  KeyR: 'reload',
};

export class KeyboardInput {
  private readonly down = new Set<string>();
  private readonly onKey = (e: KeyboardEvent): void => {
    if (!(e.code in MAP)) return;
    if (e.target instanceof HTMLInputElement) return;
    e.preventDefault();
    if (e.type === 'keydown') this.down.add(MAP[e.code]!);
    else this.down.delete(MAP[e.code]!);
  };
  private readonly onBlur = (): void => this.down.clear();

  attach(): void {
    window.addEventListener('keydown', this.onKey);
    window.addEventListener('keyup', this.onKey);
    window.addEventListener('blur', this.onBlur);
  }

  detach(): void {
    window.removeEventListener('keydown', this.onKey);
    window.removeEventListener('keyup', this.onKey);
    window.removeEventListener('blur', this.onBlur);
    this.down.clear();
  }

  get active(): boolean {
    return this.down.size > 0;
  }

  apply(f: InputFrame): void {
    const d = this.down;
    f.moveX += (d.has('right') ? 1 : 0) - (d.has('left') ? 1 : 0);
    f.moveY += (d.has('down') ? 1 : 0) - (d.has('up') ? 1 : 0);
    f.fire ||= d.has('fire');
    f.place ||= d.has('place');
    f.strafe ||= d.has('strafe');
    f.weaponNext ||= d.has('weaponNext');
    f.weaponPrev ||= d.has('weaponPrev');
    f.reload ||= d.has('reload');
  }
}

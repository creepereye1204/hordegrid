import type { InputFrame } from './encode';

const STICK_RADIUS = 56; // px (≈ dp at dpr 1)
const DEADZONE = 0.2;

/**
 * Floating stick (left half) + action buttons (docs/product-specs/mobile-play.md#조작-터치).
 * Pointer Events only; each pointer is tracked independently so move + fire work together.
 */
export class TouchInput {
  private stickId: number | null = null;
  private origin = { x: 0, y: 0 };
  private vec = { x: 0, y: 0 };
  private readonly held = new Map<number, 'fire' | 'next' | 'place'>();
  private visible = false;

  constructor(
    private readonly root: HTMLElement,
    private readonly base: HTMLElement,
    private readonly knob: HTMLElement,
  ) {}

  attach(): void {
    this.root.addEventListener('pointerdown', this.onDown);
    this.root.addEventListener('pointermove', this.onMove);
    this.root.addEventListener('pointerup', this.onUp);
    this.root.addEventListener('pointercancel', this.onUp);
    this.root.addEventListener('contextmenu', (e) => e.preventDefault());
  }

  setVisible(v: boolean): void {
    this.visible = v;
    this.root.hidden = !v;
    if (!v) this.reset();
  }

  get isVisible(): boolean {
    return this.visible;
  }

  private reset(): void {
    this.stickId = null;
    this.vec = { x: 0, y: 0 };
    this.held.clear();
    this.base.hidden = true;
    for (const b of this.root.querySelectorAll('[data-action]')) b.classList.remove('pressed');
  }

  private readonly onDown = (e: PointerEvent): void => {
    const target = e.target as HTMLElement;
    const action = target.closest<HTMLElement>('[data-action]')?.dataset['action'];
    this.root.setPointerCapture?.(e.pointerId);
    e.preventDefault();
    if (action === 'fire' || action === 'next' || action === 'place') {
      this.held.set(e.pointerId, action);
      target.closest('[data-action]')?.classList.add('pressed');
      navigator.vibrate?.(8);
      return;
    }
    if (this.stickId === null && e.clientX < window.innerWidth / 2) {
      this.stickId = e.pointerId;
      this.origin = { x: e.clientX, y: e.clientY };
      this.vec = { x: 0, y: 0 };
      this.base.hidden = false;
      this.base.style.transform = `translate(${e.clientX - STICK_RADIUS}px, ${e.clientY - STICK_RADIUS}px)`;
      this.knob.style.transform = 'translate(0px, 0px)';
    }
  };

  private readonly onMove = (e: PointerEvent): void => {
    if (e.pointerId !== this.stickId) return;
    e.preventDefault();
    let dx = e.clientX - this.origin.x;
    let dy = e.clientY - this.origin.y;
    const len = Math.hypot(dx, dy);
    if (len > STICK_RADIUS) {
      dx = (dx / len) * STICK_RADIUS;
      dy = (dy / len) * STICK_RADIUS;
    }
    this.vec = { x: dx / STICK_RADIUS, y: dy / STICK_RADIUS };
    this.knob.style.transform = `translate(${dx}px, ${dy}px)`;
  };

  private readonly onUp = (e: PointerEvent): void => {
    if (e.pointerId === this.stickId) {
      this.stickId = null;
      this.vec = { x: 0, y: 0 };
      this.base.hidden = true;
    }
    const action = this.held.get(e.pointerId);
    if (action) {
      this.held.delete(e.pointerId);
      if (![...this.held.values()].includes(action)) {
        this.root.querySelector(`[data-action="${action}"]`)?.classList.remove('pressed');
      }
    }
  };

  /** Stick magnitude past the deadzone, for hysteresis quantization by the caller. */
  get stick(): { x: number; y: number; deadzone: number } {
    return { ...this.vec, deadzone: DEADZONE };
  }

  apply(f: InputFrame): void {
    if (!this.visible) return;
    f.moveX += this.vec.x;
    f.moveY += this.vec.y;
    const actions = new Set(this.held.values());
    const firing = actions.has('fire');
    f.fire ||= firing;
    // mobile: firing locks facing so you can back away while shooting
    f.strafe ||= firing;
    f.weaponNext ||= actions.has('next');
    f.place ||= actions.has('place');
  }
}

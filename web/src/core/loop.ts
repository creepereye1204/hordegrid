/** Fixed 60 Hz simulation with variable-rate rendering (docs/FRONTEND.md#게임-루프). */
export class FixedLoop {
  static readonly STEP_MS = 1000 / 60;
  private acc = 0;
  private last = 0;
  private raf = 0;
  private running = false;

  constructor(
    private readonly step: () => void,
    private readonly render: (alpha: number) => void,
  ) {}

  start(): void {
    if (this.running) return;
    this.running = true;
    this.last = performance.now();
    this.acc = 0;
    const frame = (now: number): void => {
      if (!this.running) return;
      // clamp: a tab returning from background must not replay seconds of steps at once
      this.acc += Math.min(now - this.last, 250);
      this.last = now;
      while (this.acc >= FixedLoop.STEP_MS) {
        this.step();
        this.acc -= FixedLoop.STEP_MS;
      }
      this.render(this.acc / FixedLoop.STEP_MS);
      this.raf = requestAnimationFrame(frame);
    };
    this.raf = requestAnimationFrame(frame);
  }

  stop(): void {
    this.running = false;
    cancelAnimationFrame(this.raf);
  }
}

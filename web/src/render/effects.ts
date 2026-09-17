// Render-only feedback. Owned by JS, never touches the sim, safe under rollback (docs/DESIGN.md#모션--피드백).
interface Particle {
  x: number;
  y: number;
  vx: number;
  vy: number;
  life: number;
  max: number;
  color: string;
  size: number;
}

export class Effects {
  private particles: Particle[] = [];
  shake = 0;
  private readonly reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  burst(x: number, y: number, color: string, count: number, speed: number): void {
    for (let i = 0; i < count; i++) {
      const a = Math.random() * Math.PI * 2;
      const s = speed * (0.4 + Math.random() * 0.6);
      const max = 18 + Math.random() * 14;
      this.particles.push({ x, y, vx: Math.cos(a) * s, vy: Math.sin(a) * s, life: max, max, color, size: 0.08 + Math.random() * 0.08 });
    }
    if (this.particles.length > 600) this.particles.splice(0, this.particles.length - 600);
  }

  addShake(amount: number): void {
    if (!this.reducedMotion) this.shake = Math.min(0.35, this.shake + amount);
  }

  update(): void {
    for (const p of this.particles) {
      p.x += p.vx;
      p.y += p.vy;
      p.vx *= 0.9;
      p.vy *= 0.9;
      p.life--;
    }
    this.particles = this.particles.filter((p) => p.life > 0);
    this.shake *= 0.85;
  }

  draw(ctx: CanvasRenderingContext2D, tile: number, ox: number, oy: number): void {
    for (const p of this.particles) {
      ctx.globalAlpha = p.life / p.max;
      ctx.fillStyle = p.color;
      const s = p.size * tile;
      ctx.fillRect((p.x - ox) * tile - s / 2, (p.y - oy) * tile - s / 2, s, s);
    }
    ctx.globalAlpha = 1;
  }
}

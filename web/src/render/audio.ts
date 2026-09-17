// Tiny synthesized SFX (no assets). Created lazily after a user gesture (autoplay policy).
type Sfx = 'fire' | 'kill' | 'hurt' | 'down' | 'unlock' | 'wave';

export class Audio {
  private ctx: AudioContext | null = null;
  private lastPlayed = new Map<Sfx, number>();
  muted = false;

  unlock(): void {
    if (this.ctx) return;
    try {
      this.ctx = new AudioContext();
    } catch {
      this.ctx = null;
    }
  }

  play(kind: Sfx): void {
    const ctx = this.ctx;
    if (!ctx || this.muted) return;
    const now = ctx.currentTime;
    // rate-limit per kind: a shotgun volley shouldn't stack 20 voices
    const minGap = kind === 'fire' ? 0.05 : 0.03;
    if (now - (this.lastPlayed.get(kind) ?? 0) < minGap) return;
    this.lastPlayed.set(kind, now);

    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    osc.connect(gain).connect(ctx.destination);
    const spec: Record<Sfx, [OscillatorType, number, number, number, number]> = {
      fire: ['square', 520, 180, 0.04, 0.05],
      kill: ['triangle', 160, 50, 0.12, 0.12],
      hurt: ['sawtooth', 220, 90, 0.15, 0.1],
      down: ['sawtooth', 300, 60, 0.5, 0.14],
      unlock: ['triangle', 440, 880, 0.35, 0.12],
      wave: ['square', 330, 660, 0.25, 0.08],
    };
    const [type, f0, f1, dur, vol] = spec[kind];
    osc.type = type;
    osc.frequency.setValueAtTime(f0, now);
    osc.frequency.exponentialRampToValueAtTime(f1, now + dur);
    gain.gain.setValueAtTime(vol, now);
    gain.gain.exponentialRampToValueAtTime(0.0001, now + dur);
    osc.start(now);
    osc.stop(now + dur);
  }
}

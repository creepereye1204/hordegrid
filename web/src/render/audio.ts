// SFX: real CC0 samples for weapon fire/hit (see CREDITS.md), synthesized tones for everything
// else. Samples are decoded lazily after the unlock() gesture; until ready, fire falls back to
// the old synthesized blip so there's never a silent trigger pull.
import fireRocketUrl from '../assets/audio/fire-rocket.ogg';
import fireShotgunUrl from '../assets/audio/fire-shotgun.ogg';
import fireSmgUrl from '../assets/audio/fire-smg.ogg';
import firePistolUrl from '../assets/audio/fire-pistol.ogg';
import hitUrl from '../assets/audio/hit.ogg';

type Sfx = 'fire' | 'kill' | 'hurt' | 'down' | 'unlock' | 'wave' | 'reload';

const FIRE_SAMPLE_URLS = [firePistolUrl, fireSmgUrl, fireShotgunUrl, fireRocketUrl];

export class Audio {
  private ctx: AudioContext | null = null;
  private readonly buffers = new Map<string, AudioBuffer>();
  private readonly lastPlayed = new Map<string, number>();
  muted = false;

  unlock(): void {
    if (this.ctx) return;
    try {
      this.ctx = new AudioContext();
    } catch {
      this.ctx = null;
    }
    void this.preloadSamples();
  }

  private async preloadSamples(): Promise<void> {
    const ctx = this.ctx;
    if (!ctx) return;
    const entries: [string, string][] = FIRE_SAMPLE_URLS.map((url, i) => [`fire${i}`, url]);
    entries.push(['hit', hitUrl]);
    await Promise.all(
      entries.map(async ([key, url]) => {
        try {
          const buf = await ctx.decodeAudioData(await (await fetch(url)).arrayBuffer());
          this.buffers.set(key, buf);
        } catch {
          /* sample failed to load — playFire()/playHit() fall back silently or to synth */
        }
      }),
    );
  }

  /** Play a decoded sample. Returns false if it isn't loaded yet (caller may fall back). */
  private playSample(key: string, minGap: number, gain: number): boolean {
    const ctx = this.ctx;
    const buf = this.buffers.get(key);
    if (!ctx || !buf) return false;
    if (this.muted) return true; // loaded, just silenced — don't let the caller fall back to synth
    const now = ctx.currentTime;
    if (now - (this.lastPlayed.get(key) ?? 0) < minGap) return true;
    this.lastPlayed.set(key, now);
    const src = ctx.createBufferSource();
    const g = ctx.createGain();
    g.gain.value = gain;
    src.buffer = buf;
    src.connect(g).connect(ctx.destination);
    src.start(now);
    return true;
  }

  /** Weapon-specific gunfire sample (falls back to the synthesized 'fire' blip while loading). */
  playFire(weaponKind: number): void {
    const key = `fire${FIRE_SAMPLE_URLS[weaponKind] ? weaponKind : 0}`;
    if (!this.playSample(key, 0.05, 0.35)) this.play('fire');
  }

  /** Bullet-impact sample; silently skipped if not loaded (the visual burst still reads fine alone). */
  playHit(): void {
    this.playSample('hit', 0.06, 0.3);
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
      reload: ['sine', 200, 400, 0.2, 0.08],
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

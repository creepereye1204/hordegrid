// One running game: owns the wasm Game, fixed loop, renderer feed and packet plumbing.
import { FixedLoop } from './core/loop';
import { EventKind, GameMode, H, HEADER, HideSeekPhase, Life, NetEventCode, P, Phase, PLAYER_STRIDE, WEAPON_NAMES } from './core/view-layout';
import { iterOutbox, newGame, renderView, type Game } from './core/wasm';
import type { InputController } from './input/controller';
import type { Audio } from './render/audio';
import type { Effects } from './render/effects';
import { C } from './render/palette';
import type { Renderer } from './render/renderer';
import { $, banner, setText, toast } from './ui/dom';

export interface MatchNet {
  /** Peer ids indexed by player handle (local included). */
  players: readonly string[];
  send(peerId: string, bytes: Uint8Array): void;
}

export interface SurvivalResult {
  mode: 0;
  wave: number;
  score: number;
  bestCombo: number;
  kills: number[];
}

export interface HideSeekResult {
  mode: 1;
  /** 1 = hiders won, 2 = seeker won. */
  winner: number;
  /** Local player's role this round: 0 hider, 1 seeker. */
  role: number;
}

export type MatchResult = SurvivalResult | HideSeekResult;

export class Match {
  readonly game: Game;
  private readonly loop: FixedLoop;
  private over = false;
  private paused = false;
  private lastCombo = 0;
  private netHud = '';
  private hudFrame = 0;

  constructor(
    private readonly deps: { renderer: Renderer; effects: Effects; audio: Audio; input: InputController },
    readonly localHandle: number,
    seed: number,
    private readonly net: MatchNet | null,
    private readonly onOver: (r: MatchResult) => void,
    private readonly mode: 0 | 1 = 0,
  ) {
    const numPlayers = net ? net.players.length : 1;
    this.game = newGame(numPlayers, localHandle, seed, 0, mode);
    deps.renderer.setMap(this.game.map_width(), this.game.map_height(), this.game.map_tiles());
    deps.renderer.resize();
    deps.renderer.pushStep(renderView(this.game));
    this.loop = new FixedLoop(
      () => this.step(),
      (alpha) => this.draw(alpha),
    );
  }

  start(): void {
    this.loop.start();
  }

  stop(): void {
    this.loop.stop();
    this.game.free();
  }

  setPaused(p: boolean): void {
    // only solo can truly pause; a networked peer that stops ticking stalls everyone
    this.paused = p && !this.net;
  }

  /** Datagram from a remote peer. */
  receive(fromPeer: string, bytes: Uint8Array): void {
    if (!this.net) return;
    const slot = this.net.players.indexOf(fromPeer);
    if (slot >= 0 && slot !== this.localHandle) this.game.push_packet(slot, bytes);
  }

  private step(): void {
    if (this.paused) return;
    const bits = this.deps.input.sample();
    const status = this.game.tick(bits);
    if (this.net) {
      for (const [slot, bytes] of iterOutbox(this.game.take_outbox())) {
        const peer = this.net.players[slot];
        if (peer) this.net.send(peer, bytes);
      }
      this.handleNetEvents(status);
    }
    this.deps.renderer.pushStep(renderView(this.game));
    this.handleEvents();
    this.deps.effects.update();
  }

  private handleNetEvents(status: number): void {
    const ev = this.game.take_net_events();
    for (let i = 0; i + 2 < ev.length; i += 3) {
      const code = ev[i]!;
      const slot = ev[i + 1]!;
      const who = `${slot + 1}P`;
      if (code === NetEventCode.Interrupted) toast(`${who} 연결 불안정`);
      else if (code === NetEventCode.Resumed) toast(`${who} 연결 회복`);
      else if (code === NetEventCode.Disconnected) toast(`${who} 연결 끊김`, 4000);
      else if (code === NetEventCode.Desync) toast('⚠ 동기화 불일치 감지', 5000);
    }
    this.netHud = status === 1 ? '동기화 중…' : '';
  }

  private handleEvents(): void {
    const ev = this.game.take_events();
    const { effects, audio } = this.deps;
    for (let i = 0; i + 4 < ev.length; i += 5) {
      const kind = ev[i + 1]!;
      const a = ev[i + 2]!;
      const x = ev[i + 3]! / 65536;
      const y = ev[i + 4]! / 65536;
      switch (kind) {
        case EventKind.Fire:
          audio.play('fire');
          break;
        case EventKind.Hit:
          effects.burst(x, y, '#d8d8c0', 2, 0.08);
          break;
        case EventKind.Kill:
          effects.burst(x, y, a === 1 ? C.runner : C.walker, 10, 0.16);
          audio.play('kill');
          break;
        case EventKind.PlayerHurt:
          if (a === this.localHandle) {
            effects.addShake(0.12);
            audio.play('hurt');
            navigator.vibrate?.(25);
          }
          break;
        case EventKind.Down:
          toast(a === this.localHandle ? '쓰러짐! 동료가 와서 살려줘야 함' : `${a + 1}P 쓰러짐 — 가서 살려주기`);
          audio.play('down');
          if (a === this.localHandle) navigator.vibrate?.([60, 40, 60]);
          break;
        case EventKind.Revive:
          toast(`${a + 1}P 부활`);
          break;
        case EventKind.Unlock:
          banner(`새 무기: ${WEAPON_NAMES[a] ?? '?'}  (Q/E · ⇄)`, 2000);
          audio.play('unlock');
          break;
        case EventKind.WaveStart:
          banner(`WAVE ${a}`);
          audio.play('wave');
          break;
        case EventKind.Found:
          toast(a === this.localHandle ? '들켰습니다!' : `${a + 1}P 발견`);
          audio.play('down');
          if (a === this.localHandle) navigator.vibrate?.([80, 40, 80]);
          break;
        case EventKind.RoundEnd:
          banner(a === 2 ? '술래 승리' : '숨는 사람 승리');
          audio.play('wave');
          break;
        default:
          break;
      }
    }
  }

  private draw(alpha: number): void {
    const { renderer, effects } = this.deps;
    renderer.draw(alpha, effects);
    const v = renderer.latest;
    if (!v) return;
    if (this.mode === GameMode.HideSeek) {
      if (++this.hudFrame % 3 === 0) this.updateHideSeekHud(v);
      if (!this.over && v[H.PHASE] === HideSeekPhase.RoundOver) {
        this.over = true;
        const role = v[HEADER + this.localHandle * PLAYER_STRIDE + P.ROLE]!;
        this.onOver({ mode: 1, winner: v[H.HS_WINNER]!, role });
      }
      return;
    }
    if (++this.hudFrame % 3 === 0) this.updateHud(v);
    if (!this.over && v[H.PHASE] === Phase.GameOver) {
      this.over = true;
      const kills: number[] = [];
      for (let i = 0; i < (v[H.NUM_PLAYERS] ?? 1); i++) kills.push(v[HEADER + i * PLAYER_STRIDE + P.KILLS]!);
      this.onOver({ mode: 0, wave: v[H.WAVE]!, score: v[H.SCORE]!, bestCombo: v[H.BEST_COMBO]!, kills });
    }
  }

  private updateHideSeekHud(v: Int32Array): void {
    const phase = v[H.PHASE]!;
    const timerSec = Math.ceil(v[H.TIMER]! / 60);
    const o = HEADER + this.localHandle * PLAYER_STRIDE;
    const isSeeker = v[o + P.ROLE] === 1;
    const phaseText =
      phase === HideSeekPhase.Hiding ? `숨는 시간 ${timerSec}초` : phase === HideSeekPhase.Seeking ? `추격 중 ${timerSec}초` : '라운드 종료';
    setText($('hud-wave'), `${isSeeker ? '술래' : '숨는 사람'} · ${phaseText}`);
    let found = 0;
    const numPlayers = v[H.NUM_PLAYERS] ?? 1;
    for (let i = 0; i < numPlayers; i++) {
      const oi = HEADER + i * PLAYER_STRIDE;
      if (v[oi + P.ROLE] === 0 && v[oi + P.FOUND] === 1) found++;
    }
    setText($('hud-score'), `${found}명 발견`);
    setText($('hud-combo'), '');
    setText($('hud-weapon'), '');
    let net = this.netHud;
    if (!net && this.net) {
      const pings: string[] = [];
      for (let i = 0; i < this.net.players.length; i++) {
        if (i === this.localHandle) continue;
        const p = this.game.ping_ms(i);
        pings.push(p >= 0 ? `${i + 1}P ${p}ms` : `${i + 1}P -`);
      }
      net = pings.join(' · ');
    }
    setText($('hud-net'), net);
  }

  private updateHud(v: Int32Array): void {
    const wave = v[H.WAVE]!;
    const phase = v[H.PHASE]!;
    const timerSec = Math.ceil(v[H.TIMER]! / 60);
    setText($('hud-wave'), phase === Phase.Intermission ? (wave === 0 ? `시작까지 ${timerSec}` : `다음 웨이브 ${timerSec}`) : `WAVE ${wave}`);
    setText($('hud-score'), v[H.SCORE]!.toLocaleString('ko-KR'));
    const combo = v[H.COMBO]!;
    const comboEl = $('hud-combo');
    setText(comboEl, combo >= 2 ? `x${combo} COMBO` : '');
    if (combo >= 10 && combo !== this.lastCombo && combo % 5 === 0) {
      comboEl.classList.add('pulse');
      window.setTimeout(() => comboEl.classList.remove('pulse'), 120);
    }
    this.lastCombo = combo;
    const o = HEADER + this.localHandle * PLAYER_STRIDE;
    const life = v[o + P.LIFE]!;
    const weapon = WEAPON_NAMES[v[o + P.WEAPON]!] ?? '?';
    const status = life === Life.Downed ? '쓰러짐' : life === Life.Dead ? '다음 웨이브에 부활' : `HP ${Math.max(0, v[o + P.HP]!)}`;
    setText($('hud-weapon'), `${weapon} · ${status}`);
    let net = this.netHud;
    if (!net && this.net) {
      const pings: string[] = [];
      for (let i = 0; i < this.net.players.length; i++) {
        if (i === this.localHandle) continue;
        const p = this.game.ping_ms(i);
        pings.push(p >= 0 ? `${i + 1}P ${p}ms` : `${i + 1}P -`);
      }
      net = pings.join(' · ');
    }
    setText($('hud-net'), net);
  }
}

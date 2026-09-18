import './styles.css';
import { formatCode, newInviteCode, normalizeCode, randomNickname, randomU32, sanitizeName } from './core/ids';
import { load, save } from './core/storage';
import { loadWasm } from './core/wasm';
import { InputController } from './input/controller';
import { TouchInput } from './input/touch';
import { Match, type MatchResult } from './match';
import { GameMode } from './core/view-layout';
import { Lobby, type LobbyEvent, type Outgoing } from './net/lobby';
import type { DenyReason } from './net/protocol';
import { NetRoom } from './net/room';
import { Audio } from './render/audio';
import { Effects } from './render/effects';
import { Renderer } from './render/renderer';
import { $, el, PLAYER_COLORS, showScreen } from './ui/dom';

const BUILD = __BUILD_HASH__;

const renderer = new Renderer($<HTMLCanvasElement>('game'));
const effects = new Effects();
const audio = new Audio();
const touch = new TouchInput($('touch'), $('stick-base'), $('stick-knob'));
const input = new InputController(touch);
input.attach();

interface Online {
  code: string;
  room: NetRoom;
  lobby: Lobby;
  timer: number;
  links: Set<string>;
}

let online: Online | null = null;
let match: Match | null = null;
let hiddenSince = 0;

const DENY_TEXT: Record<DenyReason, string> = {
  rejected: '방장이 입장을 거절했습니다.',
  timeout: '방장이 응답하지 않았습니다.',
  full: '방이 가득 찼습니다 (최대 4명).',
  started: '이미 게임이 진행 중입니다.',
  version: '버전이 다릅니다. 둘 다 새로고침 후 다시 시도하세요.',
  kicked: '방에서 내보내졌습니다.',
};

function nickname(): string {
  return sanitizeName($<HTMLInputElement>('nickname').value);
}

// ---------------------------------------------------------------- game mode

let selectedMode: 0 | 1 = GameMode.Survival;
const modeButtons: Record<0 | 1, HTMLElement> = { [GameMode.Survival]: $('mode-survival'), [GameMode.HideSeek]: $('mode-hideseek') };

function setMode(m: 0 | 1): void {
  selectedMode = m;
  for (const [k, btn] of Object.entries(modeButtons)) {
    const active = Number(k) === m;
    btn.classList.toggle('active', active);
    btn.setAttribute('aria-pressed', String(active));
  }
}

modeButtons[GameMode.Survival].addEventListener('click', () => setMode(GameMode.Survival));
modeButtons[GameMode.HideSeek].addEventListener('click', () => setMode(GameMode.HideSeek));

function relayOverride(): string[] {
  return new URLSearchParams(location.search).getAll('relay').filter((u) => /^wss?:\/\/[\w.-]+(:\d+)?(\/[\w./-]*)?$/.test(u));
}

// ---------------------------------------------------------------- title

function toTitle(message = ''): void {
  void leaveOnline();
  stopMatch();
  history.replaceState(null, '', location.pathname + location.search);
  $('title-msg').textContent = message;
  showScreen('screen-title');
  input.refreshTouchVisibility(false);
}

$('btn-create').addEventListener('click', () => {
  audio.unlock();
  enterRoom(newInviteCode(), 'host');
});

$('form-join').addEventListener('submit', (e) => {
  e.preventDefault();
  audio.unlock();
  const code = normalizeCode($<HTMLInputElement>('join-code').value);
  if (!code) {
    $('title-msg').textContent = '초대코드는 숫자 8자리입니다.';
    return;
  }
  enterRoom(code, 'guest');
});

$('join-code').addEventListener('input', (e) => {
  const inputEl = e.target as HTMLInputElement;
  const digits = inputEl.value.replace(/\D/g, '').slice(0, 8);
  inputEl.value = digits.length > 4 ? `${digits.slice(0, 4)} ${digits.slice(4)}` : digits;
});

$('btn-solo').addEventListener('click', () => {
  audio.unlock();
  startMatch(0, randomU32(), null);
});

$('nickname').addEventListener('change', () => save('name', nickname()));

// ---------------------------------------------------------------- lobby

function enterRoom(code: string, role: 'host' | 'guest'): void {
  void leaveOnline();
  history.replaceState(null, '', `${location.pathname}${location.search}#/r/${code}`);
  const link = `${location.origin}${location.pathname}${location.search}#/r/${code}`;
  $('lobby-code').textContent = formatCode(code);
  $<HTMLInputElement>('lobby-link').value = link;
  $('knocks').replaceChildren();
  $('lobby-members').replaceChildren();

  const links = new Set<string>();
  const joinedAt = Date.now();
  let lobby!: Lobby;
  const room = new NetRoom(
    code,
    {
      peerJoined: (id) => dispatch(lobby.peerJoined(id)),
      peerLeft: (id) => dispatch(lobby.peerLeft(id)),
      lobby: (from, msg) => dispatch(lobby.receive(from, msg, Date.now())),
      link: (id, open) => {
        if (open) links.add(id);
        else links.delete(id);
        dispatch(lobby.linkChanged(id, open));
      },
      game: (from, bytes) => match?.receive(from, bytes),
    },
    relayOverride(),
  );
  lobby = new Lobby(room.selfId, nickname(), role, BUILD);
  const timer = window.setInterval(() => {
    dispatch(lobby.tick(Date.now()));
    renderLobbyStatus(joinedAt);
  }, 1000);
  online = { code, room, lobby, timer, links };

  $('btn-ready').hidden = role !== 'guest';
  $('btn-start').hidden = role !== 'host';
  $<HTMLButtonElement>('btn-ready').disabled = true;
  $('btn-ready').textContent = '준비';
  $('btn-ready').dataset['ready'] = '0';
  renderLobby();
  showScreen('screen-lobby');
}

function dispatch(out: Outgoing[]): void {
  const o = online;
  if (!o) return;
  for (const { to, msg } of out) {
    const targets = to === 'members' ? o.lobby.memberIds() : [to];
    o.room.sendLobby(targets, msg);
  }
  const events = o.lobby.events.splice(0);
  for (const ev of events) handleLobbyEvent(ev);
  renderLobby();
}

function handleLobbyEvent(ev: LobbyEvent): void {
  const o = online;
  if (!o) return;
  switch (ev.kind) {
    case 'knock':
      navigator.vibrate?.(30);
      break;
    case 'denied':
      toTitle(DENY_TEXT[ev.reason]);
      break;
    case 'host-left':
      if (!match) toTitle('방장이 방을 나갔습니다.');
      break;
    case 'start': {
      const local = ev.players.indexOf(o.room.selfId);
      if (local < 0) return;
      startMatch(local, ev.seed, ev.players, ev.mode === GameMode.HideSeek ? 1 : 0);
      break;
    }
    case 'back-to-lobby':
      stopMatch();
      $('btn-ready').dataset['ready'] = '0';
      $('btn-ready').textContent = '준비';
      showScreen('screen-lobby');
      break;
    default:
      break;
  }
}

function renderLobby(): void {
  const o = online;
  if (!o) return;
  const { lobby } = o;
  const list = $('lobby-members');
  list.replaceChildren(
    ...lobby.currentRoster.map((m, i) => {
      const linked = m.id === o.room.selfId || o.links.has(m.id);
      return el('li', {}, [
        Object.assign(el('span', { className: 'dot', text: String(i + 1) }), { style: `background:${PLAYER_COLORS[i] ?? '#888'}` }),
        el('span', { className: 'name', text: `${m.name}${m.id === o.room.selfId ? ' (나)' : ''}` }),
        el('span', { className: 'badge', text: m.host ? '방장' : linked ? '연결됨' : '연결 중' }),
        el('span', { className: `badge ${m.ready ? 'ok' : ''}`, text: m.ready ? '준비' : '대기' }),
      ]);
    }),
  );

  const knocks = $('knocks');
  knocks.replaceChildren(
    ...lobby.pendingKnocks.map(({ id, name }) => {
      const yes = el('button', { className: 'primary', text: '수락' });
      const no = el('button', { className: 'ghost', text: '거절' });
      yes.addEventListener('click', () => dispatch(lobby.approve(id)));
      no.addEventListener('click', () => dispatch(lobby.reject(id, Date.now())));
      return el('div', { className: 'knock' }, [el('span', { text: `${name} 입장 요청` }), yes, no]);
    }),
  );

  if (lobby.role === 'host') {
    const blocker = lobby.startBlocker();
    $<HTMLButtonElement>('btn-start').disabled = blocker !== null;
    $('btn-start').textContent = blocker ?? '시작';
  } else {
    $<HTMLButtonElement>('btn-ready').disabled = !lobby.isAdmitted;
  }
}

function renderLobbyStatus(joinedAt: number): void {
  const o = online;
  if (!o || match) return;
  const relays = o.room.relayCount();
  const waited = Math.floor((Date.now() - joinedAt) / 1000);
  let text: string;
  if (relays === 0 && waited > 10) text = '매칭 릴레이에 연결하지 못했습니다. 다른 네트워크에서 시도해 보세요.';
  else if (o.lobby.role === 'guest' && !o.lobby.isAdmitted) {
    text = waited > 30 ? '방이 없거나, 네트워크가 직접 연결을 막고 있을 수 있습니다. 한 명이 Wi-Fi로 바꿔 보세요.' : '방장 승인 대기 중…';
  } else if (o.lobby.currentRoster.length < 2) {
    text = waited > 30 ? '친구가 안 보이면: 한 명이 Wi-Fi로 전환 후 다시 참가' : '링크나 코드를 친구에게 보내세요.';
  } else text = `릴레이 ${relays}개 연결됨`;
  $('lobby-status').textContent = text;
}

$('btn-copy').addEventListener('click', async () => {
  const inputEl = $<HTMLInputElement>('lobby-link');
  try {
    await navigator.clipboard.writeText(inputEl.value);
    $('btn-copy').textContent = '복사됨';
    window.setTimeout(() => ($('btn-copy').textContent = '링크 복사'), 1500);
  } catch {
    inputEl.select();
  }
});

$('btn-ready').addEventListener('click', () => {
  const o = online;
  if (!o) return;
  const ready = $('btn-ready').dataset['ready'] !== '1';
  $('btn-ready').dataset['ready'] = ready ? '1' : '0';
  $('btn-ready').textContent = ready ? '준비 취소' : '준비';
  dispatch(o.lobby.setReady(ready));
});

$('btn-start').addEventListener('click', () => {
  const o = online;
  if (o) dispatch(o.lobby.start(randomU32(), selectedMode));
});

$('btn-leave').addEventListener('click', () => toTitle());

async function leaveOnline(): Promise<void> {
  const o = online;
  online = null;
  if (!o) return;
  window.clearInterval(o.timer);
  await o.room.leave().catch(() => undefined);
}

// ---------------------------------------------------------------- match

function startMatch(localHandle: number, seed: number, players: string[] | null, mode: 0 | 1 = 0): void {
  stopMatch();
  const o = online;
  const net = players && o ? { players, send: (peer: string, bytes: Uint8Array) => o.room.sendGame(peer, bytes) } : null;
  match = new Match({ renderer, effects, audio, input }, localHandle, seed, net, onGameOver, mode);
  showScreen('game');
  input.refreshTouchVisibility(true);
  void requestWakeLock();
  match.start();
}

function stopMatch(): void {
  match?.stop();
  match = null;
  releaseWakeLock();
}

function onGameOver(r: MatchResult): void {
  if (r.mode === GameMode.HideSeek) {
    const localWon = (r.role === 1 && r.winner === 2) || (r.role === 0 && r.winner === 1);
    $('over-summary').textContent = `${localWon ? '승리' : '패배'} · ${r.winner === 2 ? '술래가 모두 찾았습니다' : '숨는 사람이 끝까지 버텼습니다'}`;
    $('over-players').replaceChildren();
  } else {
    $('over-summary').textContent = `WAVE ${r.wave} · 점수 ${r.score.toLocaleString('ko-KR')} · 최고 콤보 x${r.bestCombo}`;
    $('over-players').replaceChildren(
      ...r.kills.map((k, i) =>
        el('li', {}, [
          Object.assign(el('span', { className: 'dot', text: String(i + 1) }), { style: `background:${PLAYER_COLORS[i] ?? '#888'}` }),
          el('span', { className: 'name', text: `${i + 1}P` }),
          el('span', { className: 'badge', text: `${k} 킬` }),
        ]),
      ),
    );
  }
  const isGuest = online?.lobby.role === 'guest';
  $('btn-over-primary').hidden = isGuest;
  $('over-wait').hidden = !isGuest;
  $('btn-over-primary').textContent = online ? '로비로' : '다시';
  showScreen('screen-over');
  input.refreshTouchVisibility(false);
}

$('btn-over-primary').addEventListener('click', () => {
  const o = online;
  if (o) {
    stopMatch();
    dispatch(o.lobby.backToLobby());
    showScreen('screen-lobby');
  } else {
    startMatch(0, randomU32(), null);
  }
});
$('btn-over-title').addEventListener('click', () => toTitle());

$('btn-menu').addEventListener('click', () => {
  match?.setPaused(true);
  $('btn-mute').textContent = audio.muted ? '소리 켜기' : '소리 끄기';
  showScreen('screen-menu');
  input.refreshTouchVisibility(false);
});
$('btn-resume').addEventListener('click', () => {
  match?.setPaused(false);
  showScreen('game');
  input.refreshTouchVisibility(true);
});
$('btn-mute').addEventListener('click', () => {
  audio.muted = !audio.muted;
  $('btn-mute').textContent = audio.muted ? '소리 켜기' : '소리 끄기';
});
$('btn-quit').addEventListener('click', () => toTitle());

// ---------------------------------------------------------------- platform

let wakeLock: WakeLockSentinel | null = null;
async function requestWakeLock(): Promise<void> {
  try {
    wakeLock = (await navigator.wakeLock?.request('screen')) ?? null;
  } catch {
    wakeLock = null;
  }
}
function releaseWakeLock(): void {
  void wakeLock?.release().catch(() => undefined);
  wakeLock = null;
}

document.addEventListener('visibilitychange', () => {
  if (document.hidden) {
    hiddenSince = Date.now();
    match?.setPaused(true);
    return;
  }
  const away = Date.now() - hiddenSince;
  if (match && online && away > 2000) {
    // networked peers can't resume after missing seconds of frames (tech-debt TD-002)
    toTitle('앱을 벗어나 있어 연결이 끊겼습니다.');
    return;
  }
  if (match && $('screen-menu').hidden) match.setPaused(false);
  if (match) void requestWakeLock();
});

const layout = (): void => {
  renderer.resize();
  const portrait = window.matchMedia('(orientation: portrait)').matches && window.matchMedia('(pointer: coarse)').matches;
  $('rotate').hidden = !(portrait && match);
};
window.addEventListener('resize', layout);
window.matchMedia('(orientation: portrait)').addEventListener('change', layout);

// ---------------------------------------------------------------- boot

async function boot(): Promise<void> {
  $<HTMLInputElement>('nickname').value = load('name') ?? randomNickname();
  if (typeof RTCPeerConnection !== 'function' || typeof WebAssembly !== 'object') {
    $('loading-text').textContent = '이 브라우저는 지원되지 않습니다 (WebRTC/WebAssembly 필요).';
    return;
  }
  if (/KAKAOTALK|Instagram|FBAN|FBAV|Line\//i.test(navigator.userAgent)) {
    $('title-msg').textContent = '인앱 브라우저에서는 연결이 불안정할 수 있어요. Safari/Chrome으로 열어주세요.';
  }
  try {
    await loadWasm();
  } catch (err) {
    console.error(err);
    $('loading-text').textContent = '게임을 불러오지 못했습니다. 새로고침해 주세요.';
    return;
  }
  if (__WASM_IS_MOCK__) console.warn('[hordegrid] running with the TS mock sim (src/wasm-pkg missing)');
  layout();
  const m = location.hash.match(/^#\/r\/(\d{8})$/);
  const code = m?.[1] ? normalizeCode(m[1]) : null;
  if (code) enterRoom(code, 'guest');
  else showScreen('screen-title');
}

void boot();

import { joinRoom, getRelaySockets, selfId } from 'trystero';

// ---------- constants (keep in sync with docs/design-docs/signaling.md) ----------
const APP_ID = 'hordegrid-probe/v1';
const GAME_CH = { negotiated: true, id: 101, ordered: false, maxRetransmits: 0 };
const STUN = [
  'stun:stun.l.google.com:19302',
  'stun:stun1.l.google.com:19302',
  'stun:stun2.l.google.com:19302',
  'stun:stun.cloudflare.com:3478',
];
const TEST_MS = 10_000;
const HZ = 60;
const CODE_RE = /^\d{8}$/;

const $ = (id) => document.getElementById(id);
const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
const now = () => performance.now();
const log = (msg) => {
  const line = document.createElement('div');
  line.textContent = `${new Date().toLocaleTimeString()}  ${msg}`;
  $('log').prepend(line);
};

// ---------- environment ----------
function envInfo() {
  const c = navigator.connection || {};
  const ua = navigator.userAgent;
  const inApp = /KAKAOTALK|Instagram|FBAN|FBAV|Line\//i.test(ua);
  return {
    ua,
    inApp,
    connType: c.type ?? 'unknown',          // Chrome Android only: wifi / cellular ...
    effectiveType: c.effectiveType ?? 'unknown',
    rtc: typeof RTCPeerConnection === 'function',
  };
}

// ---------- NAT probe ----------
function parseCandidate(str) {
  const p = str.replace(/^candidate:/, '').split(' ');
  const typIdx = p.indexOf('typ');
  return { protocol: (p[2] || '').toLowerCase(), address: p[4], port: Number(p[5]), type: p[typIdx + 1] };
}

function waitGathering(pc, ms) {
  return new Promise((resolve) => {
    if (pc.iceGatheringState === 'complete') return resolve();
    const t = setTimeout(resolve, ms);
    pc.addEventListener('icegatheringstatechange', () => {
      if (pc.iceGatheringState === 'complete') { clearTimeout(t); resolve(); }
    });
  });
}

export async function natProbe() {
  const pc = new RTCPeerConnection({ iceServers: STUN.map((urls) => ({ urls })) });
  pc.createDataChannel('probe');
  const cands = [];
  const servers = new Set();
  pc.onicecandidate = (e) => {
    if (!e.candidate?.candidate) return;
    const c = parseCandidate(e.candidate.candidate);
    cands.push(c);
    if (c.type === 'srflx' && e.url) servers.add(e.url);
  };
  const t0 = now();
  await pc.setLocalDescription(await pc.createOffer());
  await waitGathering(pc, 8000);
  pc.close();

  const udp = cands.filter((c) => c.protocol === 'udp');
  const hosts = udp.filter((c) => c.type === 'host');
  const srflx4 = udp.filter((c) => c.type === 'srflx' && c.address?.includes('.'));
  const srflx6 = udp.filter((c) => c.type === 'srflx' && c.address?.includes(':'));
  const mappings = [...new Set(srflx4.map((c) => `${c.address}:${c.port}`))];
  const publicIps = [...new Set(srflx4.map((c) => c.address))];

  let verdict, level;
  if (srflx4.length === 0 && srflx6.length === 0) {
    verdict = hosts.length ? 'UDP 차단/STUN 응답 없음 — 직접 연결 매우 어려움 (TURN 필요 영역)' : 'ICE 후보 없음 — WebRTC 비활성화 가능성';
    level = 'bad';
  } else if (mappings.length <= 1) {
    verdict = '일반 NAT(Endpoint-Independent) — 직접 연결 잘 됨';
    level = 'good';
  } else if (publicIps.length > 1) {
    verdict = `대칭형 NAT + 공인 IP 풀(${publicIps.length}개) — 상대도 제한적이면 실패 가능 높음`;
    level = 'warn';
  } else {
    verdict = `대칭형 NAT(목적지마다 포트 ${mappings.length}개) — 상대도 대칭형이면 실패 가능 높음`;
    level = 'warn';
  }
  return {
    verdict, level,
    gatherMs: Math.round(now() - t0),
    stunResponders: servers.size || null,   // e.url unsupported on some browsers → null
    mappings,
    srflxV6: srflx6.length,
    hostCandidates: hosts.length,
  };
}

// ---------- selected path via getStats ----------
async function selectedPath(pc) {
  const stats = await pc.getStats();
  let pairId;
  stats.forEach((r) => { if (r.type === 'transport' && r.selectedCandidatePairId) pairId = r.selectedCandidatePairId; });
  if (!pairId) stats.forEach((r) => { if (r.type === 'candidate-pair' && r.nominated && r.state === 'succeeded') pairId = r.id; });
  const pair = pairId && stats.get(pairId);
  if (!pair) return null;
  const l = stats.get(pair.localCandidateId) || {};
  const r = stats.get(pair.remoteCandidateId) || {};
  const addr = l.address || l.ip || r.address || r.ip || '';
  const types = [l.candidateType, r.candidateType];
  let kind;
  if (types.includes('relay')) kind = 'TURN 중계';
  else if (types.every((t) => t === 'host')) kind = addr.includes(':') ? 'IPv6 직통(host↔host)' : '같은 네트워크 직통(host↔host)';
  else kind = 'NAT 홀펀칭 성공(srflx/prflx)';
  return {
    kind,
    local: l.candidateType, remote: r.candidateType,
    protocol: l.protocol, ipv6: addr.includes(':') || null,
    rttMs: pair.currentRoundTripTime != null ? Math.round(pair.currentRoundTripTime * 1000) : null,
  };
}

// ---------- echo test over unreliable negotiated channel ----------
function runEchoTest(ch) {
  return new Promise((resolve) => {
    const sentAt = new Map();
    const rtts = [];
    let sent = 0, maxSeq = -1, reordered = 0, dup = 0;
    const got = new Set();
    const buf = new ArrayBuffer(16);
    const dv = new DataView(buf);

    const onMsg = (e) => {
      if (!(e.data instanceof ArrayBuffer) || e.data.byteLength !== 16) return;
      const v = new DataView(e.data);
      const type = v.getUint8(0), seq = v.getUint32(4, true);
      if (type === 1) { v.setUint8(0, 2); if (ch.readyState === 'open') ch.send(e.data); return; }
      if (type !== 2 || !sentAt.has(seq)) return;
      if (got.has(seq)) { dup++; return; }
      got.add(seq);
      if (seq < maxSeq) reordered++; else maxSeq = seq;
      rtts.push(now() - sentAt.get(seq));
    };
    ch.addEventListener('message', onMsg);

    const t0 = now();
    const timer = setInterval(() => {
      if (now() - t0 >= TEST_MS || ch.readyState !== 'open') {
        clearInterval(timer);
        setTimeout(finish, 1500); // stragglers
        return;
      }
      if (ch.bufferedAmount > 64 * 1024) return; // drop, like the game will
      dv.setUint8(0, 1); dv.setUint32(4, sent, true); dv.setFloat64(8, 0, true);
      sentAt.set(sent, now());
      ch.send(buf);
      sent++;
    }, 1000 / HZ);

    function finish() {
      ch.removeEventListener('message', onMsg);
      rtts.sort((a, b) => a - b);
      const q = (p) => (rtts.length ? Math.round(rtts[Math.min(rtts.length - 1, Math.floor(p * rtts.length))]) : null);
      resolve({
        sent, received: got.size,
        rtLossPct: sent ? +(100 * (1 - got.size / sent)).toFixed(2) : null,
        rttP50: q(0.5), rttP95: q(0.95), rttMax: rtts.length ? Math.round(rtts[rtts.length - 1]) : null,
        reordered, dup,
        effectiveHz: +(sent / (TEST_MS / 1000)).toFixed(1),
      });
    }
  });
}

// ---------- room ----------
let room = null;
let myDiag = null;
const peers = new Map(); // id -> {row, diag, result}

function genCode() {
  const a = new Uint32Array(1);
  crypto.getRandomValues(a);
  return String(a[0] % 100_000_000).padStart(8, '0');
}
const fmtCode = (c) => `${c.slice(0, 4)} ${c.slice(4)}`;

function relayCount() {
  try { return Object.values(getRelaySockets()).filter((s) => s.readyState === 1).length; } catch { return 0; }
}

function renderPeer(id) {
  const p = peers.get(id);
  if (!p) return;
  const r = p.result || {};
  p.row.innerHTML = `
    <div class="peer-h"><b>상대 ${esc(id.slice(0, 6))}</b> <span class="pill ${p.state === 'done' ? 'good' : p.state === 'fail' ? 'bad' : ''}">${esc(p.statusText)}</span></div>
    <dl>
      <dt>상대 NAT</dt><dd>${esc(p.diag?.nat?.verdict ?? '수신 대기')}</dd>
      <dt>상대 망</dt><dd>${esc(p.diag ? `${p.diag.env.connType}/${p.diag.env.effectiveType}` : '-')}</dd>
      <dt>경로</dt><dd>${esc(r.path?.kind ?? '-')}</dd>
      <dt>연결까지</dt><dd>나 ${r.peerFoundMs != null ? `${(r.peerFoundMs / 1000).toFixed(1)}초` : '-'} / 상대 ${r.peerSideFoundMs != null ? `${(r.peerSideFoundMs / 1000).toFixed(1)}초` : '-'} · 게임채널 open ${r.channelOpenMs != null ? `+${r.channelOpenMs}ms` : '-'}</dd>
      <dt>RTT p50/p95</dt><dd>${r.echo ? `${r.echo.rttP50} / ${r.echo.rttP95} ms` : '-'}</dd>
      <dt>왕복 손실</dt><dd>${r.echo ? `${r.echo.rtLossPct}% (${r.echo.received}/${r.echo.sent}) · 역순 ${r.echo.reordered}` : '-'}</dd>
    </dl>`;
}

function setStatus(id, state, text) {
  const p = peers.get(id);
  if (!p) return;
  p.state = state; p.statusText = text;
  renderPeer(id);
}

async function startRoom(code) {
  if (room) await room.leave();
  peers.clear();
  $('peers').innerHTML = '';
  $('roomBox').hidden = false;
  $('codeShow').textContent = fmtCode(code);
  const link = `${location.origin}${location.pathname}${location.search}#/p/${code}`;
  $('linkShow').value = link;
  history.replaceState(null, '', `#/p/${code}`);
  const joinedAt = now();
  log(`방 ${fmtCode(code)} 참가 (selfId ${selfId.slice(0, 6)})`);

  // ?relay=wss://... overrides relays (self-test / blocked networks with a known relay)
  const relayParam = new URLSearchParams(location.search).getAll('relay').filter((u) => /^wss?:\/\/[\w.-]+(:\d+)?(\/[\w./-]*)?$/.test(u));
  const cfg = { appId: APP_ID, password: code };
  if (relayParam.length) cfg.relayConfig = { urls: relayParam, redundancy: relayParam.length };
  room = joinRoom(cfg, code, {
    onJoinError: (d) => log(`참가 오류: ${d.error}`),
  });

  const info = room.makeAction('info');
  room._info = info;
  // each side reports its OWN join→peer time; the room creator's number includes the friend's link-open delay
  const timing = room.makeAction('timing');
  timing.onMessage = (data, { peerId }) => {
    const p = peers.get(peerId);
    if (p && typeof data?.foundMs === 'number') { p.result.peerSideFoundMs = data.foundMs; renderPeer(peerId); updateReport(); }
  };
  info.onMessage = (data, { peerId }) => {
    const p = peers.get(peerId);
    if (p) { p.diag = data; renderPeer(peerId); }
  };

  const relayTimer = setInterval(() => {
    const n = relayCount();
    $('relays').textContent = `Nostr 릴레이 연결 ${n}개`;
    $('relays').className = `pill ${n > 0 ? 'good' : 'bad'}`;
    const waited = (now() - joinedAt) / 1000;
    if (peers.size === 0) {
      $('waitHint').textContent = n === 0 && waited > 10
        ? '릴레이에 하나도 못 붙음 → 회사/학교망 차단 가능성. 다른 네트워크로 시도'
        : waited > 30
          ? '30초 넘게 상대가 안 보임: 상대가 아직 안 들어왔거나, 둘 다 대칭형 NAT라 직접 연결 실패일 가능성. 둘 중 한 명 Wi-Fi 전환 후 재시도'
          : `상대 기다리는 중… ${Math.floor(waited)}초`;
    } else $('waitHint').textContent = '';
  }, 1000);
  room._relayTimer = relayTimer;

  room.onPeerJoin = async (peerId) => {
    const row = document.createElement('div');
    row.className = 'peer';
    $('peers').append(row);
    peers.set(peerId, { row, state: 'run', statusText: '연결됨, 게임 채널 여는 중', result: { peerFoundMs: Math.round(now() - joinedAt) } });
    renderPeer(peerId);
    log(`피어 ${peerId.slice(0, 6)} 연결 (${((now() - joinedAt) / 1000).toFixed(1)}초)`);
    if (myDiag) info.send(myDiag, { target: peerId });
    timing.send({ foundMs: Math.round(now() - joinedAt) }, { target: peerId });

    const pc = room.getPeers()[peerId];
    if (!pc) { setStatus(peerId, 'fail', 'RTCPeerConnection 없음'); return; }
    let ch;
    try {
      ch = pc.createDataChannel('ggrs', GAME_CH);
      ch.binaryType = 'arraybuffer';
    } catch (err) {
      setStatus(peerId, 'fail', `채널 생성 실패: ${err.message}`);
      log(`채널 생성 실패 ${err}`);
      return;
    }
    const t = now();
    const opened = await new Promise((res) => {
      if (ch.readyState === 'open') return res(true);
      const to = setTimeout(() => res(false), 8000);
      ch.onopen = () => { clearTimeout(to); res(true); };
    });
    const p = peers.get(peerId);
    if (!p) return;
    if (!opened) { setStatus(peerId, 'fail', '비신뢰 채널(id 101) 8초 내 open 실패'); return; }
    p.result.channelOpenMs = Math.round(now() - t);
    setStatus(peerId, 'run', '10초 측정 중 (60pps)');
    const echo = await runEchoTest(ch);
    const path = await selectedPath(pc);
    if (!peers.has(peerId)) return;
    Object.assign(p.result, { echo, path });
    setStatus(peerId, 'done', '측정 완료');
    updateReport();
  };

  room.onPeerLeave = (peerId) => {
    const p = peers.get(peerId);
    if (p && p.state !== 'done') setStatus(peerId, 'fail', '측정 중 이탈');
    log(`피어 ${peerId.slice(0, 6)} 이탈`);
  };
}

function updateReport() {
  const report = {
    tool: APP_ID,
    at: new Date().toISOString(),
    me: myDiag,
    peers: [...peers.entries()].map(([id, p]) => ({ id: id.slice(0, 6), nat: p.diag?.nat?.verdict ?? null, env: p.diag?.env ? { connType: p.diag.env.connType, effectiveType: p.diag.env.effectiveType } : null, ...p.result })),
  };
  $('report').value = JSON.stringify(report, null, 2);
  const rows = report.peers.filter((p) => p.echo).map((p) =>
    `| ${report.at.slice(0, 10)} | 나: ${myDiag.env.connType} (${myDiag.nat.level}) | 상대: ${p.env?.connType ?? '?'} | ${p.path?.kind ?? '-'} | ${(Math.min(p.peerFoundMs, p.peerSideFoundMs ?? Infinity) / 1000).toFixed(1)}s | ${p.echo.rttP50}/${p.echo.rttP95} | ${p.echo.rtLossPct}% |`);
  $('mdRow').value = rows.join('\n');
}

// ---------- boot ----------
async function boot() {
  const env = envInfo();
  $('env').textContent = `${env.connType !== 'unknown' ? env.connType : '망 종류 알 수 없음(iOS/데스크톱은 미제공)'} · ${env.effectiveType}`;
  if (env.inApp) $('inapp').hidden = false;
  if (!env.rtc) { $('natVerdict').textContent = '이 브라우저는 WebRTC를 지원하지 않음'; return; }

  // join first so a friend waiting in the room is not delayed by the NAT probe
  const m = location.hash.match(/^#\/p\/(\d{8})$/);
  if (m) startRoom(m[1]);

  $('natVerdict').textContent = 'NAT 검사 중… (최대 8초)';
  const nat = await natProbe();
  myDiag = { env: { connType: env.connType, effectiveType: env.effectiveType, inApp: env.inApp, ua: env.ua }, nat };
  $('natVerdict').textContent = nat.verdict;
  $('natVerdict').className = `verdict ${nat.level}`;
  $('natDetail').textContent = `외부 매핑 ${nat.mappings.length}개 · STUN 응답 ${nat.stunResponders ?? '확인불가'} · 수집 ${nat.gatherMs}ms`;
  updateReport();
  log(`NAT: ${nat.verdict}`);
  if (room) room._info?.send(myDiag); // peers that joined before the probe finished
}

$('create').onclick = () => startRoom(genCode());
$('join').onclick = () => {
  const code = $('codeIn').value.replace(/\D/g, '');
  if (!CODE_RE.test(code)) { $('codeIn').setCustomValidity('숫자 8자리'); $('codeIn').reportValidity(); return; }
  startRoom(code);
};
$('codeIn').oninput = () => $('codeIn').setCustomValidity('');
const copy = async (el) => {
  try { await navigator.clipboard.writeText(el.value); log('복사됨'); } catch { el.select(); log('복사 권한 없음 — 선택됨, 직접 복사'); }
};
$('copyLink').onclick = () => copy($('linkShow'));
$('copyReport').onclick = () => copy($('report'));
$('copyRow').onclick = () => copy($('mdRow'));
$('retest').onclick = () => { const m = location.hash.match(/(\d{8})/); if (m) startRoom(m[1]); };

boot();

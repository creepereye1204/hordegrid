// Host-authoritative lobby state machine. Pure: no I/O, no timers — the caller feeds time and
// delivers the returned outgoing messages. Unit-tested in test/lobby.test.ts.
import { seedFromIds } from '../core/ids';
import { MAX_PLAYERS, PROTO_VER, type DenyReason, type LobbyMsg, type MemberView } from './protocol';

export const KNOCK_TIMEOUT_MS = 20_000;
export const REJECT_COOLDOWN_MS = 10 * 60_000;

export interface Outgoing {
  to: string | 'members';
  msg: LobbyMsg;
}

export type LobbyEvent =
  | { kind: 'knock'; id: string; name: string }
  | { kind: 'knock-cancelled'; id: string }
  | { kind: 'admitted' }
  | { kind: 'denied'; reason: DenyReason }
  | { kind: 'roster'; members: MemberView[] }
  | { kind: 'start'; players: string[]; seed: number }
  | { kind: 'back-to-lobby' }
  | { kind: 'host-left' };

interface Member {
  name: string;
  ready: boolean;
  links: Set<string>;
}

export class Lobby {
  readonly role: 'host' | 'guest';
  private readonly members = new Map<string, Member>();
  private readonly pending = new Map<string, { name: string; since: number }>();
  private readonly rejected = new Map<string, number>();
  private hostId: string | null = null;
  private admitted = false;
  private started = false;
  private myReady = false;
  private readonly myLinks = new Set<string>();
  private roster: MemberView[] = [];
  readonly events: LobbyEvent[] = [];

  constructor(
    private readonly selfId: string,
    private readonly name: string,
    role: 'host' | 'guest',
    private readonly build: string,
  ) {
    this.role = role;
    if (role === 'host') {
      this.hostId = selfId;
      this.admitted = true;
      this.members.set(selfId, { name, ready: true, links: this.myLinks });
      this.roster = this.buildRoster();
    }
  }

  get isAdmitted(): boolean {
    return this.admitted;
  }

  get currentRoster(): readonly MemberView[] {
    return this.roster;
  }

  get pendingKnocks(): ReadonlyArray<{ id: string; name: string }> {
    return [...this.pending].map(([id, p]) => ({ id, name: p.name }));
  }

  /** A transport-level peer appeared. */
  peerJoined(id: string): Outgoing[] {
    return [{ to: id, msg: { t: 'hello', role: this.role, name: this.name, proto: PROTO_VER, build: this.build } }];
  }

  peerLeft(id: string): Outgoing[] {
    this.myLinks.delete(id);
    if (this.role === 'guest') {
      if (id === this.hostId) {
        this.events.push({ kind: 'host-left' });
      }
      return this.statusMsg();
    }
    if (this.pending.delete(id)) this.events.push({ kind: 'knock-cancelled', id });
    if (this.members.delete(id)) return this.broadcastRoster();
    return this.broadcastRoster();
  }

  /** Local game channel to `id` opened/closed. */
  linkChanged(id: string, open: boolean): Outgoing[] {
    if (open) this.myLinks.add(id);
    else this.myLinks.delete(id);
    return this.role === 'host' ? this.broadcastRoster() : this.statusMsg();
  }

  setReady(ready: boolean): Outgoing[] {
    this.myReady = ready;
    return this.role === 'guest' ? this.statusMsg() : [];
  }

  receive(from: string, msg: LobbyMsg, now: number): Outgoing[] {
    return this.role === 'host' ? this.hostReceive(from, msg, now) : this.guestReceive(from, msg);
  }

  // ---------------- host ----------------

  private hostReceive(from: string, msg: LobbyMsg, now: number): Outgoing[] {
    switch (msg.t) {
      case 'knock': {
        const deny = (reason: DenyReason): Outgoing[] => [{ to: from, msg: { t: 'deny', reason } }];
        if (this.members.has(from)) return [{ to: from, msg: { t: 'admit' } }, ...this.broadcastRoster()];
        if (this.started) return deny('started');
        if (msg.proto !== PROTO_VER || msg.build !== this.build) return deny('version');
        const rejectedAt = this.rejected.get(from);
        if (rejectedAt !== undefined && now - rejectedAt < REJECT_COOLDOWN_MS) return deny('rejected');
        if (this.members.size >= MAX_PLAYERS) return deny('full');
        if (!this.pending.has(from)) {
          this.pending.set(from, { name: msg.name, since: now });
          this.events.push({ kind: 'knock', id: from, name: msg.name });
        }
        return [];
      }
      case 'status': {
        const m = this.members.get(from);
        if (!m) return [];
        m.ready = msg.ready;
        m.links = new Set(msg.links);
        return this.broadcastRoster();
      }
      default:
        return [];
    }
  }

  approve(id: string): Outgoing[] {
    const p = this.pending.get(id);
    if (!p || this.role !== 'host') return [];
    this.pending.delete(id);
    if (this.members.size >= MAX_PLAYERS || this.started) return [{ to: id, msg: { t: 'deny', reason: 'full' } }];
    this.members.set(id, { name: p.name, ready: false, links: new Set() });
    return [{ to: id, msg: { t: 'admit' } }, ...this.broadcastRoster()];
  }

  reject(id: string, now: number, reason: DenyReason = 'rejected'): Outgoing[] {
    if (this.role !== 'host') return [];
    const wasMember = this.members.delete(id);
    if (!this.pending.delete(id) && !wasMember) return [];
    this.rejected.set(id, now);
    return [{ to: id, msg: { t: 'deny', reason } }, ...(wasMember ? this.broadcastRoster() : [])];
  }

  /** Expire unanswered knocks. */
  tick(now: number): Outgoing[] {
    const out: Outgoing[] = [];
    for (const [id, p] of this.pending) {
      if (now - p.since >= KNOCK_TIMEOUT_MS) {
        this.pending.delete(id);
        this.events.push({ kind: 'knock-cancelled', id });
        out.push({ to: id, msg: { t: 'deny', reason: 'timeout' } });
      }
    }
    return out;
  }

  /** Why the host can't start yet, or null if it can. */
  startBlocker(): string | null {
    if (this.role !== 'host') return '방장만 시작할 수 있음';
    const ids = [...this.members.keys()];
    if (ids.length < 2) return '친구를 기다리는 중';
    for (const [id, m] of this.members) {
      if (id !== this.selfId && !m.ready) return `${m.name} 준비 대기`;
    }
    // every pair needs an open game channel, or GGRS can never synchronize
    for (const [id, m] of this.members) {
      if (ids.some((other) => other !== id && !m.links.has(other))) return '연결 확인 중…';
    }
    return null;
  }

  start(salt: number): Outgoing[] {
    if (this.startBlocker() !== null) return [];
    const players = [...this.members.keys()].sort();
    const seed = seedFromIds(players, salt);
    this.started = true;
    this.events.push({ kind: 'start', players, seed });
    return [{ to: 'members', msg: { t: 'start', players, seed } }];
  }

  /** After game over: everyone back to the lobby, readiness reset. */
  backToLobby(): Outgoing[] {
    if (this.role !== 'host') return [];
    this.started = false;
    for (const [id, m] of this.members) m.ready = id === this.selfId;
    return [{ to: 'members', msg: { t: 'lobby' } }, ...this.broadcastRoster()];
  }

  private buildRoster(): MemberView[] {
    return [...this.members].map(([id, m]) => ({ id, name: m.name, ready: m.ready, host: id === this.selfId }));
  }

  private broadcastRoster(): Outgoing[] {
    this.roster = this.buildRoster();
    this.events.push({ kind: 'roster', members: this.roster });
    return [{ to: 'members', msg: { t: 'roster', members: this.roster, started: this.started } }];
  }

  memberIds(): string[] {
    return [...this.members.keys()].filter((id) => id !== this.selfId);
  }

  // ---------------- guest ----------------

  private guestReceive(from: string, msg: LobbyMsg): Outgoing[] {
    switch (msg.t) {
      case 'hello':
        if (msg.role !== 'host' || this.hostId !== null) return [];
        if (msg.proto !== PROTO_VER || msg.build !== this.build) {
          this.events.push({ kind: 'denied', reason: 'version' });
          return [];
        }
        this.hostId = from;
        return [{ to: from, msg: { t: 'knock', name: this.name, proto: PROTO_VER, build: this.build } }];
      case 'admit':
        if (from !== this.hostId || this.admitted) return [];
        this.admitted = true;
        this.events.push({ kind: 'admitted' });
        return this.statusMsg();
      case 'deny':
        if (from !== this.hostId) return [];
        this.admitted = false;
        this.events.push({ kind: 'denied', reason: msg.reason });
        return [];
      case 'roster':
        if (from !== this.hostId) return [];
        this.roster = msg.members;
        this.events.push({ kind: 'roster', members: msg.members });
        return [];
      case 'start':
        if (from !== this.hostId || !this.admitted || !msg.players.includes(this.selfId)) return [];
        this.events.push({ kind: 'start', players: msg.players, seed: msg.seed });
        return [];
      case 'lobby':
        if (from !== this.hostId) return [];
        this.myReady = false;
        this.events.push({ kind: 'back-to-lobby' });
        return this.statusMsg();
      default:
        return [];
    }
  }

  private statusMsg(): Outgoing[] {
    if (!this.admitted || !this.hostId || this.role !== 'guest') return [];
    return [{ to: this.hostId, msg: { t: 'status', ready: this.myReady, links: [...this.myLinks] } }];
  }

  /** Peers a guest should know about (host + roster). */
  rosterIds(): string[] {
    return this.roster.map((m) => m.id).filter((id) => id !== this.selfId);
  }
}

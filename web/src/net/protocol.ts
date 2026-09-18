// Lobby wire protocol (docs/generated/net-protocol.md). Every inbound message is untrusted.
import { sanitizeName } from '../core/ids';

export const PROTO_VER = 1;
export const APP_ID = `hordegrid/p${PROTO_VER}`;
export const MAX_PLAYERS = 4;

export type DenyReason = 'rejected' | 'timeout' | 'full' | 'started' | 'version' | 'kicked';

export interface MemberView {
  id: string;
  name: string;
  ready: boolean;
  host: boolean;
}

export type LobbyMsg =
  | { t: 'hello'; role: 'host' | 'guest'; name: string; proto: number; build: string }
  | { t: 'knock'; name: string; proto: number; build: string }
  | { t: 'admit' }
  | { t: 'deny'; reason: DenyReason }
  | { t: 'roster'; members: MemberView[]; started: boolean }
  | { t: 'status'; ready: boolean; links: string[] }
  | { t: 'start'; players: string[]; seed: number; mode: number }
  | { t: 'lobby' };

const REASONS: readonly DenyReason[] = ['rejected', 'timeout', 'full', 'started', 'version', 'kicked'];
const isObj = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null && !Array.isArray(v);
const isStrArr = (v: unknown, max: number): v is string[] =>
  Array.isArray(v) && v.length <= max && v.every((s) => typeof s === 'string' && s.length <= 64);

/** Validate and normalize; returns null for anything malformed. */
export function parseLobbyMsg(raw: unknown): LobbyMsg | null {
  if (!isObj(raw) || typeof raw['t'] !== 'string') return null;
  switch (raw['t']) {
    case 'hello':
      if (raw['role'] !== 'host' && raw['role'] !== 'guest') return null;
      if (typeof raw['proto'] !== 'number' || typeof raw['build'] !== 'string') return null;
      return { t: 'hello', role: raw['role'], name: sanitizeName(raw['name']), proto: raw['proto'], build: raw['build'].slice(0, 40) };
    case 'knock':
      if (typeof raw['proto'] !== 'number' || typeof raw['build'] !== 'string') return null;
      return { t: 'knock', name: sanitizeName(raw['name']), proto: raw['proto'], build: raw['build'].slice(0, 40) };
    case 'admit':
      return { t: 'admit' };
    case 'deny':
      return REASONS.includes(raw['reason'] as DenyReason) ? { t: 'deny', reason: raw['reason'] as DenyReason } : null;
    case 'roster': {
      const m = raw['members'];
      if (!Array.isArray(m) || m.length > MAX_PLAYERS) return null;
      const members: MemberView[] = [];
      for (const x of m) {
        if (!isObj(x) || typeof x['id'] !== 'string') return null;
        members.push({ id: x['id'].slice(0, 64), name: sanitizeName(x['name']), ready: x['ready'] === true, host: x['host'] === true });
      }
      return { t: 'roster', members, started: raw['started'] === true };
    }
    case 'status':
      if (!isStrArr(raw['links'], MAX_PLAYERS)) return null;
      return { t: 'status', ready: raw['ready'] === true, links: raw['links'] };
    case 'start':
      if (!isStrArr(raw['players'], MAX_PLAYERS) || typeof raw['seed'] !== 'number') return null;
      if (raw['players'].length < 2 || !Number.isInteger(raw['seed'])) return null;
      if (raw['mode'] !== 0 && raw['mode'] !== 1) return null;
      return { t: 'start', players: raw['players'], seed: raw['seed'] >>> 0, mode: raw['mode'] };
    case 'lobby':
      return { t: 'lobby' };
    default:
      return null;
  }
}

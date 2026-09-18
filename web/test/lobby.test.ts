import { describe, expect, it } from 'vitest';
import { KNOCK_TIMEOUT_MS, Lobby, type Outgoing } from '../src/net/lobby';
import { parseLobbyMsg, PROTO_VER, type LobbyMsg } from '../src/net/protocol';

const BUILD = 'test';

/** Tiny in-memory network of lobbies: delivers messages until quiet. */
class Net {
  readonly nodes = new Map<string, Lobby>();
  now = 0;
  add(l: Lobby, id: string): void {
    this.nodes.set(id, l);
  }
  deliver(from: string, out: Outgoing[]): void {
    const queue: Array<[string, string, LobbyMsg]> = [];
    const push = (src: string, o: Outgoing[]): void => {
      for (const { to, msg } of o) {
        const targets = to === 'members' ? this.nodes.get(src)!.memberIds() : [to];
        for (const t of targets) queue.push([src, t, JSON.parse(JSON.stringify(msg)) as LobbyMsg]);
      }
    };
    push(from, out);
    while (queue.length) {
      const [src, dst, msg] = queue.shift()!;
      const node = this.nodes.get(dst);
      if (!node) continue;
      const parsed = parseLobbyMsg(msg);
      expect(parsed).not.toBeNull();
      push(dst, node.receive(src, parsed!, this.now));
    }
  }
  connect(a: string, b: string): void {
    this.deliver(a, this.nodes.get(a)!.peerJoined(b));
    this.deliver(b, this.nodes.get(b)!.peerJoined(a));
  }
  link(a: string, b: string): void {
    this.deliver(a, this.nodes.get(a)!.linkChanged(b, true));
    this.deliver(b, this.nodes.get(b)!.linkChanged(a, true));
  }
}

function setup(): { net: Net; host: Lobby; g1: Lobby } {
  const net = new Net();
  const host = new Lobby('h', 'Host', 'host', BUILD);
  const g1 = new Lobby('g1', 'Guest', 'guest', BUILD);
  net.add(host, 'h');
  net.add(g1, 'g1');
  net.connect('h', 'g1');
  return { net, host, g1 };
}

describe('lobby', () => {
  it('when guest knocks then host must approve before guest is a member (AC9)', () => {
    const { net, host, g1 } = setup();
    expect(host.pendingKnocks).toEqual([{ id: 'g1', name: 'Guest' }]);
    expect(host.currentRoster.map((m) => m.id)).toEqual(['h']);
    expect(g1.isAdmitted).toBe(false);
    expect(host.startBlocker()).not.toBeNull();

    net.deliver('h', host.approve('g1'));
    expect(g1.isAdmitted).toBe(true);
    expect(g1.currentRoster.map((m) => m.id).sort()).toEqual(['g1', 'h']);
  });

  it('when all ready and linked then host can start and both get the same seed', () => {
    const { net, host, g1 } = setup();
    net.deliver('h', host.approve('g1'));
    expect(host.startBlocker()).toMatch(/준비/);
    net.deliver('g1', g1.setReady(true));
    expect(host.startBlocker()).toMatch(/연결/);
    net.link('h', 'g1');
    expect(host.startBlocker()).toBeNull();

    host.events.length = 0;
    g1.events.length = 0;
    net.deliver('h', host.start(42));
    const hs = host.events.find((e) => e.kind === 'start');
    const gs = g1.events.find((e) => e.kind === 'start');
    expect(hs).toEqual(gs);
    expect(hs && 'players' in hs ? hs.players : []).toEqual(['g1', 'h']);
  });

  it('when knock unanswered then it times out with deny (AC10)', () => {
    const { net, host, g1 } = setup();
    net.now = KNOCK_TIMEOUT_MS + 1;
    net.deliver('h', host.tick(net.now));
    expect(host.pendingKnocks).toEqual([]);
    expect(g1.events.some((e) => e.kind === 'denied' && e.reason === 'timeout')).toBe(true);
  });

  it('when game started then new knocks are denied', () => {
    const { net, host, g1 } = setup();
    net.deliver('h', host.approve('g1'));
    net.deliver('g1', g1.setReady(true));
    net.link('h', 'g1');
    net.deliver('h', host.start(1));
    const late = new Lobby('late', 'Late', 'guest', BUILD);
    net.add(late, 'late');
    net.connect('h', 'late');
    expect(late.events.some((e) => e.kind === 'denied' && e.reason === 'started')).toBe(true);
  });

  it('when builds differ then guest is denied for version (AC4)', () => {
    const net = new Net();
    const host = new Lobby('h', 'Host', 'host', 'a');
    const g = new Lobby('g', 'G', 'guest', 'b');
    net.add(host, 'h');
    net.add(g, 'g');
    net.connect('h', 'g');
    expect(g.events.some((e) => e.kind === 'denied' && e.reason === 'version')).toBe(true);
    expect(host.pendingKnocks).toEqual([]);
  });

  it('when a non-host sends roster then guest ignores it', () => {
    const { host, g1 } = setup();
    void host;
    g1.receive('mallory', { t: 'roster', members: [{ id: 'x', name: 'x', ready: true, host: true }], started: false }, 0);
    expect(g1.currentRoster).toEqual([]);
  });

  it('when fifth player knocks then denied full', () => {
    const net = new Net();
    const host = new Lobby('h', 'H', 'host', BUILD);
    net.add(host, 'h');
    for (const id of ['a', 'b', 'c']) {
      const g = new Lobby(id, id, 'guest', BUILD);
      net.add(g, id);
      net.connect('h', id);
      net.deliver('h', host.approve(id));
    }
    const fifth = new Lobby('e', 'e', 'guest', BUILD);
    net.add(fifth, 'e');
    net.connect('h', 'e');
    expect(fifth.events.some((e) => e.kind === 'denied' && e.reason === 'full')).toBe(true);
  });
});

describe('protocol parsing', () => {
  it('rejects malformed and hostile messages', () => {
    expect(parseLobbyMsg(null)).toBeNull();
    expect(parseLobbyMsg({ t: 'start', players: ['a'], seed: 1, mode: 0 })).toBeNull();
    expect(parseLobbyMsg({ t: 'start', players: ['a', 'b'], seed: 1.5, mode: 0 })).toBeNull();
    expect(parseLobbyMsg({ t: 'start', players: ['a', 'b'], seed: 1, mode: 2 })).toBeNull();
    expect(parseLobbyMsg({ t: 'deny', reason: 'lol' })).toBeNull();
    expect(parseLobbyMsg({ t: 'roster', members: new Array(5).fill({ id: 'x' }), started: false })).toBeNull();
    const hello = parseLobbyMsg({ t: 'hello', role: 'host', name: '<img src=x>'.repeat(5), proto: PROTO_VER, build: 'x' });
    expect(hello && 'name' in hello ? [...hello.name].length : 0).toBe(16);
  });
});

// trystero room + per-peer unreliable game channel (docs/design-docs/signaling.md).
// API verified against trystero@0.25.4 type definitions (docs/references/trystero-llms.txt).
import { getRelaySockets, joinRoom, selfId, type Room } from 'trystero';
import { APP_ID, parseLobbyMsg, type LobbyMsg } from './protocol';

const GAME_CHANNEL: RTCDataChannelInit = { negotiated: true, id: 101, ordered: false, maxRetransmits: 0 };
const MAX_BUFFERED = 64 * 1024;

export interface RoomHandlers {
  peerJoined(id: string): void;
  peerLeft(id: string): void;
  lobby(from: string, msg: LobbyMsg): void;
  link(id: string, open: boolean): void;
  game(from: string, bytes: Uint8Array): void;
}

export class NetRoom {
  readonly selfId = selfId;
  private readonly room: Room;
  private readonly sendLobbyRaw: (data: string, target: string | string[] | null) => Promise<void>;
  private readonly channels = new Map<string, RTCDataChannel>();

  constructor(code: string, private readonly h: RoomHandlers, relayUrls?: string[]) {
    const config: Parameters<typeof joinRoom>[0] = { appId: APP_ID, password: code };
    if (relayUrls?.length) config.relayConfig = { urls: relayUrls, redundancy: relayUrls.length };
    this.room = joinRoom(config, code, { onJoinError: (d) => console.warn('[room] join error', d.error) });

    const action = this.room.makeAction<string>('lobby');
    this.sendLobbyRaw = (data, target) => action.send(data, { target });
    action.onMessage = (data, { peerId }) => {
      if (typeof data !== 'string' || data.length > 8192) return;
      let parsed: unknown;
      try {
        parsed = JSON.parse(data);
      } catch {
        return;
      }
      const msg = parseLobbyMsg(parsed);
      if (msg) this.h.lobby(peerId, msg);
    };

    this.room.onPeerJoin = (id) => {
      this.openGameChannel(id);
      this.h.peerJoined(id);
    };
    this.room.onPeerLeave = (id) => {
      this.channels.get(id)?.close();
      this.channels.delete(id);
      this.h.peerLeft(id);
    };
  }

  private openGameChannel(id: string): void {
    const pc = this.room.getPeers()[id];
    if (!pc) return;
    let ch: RTCDataChannel;
    try {
      ch = pc.createDataChannel('ggrs', GAME_CHANNEL);
    } catch (err) {
      console.error('[room] game channel failed', err);
      return;
    }
    ch.binaryType = 'arraybuffer';
    ch.onopen = () => this.h.link(id, true);
    ch.onclose = () => this.h.link(id, false);
    ch.onmessage = (e: MessageEvent) => {
      if (e.data instanceof ArrayBuffer && e.data.byteLength <= 1200) this.h.game(id, new Uint8Array(e.data));
    };
    this.channels.set(id, ch);
    if (ch.readyState === 'open') this.h.link(id, true);
  }

  sendLobby(to: string | string[], msg: LobbyMsg): void {
    const targets = Array.isArray(to) ? to : [to];
    if (targets.length === 0) return;
    void this.sendLobbyRaw(JSON.stringify(msg), targets).catch((err: unknown) => console.warn('[room] send failed', err));
  }

  sendGame(to: string, bytes: Uint8Array): void {
    const ch = this.channels.get(to);
    // stale inputs are worthless: drop rather than queue behind a congested channel
    if (!ch || ch.readyState !== 'open' || ch.bufferedAmount > MAX_BUFFERED) return;
    ch.send(bytes as Uint8Array<ArrayBuffer>);
  }

  async ping(id: string): Promise<number | null> {
    try {
      return await this.room.ping(id);
    } catch {
      return null;
    }
  }

  relayCount(): number {
    try {
      return Object.values(getRelaySockets() as Record<string, WebSocket>).filter((s) => s.readyState === 1).length;
    } catch {
      return 0;
    }
  }

  async leave(): Promise<void> {
    for (const ch of this.channels.values()) ch.close();
    this.channels.clear();
    await this.room.leave();
  }
}

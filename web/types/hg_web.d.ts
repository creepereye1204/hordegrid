// Mirror of the wasm-bindgen output for crates/web (used only when src/wasm-pkg is absent).
// CI type-checks against the generated file, so drift here fails the build.
export interface InitOutput {
  readonly memory: WebAssembly.Memory;
}
export default function init(module_or_path?: unknown): Promise<InitOutput>;
export class Game {
  constructor(num_players: number, local_handle: number, seed: number, map_id: number);
  free(): void;
  push_packet(slot: number, bytes: Uint8Array): void;
  tick(input_bits: number): number;
  take_outbox(): Uint8Array;
  render(): number;
  render_ptr(): number;
  take_events(): Int32Array;
  take_net_events(): Int32Array;
  ping_ms(slot: number): number;
  map_width(): number;
  map_height(): number;
  map_tiles(): Uint8Array;
  checksum_hex(): string;
  state_dump(): Uint8Array;
}

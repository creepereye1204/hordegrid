import init, { Game } from '@wasm';

export type { Game };

let memory: WebAssembly.Memory | null = null;

/** Load the wasm module once. */
export async function loadWasm(): Promise<void> {
  if (memory) return;
  const out = await init();
  memory = out.memory;
}

export function newGame(numPlayers: number, localHandle: number, seed: number, mapId = 0, mode = 0): Game {
  if (!memory) throw new Error('wasm not loaded');
  return new Game(numPlayers, localHandle, seed >>> 0, mapId, mode);
}

/**
 * Fresh Int32Array over the render view. Rebuilt every frame: wasm memory growth detaches
 * previous ArrayBuffers (docs/design-docs/wasm-boundary.md#메모리).
 */
export function renderView(game: Game): Int32Array {
  const len = game.render();
  const ptr = game.render_ptr();
  if (!memory) throw new Error('wasm not loaded');
  return new Int32Array(memory.buffer, ptr, len);
}

export { iterOutbox } from './wasm-framing';

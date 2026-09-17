import { defineConfig } from 'vitest/config';
import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const pkg = fileURLToPath(new URL('./src/wasm-pkg/hg_web.js', import.meta.url));
const mock = fileURLToPath(new URL('./src/core/mock-wasm.ts', import.meta.url));
const hasWasm = existsSync(pkg);

export default defineConfig(({ command, isPreview }) => ({
  // GitHub Pages project site lives under /<repo>/ (docs/FRONTEND.md)
  base: command === 'build' || isPreview ? (process.env.VITE_BASE ?? '/hordegrid/') : '/',
  resolve: {
    // Real wasm-bindgen output when built; otherwise a TS mock so the UI runs without a Rust toolchain.
    alias: { '@wasm': hasWasm ? pkg : mock },
  },
  define: {
    __BUILD_HASH__: JSON.stringify(process.env.VITE_BUILD_HASH ?? 'dev'),
    __WASM_IS_MOCK__: JSON.stringify(!hasWasm),
  },
  build: { target: 'es2022', sourcemap: true },
  test: { environment: 'node', include: ['test/**/*.test.ts'] },
}));

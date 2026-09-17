# Command contract — see AGENTS.md. Names change here first.
set shell := ["bash", "-euo", "pipefail", "-c"]

wasm_out := "web/src/wasm-pkg"

check:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
    cd web && npm ci --silent && npx tsc --noEmit && npx vitest run && node scripts/lint-arch.mjs

sim-test:
    cargo test -p hg-sim --release

netsim:
    cargo test -p hg-net --release --test netsim -- --include-ignored

wasm profile="release":
    cargo build -p hg-web --target wasm32-unknown-unknown {{ if profile == "release" { "--release" } else { "" } }}
    wasm-bindgen --target web --out-dir {{wasm_out}} target/wasm32-unknown-unknown/{{profile}}/hg_web.wasm
    if command -v wasm-opt >/dev/null && [ "{{profile}}" = "release" ]; then wasm-opt -O3 --enable-bulk-memory --enable-nontrapping-float-to-int -o {{wasm_out}}/hg_web_bg.wasm {{wasm_out}}/hg_web_bg.wasm; fi

dev: (wasm "debug")
    cd web && npx vite

build: wasm
    cd web && npm ci --silent && npm run build
    gzip -9 -c {{wasm_out}}/hg_web_bg.wasm | wc -c | xargs -I{} echo "wasm gzip bytes: {}"

diag players="2":
    cargo run -p hg-sim --release --example diag -- {{players}}

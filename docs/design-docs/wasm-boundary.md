# WASM Boundary
Status: Accepted · Date: 2026-09-16

## 결정
JS↔WASM 경계는 **`Game` 객체 하나**. 프레임당 경계 호출은 상수 개(≈ 패킷 수 + 4). 대량 데이터(렌더)는 **wasm 메모리 포인터 + 길이**를 넘기고 JS가 TypedArray 뷰로 읽는다.

## 빌드
```bash
cargo build -p hg-web --target wasm32-unknown-unknown --release
wasm-bindgen --target web --out-dir web/src/wasm-pkg target/wasm32-unknown-unknown/release/hg_web.wasm
wasm-opt -O3 --enable-bulk-memory --enable-simd -o web/src/wasm-pkg/hg_web_bg.wasm web/src/wasm-pkg/hg_web_bg.wasm
```
- `wasm-bindgen-cli` 버전 == `Cargo.lock`의 `wasm-bindgen` 버전 (불일치 시 빌드 실패). CI에서 lock에서 읽어 설치
- ggrs는 `features = ["wasm-bindgen"]` (getrandom 0.2 `js` + `instant` wasm 활성). `instant`는 RUSTSEC unmaintained 권고 대상 → `deny.toml` 예외 + TD-007

## `Game` API (crates/web)
```rust
#[wasm_bindgen]
impl Game {
    #[wasm_bindgen(constructor)]
    pub fn new(num_players: u8, local_handle: u8, seed: u32, map_id: u8) -> Result<Game, JsError>;
    pub fn push_packet(&mut self, slot: u8, bytes: &[u8]);        // 복사 1회
    pub fn tick(&mut self, input_bits: u16) -> u8;                 // TickStatus
    pub fn take_outbox(&mut self) -> Vec<u8>;                      // [slot u8][len u16 LE][bytes]...  → Uint8Array
    pub fn render_ptr(&self) -> *const i32;
    pub fn render_len(&self) -> usize;                             // i32 개수
    pub fn take_events(&mut self) -> Vec<u8>;                      // 이번 tick 새 이벤트 [frame u32][seq u16][kind u8][x i32][y i32]
    pub fn net_stats_json(&self, slot: u8) -> String;              // 1Hz 이하로만 호출
    pub fn state_dump(&self) -> Vec<u8>;                           // desync 덤프용
}
```
`TickStatus`: `0 Advanced`, `1 Waiting(sync 전)`, `2 Skipped(타임싱크)`, `3 Ended`
- 에러는 `JsError`로. `tick` 내부 에러는 status + 콘솔 로그, panic 금지 (`panic = "abort"` + `console_error_panic_hook`은 dev에서만)

## 메모리
- `Game::new`에서 State(`Box`), RenderView 버퍼(`Vec<i32>` 용량 고정), outbox 용량을 **미리 할당** → 플레이 중 메모리 growth 없음이 목표
- 그래도 JS는 **매 프레임 뷰를 새로 만든다**: `new Int32Array(wasm.memory.buffer, ptr, len)` — growth 시 기존 `ArrayBuffer`가 detach되기 때문. 비용 무시 가능
- `memory.buffer.byteLength` 변화는 dev 빌드에서 경고 로그 (예상 못 한 할당 탐지)

## RenderView 레이아웃
헤더 16×i32 + 고정 stride 레코드. 정의 원본은 `crates/web/src/view.rs`, 문서는 [generated/render-view.md](../generated/render-view.md) (gen-docs 생성).
JS 쪽 오프셋 상수 `web/src/core/view-layout.ts`도 gen-docs가 생성 → 손으로 맞추지 않는다.

## 기각한 대안
- **엔티티별 getter 호출**: 400엔티티×필드 수 경계 호출 → 프레임당 수천 번
- **serde-wasm-bindgen으로 State JSON화**: GC 압박, 프레임당 수십 KB 할당
- **web-sys로 Rust에서 Canvas 호출**: 호출마다 경계 비용, 렌더 코드가 결정론 크레이트 옆에 붙어 경계가 흐려짐

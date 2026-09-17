# ARCHITECTURE

## 조감도
```
 ┌──────────────────────────── Browser (Peer A) ─────────────────────────────┐
 │  web/ (TypeScript + Vite)                  crates/ (Rust → hordegrid.wasm)│
 │  ┌──────────┐ u16 input  ┌─────────────────────────────────────────────┐  │
 │  │ input/   │───────────▶│ crates/web  #[wasm_bindgen] Game (얇게)      │  │
 │  └──────────┘            │   └─ crates/net  GGRS P2PSession 구동        │  │
 │  ┌──────────┐ RenderView │         │   BridgeSocket(패킷 in/out 큐)    │  │
 │  │ render/  │◀───────────│         └─ crates/sim  step(&mut State, ..) │  │
 │  └──────────┘ zero-copy  └──────────────▲──────────────┬───────────────┘  │
 │  ┌──────────┐ bytes in                  │              │ bytes out        │
 │  │ net/     │───────────────────────────┘              │                  │
 │  │ trystero │◀─────────────────────────────────────────┘                  │
 │  └────┬─────┘                                                             │
 └───────┼───────────────────────────────────────────────────────────────────┘
         │ ① 시그널링: Nostr 공개 릴레이 경유 SDP/ICE (연결 후 불필요)
         │ ② 게임 패킷: 별도 DataChannel  negotiated, ordered:false, maxRetransmits:0
         ▼
   Peer B, C, D   (풀 메시, 최대 4인)
```
정적 파일(`index.html`, `*.js`, `hordegrid_bg.wasm`, 에셋)은 GitHub Pages가 서빙. **우리가 운영하는 서버는 0대.**

## 레이어와 의존 방향 (위 → 아래로만)
| 레이어 | 위치 | 책임 | 금지 |
|---|---|---|---|
| **sim** | `crates/sim` | 게임 규칙, 고정소수점 이동/충돌, 적 AI, 무기, 웨이브. `fn step(&mut State, &MapData, &mut Scratch, &[PlayerInput; 4])` | I/O, 시간, float, 힙, 외부 크레이트(`bytemuck`, `xxhash-rust` 외) |
| **net** | `crates/net` | GGRS `Config` 구현, 세션 빌드, 요청(Save/Load/Advance) 처리, `BridgeSocket`, 입력 인코딩 | 게임 규칙, JS/web-sys 의존 |
| **web** | `crates/web` | `#[wasm_bindgen]` export, 패킷 in/out, RenderView 채우기, 에러→JS 변환 | 로직. 함수 40줄 넘으면 설계 오류 |
| **app** | `web/src` | 룸 참가, trystero, DataChannel, 입력 수집, Canvas 렌더, 사운드, UI | sim 상태 쓰기, 게임 규칙 계산 |
| **xtask** | `xtask/` | desync, netsim, bench, gen-docs, lint-arch (native 전용) | 프로덕션 크레이트가 의존 |

- `sim`, `net`은 native와 `wasm32-unknown-unknown` 양쪽에서 빌드. 테스트·벤치·netsim은 native.
- 의존 방향은 `xtask lint-arch`가 강제: `cargo metadata` 의존 그래프 + TS import 경로 규칙 검사.

## 디렉터리 맵
```
Cargo.toml (workspace)  rust-toolchain.toml  clippy.toml  justfile
crates/
  sim/   src/{lib.rs,config.rs(수치 테이블),state.rs,input.rs,fx.rs,rng.rs,map.rs,flow.rs,grid.rs,physics.rs,step.rs}
         tests/{determinism.rs,gameplay.rs,common/}  examples/{diag.rs,bench.rs}
  net/   src/{lib.rs,input.rs,codec.rs,socket.rs,session.rs}  tests/{netsim.rs,synctest.rs}
  web/   src/{lib.rs,view.rs}
(xtask/ 미구현 — desync·gen-docs는 TD-010, 아키텍처 lint는 web/scripts/lint-arch.mjs)
tools/
  net-probe/   연결 실측 단일 HTML 도구 (trystero + 비신뢰 채널 검증, Phase 1 스파이크)
web/
  index.html  vite.config.ts (base: '/hordegrid/')  package.json
  src/main.ts  화면 흐름·조립(DI 지점)   src/match.ts  게임 1판(루프·패킷·이벤트·HUD)
  src/core/    wasm.ts wasm-framing.ts loop.ts view-layout.ts ids.ts storage.ts mock-wasm.ts
  src/net/     protocol.ts(검증) lobby.ts(순수 상태머신) room.ts(trystero+게임 채널)
  src/input/   encode.ts keyboard.ts gamepad.ts touch.ts controller.ts
  src/render/  renderer.ts effects.ts audio.ts palette.ts
  src/ui/      dom.ts
  test/        lobby.test.ts input.test.ts   scripts/lint-arch.mjs
  public/assets/
.github/workflows/  ci.yml  pages.yml
docs/  (AGENTS.md 참고)
```

## 한 프레임의 흐름 (60Hz, rAF 누산기)
1. `net/`: 수신 패킷 → `game.push_packet(peer_slot, bytes)` → `BridgeSocket` inbox
2. `game.tick(local_input_bits)` → `crates/net::Runner::tick`
   1. `session.poll_remote_clients()` → 이벤트 처리 (Synchronized/Disconnected/WaitRecommendation/DesyncDetected)
   2. `frames_ahead()`가 임계 이상이면 이번 틱 스킵 (타임싱크)
   3. `add_local_input` → `advance_frame()` → 요청 순서대로 처리
      - `SaveGameState{cell, frame}` → `cell.save(frame, Some(state.clone()), Some(checksum))`
      - `LoadGameState{cell, ..}` → `state = cell.load()`
      - `AdvanceFrame{inputs}` → `sim::step`
3. `net/`: `game.drain_outbox()` → 피어별 DataChannel 전송
4. `render/`: `game.render_view_ptr()/len()` → `Int32Array` 뷰 → 이전 프레임과 보간 → Canvas2D

## 주요 결정 (근거는 design-docs)
| 결정 | 선택 | 기각한 대안 | 문서 |
|---|---|---|---|
| 언어 | Rust | C++20+Emscripten(롤백 직접 구현 부담), Go(바이너리 크기) | [core-beliefs.md](docs/design-docs/core-beliefs.md) |
| 동기화 | GGRS 0.13 롤백 | 락스텝(입력지연 체감), 호스트 권위(호스트 이탈=종료) | [netcode-rollback.md](docs/design-docs/netcode-rollback.md) |
| 수치 | Q16.16 자체 `Fixed` (bytemuck Pod) | f32 (native 테스트와 비교 불가), `fixed` 크레이트(Pod 연동·API 과다) | [determinism.md](docs/design-docs/determinism.md) |
| 시그널링 | trystero Nostr + 수동 SDP 폴백 | 자체 WebSocket 서버, matchbox(시그널링 서버 필요) | [signaling.md](docs/design-docs/signaling.md) |
| JS 경계 | wasm-bindgen 단일 `Game` 객체 + 포인터 뷰 | 매 엔티티 getter(경계 호출 폭증), web-sys로 Canvas 직접 | [wasm-boundary.md](docs/design-docs/wasm-boundary.md) |
| 빌드 | cargo + wasm-bindgen-cli(버전 고정) + wasm-opt | wasm-pack(유지보수 이관), trunk(Vite와 중복) | [RELIABILITY.md](docs/RELIABILITY.md) |
| 렌더 | Canvas2D + 스프라이트 아틀라스 | WebGL (MVP 엔티티 수에선 불필요, 부채 기록) | [FRONTEND.md](docs/FRONTEND.md) |
| 배포 | GitHub Pages + Actions | Netlify/Vercel | [RELIABILITY.md](docs/RELIABILITY.md) |

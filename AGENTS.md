# AGENTS.md — hordegrid

> 에이전트용 **지도**다. 백과사전이 아니다. "어디를 봐야 하는지"만 적고 세부는 `docs/`에 둔다.
> 이 파일이 120줄을 넘기면 내용을 `docs/`로 내리고 링크만 남겨라. (`CLAUDE.md`는 이 파일의 심볼릭 링크)

## 한 줄 요약
서버 없이 브라우저끼리 WebRTC로 붙는 **2~4인 협동 탑다운 좀비 서바이벌**(박스헤드류).
게임 로직은 **Rust → WASM 결정론적 시뮬레이션**, 동기화는 **GGRS 롤백**, 매칭은 **trystero(Nostr 공개 릴레이)**,
배포는 **GitHub Pages 정적 호스팅**. `hordegrid`는 가칭 — 박스헤드 이름·에셋은 쓰지 않는다 → [SECURITY.md](docs/SECURITY.md#ip-지식재산)

## 작업 시작 전 반드시 읽을 것 (순서대로)
1. [docs/design-docs/core-beliefs.md](docs/design-docs/core-beliefs.md) — 절대 규칙. 위반 PR은 무조건 반려
2. [ARCHITECTURE.md](ARCHITECTURE.md) — 레이어와 의존 방향
3. [docs/PLANS.md](docs/PLANS.md) → `docs/exec-plans/active/` — 지금 뭘 하는 중인지
4. 작업 영역별 문서 (아래 표)

## 작업 영역 → 문서
| 건드리는 곳 | 먼저 읽을 문서 |
|---|---|
| `crates/sim` (게임 규칙) | [determinism.md](docs/design-docs/determinism.md), [product-specs/](docs/product-specs/index.md) |
| `crates/net` (GGRS) | [netcode-rollback.md](docs/design-docs/netcode-rollback.md), [references/ggrs-llms.txt](docs/references/ggrs-llms.txt) |
| `crates/web` (wasm-bindgen 경계) | [wasm-boundary.md](docs/design-docs/wasm-boundary.md), [references/wasm-bindgen-llms.txt](docs/references/wasm-bindgen-llms.txt) |
| `web/src/net` | [signaling.md](docs/design-docs/signaling.md), [references/trystero-llms.txt](docs/references/trystero-llms.txt) |
| `web/` 렌더/입력/UI | [FRONTEND.md](docs/FRONTEND.md), [DESIGN.md](docs/DESIGN.md) |
| CI/배포 | [RELIABILITY.md](docs/RELIABILITY.md), [references/github-pages-llms.txt](docs/references/github-pages-llms.txt) |
| 설계·최적화·품질 기준 | [patterns.md](docs/design-docs/patterns.md), [performance.md](docs/design-docs/performance.md), [code-quality.md](docs/design-docs/code-quality.md), [QUALITY_SCORE.md](docs/QUALITY_SCORE.md) |
| 모바일 | [mobile-play.md](docs/product-specs/mobile-play.md) — 모든 UI/입력 변경은 터치에서도 동작해야 함 |
| 기능 추가 판단 | [PRODUCT_SENSE.md](docs/PRODUCT_SENSE.md) |
| 연결 실측 | [tools/net-probe](tools/net-probe/README.md), [0002](docs/exec-plans/active/0002-connectivity-spike.md) |
| 외부 API | `docs/references/*-llms.txt` — **기억으로 API 쓰지 말고 여기부터**, 없으면 조사 후 추가 |

## 명령어 (계약 — 이름이 바뀌면 이 표부터 고친다)
```bash
just check     # fmt --check, clippy pedantic -D warnings, cargo test, tsc, vitest, web/scripts/lint-arch.mjs
just netsim    # crates/net/tests/netsim.rs: 4피어 GGRS, 지연 250ms·지터 80·손실 10% 포함 체크섬 일치
just sim-test  # sim 단위·결정론·게임플레이 AC 테스트 (release)
just diag      # 봇 플레이 헤드리스 진단 (웨이브/킬/생존)
cargo run -p hg-sim --release --example bench   # 최악 step 비용
just dev       # wasm(debug) + wasm-bindgen + vite dev
just build     # release wasm + wasm-opt + vite build → web/dist
# 미구현(TD-010): just desync (native↔wasm 비교), just gen-docs
```
PR 전 `just check` 필수. `crates/net`·입력 인코딩 변경 시 `just netsim` 추가.
Rust 툴체인 없이 UI 작업: `cd web && npx vite` → `src/core/mock-wasm.ts`가 sim 대신 동작(네트워크 게임 불가).

## 작업 루프
1. **계획**: 30분 넘는 작업은 `docs/exec-plans/active/NNNN-slug.md` 생성 ([PLANS.md](docs/PLANS.md) 템플릿)
2. **구현**: 체크박스 갱신. 결정이 바뀌면 계획 문서의 *Decision log*에 한 줄
3. **검증**: 위 명령어. 수치 예산은 [QUALITY_SCORE.md](docs/QUALITY_SCORE.md)
4. **정리**: 계획을 `completed/`로 이동, 남은 부채는 [tech-debt-tracker.md](docs/exec-plans/tech-debt-tracker.md)
5. 문서와 코드가 다르면 **문서를 먼저 고친다**. 틀린 문서는 없는 문서보다 해롭다

## 하지 말 것 (자주 틀리는 것)
- `crates/sim`에서 `f32/f64`, `HashMap/HashSet`, `std::time`, `rand`, `web-sys`, `Vec`/`Box` (clippy.toml이 막음)
- `State`에 `Pod` 아닌 필드 추가 → 스냅샷·체크섬 붕괴. `bytemuck::Pod` derive가 컴파일 타임에 막는다
- `Fixed`에 원시 `+ - *` 대신 `i32` 직접 연산 → release에선 조용히 wrap. `fx` 헬퍼만 사용
- 렌더/UI 코드가 sim 상태를 **쓰기**, 로컬 입력 즉시 반영 같은 "직접 예측" → GGRS가 한다
- JS에서 `memory.buffer` 기반 TypedArray 캐싱 → [wasm-boundary.md](docs/design-docs/wasm-boundary.md#메모리)
- 게임/시그널링 서버 추가, `SharedArrayBuffer`·wasm 스레드 (Pages는 COOP/COEP 헤더 불가)
- `wasm-pack` 도입 (rustwasm org 아카이브 이후 CLI 직접 사용으로 결정), `docs/generated/` 손수정

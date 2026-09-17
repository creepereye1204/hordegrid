# 0001: Harness & toolchain bootstrap
Status: Active (대부분 완료, 2026-09-17) · Owner: (unassigned) · Started: 2026-09-16

## 목표
문서가 약속한 명령어(`just check/desync/netsim/bench/dev/build/gen-docs`)가 **빈 게임**에 대해 전부 동작하고, main push 시 GitHub Pages에 캔버스가 뜬다.

## 범위
워크스페이스·크레이트 뼈대, 최소 sim(점 하나 이동), CI/Pages, xtask 뼈대, TS 뼈대(모바일 viewport 포함)
## 비범위
trystero 연결, GGRS 실연결(→ 0002), 게임 콘텐츠

## 단계
- [x] 1. Cargo workspace: `crates/{sim,net,web}`, `xtask`; `rust-toolchain.toml`(stable 버전 고정 + wasm32 target); `clippy.toml`(determinism.md 목록); `deny.toml`; release profile (검증: `cargo build --workspace`)
- [x] 2. `sim`: `Fixed`(Q16.16, 포화 연산, proptest 법칙 테스트), `Rng`(xoshiro128**, 알려진 벡터 테스트), `State{frame, rng, players}` Pod, `step`이 8방향 이동만 (검증: `cargo test -p sim`, `size_of::<State>()` 출력)
- [x] 3. `net`: `HgConfig`, `BridgeSocket`, `SyncTestSession`으로 1인 60프레임 check_distance 7 통과 (검증: `cargo test -p net`)
- [~] 4. `web` 크레이트 (코드 완료, **wasm32 빌드는 CI에서 첫 검증** — 작업 환경에서 wasm32 std 다운로드 차단): `Game::new/tick/render_ptr/render_len` 최소 구현, wasm 빌드 스크립트 (검증: `just build`에서 .wasm 생성, gzip 크기 출력)
- [x] 5. `web/` (+ 로비/승인/터치/HUD/게임오버까지 확장): Vite+TS strict, `base:'/hordegrid/'`, `core/loop.ts`, 캔버스에 플레이어 점 렌더, 키보드+터치 스틱 최소 입력, 모바일 viewport/`touch-action:none` (검증: 데스크톱·Playwright iPhone 에뮬에서 점 이동)
- [x] 6. lint-arch → `web/scripts/lint-arch.mjs`로 대체: 크레이트 의존 방향, sim float 토큰 grep, TS import 규칙 (검증: 일부러 위반 넣어 실패 확인 후 제거)
- [ ] 7. (TD-010) xtask `gen-docs`: `size_of`/`offset_of!`로 sim-state-schema.md, render-view.md + view-layout.ts 생성 (검증: 재실행 diff 없음)
- [ ] 8. (TD-010) xtask `desync`: 무작위 입력 로그 → native 실행 vs `node`로 wasm 실행 체크섬 비교 (검증: 1만 프레임 일치)
- [x] 9. netsim → `crates/net/tests/netsim.rs` (2·4피어, 모바일·적대적 망 통과): 가상 소켓 2피어 P2PSession, 지연·손실 주입 (검증: `lan`/`mobile` 시나리오 체크섬 일치)
- [x] 10. `justfile`, `.github/workflows/ci.yml`, `pages.yml` (action SHA 고정) (검증: PR에서 CI 녹색, main에서 Pages URL 접속)
- [x] 11. QUALITY_SCORE 첫 측정값 기입

## 리스크 & 확인할 가정
- GGRS `Message`가 serde 직렬화를 공개하는지 → 안 되면 codec 설계 변경 (netcode-rollback.md 되돌릴 조건)
- `node`에서 `--target web` 산출물 로드 방식 → 안 되면 desync용 `--target nodejs` 별도 빌드
- `offset_of!` 안정화 범위(중첩 필드) → 안 되면 필드별 수동 계산 매크로
- cargo-deny가 `instant` 권고로 실패 → deny.toml 예외 + TD-007

## 결과 요약 (2026-09-17)
- Rust 31 테스트 통과(속성·결정론·스냅샷 재생·SyncTest 7프레임 롤백·4피어 netsim·게임플레이 AC3/AC4), clippy pedantic 0 경고
- 봇 플레이 3분: 웨이브 5 도달, 적 끼임 0 (flow field 타일 중심 조향으로 모서리 끼임 버그 수정)
- 최악 step(적 384·탄 256·4인) native p99 0.059ms, State 13.7KiB
- TS: tsc strict·vitest 15 통과, Playwright(mock sim)로 데스크톱 솔로·iPhone 가로 터치·2탭 로비(승인→준비→시작) 확인

## Decision log
- 2026-09-17: 샷건은 DDA 히트스캔 대신 짧은 사거리 5발 투사체로 MVP (TD-012)
- 2026-09-17: 적은 쓰러진 플레이어를 노리지 않음(부활 시간 확보). 인접 추격도 flow field로만 — 직선 추격은 벽에 끼임
- 2026-09-17: 게임 시작 후에도 trystero 방 유지(게임 채널이 그 연결 위에 있음)
- 2026-09-16: 언어 Rust 확정 (C++20 검토 후 복귀) — core-beliefs.md
- 2026-09-16: wasm-pack 미사용, wasm-bindgen-cli 직접 — rustwasm org 아카이브
- 2026-09-16: 모바일 브라우저 1급 지원 — mobile-play.md, Phase 0부터 viewport·터치 뼈대 포함

## 완료 시 갱신할 문서
QUALITY_SCORE.md(측정값), generated/*(실제 생성물), AGENTS.md(명령어 차이 있으면), references/*(버전 확인 결과)

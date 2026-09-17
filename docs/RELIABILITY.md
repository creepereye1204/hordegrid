# RELIABILITY

"서버가 없다" = 우리가 고칠 수 있는 곳은 **클라이언트와 CI뿐**. 신뢰성은 배포 파이프라인, 연결 실패 처리, desync 대응으로 확보한다.

## 빌드 & 배포 파이프라인
### `ci.yml` (PR, main push)
1. Rust: `rust-toolchain.toml` 고정(stable 버전 명시) + `wasm32-unknown-unknown` target, `Swatinem/rust-cache`
2. `just check` → `just desync` → (net 변경 시) `just netsim`
3. `wasm-bindgen-cli` 설치: `Cargo.lock`의 wasm-bindgen 버전 추출해 `cargo install wasm-bindgen-cli --version $V --locked`
4. `binaryen`(wasm-opt) 설치 → `just build`
5. 번들 크기 검사 (QUALITY_SCORE 로딩 B 기준 초과 시 실패)
6. Playwright E2E (Loopback/Manual transport)

### `pages.yml` (main push, ci 성공 후)
- `actions/upload-pages-artifact` → `actions/deploy-pages` (Pages 소스 = GitHub Actions)
- 권한: `pages: write`, `id-token: write`
- 산출물 `web/dist`에 `buildHash`(= git short sha) 주입 → 로비 호환성 검사에 사용
- 세부는 [references/github-pages-llms.txt](references/github-pages-llms.txt)

### 롤백(배포)
Pages는 버전 보관을 안 한다 → 문제 배포 시 `git revert` 후 main push가 유일한 되돌리기. **main은 항상 배포 가능 상태 유지**, 직접 push 금지(브랜치 보호).

## 캐시 & 버전 혼재
- Pages는 캐시 헤더를 제어할 수 없음(`max-age=600` 수준) → 친구끼리 **다른 빌드**가 뜰 수 있음
- 대응: Vite 해시 파일명 + `index.html`의 buildHash를 로비 `hello`에서 비교 ([signaling.md](design-docs/signaling.md)) → 불일치 시 새로고침 안내
- 프로토콜 비호환 변경은 `PROTO_VER` 증가 → appId가 달라져 애초에 매칭 안 됨

## 연결 신뢰성
| 실패 | 빈도 가정 | 대응 | 담당 |
|---|---|---|---|
| 일부 Nostr 릴레이 다운 | 흔함 | trystero 다중 릴레이 redundancy | trystero 설정 |
| 모든 릴레이 차단(회사망 등) | 드묾 | 수동 SDP 폴백 | `ManualTransport` |
| 대칭 NAT ↔ 대칭 NAT | 측정 중 ([0002](exec-plans/active/0002-connectivity-spike.md) 표) | TURN 없음 → 명확한 안내, 실패율 > 10%면 TURN 재결정 | UX / signaling.md |
| 순간 끊김 < 3초 | 모바일에서 흔함 | GGRS disconnect_timeout 3000ms | 설정 |
| 탭 백그라운드 | 흔함 | hidden 2초 → 자진 이탈 | `loop.ts` |
- **텔레메트리 서버가 없으므로** 연결 성공률은 Phase 4 수동 테스트(여러 네트워크 조합 표)로 측정·기록

## Desync 대응 절차
1. 인게임: `DesyncDetected` → HUD 경고 + [덤프 저장] 버튼 (JSON: buildHash, seed, 플레이어 수, 최근 600프레임 확정 입력, 로컬 State bytes base64, 체크섬 이력)
2. 사용자가 이슈에 첨부 (이슈 템플릿 `desync.yml`)
3. `xtask desync --replay dump.json` → 해당 buildHash 체크아웃 후 native/wasm 재생, 첫 불일치 프레임에서 **State 필드 단위 diff** 출력(gen-docs 오프셋 표 활용)
4. 원인 수정 + 해당 입력 로그를 `crates/sim/tests/regressions/`에 회귀 테스트로 추가

## 네트워크 시뮬레이터 (`xtask netsim`)
- 4개 GGRS 세션을 한 프로세스에서 구동, 가상 소켓이 지연(고정+지터)·손실·중복·역순 주입, 가상 시계
- 시나리오: `lan`, `wifi`(RTT 40, 손실 1%), `mobile`(150/30/3%), `hostile`(250/80/10%, 중복 2%), `partition`(C가 A와만 3초 단절), `drop_out`(B가 프레임 2000에서 영구 이탈)
- 판정: 확정 프레임 체크섬 전원 일치 + QUALITY_SCORE 네트워크 체감 수치 출력
- seed 고정 → 실패 재현 가능

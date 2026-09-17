# Code Quality Bar
Status: Accepted · Date: 2026-09-16

"최상급"을 감으로 두지 않는다. 아래는 **CI가 기계적으로 막는 것**과 **리뷰가 보는 것**으로 나눈다. 점수화는 [QUALITY_SCORE.md](../QUALITY_SCORE.md).

## CI가 막는 것 (실패 = 머지 불가)
### Rust
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings -W clippy::pedantic` (+ 크레이트별 명시적 `allow`는 사유 주석 필수)
- `sim`: `#![forbid(unsafe_code)]`, `#![deny(clippy::float_arithmetic, clippy::disallowed_types, clippy::unwrap_used)]`
- `net`, `web`: `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`
- `cargo test --workspace` (unit + integration + doc tests)
- **property test** (`proptest`): 코덱 왕복, fx 연산 법칙(교환·결합·포화), 저장/복원 재생 동일
- **snapshot test** (`insta`): 시나리오 N프레임 후 요약 상태(RON) — 밸런스 변경이 의도치 않게 번지는 것 탐지
- `cargo deny check` (라이선스·보안 권고·중복 크레이트)
- `cargo llvm-cov` 라인 커버리지: `sim` ≥ 85%, `net` ≥ 80% (web 제외)
- `xtask lint-arch`: 레이어 의존, sim float 토큰, `docs/generated` 최신성(재생성 diff 없음)

### TypeScript
- `strict: true`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`
- ESLint `typescript-eslint` `strictTypeChecked` + `no-floating-promises`
- `vitest` unit (입력 인코딩, 로비 상태머신, 보간), 커버리지 `web/src/{net,input,core}` ≥ 80%
- Playwright E2E: 한 브라우저 두 컨텍스트 + `LoopbackTransport`/`ManualTransport`로 로비→게임 시작→10초 플레이→체크섬 일치 (외부 릴레이 의존 테스트는 nightly만)

### 주기 작업 (nightly)
- `cargo mutants -p sim` — 살아남은 뮤턴트는 tech-debt로
- `cargo fuzz`: `codec` 디코더 5분 (패킷 변조 가정, core-beliefs #4)
- `just netsim --long` 10만 프레임, 손실 10%·지터 80ms

## 리뷰가 보는 것
1. **이름**: 도메인 용어([PRODUCT_SENSE.md](../PRODUCT_SENSE.md#용어)) 사용. `data`, `info`, `manager` 금지
2. **함수**: 한 가지 일, 60줄 이하 권장. sim 핫루프는 예외 허용하되 블록 주석으로 단계 표시
3. **에러**: `thiserror`로 도메인 에러 enum. 문자열 에러 금지. 경계(web)에서만 `JsError` 변환
4. **주석**: *왜*만. 수치 상수는 근거(“0.5타일 미만: 터널링 방지”)
5. **테스트 이름**: `when_<조건>_then_<결과>` 
6. **커밋**: Conventional Commits, 1커밋 1의도. 결정 변경이 있으면 문서 diff가 같은 PR에 있어야 함
7. **공개 API**: `crates/*`의 `pub` 항목은 doc comment 필수 (`#![warn(missing_docs)]`)

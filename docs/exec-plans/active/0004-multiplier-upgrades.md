# 0004: 킬 배율 & 배율 해금
Status: Active · Owner: · Started: 2026-11-10

## 목표
[multiplier-and-upgrades.md](../../product-specs/multiplier-and-upgrades.md) 스펙대로 콤보(0 리셋)·점수 해금을 배율(1단계 감쇠·창 축소)·배율 해금으로 교체한다.

## 범위 / 비범위
**범위**: `crates/sim` 배율·창 테이블·해금표·`effective_weapon`, `Wave` 필드 교체, `Upgrade` 이벤트, `crates/web` 뷰 노출, HUD 배율·게이지·토스트, 관련 스펙 문서 정리
**비범위**: 바리케이드·폭발통·레일건 구현(해금 배율만 예약), 숨바꼭질 모드(배율 없음 — `hide_seek.rs` 경로는 건드리지 않는다), 원작의 멀티킬 보너스(`mMultipleKill`) 계열

## 단계
- [ ] 1. `config.rs`: `MULT_MAX`, `WIN_MIN`, `WIN_MAX`, `isqrt` `const fn`, `MULT_WINDOW` const 테이블, `UNLOCKS` 표(임계·대상·효과 enum). `COMBO_*`, `unlock_score` 삭제 (검증: AC1 단위 테스트)
- [ ] 2. `state.rs`: `Wave` 필드 교체(`mult`, `mult_timer`, `best_mult`, `mult_window`), `EventKind::Upgrade` 추가 — POD 유지 (검증: 컴파일 = `Pod` derive 통과)
- [ ] 3. `step.rs`: `update_combo` → `update_mult`(인터미션 정지·1단계 감쇠), 처치 경로에서 `mult += 1`·점수 `× mult`·`best_mult` 갱신 시 지나친 임계마다 이벤트, `unlock_weapons`를 `best_mult` 기준으로 (검증: AC2~AC5 시나리오 테스트 `tests/gameplay.rs`)
- [ ] 4. `effective_weapon(kind, best_mult)` 순수 함수, 발사·재장전 경로가 `WEAPONS[k]` 대신 이것만 쓰게 (검증: AC6 매트릭스 테스트)
- [ ] 5. `just sim-test`, `tests/determinism.rs`, `just netsim` (검증: AC7)
- [ ] 6. `crates/web` 뷰에 `mult`, `mult_timer`, `mult_window`, `best_mult` 노출, `web/types/hg_web.d.ts` 갱신 (검증: `just check`의 tsc)
- [ ] 7. HUD: 배율 숫자·게이지·"다음 해금" 표시, `Upgrade` 이벤트 토스트 문구 테이블(TS), 모바일 가로 레이아웃에서 겹침 없음 (검증: 수동 — 데스크톱 + 폰 1대)
- [ ] 8. `just diag` 봇 2인·4인 10시드로 로켓 도달 웨이브 측정 → AC8. 벗어나면 스펙의 `WIN_MAX`/임계를 먼저 고치고 재측정 (검증: diag 출력 표를 이 문서 Decision log에 붙임)

## 리스크 & 확인할 가정
- `isqrt` const fn이 MSRV(rust-toolchain.toml)에서 const 문맥 루프 허용되는지 — 안 되면 표를 생성 스크립트로 만들어 리터럴 커밋 + 공식 재계산 테스트
- 처치 경로가 여러 곳(투사체·폭발·히트스캔)이면 `mult` 증가가 한 곳으로 모였는지 확인 — 현재 `step.rs`의 "Apply damage to one enemy" 함수 하나로 모여 있음(2026-11-10 기준)
- 배율 HUD가 렌더 롤백 중복 이벤트에 토스트를 두 번 띄우지 않게 기존 `(frame, seq)` 중복 제거 경로를 탈 것

## Decision log
- 2026-11-10: 원작 디컴파일 근거로 콤보 규칙 교체 결정 — [teardown](../../references/boxhead-2play-teardown.md) §2.7, §3
- 2026-11-10: 해금 상태를 비트마스크 대신 `best_mult`에서 파생 — 배율이 1씩만 오르므로 임계를 건너뛸 수 없음, State 필드 절약
- 2026-11-10: 인터미션 중 배율 정지 — 원작엔 인터미션이 없어 그대로 옮기면 웨이브마다 사실상 리셋

## 완료 시 갱신할 문서
- [core-gameplay.md](../../product-specs/core-gameplay.md) 콤보 절 → 이미 새 스펙 링크로 교체됨, AC 체크
- [weapons-and-enemies.md](../../product-specs/weapons-and-enemies.md) 해금 열
- [product-specs/index.md](../../product-specs/index.md) 상태 `Shipped`
- `docs/generated/*` (TD-010 해결 시)

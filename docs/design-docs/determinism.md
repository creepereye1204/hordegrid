# Determinism
Status: Accepted · Date: 2026-09-16

## 결정
`crates/sim`은 정수·고정소수점만 사용하고, 오버플로 정책이 명시적이며, 상태는 패딩 없는 `Pod`다.

## 규칙
### 수치
- 좌표/속도: `fx::Fixed(i32)` = Q16.16, `#[repr(transparent)]` + `Pod`. 월드 1타일 = `Fixed::ONE` (65536)
- 연산자 impl: `Add/Sub`는 `saturating_*`, `Mul`은 `(a as i64 * b as i64) >> 16`을 `i32`로 포화 변환, `Div`는 `b == 0`이면 0 (`debug_assert!`)
- 원시 `i32` 좌표 연산 금지 — release 빌드에서 overflow는 **조용히 wrap**. `[profile.test]`, `[profile.dev]`에 `overflow-checks = true`
- 방향: 조준·이동은 **8방향**(박스헤드 감성) → `DIR8: [(Fixed, Fixed); 8]` 상수 테이블. 대각 성분 = `0.70710678 * 65536 = 46341`
- 거리 비교는 제곱거리(`i64`). `sqrt` 필요 시 `isqrt_u64` 정수 뉴턴 고정 반복

### 난수
- `sim::Rng` = xoshiro128** (`[u32; 4]`), **State 안에 포함** → 롤백 시 같이 되돌아감
- 시드: 로비 시작 시 모든 피어 `selfId`를 정렬해 연결한 문자열의 xxh3 → 특정 피어가 독점 불가

### 금지 (clippy.toml로 기계적 강제, `crates/sim/src/lib.rs`에 `#![deny(clippy::float_arithmetic, clippy::disallowed_types, clippy::disallowed_methods)]`)
```toml
# clippy.toml
disallowed-types = [
  { path = "std::collections::HashMap", reason = "iteration order nondeterministic" },
  { path = "std::collections::HashSet", reason = "iteration order nondeterministic" },
  { path = "std::time::Instant", reason = "sim must not read time" },
  { path = "std::time::SystemTime", reason = "sim must not read time" },
  { path = "std::vec::Vec", reason = "State must be Pod; use fixed arrays" },
]
```
- `f32`/`f64` 리터럴·타입 자체는 `xtask lint-arch`가 `crates/sim` 소스 grep으로 추가 차단 (clippy float lint는 산술만 잡음)
- `sim`의 `[dependencies]` 허용목록: `bytemuck`, `xxhash-rust`(xxh3 feature). 그 외 추가는 이 문서 PR 필요
- `sim`은 `#![no_std]`를 목표 (Phase 2에서 전환, 부채 TD-003)

### 반복 순서
- 엔티티는 슬롯 인덱스 순으로만 순회. 삭제는 `alive: u8` 플래그 + free-list(State 내부 배열)
- 정렬은 `(key, slot)` 복합키로 완전 순서 → `sort_unstable`이어도 결과 동일

## 상태 레이아웃
```rust
pub const MAX_PLAYERS: usize = 4;
pub const MAX_ENEMIES: usize = 384;
pub const MAX_SHOTS: usize = 512;

#[repr(C)] #[derive(Clone, Copy, Pod, Zeroable)]
pub struct Enemies {            // SoA
    pub x: [Fixed; MAX_ENEMIES], pub y: [Fixed; MAX_ENEMIES],
    pub hp: [i32; MAX_ENEMIES],
    pub kind: [u8; MAX_ENEMIES], pub alive: [u8; MAX_ENEMIES],
    pub target: [u8; MAX_ENEMIES], pub cooldown: [u8; MAX_ENEMIES],  // 384*4 = 4의 배수 → 패딩 0
}

#[repr(C)] #[derive(Clone, Copy, Pod, Zeroable)]
pub struct State {
    pub frame: u32, pub rng: Rng,
    pub players: Players, pub enemies: Enemies, pub shots: Shots,
    pub pickups: Pickups, pub placeables: Placeables,
    pub wave: Wave, pub events: EventRing,
    pub flow: FlowField,        // [u8; 4096] 64×64 DIR8, performance.md#1 참고
}
```
- `#[derive(Pod)]`는 **패딩이 있으면 컴파일 에러** → 체크섬이 피어마다 달라지는 사고를 원천 차단
- `bool` 금지(`Pod` 아님) → `u8`
- 큰 배열 `Pod` derive: bytemuck은 const generic 배열 지원(`[T; N]` where `T: Pod`) — 버전은 [references/bytemuck-llms.txt](../references/bytemuck-llms.txt)
- 맵 타일(정적)은 State 밖 `&MapData` — 롤백 대상 아님. 단 바리케이드 등 동적 차단은 `placeables`로 State 안
- 매 step 처음부터 재구축하는 스크래치(충돌 그리드, BFS 큐)는 State 밖 `Scratch` — 롤백 무관
- 목표 크기 ≤ 64 KiB. `just gen-docs`가 [generated/sim-state-schema.md](../generated/sim-state-schema.md)에 필드 오프셋·크기 출력
- State는 스택에 두지 않는다(64KiB, wasm 기본 스택 1MiB지만 롤백 경로 중첩 위험) → `Box<State>`로 `net`이 소유 (`sim`은 `&mut State`만 받음)

## 스냅샷 & 체크섬
- 저장: GGRS `GameStateCell::save(frame, Some(*state), Some(checksum))` — State가 `Copy`라 복사 1회
- 체크섬: `xxh3_64(bytemuck::bytes_of(&state)) as u128`
- GGRS `DesyncDetection::On { interval: 30 }` → `GgrsEvent::DesyncDetected` 수신 시 [RELIABILITY.md](../RELIABILITY.md#desync)의 덤프 절차

## 검증
| 명령 | 잡는 것 |
|---|---|
| `cargo test -p sim` (overflow-checks on) | 오버플로, 인덱스 OOB(panic) |
| `sim/tests/replay.rs` | 무작위 입력 1만 프레임 2회 실행 동일 / 임의 프레임에서 저장→N프레임→복원→재생 동일 |
| `just desync` | native vs wasm 결과 차이 |
| `just netsim` | 롤백 경로 포함 4피어 체크섬 일치 |

## 기각한 대안
- **f32 + 같은 wasm 바이너리**: 피어끼리는 대체로 일치하지만 native 테스트·desync 러너와 비교 불가, 최적화 플래그 하나로 붕괴 가능
- **AoS 엔티티**: 필드 추가마다 패딩 계산 필요, Pod 에러가 잦음

## 되돌릴 조건
Q16.16 범위(±32768 타일)·정밀도(1/65536) 문제가 실제로 관측될 때만 변경.

# Design Patterns
Status: Accepted · Date: 2026-09-16

원칙: **패턴은 제약(결정론·Pod·롤백·경계 비용)을 푸는 도구로만 쓴다.** 교과서 GoF를 이식하지 않는다.
각 패턴은 "어디서 / 왜 / 대신 쓰지 않는 것"을 명시한다. 새 패턴 도입은 이 표에 행을 추가하는 PR로.

## 채택
| 패턴 | 위치 | 왜 | 대신 쓰지 않는 것 |
|---|---|---|---|
| **Data-Oriented Design (SoA)** | `sim::state` | 캐시 지역성, Pod 패딩 통제, 롤백 스냅샷 1회 복사 | AoS 구조체, ECS 프레임워크(bevy_ecs 등은 Pod 스냅샷 불가) |
| **Functional Core / Imperative Shell** | `sim`(순수) ↔ `net`/`web`/`app`(부수효과) | 결정론·테스트 용이성의 뿌리 | sim 안의 콜백/전역 상태 |
| **Ports & Adapters (Hexagonal)** | `NonBlockingSocket`(Rust), `Transport`(TS) | trystero·수동·루프백·netsim 소켓 교체 | 구체 전송에 직접 의존 |
| **Command** | `PlayerInput(u16)` | 입력 = 직렬화 가능한 명령. 리플레이·롤백·desync 재현이 공짜 | 이벤트 핸들러에서 상태 직접 변경 |
| **Enum dispatch (정적 Strategy)** | 무기 `WeaponKind`, 적 `EnemyKind` → `match` | vtable 없음, Pod 유지, 인라인 가능, 완전성 검사 | `Box<dyn Weapon>` |
| **Flyweight / 데이터 테이블** | `const ENEMY_STATS: [EnemyStats; N]`, `WEAPON_STATS` | 공유 불변 데이터는 State 밖. 스냅샷 크기↓, 밸런스 조정이 한 곳 | 엔티티마다 스탯 복사 |
| **Object Pool + Free-list** | 적/투사체/픽업 슬롯 | 힙 할당 0, 결정론적 슬롯 재사용 | `Vec::push/remove` |
| **State Machine (enum)** | 적 AI `Idle/Chase/Attack/Stagger`, 웨이브 `Intermission/Spawning/Active` | 전이를 표로 테스트 가능 | bool 플래그 조합 |
| **Typestate** | `crates/net`: `SessionBuilder` → `Lobby` → `Running` → `Ended` | 잘못된 순서 호출을 컴파일 에러로 | 런타임 `if state == ...` |
| **Newtype** | `Fixed`, `PeerSlot`, `PlayerHandle`, `Frame`, `EntitySlot` | 단위 혼동 방지 (타일 vs 픽셀, 핸들 vs 슬롯) | 맨 `i32/u8` |
| **Observer (이벤트 링)** | `sim::EventRing` → JS 이펙트/사운드 | sim은 표현을 모름. 롤백 시 `(frame, seq)` 중복 제거 | sim에서 사운드 함수 호출 |
| **Fixed Timestep + Interpolation** | `web/src/core/loop.ts` | sim 60Hz 고정, 렌더는 모니터 주사율 | 가변 dt |
| **Builder** | `GameConfig`, 테스트 시나리오 `ScenarioBuilder` | 테스트에서 "좀비 20마리, 플레이어 2, 샷건" 한 줄 구성 | 거대 생성자 |

## 금지 (리뷰에서 반려)
- **Singleton / 전역 가변 상태** (`static mut`, `thread_local!`, JS 모듈 전역 가변 객체)
- `sim` 내 **trait object**와 제네릭 과다 추상화 — 구현 1개짜리 trait 금지 (포트/어댑터 경계 제외)
- 상속 흉내(Deref 남용)
- "Manager/Helper/Util" 이름의 잡동사니 모듈 — 도메인 이름으로

## TS 쪽 규칙
- 클래스는 수명 있는 리소스(Transport, Renderer)에만. 나머지는 함수 + 불변 데이터
- 상태 변화는 discriminated union + exhaustive `switch` (`never` 체크)
- 의존성은 생성자/팩토리 인자로 주입 (테스트에서 LoopbackTransport 주입)

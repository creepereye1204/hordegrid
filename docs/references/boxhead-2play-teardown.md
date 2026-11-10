# 장르 레퍼런스 분석: Boxhead 2Play (Flash, 2007) 디컴파일 리포트
Status: Reference · Date: 2026-11-10

> 목적: "박스헤드류"라고 **기억으로** 적어둔 규칙을 실제 원작 코드와 대조해, 이식할 가치가 있는 메커니즘을 근거와 함께 남긴다.
> IP: 원작의 이름·에셋·맵·코드는 게임에 쓰지 않는다([SECURITY.md](../SECURITY.md#ip-지식재산)). 이 문서는 **동작 원리만** 기록하고, 인용은 근거 확인용 짧은 발췌로 제한한다. 디컴파일 결과물 자체는 레포에 넣지 않는다.

## 1. 분석 대상과 방법
| 항목 | 값 |
|---|---|
| 파일 | `boxhead.swf` (출처: github.com/MrJeffFeng/Flash-Games) |
| SHA-256 | `34ff145d96891ddea20278700c8b1fb6fd6273ca0e71b73d8bdf0df183fdbece` |
| 형식 | `CWS` v8 (zlib 압축, Flash 8 → **ActionScript 2**), 압축 해제 6,477,486 B |
| 판본 식별 | **2Play 판**. 근거: `CWorld`의 `mGameMode`·`'DeathMatch'`·`'Coop'`·`mSplitScreen`, 클래스 `CWorld_Init_Player2`·`CScreen_Debrief_Winner`, 익스포트 `Hud_PlayerFrags` |

### 1.1 난독화와 우회
- REA(morluto/rea)는 SWF를 지원하지 않음(레포 전체 `swf|shockwave|actionscript|ruffle` grep 0건). JPEXS 디컴파일러는 세션 네트워크 정책상 다운로드 불가 → **AS1/2 디스어셈블러·디컴파일러를 직접 작성** ([tools/as2-teardown](../../tools/as2-teardown/README.md))
- 클래스별 `DoInitAction`(141개)은 전부 같은 골격의 **제어흐름 평탄화 디코이**였다: 상태변수 `\x01`(시드 = 스텁 함수 반환값 + 상수), 불투명 조건, 정크 constantPool. 선형 디스어셈블리에는 실제 연산(getMember/setMember/callMethod)이 **0개**
- 실제 코드는 **범위 밖 점프**(예: `CWorld` 블록에서 `jump -5459`)로 SWF 바이트 스트림의 *다른 태그 영역*에 숨겨져 있었다. Flash Player가 액션을 SWF 버퍼에서 직접 실행하고 점프 범위를 태그로 제한하지 않는 점을 악용한 기법
- 우회: 디스패처를 **구체값으로 실행**(상태변수·불투명 조건은 상수 폴딩, 런타임 값은 기호식)하면서 SWF 전체 버퍼를 코드 공간으로 보고 따라가는 에뮬레이터 → 141개 클래스 전부 복원. 4.6 MB짜리 `sprite755` 프레임 액션은 코드가 아니라 3D 모델(XSI) 데이터 배열

## 2. 시스템별 동작 (근거 포함)
표기: `클래스.메서드` = 복원된 AS2 메서드. 발췌는 디컴파일러 출력 그대로(변수명 원본, 레지스터는 `rN`).

### 2.1 월드 = 32px 셀 격자, 충돌은 셀 비트플래그
- `CMap_Cell.mCellSize = 32`. 각 셀이 `mCollide` 비트를 가짐: `Solid=1, Ledge=2, Shot=4, Path=8, Road=16, Creature=32, Player=64, Object=128, EdgeOfMap=256`
- 조합 상수: `NonMovable = Solid|Ledge|Creature|Object|Player`, `Navigation = Solid|Ledge|Object`, `NoSpawn = Creature|Object`
- 즉 **길찾기는 정적 벽 + 설치물만 장애물로 보고**, 다른 좀비·플레이어는 무시한다(그건 이동 단계에서 처리)

### 2.2 좀비 길찾기 = 플레이어 출발 멀티소스 BFS, **시간 분할 + 세션 스탬프**
`CThing_Collection.Process` 가 매 틱 마지막에:
```
World.Map.CMap_Cell.Nav_Restart();
...모든 플레이어의 현재 셀을 배열에 push...
World.Map.CMap_Cell.Nav_Update(<플레이어 셀 목록>);
```
`CMap_Cell.Nav_Update` / `NF_4`:
- 4방향(`ac[0],ac[2],ac[4],ac[6]`) BFS, 레이어마다 `nSDist += 1`
- 방문 표시는 배열 초기화 대신 **세션 ID 스탬프**: `if (this.nID != nSID) { this.nID = nSID; this.nDist = nSDist; nList.push(this); }` → O(1) 리셋
- **프레임 예산 분할**: `if ((getTimer() - nSTimer) > nTimeOut) { nState = 'continue'; return; }`, `nTimeOut = 3`(ms). 상태 `'restart'/'resume'/'continue'/'finished'`로 다음 틱에 이어서 계산
- 결과는 셀별 거리 `nDist`. 좀비 쪽은 `Nav_Direction`에서 **신선도 검사**: `if ((nSTimer - this.nID) < 2000)` 일 때만 거리장을 신뢰

### 2.3 좀비 이동 = 셀→셀 이산 이동 + **다음 셀 예약**
- `CThing_Creature_Zombie.State_GotoPlayer`: 가장 가까운 플레이어(`GetClosestPlayer`)를 향한 각도를 8방향으로 양자화 → 그 방향 셀에 `Player` 비트가 있으면 공격 상태, 아니면 `Nav_Direction(현재방향, 목표방향, NonMovable, false)`
- `CMap_Cell.Nav_Direction` 우선순위: ① 목표 방향 이웃의 `nDist`가 더 작고 막혀있지 않으면 그 방향 ② 현재 방향이 그렇다면 현재 방향 ③ `nDist` 내림차순 정렬로 탐색 ④ 실패 시 `NavReal_Dir`: 목표방향 ±1, ±2 (45°/90°) 회전 시도
- 대각 이동 가능 판정: `NavReal_1 = ac[1] | ac[0] | ac[2]` — **양쪽 직교 셀까지 비어야** 대각 이동(모서리 끼임 방지)
- 틱당 전체 스캔 상한: `nFullScanCount > 5`면 그 프레임엔 포기(-1) → `State_Wait` (1초 = `CMain.mFPS` 프레임 대기)
- `State_CellToCell_Init`: 출발 전에 `SetCollideDirection(다음셀, mCollide_Creature)`로 **다음 셀을 예약** → 다른 좀비는 그 셀을 막힌 것으로 봄. 이동 프레임 수 = `1 / mSpeed`(대각은 `1.414214 / mSpeed`), 도착 시 셀 중앙 스냅
- 좀비 수치(`Process_Init`): `mSpeed = 0.02` 셀/프레임 @25fps, `mDamage = 50`, `mScore = 100`, HP 100
- **분리력(boids) 없음.** 겹침은 셀 예약만으로 막는다 → 원작 특유의 "줄 서서 오는" 떼 모양의 원인

### 2.4 히트스캔 = 셀 순회 + 선분-원 판정
- `CMap_Cell.LineIntersection`: 셀이 `Solid`면 선분-셀 경계 교점 계산 후 `'wall'`, 아니면 셀 안 오브젝트마다 `QDistance_PointToLine_SQR(위치) < mRadius_SQR`이면 `'thing'` — hordegrid의 DDA 방식과 동일 계열

### 2.5 스폰
- `CWorld.Spawn_ValidPosition`: **랜덤 시작 인덱스 + 순환 탐색**
  ```
  var r3 = random(tList.length);
  ... if (!((tList[r3].mCell.mCollide | tList[r3].mNextCell.mCollide) & NoSpawn)) return tList[r3];
  r3 = (r3 + 1) % tList.length;   // 최대 length회
  ```
  스폰 문과 그 **바로 앞 셀**(`mNextCell`)까지 비어야 스폰
- 코옵 리스폰 `Spawn_Player_PositionObject`: `mGameMode == 'Coop'`이면 동료와 **가깝되 `mSpawnDist_Min = 4` 이상**, 그리고 `Spawn_ValidPosition_WithinPlayerScope`(|dx| ≤ 19, |dy| ≤ 13 셀 = 한 화면) 안. 데스매치는 반대로 먼 곳

### 2.6 레벨(웨이브) 진행 — `CUpgrades.Process` / `InitLists`
- 매 틱 스폰 쿨다운 카운터 감소, 0이 되면 `Spawn_Zombie()` 성공 시 `Zombie_SpawnRate`로 리셋
- 동시 스폰 상한 `Math.min(Level.Zombie_Count, 60)`
- 레벨 테이블(1레벨부터 반복 생성): 스폰 간격 시작 `1 * CMain.mFPS`(1초) → 레벨마다 `Math.max(r8 - 1, 1)` (1프레임씩 단축), 레벨당 좀비 수 10 → 15 → … `Math.min(r4 + 5, 60)`, 좀비 속도 `SpeedUp` 1.1, 1.2, …(상한 5)
- 데빌(원거리형): 스폰 간격 10초에서 시작, 별도 카운터
- 레벨 종료 = 할당량을 다 뽑았고 살아있는 적 0 → `mLevel + 1` (인터미션 없음)

### 2.7 ★ 킬 배율(Multiplier)과 배율 해금 — 원작 루프의 핵심
**점수** — `CWorld.LogKill`:
```
this.mHud.AddScore(..., (tThing.mScore * this.mUpgrades.__get__mMultiplier()));
```
→ 점수 = 기본점수 × **배율**(정수, 1~999)

**배율 증가** — 적 사망 시 `LogKills(1)` → `mMultiplier + 1` (동시에 `mMultipleKill + 1`)

**배율 감소** — `CUpgrades.Process`:
```
this.mMultiplierTick = (this.mMultiplierTick - 1);
if (this.mMultiplierTick < 0) this.__set__mMultiplier(this.__get__mMultiplier() - 1);
```
→ 창이 끝나면 **0으로 리셋이 아니라 1단계만 감소**, 감소 후 새 창으로 다시 카운트

**창 길이** — `CUpgrades.GetTicks`:
```
index = (100 - (index - 1));
var r1 = (((Math.pow(index, 2.5) / 100000.0) * ((CMain.mFPS * 3) - 3)) + 3);
```
→ ×1에서 3초, ×10 2.4초, ×50 0.65초, ×100 0.12초 (25fps 기준). **배율이 높을수록 유지 창이 급격히 짧아져** 고배율은 자동으로 빨리 녹는다(자기 균형)

**해금** — `__set__mMultiplier(a)`: `a > 1`이면 `mMultipliers[a]` 목록의 업그레이드 중 `mActive != true`인 것을 `AddUpgrade`하고 `mActive = true` → **처음 그 배율에 도달한 순간 영구 해금**, 배율이 다시 떨어져도 유지

**해금 테이블**(`InitLists`의 `AddToMultipliers`, 발췌) — 무기 해금과 **무기별 강화**가 한 표에 섞여 있다:
| 배율 | 해금 | 배율 | 해금 |
|---|---|---|---|
| ×3 | Pistol+ Fast Fire | ×31 | Shotgun+ Wide Shot |
| ×5 | 새 무기 UZI | ×40 | 새 무기 Claymore |
| ×8 | Pistol+ Double Damage | ×48 | UZI+ Double Damage |
| ×10 | 새 무기 Shotgun | ×50 | 새 무기 Rocket |
| ×13 | UZI+ Rapid Fire | ×56 | Shotgun+ Double Damage |
| ×15 | 새 무기 Barrel | ×61 | UZI+ Infinite Range |
| ×17 | UZI+ Double Ammo | ×70 | 새 무기 Railgun |
| ×20 | 새 무기 Grenade | ×90 | UZI+ Quad Damage |
| ×30 | 새 무기 Fake walls | ×125 | Railgun+ Long Shot (마지막) |
(총 60여 항목, ×3~×125)

**강화 적용** — `ApplyUpgrade_Weapon`: 원본 스탯 스냅샷(`mOriginalStats`)에 배수를 곱해 덮어씀
- `DoubleDamage` ×2, `QuadDamage` ×4, `FatalDamage` ×10
- `RapidFire`: `fireRate = round(max(orig/4, 2))` + 자동연사, `FastFire`: 자동연사만
- `DoubleAmmo`/`QuadAmmo` 탄약 ×2/×4, `LongShot` 사거리 ×1.5, `InfinateRange` 사거리 100
- `WideShot`/`WiderShot` 산탄 폭 1/2단계, `BigBang`/`BiggerBang` 폭발 1/2단계, `ClusterExplode`
- 적용은 **버전 카운터 지연 적용**: 업그레이드마다 `IDCount++`, 각 객체는 `mUpgradeObject.mIDCount`와 비교해 달라졌을 때만 재계산 (dirty-version 패턴)

## 3. hordegrid 현재 구현과의 차이
| 영역 | 원작 (근거 §) | hordegrid 현재 | 판단 |
|---|---|---|---|
| 길찾기 | 4방향 BFS, 시간(ms) 분할, 세션 스탬프 (§2.2) | 8방향 BFS, 8프레임마다 전체 재계산 ([performance.md](../design-docs/performance.md) §1) | **이식 안 함.** 벽시계 분할은 결정론 위반, 우리 BFS는 이미 예산 내 |
| 떼 이동 | 셀 예약, 분리력 없음 (§2.3) | flow field + 분리력 | 후보 C (아래) |
| 콤보 | 배율 1씩 감소, 창이 배율에 반비례 (§2.7) | 2초 끊기면 0 리셋 | **이식** → [multiplier-and-upgrades.md](../product-specs/multiplier-and-upgrades.md) |
| 점수 | 기본 × 배율 | 기본 × (1 + 0.1·min(combo,50)) | **이식** |
| 해금 | 배율 임계, 무기 + **무기별 강화** (§2.7) | 팀 점수 임계, 무기만 | **이식** |
| 웨이브 | 인터미션 없음, 간격 1프레임씩 단축 (§2.6) | 인터미션 + 가중치 테이블 | 유지 (우리 설계가 4인 협동에 맞음) |
| 리스폰 | 동료 화면 안·4셀 이상 거리 (§2.5) | 다음 웨이브 시작 시 | 후보 B |

### 이식 후보 (계획 없음, 필요해지면 exec-plan으로)
- **A. 사망 시 픽업 드랍 확률** — 원작 `_CreatePickup` 은 적 사망 시마다 호출. 우리 4% 탄약 드랍과 비교 튜닝 시 참고
- **B. 코옵 즉시 리스폰** — 사망 사이클이 긴 4인 판에서 관전 시간이 길면 도입. 결정론 RNG로 스폰점 선택만 바꾸면 됨
- **C. Brute 전용 셀 예약 이동** — 거대 적이 떼 사이로 미끄러지는 문제가 생기면, 분리력 대신 다음 셀 예약(§2.3)이 더 싸고 결정론적

## 4. 재현 방법
[tools/as2-teardown/README.md](../../tools/as2-teardown/README.md). 같은 SHA-256 파일이면 같은 출력이 나온다.

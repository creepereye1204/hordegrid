# Performance & Algorithms
Status: Accepted · Date: 2026-09-16

## 예산이 먼저다
롤백 최악 경로 = **1 tick에 sim step 9회** (max_prediction 8 + 현재). 60Hz 한 프레임 16.6ms 중 sim에 줄 수 있는 몫 ≈ 6ms
→ **sim step p99 ≤ 0.6ms @ 적 384 + 투사체 512, wasm, 중급 노트북.** 수치·측정은 [QUALITY_SCORE.md](../QUALITY_SCORE.md#성능-예산)

## 알고리즘 선택
### 1. 적 길찾기: 멀티소스 BFS Flow Field (+ 분산 갱신)
- 64×64 타일 그리드, **모든 생존 플레이어를 소스로 한 BFS 1회** → 각 타일에 "가장 가까운 플레이어 방향" `u8`(DIR8) 저장
- 적 N마리가 각자 A*를 돌리는 O(N·E log V) 대신 **O(V) 1회 + 적당 O(1) 조회**
- 결과 `flow: [u8; 4096]`은 **State에 포함** (파생 데이터지만 롤백 정합성을 위해). 4KiB
- 갱신: 매 프레임 전체 BFS 대신 **8프레임마다 1회**, 플레이어가 타일 경계를 넘은 프레임엔 즉시. BFS 큐는 State 밖 스크래치 버퍼(매번 처음부터 계산하므로 롤백 무관)
- 대각 이동은 "양쪽 직교 타일이 모두 통행 가능할 때만" → 벽 모서리 끼임 방지
- 기각: 개별 A*(비용), JPS(동적 바리케이드 설치 시 전처리 무효화), 네비메시(타일 게임에 과함)

### 2. 광역 충돌: 균일 그리드 + Counting Sort 재구축
- 셀 크기 = 1타일. 매 step: ① 각 엔티티 셀 인덱스 계산 ② 셀별 카운트 ③ prefix sum → `cell_start` ④ 슬롯 인덱스 배치 (`cell_items`)
- **O(N), 할당 0, 결정론적 순서**(슬롯 순 삽입이라 셀 내 순서 고정). 해시맵·트리 없음
- 이 그리드는 매 step 재구축하므로 State 밖 스크래치 (롤백 무관)
- 질의: 적-적 분리(3×3 셀), 투사체-적 히트, 폭발 반경
- 기각: 쿼드트리(재구축 비용·분기 많음), sweep-and-prune(분포가 한 축에 몰리는 좀비 떼에 불리), BVH(동적 엔티티)

### 3. 좀비 떼 움직임: Flow Field + 분리력(Boids-lite)
- 속도 = flow 방향 × 속도 + Σ(인접 적 반발, 제곱거리 역비례 근사, 최대 6개만 샘플) → 겹침 없는 "떼" 연출
- 반발 계산 수를 상한으로 고정해 최악 비용 O(N)

### 4. 벽 충돌: 축 분리 AABB vs 타일
- X 이동 → 겹친 타일로 밀어냄 → Y 이동 반복. 터널링 방지: 1 step 최대 이동 < 0.5타일 (속도 상한 상수로 보장, `const _: () = assert!(...)`)

### 5. 히트스캔 무기(샷건/레일건): DDA 격자 순회 (Amanatides & Woo)
- 광선이 지나는 타일을 순서대로 방문하며 벽에서 정지, 각 타일의 그리드 셀에서 적 선분-원 판정
- 레일건 관통은 방문 순서대로 N명까지

### 6. 폭발/범위: 그리드 반경 질의 + 제곱거리 감쇠 테이블

### 7. 웨이브 스폰: 가중치 테이블 + 결정론 RNG, "플레이어 시야 밖 스폰 포인트"만 후보

## 저수준 최적화 (측정 후에만)
| 기법 | 적용 조건 |
|---|---|
| `[profile.release] lto = "fat"`, `codegen-units = 1`, `panic = "abort"`, `opt-level = 3` | 기본 적용 |
| `wasm-opt -O3` (크기보다 속도) | 기본 적용. `.wasm` gzip ≤ 400KiB 넘으면 재검토 |
| wasm **simd128** (`-C target-feature=+simd128`) | 분리력·거리 계산 루프에서 bench로 ≥15% 이득 확인 시. Safari 16.4+ 필요 → [FRONTEND.md](../FRONTEND.md) 지원 매트릭스 |
| alive 비트셋 + `trailing_zeros` 순회 | 적 생존률 < 50% 구간이 bench에서 병목일 때 |
| 분기 제거(branchless min/max, 테이블 룩업) | 핫루프 프로파일에서 분기 예측 실패 확인 시 |
| 렌더: 카메라 컬링, 아틀라스 1장 `drawImage`, 정수 좌표 스냅, 배경 타일 오프스크린 캐시 | 기본 적용 |
| 렌더: WebGL 인스턴싱 | Canvas2D에서 적 384 p95 프레임 > 8ms일 때 (TD-005) |

## 측정 도구
- `just bench`: criterion (native) — `step_empty`, `step_full_horde`, `rollback_9_steps`, `flow_field_bfs`, `grid_rebuild`
- wasm 실측: `web/bench.html` — 같은 시나리오를 브라우저에서 `performance.now()` 1000회, p50/p99 출력
- CI는 native bench만 기록(회귀 추세용, 러너 편차로 실패 판정은 안 함). 실패 판정은 **스텝당 연산 카운터**(결정론적 지표: BFS 방문 노드 수, 충돌 쌍 수)로

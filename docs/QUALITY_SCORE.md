# QUALITY SCORE

영역별 등급을 **측정값으로** 매긴다. 에이전트는 PR 설명에 영향받는 영역의 전/후 등급을 적는다.
등급: **A** 목표 달성 · **B** 허용 · **C** 개선 필요(tech-debt 등록) · **F** 머지 차단 · **–** 아직 측정 불가
최종 갱신: 2026-09-17 (첫 플레이어블)

## 요약표
| 영역 | 현재 | 목표(Phase 3) | 측정 |
|---|---|---|---|
| 결정론 | B | A | netsim 4피어(손실 10%·지연 250ms) 일치, SyncTest 통과. native↔wasm 비교 미구현(TD-010) |
| 성능: sim | A(native) | A | 최악 step p99 0.059ms native, 롤백 9스텝 ≈0.54ms. wasm 실측 대기 |
| 성능: 렌더 | – | A | web/bench.html |
| 네트워크 체감 | – | B+ | netsim + 수동 |
| 로딩 | – | A | 번들 크기 + Lighthouse |
| 테스트 | B | A | Rust 31 + TS 15, 커버리지 미측정 |
| 코드 위생 | A | A | clippy pedantic -D warnings 0, tsc strict 0 (ESLint 미도입) |
| 문서 정합성 | B | A | lint-arch 문서 링크·generated diff |
| 접근성 | – | B | 체크리스트 |

## 결정론
| 등급 | 기준 |
|---|---|
| A | `desync` 전 시나리오 일치 + `netsim --long`(10만 프레임, 손실 10%, 지터 80ms) 4피어 일치 + 실사용 desync 리포트 0 |
| B | 위 둘 통과, 실사용 리포트 원인 규명됨 |
| F | 어떤 CI 시나리오든 체크섬 불일치 |

## 성능 예산
### sim (wasm, 기준 기기: 2021년형 중급 노트북 Chrome / CI는 native 비율 환산)
| 시나리오 | A | B | F |
|---|---|---|---|
| `step_full_horde` (적 384, 탄 512, 4인) p99 | ≤ 0.6ms | ≤ 1.0ms | > 1.5ms |
| `rollback_9_steps` p99 | ≤ 5ms | ≤ 8ms | > 10ms |
| `flow_field_bfs` 64×64 | ≤ 0.15ms | ≤ 0.3ms | |
| `State` 크기 | ≤ 64KiB | ≤ 96KiB | > 128KiB |
- 결정론적 보조 지표(CI 실패 판정용): 풀호드 시나리오 1스텝당 충돌 후보 쌍 ≤ 384×8, BFS 방문 ≤ 4096

### 렌더
| 지표 | A | B |
|---|---|---|
| 적 384 + 파티클 프레임 시간 p95 (Chrome, 중급 노트북) | ≤ 6ms | ≤ 10ms |
| 중급 안드로이드 p95 | ≤ 12ms | ≤ 16ms |

## 네트워크 체감 (netsim)
| 조건 | A | B |
|---|---|---|
| RTT 60ms, 손실 1% | 롤백 평균 ≤ 2프레임, 대기 스킵 0 | ≤ 3 |
| RTT 150ms, 지터 30ms, 손실 3% | 롤백 평균 ≤ 5, 1초당 스킵 ≤ 1 | ≤ 7 / ≤ 3 |

## 로딩
| 지표 | A | B |
|---|---|---|
| wasm gzip | ≤ 300KiB | ≤ 400KiB |
| JS gzip | ≤ 60KiB | ≤ 80KiB |
| 링크→로비 p75 (4G) | ≤ 2s | ≤ 3s |

## 테스트
| 지표 | A | B |
|---|---|---|
| 라인 커버리지 sim / net / web-ts | ≥ 90 / 85 / 85 | ≥ 85 / 80 / 80 |
| `cargo mutants -p sim` 생존률 | ≤ 10% | ≤ 20% |
| product-spec AC 자동화 비율 | ≥ 80% | ≥ 60% |

## 코드 위생
A: clippy pedantic·eslint strict 경고 0, `allow` 전부 사유 주석, `TODO` 는 TD 번호 필수(`TODO(TD-007)`) · C: 사유 없는 allow 존재

## 접근성
- [ ] 키 리매핑 · [ ] 흔들림 끄기 / reduced-motion · [ ] 플레이어 색+번호 이중 표기 · [ ] 색각이상 시뮬 확인 · [ ] 로비 키보드 조작 · [ ] 음량 개별 조절
A: 전부 / B: 4개 이상

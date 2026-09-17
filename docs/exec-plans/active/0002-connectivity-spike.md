# 0002: Connectivity spike — 서버 없이 실제로 붙나
Status: Active · Owner: 안냥(실기기 측정) / agent(도구·분석) · Started: 2026-09-16

## 목표
실제 한국 모바일·가정망 조합에서 **TURN 없이 직접 연결 성공률**과 게임 채널 품질을 측정해 TURN 결정([signaling.md](../../design-docs/signaling.md#turn-결정)) C단계 여부를 정한다.

## 범위 / 비범위
범위: `tools/net-probe` 배포·측정·기록, 이후 GGRS 점 이동 스파이크 · 비범위: 게임 로직

## 단계
- [x] 1. net-probe 제작: NAT 추정, trystero 매칭, negotiated id 101 비신뢰 채널, 60pps 손실/RTT, 경로 판정, 결과 JSON/표 행 (검증: 로컬 릴레이 + Chromium 2컨텍스트 E2E 통과)
- [ ] 2. Pages에 `/hordegrid/probe/` 배포 (0001 단계 10과 병합, 그 전엔 임시 리포 가능)
- [ ] 3. 측정 20쌍 이상 — 아래 표. 필수 조합: SKT/KT/LGU+ LTE·5G 각각 ↔ 타 통신사, LTE ↔ 가정 Wi-Fi, iPhone Safari ↔ Android Chrome, 카톡 인앱
- [ ] 4. 분석: 직접 연결 실패율, 실패 조합의 NAT 판정 패턴, RTT/손실 분포 → Decision log
- [ ] 5. GGRS 점 이동 스파이크 (BridgeSocket ↔ net-probe 채널 코드 재사용), 다른 네트워크 2대 5분 무desync

## 연결 실측표
실패도 반드시 기록(경로 칸에 `실패`, 연결까지 `>30s`). 표 행은 net-probe 3번 섹션에서 복사 후 통신사·기기 칸을 직접 채운다.

| 날짜 | 나 (망 · NAT) | 상대 (망 · NAT) | 기기 | 경로 | 연결까지 | RTT p50/p95 (ms) | 왕복 손실 |
|---|---|---|---|---|---|---|---|
| 2026-09-16 | 샌드박스 Chromium (UDP 차단) | 동일 호스트 Chromium | – | 같은 네트워크 직통(host↔host) | 0.4s | 1/3 | 0% |
| 2026-09-17 | ? · 일반 NAT(good) | cellular · ? | ? | NAT 홀펀칭 성공(srflx/prflx) | 18.7s* | 47/72 | 0% |

\* 방장 쪽 기준 시간이라 친구가 링크 열기까지 걸린 시간 포함. 2026-09-17부터 도구가 양쪽 시간을 재고 작은 값을 기록.

### 중간 분석 (n=1)
- 일반 NAT ↔ 셀룰러: 직접 연결 성공, 손실 0%. **위험 조합(셀룰러↔셀룰러, 대칭↔대칭)은 아직 0건**
- RTT p95 72ms → input_delay 2(33ms) + max_prediction 8(133ms) 범위 안. 롤백 평균 2~3프레임 예상 → QUALITY_SCORE 네트워크 체감 A 조건권

## 리스크 & 확인할 가정
- NAT 판정 휴리스틱이 브라우저 후보 병합 때문에 과소 판정할 수 있음 → 실패 사례에서 교차 확인
- Safari에서 negotiated 채널 동작 미검증 → 표에 iPhone 조합 필수
- 공개 Nostr 릴레이 가용성은 날마다 다름 → 릴레이 연결 수도 메모

## Decision log
- 2026-09-16: 초대코드 숫자 8자리 + 방장 승인 채택 (signaling.md Rev 2)
- 2026-09-16: TURN은 단계 결정 — MVP 없음, 실패율 > 10%면 재결정
- 2026-09-16: trystero 0.25.4 실제 API가 README와 다름 확인 → references 갱신

## 완료 시 갱신할 문서
signaling.md(TURN 단계 결과), tech-debt TD-004, RELIABILITY.md 연결 신뢰성 표(빈도 가정 → 실측값), QUALITY_SCORE 네트워크 체감

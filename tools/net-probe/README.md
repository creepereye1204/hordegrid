# net-probe — 연결 실측 도구 (Phase 1 스파이크)

두 기기가 **서버 없이** 직접 붙는지, 붙으면 GGRS용 비신뢰 채널 품질이 어떤지 잰다. 결과는 [0002-connectivity-spike](../../docs/exec-plans/active/0002-connectivity-spike.md) 표에 기록.

## 무엇을 재나
| 항목 | 방법 |
|---|---|
| 내 NAT 유형(추정) | STUN 4곳에 한 소켓으로 질의 → 외부 매핑 포트가 1개면 일반 NAT, 여러 개면 대칭형, 0개면 UDP 차단 |
| 매칭 | trystero Nostr(8자리 숫자 코드 = roomId = password) |
| 게임 채널 | `getPeers()[id]`에 `negotiated:true, id:101, ordered:false, maxRetransmits:0` 채널 추가 → open 시간 |
| 품질 | 10초 동안 60pps 16바이트 핑/퐁 → 왕복 손실, RTT p50/p95, 역순 |
| 경로 | `getStats()` 선택된 candidate-pair → host↔host / 홀펀칭 / TURN / IPv6 여부 |

## 사용
```bash
npm i && npm run build        # dist/index.html 한 파일 (≈75KiB)
```
GitHub Pages에 `probe/index.html`로 올리고 폰 두 대로 링크 공유. 외부 스크립트 없음.
`?relay=wss://...`로 릴레이 지정 가능(자체 테스트·릴레이 차단망).

## 로컬 E2E (외부망 없이)
```bash
npm run relay & npm run serve & npm run e2e   # 최소 Nostr 릴레이 + Chromium 두 컨텍스트
```
2026-09-16 샌드박스 결과: 매칭 0.4초, 채널 open +10ms, 624/624 수신, RTT 1/3ms, 경로 host↔host.
→ **trystero 0.25.4 연결 위 negotiated id 101 비신뢰 채널이 Chromium에서 열림 확인** (Safari/Firefox, 실망은 실기기로).

## 한계
- NAT 판정은 휴리스틱. 브라우저가 동일 매핑 후보를 합치기 때문에 "매핑 1개"가 STUN 1곳만 응답한 경우일 수도 있음 (`STUN 응답` 수 표시, Firefox는 확인불가)
- 상대가 안 보이는 원인(상대 미입장 vs NAT 실패)은 trystero가 구분해주지 않음 → 30초 후 안내만
- 망 종류(`navigator.connection.type`)는 Android Chrome만 제공 → 표에는 직접 적기

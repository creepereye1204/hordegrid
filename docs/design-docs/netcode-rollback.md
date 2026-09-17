# Netcode: Rollback (GGRS)
Status: Accepted · Date: 2026-09-16

## 결정
GGRS 0.13 `P2PSession`을 풀 메시 2~4인으로 사용. 전송은 trystero가 연 `RTCPeerConnection` 위의 **비신뢰·비순서 DataChannel**. GGRS가 자체 신뢰성/순서를 처리한다.

## 왜 롤백인가 (협동 게임인데도)
| 방식 | 체감 | 호스트 이탈 | 구현 |
|---|---|---|---|
| 락스텝 | RTT/2 + 버퍼만큼 **모든 입력 지연** (80~150ms) | 게임 멈춤 | 쉬움 |
| 호스트 권위 | 호스트 0ms, 게스트 RTT 지연 + 보정 | **게임 종료** | 보정 코드 많음 |
| **롤백** | 입력 지연 2프레임 고정, 나머지는 예측·보정 | 해당 플레이어만 이탈 처리 | GGRS가 담당, 우리는 결정론만 책임 |
박스헤드류는 조준·이동 반응성이 재미의 핵심 → 롤백. 좀비 위치 튐(롤백 보정)은 협동 PvE라 허용 범위.

## GGRS 설정
```rust
pub struct HgConfig;
impl ggrs::Config for HgConfig {
    type Input = PlayerInput;                 // #[repr(transparent)] u16, Pod + Serialize/Deserialize + Default
    type InputPredictor = ggrs::PredictRepeatLast;
    type State = sim::State;                  // Copy
    type Address = PeerSlot;                  // u8 newtype, Debug+Hash+Eq
}
```
| 항목 | 값 | 근거 |
|---|---|---|
| fps | 60 | sim 고정 스텝 |
| input_delay | 2 | 33ms. 0~1은 롤백 빈도↑(좀비 384마리 재시뮬), 3 이상은 체감 |
| max_prediction_window | 8 | 133ms 초과 RTT 편차는 대기. 최악 재시뮬 9스텝 예산 → [QUALITY_SCORE.md](../QUALITY_SCORE.md) |
| desync_detection | `On { interval: 30 }` | 0.5초마다 확정 프레임 체크섬 교환 |
| disconnect_timeout | 3000ms | 모바일 핫스팟 순간 끊김 허용 |
| disconnect_notify_start | 1000ms | HUD에 "연결 불안정" 표시 |
| 플레이어 핸들 | 로비 확정 시 `selfId` 사전순 정렬 인덱스 0..n | 모든 피어가 같은 순서를 독립적으로 계산 |
API 시그니처는 [references/ggrs-llms.txt](../references/ggrs-llms.txt) 기준. 기억으로 쓰지 말 것.

## 입력 인코딩 (`PlayerInput(u16)`)
| bit | 의미 |
|---|---|
| 0–3 | 이동 방향 0=없음, 1..=8 = N,NE,E,SE,S,SW,W,NW |
| 4 | 발사(누르고 있는 동안 1) |
| 5 | 설치(바리케이드/폭발통) |
| 6 | 조준 고정(strafe) — 누르는 동안 facing 유지 |
| 7 | 다음 무기 |
| 8 | 이전 무기 |
| 9–14 | 예약 (항상 0, 수신 시 0 아니면 무시) |
| 15 | **present** — 로컬은 항상 1. GGRS가 이탈자에 넣는 `Default`(=0)를 sim이 "부재"로 인식 |
- 무기 직접 선택(1~7 키)은 JS에서 "다음/이전" 펄스 시퀀스로 변환하지 않고, Phase 3에서 bit 9–11로 확장 (TD-006)
- 에지(눌림 순간)는 sim이 이전 프레임 입력을 State에 저장해 계산 → 입력 자체는 레벨 신호만

## 소켓 어댑터 (`BridgeSocket`)
```rust
impl NonBlockingSocket<PeerSlot> for BridgeSocket {
    fn send_to(&mut self, msg: &Message, addr: &PeerSlot) { /* postcard::to_slice → outbox.push((addr, bytes)) */ }
    fn receive_all_messages(&mut self) -> Vec<(PeerSlot, Message)> { /* inbox drain, 디코드 실패는 drop + counter */ }
}
```
- 직렬화: **postcard** (serde, 컴팩트, 포맷 안정). bincode는 유지보수 중단 이슈로 기각
- 와이어 프레임: `[proto_ver: u8 = 1][postcard(Message)]`. 버전 불일치 → drop + HUD 경고 1회
- 최대 패킷 1200B 초과 시 drop + `oversize` 카운터 (GGRS 메시지는 통상 수십 바이트)
- inbox 상한 1024개 — 초과분 drop (탭 백그라운드 복귀 시 폭주 방지)

## 타임싱크 & 백그라운드 탭
- `GgrsEvent::WaitRecommendation { skip_frames }` → 그만큼 `tick()` 스킵
- 탭이 hidden이면 rAF가 멈춤 → 상대는 입력 대기/이탈 판정. `visibilitychange`에서 hidden 2초 이상이면 **스스로 세션 이탈 처리 후 재참가 불가 안내** (MVP). 복귀 동기화는 TD-002

## 이탈 처리
1. `GgrsEvent::Disconnected { addr }` → GGRS가 해당 핸들에 `Default` 입력 공급
2. sim: `present == 0`인 플레이어는 180프레임 후 `Players.state = Gone`, 드롭된 무기 없음, 웨이브 난이도는 **현재 present 수**로 재계산
3. 전원이 서로 이탈 판정이 다를 수 있음(A는 C를 끊겼다고 보고 B는 아님) → GGRS가 확정 입력 기준으로 합의하므로 sim에는 동일 입력이 들어간다. 이 가정은 `netsim`의 "부분 분할(partition)" 시나리오로 검증

## Desync 발생 시
`GgrsEvent::DesyncDetected { frame, local_checksum, remote_checksum, addr }` → 세션 즉시 중단하지 않고 HUD 경고 + 로컬 덤프(`State` 바이트 + 최근 600프레임 입력 로그)를 JSON 다운로드 버튼으로 제공. 재현은 `xtask desync --replay dump.json`. 절차: [RELIABILITY.md](../RELIABILITY.md#desync)

## 기각한 대안
- **matchbox_socket**: GGRS와 궁합 최고지만 시그널링 서버 필요 → core-beliefs #1 위반
- **trystero 기본 액션 채널로 GGRS 전송**: 신뢰·순서 채널이면 HOL 블로킹으로 손실 시 스파이크. 비신뢰 채널 별도 생성
- **SyncTestSession만으로 검증**: 로컬 결정론만 확인됨. 네트워크 경로는 `netsim` 필요 (둘 다 씀)

## 되돌릴 조건
(Chromium은 2026-09-16 net-probe로 확인됨. Safari·실망 확인 대기) Phase 1 스파이크에서 negotiated DataChannel이 trystero 연결 위에서 안정적으로 열리지 않으면 → trystero 액션 채널로 임시 전송 후 TD 등록.

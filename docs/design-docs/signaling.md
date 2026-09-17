# Signaling & Matchmaking
Status: Accepted · Date: 2026-09-16 · Rev 2 (초대코드·방장 승인·TURN 단계 결정, net-probe 검증 반영)

## 결정
- 기본: **trystero Nostr 전략** (공개 Nostr 릴레이로 암호화된 SDP 교환). 서버 운영 0
- 폴백: 릴레이가 전부 막힌 환경용 **수동 SDP 교환**(링크/QR, 2인 한정)
- NAT: 공개 STUN만. **TURN 없음(MVP)** → 대칭 NAT끼리는 연결 불가를 명시적으로 안내. 실측 후 단계적 결정 → [TURN 결정](#turn-결정)

## 룸 모델
| 항목 | 값 |
|---|---|
| appId | `hordegrid/p{PROTO_VER}` — 프로토콜이 바뀌면 다른 버전끼리 아예 매칭 안 됨 |
| 초대코드 | **숫자 8자리** (`crypto.getRandomValues` → `% 1e8`, 약 26.6bit), 표시는 `4821 0937` |
| 왜 숫자 | 링크는 카톡으로, 코드는 **말로 불러줄 때** 쓰임 → 숫자가 발음·입력 쉬움. 폰에서 `inputmode="numeric"` 키패드, 대소문자·0/O 혼동 없음 |
| URL | `https://<user>.github.io/hordegrid/#/r/48210937` — hash 라우팅이라 Pages 404 문제 없음 |
| roomId / password | 둘 다 코드 → 릴레이 위 SDP가 AES-GCM 암호화, 코드 모르는 제3자는 복호화 불가 |
| 유효기간 | 게임 채널이 trystero 연결 위에 있으므로 **게임 중에도 방을 떠나지 않는다**. 대신 방장이 시작 후 모든 knock에 `deny{started}`. 코드 재발급(`rehost`)은 미구현(TD-011) |
| 방장 | 방 만든 피어. 방장 이탈 시 로비는 해산(승계 미구현, TD-011). 게임 중 이탈은 GGRS 이탈 처리로 계속 진행 |
| 최대 인원 | 4. 5번째 knock에는 방장이 `deny{reason:'full'}` |

## 흐름
```
[만들기] 코드 생성 → joinRoom → URL 복사/QR 표시 → 대기
[참가]   URL 열기 or 코드 입력 → joinRoom
  ↓ onPeerJoin(peerId)
  getPeers()[peerId]: RTCPeerConnection
  → pc.createDataChannel('ggrs', { negotiated: true, id: 101, ordered: false, maxRetransmits: 0 })
     (양쪽이 같은 id로 생성해야 열림. 재협상 불필요)
  ↓ 입장 승인 (trystero action = 신뢰 채널)
  신규 → 방장: 'knock' {name, protoVer, buildHash}
  방장 화면: "좀비사냥꾼-42 입장 요청 [수락] [거절]"  (20초 무응답 = 거절)
  방장 → 전원: 'admit' {peerId} | 신규에게 'deny'
  - admit 전 피어: 기존 멤버의 로비 목록·ready 계산·start 대상에서 제외, 게임 채널 트래픽 무시
  - deny/무응답 → 신규는 "입장이 거절됨" 후 room.leave(), 방장 쪽은 10분간 같은 peerId knock 무시
  ↓ 로비
  'hello' {name, protoVer, buildHash}   buildHash 다르면 "새로고침 필요" 표시
  'ready' {bool}
  전원 ready → 방장이 'start' {players: sortedIds, seed, mapId} 송신
  각 피어는 자기가 계산한 값과 일치 확인 후 Game::new → GGRS 세션 시작
```
- `start` 이후 입장한 피어는 관전/난입 불가 (TD-001)
- 이름은 로컬 저장(localStorage, try/catch) 외엔 저장하지 않음

## 실패 모드와 UX
| 상황 | 감지 | 사용자에게 |
|---|---|---|
| 릴레이 전부 연결 실패 | joinRoom 10초 내 relay 연결 0 | "매칭 서버에 닿지 못함 → 수동 연결 시도" 버튼 |
| 직접 연결 실패 (대칭 NAT/방화벽) | 릴레이 연결 ≥1인데 30초간 onPeerJoin 없음 (trystero가 ICE 실패를 알려주지 않음) | "친구가 아직 안 들어왔거나 네트워크가 직접 연결을 막음 → 한 명 Wi-Fi 전환" |
| `ggrs` 채널만 안 열림 | onPeerJoin 후 8초 내 open 없음 | 버그로 간주, 콘솔 리포트 |
| buildHash 불일치 | hello | "상대와 버전이 다름. 둘 다 새로고침" |
| 로비 중 이탈 | onPeerLeave | 목록에서 제거, ready 초기화 |

## 수동 SDP 폴백 (2인)
1. 방장: `RTCPeerConnection` 직접 생성(trystero 미사용) → 채널 2개(`lobby` 신뢰, `ggrs` 비신뢰 negotiated) → offer → **ICE gathering complete까지 대기**(non-trickle)
2. SDP를 `CompressionStream('deflate-raw')` → base64url → `#/m/<offer>` 링크 또는 QR
3. 참가자: 링크 열기 → answer 생성 → 같은 방식으로 코드 표시 → 방장이 붙여넣기
4. 이후 로비 프로토콜은 `transport.ts` 인터페이스로 동일하게 사용
- 한계: 릴레이 차단 문제만 해결. NAT 문제는 해결 못 함 (문서·UI에 명시)

## 추상화
```ts
interface Transport {
  peers(): PeerInfo[];
  onPeerJoin(cb): void; onPeerLeave(cb): void;
  sendLobby(peer: PeerId | 'all', msg: LobbyMsg): void; onLobby(cb): void;
  sendGame(slot: number, bytes: Uint8Array): void;   // 비신뢰 채널
  onGame(cb: (slot: number, bytes: Uint8Array) => void): void;
}
```
`TrysteroTransport`, `ManualTransport`, `LoopbackTransport`(테스트·싱글플레이) 3구현.

## TURN 결정
배경: P2P 통화의 약 17.7%가 TURN 중계를 사용했다는 실측(appear.in, Philipp Hancke 분석)이 있음. 오래된 해외 데이터라 한국 모바일(CGNAT) 환경은 직접 측정한다.

| 단계 | 조건 | 조치 |
|---|---|---|
| **A. MVP (현재)** | – | TURN 없음. 실패 시 "한 명 Wi-Fi 전환" 안내. `turnConfig` 주입 지점만 `transport.ts`에 열어둠 |
| B. 실측 | Phase 1, [0002](../exec-plans/active/0002-connectivity-spike.md) | `tools/net-probe`로 LTE↔LTE / LTE↔Wi-Fi / 통신사 조합 ≥ 20쌍 측정 |
| C. 재결정 | 실측 **직접 연결 실패율 > 10%** | 옵션 비교 후 core-beliefs #1 개정 PR: ① 사용자 입력 TURN(설정 화면) ② Cloudflare Worker 1개로 **단기 TURN 자격증명 발급만**("게임 서버 0, 연결 보조 허용") |

- 정적 페이지에 TURN 비밀 자격증명을 박는 방식은 **금지**: 누구나 추출해 우리 비용으로 중계 가능
- Cloudflare Realtime TURN은 SFU와 함께 쓸 때만 무료, 단독은 $0.05/GB (2026-09 문서). 게임 트래픽은 작아 비용보다 **자격증명 발급 서버 필요성**이 쟁점

## 검증된 사실 (tools/net-probe, 2026-09-16)
- trystero 0.25.4 내부 채널은 `createDataChannel("data")`(비-negotiated) → 우리 `negotiated id 101` 채널과 충돌 없음, Chromium 두 컨텍스트에서 open +10ms, 60pps 10초 무손실
- `onPeerJoin`은 trystero 채널이 열린 뒤에만 호출 → **ICE 실패는 이벤트로 안 옴**. "상대가 안 들어옴"과 "NAT 실패"를 구분할 수 없어 시간 기반 안내만 가능
- trystero API는 README 예시와 다름(액션이 객체, 콜백은 프로퍼티 대입) → [references/trystero-llms.txt](../references/trystero-llms.txt)

## 기각한 대안
- **BitTorrent 트래커 전략**: 공개 WS 트래커 수가 적고 불안정 이력. Nostr는 릴레이 중복이 많음 (전략 교체는 import 한 줄이므로 설정으로 노출만)
- **자체 Cloudflare Worker 시그널링**: 무료지만 "운영 서버 0" 위반
- **공개 무료 TURN**: 신뢰성·약관 불확실, 트래픽이 제3자 경유
- **영숫자 6자 코드**: 엔트로피는 약간 높지만 말로 전달·모바일 입력이 불편 → 숫자 8자리 + 방장 승인으로 대체

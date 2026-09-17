# FRONTEND

## 스택
Vite + TypeScript(strict) + 바닐라 DOM. **프레임워크 없음** — UI는 로비·HUD뿐이고, 게임 루프에 가상 DOM diff는 방해.
패키지: `trystero`(버전은 references 확인), `qrcode`(로비 QR). 그 외 추가는 이 문서 PR.

## 모듈 경계 (`xtask lint-arch`가 import 규칙 검사)
```
core/   ← 어디서나 import 가능 (wasm 로드, loop, view-layout 생성물)
input/  → core
net/    → core            (render/ui import 금지)
render/ → core            (net import 금지, wasm 상태 쓰기 금지)
ui/     → core, net(타입만)
main.ts → 전부 조립 (의존성 주입 지점은 여기 하나)
```

## 게임 루프 (`core/loop.ts`)
```ts
const STEP = 1000 / 60;
let acc = 0, last = performance.now();
function frame(now: number) {
  acc += Math.min(now - last, 250); last = now;       // 250ms 클램프: 탭 복귀 폭주 방지
  while (acc >= STEP) { tickOnce(); acc -= STEP; }    // net 수신 → game.tick → outbox 송신 → 이벤트
  render(acc / STEP);                                  // 보간 alpha
  requestAnimationFrame(frame);
}
```
- 보간: RenderView 이전/현재 두 벌 보관, 롤백으로 위치가 1타일 이상 점프하면 보간 생략(순간이동이 늘어지는 것 방지)

## 렌더 (`render/`)
- Canvas2D, 아틀라스 1장, 정수 좌표 스냅, 카메라 밖 컬링
- 정적 바닥/벽은 오프스크린 캔버스에 한 번 그려 캐시
- 카메라: 로컬 플레이어 중심, 다른 플레이어가 화면 밖이면 가장자리 화살표
- 렌더 전용 파티클은 JS 소유 (sim 이벤트로 생성), 롤백 무관

## 입력
| 장치 | 매핑 |
|---|---|
| 키보드 | WASD/방향키 이동, Space/J 발사, Q/E 무기, F/K 설치, Shift 조준고정 |
| 게임패드 | 좌스틱 8방향 양자화(데드존 0.3), RT 발사, LB/RB 무기, A 설치, LT 조준고정 |
| 터치 | 좌측 플로팅 스틱 이동, 우측 발사(누르는 동안 조준고정 자동)/설치/무기 — 상세 [mobile-play.md](product-specs/mobile-play.md) |
- 인코딩은 `input/encode.ts` 하나 → 비트 레이아웃은 [generated/net-protocol.md](generated/net-protocol.md)와 gen-docs 생성 상수로 일치 보장
- 포커스 잃으면(`blur`) 입력 0으로 (계속 달리는 버그 방지). present 비트는 유지

## 모바일
- 모바일은 1급 플랫폼. 요구사항 원본은 [product-specs/mobile-play.md](product-specs/mobile-play.md)
- `ui/touch-controls.ts`는 DOM이 아니라 **캔버스 위 오버레이 캔버스**에 그림(레이아웃 리플로 없음), 입력은 Pointer Events만 사용(`touchstart` 직접 사용 금지)
- 해상도: `devicePixelRatio` 최대 2로 클램프 (3x 기기에서 채움 비용 절감)
- 렌더 강등 스위치 `render/quality.ts`: `full` ↔ `reduced`(30fps, 파티클 50%, 그림자 끔)
- 테스트: Playwright `devices['iPhone 13']`, `devices['Pixel 7']` 에뮬 E2E + Phase 3에 실기기 체크리스트

## 브라우저 지원
| 브라우저 | 최소 | 이유 |
|---|---|---|
| Chrome/Edge | 최신 2개 | WebRTC DC, wasm |
| Firefox | 최신 2개 | |
| Safari (macOS/iOS) | 16.4+ | wasm SIMD, `CompressionStream`, Screen Wake Lock(iOS 16.4+) |
| Android Chrome | 최신 2개 | |
| 인앱 브라우저(카톡 등) | 동작 보장 안 함 | 외부 브라우저 안내 |
- 미지원 감지(WebAssembly, RTCPeerConnection, CompressionStream) → 안내 화면

## 로딩 성능
- `.wasm` gzip ≤ 400KiB, JS gzip ≤ 80KiB (trystero 포함), 폰트·아틀라스 합 ≤ 300KiB
- `WebAssembly.instantiateStreaming` (Pages가 `application/wasm` MIME으로 서빙)
- Vite `base: '/hordegrid/'` — 리포 이름 바뀌면 여기와 [signaling.md](design-docs/signaling.md) URL 같이 변경

## 저장소
`localStorage`는 닉네임·키 설정·음량만, 전부 try/catch. 게임 상태 저장 없음.

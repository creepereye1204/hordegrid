# Spec: Mobile Play
Status: Ready · Phase 2(기본 조작) – 3(완성도)

## 목표
폰 브라우저에서 링크를 열어 **PC 친구와 섞여서** 같은 판을 무리 없이 플레이. 앱 설치 없음.
모바일은 "지원"이 아니라 **1급 플랫폼** — 링크를 받는 곳은 대부분 메신저(카톡)이고 곧 폰이다.

## 대상 기기
| 구분 | 기준 |
|---|---|
| iOS | Safari 16.4+ (iPhone 11급 이상) — iOS의 모든 브라우저는 WebKit이라 사실상 Safari 기준 |
| Android | Chrome 최신 2개 (2021년 중급 기기, 예: Galaxy A52급) |
| 인앱 브라우저 | 카카오톡·인스타 인앱 브라우저 감지 시 **"외부 브라우저로 열기" 안내 배너** (WebRTC/wasm 제약·백그라운드 정책 불확실) — 차단은 하지 않음 |

## 화면
- **가로 모드 전용 플레이**. 세로면 로비는 정상, 게임 시작 시 "기기를 가로로 돌려주세요" 오버레이(시뮬은 계속 돌아감 — 멈추면 상대가 대기함)
- 가시 영역: PC와 **동일한 월드 타일 수**(32×24 기준 비율 유지, 레터박스) → 모바일이 시야에서 불리하지 않게
- `env(safe-area-inset-*)` 반영: 노치·홈 인디케이터 영역에 버튼 금지
- `viewport`: `width=device-width, initial-scale=1, viewport-fit=cover, user-scalable=no` + `touch-action: none`(캔버스) — 더블탭 줌·스크롤·당겨서 새로고침 방지
- 전체화면: Android는 [시작] 탭 시 `requestFullscreen()` + `screen.orientation.lock('landscape')` 시도(실패 무시). iPhone Safari는 요소 전체화면 미지원 → **홈 화면에 추가(PWA standalone)** 안내를 로비에 1회 표시

## 조작 (터치)
```
┌──────────────────────────────────────────────┐
│ 점수/웨이브                        콤보       │
│                                              │
│   (  ◯  )                        [무기◀▶]    │
│   이동 스틱                      [설치]      │
│   (화면 좌측 절반 어디든                [ 발사 ] │
│    터치한 곳이 스틱 중심)                     │
└──────────────────────────────────────────────┘
```
- **플로팅 스틱**: 좌측 절반 터치 지점이 중심, 반경 56dp, 데드존 20%, 8방향 양자화(각 45° 섹터, 경계 ±5° 히스테리시스로 떨림 방지)
- **발사 버튼 누르는 동안 조준고정(bit6) 자동 ON** → 뒷걸음 사격이 모바일에서도 가능. 발사 시작 순간 방향 = 현재 스틱 방향(없으면 기존 facing)
- 버튼 최소 48×48dp, 발사 버튼 72dp. 한손 엄지 도달 범위 기준 배치
- 멀티터치: `pointerId`별 추적, 스틱·발사·무기 동시 입력 가능
- 햅틱: Android `navigator.vibrate(15)` 피격·다운 시 (설정 끄기 가능). iOS는 미지원 → 생략
- 게임패드 블루투스 연결 시 터치 UI 자동 숨김

## 수명주기 & 네트워크 (모바일 특유)
| 상황 | 동작 |
|---|---|
| 앱 전환/화면 꺼짐 (`visibilitychange` hidden) | 2초 유예 후 자진 이탈 ([netcode-rollback.md](../design-docs/netcode-rollback.md)). 게임 중엔 **Screen Wake Lock** 요청해 화면 꺼짐 방지 |
| 전화 수신 | 위와 동일 |
| Wi-Fi ↔ LTE 전환 | ICE 경로 끊김 → 3초 내 복구 안 되면 이탈. ICE restart는 TD-008 |
| 셀룰러 CGNAT | 대칭 NAT 확률↑ → 연결 실패 안내에 "Wi-Fi로 전환" 1순위 표시 |
| 발열·스로틀 | sim은 60Hz 유지, **렌더만** 30fps로 강등(프레임 p95 > 20ms가 5초 지속 시), 파티클 절반 |

## PWA
- `manifest.webmanifest`(가로 orientation, standalone, 아이콘) — iOS 홈 화면 실행 시 주소창 제거
- Service Worker: 에셋·wasm은 cache-first(해시 파일명), **`index.html`은 network-first** → buildHash 혼재 최소화 ([RELIABILITY.md](../RELIABILITY.md))
- 오프라인 시 "혼자 연습"만 활성

## 수용 기준
- [ ] AC1 iPhone(Safari 16.4+)·Android Chrome 실기기에서 링크 → 로비 → PC와 2인 5분 플레이 무desync
- [ ] AC2 터치 조작만으로 웨이브 5 도달 가능 (내부 테스터 3명 중 2명)
- [ ] AC3 게임 중 핀치 줌·스크롤·당겨서 새로고침·텍스트 선택·롱프레스 메뉴 **발생 0**
- [ ] AC4 중급 Android 적 384 상황 렌더 p95 ≤ 16ms (QUALITY_SCORE 렌더 B), 강등 모드 동작 확인
- [ ] AC5 앱 전환 후 복귀 시 크래시 없이 "연결이 끊겼습니다 → 로비로" 안내
- [ ] AC6 세로 ↔ 가로 회전 시 캔버스·버튼 레이아웃 재계산, safe-area 침범 없음 (Playwright 모바일 에뮬 스크린샷 비교 + 실기기 1회)
- [ ] AC7 카카오톡 인앱 브라우저에서 외부 브라우저 안내 표시

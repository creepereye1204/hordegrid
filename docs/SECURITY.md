# SECURITY

## 위협 모델 (서버 없는 협동 게임)
자산: 사용자 IP 주소, 브라우저 안정성, 게임 공정성(협동이라 낮음), 저장소·배포 파이프라인.

| 위협 | 영향 | 대응 |
|---|---|---|
| **IP 노출**: WebRTC는 피어 간 IP 교환이 본질 | 방 참가자에게 공인 IP 노출 | 로비·README에 명시. 초대코드를 모르는 사람은 SDP 복호화 불가(trystero password). 공개 방 목록 없음(비목표). **주의: WebRTC 연결은 방장 승인 전에 이미 맺어지므로 승인은 IP 노출을 막지 못함** — 코드 유출 = IP 노출로 안내 |
| 초대코드 추측 | 모르는 사람 난입 | 8자리 숫자(1억) — 공개 릴레이라 봇 대입을 막을 수단은 없다고 가정. **방장 승인 전엔 아무 것도 못 함**, 로비 열린 동안만 유효, 재발급·강퇴 가능, 게임 시작 후 방 떠남 |
| **악성 패킷**: 변조·과대·폭주 | wasm panic → 크래시, 메모리 폭주 | 크기 상한 1200B, inbox 상한, postcard 디코드 실패 drop, `net`/`web` unwrap/panic 금지 lint, 코덱 fuzz nightly |
| 로비 메시지 스푸핑/XSS | 닉네임에 HTML | 닉네임 `textContent`만 사용, `innerHTML` 금지(ESLint `no-restricted-properties`), 길이 16자·제어문자 제거 |
| 치팅(메모리 수정·입력 매크로) | 협동이라 피해 제한적 | **비목표.** 결정론 롤백 특성상 상태 조작 = 즉시 desync 감지로 드러남. 그 이상 방어 안 함 |
| 공급망: npm/crates 악성 패키지 | 빌드 산출물 오염 | lockfile 커밋, `cargo deny`, `npm ci`, Dependabot, 의존성 추가는 문서 PR 필요 |
| CI 토큰 탈취 | 악성 배포 | 워크플로 최소 권한(`contents: read` 기본, pages job만 write), 서드파티 action은 **커밋 SHA 고정**, fork PR에 secrets 없음 |
| 제3자 릴레이가 메타데이터 수집 | 누가 언제 어떤 appId 방에 들어갔는지 | SDP 내용은 암호화, 메타데이터는 한계로 명시 |

## 규칙
- CSP: Pages는 헤더 불가 → `<meta http-equiv="Content-Security-Policy">`로 `default-src 'self'; connect-src 'self' wss: stun:; script-src 'self' 'wasm-unsafe-eval'; img-src 'self' data:; style-src 'self' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com`
  - Nostr 릴레이는 `wss:` 전체 허용 필요(릴레이 목록 변동) — 한계로 기록
- `eval`/`new Function` 금지. wasm은 `'wasm-unsafe-eval'`만
- 사용자 입력을 URL hash/query에서 읽을 때(초대코드 `^\d{8}$`, 수동 SDP, 개발용 `?relay=wss://` 정규식) 검증 후 사용, SDP는 크기 상한 16KiB
- TURN 자격증명을 정적 번들에 포함 금지 ([signaling.md](design-docs/signaling.md#turn-결정))

## IP (지식재산)
- "박스헤드(Boxhead)" 명칭·로고·스프라이트·사운드·맵 레이아웃을 **사용하지 않는다**. 장르적 아이디어(탑다운, 떼, 콤보 해금)만 차용
- README에 "inspired by classic flash top-down zombie shooters" 수준 표현만
- 에셋은 자체 제작 또는 CC0/OFL, `CREDITS.md`에 출처·라이선스
- 코드 라이선스: MIT OR Apache-2.0 (Rust 생태계 관례)

## 취약점 제보
`SECURITY` 탭(GitHub private vulnerability reporting) 활성화.

# PLANS

## 로드맵
| Phase | 목표 | 완료 조건 (Exit criteria) | 계획 |
|---|---|---|---|
| **0. Harness** | 문서·툴체인·CI 뼈대 | `just check` 녹색, Pages에 빈 캔버스 배포 | [0001-bootstrap](exec-plans/active/0001-bootstrap.md) |
| **1. Spike: 연결** | 두 브라우저가 trystero로 붙고 GGRS 세션이 **점 하나 움직이기**로 동기화 | 실제 다른 네트워크 2대에서 5분 무desync, netsim 4피어 통과, **연결 실측표 20쌍** | [0002-connectivity-spike](exec-plans/active/0002-connectivity-spike.md) (net-probe는 Phase 0과 병행 가능) |
| **2. Playable MVP** | 1맵, 무기 1~4, 적 Walker/Runner, 웨이브, 다운/부활, **터치 조작** | core-gameplay AC 전부, onboarding AC1~6, mobile-play AC1·AC3 | |
| **3. Content & Polish** | 무기 7종, 적 4종, 사운드, 게임패드, PWA·모바일 완성도, 결과 화면 | weapons-and-enemies Draft 해제, QUALITY_SCORE 전 영역 B 이상 | |
| **4. Portfolio** | README 데모 GIF, 아키텍처 글, desync·롤백 시각화 디버그 오버레이 | 외부인 3명 링크만으로 플레이 성공 | |

## exec-plan 규칙
- 위치: `exec-plans/active/NNNN-slug.md`, 끝나면 `completed/`로 `git mv`
- 번호는 전역 증가, 재사용 금지
- 한 계획 = 한 PR~수 PR. 1주 넘으면 쪼갠다
- 에이전트는 작업 시작 시 `active/`에서 `Owner`가 비었거나 자기인 계획만 집는다

## 템플릿
```markdown
# NNNN: 제목
Status: Active | Blocked | Done · Owner: <agent/human> · Started: YYYY-MM-DD

## 목표
한 문장.

## 범위 / 비범위

## 단계
- [ ] 1. ... (검증: 어떤 명령/테스트로 확인하는지)

## 리스크 & 확인할 가정

## Decision log
- YYYY-MM-DD: 결정 — 이유

## 완료 시 갱신할 문서
```

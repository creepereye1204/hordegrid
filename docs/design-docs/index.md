# Design Docs Index

설계 결정의 **이유**를 담는 곳. 코드는 "무엇", 여기는 "왜".
새 문서는 이 표에 한 줄 추가. 상태: `Draft` → `Accepted` → (`Superseded by X`)

| 문서 | 상태 | 요약 | 검증 방법 |
|---|---|---|---|
| [core-beliefs.md](core-beliefs.md) | Accepted | 타협 불가 원칙 7개 + 언어 선택 근거 | 리뷰, lint-arch |
| [determinism.md](determinism.md) | Accepted | 고정소수점, overflow 정책, Pod 상태 레이아웃, 체크섬 | `just check`(clippy), `just desync` |
| [netcode-rollback.md](netcode-rollback.md) | Accepted | GGRS 설정값, 입력 인코딩, 소켓 어댑터, 타임싱크, 이탈·desync 처리 | `just netsim` |
| [signaling.md](signaling.md) | Accepted | trystero Nostr 매칭, 비신뢰 채널 추가, 수동 SDP 폴백 | 수동 E2E 체크리스트 |
| [wasm-boundary.md](wasm-boundary.md) | Accepted | `Game` API, 메모리 모델, RenderView | `just check`(생성 .d.ts diff) |
| [patterns.md](patterns.md) | Accepted | 채택/금지 디자인 패턴과 적용 위치 | 리뷰 체크리스트 |
| [performance.md](performance.md) | Accepted | 예산, Flow Field BFS·그리드 counting sort·DDA 등 알고리즘 선택 | `just bench`, 연산 카운터 테스트 |
| [code-quality.md](code-quality.md) | Accepted | CI 강제 품질 게이트(clippy pedantic, proptest, insta, mutants, fuzz) | CI |

## 작성 규칙
- 맨 위에 `Status / Date / Owner` 3줄
- **결정 → 근거 → 기각한 대안 → 되돌릴 조건** 순서
- 수치 주장(지연 ms, 엔티티 수)은 측정 방법과 함께

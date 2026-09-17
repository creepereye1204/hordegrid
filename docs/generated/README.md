# generated/
`just gen-docs`(xtask gen_docs)가 **코드에서 추출해 덮어쓰는** 문서. 손으로 고치지 말고 원본 코드를 고친다.
CI의 `lint-arch`가 재생성 후 diff가 있으면 실패시킨다.

| 파일 | 원본 | 내용 |
|---|---|---|
| [sim-state-schema.md](sim-state-schema.md) | `crates/sim/src/state.rs` | `State` 필드 오프셋·크기 (desync 필드 diff에 사용) |
| [net-protocol.md](net-protocol.md) | `crates/net/src/{codec,config}.rs`, `web/src/net/lobby-msg.ts` | 입력 비트, 와이어 프레임, 로비 메시지 |
| [render-view.md](render-view.md) | `crates/web/src/view.rs` | RenderView 헤더·레코드 레이아웃 (+ `web/src/core/view-layout.ts` 생성) |

> 원래 템플릿의 `db-schema.md` 자리. 이 프로젝트엔 DB가 없고, 그 역할(“코드와 반드시 일치해야 하는 스키마”)을 위 3개가 맡는다.
> 현재 내용은 **Phase 0 시드**(설계 문서에서 옮김). gen-docs 구현(0001 단계 7) 후 첫 실행에서 덮어쓴다.

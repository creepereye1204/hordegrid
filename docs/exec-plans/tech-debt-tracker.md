# Tech Debt Tracker

알고 선택한 부채만 적는다(모르는 버그는 이슈로). 코드의 `TODO`는 반드시 `TODO(TD-NNN)` 형식.
심각도: **H** 사용자 체감/포폴 데모 위협 · **M** 품질 등급 하락 · **L** 개선

| ID | 심각도 | 내용 | 왜 지금 안 하나 | 갚는 조건/시점 | 상태 |
|---|---|---|---|---|---|
| TD-001 | M | 게임 시작 후 난입/관전 불가 | GGRS 세션 중 플레이어 추가 불가, State 전송 필요 | Phase 4, 관전은 GGRS Spectator로 먼저 | Open |
| TD-002 | H | 탭 백그라운드/앱 전환 후 복귀 동기화 없음(자진 이탈) | 복귀 시 상태 따라잡기 = 난입과 같은 문제 | TD-001과 함께 | Open |
| TD-003 | L | `sim`이 `#![no_std]` 아님 | 초기 개발 편의 | Phase 2 종료 전 | Open |
| TD-004 | H | TURN 없음 → 대칭 NAT/모바일 CGNAT 조합 연결 불가 | core-beliefs #1 (서버 0) | [0002](active/0002-connectivity-spike.md) 실측 직접연결 실패율 > 10% → signaling.md TURN 결정 C단계 | Open (측정 대기) |
| TD-005 | L | Canvas2D 렌더 (WebGL 아님) | MVP 엔티티 수에서 불필요 | 렌더 p95 B 미달 시 | Open |
| TD-006 | L | 무기 직접 선택 비트 없음 (다음/이전만) | 입력 비트 절약, 7종 전엔 불필요 | Phase 3 무기 7종 시 bit 9–11 | Open |
| TD-007 | M | `ggrs` wasm feature가 unmaintained `instant` 의존 | 상위 크레이트 선택 | ggrs 업스트림 교체 시 or 패치 PR 기여 | Open |
| TD-008 | M | 네트워크 전환(Wi-Fi↔LTE) 시 ICE restart 없음 | trystero 내부 PC 관리와 충돌 가능성 조사 필요 | Phase 3 모바일 완성도 | Open |
| TD-009 | L | ICE 실패를 trystero가 이벤트로 주지 않아 "미입장 vs NAT 실패" 구분 불가 | 라이브러리 한계 | trystero에 기여 or `rtcPolyfill`로 PC 감싸 `iceconnectionstatechange` 관찰 | Open |
| TD-010 | M | `just desync`(native↔wasm 체크섬 비교)·`gen-docs` 미구현 | 첫 플레이어블 우선 | Phase 1 종료 전 | Open |
| TD-011 | L | 방장 승계·코드 재발급(`rehost`)·QR 미구현 | MVP 범위 밖 | Phase 3 | Open |
| TD-012 | L | 샷건 DDA 히트스캔 대신 투사체, 탄약·설치물(바리케이드/폭발통)·Spitter/Brute 미구현 | 콘텐츠는 마지막 (PRODUCT_SENSE) | Phase 3 | Open |
| TD-013 | L | RenderView를 매 스텝 `slice()` 복사(≈3KiB/스텝) — 문서상 zero-copy와 다름 | 보간에 이전 스텝 필요, 비용 무시 가능 | 렌더 p95 B 미달 시 | Open |
| TD-014 | M | PWA(manifest/SW)·렌더 품질 강등·가로 회전 강제 전체화면 미구현 | 모바일 기본 조작 우선 | Phase 3 | Open |
| TD-015 | L | 총소리가 `render/audio.ts` 오실레이터 합성음뿐, 샘플 기반 리얼 사운드 아님 | 출시 임박, 새 에셋/오디오 파이프라인은 안정화 이후로 | 출시 후, 무기별 사운드 필요해지면 | Open |
| TD-016 | L | 무기 3종(권총/기관단총/샷건)뿐, 종류 확장 미구현 | 출시 임박, 밸런싱·이벤트·UI 텍스트까지 같이 손대야 해서 범위 큼 | 출시 후, [product-specs/weapons-and-enemies.md](../product-specs/weapons-and-enemies.md) 기준 7종까지 | Open |
| TD-017 | M | 기지 건설(플레이어가 직접 짓는 거점/바리케이드) 시스템 전체 미구현 | 출시 임박, 완전히 새로운 게임플레이 축이라 sim 상태·렌더·UI·밸런싱 전부 새로 설계해야 함 | 출시 후, 별도 exec-plan(0004~)으로 설계부터 | Open |

# Core Beliefs
Status: Accepted · Date: 2026-09-16

위반하는 변경은 이유가 아무리 좋아도 먼저 이 문서를 고치는 PR로 합의한다.

1. **운영 서버 0대.** 게임 서버·시그널링 서버·DB를 만들지 않는다. 공개 인프라(Nostr 릴레이, 공개 STUN)와 GitHub Pages만 쓴다. 이게 무너지면 프로젝트 정체성이 무너진다.
2. **시뮬레이션은 순수 함수다.** `State_{n+1} = step(State_n, Inputs_n)`. 같은 입력이면 모든 피어·모든 빌드(native/wasm)에서 비트 단위로 같은 상태. 롤백의 전제.
3. **상태는 평평하다.** `sim::State`는 `#[repr(C)]` + `bytemuck::Pod`(= 패딩 없음, 포인터 없음, 고정 크기 배열). 스냅샷은 memcpy급 clone, 체크섬은 바이트 해시.
4. **네트워크를 믿지 않는다.** 모든 패킷은 손실·중복·역순·변조될 수 있다. 파싱 실패는 드롭이지 panic이 아니다. `crates/web`에서 panic은 곧 게임 종료이므로 `net`/`web`에 `unwrap()` 금지.
5. **표현 계층은 읽기 전용이다.** JS는 sim 상태를 읽어 그리기만 한다. 이펙트·사운드는 sim이 내보낸 이벤트로만, 롤백 중복은 JS에서 `(frame, seq)`로 제거.
6. **측정하지 않은 건 모른다.** 성능·지연 주장은 `bench`/`netsim` 수치로. [QUALITY_SCORE.md](../QUALITY_SCORE.md) 예산 초과 시 기능보다 최적화가 먼저.
7. **문서가 스펙이다.** 에이전트는 문서만 보고 구현할 수 있어야 한다. 모호하면 추측하지 말고 문서를 먼저 채운다.

## 왜 Rust인가 (2026-09-16 결정, C++ 검토 후 복귀)
- **GGRS**(GGPO의 Rust 재구현, 0.13.0 / 2026-06 릴리스)가 wasm에서 동작하고 소켓이 trait으로 추상화돼 있어 trystero 위에 얹기 쉽다 → 롤백 코어를 직접 쓰는 리스크 제거
- `bytemuck::Pod` derive로 "패딩 없는 POD 상태"를 **컴파일러가 강제**, clippy `disallowed_types`/`float_arithmetic`으로 결정론 규칙을 lint로 강제 → 하네스(기계적 강제)와 궁합이 좋음
- wasm-bindgen으로 JS 경계가 C++/Emscripten보다 가벼움
- 대가: 국내 채용 공고에서 Rust 비중이 낮음. 포폴에서 어필할 것은 언어가 아니라 **결정론 시뮬·롤백 넷코드·서버리스 P2P·하네스 문서 체계**라는 설계 역량으로 잡는다
- 기각: C++20 + Emscripten(롤백 1~2천 줄 직접 구현·UB 관리 부담), 락스텝(입력 지연)
- 되돌릴 조건: GGRS가 trystero 위에서 Phase 1 스파이크를 통과하지 못하면 락스텝 자체 구현으로 강등 검토

# hordegrid

서버 없이 브라우저끼리 WebRTC로 붙는 2~4인 협동 탑다운 좀비 서바이벌.
**Rust → WASM 결정론 시뮬레이션 · GGRS 롤백 넷코드 · trystero(Nostr) 매칭 · GitHub Pages 정적 배포.**

- 링크/8자리 코드로 초대 → 방장 승인 → 준비 → 시작
- PC 키보드/게임패드, 모바일 터치(플로팅 스틱) 동시 지원
- 운영 서버 0대

## 구조
에이전트·사람 모두 [AGENTS.md](AGENTS.md)부터. 아키텍처는 [ARCHITECTURE.md](ARCHITECTURE.md).

## 로컬 실행
```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version <Cargo.lock의 wasm-bindgen 버전> --locked
just dev            # wasm(debug) 빌드 + vite dev
```
Rust 없이 UI만: `cd web && npm i && npx vite` (TS mock sim으로 동작, 네트워크 게임 불가)

## 배포
main에 push → GitHub Actions가 wasm 빌드 → Pages 배포. Settings > Pages > Source를 **GitHub Actions**로 설정.

## 검증
`just check` (fmt·clippy pedantic·테스트·tsc·vitest·아키텍처 lint), `just netsim` (4피어·손실 10%·지연 250ms 롤백 일치)

Inspired by classic flash top-down zombie shooters. 모든 이름·그래픽은 자체 제작. MIT OR Apache-2.0.

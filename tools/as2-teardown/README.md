# as2-teardown

Flash 8 이하(AS1/AS2) SWF용 최소 디스어셈블러 + **평탄화 난독화 우회 디컴파일러**.
[boxhead-2play-teardown.md](../../docs/references/boxhead-2play-teardown.md)의 근거를 재현하려고 만든 도구. 게임 빌드와 무관(파이썬 표준 라이브러리만).

## 사용
분석할 SWF는 **레포 밖** 빈 디렉터리에 두고, 출력도 레포 밖으로. 디컴파일 결과(원작 코드)는 커밋하지 않는다 → [SECURITY.md](../../docs/SECURITY.md#ip-지식재산)
```bash
python3 -I tools/as2-teardown/as2dis.py /tmp/swf/boxhead.swf /tmp/swf/asm      # 태그별 원시 디스어셈블리
python3 -I tools/as2-teardown/deob.py   /tmp/swf/boxhead.swf /tmp/swf/as       # 클래스별 복원 의사코드
python3 -I tools/as2-teardown/deob.py   /tmp/swf/boxhead.swf /tmp/swf/as 854   # DoInitAction 스프라이트 ID 지정
```
출력 파일명은 `<스프라이트ID>_<클래스>.as`(파일 안 `_CLASSID_` 기준으로 직접 이름을 바꿔 쓰면 편함).

## 동작 원리
- `as2dis.py`: SWF 헤더(CWS/FWS) → 태그 순회(DefineSprite 재귀, DoAction/DoInitAction/PlaceObject2 clip actions) → 액션 레코드 디코드
- `deob.py`: 바이트코드를 **구체값/기호식 혼합 스택**으로 에뮬레이트
  - 디스패처 상태변수·불투명 조건·정크 연산은 구체값으로 접혀 사라지고, 런타임 데이터에 의존하는 것만 식으로 남아 문장으로 출력
  - 코드 공간 = **SWF 전체 버퍼**. 범위 밖 점프로 다른 태그 바이트에 숨긴 코드를 따라간다
  - 기호 조건 분기만 fork, `(pc, 스택, 지역 구체값, 레지스터 구체값)` 방문 1회 → 라벨/goto 형태 출력 (구조화된 if/while 복원은 안 함)
  - 상수만 반환하는 스텁 함수(`function(){return 1989;}`)는 시드로 인식해 호출을 상수로 치환

## 한계
- 출력은 라벨+goto 의사코드. 루프는 구체 카운터면 펼쳐지고, 기호 카운터면 back-edge goto로 남음
- `for-in`은 `nextKey(obj)` 근사, `try/with`는 주석만
- 다른 난독화기(문자열 암호화, 런타임 eval 로딩)는 미지원

# 작업 재개 안내

**진행 재개해**라고 하면 아래 순서로 저장된 작업을 이어간다. 새 기능과 검증 절차는 [MCP 개발 절차](MCP_DEVELOPMENT.md)에서 확인한다.

1. `feature/cv-structured-lifecycle`의 [현재 상태 파일](https://github.com/dev-universe/fender-studio-pro-mcp/blob/feature/cv-structured-lifecycle/docs/RESUME_STATE.json)에서 `status`와 `resume_document`를 읽는다. 작업 브랜치는 유지하고 최신 원격 기록을 확인한다.
2. `resume_document`가 가리키는 문서를 **같은 브랜치의 같은 커밋**에서 연다. 마지막 검증, 다음 작업, 열린 PR, 설치와 정리 상태에 해당하는 절부터 읽는다. 추가 정보가 필요하면 그 절이 가리키는 증거를 읽는다.
3. GitHub의 PR 헤드와 검사, 앱·플러그인 설치, 제어권과 장치를 새로 확인한다. 실앱 작업에는 새 세션·문서·인스턴스를 묶는다. 이전 마감과 장치 상태는 해당 날짜의 기록이다.

저장소에서 확인할 때는 아래처럼 상태 파일을 가져온 커밋을 고정한다.

```sh
git fetch origin feature/cv-structured-lifecycle
handoff_commit=$(git rev-parse FETCH_HEAD)
handoff_document=$(git show "$handoff_commit:docs/RESUME_STATE.json" |
	python3 -c 'import json, sys; print(json.load(sys.stdin)["resume_document"])')
git show "$handoff_commit:$handoff_document" | rg -n '^#{1,3} '
```

아래 기록은 날짜별 역사다. 이미 끝난 검증은 최신 인수인계에서 범위와 근거를 확인하고 재사용한다.

[11월 18일 전체 대기열](https://github.com/dev-universe/fender-studio-pro-mcp/blob/feature/cv-structured-lifecycle/docs/LOOP_HANDOFF_2026-11-18.md) · [11월 18일 진행 기록](PROGRESS_2026-11-18.md) · [11월 17일 인수인계](LOOP_HANDOFF_2026-11-17.md) · [11월 15일 인수인계](LOOP_HANDOFF_2026-11-15.md)

## 이전 실행의 제품과 범위

제품 소스는 `7f0cd1da13b18a0097344d1ad7a84c550d764b77`, 정상 설치와 마지막 실제 수용은 Native59다. R81의 실제27요청·7행·상세56필드에서 구조형 인터페이스 지원7/7을 독립 수용했다. 실제620.268초와 독립검토273.821초에 원본CV/Main4쌍·NoSave·owner의 명시적exit0·소스181/설치13/저장10/Main보존을 확인했다. R78·R79의 Root 기록 오류17요청은 미수용으로 남긴다. 준비·정리·선택지·포트쓰기·프리셋경로보존은 미검증이다. 다음은 공개 구조형 인터페이스의 cleanup/count/get과 owner계약이며 존재조회는 반복하지 않는다.
도구 46개, 고정 버튼 동작 5개, 상태 동작 7개이며 소스 181개와 설치 파일 13개를 대조한다.
문서만 바뀌면 같은 제품을 다시 빌드하거나 설치하지 않는다.

고정 43개 중 15개 완료, 28개 잔여다. 소프트웨어 24개와 하드웨어 4개를 구분한다.
프리셋은 40/46종과 77사례, 이름은 6866/7112개다. 이 분모를 서로 환산하지 않는다.
전체 완료 예상 시간은 아직 신뢰할 수 없어 미정이다. 단위별 실제 시간과 다음 유한 단위 예상, 예정된 정리 종료까지의 시간을 보고한다.

Scope r68의 실제 81요청과 원본 12장으로 기본 A의 단발 캡처 유지, 한 번의 재트리거 갱신, 전체 41개 설정 복원을 독립 수용했다.
직접 armed 상태·완료 시각, 숨은 파형 메모리 복원, 비활성 C 표시 차이는 남으므로 전체 CONTROL-SCOPE 완료 수는 올리지 않았다.
Vocal r69/r70의 집계값·동적 접근 예상 실패는 보존했다. 새 r71의 36요청·원본6장 host/UI 왕복과 전체26복원을 독립 수용해 CONTROL-VOCAL을 완료 처리했다. 오디오 신호와 저장 지속성은 별도다.
실패한 연결을 계속 쓰거나 계획되지 않은 역변경을 보내지 않는다.

## 유지할 사용자 결정

- 공식 8.1.1 설치 파일 대조는 사용자가 생략했다. 설치 경로나 MyFender 로그인을 다시 요청하거나 기다리지 않는다.
- Volt 276과 Keystation88 MK3는 11월 18일 USB로 재연결했다. 루프백 배선은 확인되지 않았고 당시 연결 마감은 오전 6시였다.
- Hopper를 사용하지 않는다. 추가 분석에는 Ghidra와 REA의 기존 증거를 활용한다.
- 사용자 보고는 한국어, 내부 지시는 한국어 또는 영어만 사용한다.
- branch와 PR 제목에 `codex`를 쓰지 않는다.
- 원본 곡·오디오·로그·로컬 경로·독점 앱 리소스를 Git에 넣지 않는다.
- 중단 후에는 새 재개 지시 없이 실제 검증을 자동 시작하지 않는다.

이전 [11월 12일 인수인계](LOOP_HANDOFF_2026-11-12.md)와 [11월 10일 인수인계](LOOP_HANDOFF_2026-11-10.md)는 역사적 증거다.
그 문서의 과거 시작 단위를 다시 실행하지 않는다.

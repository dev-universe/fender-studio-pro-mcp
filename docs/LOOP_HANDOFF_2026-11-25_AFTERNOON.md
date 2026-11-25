# 2026년 11월 25일 오후 개발 재개

## 완료 기준과 현재 상태

재개 당시 11월 24일의 고정 단위는 43개였다. 18개 완료, 25개 잔여였으며 그중 4개는
하드웨어가 필요하다. 이번 세션에 지정된 종료 시각은 없다. 11월 24일의
소프트웨어 2~5일 추정에 장비 대기는 포함되지 않는다. 재개 시 실제
Studio Pro 8.1.1 빌드 113103을 확인했고 Volt 276과 Keystation88 MK3는
감지되지 않았다. [25개 대기열](LOOP_HANDOFF_2026-11-24.md#재개-대기열-전체)

당시 최신 `main`에서 단위별로 작업한다. 실앱에서는 이름을 붙인 소유
시험곡, 단일 앱·MCP owner, 새 세션·문서·인스턴스를 확인한다. 변경 전후
독립 읽기와 화면, 원복 및 정리까지 끝낸 범위를 기록한다. 독립 결과가
있으면 PR을 열고, 정확한 최신 헤드의 Core 및 macOS arm64 필수 검사 후
머지와 원격 결과 확인을 마친다. 매 단위의 체크리스트, 실제 소요 시간,
고정 대기열 잔여 수, 다음 단위와 전체 예상, 지정된 종료 시각을 보고한다.

## 15시 10분 체크포인트

- macOS arm64 패키지의 빌드 두 개를 격리 프로필에서 설치·갱신·복구·제거했다.
  원래 설정과 사용자 수정 파일을 보존했다. [#109](https://github.com/dev-universe/fender-studio-pro-mcp/pull/109)는
  필수 검사 후 `cbb0540`으로 머지됐다. 공개 Release 자산이 없어서 배포본
  다운로드와 설치는 미검증이다. Actions 업로드 후보는 로컬 조립·설치가
  통과했다. GitHub CLI 푸시는 `workflow` 범위 부족, GitHub 연결 도구는
  HTTP 403으로 거부됐다. 권한 변경 없이 쓸 수 있는 로컬 조립 스크립트의
  별도 검증을 진행한다.
- R320에서 독립형 Pedalboard에 앰프 전후 구분선이 없음을 확인했다.
  Ampire는 `opt.stomporder` 4→1→4와 화면, 원복 설정 102개, Pedalboard
  40개 및 두 상태 지문을 확인했다. 앱은 141.111초, MCP는 93요청이었다.
  빈 슬롯과 동일 모델 시나리오의 전체 복원은 남았다.
  [#110](https://github.com/dev-universe/fender-studio-pro-mcp/pull/110)
- R321A는 Rumble 공장 프리셋 10에서 1로 돌아와 39개 DSP 행, 26개 슬롯,
  22개 숫자 설정의 정확한 복원과 토큰 만료를 확인했다. R321B는 프리셋
  100 이후 기준 1을 불렀을 때 DSP 수치와 제목이 돌아왔지만 `effect2`와
  `effect3` 모델 ID가 그대로였다. 앱 두 번 합계 192.210초였다. 다음은
  프리셋 전용 Undo 기록을 확인하고 그 항목이 실제로 생긴 경우에만
  Undo 한 번을 실행해 전체 원복을 시험하는 R324다.
  [#111](https://github.com/dev-universe/fender-studio-pro-mcp/pull/111)
- R322는 Multi Instrument 카탈로그의 1-Mai Tai 프리셋을 실제로 적용했다.
  화면과 새 owner의 150개 DSP 행이 확인됐다. 클래스와 인스턴스 UID는
  같고 호스트 `object_id`가 프리셋 제목으로 바뀌었다. 기존 `plugin_controller`
  읽기 경로가 표시 이름 조건에서 거부됐다. 정확한 클래스·종류·UID로 묶는
  수정 뒤 새 시험곡에서 1→2→1을 진행한다. 앱은 155.977초였다.
- Impact는 기존 `focus()`와 일반 선택을 반복하지 않는다. 설치된 앱의
  `Track / Show in Console` 명령을 확인했으며, 이전 Undo로 복구한 주 출력에
  이 명령이 두 선택을 함께 유지하는지 R323으로 시험할 준비 중이다.

R320과 R321A/B 및 R322 앱과 MCP owner가 정상 종료했다. 모든 소유
시험곡은 저장 없이 닫았고 원본 바이트, 정상 설치 18개와 복원한 설정
10개를 대조했다. 각 PR의 상태는 다음 재개에서 실제 GitHub 값으로 확인한다.

## 15시 37분 체크포인트

- `main`은 `edb4165`다. [#110](https://github.com/dev-universe/fender-studio-pro-mcp/pull/110)은
  `c2678d9`, [#111](https://github.com/dev-universe/fender-studio-pro-mcp/pull/111)은
  `edb4165`에 병합했다. 두 PR 모두 병합 직전 현재 `main`과 정합하는 정확한
  헤드에서 필수 검사 두 개를 통과했다. R324 Rumble의 프리셋 적용 뒤 Undo
  메뉴 맨 위는 그대로여서 Undo는 실행하지 않았다.
- R325에서 정확한 클래스·종류·인스턴스 ID로 Multi Instrument의 프리셋
  적용 뒤 컨트롤러를 두 번 읽고 다른 인스턴스 요청을 거부했다. macOS arm64
  릴리스 테스트 498개 통과. [#113](https://github.com/dev-universe/fender-studio-pro-mcp/pull/113)
- 새 R326에서 Multi 공장 프리셋 47→48→47의 UI 연결과 두 독립 조회가
  Mai Tai 슬롯 1→2→1을 표시했다. 루트 150행과 정렬 슬롯 목록은 정확히
  돌아왔고 세 가지 거부 조건을 통과했다. 같은 루트 상태에서 연속 캡처의
  해시가 달랐다. 자식 DSP 읽기와 실제 이벤트 경로를 별도로 검증한다.
  109요청·103.255초, 앱 138.894초이다. [#114](https://github.com/dev-universe/fender-studio-pro-mcp/pull/114)
- R323 Impact는 출력 선택기를 확인하기 전에 종료했다. 분리 콘솔을 사용한
  R324 Impact는 한 번의 Undo로 St 1과 색상, 편곡 선택을 확인했다. 보존된
  DSP 4,262행과 출력 32개를 읽었다. `Console / Previous Channel` 한 번은
  콘솔 선택을 St 1로 바꾸면서 편곡 선택을 해제했다. 207요청·109.469초,
  앱 161.940초이다. R328에서 `Track / Show in Console`을 시험한다.
  [#115](https://github.com/dev-universe/fender-studio-pro-mcp/pull/115)
- Rumble 028 레벨 시험은 `input.level`을 −33.36 dB로 바꾼 뒤 028이
  입력 0 dB, 출력 −3.10 dB와 다른 앰프·첫 효과를 적용함을 확인했다.
  001 재적용으로 DSP 39행, 26개 모델, 22개 숫자 설정과 출력 −11.00 dB를
  정확히 복원했다. 100요청·29.417초, 앱 69.139초이다. `010`에 이은 두 번째
  수용 사례다. `PRESET-RUMBLE` 검증 기준을 충족해 검증 잔여는 24개이며
  기록 PR 병합을 포함한 잔여는 25개다. [#116](https://github.com/dev-universe/fender-studio-pro-mcp/pull/116)
- R327은 플러그인 자체 `List View`에서 100→001을 시험했다. 제목, DSP
  39행과 숫자 22개는 돌아왔고 두 독립 읽기에 `effect2`, `effect3`의 모델
  ID가 남았다. 88요청·66.442초, 앱 106.431초이다. 실제 처리 여부는 미확인
  상태며 `CONTROL-RUMBLE`에서 추적한다. [#117](https://github.com/dev-universe/fender-studio-pro-mcp/pull/117)
- macOS 로컬 arm64 아카이브를 조립하고 새 격리 환경에서 설치·진단·제거했다.
  [#112](https://github.com/dev-universe/fender-studio-pro-mcp/pull/112)는
  최신 `main` 반영 헤드 `8292b42`에서 재검사 중이다. 이후 #113, #116,
  #114, #115, #117을 최신 `main`에서 차례로 검사한다. 공개 Release 자산의
  설치와 이전 설치 원복은 아직 남았다.

위 실앱 시험은 모두 이름을 확인한 소유 시험곡에서 진행했다. 앱과 MCP
owner를 정상 종료했고 저장을 거부했다. 18개 관리 파일과 모드, 앱 설정 및
시험곡 원본을 다시 확인했다. 전체 소프트웨어 예상 2~5일, 하드웨어 4개는
실제 장치와 배선 확인이 필요하다. 이번 작업의 종료 시각은 정해져 있지 않다.

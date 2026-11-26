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

## 16시 18분 체크포인트

- `main`은 `2fd0a62`다. 오후 재개 후 #109, #110, #111, #112, #113,
  #116, #114가 병합됐다. #116까지 반영한 고정 대기열은 43개 중 19개
  완료, 24개 잔여다. Multi 47/48 실앱 검증을 반영하면 로컬 기준 20개
  완료, 23개 잔여다. 이 단위는 #121, #122, #123의 원격 병합을 기다린다.
- R328에서 `Track / Show in Console`은 편곡 선택과 DSP 4,262행, 출력
  32개를 유지했지만 콘솔 선택은 St 2에 남았다. 252요청·148.336초,
  앱 200.061초다. [#118](https://github.com/dev-universe/fender-studio-pro-mcp/pull/118)
- R329에서 임시 프로필의 내장 Qwerty Keyboard를 사용해 C3 한 번의 누름과
  뗌, Chorder의 5음 출력과 두 번의 중립 복귀를 독립 조회로 확인했다.
  242요청·119.553초와 원복 39요청·3.820초다. 별도 물리 MIDI 입력과 중복·
  보류·컨텍스트 유실은 확인이 필요하다. [#119](https://github.com/dev-universe/fender-studio-pro-mcp/pull/119)
- 설치 프로그램에 번들 `CodeResources`를 보존하고 실패 시 복원하는 변경을
  추가했다. 격리 설치의 ad-hoc 서명 검증과 복원 시험을 통과했다. 공개
  Release, Developer ID 서명·공증과 기본 Gatekeeper 통과는 미검증이다.
  [#120](https://github.com/dev-universe/fender-studio-pro-mcp/pull/120)
- R330에서 새 Multi 자식 조회로 47→48→47을 실행했다. 같은 Mai Tai 클래스의
  각 슬롯에서 정확한 인스턴스에 바인딩한 283행을 별도로 읽고 정확한 값 전환과
  복원을 확인했다. 루트 150행도 일치했다. 174요청·117.724초,
  앱 181.414초다. [#121](https://github.com/dev-universe/fender-studio-pro-mcp/pull/121)
- R331에서 프리셋 48의 두 자식, 명시적인 그래프 7노드·6연결 및 저장 후
  새 세션의 루트·자식 조회를 대조했다. 저장·재개 합계 3분 14초다.
  [#122](https://github.com/dev-universe/fender-studio-pro-mcp/pull/122)
- R332에서 역방향 48→47→48의 화면, 루트 150행 및 자식 최대 566행을
  확인했다. 소유 시험곡에는 47을 한 번만 저장해 공장 그래프 3노드·2연결 및
  전체 키 범위를 대조했다. 236요청·149.303초, 앱 229.084초다. 세 PR이
  병합되면 `PRESET-MULTI`를 닫는다. 실제 MIDI 전달과 일반 중첩 그래프
  제어는 `CONTROL-MULTI`에서 추적한다.
  [#123](https://github.com/dev-universe/fender-studio-pro-mcp/pull/123)

R328–R332의 앱·owner 종료 및 정상 관리 설치와 설정 복구를 대조했다.
R331과 R332의 명시된 저장은 각 소유 시험곡에만 수행했다. #115, #117,
#118–#123은 현재 `main`에서 각 PR의 정확한 헤드를 다시 검사한 뒤 순서대로
병합한다. 다음 실험은 Scope의 기존 캡처·홀드 근거를 재사용하고 원샷
시작·완료의 독립 관측 경로를 조사하는 작업이다. 전체 소프트웨어 예상은
2~5일이며 장비가 필요한 4개는 포함하지 않는다. 지정된 종료 시각은 없다.

## 16시 43분 체크포인트

- `main`은 `593dc99`다. #115와 #117은 각각 `c21d4f4`, `593dc99`에
  병합됐다. 두 PR의 최신 `main`을 포함하는 정확한 헤드에서 필수 검사 두
  개를 확인했다. #118–#126은 순서대로 현재 `main`에서 다시 검사한다.
- R333에서 Scope 원샷과 재트리거가 공통 내부 재시작 경로에 연결되고,
  설정 모드와 실제 처리 플래그·샘플 카운터가 별개임을 REA·Ghidra와
  ARM64 명령으로 확인했다. Craft Anything Deep에서 초기화 명령을
  대조했다. R68의 제한된 파형 근거를 재사용했다. 실제 캡처 시점의 독립
  관측, 숨은 버퍼 복원, 비활성 C 상태는 열린 상태다.
  [#124](https://github.com/dev-universe/fender-studio-pro-mcp/pull/124)
- R334는 앱 시작 뒤 등록한 가상 포트를 고르지 못해 전송 전에 중단했다.
  R334B는 가상 Core MIDI 포트를 앱 시작 전에 열었다. Mojito의 채널
  압력과 모듈레이션은 0→7559.055→0이었고, 피치 벤드는 MIDI 중앙으로
  돌아갈 때 7500.458→5000.305였다. 별개 정확 쓰기 한 번으로 해당
  DSP를 5000으로 되돌려 35개 전체를 독립 조회 두 번으로 확인했다.
  153요청, R334B 앱 197.569초다. 임시 장치와 설정 및 설치를 복구했다.
  [#125](https://github.com/dev-universe/fender-studio-pro-mcp/pull/125)
- R335는 구형 `pan=0`, `pan=1`, `pan=1`과 `volume=0.25` 대조곡을
  각각 새 세션에서 열었다. `pan`만 다를 때 37개 전체와 믹서는 같고,
  `volume`을 바꾸면 그 행만 10000에서 2500으로 달라졌다. REA·Ghidra와
  Craft Anything은 Mojito의 레거시 변환 함수가 값을 읽지 않는 것을
  확인했다. 세 앱 실행 합계 96.970초, 30요청이다. 이 증거와 R334B로
  `CONTROL-MOJITO`의 로컬 수용 조건을 충족했다.
  [#126](https://github.com/dev-universe/fender-studio-pro-mcp/pull/126)

R334와 R335 앱·owner는 모두 닫았고 정상 관리 파일 18개와 앱 설정을
원복했다. 고정 43개 중 `main` 기준 19개 완료, 24개 잔여다. Multi와
Mojito 로컬 검증까지 포함하면 21개 완료, 22개 잔여다. 다음은 기존 Mai Tai
근거에서 MIDI 6역할과 `character1.editchar`의 화면·모델 조건을 분리한다.
가상 Core MIDI 입력에는 R334B의 시작 순서와 전용 트랙 확인을 재사용하며,
물리 장치 4개에는 별도 시험이 필요하다. 전체 소프트웨어 예상 2~5일이며
이번 세션에 지정된 종료 시각은 없다.

## 17시 02분 체크포인트

- `main`은 `7cb8ef0`다. #118과 #119는 각각 `4621b98`, `7cb8ef0`으로
  병합됐다. 현재 `main`이 포함된 정확한 각 헤드에서 Core와 macOS arm64
  검사 모두를 확인했다. #120–#128은 같은 기준으로 대기 중이다.
- R336D는 Studio Pro 8.1.1의 Mai Tai에서 채널 압력, CC1·2·11·64와
  피치 벤드의 수신·해제 및 전체 283값을 확인했다. 피치 벤드 중앙은
  5000.305였고 바인딩된 쓰기 한 번으로 5000을 복원했다. 169요청·95.540초,
  앱 128.867초다. 사전 R336/R336B/R336C는 전송 전에 catalog UUID 형식,
  Main의 장치 상태 필드와 행 인덱스 판단을 고친 뒤 종료했다.
  [#127](https://github.com/dev-universe/fender-studio-pro-mcp/pull/127)
- R337에서 `editchar` 0→1은 한 DSP 값만 바꾸고 기본 화면에 별도 에디터를
  띄우지 않았다. REA·Ghidra·Craft Anything에서 해당 기능 조회의 거짓
  값을 대조했다. 화면의 GrandClass→CharacterSaw는 `presets` 0→3으로
  반영됐고 `type=Blend`는 유지됐다. 두 항목을 원복하자 283값과 믹서가
  돌아왔다. 96요청과 앱 200.299초다. [#128](https://github.com/dev-universe/fender-studio-pro-mcp/pull/128)

R336/R337 소유 시험곡과 앱, owner를 닫았으며 저장하지 않았다. 원본 곡
해시, 정상 관리 파일 18개, 앱 설정 복원을 대조했다. `CONTROL-MAITAI`의
검증 범위가 로컬에서 충족돼 `main` 기준 잔여는 24개, 로컬 검증 기준은
21개다. 다음은 Pedalboard R317의 빈 슬롯 잔류값과 실제 저장 상태를
분리하는 시험이다. 장비가 필요한 4개, 전체 소프트웨어 예상 2~5일과
지정된 종료 시각 없음은 기존과 같다.

## 17시 20분 체크포인트

- `main`은 `53f44d6`다. [#120](https://github.com/dev-universe/fender-studio-pro-mcp/pull/120)이
  현재 `main`을 포함한 정확한 헤드에서 필수 검사 두 개를 통과해 병합됐다.
  #121–#130은 현재 `main` 반영과 필수 검사 순서대로 대기 중이다.
- R338은 빈 슬롯 역교환 직후의 `controls.active[stomp2]` 10000이
  저장 후 같은 프로세스에도 남음을 확인했다. 저장한 빈 슬롯에는 0 클래스
  ID만 있었고 재시작 뒤에는 값이 0이 되어 33행 전체와 동반 Phase Meter가
  원래 상태와 일치했다. 두 앱 총 133.740초, 107요청이다.
  [#129](https://github.com/dev-universe/fender-studio-pro-mcp/pull/129)
- R339의 Compressor 두 개는 `Gain` 0.324/0.683과 active 1/0에서
  교환됐다. 화면과 저장 파일은 값이 바뀌었고 같은 세션의 `stompN` DSP
  값은 유지됐다. 재시작 뒤에는 활성 별칭과 슬롯별 DSP의 교환된 6행이
  저장 내용에 맞았고 나머지 32행과 Phase Meter가 보존됐다. 앱 총
  111.656초, 99요청이다. [#130](https://github.com/dev-universe/fender-studio-pro-mcp/pull/130)

두 시험은 각 소유 시험곡을 한 번씩만 저장했고 후속 재시작에서 곡을 다시
저장하지 않았다. 앱과 owner를 닫고 정상 관리 18개와 변경된 앱 설정
10개를 원복했다. `main` 기준 24개, 로컬 검증 기준 21개가 남았다.
다음은 별도 R340에서 원래 동일 모델 판독 차이를 재현한 뒤 현재 문서의
유효한 캡슐을 같은 인스턴스에 한 번 복원해 새 DSP 읽기와 지문을 대조하는
시험이다. 이 비교는 이미 병합한 공개 교환 허용 범위를 변경하지 않는다.

## 17시 35분 체크포인트

- [#121](https://github.com/dev-universe/fender-studio-pro-mcp/pull/121)이
  필수 검사 두 개를 통과해 `316a252`로 병합됐다. #122–#133은 순서대로
  현재 `main`과 정확한 헤드의 필수 검사를 기다린다.
- R340 동일 모델 교환 뒤 원래 값으로 남은 슬롯별 Gain과 active 4개를
  교환 직후의 캡슐을 같은 인스턴스에 한 번 로드해 갱신했다. 원래
  캡슐을 한 번 로드해 전체 38행과 지문, Phase Meter를 복원했다.
  96요청, 앱 185.020초. [#131](https://github.com/dev-universe/fender-studio-pro-mcp/pull/131)
- R341 빈 슬롯 역교환 후 동일한 지문을 한 번 로드해 active 별칭이
  10000에서 0으로 돌아왔다. 독립 조회 두 번에서 33행과 지문이
  일치했다. 95요청, 앱 75.496초. [#132](https://github.com/dev-universe/fender-studio-pro-mcp/pull/132)
- R342에서 `plugin_state capture`의 슬롯 8개 지문을 검증했다.
  빈 슬롯과 다른 설정의 같은 모델을 구분했다. 공개 Compressor/Reverb
  교환과 역교환에서 지문, 화면, 전체 혼합 랙 상태 및 동반 Phase Meter를
  대조했다. 복원 요청은 없었다. 95요청, 앱 68.062초, 서버 259개·
  네이티브 503개 테스트 통과. [#133](https://github.com/dev-universe/fender-studio-pro-mcp/pull/133)

R340–R342 시험곡은 저장 없이 닫혔다. 앱과 owner 종료, 매회 변경된 앱
설정 10개와 관리 파일 18개 원복을 확인했다. `main` 기준 24개, 로컬
검증 기준 21개가 남았다. 다음은 새 지문을 공개 교환의 전제와 독립
재확인에 연결하고 빈 슬롯·동일 모델·비인접 교환을 실제 앱에서
검증하는 작업이다. 기존 인접 교환 호출을 유지한다.

## 17시 49분 체크포인트

- [#122](https://github.com/dev-universe/fender-studio-pro-mcp/pull/122)
  저장·재개와 [#123](https://github.com/dev-universe/fender-studio-pro-mcp/pull/123)
  양방향 프리셋 검증이 병합됐다. `main`은 `2d3ed04`이며
  `PRESET-MULTI`를 집계에 반영했다.
- R343 공개 MCP 검증에서 잘못된 지문을 세 경우 모두 거부했다.
  빈 슬롯 1↔2, 같은 Compressor 1↔2, Compressor와 Big Fuzz 1↔3을
  개별 곡에서 교환·역교환했다. 독립 캡처와 화면에 일치하는 원래
  컴포넌트 지문이 복구됐고, 중간 Reverb와 Phase Meter가 보존됐다.
  빈 슬롯의 활성 별칭과 동일 모델의 Gain/active 컨트롤러 조회는
  이전 값을 보고할 수 있다. 상태 재로드 호출은 없었다.
  [#134](https://github.com/dev-universe/fender-studio-pro-mcp/pull/134)
- 성공한 세 번은 256요청, 앱 190.806초다. 사전 거부의 MCP 오류 플래그,
  역교환 뒤 동일 모델의 조회 지연, 읽기 전용 피크 두 개를 구분해
  재검증했다. 서버 260개, 브리지 438개, 네이티브 504개를 통과했다.
  여섯 시험 곡은 저장하지 않았고 앱 및 owner 종료, 정상 관리 파일
  18개와 앱 설정 원복을 확인했다.

`main` 기준 23개, 로컬 검증 기준 20개가 남았다.
`CONTROL-PEDALBOARD`는 #129–#134의 병합을 기다린다.
#134의 초기 base는 #133이고, #133 병합 후 `main`으로 변경해 정확한
헤드를 재검사한다. 다음은 기존 후보를 되풀이하지 않고 Mustang Native
현재 모듈과 모델 선택 경로를 읽기부터 확인하는 작업이다.

## 18시 03분 체크포인트

- Scope [#124](https://github.com/dev-universe/fender-studio-pro-mcp/pull/124)는
  두 필수 검사를 통과해 `947753e`로 병합됐다. 직접 파형 완료 시점과
  숨은 버퍼 상태를 확인해야 `CONTROL-SCOPE`가 끝난다.
- R344는 Mustang Native의 39개 DSP 값과 부분 컨트롤러 보고를 확인했다.
  Add 모델 화면과 MCP 두 카탈로그는 0행이었다. 같은 앱 실행의 로더
  로그와 별도 서명 검사에 arm64 모델 모듈 서명 누락이 기록됐다.
  31요청, 앱 60.853초. 모델 선택은 실행하지 않았다.
  [#135](https://github.com/dev-universe/fender-studio-pro-mcp/pull/135)
- R345는 소유한 두 SoundFont를 Presence에 번갈아 불러왔다. 첫 파일을
  다시 불러오면 화면과 DSP 241개는 일치하고 전체 지문은 달랐다.
  같은 인스턴스에서 최초 캡슐을 한 번 불러오자 독립 캡처 두 번의
  지문과 241개 모두 일치했다. 본 검증 98요청과 앱 286.653초다.
  [#136](https://github.com/dev-universe/fender-studio-pro-mcp/pull/136)

R344–R345 시험 곡은 저장 없이 닫았으며 앱과 owner 종료,
관리 파일 18개와 변경된 앱 설정 10개 원복을 확인했다.
`main` 기준 23개와 로컬 기준 20개가 남았다. 다음은 Presence
사운드가 로드된 인스턴스에 전용 가상 MIDI 입력을 연결해
여섯 역할의 활성값·해제 및 원복을 확인하는 시험이다.

## 18시 30분 체크포인트

- Mojito [#125](https://github.com/dev-universe/fender-studio-pro-mcp/pull/125)와
  [#126](https://github.com/dev-universe/fender-studio-pro-mcp/pull/126)이 정확한
  최신 헤드에서 두 필수 검사를 통과해 병합됐다. `main`은 `8fb4121`이다.
  `CONTROL-MOJITO`를 닫았고 #127이 다음 검사에 들어갔다.
- Presence R346C에서 전용 가상 Core MIDI 입력으로 채널 압력, CC1·2·11·64와
  피치 벤드의 활성값과 해제를 대조했다. 벤드 중앙의 변환값을 정확한 비트로
  복원해 241행과 상태 지문을 모두 돌려놓았다. 성공 203요청, 앱 148.746초다.
  입력 전 중단한 R346 및 R346B를 포함하면 앱 586.074초다.
  [#137](https://github.com/dev-universe/fender-studio-pro-mcp/pull/137)
- Presence R347에서 스크립트 매크로 16개를 읽고 그중 두 값의 독립 읽기와
  연속 매크로의 실제 콜백을 확인했다. 시험한 직접 존 Tune 접근은
  `undefined`를 반환하고 화면에서도 `0.00 st`가 유지됐다. 같은 세션의
  상태 복원 이전에 중단했고 시험 곡은 저장하지 않았다. 편집기 조회
  56요청과 매크로 38요청, 앱 458.224초다.
  [#138](https://github.com/dev-universe/fender-studio-pro-mcp/pull/138)

앱과 각 owner 종료, 시험 곡 원본과 관리 파일 18개 및 앱 설정의 원복을
대조했다. `main` 기준 22개와 로컬 기준 20개가 남았다. 다음은 소유한
Presence 프로그램에서 검증 가능한 타깃에 매크로 16개를 연결하고
실제 값과 정확한 복원을 시험한다. 전체 소프트웨어 추정 2~5일, 장비
의존 4개, 지정된 세션 종료 시각 없음.

## 18시 45분 체크포인트

- Mai Tai 가상 MIDI [#127](https://github.com/dev-universe/fender-studio-pro-mcp/pull/127)이
  두 필수 검사를 통과해 `85984a9`에 병합됐다. #128이 현재 베이스에서
  검사 중이며, 해당 PR까지 병합되면 Mai Tai를 집계에 반영한다.
- R348은 소유 SoundFont에서 Presence 매크로 16개의 독립값과 실제
  스크립트 콜백을 확인했다. REA/Ghidra와 Craft Anything Deep으로 스크립트
  함수 디스패치를 확인했다. 내장 키보드 누름에서는 스크립트 노트 로그를
  관측하지 못했다. 앱 250.617초. [#139](https://github.com/dev-universe/fender-studio-pro-mcp/pull/139)
- R349B는 전용 가상 장치로 같은 MIDI 60번 노트를 두 번 보냈다.
  매크로 16개 활성 상태에서 폐기, 중립 상태에서 보이스 시작을 실제
  스크립트 로그로 확인했다. 두 단계에서 매크로 16개를 독립 조회했고
  다른 225개 값은 유지됐다. 캡슐을 한 번 복원해 지문·241행·원래
  프로그램과 빈 스크립트 화면을 확인했다. 호스트의 변경 표시 별표는
  남았다. 첫 R349는 확인 단계 누락으로 MIDI 전송 전에 중단됐다.
  성공한 앱 219.700초, 양쪽 합계 418.521초, 성공 301요청.
  [#140](https://github.com/dev-universe/fender-studio-pro-mcp/pull/140)

모든 시험곡은 저장하지 않았다. 두 owner와 가상 MIDI 소스 종료,
시험곡 원본, 관리 파일 18개와 앱 설정 원복을 확인했다.
`CONTROL-PRESENCE`는 로컬 검증을 충족해 #137–#140 병합을 기다린다.
`main` 기준 22개, 로컬 기준 19개가 남았다. 다음은 기존 Ampire의
경계 이동·프리셋 근거를 재사용해 마이크 믹스 연동, 호스트 바이패스와
정확한 복원을 확인하는 작업이다. 전체 소프트웨어 추정 2~5일, 장비
의존 4개, 지정된 세션 종료 시각 없음.

## 18시 59분 체크포인트

- Mai Tai [#128](https://github.com/dev-universe/fender-studio-pro-mcp/pull/128)이
  두 필수 검사 후 `8cd3370`에 병합됐다. Mai Tai를 집계에 반영했다.
  #129는 최신 기반의 정확한 헤드에서 macOS 검사를 통과했고 Core를 기다린다.
- Ampire R350의 연동 Mic B 한 번은 A·B·C에 영향을 줬다. 캡슐 로드
  뒤 Mic A의 원본보다 약 0.000596 낮은 값과 다른 전체 지문이 두 새
  조회에서 반복됐다. 앱 78.707초. [#141](https://github.com/dev-universe/fender-studio-pro-mcp/pull/141)
- R351은 링크를 끈 Mic B/C의 독립 변경과 정확한 비트 원복을 확인했다.
  링크를 켰을 때 두 값에 각각 약 0.000298 차이와 다른 지문이 생겼다.
  90요청, 앱 46.533초. [#142](https://github.com/dev-universe/fender-studio-pro-mcp/pull/142)
- R352는 준비한 25%·25%·50%의 C를 37.5%로 바꾸면 연동 A/B가
  각각 31.25%인 것을 봤다. 링크를 끄면 A/B가 먼저 25%로 복귀했고,
  새 owner에서 정확한 C 복원과 재연동 후 90행, 원래 지문, 화면이
  일치했다. 앱 201.852초와 두 owner 142요청.
  [#143](https://github.com/dev-universe/fender-studio-pro-mcp/pull/143)
- R353은 세 종류의 바이패스를 왕복했다. 호스트 바이패스 중 값과 지문은
  유지되고 90행이 비활성화됐다. 원복 후 전체 구조와 값이 일치했다.
  내부 스톰프와 튜너 바이패스는 각각 자신만 바뀌었다. 앱 149.814초,
  성공한 후속 owner 98요청. [#144](https://github.com/dev-universe/fender-studio-pro-mcp/pull/144)

R350–R353 시험 곡은 저장하지 않았으며 앱과 owner 종료, 원본 바이트,
18개 관리 파일 및 10개 앱 설정을 확인했다. `main` 기준 21개,
로컬 기준 19개가 남았다. Ampire의 공장 초기 믹스 정밀 복원과 별칭,
튜너 표시 모드는 계속 추적한다. 전체 소프트웨어 추정 2~5일,
장비 의존 4개, 지정된 종료 시각 없음.

## 19시 15분 체크포인트

- Pedalboard 빈 슬롯 저장 [#129](https://github.com/dev-universe/fender-studio-pro-mcp/pull/129)와
  동일 모델 저장 [#130](https://github.com/dev-universe/fender-studio-pro-mcp/pull/130)이
  두 필수 검사 후 병합됐다. `main`은 `d844c2d`다. #131부터 이어간다.
- R354는 Ampire 튜너의 실제 Strobe 스위치와 `tuner.gaugeMode` 동작,
  전체 90개와 상태 지문의 원복을 확인했다. 앱 101.637초.
  [#145](https://github.com/dev-universe/fender-studio-pro-mcp/pull/145)
- R355 기본 슬롯 1·5·6과 R356 신규 Compressor 슬롯 2·3·4·7·8에서
  슬롯별 활성 별칭, 처리부 활성값과 전체 지문을 원복했다. Chorus의
  움직이는 읽기 전용 LFO는 별도 관찰했다. 앱 108.613초 및 69.554초.
  [#146](https://github.com/dev-universe/fender-studio-pro-mcp/pull/146) ·
  [#147](https://github.com/dev-universe/fender-studio-pro-mcp/pull/147)
- R357에서 Speed 별칭과 `stomp5.vibratoRate`가 함께 6250까지
  바뀌고 두 값과 전체 90행 및 지문이 정확히 복원됐다. 앱 74.629초.
  같은 곡에서 모델 List 선택 조회는 원래 native min/max가 Float
  0/15라 거부됐다. [#148](https://github.com/dev-universe/fender-studio-pro-mcp/pull/148)
- `fix/native-mutable-list-choices`의 로컬 `ec89417`은 정수값인
  Float 경계와 원래 변형을 유지한 포매터 호출을 추가했다. 라이브
  검증은 별도 R358에서 새 네이티브 빌드를 사용한다. 148개 파라미터
  라이브러리 단위 테스트와 4개 문서 테스트를 통과했다.

R354–R357 앱과 owner 종료, 시험 곡 원본, 18개 설치와 앱 설정 10개를
대조했다. `main` 21개, 로컬 19개 잔여다. Ampire의 Wah·Volume·Pitch
대상과 공장 마이크 믹스 정확 원복은 남았다. 전체 소프트웨어 추정
2~5일, 장비 의존 4개, 지정된 종료 시각 없음.

## 19시 34분 체크포인트

- #131이 두 필수 검사 후 `da57409`에 병합됐다. #132가 현재
  정확한 베이스에서 검사 중이다. [#131](https://github.com/dev-universe/fender-studio-pro-mcp/pull/131)
- R358은 부동소수점 경계를 반환하는 Ampire의 모델 목록을 새
  네이티브 빌드에서 스톰프 16, 앰프 6, 캐비닛 18개까지 읽었다.
  모델 선택 없이 90행과 전체 지문을 보존했다. 59요청,
  앱 49.164초. [#149](https://github.com/dev-universe/fender-studio-pro-mcp/pull/149)
- R359는 실제 Wah Wah와 별칭을 함께 0→6250→0으로 왕복해
  93행과 지문을 복원했다. R360은 빈 슬롯과 설치 모델 15개에서
  Volume/Pitch가 계속 0과 빈 표시임을 봤다. 처리 대상은 확인하지
  못했다. 슬롯을 비운 뒤 원래 지문과 별도로 활성 별칭 및 읽기
  전용 튜너 값 차이가 남았다. 앱 61.780초, 64.493초.
  [#150](https://github.com/dev-universe/fender-studio-pro-mcp/pull/150) ·
  [#151](https://github.com/dev-universe/fender-studio-pro-mcp/pull/151)
- R361의 내부 `bypass`는 화면과 DSP 값에 나타났으며 호스트
  인서트 활성과 전체 지문은 그대로였다. R362는 캡슐 로드 후
  Mic A를 정확히 보정하면 Mic B/C에 각각 약 0.000298 차이가
  생기는 경계를 확인했다. 앱 58.684초, 40.569초.
  [#152](https://github.com/dev-universe/fender-studio-pro-mcp/pull/152) ·
  [#153](https://github.com/dev-universe/fender-studio-pro-mcp/pull/153)
- SampleOne R363은 소유 WAV 12,000·24,000프레임을 불러와 화면과
  각 지문을 확인했다. 빈 캡슐 복원은 원래 235행과 지문이 정확했다.
  R364는 첫 프레임 0→256의 실제 변경과 로드된 프로그램 상태 복원을
  확인했다. 앱 136.636초, 128.707초.
  [#154](https://github.com/dev-universe/fender-studio-pro-mcp/pull/154) ·
  [#155](https://github.com/dev-universe/fender-studio-pro-mcp/pull/155)

이 시험에서 저장한 곡은 없다. 앱과 owner가 닫혔고 곡 원본,
관리 파일 18개와 변경된 설정 10개가 일치했다. 다음은 SampleOne
영역의 저장·새 세션 검증과 여섯 MIDI 입력이다. `main` 21개·로컬
19개 잔여, 전체 소프트웨어 추정 2~5일, 장비 의존 4개, 지정된 종료
시각은 없다.

## 19시 43분 체크포인트

- #132가 두 필수 검사 후 `d15bc77`에 병합됐다. #133은
  현재 베이스에서 두 검사를 실행 중이다.
  [#132](https://github.com/dev-universe/fender-studio-pro-mcp/pull/132)
- R365에서 소유 WAV의 영역을 Start 256 / End 12000으로 바꿔
  한 번 저장했다. 새 Studio Pro 프로세스에서 같은 곡 파일, 화면의
  파형과 범위, DSP 235행과 5,613바이트 컴포넌트 지문이 일치했다.
  새 문서와 세션은 이전 문서 토큰을 거부했다. 저장된 인스턴스 UID는
  같았다. 두 앱은 211.011초와 49.663초에 정상 종료했다. 설치 18개와
  설정 10개를 복원했다.
  [#156](https://github.com/dev-universe/fender-studio-pro-mcp/pull/156)

마지막 정리 완료 지점은 R365다. 다음은 SampleOne MIDI 6종, 이후
Chorder 제어다. `main` 21개·로컬 19개 잔여, 소프트웨어 2~5일,
장비 의존 4개, 지정된 종료 시각 없음.

## 19시 47분 체크포인트

- R366은 임시 XML 입력을 추가한 상태에서 브리지 바인딩이 타임아웃돼
  MIDI 발신 없이 종료됐다. 앱 49.603초, 송 원본과 설정·설치 복원.
- R366B는 실제 외장 장치 설정을 통해 전용 수신 포트를 연결했다.
  채널 압력, CC1/2/11/64, 피치 벤드를 각각 17회 독립 읽기로
  대조했다. 모듈레이션과 피치 벤드의 두 원래 Float64 비트를 보정해
  DSP 235행과 전체 샘플 상태 지문을 복원했다. owner 203요청,
  74.323초, 앱 129.630초. 송 저장 없이 종료했고 관리 설치 18개와
  설정 11개 및 원래 입력 프로필을 확인했다.
  [#157](https://github.com/dev-universe/fender-studio-pro-mcp/pull/157)

SampleOne 고정 항목은 R363–R366B로 로컬 완료, 공개 PR 병합 대기다.
`main` 21개·로컬 18개 잔여. 다음은 Chorder 전용 MIDI 눌림·해제,
학습과 다섯 동작이다. 이전 128개 개별 GUI 값 확인을 반복하지 않는다.
전체 소프트웨어 2~5일, 장비 의존 4개, 지정된 종료 시각 없음.

## 19시 56분 체크포인트

- #133이 두 필수 검사 후 `f2fc27f`에 병합됐다. #134는 베이스를
  `main`으로 바꾸고 새 헤드에서 필수 검사를 실행 중이다.
  [#133](https://github.com/dev-universe/fender-studio-pro-mcp/pull/133)
- R367의 가상 소스는 Core MIDI에서 확인됐으나 외장 장치 추가 창에
  보이지 않았다. R367B/C는 네이티브 설정 양식에서 수신 포트를
  확인했으며 검증기의 이름 조회 응답과 빈 코드 엔트리 전제를
  고치기 전 순서에서 중단했다. 모두 MIDI 발신 없이 종료했다.
- R367D는 실제 MIDI 60의 출력 [60, 63, 67, 70, 74]와
  미할당 MIDI 61의 입력만 활성화를 각각 확인했다. Note Off 두 번의
  독립 읽기에서 모든 관찰 플래그가 0이었다. DSP 908행, 저장 모델,
  전체 컴포넌트와 원래 믹서가 일치했다. owner 129요청, 19.827초,
  앱 55.599초. 송 변경을 저장하지 않았고 설정 10개, 관리 설치
  18개와 원래 입력 프로필을 확인했다.
  [#158](https://github.com/dev-universe/fender-studio-pro-mcp/pull/158)

`main` 21개·로컬 18개 잔여. 다음은 MCP의 Chorder 학습 음표 자체
편집과 저장 모델 왕복이다. 다섯 편집 명령, 하드웨어 4개는 열린 상태다.
전체 소프트웨어 2~5일, 지정된 종료 시각 없음.

## 20시 00분 체크포인트

- #134가 필수 검사 두 개를 통과해 `af4c588`에 병합됐다.
  빈 슬롯·동일 모델·비인접과 저장·되돌리기 증거까지 포함해
  Pedalboard 고정 항목을 공개 완료로 집계한다.
  [#134](https://github.com/dev-universe/fender-studio-pro-mcp/pull/134)
- R368에서 공개 MCP로 비어 있던 Chorder MIDI 61에 출력 65를
  추가했다. 두 독립 조회가 전체 128개 입력 중 해당 항목만 바뀌었고
  컴포넌트가 581→659바이트로 바뀌었음을 확인했다. 출력 65만
  제거하고 학습 모드·입력 선택을 원복하자 DSP 908행, 코드 128개,
  모델·컴포넌트 지문이 정확히 돌아왔다. 143요청, owner 25.730초,
  앱 59.470초. 저장 없이 종료했고 설정 9개 및 관리 설치 18개 복원.
  [#159](https://github.com/dev-universe/fender-studio-pro-mcp/pull/159)

다음은 Chorder Clear·Clear All, Copy·Paste 및 Preview다.
`main` 20개·로컬 18개 잔여, 소프트웨어 2~5일, 장비 의존 4개,
지정된 종료 시각 없음.

## 20시 04분 체크포인트

- R369에서 입력 60·61에 코드가 있는 상태로 선택된 61의 Clear는
  61만 비웠고, Clear All은 다시 준비한 두 코드를 비웠다.
  각 단계에서 전체 128개 저장 모델을 두 번 읽었다. 원본 캡슐
  한 번으로 581바이트와 두 지문이 돌아왔다. 해제 응답 바깥 상태는
  `applied_unverified`였고 안쪽 결과는 `released: true`라
  새 owner가 두 명령을 반복하지 않고 학습 UI와 DSP 908행을
  마저 원복 확인했다. 153·64요청, 앱 133.650초. 송·설정 9개·
  관리 설치 18개를 검증했다.
  [#160](https://github.com/dev-universe/fender-studio-pro-mcp/pull/160)

다음은 Copy·Paste의 범위와 Preview 눌림·해제다.
`main` 20개·로컬 18개 잔여, 소프트웨어 2~5일, 장비 의존 4개,
지정된 종료 시각 없음.

## 20시 10분 체크포인트

- #135의 Mustang Native 준비 상태 기록이 필수 검사 두 개를 통과해
  `700969a`에 병합됐다. 공식 정상 모듈과 실제 모델 목록 검증은 남았다.
  #136은 현재 베이스에서 검사 중이다.
  [#135](https://github.com/dev-universe/fender-studio-pro-mcp/pull/135)
- R370에서 Chorder C3의 Copy는 저장 모델을 보존했고 C♯3의
  Paste는 다섯 출력을 1반음 올렸다. 대상 외 127개 입력은 같았다.
  원본 캡슐, 두 지문, 전체 DSP 908행과 GUI 선택을 복원했다.
  179요청 113.665초, 앱 147.959초. 송·설정 9개·관리 설치 18개의
  닫힌 경계를 대조했다. 클립보드 구현은 실앱 측정 범위 밖이다.
  [#161](https://github.com/dev-universe/fender-studio-pro-mcp/pull/161)

다음은 Chorder Preview 눌림·해제와 도구 경계 확인이다.
`main` 20개·로컬 18개 잔여, 소프트웨어 2~5일, 장비 의존 4개,
지정된 종료 시각 없음.

## 20시 14분 체크포인트

- R371에서 Chorder의 DSP `playLearnChord` 0→1→0은 원래
  값으로 돌아왔지만 관찰한 출력 플래그는 바뀌지 않았다. 별도
  실제 편집기 하단 C3 누름은 열네 번의 독립 읽기에서 입력
  GUI velocity와 다섯 출력 활성값을 보였다. 해제 후 전부 0,
  DSP 908행과 저장 모델·컴포넌트 지문이 일치했다.
- 앱 180.897초, DSP 시험 owner 127요청과 편집기 재시험 owner
  148요청. 송 저장 없이 종료했으며 설정 9개와 설치 18개를
  복구했다. [#162](https://github.com/dev-universe/fender-studio-pro-mcp/pull/162)

다음은 MCP의 직접 Clear가 저장 모델을 변경하는지 확인하고
필요한 유한 도구를 구현하는 작업이다. `main` 20개·로컬 18개
잔여, 소프트웨어 2~5일, 장비 의존 4개, 지정된 종료 시각 없음.

## 20시 18분 체크포인트

- #136이 두 필수 검사를 통과해 `d39fe8f`에 병합됐다.
  Presence의 다음 공개 범위는 #137부터 확인한다.
  [#136](https://github.com/dev-universe/fender-studio-pro-mcp/pull/136)
- R372에서 학습 입력 MIDI 61을 선택한 뒤 DSP `chorder.clear`
  tag 9를 0→1→0으로 한 번 왕복했다. 1을 읽는 동안 전체 128개
  모델에서 선택된 61만 비었고, 0으로 내린 뒤에도 유지됐다.
  원본 컴포넌트와 DSP 908행이 복원되고 앱과 owner가 종료됐다.
  179요청, 앱 73.576초. 설정 9개·관리 설치 18개·송 원본 확인.
  [#163](https://github.com/dev-universe/fender-studio-pro-mcp/pull/163)

다음 이행 단위는 `feature/chorder-guarded-clear`의 선택 음정 및
모델 지문 일치 조건을 갖는 유한 MCP 호출이다. 분리 worktree를
`d39fe8f`에서 시작했다. `main` 20개·로컬 18개 잔여, 소프트웨어
2~5일, 장비 의존 4개, 지정된 종료 시각 없음.

## 20시 34분 체크포인트

- #137이 두 필수 검사 통과 후 `b6b16d5`에 병합됐다. #138은
  최신 베이스를 반영한 정확한 헤드에서 두 검사 실행 중이다.
  [#137](https://github.com/dev-universe/fender-studio-pro-mcp/pull/137)
- 보호형 `plugin_action.clear_chorder_selected` 구현을
  실제 8.1.1에서 확인했다. R373은 DSP의 Integer 종류가
  Float64 값을 갖는 실제 차이와 외부 거부 결과 변환에서 멈췄다.
  새 owner로 두 코드 모두 보존됐음을 확인했다.
- R373B는 기대 음정·지문·인스턴스·문서가 다른 네 요청을 실행
  전 거부했다. 정확한 요청만 MIDI 61을 비웠고 기존 MIDI 60 및
  나머지 입력, DSP 908행 및 원래 지문이 일치했다. 169요청
  32.257초, 앱 73.204초. 송 원본, 설정 9개, 관리 설치 18개 복원.
  [#164](https://github.com/dev-universe/fender-studio-pro-mcp/pull/164)

다음은 Chorder Clear All·Copy·Paste의 실제 MCP 동작 경계다.
`main` 20개·로컬 18개 잔여, 소프트웨어 2~5일, 장비 의존 4개,
지정된 종료 시각 없음.

## 20시 44분 체크포인트

- R374에서 DSP Copy, Paste, Clear All을 각각 실제로 실행했다.
  별도 전체 모델 읽기로 상대 음정 붙여넣기와 전체 지우기를 확인했고
  DSP 908행 및 원래 581바이트 상태가 돌아왔다. 278요청 48.863초,
  앱 74.533초. [#165](https://github.com/dev-universe/fender-studio-pro-mcp/pull/165)
- R375에서 편집기를 닫은 채 MCP로 MIDI 60 버튼에 해당하는 파라미터를
  눌렀다가 뗐다. 별도 읽기에서 출력 60·63·67·70·74가 모두 눌리고
  해제됨을 확인했다. 908행, 128개 코드, 상태 지문 및 믹서가 일치했다.
  91요청 8.854초, 앱 159.629초. 관리 설치 18개와 앱 설정 9개를
  복원했고 두 시험곡의 원본 바이트를 확인했다.
  [#166](https://github.com/dev-universe/fender-studio-pro-mcp/pull/166)

Learn 모드 전용 `playLearnChord`의 독립 출력 확인이 Chorder에 남았다.
`main` 20개·로컬 18개 잔여, 소프트웨어 2~5일, 장비 의존 4개,
지정된 종료 시각 없음.

## 20시 50분 체크포인트

- #138의 두 필수 검사와 최신 베이스를 확인해 `c1fe290`으로 병합했다.
  순서에 맞춰 #139의 새 헤드에서 검사가 실행 중이다.
  [#138](https://github.com/dev-universe/fender-studio-pro-mcp/pull/138)
- R376은 고유 UID 두 개의 Chorder를 직렬 연결했다. 앞쪽의 일반 60번
  건반과 Learn 모드 `playLearnChord`에서 각각 뒤쪽의 128개 입력 플래그를
  읽었다. 두 경우 모두 각각 두 번 60·63·67·70·74만 관찰됐고 해제 후
  모두 0이었다. 각 인스턴스의 908행, 전체 모델과 상태 지문이 원복됐다.
  211요청 16.973초, 앱 48.951초. 시험곡 원본 바이트, 관리 설치 18개,
  앱 설정 9개를 확인했다. [#167](https://github.com/dev-universe/fender-studio-pro-mcp/pull/167)

Chorder는 로컬 완료다. Scope는 R68, #117의 근거를 재사용해 캡처 시작과
완료를 구분할 관측과 비활성 C 매핑을 이어서 확인한다. `main` 20개·로컬
17개 잔여, 소프트웨어 2~5일, 장비 의존 4개, 지정된 종료 시각 없음.

## 20시 58분 체크포인트

- #139가 최신 베이스의 두 필수 검사를 통과해 `25ec952`에 병합됐다.
  검사와 원격 확인에 549.9초가 걸렸다. #140의 새 헤드에서 검사 중이다.
  [#139](https://github.com/dev-universe/fender-studio-pro-mcp/pull/139)
- R377은 Scope에서 C가 꺼졌을 때와 켜졌을 때, DSP `thirdSrc`로
  `Main Left`를 지정한 뒤에도 실제 선택 상자에 `Main Right`가 남음을
  확인했다. 31개 행, 믹서와 트랙을 대조하고 송 저장 없이 종료했다.
  42요청 51.335초, 앱 85.454초. 관리 설치 18개와 설정 10개를 복원했다.
  [#168](https://github.com/dev-universe/fender-studio-pro-mcp/pull/168)

다음은 독립 주파수 두 개가 든 자체 생성 스테레오 신호를 통한 C 파형
검증이다. `main` 20개·로컬 17개 잔여, 전체 2~5일, 장비 의존 4건,
지정 종료 시각 없음.

## 21시 08분 체크포인트

- #140이 552.7초 만에 두 필수 검사를 통과해 `d916085`로 병합됐다.
  Presence를 `main` 완료로 집계했다. [#140](https://github.com/dev-universe/fender-studio-pro-mcp/pull/140)
- R378에서는 소유 시험곡의 시작 위치가 4.423초라 14요청에서 멈췄다.
  준비 전 열었던 파일 선택기가 응답하지 않았다. R378B에서 위치를 0초로
  맞추고 17요청 후 새 선택기를 열었으나 선택 확인 중 다시 연결이 끊겼다.
  두 실행 모두 재생은 없었다. 앱은 각각 271.789초 및 109.216초에 소유
  프로세스만 종료했다. 원래 시험곡과 관리 설치를 확인했고 설정 및 새
  진단 로그를 정리했다.

다음은 파일 선택기 없이 Tone Generator와 Mixtool의 오른쪽 극성 반전을
사용해 Scope C 파형의 실제 입력을 확인한다. `main` 19개·로컬 17개 잔여,
전체 2~5일, 장비 의존 4건, 지정 종료 시각 없음.

## 21시 12분 체크포인트

- R379는 Scope 수직 오프셋 3500의 실제 내부값이 3499.999940395355라
  정확 비교에서 정지했다. 42요청·앱 39.787초. R379B는 정확히 표현되는
  2500과 7500을 사용했다.
- R379B의 C 파형은 `Main Left` 지정에서 왼쪽 기준과 일치했고
  `Main Right` 지정에서 오른쪽 기준과 일치했다. 화면 선택 상자는
  계속 `Main Right`를 표시했다. 사이드체인이 꺼진 상태에서 `Side Left`
  원시값 2의 파형도 오른쪽과 위상이 일치했다. 104요청 60.859초,
  앱 88.468초. 세 플러그인의 113개 행, 관리 설치 18개, 설정 10개와
  송 바이트를 확인했다. [#169](https://github.com/dev-universe/fender-studio-pro-mcp/pull/169)

다음은 무신호 단발 모드부터 임계값 이하 입력, 캡처, 홀드 및 한 번의
재트리거까지 검증하는 작업이다. `main` 19개·로컬 17개 잔여,
전체 2~5일, 장비 의존 4건, 지정 종료 시각 없음.

## 21시 17분 Scope 단발 검증

- R380은 네이티브 포매터에서 원시값 6250이 37.5%임을 확인하고
  신호를 켜기 전에 종료했다. R380B는 실제 포맷을 사용했다.
- R380B에서 무신호로 단발 모드를 켠 뒤 37.5% 임계값보다 낮은
  440 Hz, −24 dB를 넣어도 A 파형은 평탄했다. 880 Hz, −7.50 dB
  입력 뒤 처음 파형이 나타났고, 440 Hz로 바꿔도 유지됐다. 한 번의
  Retrigger 뒤 새 파형이 나타났다. 41개 DSP 행과 평탄한 표시를 복원했다.
  90요청 91.337초, 앱 117.776초. [#170](https://github.com/dev-universe/fender-studio-pro-mcp/pull/170)
- #141이 두 필수 검사를 통과한 뒤 `b87fb3e`로 병합됐다. 대기열 단계는
  554.1초였다. #142는 최신 헤드에서 검사 중이다.
  [#141](https://github.com/dev-universe/fender-studio-pro-mcp/pull/141)

다음은 서로 다른 두 홀드 파형과 컴포넌트 캡슐 두 종류를 대조하고 첫
캡슐의 복원 결과를 별도 화면에서 읽는 R381이다. `main` 19개·로컬 17개
잔여, 전체 2~5일, 장비 의존 4건, 지정 종료 시각 없음.

## 21시 25분 체크포인트

- R381에서 440 Hz 및 880 Hz로 만든 두 다른 홀드의 Scope DSP 31행과
  762바이트 캡슐 지문이 일치했다. Tone을 꺼도 880 Hz 입력에서 얻은
  홀드가 유지됐지만 첫 캡슐을 복원하니 평탄해졌다. 단발 모드와 임계값,
  전체 41행을 확인했다. 토큰 5개를 해제하고 관리 설치 18개, 설정 10개,
  송 원본을 대조했다. 99요청 64.265초, 앱 86.820초.
  [#171](https://github.com/dev-universe/fender-studio-pro-mcp/pull/171)
- #142가 두 필수 검사 후 556.3초에 `112e571`로 병합됐다. #143은 최신
  베이스의 검사 중이다. [#142](https://github.com/dev-universe/fender-studio-pro-mcp/pull/142)

Scope 캡슐 응답에 파형 범위를 명시하는 `feature/scope-state-coverage`
후보를 분리했고 다음 실제 R382에서 Scope와 별도 피어 효과를 대조한다.
`main` 19개·로컬 17개 잔여, 전체 2~5일, 장비 의존 4건, 지정 종료 시각 없음.

## 21시 37분 체크포인트

- R382가 Scope의 `scope_state_coverage` 응답, Tone Generator에서의 필드
  부재와 토큰 4개 해제를 확인했다. 42요청 4.107초, 앱 31.025초.
  네이티브 505개, 핵심 Rust 260개, JS 438개, Python 560개가 통과했다.
  두 플러그인의 원래 41행, 관리 파일 18개와 설정 10개를 대조했다.
  [#172](https://github.com/dev-universe/fender-studio-pro-mcp/pull/172)
- #143이 두 검사 후 550.3초에 `01e62d1`로 병합됐다. #144의 새
  헤드에서 검사 중이다. [#143](https://github.com/dev-universe/fender-studio-pro-mcp/pull/143)
- R383에서 모니터가 꺼진 상태의 Input Filter GUI 입력 7개는 자체
  눌림·해제가 관측됐고 별도 Chorder에선 입력이 잡히지 않았다.
  R384는 소유 가상 MIDI에서 동일 7음을 입력했다. 기본 범위 48~72의
  48·60·72만 다음 Chorder에서 읽혔다. 모두 해제되고 1296행을 원복했다.
  399요청 50.039초, 앱 80.733초. 임시 MIDI 설정과 소스도 정리했다.
  [#173](https://github.com/dev-universe/fender-studio-pro-mcp/pull/173)

다음은 R385의 모니터 활성 GUI 입력이다. 공통 Chorder audition은
`feature/chorder-audition-lifecycle`에서 문서·인스턴스와 일회용 토큰을
묶어 구현 중이다. `main` 19개·로컬 17개 잔여, 전체 2~5일,
장비 의존 4건, 지정 종료 시각 없음.

## 21시 44분 Input Filter 완료

- R385는 동일 7개 GUI 입력을 모니터 활성 상태에서 대조해 범위 안의
  48·60·72가 두 번째 Chorder에 전달됐음을 확인했다. 240요청 19.527초,
  앱 59.616초. 1296행·설치 18개·설정 9개 및 시험곡 원본을 확인했다.
  [#174](https://github.com/dev-universe/fender-studio-pro-mcp/pull/174)
- R386은 범위를 0~127로 넓혀 모든 GUI 입력의 정확한 MIDI 음정 전달과
  해제를 두 개의 독립 읽기로 확인했다. 1615요청 123.855초, 앱 154.160초.
  범위 48~72, 모니터, 1296행, 시험곡과 설치 18개·설정 9개를 복원했다.
  [#175](https://github.com/dev-universe/fender-studio-pro-mcp/pull/175)

Input Filter를 로컬 완료로 집계했다. 다음은 Rumble Native 앰프 위치와
앰프·튜너 바이패스 제어를 실제로 대조한다. `main` 19개·로컬 16개 잔여,
전체 2~5일, 장비 의존 4건, 지정 종료 시각 없음.

## 21시 52분 Rumble 제어 체크포인트

- #144는 정확한 베이스와 헤드에서 두 검사를 통과해 `b06e650`로
  병합됐다. 다음 순번은 #145다.
  [#144](https://github.com/dev-universe/fender-studio-pro-mcp/pull/144)
- R387은 연속 읽기의 전체 캡처 해시 차이로 해당 DSP 쓰기 전에
  중단했다. 39 DSP 행과 투영된 모델·설정은 일치했다. 앱 49.205초.
- R387B는 앰프 위치 0→3→0, 앰프 바이패스 0→1→0, 튜너 바이패스
  1→0→1을 두 번씩 읽었다. 직접 쓰기로 튜너 화면이 열렸다 닫혔다.
  빈 체인의 DSP 확인만 확보했다. 26 슬롯, 22 설정, 39 DSP 행,
  원본 시험곡과 설치 18개·설정 10개를 확인했다. 요청 66건 73.686초,
  앱 116.960초. [#176](https://github.com/dev-universe/fender-studio-pro-mcp/pull/176)

다음은 Chorder 공통 permit의 실제 앱 검증이다. Rumble의 활성 앰프와
스톰프 모델 선택·배치·효과는 남아 있다. `main` 19개·로컬 16개,
전체 2~5일, 장비 의존 4건, 지정 종료 시각 없음.

## 21시 56분 공통 이벤트 수명 완료

- R388의 정확한 후보 `ad1a9c8`에서 같은 소유자·문서·인스턴스를
  묶는 MIDI 60 permit을 실제로 검증했다. 별도 눌림·중립 읽기,
  중복 기록, 인스턴스·문서 불일치, 교체 및 만료 거부를 확인했다.
- 99요청 24.151초, 앱 55.310초. 908개 DSP 값, 코드 모델 128개와
  581바이트 캡처, 시험곡, 관리 파일 18개, 설정 9개가 정확히 돌아왔다.
  누름·해제 응답 손실과 격리는 자동 검사로 대조했다. 이 실행의 공개
  성공 범위는 로컬 키이며 R376의 별도 후단 관측과 구분한다.
  [#177](https://github.com/dev-universe/fender-studio-pro-mcp/pull/177)

공통 이벤트 수명을 로컬 완료로 집계했다. 다음은 Rumble Add Block
및 활성 앰프·스톰프 배치다. `main` 19개·로컬 15개 잔여, 전체
2~5일, 장비 의존 4건, 지정 종료 시각 없음.

## 22시 06분 Rumble 모델과 내부 신호

- #145가 최신 베이스의 정확한 헤드에서 두 검사를 통과해
  `d5511f5`에 병합됐다. #146부터 순서대로 검사한다.
  [#145](https://github.com/dev-universe/fender-studio-pro-mcp/pull/145)
- R389에서 Add Block의 위치와 Add 화면은 열렸고 선택 행이나 범주는
  표시되지 않았다. 적용 없이 닫았다. 설치 파일의 전체 앰프 배치 26가지와
  111개 공장 프리셋이 참조하는 앰프 17종·이펙트 38종을 대조했다.
  요청 25건 37.676초, 앱 76.642초.
  [#178](https://github.com/dev-universe/fender-studio-pro-mcp/pull/178)
- R390의 파라미터 이름 조회 응답에는 인스턴스 필드가 없어 신호 전에
  중단했다. 앱 52.844초. R390B는 인스턴스가 있는 전체 페이지로
  440 Hz 신호를 확인했다. 앰프 바이패스·위치별 입력과 출력 피크는
  각 표본에서 같았고 조건 간 범위가 겹쳤다. 튜너 음 이름은 표시되지
  않았다. 피크 감쇠 뒤 별도 읽기에서 0과 49개 DSP 행, 26개 슬롯과
  22개 설정 원복을 확인했다. 102+10요청, 40.241+2.163초, 앱 122.630초.
  두 시험의 곡 원본과 설치 18개·설정 10개를 확인했다.
  [#179](https://github.com/dev-universe/fender-studio-pro-mcp/pull/179)

다음은 저장된 Multi Instrument의 매크로와 실제 자식 파라미터를
대조하는 시험이다. `main` 19개·로컬 15개 잔여, 전체 2~5일,
장비 의존 4건, 지정 종료 시각 없음.

## 22시 19분 Multi 매크로 복구 체크포인트

- #146은 두 필수 검사 후 정확한 헤드에서 `3ddfb0a`에 병합됐다.
  [#146](https://github.com/dev-universe/fender-studio-pro-mcp/pull/146)
- R391에서 저장된 경로의 Device ID와 새 실행에서의 Device ID가
  달라 쓰기 전에 중단했다. R391B는 같은 슬롯과 현재 Device를 다시
  묶었다. 매크로 20개의 저장 대상 30개 중 Cutoff, Synth 1, Amount를
  실기했다. 716개 값이 정확히 복구됐다. 196요청 15.375초, 앱 62.553초.
  [#180](https://github.com/dev-universe/fender-studio-pro-mcp/pull/180)
- R392에서 Resonance는 양쪽 자식과 부모를 복원했다. Filter Env는
  원래 부모 비트를 되돌려도 두 자식 값이 5003.644824028015 대신
  5449.993014335632였다. 127요청 10.051초, 앱 90.543초.
  [#181](https://github.com/dev-universe/fender-studio-pro-mcp/pull/181)
- R393에서 원래 캡슐을 적용하니 566 자식 행은 돌아왔다. 두 자식
  Device ID가 교체됐고 부모 150행 중 Filter Env만 다른 값이 됐다.
  토큰 4개 해제, 80요청 7.166초, 앱 44.483초.
  [#182](https://github.com/dev-universe/fender-studio-pro-mcp/pull/182)

R391–R393의 종료 코드, 시험곡 원본, 관리 파일 18개와 설정 9개를
대조했다. 다음은 Multi의 가상 MIDI 전달과 유한 자식 값 복구 설계다.
`main` 19개·로컬 15개, 전체 2~5일, 장비 의존 4건, 지정 종료 시각 없음.

## 22시 26분 Multi 가상 MIDI 전달

- #147이 두 필수 검사 후 `f8e3faf`에 병합됐다.
  [#147](https://github.com/dev-universe/fender-studio-pro-mcp/pull/147)
- R394는 다른 시험곡의 해시를 사용해 앱과 가상 소스를 열기 전에
  중단했다. 설정 36개가 그대로였고 관리 설치를 원복했다.
- R394B는 자체 가상 소스와 실제 모든 입력 경로에서 MIDI 압력,
  CC1·2·11·64와 피치 벤드가 부모와 두 Mai Tai에 전달됨을 확인했다.
  압력과 CC 네 개는 정확히 원복됐다. 피치 벤드의 중앙은
  5000.30517578125였다. 별도 부모 쓰기 뒤 부모 150행은 돌아왔고
  자식 566행 가운데 피치 벤드 두 행만 차이가 남았다.
- 297요청 84.021초와 복구 대조 57요청 5.453초. 앱 197.996초.
  원본 시험곡, 설치 18개, MIDI 파일을 포함한 설정 10개를 원복했다.
  두 owner와 임시 소스가 정상 종료했다.
  [#183](https://github.com/dev-universe/fender-studio-pro-mcp/pull/183)

다음은 나머지 일반 CC와 한 자식 Filter Env 복구 도구다. 공통 Float
비교를 재사용하고 각 요청을 슬롯 하나에 묶는다. `main` 19개·로컬
15개 잔여, 전체 2~5일, 장비 의존 4건, 지정 종료 시각 없음.

## 22시 34분 일반 MIDI CC와 지원표

- #148이 정확한 최신 베이스의 헤드에서 두 검사를 통과해
  `9a5f5b8`에 병합됐다. [#148](https://github.com/dev-universe/fender-studio-pro-mcp/pull/148)
- 중첩 인서트의 추가·이동·삭제와 Undo·재열기 범위를 기존 검증에 맞게
  지원표에 반영했다. [#184](https://github.com/dev-universe/fender-studio-pro-mcp/pull/184)
- R395는 나머지 일반 CC 116개의 독립 활성·중립과 매 회 부모 150행
  원복을 확인했다. R394B와 합쳐 0~119 전체다. 두 자식의 게인만 0으로
  남아 별도 owner가 할당된 Vol 1·2의 원래 비트를 사용해 복원했다.
  1411요청 185.817초와 83요청 7.509초, 앱 288.927초, 716행 원복.
  시험곡과 설치 18개·MIDI 포함 설정 10개를 확인했다.
  [#185](https://github.com/dev-universe/fender-studio-pro-mcp/pull/185)

다음은 MIDI 채널 모드와 Multi 자식 Filter Env 복구 후보의 실제 검증이다.
`main` 19개·로컬 15개 잔여, 전체 2~5일, 장비 의존 4건,
지정 종료 시각 없음.

## 22시 41분 Multi 채널 모드와 Filter Env 복구

- R397은 채널 모드 메시지를 가상 입력으로 전달했다. CC122·126은
  부모 값만 변했고 CC121 뒤에는 부모·두 자식의 CC1 값이 유지됐다.
  명시적 CC1=0으로 돌아왔고 최종 716행이 일치했다. 378요청 71.777초,
  앱 122.253초. 시험곡과 18개 관리 파일, MIDI 포함 설정 10개를
  복구했다. [#186](https://github.com/dev-universe/fender-studio-pro-mcp/pull/186)
- R396 후보 `fcf91fd`의 새 `plugin_multi_child_filter_env_set`을
  테스트했다. 정확한 이전 비트를 확인해 자식을 한 대씩 복구했다.
  각 단계의 부모·피어 값을 따로 읽었다. 최종 716행과 두 Device ID가
  일치했다. 잘못된 문서·부모·자식·비트와 재전송은 거부했다.
  144요청 11.934초, 앱 53.025초, 설치 18개·설정 9개 및 오프라인
  Rust 271/Native 506/JS 443개 검사를 확인했다.
  [#187](https://github.com/dev-universe/fender-studio-pro-mcp/pull/187)
- #187은 공통 비교 쓰기의 #177 뒤에 순서대로 병합한다. 리뷰 브랜치
  `feature/multi-child-filter-env-release`의 구현 파일이 R396 후보와
  같음을 대조했다.

다음은 나머지 매크로 14개와 별도 Filter Decay 시험이다.
`main` 19개·로컬 15개 잔여, 전체 2~5일, 장비 의존 4건, 지정
종료 시각 없음.

## 22시 46분 Multi 매크로 전체 변경 확인

- #149가 두 검사를 통과해 `7f90198`에 병합됐다.
  [#149](https://github.com/dev-universe/fender-studio-pro-mcp/pull/149)
- R398에서 Spread·Vol·온오프·패드 축 등 14개의 정확한 매핑과
  매번 716행 원복을 확인했다. 앞선 시험을 합쳐 19개가 정확히 복구됐다.
  790요청 59.934초, 앱 112.570초.
  [#188](https://github.com/dev-universe/fender-studio-pro-mcp/pull/188)
- R399는 Filter Decay가 두 자식의 `adsr2.decay`를 바꾼다는 점을
  확인했다. 부모가 돌아와도 자식 두 값은 원래
  `3FAE90831A900000` 대신 `3FAE908404F00000`였다.
  85요청 7.328초, 앱 69.482초. 저장 없이 닫아 원본과 설치 18개,
  설정 9개를 검증했다. [#189](https://github.com/dev-universe/fender-studio-pro-mcp/pull/189)

다음은 Filter Decay 캡슐 복원이다. `main` 19개·로컬 15개 잔여,
전체 2~5일, 장비 의존 4건, 지정 종료 시각 없음.

## 22시 52분 Multi 매크로 20개 DSP 복구

- R400의 캡슐은 원래 Filter Decay와 자식 전체를 복구했다. 두 새
  Device가 생겼고 부모 Filter Env만 다르게 남았다. 80요청 7.103초,
  앱 79.560초.
- R401은 캡슐 뒤 부모 Filter Env를 되돌리고 새로 바인딩한 자식을
  한 대씩 복원했다. 이전 Device ID를 사용한 요청은 거부했다.
  716행의 두 독립 읽기가 일치했다. 마지막 직렬화 캡처 지문은
  달랐다. 176요청 14.166초, 앱 66.508초. 두 시험에서 토큰 9개를
  해제하고 원본 시험곡과 관리 파일 18개·설정 9개를 확인했다.
  [#190](https://github.com/dev-universe/fender-studio-pro-mcp/pull/190)

매크로 20개의 실제 대상과 정확한 DSP값 복원 범위를 확보했다.
Multi 일반 그래프 수정은 열린 상태다. 다음은 Impact의 다른 패드
Focus 대상 시험이다. `main` 19개·로컬 15개 잔여, 전체 2~5일,
장비 의존 4건, 지정 종료 시각 없음.

## 23시 03분 Impact MIDI Pad Focus

- R402는 Pad Focus를 끈 상태에서 C♯1을 화면으로 선택했다. 전체
  제공값은 그대로였으며 Focus cutoff 쓰기는 alias와 pad0에만 적용됐다.
  261요청 49.840초, 앱 222.923초. 저장 없이 닫고 원본을 대조했다.
- R403은 Pad Focus를 켜고 실제 Studio Pro의 모든 입력에 가상 CoreMIDI
  노트 36·37·53·36을 보냈다. 패드 1·17의 cutoff와 Focus값은 Float32
  정규화 비트가 일치하고 원시 Float64는 달랐다. 뱅크 B F2 선택과
  뱅크 A C1 복귀, 4,262 DSP행의 정확한 복구를 확인했다. 661요청
  137.225초, 앱 174.944초. 두 시험에서 관리 파일 18개와 각 설정
  9개·10개를 확인했다. [#191](https://github.com/dev-universe/fender-studio-pro-mcp/pull/191)
- [#150](https://github.com/dev-universe/fender-studio-pro-mcp/pull/150)은
  현재 main을 반영한 헤드에서 두 검사 통과 후 `a3c8cec`로 병합됐다.

다음은 이진 분수 cutoff를 이용한 두 MIDI 선택 패드의 별도 쓰기와
복원이다. 샘플의 note-on/off 처리와 저장 후 다시 열기도 남았다.
`main` 19개·로컬 15개 잔여, 전체 2~5일, 장비 의존 4건,
지정 종료 시각 없음.

## 23시 09분 Impact MIDI 선택 패드 쓰기

- R404의 MIDI 노트 37·53이 선택한 패드 1·17에서 cutoff는 각각
  2500→5000→2500, 7500→5000→7500이었다. 독립 읽기 두 번에서
  Focus와 해당 indexed cutoff만 바뀌었다. 뱅크 A C1로 돌아와
  4,262행과 원본 파일, 관리 파일 18개, 프로필 10개를 확인했다.
  1,015요청 203.668초, 앱 255.751초.
  [#192](https://github.com/dev-universe/fender-studio-pro-mcp/pull/192)
- [#151](https://github.com/dev-universe/fender-studio-pro-mcp/pull/151)은
  현재 main을 반영한 헤드에서 필수 두 검사 통과 후 `353c34f`로 병합됐다.

다음 R405는 생성한 WAV를 사용한 Impact의 downstream note-on/off
처리다. 검증 잔여는 `main` 19개·로컬 15개, 전체 2~5일,
장비 의존 4건, 지정 종료 시각 없음.

## 23시 20분 Impact 샘플의 노트 수명

- R405는 빈 패드의 무신호와 샘플 패드의 첫 양수 피크를 확인했다.
  계측기의 원시 범위를 0–1로 잘못 검사해 첫 시도는 중단했다.
  195요청 223.052초, 앱 318.094초.
- R405B는 14초 440 Hz WAV를 C1에 넣고 Normal을 사용했다. 빈
  C♯1과 대기 시 네 피크는 0, C1 약 1.880초 유지 중 최소
  220.7846이었다. note-off 뒤 8.153초 내 마지막 두 값은
  유지값의 백만분의 일 아래였다. 같은 DAW 세션의 별도 읽기
  전용 MCP owner는 네 피크가 정확히 0이고 로드 직후 DSP 4,262행과
  믹서·타임라인이 그대로임을 확인했다. 234요청 176.329초와
  별도 78요청 10.908초, 앱 296.922초. 둘 다 저장 없이 닫고
  원본과 관리 파일 18개, 설정 11개를 대조했다.
  [#193](https://github.com/dev-universe/fender-studio-pro-mcp/pull/193)
- [#152](https://github.com/dev-universe/fender-studio-pro-mcp/pull/152)는
  최신 main 반영 헤드에서 필수 두 검사 후 `c7139ac`로 병합됐다.

다음은 R406의 소유 시험곡 저장과 새 앱·MCP 바인딩이다.
`main` 19개·로컬 15개 잔여, 전체 2~5일, 장비 의존 4건,
지정 종료 시각 없음.

## 23시 27분 Impact 저장 및 새 세션 복원

- R406은 C1에 넣은 14초 WAV의 프레임 0–672000과 Normal 트리거,
  C♯1의 2500 cutoff 값을 저장했다. 새 Studio Pro와 MCP 문서에서
  같은 저장 UUID를 바인딩하고 이전 문서 요청을 거부했다. 화면,
  독립 DSP 4,262행, 36,042바이트 구성요소 지문이 모두 일치했다.
  WAV 참조가 외부 원래 경로에서 계속 열렸다. 이 파일의 이식성은
  별도다. 저장 310요청 120.242초, 재열기 155요청 56.112초,
  앱 199.890초와 100.545초. 토큰 6개를 해제했고 두 번째 저장 없이
  닫아 저장본과 원래 시험 재료, 관리 파일 18개와 설정 9개를 확인했다.
  [#194](https://github.com/dev-universe/fender-studio-pro-mcp/pull/194)
- Impact의 다른 패드 Focus, 샘플 note-on/off, 저장/재열기 고정
  조건을 로컬 충족했다. PR #191–194는 최신 main 검사를 기다린다.
- [#153](https://github.com/dev-universe/fender-studio-pro-mcp/pull/153)은
  두 검사 통과 후 `da498a5`로 병합됐다.

다음은 Multi Instrument와 공통 그래프의 정확한 소유권과 기존
편집자 범위를 비교한 뒤 가장 작은 새 실제 변경을 검증하는 일이다.
`main` 19개·로컬 14개 잔여, 전체 2~5일, 장비 의존 4건,
지정 종료 시각 없음.

## 11월 26일 01시 16분 Multi 실제 편집과 직렬 Splitter

- R407은 Multi Routing에서 두 번째 Mai Tai 한 대를 추가했다. 그래프
  3→7 노드와 별도 슬롯·Device를 읽고, 기존 부모 150행과 자식 283행,
  믹서가 그대로임을 확인했다. Undo 한 번 뒤 원래 노드 레코드 셋과
  바인딩·값이 돌아왔다. controller 읽기 순서는 달라졌다.
  124요청 167.581초, 앱 237.633초. 원본, 설치 18개, 설정 10개 확인.
  [#195](https://github.com/dev-universe/fender-studio-pro-mcp/pull/195)
- R408은 연속한 두 Mixtool 사이 선 한 지점에 Splitter를 드롭했다.
  두 독립 읽기에서 6노드·7연결과 두 Mixtool의 각 72행, 소유권이
  동일했다. 같은 상태의 캡처 지문은 시험 전에도 달랐다. Undo와 저장
  없이 닫고 토큰 6개, 원본, 설치 18개, 설정 9개를 확인했다.
  89요청 157.743초, 앱 305.765초.
  [#196](https://github.com/dev-universe/fender-studio-pro-mcp/pull/196)
- #154, #155, #156이 최신 베이스의 각 정확한 헤드에서 두 필수 검사를
  통과해 병합됐다. 관측된 main은 `6115eda`다. [#154](https://github.com/dev-universe/fender-studio-pro-mcp/pull/154) [#155](https://github.com/dev-universe/fender-studio-pro-mcp/pull/155) [#156](https://github.com/dev-universe/fender-studio-pro-mcp/pull/156)

다음은 R409의 독립 시험곡 한 번 저장과 재실행이다. 저장 그래프의
successor 연결을 라이브 노드·슬롯 및 새 문서 바인딩과 대조한다.
`main` 19개·로컬 14개, 전체 2~5일, 장비 의존 4건,
지정 종료 시각 없음.

## 11월 26일 01시 24분 Multi 편집 그래프 저장과 재열기

- R409는 Routing에서 Mai Tai 한 대를 추가하고 독립 시험곡을 한 번
  저장했다. 7노드·6 successor 연결, 기존 슬롯 UUID와 순서가 유지됐고
  재실행 GUI에도 두 병렬 가지가 보였다. 새 세션에서 두 Device가 다시
  바인딩됐고 이전 문서는 거부됐다. 자식 566행과 부모 149/150행 일치.
- 부모 Filter Env만 86.0%→63.6%였다. 20개 매크로 정의의 비교에서
  저장 시 Filter Env pilot 하나가 바뀌었다. 새 값은 편집 전부터
  유지된 Inst01의 유일한 할당 대상 값과 비트 단위로 같다. 재열기
  값도 저장된 pilot과 같다.
- 저장 134요청 106.567초, 첫 재열기 읽기 49요청 4.500초와 별도
  owner 94요청 15.271초. 앱 137.236초 및 98.497초. 두 앱과 세
  owner 종료, 원본 및 저장본 해시, 관리 설치 18개와 설정 10개 확인.
  [#197](https://github.com/dev-universe/fender-studio-pro-mcp/pull/197)
- #157의 서버 측 브랜치 업데이트가 GraphQL 오류로 중단됐다.
  기존 격리 작업 디렉터리에서 최신 `main`을 합쳐 새 헤드
  `0b36ebc`에 두 검사를 다시 시작했다.

다음은 자식을 추가하기 전 한 대 구성을 저장해 같은 pilot 변경이
나타나는지 확인하는 대조시험과 Multi 편집의 실제 호출 경로 확인이다.
`main` 19개·로컬 14개, 전체 2~5일, 장비 의존 4건,
지정 종료 시각 없음.

## 11월 26일 01시 33분 Multi 저장 대조와 삽입 호출

- R410은 같은 한 대 구성에서 편집기와 악기 추가 없이 저장했다.
  새 파일 해시에서 기존 3노드·2연결과 모든 매크로 20개가
  유지됐다. Filter Env의 직렬화 pilot은 86.0% 부모와 일치했고,
  대상 자식은 63.6%였다. 라이브 433행·믹서·소유권도 정확했다.
  75요청 17.564초, 앱 70.113초, 관리 파일 18개와 설정 9개 확인.
  [#198](https://github.com/dev-universe/fender-studio-pro-mcp/pull/198)
- R411은 REA와 Ghidra로 실제 설치 arm64 슬라이스를 대조했다.
  `CombinatorSynthFolder`의 삽입 재정의와 별도로 등록된 그래프
  편집 부수 동작을 확인했다. 공개 문서는 시그니처와 관찰 범위를
  담는다. 로컬 보관본은 `multi-synth-folder-static-r411`에 있고,
  REA 세션을 정리했다.
  [#199](https://github.com/dev-universe/fender-studio-pro-mcp/pull/199)

다음은 R412의 고정 read-only 스크립트 receiver 확인과 R413의 한
레이어 트랜스포즈 독립 시험이다. `main` 19개·로컬 14개, 전체
2~5일, 장비 의존 4건, 지정 종료 시각 없음.

## 11월 26일 01시 35분 SampleOne 병합과 설치 모듈 확인

- #157의 최신 main 반영 헤드 `0b36ebc`에서 두 검사 통과를 확인해
  `8509891`에 병합했다. SampleOne 고정 검증 항목이 main에서
  충족됐다. [#157](https://github.com/dev-universe/fender-studio-pro-mcp/pull/157)
- #158은 갱신 헤드 `67d9af6`에서 두 검사 진행 중이다.
  로컬 병합 대기열을 #199까지 갱신했다.
- 실제 설치 앱의 Mustang/Rumble 공용 모델 모듈에서 arm64 서명이
  빠진 현상이 현재도 이어진다. R344의 실제 로더 실패와 빈 카탈로그
  근거를 유지한다. Volt 276·Keystation88도 열거 결과에 없었다.
- 공개용 macOS 아카이브를 별도 프로필에서 검토 중이다. 첫 빌드에
  로컬 절대 경로와 해당 경로를 가진 Mach-O install name이 있어
  공개 자산 후보에서 제외했다. 경로 제거 빌드를 대조 중이다.

`main` 18개·로컬 14개, 전체 소프트웨어 2~5일, 장비 의존 4건,
지정 종료 시각 없음.

## 11월 26일 01시 44분 Multi 레이어와 배포 파일 경로

- R413에서 원래 왼쪽 MaiTai만 +12 반음, 피어는 0으로 유지했다.
  실제 한 번 저장한 그래프는 해당 Transformer 하나만 바뀌고
  노드 7개·순서가 있는 연결 6개와 두 경로가 그대로였다. 새 세션과
  문서에서 화면, 바인딩과 DSP 716개 일치 및 이전 문서 거부를
  확인했다. 저장 142요청 133.229초와 재열기 94요청 24.954초,
  앱 159.986·67.435초. 원본과 저장본, 설치 18개·설정 9개 확인.
  [#200](https://github.com/dev-universe/fender-studio-pro-mcp/pull/200)
- R412에서 네이티브 소유권을 별도 조회로 확인했다. 네 개의 브리지
  경로, EditTask의 두 내부 폴더 경로와 그래프 전용 factory는
  해당 폴더를 노출하지 않았다. 일반 DeviceEditFunctions는 있었다.
  DSP 433개·믹서·파일이 유지됐다. 27요청 3.678초, 앱 50.628초,
  설치 18개·설정 9개 확인. [#202](https://github.com/dev-universe/fender-studio-pro-mcp/pull/202)
- 두 arm64 배포 바이너리의 비공개 홈·소스·Cargo·Rust 경로와
  네이티브 절대 install name을 제거했다. 클린 빌드 약 77초,
  체크섬·dSYM UUID·두 진입점, 별도 로드와 격리 설치·제거를 확인했다.
  공개 자산, 리소스 서명, Developer ID와 Gatekeeper 검증은 별도다.
  [#201](https://github.com/dev-universe/fender-studio-pro-mcp/pull/201)

다음 R414는 정확한 폴더에서 고정 네이티브 메서드 시험을 준비한다.
기존 역할과 실제 Graph 편입·Undo를 대조해야 완료다. macOS는
공개 아카이브에 들어가는 자체 번들의 리소스 서명을 검증한다.
`main` 18개·로컬 14개, 전체 2~5일, 장비 의존 4건,
지정 종료 시각 없음.

## 11월 26일 01시 57분 Ampire Undo와 Splitter 대조

- R416은 MCM 800·4x12 M65의 Mic B 실제 페이더를 움직이고
  정확히 해당 Undo 한 번을 실행했다. 화면은 80·10·10%이고
  네이티브 B는 원복됐다. A와 C 두 행과 컴포넌트 지문은
  달랐다. 60요청 112.004초, 앱 150.141초. 토큰 3개와 원본,
  설치 18개·설정 10개 확인. [#204](https://github.com/dev-universe/fender-studio-pro-mcp/pull/204)
- R415는 비교 코드의 UUID 대소문자 차이로 편집 전에 멈췄다.
  실제 응답 두 앵커를 정규화해 읽은 결과는 같았다. 새 R415B에서
  B와 바로 다음 안쪽 Merger의 실제 연결에 한 번 드롭했다.
  드롭 표식은 보이지 않았다. 전후 노드 6개·순서 연결 7개·
  두 DSP 144행이 동일했고 Undo 없이 종료했다. 32·97요청,
  앱 94.172·107.578초, 캡처 8개와 원본·설치 18개·설정 9개 확인.
  [#205](https://github.com/dev-universe/fender-studio-pro-mcp/pull/205)
- 공개 후보 네이티브 번들에 ad-hoc 리소스 서명을 추가했다.
  독립과 #201 결합 빌드, 합계 21회 격리 CLI 수명주기로
  올바른 현재 CLI가 예전 설치를 정확히 복원할 수 있음을 확인했다.
  Developer ID·Gatekeeper와 공개 자산 자체는 열려 있다.
  [#203](https://github.com/dev-universe/fender-studio-pro-mcp/pull/203)
- #158이 두 검사 후 `4bb22c9`에 병합됐다. #159 뒤 #201·#203을
  먼저 검사해 실제 공개 패키지 경로를 열고 나머지 PR을 이어간다.
  [#158](https://github.com/dev-universe/fender-studio-pro-mcp/pull/158)

다음은 R414의 네이티브 단일 추가와 독립 그래프 대조다. `main`
18개·로컬 14개, 전체 2~5일, 장비 의존 4건, 지정 종료 시각 없음.

## 11월 26일 02시 06분 Multi 네이티브 추가와 Undo

- R414의 정확한 네이티브 `IObject` 메서드 호출 한 번은
  `applied_unverified`로 반환됐다. 독립 기록에서 3→7 노드,
  두 exact Slot 연결, 새 283행, 기존 433행과 원래 믹서·트랙을
  확인했다. 화면에 두 병렬 가지와 해당 Mai Tai 추가 Undo가
  나타났다. Undo 한 번으로 원래 세 노드 집합, 바인딩, 433행과
  믹서가 정확히 돌아왔다.
- 158요청 124.970초, 앱 213.611초. 정상 종료 뒤 원본과 복사본,
  설치 18개·설정 10개 확인. 비공개 후보 `b66b773`으로 확인한
  범위는 열린 Routing 편집기의 Mai Tai 1→2다.
  [#206](https://github.com/dev-universe/fender-studio-pro-mcp/pull/206)
- 공개 `plugin_multi_add_mai_tai` 구현과 독립 R417 검증을 준비한다.
  편집기 닫힘, 잘못된 문서와 각 UUID, 중복 요청, 독립 저장·재시작을
  차례로 비교한다. #159는 Core 통과, macOS job이 시작 전에 취소돼
  같은 헤드의 해당 검사를 다시 시작했다.

`main` 18개·로컬 14개, 전체 예상 2~5일, 장비 의존 4건,
지정 종료 시각 없음.

## 11월 26일 02시 09분 Scope의 활성 사이드체인

- R419는 자체 440 Hz Tone Generator와 오른쪽만 반전한 Mixtool을
  프리페이더 Send로 Scope에 보냈다. 정지·Main 뮤트·0 상태였다.
  C가 raw 2이면 Side Left와, raw 3이면 Side Right와 같은 위상을
  보였다. raw 3에서도 C 화면 글자는 Side Left였다. Tone을 끄고
  Send를 제거한 뒤 DSP 113행을 정확히 원복했다.
- 138요청 132.649초, 앱 190.765초. 소유 원본과 시험곡, 설치
  18개·설정 10개 원복. 실제 완료 시각과 숨은 파형은 별도다.
  [#207](https://github.com/dev-universe/fender-studio-pro-mcp/pull/207)

Multi 공개 후보의 오프라인 검사와 R417을 이어간다. #159의 재실행은
Core 성공과 같은 헤드의 macOS 검사를 진행 중이다. `main` 18개·
로컬 14개, 전체 2~5일, 장비 의존 4건, 지정 종료 시각 없음.

## 11월 26일 02시 21분 공개 Multi 도구와 Chorder 병합

- R417은 Studio Pro 8.1.1 arm64의 닫힌 편집기에서 공개
  `plugin_multi_add_mai_tai`를 실제 실행했다. 문서 및 UID가
  잘못된 여섯 요청과 두 레이어 상태의 반복 요청은 실행 전에 거부됐다.
  두 병렬 분기, 원래 433행과 원래 믹서, 새 DSP 283행을 독립 조회했다.
  한 번 저장한 파일의 두 분기·7노드·순서 연결 6개와 기존 슬롯 UUID,
  경로를 새 앱에서 대조했다. 자식 566행은 모두 일치했다. 부모
  150행 중 Filter Env 한 행은 파일에 기록된 원래 자식의 값으로 바뀌었다.
  이전 문서를 거부했다. 646요청 235.185초, 재열기 137요청 21.929초,
  앱 264.319·55.285초였다.
- R418은 열린 편집기에서 재생 중 실행 전 거부와 Transport 복구를
  대조했다. 정지 뒤 공개 도구 한 번과 화면의 정확한 악기 추가 Undo
  한 번으로 기존 433행·바인딩·그래프·믹서·트랙이 복원됐다. 199요청
  68.794초, 앱 113.309초. R417·R418의 모든 앱·owner가 정상 종료했고
  원본과 시험곡, 설치 18개·설정 10개씩 확인했다.
- R413의 개별 레이어·라우팅 증거와 함께 `CONTROL-MULTI`의 로컬
  완료 조건을 충족했다. 기존 1개 stock Mai Tai에 같은 모델 1개를
  병렬 추가하는 네이티브 경로로 제한했다. 일반 그래프 연결과 다른
  삽입·삭제 구조는 계속 검증한다. [#208](https://github.com/dev-universe/fender-studio-pro-mcp/pull/208)
- #159가 정확한 헤드의 필수 두 검사에 통과해 `f50c31c`에 병합됐다.
  #201 최신 헤드 검사와 #203을 이어가며 공개 패키지를 준비한다.
  [#159](https://github.com/dev-universe/fender-studio-pro-mcp/pull/159)

다음은 Scope 캡처의 직접 시작·완료를 들여다보는 동기화된 읽기 경로다.
`main` 18개·로컬 13개, 소프트웨어 2~5일, 장비 의존 4건,
지정 종료 시각 없음.

## 11월 26일 02시 26분 패키지 경로 병합과 Pedalboard 가이드

- #201은 최신 `main`을 포함하는 정확한 `77fa734`에서 필수
  두 검사를 통과했다. `main`의 결과는 `07b7a0b`이며
  검사 시작부터 병합까지 11분 50초였다. #203은 최신 베이스의
  `3355c8c`를 재검사한다. 공개용 패키지의 새 다운로드, 설치,
  업데이트, 제거와 롤백을 아직 시험해야 한다.
  [#201](https://github.com/dev-universe/fender-studio-pro-mcp/pull/201)
  [#203](https://github.com/dev-universe/fender-studio-pro-mcp/pull/203)
- #209는 이미 병합된 빈 슬롯·동일 모델·비인접 교체의 최신 컨트롤을
  README와 지원표에 연결했다. 예상 랙 SHA, 빈 슬롯 문자열,
  별도 캡처와 토큰 반환, 지연된 별칭과 Ampire 구분선을 포함한다.
  현재 스키마와 로컬 증거 링크 8개를 대조했다.
  [#209](https://github.com/dev-universe/fender-studio-pro-mcp/pull/209)

`main` 18개·로컬 13개, 소프트웨어 2~5일, 장비 의존 4건,
지정 종료 시각 없음. R420의 Scope 독립 관측 경로, R421의 새
Splitter 분기, R422의 Ampire Save 전후 대조를 준비한다.

## 11월 26일 02시 30분 Scope 표시와 중첩 Splitter 분기

- R420에서 정확한 Studio Pro 8.1.1 arm64 설치 이미지 SHA와 UUID를
  고정했다. REA/Ghidra 및 Craft Anything으로 Scope의 표시 배열을
  찾고 28개 명령 바이트를 별도 대조했다. DSP가 캡처 도중에도 배열을
  갱신한다. 해당 렌더러의 직접 시작·완료 플래그와 누적 샘플 수 관측
  경로, 별도 동기화 계약은 확인하지 못했다. 최대 창 80,000샘플의
  화면 진행은 R423에서 시험한다. 분석 7분 4초.
  [#210](https://github.com/dev-universe/fender-studio-pro-mcp/pull/210)
- R421에서 공개 `insert_slot_move` 한 번으로 Mixtool 2를
  Splitter 3의 빈 첫 레인에 옮겼다. 연결 목록 세 곳만 바뀌었고
  8노드·순서 연결 10개, 소유권과 두 DSP 144행은 그대로였다.
  네이티브 자식 나열 순서는 바뀌었다. 85요청 77.619초, 앱 128.402초.
  소유 파일을 저장하지 않고 닫았고 토큰 6개를 해제했다. 원본 파일,
  설치 18개·설정 9개를 확인했다.
  [#211](https://github.com/dev-universe/fender-studio-pro-mcp/pull/211)

`main` 18개·로컬 13개, 전체 소프트웨어 2~5일, 장비 의존 4건,
지정 종료 시각 없음. 실제 USB·오디오 재열거에 Volt 276과 Keystation88
항목이 없었다. 이어서 R422 Ampire 저장 대조와 R424 빈 Splitter
제거를 한 앱·한 owner로 직렬 시험한다.

## 11월 26일 02시 34분 Ampire 공장 마이크 저장 대조

- R422에서 한 Ampire의 MCM 800·4x12 M65 마이크 A/B/C와 Link를
  기존 R416의 원시 비트와 대조했다. 건드리지 않은 상태를 한 번 저장해
  90개 라이브 행, 디스크의 정규화 값, 2,297바이트 컴포넌트와 새 앱의
  재조회가 모두 일치했다. 이전 문서 읽기도 거부됐다.
- 62요청 21.833초, 재열기 45요청 4.636초, 앱 67.637·41.136초.
  원본과 저장본, 설치 18개·설정 10개, 두 앱과 owner 정상 종료를 대조했다.
  링크 편집 후 Undo·캡슐 적재의 정밀도 차이는 별도다.
  [#212](https://github.com/dev-universe/fender-studio-pro-mcp/pull/212)

다음은 R424 빈 중첩 Splitter 제거와 R423 Scope 시각 표시 대조다.
`main` 18개·로컬 13개, 전체 소프트웨어 2~5일, 장비 의존 4건,
지정 종료 시각 없음.

## 11월 26일 02시 43분 빈 Splitter, Scope 전이와 패키지

- R424는 Splitter 3 노드 메뉴의 Remove를 한 번 실행했다. 정확히
  해당 Splitter와 Merger가 사라져 8노드·10연결에서 6노드·7연결이
  됐다. 원래 첫 Mixtool의 다음 연결만 바뀌었고 두 DSP 144행,
  남은 객체와 믹서는 그대로였다. Undo 항목을 읽고 실행은 하지 않았다.
  86요청 78.977초, 앱 126.741초, 6개 토큰과 설치 18개·설정 9개 확인.
  [#213](https://github.com/dev-universe/fender-studio-pro-mcp/pull/213)
- R423에서 41.3Hz를 한 번 공급했다. 80,000샘플로 표시된 Scope
  창은 평평한 기준에서 왼쪽 일부·1/4·1/2·3/4·전체 폭으로 채워졌다.
  실제 Off 확인 뒤 찍은 프레임은 앞선 전체 파형과 달랐다. 바뀐 정확한
  시각, 내부 래치와 파형 버퍼는 미검증이다. 41개 값을 복원해
  평평한 화면을 확인했다. 84요청 88.469초, 앱 133.421초,
  원본·설치 18개·설정 10개 확인.
  [#214](https://github.com/dev-universe/fender-studio-pro-mcp/pull/214)
- R425는 Ampire의 링크 기준값과 표시 게인이 별개임을 확인했다.
  REA/Ghidra와 Craft Anything 및 C·Python Float32 수치 대조, 약 7분.
  A를 마지막으로 쓰던 방식의 한계를 확인하고 C를 마지막으로 쓰는
  여섯 단계 후보를 모델링했다. 표시 게인과 컴포넌트의 실제 결과,
  Link 해제 시의 숨은 기준값은 별도다.
  [#215](https://github.com/dev-universe/fender-studio-pro-mcp/pull/215)
- #203은 정확한 헤드 `3355c8c` 두 검사 뒤 `e5ebb53`으로 병합됐다.
  R426은 그 커밋에서 프리뷰를 만들고 배포 파일 9개, 개인 경로 0개,
  로컬 ad-hoc 서명, 21회 격리 설치·갱신·원복·제거를 확인했다.
  Gatekeeper는 CLI를 거부했다. 정확한 `main` 실행, 공개 자산과
  비인증 재다운로드는 다음 단계다. #208은 최신 `main`을 포함하는
  `7455031`의 두 검사를 진행 중이다.
  [#203](https://github.com/dev-universe/fender-studio-pro-mcp/pull/203)
  [#208](https://github.com/dev-universe/fender-studio-pro-mcp/pull/208)

두 앱 시험 모두 저장하지 않고 정상 종료했다. 단일 호스트로 R427
Ampire 대조와 R428 공개용 빈 Splitter 제거·저장·재열기를 이어간다.
`main` 18개·로컬 13개, 전체 소프트웨어 2~5일, 장비 의존 4건,
지정 종료 시각 없음.

## 11월 26일 02시 47분 공개 Multi 명령 병합

- #208이 정확한 최신 베이스 헤드 `7455031`에서 필수 검사 두 개를
  통과해 `1654cd1`에 병합됐다. 검사 시작부터 9분 13초.
  `CONTROL-MULTI`의 제한된 완료 범위가 이제 main에 들어갔다.
  일반 연결과 다른 그래프 유형은 별도다.
  [#208](https://github.com/dev-universe/fender-studio-pro-mcp/pull/208)
- R426 첫 빌드와 검증은 보존했다. 프리뷰에 #208까지 포함하도록
  정확한 새 main으로 재빌드한다. #209는 최신 베이스 재검사 중이다.
  이전 체크포인트의 공개 자산·비인증 다운로드·설치 검증은 아직 열려 있다.

`main` 17개·로컬 13개, 전체 소프트웨어 2~5일, 장비 의존 4건,
지정 종료 시각 없음. Studio Pro와 MCP owner는 닫혀 있다.

## 11월 26일 02시 53분 Ampire 정확 복구 및 macOS 프리뷰

- R427이 R425의 여섯 단계 MCP 쓰기를 정확히 재현했다. 최종 C 뒤
  공장 마이크의 세 값과 Link, DSP 90행, 2,297바이트 캡슐 SHA-256,
  믹서와 트랙이 모두 일치했다. 138요청 45.841초, 앱 96.754초,
  캡처 4개 해제, 원본 시험곡·설치 18개·설정 10개 원복.
  숨은 Link 해제 기준값은 R429에서 별도 대조한다.
  [#216](https://github.com/dev-universe/fender-studio-pro-mcp/pull/216)
- 정확한 `1654cd1`의 macOS arm64 프리뷰 `v0.1.0-preview.1`을
  02시 52분 47초 KST에 공개했다. 최초 빌드와 최신 main 재빌드를
  포함해 약 15분이다. 태그, 검사 통과 PR 트리, 두 자산의 draft
  재다운로드와 비인증 공개 재다운로드를 대조했다. 공개 아카이브에서
  최초 설치·갱신·이전 설치 원복·제거를 21회 격리 명령으로 확인했다.
  체크섬 `db04621a6c6c435cf05ccb65e070fc7ca259828ac31d44f57f76a66813691a7b`.
  strict ad-hoc 서명은 결과가 일치했고 Gatekeeper는 CLI를 거부했다.
  `SHARED-MACOS-PACKAGE`의 공개 자산·격리 설치 검증을 닫았다.
  [공개 프리뷰](https://github.com/dev-universe/fender-studio-pro-mcp/releases/tag/v0.1.0-preview.1)
  [#217](https://github.com/dev-universe/fender-studio-pro-mcp/pull/217)
- R428 구현은 공유 앱을 열기 전에 정확한 그래프 소유권과 거부·불명
  응답을 독립 리뷰한다. 현재 후보의 저장·재열기 시험은 준비만 됐다.

`main` 16개·로컬 12개, 전체 소프트웨어 2~5일, 장비 의존 4건,
지정 종료 시각 없음. 현재 Studio Pro와 MCP owner는 닫혀 있다.

## 11월 26일 03시 00분 Ampire 숨은 기준값과 Pedalboard 문서

- R429는 이전 여섯 쓰기와 링크 상태 90행, 2,297바이트 캡슐 일치를
  재현했다. 하나의 새 승인으로 Link Off를 실행하니 공장 Link Off 대비
  A·B가 각각 정규화 Float32 1 ULP 작고 C는 일치했다. Link와 A·B
  이외의 행과 믹서·트랙·인서트는 그대로였다. 168요청 68.357초,
  앱 116.192초, 여섯 토큰 해제와 원본·설치 18개·설정 10개 확인.
  R430에서 복구된 링크 상태를 한 번 저장한 뒤 새 세션에서 확인한다.
  [#218](https://github.com/dev-universe/fender-studio-pro-mcp/pull/218)
- #209 완료. 최신 베이스 헤드 `b437c14`에서 두 검사를 통과해
  `602984c`에 병합됐다. 재검사 시작부터 병합까지 12분 41초.
  다음 #210 현재 헤드 `1c52427`을 검사 중이다.
  [#209](https://github.com/dev-universe/fender-studio-pro-mcp/pull/209)
- R428 로컬 구현은 초기 리뷰에서 드러난 정확한 그래프 8개 소유권 및
  오래된 응답 처리 보강을 적용했다. 최종 독립 검토와 실앱 Save·재열기가
  남았다. 폐기된 초기 후보의 빌드 결과를 실제 시험에 사용하지 않는다.

고정 항목은 `main` 16개·로컬 12개, 전체 소프트웨어 2~5일,
지정 종료 시각 없음. 로컬 수용 뒤 병합을 기다리는 네 가지는 Chorder,
Input Filter, Shared Event, Impact다. 남은 로컬 항목은 Ampire, Mustang,
Rumble, Scope, Shared Graph, Shared Persistence, Shared Docs Release,
CV 프리셋·CV 제어, External Mono, External Stereo, 물리 키다.
장비가 필요한 시험은 CV 두 항목을 한 묶음으로 세어 네 가지다.

## 11월 26일 03시 17분 복구된 Ampire 저장본과 Scope Hold 대조

- R430에서 정확히 복구된 Ampire를 한 번 저장하고 새 프로세스 및
  세션에서 열었다. 저장된 컴포넌트 2,297바이트와 DSP 90행이 원래
  링크 상태와 같았다. 이전 문서 거부 뒤 한 번 Link Off를 실행하니
  A/B/C 원시 비트가 공장 초기 Link Off와 정확히 같았다. 링크 이외의
  89행과 믹서·트랙이 유지됐다. 166요청 77.929초와 63요청 27.492초,
  앱 133.914·88.617초, 토큰 10개. 저장본과 원래 관리 파일 18개,
  설정 10개를 확인했다. Volume·Pitch는 R433에서 별도 조사한다.
  [#220](https://github.com/dev-universe/fender-studio-pro-mcp/pull/220)
- R431은 Scope Hold의 검출 억제와 Oneshot의 블록 시작 분기를
  정확한 설치본에서 확인했다. REA/Ghidra·Craft·LLVM 대조 8분 20초.
  R432는 Hold 200%와 Trigger 160,000 smp를 확인해 이전과 같은
  입력·시간 창에서 전체 폭 프레임 세 개가 바이트 단위로 일치했다.
  두 개는 Off 전, 하나는 실제 Off 확인 뒤다. DSP 41행과 Hold 100%
  복원, 앱 143.256초, owner 96요청 84.972초, 18+10 파일 확인.
  내부 카운터와 버퍼는 여전히 열려 있다.
  [#219](https://github.com/dev-universe/fender-studio-pro-mcp/pull/219)
  [#221](https://github.com/dev-universe/fender-studio-pro-mcp/pull/221)
- R428 첫 후보 `35ec765`에서 잘못된 바인딩 5개와 올바른 앵커
  요청이 모두 실행 전 거부됐다. 올바른 요청은 retained primary
  인터페이스 불일치였다. 원래 8노드·10연결·DSP 144행을 재조회했고
  207요청 54.897초, 앱 124.538초, 16회 토큰 해제, 18+9 파일 복원.
  `d625d07`은 exact INode/IUnknown 오프셋을 정적으로 확인해 수정하고
  독립 리뷰를 통과했다. 원본과 분리된 R428B에서 다시 검증한다.
- #210이 `1c52427`의 두 검사 후 `74c35f8`에 병합됐다.
  검사 시작부터 8분 55초. #211은 최신 main 검사 중이다.
  [#210](https://github.com/dev-universe/fender-studio-pro-mcp/pull/210)

`main` 16개·로컬 12개, 전체 소프트웨어 2~5일, 장비 의존 4건,
지정 종료 시각 없음.

## 11월 26일 03시 24분 빈 중첩 Splitter 공개 도구

- 정확한 후보 `d625d07`의 R428B에서 잘못된 앵커 5개를 실행 전에
  거부하고 원래 그래프·DSP를 각각 재조회했다. 공개 API의 올바른
  요청 1회로 빈 Splitter 3과 그 merger만 제거했다. 8노드·10연결이
  6노드·7연결로 바뀌고, 원래 병렬 Mixtool 두 개와 DSP 144행은
  유지됐다. Routing 화면과 독립 MCP 그래프를 대조했다.
- 최초 앱에서 시험곡을 한 번 저장했다. 새 프로세스에서 이전 문서
  요청 거부, 파일 해시, 보존 그래프와 DSP 144행을 다시 확인했다.
  재개방 후 닫을 때 표시된 저장 안내는 거절했다. 앱 284.232초와
  57.980초, MCP 253요청·63.064초와 75요청·23.786초, 토큰 26개.
  두 앱과 owner 정상 종료, 원본 시험곡 및 저장본 확인, 설치 18개와
  설정 9개 원복. 초기 인터페이스 오류와 정정 내용을 공개 근거에
  남겼다. 현재 #222는 열린 상태이며 #211 뒤에 최신 베이스 검사를
  진행한다. [#222](https://github.com/dev-universe/fender-studio-pro-mcp/pull/222)
- R434는 첫 번째 빈 레인 이동을 저장하고 새 프로세스에서 대조한다.
  R435는 공개 제거의 Undo를 별도 시험곡에서 검증한다. 일반 그래프
  연결 범위와 저장 지원표가 남아 있어 전체 고정 개수는 유지한다.

`main` 16개·로컬 12개, 전체 소프트웨어 2~5일, 장비 의존 4건,
지정 종료 시각 없음. R434 준비 예상 8~15분. Studio Pro와 MCP owner는
닫혀 있고 평상시 설치로 돌아왔다.

## 11월 26일 03시 26분 Ampire 글로벌 역할과 CI 재실행

- R433은 설치된 Ampire 8.1.1의 동일 arm64 이미지를 LLVM과 Craft로
  확인했다. 검사한 native provider 7개에서 Wah 1개와 Speed 6개를
  확인했다. FAT Volume은 별도 로컬 값을 읽고 쓴다. 글로벌 Volume과
  Pitch의 provider나 UI·RemotePages 연결은 찾지 못했다. 일반 CCL
  별칭이 원본 부재 시 0과 빈 표시를 반환해도, 라이브 두 항목의
  wrapper와 포인터는 확인되지 않았다. 지원 대상이나 오디오 효과로
  승격하지 않는다. 읽기 자료만 사용했으며 10분 29초가 걸렸다.
  [#223](https://github.com/dev-universe/fender-studio-pro-mcp/pull/223)
- #211은 `f72504b`의 Core를 통과했다. macOS 작업은 단계 실행
  전에 취소돼 같은 헤드의 해당 작업을 재요청했고 병합 감시를 이어간다.
  [#211 검사](https://github.com/dev-universe/fender-studio-pro-mcp/actions/runs/45672192591)

## 11월 26일 03시 34분 첫 분기 재열기와 공개 제거 Undo

- R434는 첫 빈 레인의 Mixtool 2 이동을 실제 8.1.1에서 검증하고
  시험곡을 한 번 저장했다. 새 앱에서 8노드·10순서 연결·DSP 144행,
  UI, 파일 해시를 대조했고 이전 문서는 변경 전에 거부됐다. 이동
  전후 자식 열거 순서는 달랐다. 앱 75.979·59.708초, 두 owner
  91요청·45.061초와 62요청·23.953초, 토큰 10개. 저장 안내를
  거절하고 원본과 파일, 설치 18개 및 설정 9개를 확인했다.
  [#224](https://github.com/dev-universe/fender-studio-pro-mcp/pull/224)
- R435는 빈 중첩 Splitter를 공개 도구로 한 번 제거한 뒤 앱 편집
  메뉴의 활성 `되돌리기: 스플리터 제거` 항목을 한 번 눌렀다.
  원래 노드 ID 8개·순서 연결 10개·열거 순서·DSP 144행과
  두 장치 및 믹서가 일치했다. 앱 185.851초, owner 162요청
  120.362초, 토큰 12개. 앱과 owner 정상 종료, 저장 없음, 두
  곡의 해시 확인과 설치 18개·설정 9개 복원.
  [#225](https://github.com/dev-universe/fender-studio-pro-mcp/pull/225)
- #222는 정확한 `daef130`의 두 필수 검사 통과 뒤 `265e8c9`에
  병합됐다. PR 생성부터 병합까지 10분. #211은 현재 베이스의
  새 헤드 `126995f`로 검사 중이다.
  [#222](https://github.com/dev-universe/fender-studio-pro-mcp/pull/222)
- R437은 두 번째 빈 분기 이동의 Save/reopen과 이전 문서·그래프
  거부 근거가 기존 main R191에 있음을 확인했다. 다음 R438은
  캡처 체인 복구, R439는 채운 Pedalboard 페달을 빈 위치에 옮긴
  상태 그대로 저장하는 독립 시험이다.

고정 항목은 `main` 16개·로컬 12개, 소프트웨어 2~5일, 장비 검증
4건, 지정 종료 시각 없음. 현재 앱·MCP owner 종료와 정상 설치를 확인했다.

## 11월 26일 03시 36분 Scope 파형과 설정 스트림

- R436은 정확한 8.1.1 Scope 이미지의 REA/Ghidra 자료에 LLVM·Craft
  검사를 더했다. 세 개 Float32 원시 버퍼와 네 개 화면 요약 배열을
  구분했다. 공개 `IStorable`은 설정을 전달하며 Scope 저장·읽기
  훅에 원시 파형 전달이 없었다. DSP 캡처 동기화·실제 시간·복원은
  미확인이다. 정적 대조 8분 11초, 앱 실행은 없었다.
  [#226](https://github.com/dev-universe/fender-studio-pro-mcp/pull/226)
- R438 인서트 체인 복구와 R439 빈 Pedalboard 슬롯 이동본의 독립
  Save/reopen 시험을 준비하고 있다. R440은 그래프의 포트·연결을
  소유권과 함께 읽을 수 있는 정확한 경로를 조사한다.

## 11월 26일 03시 49분 인서트 복구·Pedalboard 전방 이동 저장

- R438은 옛 시험곡의 Main이 unity라 그래프 요청 전에 멈췄다.
  별도 R438B에서 Main mute와 0볼륨을 독립 확인하고 실제
  제거와 복원을 한 번씩 실행했다. 그래프 6/7→4/4→6/7와
  두 새 Slot·Device, DSP 144행을 확인했다. 지정 곡을 한 번
  저장하고 새 프로세스에서 같은 그래프·ID·DSP와 이전 문서
  restore 거부를 확인했다. 두 앱 101.792·74.028초, owner
  123요청·61.641초와 62요청·29.284초, 토큰 13개. 원본, 한 번
  저장한 파일, 설치 18개·설정 9개 확인.
  [#227](https://github.com/dev-universe/fender-studio-pro-mcp/pull/227)
- R439는 Pedalboard의 Compressor를 1번에서 빈 2번으로 옮긴 채
  저장했다. 새 프로세스에서도 8슬롯의 payload와 성분 지문이
  같았다. 비운 슬롯의 활성 별칭만 10000→0으로 정리됐다.
  33행을 기대값과 대조했고 이전 문서는 쓰기 전에 거부됐다.
  두 앱 116.408·59.178초, owner 88요청·59.830초와
  45요청·19.787초. 보호 원본·저장본·설치 18개·프로필 10개 확인.
  [#228](https://github.com/dev-universe/fender-studio-pro-mcp/pull/228)
- R440은 실제 호스트 저장과 고정 Move가 쓰는 방향 연결과 중복
  ordinal을 확인했다. 역방향 리스트의 실제 중복 개수와 물리
  포트 ID는 미확인이다. 10분 15초. R443은 좁은 소유권 조회
  후보를 준비한다. [#229](https://github.com/dev-universe/fender-studio-pro-mcp/pull/229)
- R441은 카탈로그의 중괄호 포함 클래스 UID와 네이티브의
  중괄호 없는 UID를 같은 표기로 비교해서 중단했다. 8요청
  1.589초, 앱 82.266초. Main mute 한 번 이외에 인서트 추가,
  테스트 신호, Save는 없었다. 18+9 파일 확인. 새 R441B에서
  각각의 실제 필드 형식을 확인한다. 다음 앱은 R442 비인접
  교환 상태 저장 시험이다.

고정 항목 `main` 16개·로컬 12개, 전체 소프트웨어 2~5일, 장비
검증 4건, 지정 종료 시각 없음. 두 앱과 MCP owner는 현재 닫혀 있다.

## 11월 26일 03시 56분 Pedalboard 비인접 저장 및 체크 재개

- R442는 Standalone Pedalboard 1↔3 교환 후 한 번 저장하고
  새 앱에서 열었다. Big Fuzz·Compressor의 이동 payload와
  가운데 Reverb 7행·payload, 5개 빈 슬롯 및 컴포넌트 지문이
  일치했다. 42개 고정 행을 정확 대조했고 두 read-only peak는
  스키마와 순간값을 따로 기록했다. 이전 문서 거부, 추가 저장
  없음. 앱 90.369·61.319초, owner 92요청·58.859초와
  45요청·15.985초. 18+10 파일과 원본·저장본 확인.
  [#230](https://github.com/dev-universe/fender-studio-pro-mcp/pull/230)
- #211은 현재 헤드 `126995f`의 두 검사 후 `b01a38f`에
  병합됐다. 최근 재검사 시작부터 18분 11초. #224는 새 베이스
  헤드 `e11336e`에서 검사 중이다.
  [#211](https://github.com/dev-universe/fender-studio-pro-mcp/pull/211)
- R441B는 Tone과 Ampire의 정확한 본체를 추가했다. 수정된
  카탈로그 조회는 통과했으나 후보 `92145d5f`가 List bound
  수정 이전이라 35번째 Wah 모델 목록 요청을 거부했다. 신호와
  모델 값 변경 없이 캡처 1개 해제, No Save, 앱 82.494초,
  owner 3.665초, 정상 설치 18개·설정 10개 확인. R441C는
  동일 실제 역할을 검증했던 `5d0fea1`의 공식 tool 경로를 쓴다.
- 다음 R444는 공개 같은 모델 교환의 변경 랙을 저장한다.
  R439와 R442는 각각 빈 슬롯·비인접 변경 자체를 저장해
  재열기까지 완료했다. Volt 276·Keystation88은 새 USB 및
  CoreAudio 열거에도 나타나지 않았다.

## 11월 26일 04시 07분 동일 모델 저장 및 Ampire Wah 신호

- R444에서 공개 교환의 Compressor 두 payload가 서로 바뀌었다.
  활성 별칭은 즉시 바뀌었고 슬롯별 Gain·활성 4행은 첫
  프로세스의 Save 뒤에도 원래값을 보고했다. 독립 디스크와
  새로운 세션의 38행·성분 지문·화면은 바뀐 payload와 일치했다.
  이전 문서의 공개 요청은 거부됐다. 앱 88.056·61.037초,
  owner 94요청·55.672초와 45요청·17.553초, 18+10 파일과
  보존 원본·저장본 확인. [#231](https://github.com/dev-universe/fender-studio-pro-mcp/pull/231)
- 저장·재시작 고정 항목의 대표 지원표를 #231에 추가했다. 앞선
  R191, R434, R428B, R438B, R439, R442 및 R444를 반영해
  `SHARED-PERSISTENCE`는 로컬 충족으로 집계한다. #224·#227·
  #228·#230·#231의 병합을 기다린다. `main` 16개·로컬 11개다.
- R441C의 정확한 `5d0fea1`로 440 Hz −24 dB 톤을 Ampire에
  전달했다. 고정 입력에서 Wah 별칭과 대상 pedal이 raw
  0→6250→0을 따라 좌우 출력/입력 비율 4.279→3.594→4.275를
  보였다. 첫 정착 기준은 피크 감쇠 중이라 멈췄고, 같은 문서의
  새 read-only owner가 Tone Off·입력 0·출력 3.4e−9 미만을
  확인했다. 남은 모델 원복 한 번 뒤 빈 슬롯과 원래 성분 지문,
  90행 중 88행이 일치했다. 튜너 `diffCents` 및 빈 활성 별칭
  두 값이 달랐다. 앱 362.268초, 본 owner 123요청·62.373초,
  후속 읽기 16요청·원복 18요청, 캡처 4개 해제. No Save,
  원본과 시험 파일·설치 18개·프로필 10개 확인.
  [#232](https://github.com/dev-universe/fender-studio-pro-mcp/pull/232)
- 다음은 R443의 격리 read-only 그래프 진단 `9fdedf4`와 Xfer
  서명판 R445 Serum 2·Serum 2 FX의 실제 호스트 내부 신호다.
  R446은 Scope의 호스트 소유 시간과 파형 관측 경로를 별도로
  조사 중이다. 전체 소프트웨어 2~5일, 장비 의존 4건, 지정 종료
  시각 없음. 앱과 owner 종료 및 정상 설치를 확인했다.

## 11월 26일 04시 23분 실제 역방향 그래프와 Serum 신호 준비

- #224는 macOS 작업의 실행 전 취소 뒤 같은 헤드를 재실행해
  `e11336e`의 두 검사에서 통과했다. `1bb778f`에 병합됐다.
  재실행 모니터 시작부터 5분 25초. #225의 현재 헤드는
  `e386d35`다. [#224](https://github.com/dev-universe/fender-studio-pro-mcp/pull/224)
- R446은 AudioEngine에서 장치 샘플레이트를 게시하는 경로와
  44,100 Hz 대체값, 별도의 CoreAudio 장치 시각을 확인했다.
  실제 Scope 캡처 인스턴스에 묶인 시각·완료 게시 경로는
  여전히 미확인이다. 정적 조사 약 9분. [#233](https://github.com/dev-universe/fender-studio-pro-mcp/pull/233)
- R443은 실제 화면 관문에서 멈췄고 R443B는 평상시 이전 브리지의
  `include_routing` 전달 누락으로 도달 전 거부됐다. R443C는
  정확한 `9fdedf4` 브리지와 진단 네이티브를 함께 사용해 두
  Mixtool의 두 회차 직접 조회, 두 ZIP, 노드 8개·순서 연결
  10개·DSP 144행을 맞췄다. 두 빈 레인의 동일 Merger는 들어오는
  연결을 2개 보고했다. 잘못된 anchor 2회와 생략 2회를 대조했다.
  앱 110.198초, owner 56요청·32.474초, 토큰 2개 해제,
  그래프 변경과 Save 없음. 설치 18개와 프로필 9개 원복.
  진단은 기본 빌드에서 꺼져 있다. [#234](https://github.com/dev-universe/fender-studio-pro-mcp/pull/234)
- R445I는 Xfer 서명판 Demo와 정확한 R319 악기 캐리어를 로드했다.
  메인 창에서 전용 MIDI 입력을 선택할 수 없어 노트 없이
  종료했다. 75요청·133.472초, 송신기 종료, 설치 18개·설정 11개
  복원. 새 R445IA는 플러그인 편집기 자체 건반을 관측한다.
- R445FX는 공식 FX의 2,623행과 Distortion 다음 EQ, 네
  인서트 순서를 확인했다. 두 독립 미터의 첫 이름 필터 응답에
  인스턴스가 없어 Tone 쓰기 전 중단했다. 123요청·81.437초.
  설치 18개·설정 10개와 임시 공식 번들 회수 확인.
  R445FXB는 전체 native 페이지에서 두 미터를 읽는다.

현재 앱과 두 owner 종료, 공식 번들 임시 스캔 폴더 비어 있음,
정상 설치 원복. 고정 작업 `main` 16개·로컬 11개, 전체 소프트웨어
2~5일, 장비 검증 4건, 지정된 종료 시각 없음.

## 11월 26일 04시 40분 Serum 내부 스테레오 검증

- #225의 현재 기반 `e386d35`는 두 필수 검사에서 통과했고
  `072a1a7`에 병합됐다. CI 준비부터 10분 7초.
  [#225](https://github.com/dev-universe/fender-studio-pro-mcp/pull/225)
- R445IA는 공식 악기 Demo의 편집기 C3를 한 번 눌러 초기 정확한
  0 세 번, 활성 스테레오 피크 여섯 번, 해제 후 2,623행 일치를
  확인했다. 6초 안에 무음 조건은 충족하지 못했다. 316요청·79.206초.
  R445IAB는 같은 소스에서 읽기 상한만 20초로 늘렸다. 활성 스테레오
  피크 7회, 양쪽 무음 3회 연속 및 반환 뒤 9.179초 시점의 계측을
  확인했다. 최종 2,623행 두 번 일치, 319요청·74.821초.
  외부 MIDI·물리 출력·편집 가능 저장은 별도다.
  [#236](https://github.com/dev-universe/fender-studio-pro-mcp/pull/236)
- R445FXB는 공식 FX 슬롯 2에서 두 독립 39행 DSP 미터, 440 Hz 기준
  신호와 단일 On·Off를 확인했다. Off까지 3.457초, 210요청·75.995초.
  실제 Off 뒤 약 4.662초의 마지막 표본은 무음 기준을 넘었다.
  R445FXC는 독립 Off 이후 최대 20초 읽기만 허용했다. On부터
  Off까지 3.563초, 스테레오 전후 여섯 쌍 출력/입력 2.2634~2.2662,
  Off 이후 8.699초에 무음 연속 세 쌍, 최종 2,623행 일치.
  294요청·116.116초. 네 시험 모두 저장 없이 종료했고 각각 설치
  18개·프로필 10개 원복과 임시 공식 스캐너 경로 회수를 확인했다.
  [#237](https://github.com/dev-universe/fender-studio-pro-mcp/pull/237)
- R447은 정확한 빌드에서 순서와 중복을 보존하는 DSP `AudioTask`
  처리 목록 및 Splitter의 분기 버퍼와 Merger의 입력 수 기반 처리를
  확인했다. Sidechain의 소유자별 source/destination 검색과 두 포트의
  상호 갱신 경로도 확인했다. 11분 14초, 실앱 요청 0건. R448은
  DSP 목록을 같은 시점에 읽을 수 있는 엔진 보호와 소유권을 조사 중이다.
  [#235](https://github.com/dev-universe/fender-studio-pro-mcp/pull/235)
- R449는 공식 악기의 공개 `tracks_edit`로 단일 MIDI 노트를 만들어
  격리된 편곡 재생과 파라미터 독립 읽기를 확인할 준비다. 현재
  앱·owner 종료와 정상 설치 원복. `main` 16개·로컬 11개, 장비 4건,
  전체 예상 2~5일과 CI·장비 대기, 지정된 종료 시각 없음.

## 11월 26일 04시 47분 인서트 원복 병합과 편곡 시험

- #227은 최신 `6980f674`의 두 필수 검사에서 통과했다. 검사
  확인부터 17분 48초, `f05980a`에 병합됐다. #236은 이 기반을
  반영한 정확한 `39edb2ec`에서 검사 중이다.
  [#227](https://github.com/dev-universe/fender-studio-pro-mcp/pull/227)
  [#236](https://github.com/dev-universe/fender-studio-pro-mcp/pull/236)
- 기존 대표 검증표에 상태 읽기, Save/reopen, 이벤트와 내부 신호를
  근거별로 연결했다. 상대 링크와 앵커 26개 확인, 약 4분.
  [#238](https://github.com/dev-universe/fender-studio-pro-mcp/pull/238)
- R448은 그래프 게시와 인서트 재빌드의 잠금 경로, 태스크 배열의
  제자리 재할당을 대조했다. 실제 Routing 태스크와 게시 엔진의
  잠금 및 객체 수명 연결은 남아 있다. 11분 12초, 실앱 요청 0.
  [#239](https://github.com/dev-universe/fender-studio-pro-mcp/pull/239)
- R449는 메트로놈 조회의 `preroll:null`과 정확한 독립 transport의
  `preroll:false` 차이로 part/Play/Stop 전에 중단했다. 클릭과
  프리카운트 off, 73요청·30.476초, 앱 약 57초. 앱·owner 정상 종료,
  No Save, 임시 공식 번들 회수, 설치 18개·프로필 10개 원복.
  독립 Transport가 프리롤을 확인하는 새 R449B를 준비한다.
- 이전 R422·R441 앱에 딸렸던 두 스캐너 프로세스를 전용 로그의
  파일 참조로 식별해 정상 종료했다. 현재 USB/CoreAudio에 Volt 276과
  Keystation88은 보이지 않는다. R451은 Scope 캡처 게시, R452는
  Ampire Speed 내부 처리의 검증 가능성을 독립 조사 중이다.

## 11월 26일 05시 Serum 편곡 재생과 Ampire 표시값

- #236은 새 기반의 `39edb2ec` 두 필수 검사 통과 뒤 `e9c356b`에
  병합됐다. 검사 준비부터 10분 38초. 공용 `FETCH_HEAD` 경합으로
  #237 확인이 한 번 멈췄다. 각 PR의 전용 로컬 ref로 바꿔 같은
  비교 조건을 유지하고 #237 최신 `dc104f17` 검사를 재개했다.
  [#236](https://github.com/dev-universe/fender-studio-pro-mcp/pull/236)
  [#237](https://github.com/dev-universe/fender-studio-pro-mcp/pull/237)
- R449B는 프리롤 확인 뒤 정확한 MIDI 파트 1개와 pitch 60 음표 1개를
  쓰고 실제 편곡에 표시했다. 빈 화면 전용 include_context 조회가
  차단해 Play 전 중단했다. 83요청·43.786초, Save 없음.
  R449C는 같은 검사를 빈 화면에 한정하고 populated 편곡은 현재
  바인딩, 파트·노트·믹서와 조합해 검증했다. 실제 Play/Stop 각 1회,
  노트 전 무음 5회·활성 stereo 7회·정지 후 8.109초의 무음 3회와
  2,623행 일치. 300요청·64.074초. 두 시험 모두 호스트·owner 종료,
  설치 18개·프로필 10개 원복, 임시 스캔 경로 비어 있음.
  [#240](https://github.com/dev-universe/fender-studio-pro-mcp/pull/240)
- R451은 Scope DSP가 블록 크기·버퍼를 사용하고 캡처 표시 배열을
  제자리 갱신하는 경로를 확인했다. 엔진 수명·잠금·알림으로 같은
  세대의 프레임·latch·count를 원자적으로 읽는 계약은 미확인이다.
  10분 26초, 실앱 요청 0. [#241](https://github.com/dev-universe/fender-studio-pro-mcp/pull/241)
- R452A의 활성 Chorus Speed A/B/A에서 슬롯 5 위상 표시 72개를
  대조했다. Off 뒤 읽기 전용 raw `1129.0789395570755`가 남아
  최초 0 조건은 실패했다. 별도 쓰기 없는 owner는 동일한 여섯 값,
  고정 DSP 89행과 원래 2,297바이트 컴포넌트 지문을 확인했다.
  UI Off와 수정 표시, 210요청·41.080초 + 24요청·4.138초,
  앱 274.590초, Save 없음, 설치 18개·설정 10개 원복.
  [#242](https://github.com/dev-universe/fender-studio-pro-mcp/pull/242)
- R453은 Xfer 공식 2.1.5 arm64 두 클래스의 입력 bus 0/channel 0
  130개 매핑과 실제 호스트의 각 130개 DSP 이름을 맞췄다. REA/Ghidra와
  Craft 정적 대조 완료. 현재 공개 read-only MCP 조회를 테스트 중이다.
  `main` 잔여 16개·로컬 11개, 장비 의존 4건. 소프트웨어 전체
  2~5일과 CI·장비 대기. 지정된 종료 시각 없음. 앱·owner 종료와
  정상 설치 확인.

## 11월 26일 05시 09분 Serum FX 병합과 MIDI 조회 구현

- #237은 정확한 `dc104f17`에서 두 필수 검사를 통과한 뒤 `922b9c3`에
  병합됐다. 검사 재개부터 9분 50초. #240의 갱신된 `ee331185`에서
  두 검사를 진행 중이다. [#237](https://github.com/dev-universe/fender-studio-pro-mcp/pull/237)
  [#240](https://github.com/dev-universe/fender-studio-pro-mcp/pull/240)
- R453은 서명된 2.1.5 arm64 VST3의 Serum 2·FX에서 첫 MIDI
  채널 스칼라 매핑 각 130개를 독립 SDK로 검증했다. 실제 R449C·R445FXC
  DSP 이름과 각 130개를 맞췄다. REA/Ghidra의 29개 명령 및 Craft의
  116바이트가 일치했다. 넓은 별도 SDK 시험은 완료되지 않았다.
  두 변형의 첫 채널 밖 범위와 Note Expression은 공개 조회에 포함하지
  않았다. 새로운 `serum2_midi_mapping`이 실제 독립 MCP 요청 260개를
  SDK와 동일하게 반환했고 잘못된 다섯 요청을 거부했다. 전체 준비부터
  MCP 확인 17분 43초, 실제 stdio 1.720초, Rust 264·JS 441 통과.
  [#243](https://github.com/dev-universe/fender-studio-pro-mcp/pull/243)
- R454는 FabFilter의 legacy feedback 7개에 대한 호스트 전달을
  정적으로 조사 중이다. R452B는 신호를 물리로 내보내지 않는
  채널 스템 사전 검증을 준비한다. 앱과 모든 소유자 종료, 정상 설치
  확인. `main` 16개·로컬 11개, 장비 4건, 전체 2~5일과 CI·장비 대기,
  지정된 종료 시각 없음.

## 11월 26일 05시 25분 Serum 값 원복과 건식 스템 확인

- #240은 최신 `ee331185`에서 두 검사 통과 뒤 `cb84a71`에 병합됐다.
  검사 준비부터 11분 14초. #243의 새 `330e0436` 검사 중이다.
  Rust·JS에 더해 로컬 Python 560개도 통과했다.
  [#240](https://github.com/dev-universe/fender-studio-pro-mcp/pull/240)
  [#243](https://github.com/dev-universe/fender-studio-pro-mcp/pull/243)
- R456은 Audio 무입력을 한국어 `없음`으로 읽는 지점에서 setter 전에
  멈췄다. 28요청·27.104초. 독립 R456B는 해당 FX 표기와 악기 `null`을
  각각 확인하고 CC1·채널 압력·피치 벤드의 매핑값을 두 변형에서
  각 한 번 쓰고 원복했다. 각 변경 중 피어 2,623행, 각 원복 뒤 양쪽
  2,623행 및 현 문서·라우팅·인스턴스를 확인했다. 767요청·105.095초,
  12 setter, 재생·MIDI·Save 0. 두 호스트와 owner 종료, 임시 VST3 회수,
  설치 18개·프로필 10개 원복. [#244](https://github.com/dev-universe/fender-studio-pro-mcp/pull/244)
- R454는 실제 빌드 VST3 출력점이 host queue와 현재 hosted 파라미터에
  전달되는 경로를 확인했다. 편집 컨트롤러의 실제 반환이나 Pro-G 음수
  처리는 확인되지 않았다. 13분 43초, Ghidra 34함수, Craft 18범위,
  실앱 요청 0. 7개 유지 확인은 계속 열려 있다.
  [#245](https://github.com/dev-universe/fender-studio-pro-mcp/pull/245)
- R455의 독립 오프라인 대조는 기존 두 공개 라우팅 캡처로 모든
  소스 순번·중복 및 8노드 유입 수를 네 직접 조회에서 재구성했다.
  3분 37초. 고정 SHARED-GRAPH의 일반 연결/삭제·원복은 남아 있다.
- R452B는 정확한 Audio track에 24초 기준 WAV를 하나 넣고 Main mute/0
  아래서 48 kHz stereo PCM24 채널 한 개를 오프라인 스템으로 내보냈다.
  파일과 프레임 경계는 대조됐고 1 LSB 판정은 첫 비영 표본에서 실패했다.
  37요청·195.055초, 앱 297.587초, No Save·정상 종료, 설치 18개와
  프로필 10개 원복. 전체 잔차와 위상의 독립 정량화 후 실제 Ampire
  오디오 비교를 진행한다. 앱·owner 종료, `main` 16개·로컬 11개,
  장비 4건, 전체 2~5일과 CI·장비 대기, 지정 종료 시각 없음.

## 11월 26일 05시 30분 Serum MIDI 공개와 스템 잔차

- #243 exact `330e0436`의 두 필수 검사에서 통과해 `5a65f0a`에
  병합됐다. 준비부터 13분 37초. `serum2_midi_mapping`은 두 공식
  2.1.5 arm64 변형의 첫 입력 채널을 읽기 전용으로 조회한다.
  #244의 `fd15015`가 새 `main`을 반영했다.
  [#243](https://github.com/dev-universe/fender-studio-pro-mcp/pull/243)
  [#244](https://github.com/dev-universe/fender-studio-pro-mcp/pull/244)
- R452B의 보존 스템 1,152,000 프레임을 두 독립 분석에서 다시
  대조했다. 게인은 L/R `1.000000007276/1.000000007743`, 영 지연
  상관계수 각각 0.999999999810 초과, 최대 PCM 잔차 32/39 LSB,
  RMS 7.284/7.266 LSB. 1 LSB 원 판정 실패 기록을 보존했다.
  [#246](https://github.com/dev-universe/fender-studio-pro-mcp/pull/246)
- R455의 동일 public archive 대조는 #234에 추가됐다. 새 그래프
  조회 작성 없이 기본 읽기 근거를 유지한다. R457은 FabFilter 최종
  도착과 기존 알림, R458은 실제 Sidechain 소유 경계, R452C는 Ampire
  스템의 Speed A/B/A 검증을 준비한다.
  [#234](https://github.com/dev-universe/fender-studio-pro-mcp/pull/234)
- 독립 CoreAudio/CoreMIDI 새 목록에서 Volt 276과 Keystation88은
  계속 감지되지 않았다. 앱과 owner 종료, 정상 관리 설치 확인.
  `main` 16개·로컬 11개·장비 4건, 전체 2~5일과 CI·장비 대기,
  지정 종료 시각 없음.

## 11월 26일 05시 42분 Serum 변경·원복 근거와 Ampire 스템

- #244의 최신 `main` 기반 `fd15015`에서 두 필수 검사를 통과해
  `cdbb060`에 병합했다. 9분 26초. 두 공식 Serum VST3에서 CC1,
  채널 압력, 피치 벤드의 DSP 값 변경과 2,623행 독립 원복을 확인했다.
  #228은 새 `main`을 반영한 `1a14332`를 검사 중이다.
  [#244](https://github.com/dev-universe/fender-studio-pro-mcp/pull/244)
  [#228](https://github.com/dev-universe/fender-studio-pro-mcp/pull/228)
- R452C는 실제 8.1.1에서 dry 및 Chorus A/B/복귀 A의 24초 스템을
  네 개 내보냈다. 기본 PCM 판독과 dry 보정 통과. 플러그인 설정 전체의
  RMS는 dry −27.010 dBFS, active 약 −16.18 dBFS였다. 왼쪽 변조
  기준 미달로 Speed 간격 판정 중단. 오른쪽 최강 변조 성분은 세 단계
  모두 3 Hz였다. 안정값 89개와 동일 2,297바이트 캡처를 복구했다.
  148회·218.501초, 앱 254.312초, No Save, 설치 18 및 설정 11개 원복.
  [#247](https://github.com/dev-universe/fender-studio-pro-mcp/pull/247)
- R457은 Ghidra 22함수와 Craft 13범위로 VST3 컨트롤러의 값 비교 후
  setter 경로를 닫았다. 알림에 호출 결과와 원래 처리점의 대응이 없다.
  실제 값 유지 7건은 열려 있다. 8분 49초.
  [#249](https://github.com/dev-universe/fender-studio-pro-mcp/pull/249)
- R458은 기존 Send의 typed destination 지정과 상호 포트 추가를
  확인했다. 현재 문서에서 source Send와 target Scope 입력을 함께 읽는
  수명 보장 경계는 R460에 남겼다. 약 9분 30초.
  [#248](https://github.com/dev-universe/fender-studio-pro-mcp/pull/248)
- 다음 실앱은 R459의 실제 Serum instrument MIDI 입력 및 독립 조회다.
  CUA와 앱·MCP owner 종료, 정상 관리 설치 확인. `main` 16개·로컬
  11개·장비 4건, 전체 2~5일과 CI·장비 대기, 지정 종료 시각 없음.

## 11월 26일 05시 55분 Serum MIDI 입력과 FX 목적지

- R459는 공식 2.1.5 Serum 2에서 가상 CoreMIDI의 CC1, 압력, 벤드
  세 가지가 실제 대응 DSP값을 움직이는 것을 확인했다. CC1 및 압력은
  정확히 0으로 돌아왔다. 새 읽기 전용 owner와 가상 입력 종료 뒤
  전체 2,623행 중 `vstp516000001`만 MIDI 중앙값
  `5000.30517578125`에 남았다. 160회·56.565초와 재확인 38회·4.741초.
  No Save, 스캐너 원복, 관리 18 및 설정 11개 원복.
  [#250](https://github.com/dev-universe/fender-studio-pro-mcp/pull/250)
- R461은 같은 8.1.1의 별도 Audio 트랙에 Serum 2 FX를 삽입했을 때
  Music 트랙의 Out 장치 목록이 `없음, Serum 2`에서 `없음, Serum 2,
  Serum 2 FX`가 되는 것을 확인했다. 선택·라우팅 변경·MIDI 발신 0회,
  각 DSP 2,623행 유지, 141회·83.855초, No Save와 18+10개 원복.
  [#251](https://github.com/dev-universe/fender-studio-pro-mcp/pull/251)
- #228의 최신 `1a14332`에서 필수 검사 두 개를 통과해 `9ee5ea0`에
  병합했다. 12분 58초. #230은 최신 `main` 기반 `21e001f` 검사 중이다.
  [#228](https://github.com/dev-universe/fender-studio-pro-mcp/pull/228)
  [#230](https://github.com/dev-universe/fender-studio-pro-mcp/pull/230)
- R460은 보유되는 Send 설정 포트와 Scope 선택 포트의 정규 객체
  동일성을 비교할 후보를 확인했다. 컴파일 대조를 포함해 약 12분 30초.
  R463에서 작은 비공개 후보와 실앱 시험을 준비한다. 다음 실앱은 R462의
  Serum 2 FX 목적지 선택과 실제 MIDI 수신이다.
- 앱·owner·가상 신호 종료, 사용자 관리 설치와 프로필 원복 확인.
  `main` 16개·로컬 11개·장비 4건. 전체 2~5일과 CI·장비 대기이며
  지정 종료 시각은 없다.

## 11월 26일 06시 02분 Serum FX 실제 MIDI 수신

- R462는 single-item Insert/Send 응답의 형식을 원래 객체로 비교해
  27회·24.975초에서 멈췄다. 라우팅 수정과 MIDI 전송은 0회였다.
  분리한 R462B는 실제 배열 응답을 검증하고 stock Out 메뉴의 exact
  Serum 2 FX를 선택해 CC1·압력·피치 벤드를 보냈다. 각 활성 13회와
  중립 3회가 일치했다. 라우팅·모니터 원복 뒤 악기 2,623개는 같았고
  FX는 피치 벤드 MIDI 중심 `5000.30517578125` 한 행만 달랐다.
  212회·111.695초. No Save, 앱·owner·가상 입력 종료, 18+11개 원복.
  [#253](https://github.com/dev-universe/fender-studio-pro-mcp/pull/253)
- #230의 최신 기준 `21e001f`가 두 필수 검사를 통과해 `e121fe8`에
  병합됐다. 9분 4초. #231의 `da8c1b1`이 새 `main`을 검사한다.
  [#230](https://github.com/dev-universe/fender-studio-pro-mcp/pull/230)
  [#231](https://github.com/dev-universe/fender-studio-pro-mcp/pull/231)
- R460의 Sidechain 보유 객체 대조 후보는 #252에 기록했다.
  R463은 실제 후보 구현, R464는 기존 Ampire 스템의 독립 분석,
  R465는 Serum 2 FX의 현재 MIDI 포트 소유자를 MCP에서 확인하는
  경로를 조사한다. 로컬 11개·`main` 16개·장비 4건, 전체 2~5일과
  CI·장비 대기. 지정 종료 시각 없음.
  [#252](https://github.com/dev-universe/fender-studio-pro-mcp/pull/252)

## 11월 26일 06시 13분 Pedalboard 저장·재열기 닫힘

- #231 최신 `da8c1b1`에 두 필수 검사 통과, 원격 `377d002` 병합.
  11분 35초. #228/230/231 및 앞선 분기·캡슐 근거가 모두 반영돼
  SHARED-PERSISTENCE를 고정 단위에서 닫았다.
  [#231](https://github.com/dev-universe/fender-studio-pro-mcp/pull/231)
- R464는 R452C 오른쪽 반송파의 약 1.5132 Hz 기본 간격과 약
  3.0264 Hz 최강 두 번째 성분을 세 스템에서 대조했다. 모두 Chorus
  모드였으므로 R466이 Vibrato 모드 독립 대조를 준비한다. 10분 27초.
  [#254](https://github.com/dev-universe/fender-studio-pro-mcp/pull/254)
- R465는 REA/Ghidra의 41개 진입점과 Craft Anything의 7범위로
  Music 출력의 retained getter와 AudioFXMusicUnit 소유 관계를 확인했다.
  현재 포트에서 정확한 인서트까지의 공개 연결자는 검증되지 않았다.
  [#255](https://github.com/dev-universe/fender-studio-pro-mcp/pull/255)
- R463은 실제 Sidechain 설정 대상의 정규 객체 동일성 비교를 준비한다.
  앱·owner·임시 플러그인·MIDI 입력 없음, 정상 관리 설치 확인.
  새 CoreAudio/CoreMIDI 장치 목록에도 Volt 276·Keystation88 없음.
  `main` 15개·로컬 11개·장비 4건. 전체 2~5일과 CI·장비 대기,
  지정 종료 시각 없음.

## 11월 26일 06시 25분 Sidechain 경고와 후속 검증

- R463의 29번째 요청에서 비공개 진단에 필요한 보조 객체를 공용 스크립트가
  거부했다. 잘못된 슬롯과 소스는 먼저 거부됐고 Send나 Save는 실행하지 않았다.
  앱과 owner를 종료하고 관리 파일 18개·프로필 10개를 원복했다. R463B에서
  실제 BridgeCore → BridgeComponent → NativePluginControls 경로로 원래
  경고를 재현했다. 수정 뒤 정상 요청과 예외 입력을 포함한 집중 검사 6건이
  통과했다. 정확한 Studio Pro 빌드에서 재검증한다.
- R466은 실제 dry 스템을 분석한 뒤 Ampire 편집기에서 스톰프 바이패스
  상태 변화가 보여 Mode 전환 전에 중단했다. 79회·126.828초. No Save,
  관리 파일 18개·프로필 11개 원복. stock 스킨으로 확인한 Pedalboard
  공개 컨트롤은 상단 Amp 선택 왼쪽의 아래쪽 버튼이다.
- #250의 최신 기준 `d8063b6`가 두 필수 검사에 통과해 `5cfb17a`에
  병합됐다. 9분 49초. #251의 `d11b24d`가 새 `main`을 검사 중이다.
  R467은 Serum 2 FX로 지정된 MIDI 출력과 같은 모델의 별도 인서트를
  저장·재열기 후 구분할 수 있는지 준비한다.
  [#250](https://github.com/dev-universe/fender-studio-pro-mcp/pull/250)
  [#251](https://github.com/dev-universe/fender-studio-pro-mcp/pull/251)
- 앱과 owner를 닫은 경계다. `main` 15개·로컬 11개·장비 4건. 전체 2~5일과
  CI·장비 대기이며 지정 종료 시각은 없다.

## 11월 26일 06시 54분 Serum FX 저장·재열기와 Ampire 실측

- R463B에서 종전의 29번째 요청이 실제 8.1.1에서 일치하지 않는 Scope를
  명시적으로 거부했고, 편집기는 오류 없이 열렸다. 소스 목록을 확인하지
  못해 Send 추가 전 중단했다. 33회·76.598초, No Save, 18+10개 원복.
  [#256](https://github.com/dev-universe/fender-studio-pro-mcp/pull/256)
- #251과 #253은 각 최신 헤드의 두 필수 검사를 통과해 `81f9d27`과
  `8da4a28`에 병합됐다. 각각 609.9초와 572.3초였다.
  [#251](https://github.com/dev-universe/fender-studio-pro-mcp/pull/251)
  [#253](https://github.com/dev-universe/fender-studio-pro-mcp/pull/253)
- R467에서 Serum 2 FX MIDI 목적지가 같은 모델의 두 번째 인서트와
  구분된 채 실제 저장·재열기 뒤에도 유지됐다. Music 트랙에 `kind=instrument`
  DSP 판독을 하면 현재 Out의 FX 인스턴스로 이어졌다. exact 인서트 판독 및
  저장 `MusicInput00` Device UID가 모두 일치했다. 소유한 시험곡만 두 번 저장.
  `song_read`와 `song-read`의 실제 아카이브 검증 및 로컬 `scripts/check.sh`
  통과. 라이브 owner 50+24+16+16회. 원래 설치 18개·프로필 10개 원복.
  다음은 기존 MCP 판독의 현재 경로와 잘못 지정한 같은 모델의 분리 검증이다.
  [#257](https://github.com/dev-universe/fender-studio-pro-mcp/pull/257)
- R466B는 실제 Vibrato의 오른쪽 Sweep 간격 5.9821→9.7137→5.9866 Hz를
  두 계산 경로로 검증했다. 왼쪽은 기준 미달이다. 복귀한 Chorus·전원·속도와
  89개 제어값은 정확했고, 같은 크기의 불투명 상태 해시가 달라 완전 복원을
  인정하지 않았다. 182회·280.753초. No Save, 18+12개 원복.
  [#258](https://github.com/dev-universe/fender-studio-pro-mcp/pull/258)
- 앱·owner는 닫혔고 임시 서명 플러그인은 제거됐다. `main` 15개·로컬 11개,
  이 중 장비 4건. 전체 2~5일과 CI·장비 대기, 지정된 종료 시각 없음.

## 11월 26일 07시 20분 Serum 읽기 병합과 Scope·Ampire 경계

- #255, #257, #259가 각각 780.5초, 555.9초, 573.1초에 최신 기준의
  두 필수 검사를 통과해 병합됐다. #257이 `song_read`의 저장 목적지
  필드를 제공한다. R468의 실제 `plugin_inspect`는 같은 모델의 다른
  인스턴스에 `host_rejected`, 이전 문서에 `document_changed`를 반환했다.
  33회·22.191초, 쓰기와 Save 없음, 원래 설치 18개·프로필 9개 원복.
  [#255](https://github.com/dev-universe/fender-studio-pro-mcp/pull/255)
  [#257](https://github.com/dev-universe/fender-studio-pro-mcp/pull/257)
  [#259](https://github.com/dev-universe/fender-studio-pro-mcp/pull/259)
- R463C는 빈 Send의 여러 직접 노드로 중단했고 R463D는 실제
  `AudioSendSlot`만 구분해 Scope의 선택 Variant 거부까지 진행했다.
  R463E는 제한된 `IListParameter::isEmpty`에서 정확한 선택 목록이
  비어 있음을 확인했다. 세 번 모두 40요청, 20.104/30.175/25.592초,
  이전 스크립팅 팝업 재발 없이 Send·Save 0, 각 18+10 원복.
  빈 선택기를 대신할 독립 보유 경로는 열려 있다.
  [#260](https://github.com/dev-universe/fender-studio-pro-mcp/pull/260)
- R469의 FAT 50→75→50%는 전체 FAT 상태 해시와 함께 돌아왔다.
  전역 Volume/Pitch는 0·빈 표시였다. 빈 슬롯의 활성 값은 남았다.
  R470 최신 `ccfaec0` 빌드는 16개 선택지와 같은 변화를 확인했다.
  캡슐 복원 뒤 빈 슬롯 활성값은 원래대로 돌아왔고 micmix A는 Float32
  1 ULP 차이, 캡슐 2,294/2,297바이트였다. 확정적인 전체 원복은 미완료다.
  R469 95회·11.191초, R470 83회·7.874초. No Save, 각각 18+10 원복.
  [#261](https://github.com/dev-universe/fender-studio-pro-mcp/pull/261)
- 현재 #256의 최신 head 검사 중이다. 다음 #245, #249 FabFilter 이후
  로컬 적격 Input Filter, Impact, Chorder, Shared Event를 우선 병합한다.
  07시경 목록에서 Volt 276과 Keystation 88은 관찰되지 않았다.
  앱·owner 닫힘, 정상 설치 확인. `main` 15개·로컬 11개·장비 4건.
  전체 2~5일과 CI·장비 대기, 지정 종료 시각 없음.

## 11월 26일 07시 29분 Scope 경고와 Serum 매핑 후속

- #256은 최신 head의 두 필수 검사에 통과해 `c3d8ae2`에 병합됐다.
  갱신부터 병합까지 553.3초였다. 경고를 발생시킨 R463 요청은 Send와
  Save 전에 중단했다. 수정한 R463B는 동일한 불일치를 팝업 없이
  `host_rejected`로 전달했다. [#256](https://github.com/dev-universe/fender-studio-pro-mcp/pull/256)
- R471은 REA·Ghidra 여섯 함수와 LLVM·Craft-Anything 바이트를 대조했다.
  주 경로는 선택된 입력의 타입을 검사한다. 빈 선택의 대체 경로는 효과의
  비공개 입력 배열에서 첫 항목을 빌린다. 정확한 입력을 독립적으로 찾아
  소유권을 유지하는 방법은 확인되지 않았다. 주 조사 242초, 실앱 0회.
  [#260](https://github.com/dev-universe/fender-studio-pro-mcp/pull/260)
- R472에서 Xfer 서명을 다시 검증한 2.1.5 VST3 두 클래스를 각각 격리했다.
  각 프로세스는 기존 채널의 130개와 다음 채널의 첫 결과를 내고
  비정상 종료했다. 두 프로세스 합계 1.015초. 첫 번째 샌드박스 초기화
  오류는 플러그인 실행 전에 수정했다. MCP stdio는 260개를 정확히
  반환했고 두 종류에서 다른 채널을 명시한 30건 등 35건을 거부했다.
  MCP 검사 1.237초, 종료 코드 0, Studio Pro 실행 없음.
  [#262](https://github.com/dev-universe/fender-studio-pro-mcp/pull/262)
- #245 재검사가 진행 중이다. 이후 #249, #262, #260을 최신 `main` 기준으로
  검사한다. 앱·owner 닫힘, 정상 설치 확인. `main` 15개·로컬 11개,
  그중 장비 4건. 전체 2~5일과 CI·장비 대기, 지정 종료 시각 없음.

## 11월 26일 07시 35분 FabFilter 상태

- #245의 최신 기준 `a5648e8`이 필수 검사 두 개에 통과해
  `f268dcd`로 병합됐다. 551.0초. #249의 최신 기준 `13b5440`이
  필수 검사를 진행한다. [#245](https://github.com/dev-universe/fender-studio-pro-mcp/pull/245)
  [#249](https://github.com/dev-universe/fender-studio-pro-mcp/pull/249)
- R473은 서명이 확인된 FabFilter Pro-G 1.42 arm64의 독립 SDK
  프로세스 두 개를 비교했다. 두 처리부 모두 같은 음수 피드백을
  내보냈다. 전달을 생략한 대조군은 0.035, 전달한 쪽은 반환 성공과
  독립 값 0을 확인했다. 두 컴포넌트는 원래 바이트를 되찾았다.
  0.228초와 0.222초, 둘 다 종료 코드 0. Studio Pro에서 실제 전달된
  값과 정지 중에는 어느 경로가 동작했는지 열린 상태다.
  [#263](https://github.com/dev-universe/fender-studio-pro-mcp/pull/263)
- 다음은 #249 후 #262, #260, #263, 그 뒤 Input Filter·Impact·Chorder와
  Shared Event다. `main` 15개·로컬 11개, 장비 의존 4건. 전체 2~5일과
  CI·장비 대기, 지정 종료 시각 없음. 앱과 owner는 닫혀 있다.

## 11월 26일 07시 40분 Ampire 산술 대조와 Serum 사용 설명

- R474는 R470의 같은 인스턴스에서 6회 읽은 값을 다시 대조했다. A만
  직접 되돌리면 B·C도 움직이는 것을 기존 독립 Python과 arm64 C 산술로
  확인했다. Link를 껐을 때의 보유 값을 먼저 읽어야 하며, R470 상태에서
  R427 복구 순서와 캡슐 복귀는 실앱 미확인이다. 오프라인 1초 미만.
  원래 설치 18개와 앱·owner 종료도 재확인했다.
  [#261](https://github.com/dev-universe/fender-studio-pro-mcp/pull/261)
- R475는 R467 실제 아카이브를 저장 읽기 CLI와 `song_read`로 검사했다.
  같은 모델 두 개 중 대상 인서트 하나의 UUID, 채널, 슬롯이 일치했고
  파일은 그대로였다. 0.634초. 실앱 경로와 저장 경로의 확인 방법을
  README에 연결했다. 전후 아카이브 두 개를 각각 CLI로 읽어 기존
  Serum 2와 지정 Serum 2 FX가 각자의 정확한 Device UID로 연결됨도
  확인했다. 읽기 합계 0.014초, 두 파일 모두 바이트가 동일했다.
  [#264](https://github.com/dev-universe/fender-studio-pro-mcp/pull/264)
- 현재 대기 순서는 #249, #262, #264, #260, #263 이후 Input Filter다.
  `main` 15개·로컬 11개, 장비 의존 4건. 전체 2~5일과 CI·장비 대기,
  지정 종료 시각 없음.

## 11월 26일 07시 46분 FabFilter 우선 병합 완료

- #249의 최신 기준 `13b5440`이 필수 검사 두 개를 통과해
  `cbc24ee`에 병합됐다. 갱신부터 병합까지 817.3초.
  [#249](https://github.com/dev-universe/fender-studio-pro-mcp/pull/249)
- #262부터 새 `main` 기준으로 검사한다. 그 뒤 #264, #260, #263,
  Input Filter, Impact, Chorder, Shared Event 순이다. 로컬 병합 점검
  프로세스는 실행 중이다. 앱과 MCP owner 닫힘 및 원래 관리 파일
  18개는 독립 해시를 다시 확인했다.
- `main` 15개·로컬 11개, 이 안에 장비 의존 4건. 다음 검사 9~14분,
  전체 2~5일과 CI·장비 대기 예상. 지정 종료 시각 없음.

## 11월 26일 09시 03분 Input Filter 병합과 Ampire 복원

- 재개 때 #262, #264, #260, #263, #173, #174의 병합과 각 필수
  검사 두 개를 원격에서 확인했다. #175도 최신 기준 `06a9579`에서
  두 검사를 통과해 `ad31cb7`에 병합됐다. 갱신부터 818.5초였다.
  Input Filter는 `main` 적격으로 옮겨 고정 잔여가 15→14개다.
  [#175](https://github.com/dev-universe/fender-studio-pro-mcp/pull/175)
- R476은 Xfer 서명 Serum 2 2.1.5 arm64 VST3의 instrument와 FX에
  같은 Note Expression 인터페이스 진입점 두 개가 있음을 확인했다.
  격리 조회는 두 클래스 모두 지원 목록을 얻기 전에 종료했다.
  REA·Ghidra와 Craft-Anything·LLVM로 진입점 두 곳을 맞췄다.
  분석 약 173.9초, Studio Pro 0회, 이벤트 쓰기 0회.
  [#265](https://github.com/dev-universe/fender-studio-pro-mcp/pull/265)
- R477은 8.1.1에서 FAT 순환과 캡슐 적용 후의 마이크 A 차이를
  재현했다. Link을 끈 후 A가 준비값에 있어 B만 조정했다. Link을
  다시 켜고 C를 복귀시켜 설정 89개 및 원래 2,297바이트 캡슐 해시를
  맞췄다. 숨은 Link 해제 기준값은 최종 후 미확인이다. 처음에는
  장치 재연결 대화상자로 두 요청에서 멈췄고 설정을 변경하지 않고
  넘긴 뒤 새 owner를 붙였다. 125요청·12.182초, 앱 72.317초.
  No Save, 종료 코드 0, 관리 파일 18개·프로필 10개 원복.
  [#266](https://github.com/dev-universe/fender-studio-pro-mcp/pull/266)
- #265의 현재 기준 `8958d0a` 검사 중. 다음 #266과 Impact·Chorder·
  Shared Event 순이다. 현재 앱·owner 닫힘. 08시 54분 장치 목록에서
  Volt 276 및 Keystation 88은 관찰되지 않았다. `main` 14개·로컬
  11개, 장비 의존 4건. 다음 검사 9~14분, 전체 2~5일과 CI·장비 대기.
  지정 종료 시각 없음.

## 11월 26일 09시 11분 Serum Macro 16개 실제 검증

- #265의 최신 기준 `8958d0a`는 필수 두 검사 통과 후
  `179cfeb`에 병합됐다. 갱신과 병합 568.7초. 동작 범위는
  SDK 인터페이스와 정적 진입점이며 Note Expression 실제 전달은
  열려 있다. [#265](https://github.com/dev-universe/fender-studio-pro-mcp/pull/265)
- R478은 Xfer 서명 2.1.5 VST3 instrument와 FX를 실제 8.1.1에
  하나씩 넣고 Macro 1~8을 각 0→25%→0으로 확인했다. 독립 판독으로
  매번 대상 행 한 개만 변했고 상대의 2,623개 값이 그대로였음을
  확인했다. 각 원복 뒤 양쪽 2,623행이 모두 정확했다. 32쓰기,
  1,917요청·206.946초, 앱 235.027초. MIDI·재생·Save 각 0.
  Studio Pro와 owner 정상 종료, 임시 서명 VST3 제거, 관리 파일
  18개·프로필 10개 원복. [#267](https://github.com/dev-universe/fender-studio-pro-mcp/pull/267)
- #266의 최신 기준 `371874e` 검사 중이다. 그 뒤 #267·Impact·
  Chorder·Shared Event. `main` 14개·로컬 11개, 장비 의존 4건.
  다음 검사 9~14분, 전체 2~5일과 CI·장비 대기. 지정 종료 시각 없음.

## 11월 26일 09시 20분 Serum 프리셋 파일 조회 구현

- R479는 읽기 전용 `serum2_preset_inspect`를 추가한다. 2.1.5
  스키마 11의 editor `.SerumPreset`과 처리부 전용 `.vstpreset`을
  구분하고 제품과 SHA-256, 각 적용 방식을 알려 준다. 실제 네이티브
  두 파일이 Zstandard 프레임 크기 필드를 생략해 해당 형식의
  헤더 검사에만 허용했다. 본문 해석이나 호스트 적용은 별도다.
- 앞서 확보한 두 형식·두 제품 4파일을 MCP stdio에서 맞췄고,
  확장만 바꾼 파일·심볼릭 링크·용량 초과·상대 경로를 거부했다.
  파일 SHA-256은 그대로였다. 구현 브랜치 생성부터 커밋까지
  6분 47초, 실제 파일 확인 1.564초. Rust 269개·JS 441개 통과,
  Rust 1.88 Clippy와 포맷 통과. 앱과 owner 닫힘.
  [#268](https://github.com/dev-universe/fender-studio-pro-mcp/pull/268)
- #266의 최신 기준은 macOS 통과·Core 대기다. 이어 #267·#268·
  Impact 순으로 검사한다. `main` 14개·로컬 11개, 장비 의존 4건.
  다음 검사 9~14분, 전체 2~5일과 CI·장비 대기. 지정 종료 시각 없음.

## 11월 26일 09시 21분 Ampire 실앱 기록 병합

- #266은 `371874e`의 필수 검사 두 개를 통과해 `e5c3e29`에
  병합됐다. 갱신부터 병합까지 692.3초. #267은 최신 `main`을
  포함한 `857a943` 기준으로 필수 검사를 시작했다.
  [#266](https://github.com/dev-universe/fender-studio-pro-mcp/pull/266)
  [#267](https://github.com/dev-universe/fender-studio-pro-mcp/pull/267)
- #268의 Python 오프라인 검사 560개도 통과했다. Rust 269개,
  JS 441개와 함께 로컬 확인을 마쳤다. 다음은 #267, #268, Impact,
  Chorder, Shared Event 순이다. 병합 점검 프로세스는 실행 중이다.
- 앱과 owner 닫힘, 원본 설치 관리 파일 18개 확인. `main` 14개·
  로컬 11개, 장비 의존 4건. 다음 검사 9~14분, 전체 2~5일과
  CI·장비 대기. 지정 종료 시각 없음.

## 11월 26일 09시 25분 Serum 프리셋 조회와 실제 적용 검증

- R480의 8.1.1 시험곡에서 후보 새 MCP의 조회 SHA-256을 현행 적용
  도구에 보내 정확한 악기·FX 인스턴스에 한 번씩 전달했다. 두 영수증은
  해시와 인스턴스가 맞았다. 원본 캐리어는 바이트가 같았다.
- 첫 비교는 FX 값과 접근 속성 변경을 섞어 29행으로 세고 멈췄다.
  추가 쓰기 없이 현재 문서·세션·인스턴스를 고정한 새 owner의
  독립 판독에서 악기 값 15개·FX 값 13개와 FX 접근 속성만 변경된
  16개를 구분했다. 접근 속성 변경은 쓰기 가능→읽기 전용이다.
  각 대상의 2,623개 판독은 정착 뒤 일치했다. 전체 229요청·25.807초,
  앱 106.606초. No Save·MIDI·재생. 정상 종료 후 공식 임시 VST3 제거,
  설치 18개·프로필 10개 원복. [#268](https://github.com/dev-universe/fender-studio-pro-mcp/pull/268)
- #267의 정확한 `857a943` 기준 필수 검사 두 개 대기. 다음 #268의
  최신 브랜치 `6f84832`는 새 `main`에서 다시 확인한다.
  앱과 owner 닫힘. `main` 14개·로컬 11개, 장비 의존 4건.
  다음 검사 9~14분, 전체 2~5일과 CI·장비 대기. 지정 종료 없음.

## 11월 26일 09시 31분 Serum Macro 병합과 Chorder 충돌 대비

- #267은 최신 기준 `857a943` 두 필수 검사 통과 후 `409f03d`에
  병합됐다. 546.0초. #268의 최신 기준 `3171ac2`로 두 검사 진행 중이다.
  해당 헤드의 제품 소스가 로컬 검사와 R480에 사용한 소스와 같았다.
  [#267](https://github.com/dev-universe/fender-studio-pro-mcp/pull/267)
  [#268](https://github.com/dev-universe/fender-studio-pro-mcp/pull/268)
- #177과 당시 `main`의 네 충돌에서 Chorder 가드와 Multi·Splitter
  가드를 모두 살렸다. 로컬 `integration/chorder-audition-main`
  `bf5096b`, 2분 50초, Rust 273개·JS 443개·네이티브 509개 통과.
  #268과 함께 검사한 로컬 `integration/chorder-preset-check`의
  `02f11c3`은 도구 수를 65로 맞춰 Rust 277개가 통과했다. 출시할 때
  #268이 실제로 `main`에 포함됐는지 먼저 확인하고 최신 기준을
  합친 뒤 #177의 기존 브랜치에 반영한다. 원격 #177은 아직 미변경이다.
  [#177](https://github.com/dev-universe/fender-studio-pro-mcp/pull/177)
- #191–194는 #268의 현재 트리와 병합 충돌이 없음을 미리 확인했다.
  앱·owner 닫힘 및 관리 파일 18개 원래 해시 확인. `main` 14개·
  로컬 11개이며 장비 의존 4건, 다음 검사 9~14분, 전체 2~5일과
  CI·장비 대기다. 지정 종료 시각 없음.

## 11월 26일 09시 39분 Serum 프리셋 조회 병합과 Chorder 갱신

- #268의 최신 `3171ac2`가 필수 두 검사를 통과해 `2323a11`에
  병합됐다. 547.6초. `serum2_preset_inspect`가 `main`에 있다.
  [#268](https://github.com/dev-universe/fender-studio-pro-mcp/pull/268)
- 정확한 새 `main`을 합친 Chorder 트리가 로컬에서 277 Rust 검사를
  통과한 통합 트리와 동일했다. 기존 #177 브랜치를 fast-forward로
  `8d07e42`까지 갱신했다. 대기열에서 차례가 오면 그때의 `main`을
  기준으로 필수 검사 두 개를 확인한다. [#177](https://github.com/dev-universe/fender-studio-pro-mcp/pull/177)
- #191은 `a6008e2` 기준 검사 중이며 #192–194가 이어진다.
  로컬 병합 점검 프로세스 실행 중. Studio Pro·owner 종료,
  관리 파일 18개 원본 해시 확인. 신선 장치 목록에서 Volt 276과
  Keystation 88이 관찰되지 않았다. `main` 14개·로컬 11개, 장비
  의존 4건. 다음 검사 9~14분, 전체 2~5일과 CI·장비 대기,
  지정 종료 시각 없음.

## 11월 26일 13시 15분 대기열 재개와 새 실앱 검증

- 09시 39분 이후 Impact #191–194와 Chorder #160–164가 필수 두 검사를
  통과해 순서대로 병합됐다. 앞선 자동 병합은 #163 뒤에 #164의
  `BridgeCore.js` 충돌로 종료됐다. Chorder Clear와 빈 Splitter 가드를
  함께 보존했고 JS 443개 통과 후 `b28ba1d`를 갱신했다. 최신 기준
  두 검사를 통과해 #164가 `e4f6c61`에 639.6초 만에 병합됐다.
  [#164](https://github.com/dev-universe/fender-studio-pro-mcp/pull/164)
- R482는 8.1.1에서 FAT 캡슐 복원 뒤 Link를 한 번 더 껐다. Link가
  켜졌을 때 원본 89행과 2,297바이트 캡슐 SHA-256이 같았다. Link를
  끄자 A·B는 공장 첫 Link-off보다 Float32 1 ULP씩 낮았고 C는 같았다.
  135요청·12.437초, 앱 38.308초. No Save, 관리 18개와 프로필 10개 원복.
  [#269](https://github.com/dev-universe/fender-studio-pro-mcp/pull/269)
- R483은 SDK의 같은 노트·네 매크로 점에서 배선 유무를 대조했다.
  R484는 8.1.1에서 Serum 2 Macro 1 → Osc A Level을 정확한 MCP
  인스턴스에 적용했다. 같은 C4 세 번의 양쪽 내부 미터가 약
  839 → 5,248 → 839였고 최종 2,623행이 일치했다. 743요청·117.528초,
  앱 136.924초. No Save, 관리 18개와 프로필 10개 원복, 서명 번들
  스캔 경로에서 제거. 다음 Serum 단위는 FX 매크로 대상 확인이다.
  [#270](https://github.com/dev-universe/fender-studio-pro-mcp/pull/270)
- #269는 같은 최신 `main`의 `25d0595` 두 검사 진행 중이며 #270이
  이어진다. 이어서 #165–167 및 #177을 진행한다. 가상 #167 결과의
  #177 통합 트리 `b9bde5a`는 Clear와 Audition 테스트를 모두 보존했고
  Rust 278, JS 445, 네이티브 510개를 Rust 1.88 경로에서 통과했다.
  실제 #177 헤드는 병합 차례의 원격 `main`에서 갱신한다. 이후
  #212–226의 앞선 12개도 가상 대기열에서 충돌이 없었다.
- 앱과 owner 종료, 관리 18개 원래 해시 확인. Volt 276과 Keystation 88은
  최신 장치 목록에 없다. `main` 13단위, 로컬 11단위, 장비 의존 4건.
  다음 CI 9~14분, 전체 2~5일 및 장비 대기. 지정 종료 시각 없음.

## 11월 26일 13시 48분 Serum FX 병합과 Rumble 이어가기

- #269는 `25d0595`의 필수 두 검사 통과 뒤 `14d4b3b`에 병합됐다.
  최신 기준 갱신부터 692.8초. #270은 `bef28b9`에서 통과해
  `ea42d06`에, #271은 `1bd0930`에서 통과해 `f1bbaa7`에 병합됐다.
  [#269](https://github.com/dev-universe/fender-studio-pro-mcp/pull/269)
  [#270](https://github.com/dev-universe/fender-studio-pro-mcp/pull/270)
  [#271](https://github.com/dev-universe/fender-studio-pro-mcp/pull/271)
- R485는 공식 Serum 2 FX의 Distortion level 필드를 REA/Ghidra와
  Craft-Anything에서 대조했다. 같은 SDK의 입력 및 대조 패치에서
  Macro 1 배선의 출력 변화와 복원을 확인했다. R486의 실제 8.1.1
  Matrix는 `Macro 1 → FX 1: Dist Level`을 표시했고 내부 비율은
  0.862 → 0.138 → 0.862였다. 689요청·125.184초, 앱 136.653초,
  DSP 2,623행과 설치 18개·프로필 10개 원복. 스캔 경로 임시 번들 제거.
- R487–489에서 번호가 붙은 Rumble의 상태 읽기를 거부한 두 경로를
  실제 호스트와 코드에서 추적했다. R490 후보 `59ac9e0`는 세 동일
  모델 인서트의 슬롯 26개를 각각 읽고 교차 UID를 거절했다. 186요청·
  19.992초, 앱 92.592초. 정상 설치·프로필 원복. 해당 제품 수정은 #272다.
- R491은 001→100→001을 정확한 중간 인스턴스에서 독립 확인했다.
  440 Hz 출력/입력 1.12123→2.31654→1.12258, 복원 후 100.12%.
  안정 DSP 35행·수치 22개와 두 피어는 일치했으며 모델 ID 두 개는
  남았다. 468요청·50.605초, 앱 80.567초, 저장 없음, 18+10 원복. #273.
- R492는 시험기에서 이전 톤 순서를 요구해 Save 전에 중단했다.
  R493은 소유 곡을 한 번 저장한 뒤 쓰기를 차단한 새 프로세스에서
  열었다. 26개 모델 ID와 대상 DSP 35행은 일치했다. 세 인스턴스의
  같은 수치 설정 5개가 Float64 1~2 ULP씩 달랐다. 123+11+22요청,
  29.449+1.827+2.561초. 두 앱 78.618·67.542초, 18+10 원복. #274.
  [#272](https://github.com/dev-universe/fender-studio-pro-mcp/pull/272)
  [#273](https://github.com/dev-universe/fender-studio-pro-mcp/pull/273)
  [#274](https://github.com/dev-universe/fender-studio-pro-mcp/pull/274)
- #272 최신 `325b761` 두 검사 진행 중. 이어 #273, #274,
  Chorder #165–167 및 #177 순이다. #177의 실원격 `fbf130c`는
  합성 통합 트리와 제품 소스·검사 입력이 일치한다. #172 `b2b6a4f`와
  #184 `3915486`의 예상 문서 충돌을 해결하고 원격 브랜치를 갱신했다.
  다음 작업은 실제 Rumble의 이름 기반 모델별 제어 조회다.
- 앱과 owner 닫힘, 정상 관리 18개 확인. `main` 13단위, 로컬 11단위,
  장비 의존 4건. 다음 검사 5~9분, 전체 2~5일과 장비 대기.
  지정 종료 시각 없음.

## 11월 26일 14시 10분 Rumble 병합과 중첩 Splitter 교차 이동

- #272와 #273의 갱신 head에서 필수 검사 두 개가 통과했다.
  병합 커밋은 각각 `efa74c8`과 `26a32b6`이며 준비부터 753.5초와
  545.4초였다. #274는 새 head `7cf16d5`에서 검사 중이다.
  [#272](https://github.com/dev-universe/fender-studio-pro-mcp/pull/272)
  [#273](https://github.com/dev-universe/fender-studio-pro-mcp/pull/273)
  [#274](https://github.com/dev-universe/fender-studio-pro-mcp/pull/274)
- R494는 Rumble 프리셋 001→100→001의 각 상태에서 이름과 인덱스를
  대조했다. 26개 인덱스와 이름 묶음 22/39/22에서 선택 항목이 0이었다.
  101요청·8.252초, 앱 59.571초, 저장 없음, 설치 18개와 프로필 10개 원복.
  [#275](https://github.com/dev-universe/fender-studio-pro-mcp/pull/275)
- R495는 Ampire FAT 복원 뒤 Link-on 상태 89행과 2,297바이트 캡슐을
  맞췄다. 한 번 저장하고 쓰기를 막은 별도 프로세스에서 Link를 해제하니
  마이크 A·B·C가 공장 기준과 비트 단위로 일치했다. 재열기 첫 SDK
  요청은 CLI pin 누락으로 schema 단계에서 전달 전에 거절됐으며, 일치하는
  CLI로 재확인했다. 135+20+28요청·38.546초, 설치 18개·프로필 10개 원복.
  [#276](https://github.com/dev-universe/fender-studio-pro-mcp/pull/276)
- R496은 기존 Mixtool 2를 중첩 Splitter 3의 두 번째 빈 레인으로 옮겼다.
  R497은 다른 레인에서 기존 Mixtool 직후, Splitter 3의 직전으로 같은
  효과를 옮겼다. 각 MCP receipt와 편집기 및 독립 라우팅 읽기를 대조했고
  각 시험에서 세 successor 목록이 바뀌었다. 8노드, 링크 10개, DSP
  144행, 식별자, 믹서·타임라인은 일치했다. 각 85요청·68.406/71.199초,
  앱 131.066/103.972초. 두 시험의 정상 설치 18개·프로필 9개 원복,
  저장 없음, 각 캡처 토큰 6개 반납.
  [#277](https://github.com/dev-universe/fender-studio-pro-mcp/pull/277)
  [#278](https://github.com/dev-universe/fender-studio-pro-mcp/pull/278)
- #177, #172, #184, #187의 예상 충돌을 각 head에서 해결했다.
  #272 직후 합성 병합 점검에서 당시 다음 65개가 충돌 없었다.
  현 병합 대기 67개. #274→#275→#276→#277→#278 이후 Chorder와
  Shared Event 우선 순서를 이어간다. 남은 소프트웨어에서는 Ampire
  전역 Volume·Pitch, Scope 캡처 및 그래프 추가 연결·삭제의 독립
  검증 가능성을 먼저 확인한다.
- 앱·owner 닫힘과 설치 원복 확인. 고정 잔여 `main` 13개·로컬 11개,
  그중 장비 4건. 전체 2~5일과 장비 대기, 지정 종료 시각 없음.

## 11월 26일 14시 25분 Serum 공장 목록 및 편집기 검증

- #274와 #275는 최신 head의 필수 두 검사 뒤 각각 `f0d3bff`,
  `5ab2fbb`에 병합됐다. 배포 준비부터 545.5초·441.4초.
  [#274](https://github.com/dev-universe/fender-studio-pro-mcp/pull/274)
  [#275](https://github.com/dev-universe/fender-studio-pro-mcp/pull/275)
- R498의 읽기 전용 `serum2_factory_presets_list`는 실제 설치 라이브러리
  악기 624개·FX 2개를 독립 파일 판독과 대조했다. 다섯 페이지와 탐색,
  세 거부, 모든 원본 해시 일치. 2.974초. Rust 273·JS 444·Python 560.
  #279는 최신 기준 `04c4eda`에서 두 필수 검사 중이다.
  [#279](https://github.com/dev-universe/fender-studio-pro-mcp/pull/279)
- R499는 Serum 2.1.5 서명본을 실제 8.1.1에 올려, 각 검색 결과를
  공식 편집기의 Load Preset에서 한 번씩 열었다. 두 원본은 2.0.13/
  스키마 5였다. 각각 2,623행에서 값 61·26개와 메타데이터만 23·16개가
  바뀌었고 반대 인스턴스·믹서·타임라인과 파일은 일치했다.
  203요청·177.941초, 앱 226.436초, 외부 MIDI·재생·저장 없음.
  설치 18개·프로필 10개 검증 원복, 임시 서명 VST3 회수.
  [#280](https://github.com/dev-universe/fender-studio-pro-mcp/pull/280)
- #279→#280→#276→#277→#278 순으로 병합하고 이후 원래 Chorder와
  Shared Event 순서를 이어간다. Ampire 전역 Volume·Pitch의 기존
  실제 읽기와 바이너리 대조는 연결된 제어 대상을 확정하지 못했다.
  다음 독립 실앱은 레인 한 개만 채워진 중첩 Splitter 삭제의 호스트
  동작과 Undo다. 앱·owner 닫힘 및 정상 설치 복구 확인.
- 고정 잔여 `main` 13개·로컬 11개, 장비 의존 4건. 병합 대기 67개.
  전체 2~5일과 장비 대기, 지정 종료 시각 없음.

## 11월 26일 14시 34분 Serum 검색 병합과 한 레인 제거 근거

- Serum 검색 도구 #279는 `04c4eda`에서 두 필수 검사 통과 후
  `3b010ad`에 병합됐다. 최신 기준 갱신부터 589.0초. #280은
  새 head `0bf2214`에서 검사 중이다.
  [#279](https://github.com/dev-universe/fender-studio-pro-mcp/pull/279)
  [#280](https://github.com/dev-universe/fender-studio-pro-mcp/pull/280)
- R500은 공개 slot move로 Splitter 3의 두 번째 레인에 효과 하나를
  넣고 기본 제거 메뉴를 사용했다. 노드 8→6, 링크 10→7, 그 레인의
  Mixtool들은 직렬로 남았다. 기본 편집 메뉴 Undo 한 번으로 노드와
  링크, 자식 순서, DSP 144행을 정확히 복구했다. 173요청·141.526초,
  앱 179.961초, 캡처 토큰 14개 해제 및 18+9 원복, 저장 없음.
  [#281](https://github.com/dev-universe/fender-studio-pro-mcp/pull/281)
- `feature/splitter-remove-one-lane`의 로컬 후보는 같은 8노드
  유형, 순서 및 상호 연결, 정확한 피어 UUID를 요구하는 별도 MCP
  제거 명령을 준비한다. 기존 빈 Splitter 명령의 거부·결과 및
  오래된 확장 대비를 재사용한다. 다음 경계 R501은 같은 정확한
  UUID로 원래 빈 구조를 거부한 뒤, 호스트에서 채워진 두 번째
  레인을 제거하고 Undo하는 검증이다. 앱·owner 닫힘과 원복 확인.
- 고정 잔여 `main` 13개·로컬 11개, 장비 의존 4건. 병합 대기 67개,
  전체 2~5일과 장비 대기, 지정 종료 시각 없음.

## 11월 26일 14시 46분 Serum 편집기 근거 병합과 MCP 제거 검증

- #280은 최신 `0bf2214`의 두 필수 검사 뒤 `65b9779`에 병합됐다.
  준비부터 546.7초. #276은 `4a8e9d0`에서 검사 중이다.
  [#280](https://github.com/dev-universe/fender-studio-pro-mcp/pull/280)
  [#276](https://github.com/dev-universe/fender-studio-pro-mcp/pull/276)
- R501의 제품 후보 `98bbace`는 같은 GUID로 빈 두 레인을 지정한
  제거 요청을 전달 전에 `not_applied`로 거부하고, 원래 그래프와
  144행을 독립 확인했다. 둘째 레인을 채운 뒤 한 번 제거해
  `applied_unverified`, 6노드·7연결·직렬 효과를 확인했다.
  실제 Undo 뒤 8노드·10연결·자식 순서·144 DSP 행이 모두 일치했다.
  181요청·120.214초, 앱 154.638초, 토큰 14개 해제, 저장 없음,
  정상 설치 18개·프로필 9개 원복. Rust 273·JS 445·네이티브 511.
  [#282](https://github.com/dev-universe/fender-studio-pro-mcp/pull/282)
- R502는 이전 SDK 호출이 접근했던 특수 두 경로를 높은 채널에서
  제외했다. 서명된 두 플러그인의 나머지 정확한 매핑을 SDK에서
  각각 2,050개 조회했고 두 프로세스 모두 정상 종료했다.
  현재 R499의 실제 8.1.1 호스트 파라미터 및 새 MCP 조회와 대조 중이다.
- 앱·owner 닫힘, 정상 설치 확인. 고정 `main` 13개·로컬 11개,
  장비 의존 4건, 병합 대기 67개. 전체 2~5일과 장비 대기,
  지정 종료 시각 없음.

## 11월 26일 15시 01분 Ampire 원복과 Serum MIDI 조회 병합

- #276은 정확한 head `4a8e9d0`에서 두 필수 검사 통과 후
  `8be43e4`에 병합됐다. 기준 갱신부터 566.2초. FAT 저장본을
  앱 종료 후 다시 열어 Link Off 및 Factory A/B/C를 확인했다.
  [#276](https://github.com/dev-universe/fender-studio-pro-mcp/pull/276)
- R502는 2.1.5 공식 서명 악기와 FX 각각 2,050개 매핑을 검증했다.
  실제 8.1.1 R499 호스트 목록과 모든 MCP 조회 결과가 일치했다.
  높은 채널의 CC1과 피치 벤드는 범위 밖이다. #283은 최신 head
  `a083cd5`의 두 필수 검사 통과 후 `59568c8`에 병합됐다.
  기준 갱신부터 567.3초, MCP 4,100개 조회와 64개 거부는 47.450초.
  [#283](https://github.com/dev-universe/fender-studio-pro-mcp/pull/283)
- R503 실제 악기의 Event Input 1은 와이어 채널 1과 2의 CC74를
  같은 첫 DSP 행에 전달했다. 능동 111요청·75.814초 및 종료 전
  독립 읽기 31요청·4.239초. #284는 최신 head `08541eb`에서
  검사 중이다. 다음 순서는 #277, #278, #281, #282와 기존 Chorder
  및 Shared Event다.
  [#284](https://github.com/dev-universe/fender-studio-pro-mcp/pull/284)
- R504는 Event Input 2 선택 시 데스크톱이 앱 창 좌표를 받지
  않아 MIDI 전송 전에 중단했다. 61요청·200.373초. 테스트 곡
  저장 없음. 일반 앱·owner 종료, 가상 MIDI 및 서명 VST3 회수,
  관리 설치 18개와 프로필 11개 원복. 후속 R505는 서명된 8.1.1
  기본 스크립트의 악기 연결 인자를 조사한다.
- 고정 잔여 `main` 13개·로컬 11개, 장비 의존 4건, 병합 대기
  67개. 전체 2~5일과 장비 대기, 지정 종료 시각 없음.

## 11월 26일 15시 12분 Serum 이벤트 포트 실앱 조회

- #284 정확한 head `08541eb`에서 두 필수 검사 통과 후
  `98103e3`에 병합, 567.3초. #277 정확한 최신 head
  `be890d0`이 검사 중이다. 다음은 #285, #278, #281, #282부터
  기존 Chorder와 Shared Event 진행 순서다.
  [#284](https://github.com/dev-universe/fender-studio-pro-mcp/pull/284)
- R505의 8.1.1 기본 트랙 스크립트는 `Devices:PortParam`을 통해
  Music 대상 포트를 열거하고, 포트 객체를 `track.channel.connectTo`
  에 넘긴다. 사설 EditTask에서 공식 서명 Serum 악기 한 개의 포트
  16개를 독립 2회 읽어 일치 확인했다. 29요청·29.408초, 라우팅
  변경·이벤트 전송·저장 없음. 정상 앱 및 owner 종료, 서명 VST3 회수,
  관리 설치 18개와 프로필 11개 원복. R506에서 기본 메서드와 두
  대체 입력의 CC74 수신 및 원복을 실제 확인할 예정이다.
  [#285](https://github.com/dev-universe/fender-studio-pro-mcp/pull/285)
- #177 `e1ebea9`, #187 `f694e3c`에 정확한 기능·테스트 충돌 해소를
  올렸다. Rust 서버 테스트 30개 및 전체 284개 통과. #285 추가 이전
  실제 소스를 기준으로 남은 큐 67개 병합 순서가 오프라인에서 모두
  합쳐졌다. 원격 필수 검사는 현행 기준과 각 head에서 별도 확인한다.
- 고정 잔여 `main` 13개·로컬 11개, 장비 의존 4건, 병합 대기
  67개. 전체 2~5일과 장비 대기, 지정 종료 시각 없음.

## 11월 26일 15시 17분 Serum 대체 이벤트 포트 실전달

- R506에서 기본 트랙 `connectTo`가 Event Input 2, 16, 1 각각
  1을 반환했다. 두 시험은 와이어 채널 1 CC74 96→0을 동일하게
  보냈고 실제 두 번째 및 열여섯 번째 DSP 행을 각각 13회 읽어
  목표값과 일치했다. 감시 피어는 0, 각 중립 3회는 정확했다.
  인스펙터의 Input 1 복귀와 전체 2,623행, 믹서, 타임라인,
  정확한 인스턴스를 독립 확인했다.
- 161요청·78.753초, 앱 98.719초, 저장·노트·재생·직접 DSP 쓰기
  없음. 호스트·owner·가상 MIDI와 서명 플러그인 종료·회수,
  관리 설치 18개·프로필 11개 검증 원복. #286을 #285 다음으로
  넣었다. R507은 새 테스트 곡에만 한정한 3회 정상 Save로
  실제 포트 토큰과 UUID를 읽는다.
  [#286](https://github.com/dev-universe/fender-studio-pro-mcp/pull/286)
- #277 최신 head에서 macOS arm64는 통과했고 Core 검사가
  진행 중이다. 고정 잔여 `main` 13개·로컬 11개, 장비 의존
  4건, 병합 대기 68개. 전체 2~5일과 장비 대기, 지정 종료 시각 없음.

## 11월 26일 15시 21분 경계 정리와 저장 조건 수정

- #277의 최신 head `be890d0`가 두 필수 검사 후 `c443db2`에
  병합됐다. 546.4초. #285는 최신 head `b62c86c`에서 검사 중이고
  #286, #278, #281, #282가 뒤따른다.
  [#277](https://github.com/dev-universe/fender-studio-pro-mcp/pull/277)
  [#285](https://github.com/dev-universe/fender-studio-pro-mcp/pull/285)
- R507 첫 저장의 정확한 악기 UUID와 포트 `MusicInput00`은
  확인됐다. 이후 직접 `connectTo`를 호출해도 문서가 변경됨으로
  표시되지 않아 Save 메뉴가 비활성화됐고 기존 해시는 같았다.
  두 번째 저장을 검증하지 못했고 16번은 시도하지 않았다.
  52요청·66.795초, 앱 110.537초, Save 1회, MIDI 없음.
  정상 종료 및 관리 설치 18개와 프로필 10개 원복.
- 새 테스트 곡 두 개에서 최초 저장 전에 각각 Event Input 2와 16을
  선택한다. 각 곡의 실제 저장 토큰, 라이브와 일치하는 악기 UUID,
  보호된 원본 해시를 확인한다. 잔여 `main` 13개·로컬 11개,
  장비 의존 4건, 병합 대기 67개. 전체 2~5일과 장비 대기,
  지정 종료 시각 없음.

## 11월 26일 15시 29분 Serum 저장 결과와 FX 준비

- #285 최신 head `b62c86c`의 두 필수 검사 뒤 `e7a3975`에
  병합했다. 566.7초. #286 최신 head `c6ce26c`가 검사 중이고
  #287이 뒤따른다.
  [#285](https://github.com/dev-universe/fender-studio-pro-mcp/pull/285)
- R508 첫 Save에서 Event Input 2는 `MusicInput01`, R509
  첫 Save에서 Input 16은 `MusicInput15`였다. 저장 목적지 및
  악기 Device UUID가 실제 로드한 인스턴스와 각각 일치했다.
  각 73요청·48.488/66.129초, 앱 76.745/78.598초. DSP 2,623행과
  믹서 및 타임라인 동일. 종료 후 18+10 원복, 서명 VST3 회수.
- #287의 `song_read`와 `song-read`를 실제 저장본 4개에서 대조했다.
  악기 1·2·16과 이전 FX 1, 원본 해시와 결과 및 UUID 일치
  1.715초. Rust 273 통과. R510은 별도 Audio 트랙에 Serum 2 FX
  한 개와 Music 트랙 하나만 두고 정확한 FX 포트 전달을 시험한다.
  [#287](https://github.com/dev-universe/fender-studio-pro-mcp/pull/287)
- 고정 잔여 `main` 13개·로컬 11개, 장비 의존 4건, 병합 대기
  67개. 현재 USB에서 Volt 276과 Keystation 88은 확인되지 않았다.
  전체 2~5일과 장비 대기, 지정 종료 시각 없음.

## 11월 26일 15시 37분 Serum FX 대체 이벤트 실전달

- R510 일반 Add Track 응답이 빈 목록이어서 중단했다.
  19요청·23.071초. R511 사설 진단은 기본 플래그의 Boolean
  읽기를 필요로 해 거부했다. 19요청·18.944초. 둘 다 라우팅
  변경, MIDI 및 Save 없음.
- R512의 정확한 FX 인서트로 채널 1 CC74를 보내 Input 2와 16의
  각 DSP 행에서 일치 13회와 중립 3회를 읽었다. `plugin_inspect`
  두 경로의 실제 UUID, 기본 인스펙터 Input 1과 DSP 2,623행의
  원복을 확인했다. 171요청·69.145초, 앱 83.527초, 저장 없음.
  세 시험 모두 앱·owner·가상 MIDI 종료, 서명 VST3 회수 및
  관리 설치 18개·프로필 11개 원복. 다음은 같은 모델 FX를 추가한
  별도 저장본의 포트 및 정확한 대상 UUID 확인이다.
  [#288](https://github.com/dev-universe/fender-studio-pro-mcp/pull/288)
- #286 검사 뒤 #287, #288, #278, #281, #282 순이다.
  #288 추가 전 최신 기준 67개가 오프라인 합치기에서 전부
  통과했다. 지정 루프 종료 시각 없음.

## 11월 26일 15시 43분 Serum FX 저장과 동일 모델 식별

- #286 정확한 head `c6ce26c`에서 두 필수 검사 통과 후
  `125a59c`에 병합했다. 796.8초. #287 최신 head `385631b`에서
  검사 중이다.
  [#286](https://github.com/dev-universe/fender-studio-pro-mcp/pull/286)
- R513은 두 번째 인서트의 표시 이름이 Serum 2 FX 2임을
  확인한 뒤 저장 없이 닫았다. R514는 Event Input 16의
  `MusicInput15`, R515는 Event Input 2의 `MusicInput01`을
  기본 Save로 확인했다. 저장 UUID가 서로 다른 같은 모델
  중 의도한 인서트였고 라이브와 저장 UUID가 일치했다.
  각 143요청·52.680/78.688초, 앱 64.614/89.727초. 두 인스턴스의
  5,246행과 믹서·타임라인이 일치했다. 18+10 원복, MIDI 없음.
- #289는 #287 위에 쌓았으며 #288 다음 순서다. 실제 저장본
  6개 CLI/MCP 결과와 해시가 일치했다. 1.739초. Rust 273 통과.
  R516은 저장 보호된 새 복사본을 실제 앱에서 다시 열어 확인한다.
  [#289](https://github.com/dev-universe/fender-studio-pro-mcp/pull/289)
- 고정 잔여 `main` 13개·로컬 11개, 장비 의존 4건, 병합 대기
  68개. 전체 2~5일과 장비 대기, 지정 종료 시각 없음.

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

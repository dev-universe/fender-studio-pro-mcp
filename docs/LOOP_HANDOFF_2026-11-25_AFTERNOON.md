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

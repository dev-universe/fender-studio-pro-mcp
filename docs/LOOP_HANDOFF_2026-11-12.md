# 2026-11-12 재개 루프 인수인계

사용자가 이동을 위해 현재 단위까지만 마치고 루프를 종료하도록 요청했다.
이번 재개는 2026-11-10 중단 기록의 다음 작업부터 시작했다.
이 문서는 이번 재개의 작업 이력과 다음 실행 순서를 보존한다.
Native34의 소스 검토·빌드·설치는 끝났지만, 새 실제 앱 검증 unit64는 시작하지 않았다.
사용자의 새 재개 지시 전에는 기능 작업이나 실제 검증을 자동으로 시작하지 않는다.

## 1. 한 문장으로 재개하는 방법

사용자가 "진행 재개해"라고 하면 이 문서의 11절부터 이어간다.
완료한 수천 개 파라미터 검사나 unit48부터의 작업을 처음부터 반복하지 않는다.
실행 전에는 현재 파일·프로세스·설치 상태를 새로 확인한다.
이 문서의 해시와 과거 runtime token은 현재 상태 확인을 대신하지 않는다.

재개 담당자가 읽을 문서는 다음 순서다.

1. 저장소 `AGENTS.md`와 사용자의 최신 지시.
2. [RESUME.md](RESUME.md)와 [RESUME_STATE.json](RESUME_STATE.json).
3. 이 문서의 중단 상태·다음 단위·전체 잔여 목록.
4. [2026-11-12 진행 기록](PROGRESS_2026-11-12.md)의 개별 단위 결과.
5. [2026-11-10 인수인계](LOOP_HANDOFF_2026-11-10.md)의 이전 구현·검증 이력.
6. [플러그인 상태 검증](PLUGIN_STATE_VALIDATION.md)과 각 비공개 증거의 소스·해시 바인딩.
7. [최초 구현 계획](IMPLEMENTATION_PLAN.md)의 P0–P5 장기 범위.

2026-11-10 문서의 첫 pitch pilot과 Mode 측정은 이번에 이미 수행했다.
그 문서의 오래된 재개 순서를 다시 실행하지 않는다.
이번 중단 뒤 첫 실제 단위는 Native34의 저장 코드 읽기 검증 unit64다.
그 다음은 unit65의 일반 효과 9종이다.

## 2. 사용자 목표와 작업 방식

- macOS를 우선 지원하는 native Rust MCP 서버를 만든다.
- Fender Studio Pro의 내장 플러그인과 가능한 많은 작업을 LLM이 다루게 한다.
- 실제 Fender Studio Pro 8.1.1 Build 113103에서 bridge와 EditTask 및 플러그인 조작을 확인한다.
- 대표 값 몇 개뿐 아니라 쓰기 가능한 전체 파라미터의 변경·읽기·복원을 확대한다.
- Splitter, Surround Panner, External Effect, 모델, 프리셋과 중첩 그래프를 포함한다.
- 공개 저장소 [dev-universe/fender-studio-pro-mcp](https://github.com/dev-universe/fender-studio-pro-mcp)에서 버전 관리한다.
- Hopper는 사용하지 않는다. 추가 바이너리 분석이 필요하면 Ghidra와 기존 정적 증거를 사용한다.
- 실제 앱·owner·설치·REA 조작은 root 한 명이 직렬로 수행한다.
- 소스 구현과 독립 오프라인 검토는 병행할 수 있다.
- 사용자가 다른 일을 하므로 반복적인 외부 터미널 입력 승인 대기를 줄인다.
- 단위 완료마다 체크리스트, 실제 소요 시간, 완료·잔여 수와 전체 예상 잔여 시간을 보고한다.
- 질문이나 상태 요청만으로 작업을 멈추지 않는다. 이번에는 명시적인 중단 요청이므로 멈춘다.

branch 이름과 PR 제목에 `codex`를 사용하지 않는다.
상위 프로젝트의 기술 선택과 검증 증거를 보존한다.
Rust 서버를 다른 언어의 서버로 다시 만드는 작업은 필요 없다.

## 3. 중단 시점의 제품과 설치 상태

아래는 Native34 설치를 마친 상태다. 실제 unit64 결과를 뜻하지 않는다.

| 항목 | 중단 시점 |
|---|---|
| 제품 소스 | `3815714`, `feat: read stored Chorder chord maps` |
| 작업 branch | `feature/plugin-state-and-exhaustive-validation` |
| 공개 PR | [#4](https://github.com/dev-universe/fender-studio-pro-mcp/pull/4), Draft 유지 |
| 빌드한 Rust MCP 도구 | 43개 |
| 설치 native 단위 | Native34 R2 |
| 실제 8.1.1 검증을 마친 마지막 native 단위 | Native33 |
| 마지막 실제 단위 | unit63, Analog Delay 완료 후 Room Reverb 복원 검사에서 중단 |
| 다음 실제 단위 | unit64, Chorder 저장 코드 전체 읽기 |
| unit64·65 | 오프라인 준비·독립 검토 완료, 실행하지 않음, artifact binding은 pending 유지 |
| 앱과 owner | 최종 관찰에서 앱 종료·serve/owner 프로세스 없음 |
| 소유 곡 | 최종 관찰에서 세 저장 파일의 원본 해시 일치 |
| 자동 재개·예약 실행 | 없음 |
| 활성 목표 객체 | 없음. 중단 상태는 저장소 문서와 상태 JSON에 기록 |

Native34 정상 패키지와 main release의 식별값은 다음과 같다.

| 식별 항목 | 값 |
|---|---|
| native UUID | `64DF1790-3306-3068-ADC0-AF3F0EAE8F92` |
| native SHA256 | `ff3e093c66fae05105c4343eddd67166c549370603decfe4b0d4aa7ebcd6cb2f` |
| main release SHA256 | `6dcd06ff927d6f0b8801095555e78b76a6326e260971e12f97bf5505e4969d85` |
| Native34 R2 source freeze SHA256 | `29385fa83423de69f10f17497f463327c090e5481ac3fa935be8d6a0afcb7e8e` |
| 정상 native manifest SHA256 | `38c141e224179b9d1ee4f48c035ddfc2bc4574b8b3355aee095ed8512d7426f2` |

설치에는 native 네 파일과 변경된 bridge 세 파일이 포함된다.
bridge 세 파일은 `BridgeComponent.js`, `BridgeCore.js`, `NativePluginControls.js`다.
기존 관리 대상 파일을 정상 installer로 갱신했고 이전 파일은 installer의 private backup에 보존했다.
관련 없는 DAW 설정이나 사용자 곡을 바꾸는 설치 경로는 추가하지 않았다.

isolated test build의 UUID와 정상 패키지 UUID는 다르다.
unit64 바인딩에 isolated UUID를 복사하면 안 된다.
재개 때 정상 manifest의 네 파일, 실제 설치 파일, main과 bridge 세 파일을 다시 확인한다.
문서만 수정한 후에는 같은 제품 소스를 불필요하게 다시 빌드하지 않는다.

## 4. 이번 재개의 결과를 읽는 기준

소스 구현, 오프라인 테스트, 실제 요청 수락, 실제 값 관찰, 저장·재시작과 오디오를 구분한다.
요청이 `applied_unverified`로 끝났다는 사실만으로 변경이나 복원을 완료 처리하지 않는다.
원본 전체 provider와 최종 독립 체크포인트를 통과한 단위만 수락한다.
한 단위의 일부 setter가 성공했어도 최종 원본 검사가 없으면 그 단위는 미완료다.

capsule load 뒤 값이 한 ULP라도 다르면 exact restoration 성공으로 처리하지 않았다.
표시 문자열이 같다는 이유로 typed raw·normalized bit 차이를 무시하지 않았다.
read-only meter 값의 관찰 정책과 writable 원본 복원 조건을 분리했다.
사전 계획한 scalar 복원을 사용한 hybrid 사례와 capsule 단독 사례도 분리한다.

전체 지원률은 계산하지 않았다.
현재 제공 record 수, 서로 다른 이름, 선택한 모델, 프리셋 사례와 역할 수는 서로 다른 분모다.
마지막 고정 전역 이름 합집합 감사는 Focus7 경계의 6,288/7,112다.
그 뒤의 Focus·MIDI·Vocal Tune·Chorder 결과를 합친 최신 전역 합집합은 아직 재계산하지 않았다.

## 5. unit48부터 unit63까지의 작업 이력

| 단위 | 수행한 작업 | 완료 또는 중단 결과 |
|---|---|---|
| 재개 preflight | Git·Native31·bridge·EditTask·곡·완료 소스 바인딩 대조 | 기존 증거 재사용, 대규모 sweep 재실행 없음 |
| unit48a | Mode-off 입력 `null → MIDI0 → null` | current-value 왕복 1개 완료 |
| unit48b | 정상 GUI Mode-on 탐색과 전체 908개 비교 | 131뷰와 연동 7개 이름 측정 후 계획 차이로 중단 |
| unit49 | learning setter 전용 탐색 한도 보강 | 고정 144뷰, 소스·경계 검사 후 Native32 설치 |
| unit49a | 기존 대화형 Mode 단위 | 마지막 null 복원 전 실행 방식 전환으로 중단, 완료로 집계 안 함 |
| unit50 | 고정 일괄 Mode on/off/null 왕복 | 3 setter·5 capture·5 release, 전체 원본 복원 완료 |
| unit51 | Channel Strip·Dual Pan·Limiter와 Expander | 앞 3종 완료, Expander capsule 복원 차이로 중단 |
| unit52 | 별도 transient 재시작용 Save As 복사본 | documentPath 차이로 준비 비교에서 중단 |
| unit53 | 정확히 제한한 복사본의 Mode-only Save 실험 | Save·Revert 비활성, Save0, 계획된 대기 후 중단 |
| unit55 | 저장 없이 완전 재시작한 읽기 검사 | Mode0·input-null·전체 908개 원본 복귀 확인 |
| unit54 | 일반 효과 24종 계획의 첫 Gate | attack 복원 차이로 중단, 뒤 23종 미실행 |
| unit56 | Expander index1의 계획된 hybrid 복원 | capsule load와 원본 attack setter 각 1회, 완료 |
| unit57 | Tricomp와 Multiband Dynamics | Tricomp 완료, releaseband 5개 차이로 Multiband 중단 |
| unit58 | 변조·왜곡·delay 9종과 Surround Delay | 9종 완료, Surround Delay 복원 차이로 중단 |
| unit59 | Expander index2 | hybrid 완료, Expander 클래스 계획 종료 |
| unit60 | Gate index1 | hybrid 1사례 완료, 당시 클래스는 부분 완료 |
| unit61 | Native33의 고정 Chorder child 메타데이터 읽기 | `pitch61` 관찰, 전체 908개 원본 유지 |
| unit62 | Gate index2 | hybrid 완료, Gate 클래스 계획 종료 |
| unit63 | Analog Delay와 Room Reverb | Analog Delay 완료, Room Reverb 복원 차이로 중단 |
| Native34 | 저장 Chorder 모델 getter 소스·검토·패키징·설치 | 완료, 실제 unit64는 사용자 중단으로 미실행 |

번호55의 읽기 검사를54의 일반 효과 묶음보다 먼저 실행했다.
번호 순서만으로 실제 실행 순서나 의존 관계를 추정하지 않는다.

### 5.1 Mode-off pitch 왕복과 Mode-on 측정

unit48a는 GUI 미선택을 뜻하는 `null`과 MIDI0을 구분해 두 setter를 실행했다.
변경 전·후·복원 뒤 전체 908개를 각각 두 번 읽었다.
GUI MIDI0은 이 사례에서 DSP selector1과 `C -2` 표시로 관찰됐다.
다른 record·typed schema·normalized·display는 원본과 같았다.
transport·state load·Song Save는 모두0회였다.

unit48b는 정상 GUI로 Mode를 켠 뒤 getter256으로 탐색했다.
Mode-off는125뷰, Mode-on은131뷰였으며 완전 탐색 두 번이 같았다.
기존 learning setter128로 역방향 탐색을 할 수 있다고 가정하지 않았다.
MCP Mode setter는 이 단위에서 호출하지 않았다.

Mode-on에서는 `learnMode`, `learnInputPitch`와 다섯 learnChord 비트가 바뀌었다.
저장 fixture의 입력 MIDI60과 출력60·63·67·70·74에 대응하는 연동 집합이었다.
기존 계획은 Mode와 selector만 허용했으므로 추가 쓰기·load·retry 없이 중단했다.
소유한 unsaved diagnostic을 폐기했고 저장 원본은 그대로였다.

### 5.2 Native32 고정 한도와 Mode 왕복

unit49는 learning setter의 discovery ceiling만128에서144로 바꿨다.
getter256, 기존 comparison·Auto Fill128과 depth32는 유지했다.
caller가 budget을 지정하거나 보지 않은 branch를 건너뛰는 기능은 만들지 않았다.
131·144뷰 실제 함수 검사는 통과했고145뷰는 begin 전에 거부됐다.
기존 Auto Fill은128뷰에서 통과하고131뷰에서 begin 전에 거부됐다.

대화형 unit49a는 Mode-on과 Mode-off-retained 관찰까지 진행했다.
외부 터미널 입력을 줄이는 새 실행기로 전환하면서 마지막 null 복원 전 폐기했다.
이 부분 실행은 완료 수에 넣지 않았다.

새 unit50은13개 고정 단계 전체를 완료했다.
setter3회, state capture5회와 release5회였으며 load는0회였다.
9단계의 전체908 pair, 총18회 전체 읽기와 각 GUI getter를 비교했다.
Mode-off는 다섯 learning bit를 지우고 입력 선택을 유지했다.
마지막 explicit-null 호출이 전체 원본과 GUI descriptor를 복원했다.
5개 capture는 모두581바이트와 같은 SHA256이었다.
440 RPC에 오류가 없었고 transport·Song Save는0회였다.
UI 준비와 정리를 포함해296.855초가 걸렸다.

### 5.3 저장 없는 transient reset과 실패한 Save 실험

unit52는 원본을 보존하는 정상 Save As로 별도 소유 복사본을 만들었다.
저장 ZIP 비교에서 제목 외에 MediaPool의 documentPath도 바뀐 사실을 발견했다.
제목만 허용한 계획이므로 Mode setter나 capture를 보내지 않고 중단했다.
나머지 member와 Chorder581바이트는 원본과 같았다.
이 준비 단위는138.271초였다.

unit53은 두 정확한 URL 사이의 documentPath 변환만 허용하도록 준비 비교를 고쳤다.
현재 복사본 해시를 고정한 새 단위에서 Mode-on·전체 읽기·capture·release를 수행했다.
정상 File 메뉴의 Save와 Revert가 비활성이었다.
Save 완료 신호를 만들거나 비활성 메뉴를 우회하지 않았다.
30초 고정 대기 후 중단했고 정상 Close는 Save prompt 없이 시작 페이지로 돌아왔다.
setter1회·capture1회·release1회, load·transport·Save0회였다.
UI 정리까지139.674초였다.

unit55는 이전 Mode-on 종료 증거를 재사용하고 새 세션에서 읽기만 수행했다.
Mode0·pitch-1과 전체908개 원본을 네 번의 전체 읽기 및 GUI getter4회로 확인했다.
78 RPC, setter·state action·transport·Save0회였고100.715초가 걸렸다.
이는 저장 없이 재시작한 reset 특성화다. Save persistence 성공이 아니다.
앞선 Auto Fill의 양방향 Save·완전 재시작 성공은 별도 이전 증거로 보존한다.

### 5.4 원본 복원 검사 보강

unit51의 Channel Strip·Dual Pan·Limiter는 각각 두 사례를 완료했다.
그 뒤 Expander attack 차이로 같은 단위를 중단했다.
unit54도 Gate의 capsule 복원 뒤 attack 차이로 중단했다.
실패 단위 안에 추가 보정 setter나 retry를 덧붙이지 않았다.

unit57부터 원본 복원 관찰마다 전체 typed schema와 비값 metadata를 검사한다.
native default·access flag와 알 수 없는 필드도 원본과 같아야 한다.
마지막 연속 두 full-provider 읽기가 exact 원본이어야 해제나 다음 preset을 허용한다.
변경된 preset 자체가 schema를 바꾸는 경우는 별도로 취급한다.
read-only 현재값은 복원 주장과 분리해 관찰한다.
기존 완료3종의 원본 관찰30개를 원장에서 재투영한 결과 모두 통과했다.
이 감사 때문에 실제 앱에서 완료한 프리셋을 다시 적용하지 않았다.

### 5.5 Expander와 Gate의 hybrid 복원

Expander의 capsule load는 attack normalized 값을 한 ULP 낮게 복원했다.
표시값은 같은10.00ms였지만 exact 복원은 아니었다.
새 unit56·59는 측정한 전체 중간 상태와 원본 attack setter 한 번을 사전에 고정했다.
중간 full-provider4회, 보정 후 exact 원본 pair, 해제와 만료 catalog 거부를 통과했다.
각각 index1·2 한 사례이며 합쳐 Expander의 계획된 두 사례를 완료했다.
검증 실행은7.505초·8.086초, UI 포함 전체는146.104초·303.367초였다.

Gate도 원본 load 뒤 attack의 normalized 값이 한 ULP 낮았다.
unit60·62는 계획된 attack setter 한 번과 read-only reduction 관찰 한 번을 분리했다.
그 첫 관찰에서 다른18개 값과 전체 metadata는 엄격히 비교했다.
이후 전체19개 중간 상태4회와 보정 뒤 exact 원본 pair를 요구했다.
두 index는 각각8.466초·8.406초에 검증했고 전체86.877초·104.222초였다.
두 클래스 모두 capsule 단독 성공으로 기록하지 않는다.

### 5.6 추가 일반 효과와 세 복원 실패

unit57에서 Tricomp 두 사례가 완료됐다. 실행8.884초였다.
다음 Multiband Dynamics는 releaseband1..5의 차이로 중단했다.
unit58은 다음9종의 두 사례씩을 완료했다.

| 클래스 | 전체 record | catalog 관찰 수 | 실행 초 |
|---|---:|---:|---:|
| RedlightDist | 10 | 8 | 8.881 |
| Bitcrusher | 12 | 19 | 8.816 |
| Phaser | 16 | 25 | 8.782 |
| Flanger | 12 | 25 | 8.796 |
| Chorus | 14 | 44 | 8.860 |
| X-Trem | 27 | 20 | 8.921 |
| Autofilter | 44 | 48 | 8.952 |
| Beat Delay | 14 | 29 | 8.566 |
| Groove Delay | 55 | 15 | 8.740 |

Surround Delay는 다음 capsule 복원 검사에서 중단했다.
전체79개 중 time3·time6의 차이가8회 전체 읽기에서 유지됐다.
normalized bits는 각각2 ULP·1 ULP 차이였고 표시값은 같았다.
원본 capsule은2,104바이트였다. 후속 두 scalar 보정 계획은 아직 실행하지 않았다.

unit63의 Analog Delay는24 record·catalog27에서 두 사례를 완료했다.
검증 실행9.513초였다. 그 뒤 Room Reverb가 exact 원본 복원 검사에서 중단됐다.
전체19개 중 writable roomwidth와 read-only outdepth·outheight 차이가8회 관측에 유지됐다.
각 차이는 normalized1 ULP였고 display와 비값 metadata는 같았다.
Room Reverb capsule은725바이트였다. Room용 쓰기 계획은 아직 만들지 않았다.

Multiband Dynamics·Surround Delay·Room Reverb는 각각 수락0개로 유지한다.
후속 실험은 측정한 중간 상태, 허용할 scalar와 전체 원본 조건을 먼저 고정해야 한다.
같은 실패 단위에 tolerance나 새 보정 동작을 사후 추가하지 않는다.

## 6. Native33 고정 Chorder child 읽기

이전 canonical Auto Fill observer/controller에서 고정 `pitch61` child를 읽는 진단을 추가했다.
public CCL node/controller ABI를 사용했고 caller가 child path나 method를 지정하지 못하게 했다.
`plugin_parameter_details`의 기존 `compare_gui:true`, `include_owner:true` 경로에서만 실행한다.
새 MCP 도구나 새 쓰기 capability는 추가하지 않았다.

root·child·parent·controller의 identity와 연결을 일반 검사와 최종 publication에서 확인한다.
독립 검토는 native Release callback이 마지막 controller 연결을 바꾸는 R1 반례를 재현했다.
R2는 terminal QI owner를 두 borrowed 검사와 Reply·Receipt publication까지 유지한다.
그 수는14로 제한하며 다음 정상 검사 전에 이전 witness를 해제한다.
최종 child IController 검사를 제거한 사본에서는 같은 거부 검사가 실패했다.

unit61에서 실제8.1.1의 `pitch61`과 로컬 두 descriptor를 관찰했다.
root child 수128, parent/root/controller 연결이 같았다.
learnChord는 Toggle0, chordAssigned는 읽기 전용 Integer1이었다.
descriptor의 writable 여부는 메타데이터이며 새 mutation 권한이 아니다.
MIDI 출력 음정과 child 이름의 일반 매핑을 확정했다고 표현하지 않는다.

전후 전체908개 읽기4회와 GUI getter4회가 원본과 같았다.
details1회·editor open1회, setter·state action·transport·Save0회였다.
저장 파일이 그대로였고 앱을 닫은 뒤 owner를 종료했다.
unit61의 전체 시간은105.756초였다.

## 7. Native34 저장 코드 getter 구현과 검토

새 도구는 `chorder_chords_get`이며 내부 wire operation은 `chorderChordsGet`다.
native operation은 `chorder_chords_get`다.
이 도구를 추가해 Rust MCP 도구 수가42에서43으로 늘었다.
입력은 track, Note FX slot0..15, expected_instance, offset0..127, limit1..16이다.
읽기 전용이지만 공통 expected_document가 필수다.

정확한 Chorder class의 Device component alias IStorable를 읽는다.
기존 AliasLease를 Reply와 Receipt publication까지 유지한다.
일반 capsule registry에 등록하지 않고 고정1MiB 메모리 stream으로 serialize한다.
기존 capsule capture의16MiB 한도와8개 registry 동작은 바꾸지 않았다.
상태를 load하거나 XML·파일·state token을 caller에게 전달하는 경로는 없다.

디코더는128개 입력의 출력 음정 집합을 만든다.
페이지는 빈 입력도 명시하며 출력은 정렬된 고유 MIDI0..127 정수다.
capture byte 수·capture SHA256과 전체 model SHA256을 별도로 반환한다.
각 페이지는 새로운 순차 관찰이다. 여러 페이지가 하나의 atomic snapshot이라고 주장하지 않는다.
지원하지 않는 serializer 구조는 partial model 없이 거부한다.

허용한 XML은 정확한 Chorder class와 알려진 component/configuration/model 구조다.
UTF-8, 선택적인 단일 BOM과 첫 XML declaration을 지원한다.
깊이5, 입력128, 입력별 출력128과 전체16,384 출력이 상한이다.
DTD·entity·알 수 없는 attribute·중복 key/output·다른 root·범위 밖 음정은 거부한다.
비공백 text, comment, CDATA, 임의 PI, 잘못된 closing tag와 encoding도 거부한다.
새 add-on dependency는 기존 root에서 쓰던 cached quick-xml0.41.0이다.

성공 DTO에는 mutation_capability·persistence_verified·audio_verified가 모두false다.
native·JS·Rust의 늦은 오류나 문서 변경은 모든 모델·digest·page 완료 사실을 무효화한다.
input 값이 숫자로 보인다는 사실만으로 직접 facade writer를 우회해 호출하지 않는다.

### 7.1 검증 결과와 R2 수정

| 검사 | 결과 |
|---|---|
| native add-on | 265개 통과, 기존253개와 새12개 |
| JS 전체 | 290개 통과, 새 getter11개 포함 |
| 해당 Rust 검사 | 고유24개 통과, 두 그룹의 중복2개 제외 |
| 형식·lint | fmt와 지원 Rust1.88 strict Clippy 통과 |
| native 독립 검토 | actual-function6개 묶음 통과 |
| wire 독립 검토 | actual native fixture 교차 수용 포함8개 묶음 통과 |
| 정상 패키지 | Root가 R2 source와 네 native 파일을 바인딩해 재빌드·설치 |
| 실제8.1.1 getter | 아직 실행하지 않음 |

native 독립 검토는 첫 BOM을 수동 제거한 뒤 parser가 두 번째 BOM도 제거하는 반례를 찾았다.
R2는 추가 leading BOM을 명시적으로 거부한다.
기존 malformed-input 검사에 반례를 추가했고265개 검사·fmt·Clippy를 다시 통과했다.
수정만 제거한 사본은 같은 독립 검사에서 exit101로 실패했다.
R1 source·실패 로그는 별도 폴더에 보존했다.

native publication 검사는 출력 owner의 Release callback이 alias·parent를 분리하거나
document/model을 바꾸거나 service를 정지할 때 모델 증거를 지우는지 확인했다.
wire 검토는 실제 fake-native DTO를 Note FX resolver, JS와 Rust 검증기에 통과시켰다.
이 fake-host 증거는 실제 DAW 관찰과 분리한다.

초기 test fixture export의 상대 경로 오류와 review mock 설정 오류는 로그에 남겼다.
그 실행을 성공 수에 넣지 않았으며 수정한 검사 실행 결과를 별도로 동결했다.
검토는 선택 모델을 상속한 독립 에이전트로 수행했다. 여러 모델의 합의라고 표현하지 않는다.

## 8. 반복 승인 요청을 줄인 실행기

이전 방식은 sandbox 밖의 기존 대화형 터미널에 입력을 반복 전송했다.
그때 나타난 "Send input to an existing terminal" 승인 때문에 사용자가 다른 일에 집중하기 어려웠다.
검토한 유한 Python job이 자체 PIPE로 MCP owner를 제어하는 방식으로 바꿨다.
진행 상황은 workspace의 progress·result 파일로 읽고 외부 terminal input을 사용하지 않았다.

고정 launcher는 `.local/next-state/run_next_units.py`다.
인자를 받지 않으며 검토한 자식 경로·SHA256·순서만 실행한다.
새 queue는 이전 source freeze와 결과를 보존한 뒤 구성한다.
이 launcher를 임의 shell이나 caller script를 실행하는 우회 통로로 바꾸지 않는다.

UI-ready와 cleanup flag는 job ID와 고정 Boolean·title만 받는다.
현재 앱 상태를 직접 관찰하지 않고 flag를 만들어 넣으면 안 된다.
owner는 앱 종료 확인 뒤 마지막에 종료한다.

실행기 검토에서 client timeout과 non-JSON stdout 오류가 terminal 응답으로 오인될 수 있음을 찾았다.
현재 fixed owner는 정확한 요청 번호의 MCP 응답을 받을 때만 완료를 알린다.
timeout, send error, EOF marker와 diagnostic만으로 quiescence를 선언하지 않는다.
완료가 불명확하면 owner를 유지하고 cleanup을 허용하지 않는다.

승인된 command prefix는 같은 범주의 반복 실행·설치·Git 작업에서 재사용할 수 있다.
관리형 승인 정책을 root가 대신 허용하거나 비활성화할 수 있다는 뜻은 아니다.
승인이 필요한 실제 경계에서는 tool의 정식 escalation을 사용한다.
사용자의 기존 허용과 reversible 작업에 불필요한 재확인을 덧붙이지 않는다.

중단 마무리 때 launcher를 owner 시작 전에 거부하는 pause guard로 바꿨다.
guard를 제거해 옛 unit62·63 queue를 그대로 실행하면 안 된다.
재개 때 unit64·65의 최종 binding·review·SHA를 확인한 뒤 새 queue를 명시적으로 구성한다.
이전 `program62-63-source-freeze.py`와 `.local/live/program62-63`은 보존한다.
옛 parent source SHA256은 `416543de50a0b602813d02f0994aae61583032c982b0c05728088eaa876f6213`다.

## 9. 프리셋46종 계획의 정확한 현재 집계

완료18종과 잔여28종이다. 총 긍정 사례는36개다.
capsule 단독16종·32사례와 hybrid2종·4사례를 합친 값이다.
현재 부분 완료 클래스는 없다.

| 완료 방식 | 클래스 |
|---|---|
| capsule 단독 | Mixtool, Binaural Pan, Channel Strip, Dual Pan, Limiter, Tricomp, RedlightDist, Bitcrusher, Phaser, Flanger, Chorus, X-Trem, Autofilter, Beat Delay, Groove Delay, Analog Delay |
| capsule와 사전 계획한 attack setter | Expander, Gate |

각 완료 클래스는 선택된 두 issued index를 검증했다.
catalog의 모든 preset을 실행했다는 뜻이 아니다.
기존 positive6종과 empty catalog7종을 이 추가46종 계획에 다시 넣지 않는다.
singleton이나 default-only preset의 무변화 결과를 성공으로 만들려고 원본을 임의로 바꾸지 않는다.
두 후보가 무변화면 이유를 기록하고 끝없는 preset 순회를 하지 않는다.

잔여28종은 다음과 같이 분할한다.

| 묶음 | 수 | 클래스 |
|---|---:|---|
| 정확 복원 실패 후 별도 계획 필요 | 3 | Multiband Dynamics, Surround Delay, Room Reverb |
| unit65 일반 효과 | 9 | Mixverb, Studio Verb, Compressor, Voice FX, Rotor, De-Esser, Vocoder, Vocal Tune, Open AIR |
| meter·generator·tuner | 6 | Spectrum Meter, Level Meter, Scope, Tone Generator, Tuner, VU Meter |
| amp·pedal | 3 | Ampire, Pedalboard, Rumble Native |
| 악기·Note FX·CV | 7 | Multi Instrument, SampleOne, Impact, CV Instrument, Mojito, Chorder, Repeater |

클래스 식별값과 과거 catalog/provider 수는 부록A에 있다.
그 수는 과거 관찰을 식별하는 참고값이며 새 issued capability가 아니다.
CV Instrument는 Main0만으로 별도 CV 출력 격리를 증명할 수 없으므로 일반 효과 묶음에 넣지 않는다.

## 10. 소스 검토만 끝난 다음 실제 단위

### 10.1 unit64 저장 코드 읽기

private 폴더는 `.local/next-state/unit64-chorder-chords-readonly/`다.
실행 파일은 `run_getter64.py`, owner는 `owner64.py`다.
준비 중94개 고정 RPC와16회 serialize read, 전후 전체908개4회를 계획했다.
editor open1회는 기존 GUI 원본 getter를 재사용하기 위한 것이다.
setter·state capsule action·load·transport·normal Song Save는 모두 금지한다.

입력0·16·32·48·64·80·96·112를 limit16으로 읽는 전체 pass를 두 번 수행한다.
기대 원본은 입력 MIDI60의 출력60·63·67·70·74와 나머지127개 빈 입력이다.
16회 capture 모두581바이트와 같은 원본 capture hash를 요구한다.
canonical model hash는 별도 고정 encoding으로 계산했다.

| 기대 항목 | 값 |
|---|---|
| capture SHA256 | `6874d20b68956d27d749e48d309a47467bad3302f0fd7085754565d872777b3d` |
| canonical model SHA256 | `1dc362665f88c5e083ca1334db73d038ba1c2d9941cdf0fbab69e102847d06d4` |
| Root가 provenance 경로를 R2로 고친 runner SHA256 | `c2b184e569072665448711762c65d22c4138f2b2f685ea60683ec5d931c0ff97` |
| owner SHA256 | `952a62ad02dcba2324299fbe6f5345e9ced9b3b686fd63abdcaf3c2caf4601f5` |

builder의 옛 runner SHA는 `e38da5db5bc08bc07cf08c8494380781593e7a68157414cba99bbac46b5de044`다.
그 뒤 Root는 provenance 파일 경로만 `source-freeze-r2.json`으로 바꾸고 freeze를 갱신했다.
최종 independent review의 source hash를 기준으로 재개한다.
private readiness에 남은 builder 시점의 옛 hash를 최종값으로 복사하지 않는다.

unit64는 original `MCP Chorder Processing Diagnostic`만 연다.
전역적으로 빈 한 Music 트랙, arm0·monitor0, stopped·loop/precount/preroll off를 요구한다.
Main은 이미 numeric0이고 output 키가 명시적으로 null이어야 한다.
Main을 준비하는 setter는 unit64에 없다.

binding의 execution_ready와 installed는 pending으로 남겼다.
재개 후 실제 설치를 관찰한 뒤 UUID, main/native SHA, bridge 세 파일 SHA를 채운다.
R2 source freeze, 정상 package manifest와 wire evidence hash도 함께 바인딩한다.
freeze에 binding SHA와 같은 normal UUID를 넣고 전체 source/data pin을 확인한다.
그 뒤에만 새 owner를 시작한다.

### 10.2 unit65 일반 효과9종

private 폴더는 `.local/next-state/unit65-preset-remainder/`다.
`run_presets65_batch.py`와 `owner65.py`는 unit63의 완료·실패 클래스를 제외한 새 namespace다.
첫8종은 두 선택지, Open AIR는 singleton index0을 마지막에 실행한다.
최대17개 사례이며 scalar repair나 실패 후 retry는 추가하지 않았다.

| 준비 source | SHA256 |
|---|---|
| runner | `3b9b02d9476e88f48dd9fa6005514259bcacdb9ac6b235b65d98f961fb6df8c1` |
| owner | `6d3a65b80ba091d42ed982b79cfb9686025cc22fb60545d02255bf705a66b20e` |

Native34 binding은 null UUID·hash와 execution_ready:false 상태다.
Root가 실제 설치를 확인한 뒤 README의 필드를 채우고 binding SHA literal만 worker에 갱신한다.
worker·freeze·readiness와 parent queue의 최종 SHA도 함께 갱신한다.
준비된 plan의 index나 완료된 immutable helper는 바꾸지 않는다.

worker는 설치한 binary/native/bridge와 frozen helper를 확인한다.
아직 설치하지 않은 다음 제품 source와 현재 설치를 잘못 비교하지 않는다.
첫 비긍정 클래스에서 중단하고 뒤 클래스를 호출하지 않는다.
한 번의 Main0 준비 setter와 그 뒤 독립 확인만 허용한다.
저장이나 재생은 없다.

두 unit의 독립 검토는 사용자 중단 시 이미 진행 중이었다.
중단 마무리 과정에서 그 오프라인 검토만 끝내고 실제 owner·DAW 호출은 시작하지 않았다.
unit64는 독립7개 검사 묶음, unit65는 독립5개 검사 묶음을 통과했다.
unit64 검토는66개 source·39개 data pin과 잘못된20개 page 및376개 unsafe request 변형을 확인했다.
unit65 검토는28개 잘못된 binding 거부와 Room Reverb8회 전체 관측의 분석을 확인했다.
두 review 모두 PASS이며 readiness는 사용자 중단에 따라 false다.
재개 때 `unit64-independent-review`와 `unit65-independent-review`의 최종 verdict·hash를 확인한다.

## 11. 재개 직후 실행 체크리스트

- [ ] 사용자의 새 재개 지시가 있는지 확인한다.
- [ ] AGENTS·RESUME·최신 상태 JSON과 이 문서를 읽는다.
- [ ] Git branch·HEAD·remote와 사용자 변경을 조회한다. 변경을 지우지 않는다.
- [ ] 앱·serve·owner가 실제로 종료돼 있는지 확인한다.
- [ ] Native34 정상 UUID·네 native 파일·main과 bridge 세 파일을 대조한다.
- [ ] R2 source freeze와 native/wire 독립 검토의 현재 source hash를 확인한다.
- [ ] 세 소유 곡의 저장 해시가 중단 원본과 같은지 확인한다.
- [ ] unit64·65의 최종 오프라인 review에서 open blocker가 없는지 확인한다.
- [ ] binding을 fresh 관찰로 채운다. pending 필드를 과거 token으로 대체하지 않는다.
- [ ] 완료한 unit62·63 대신 unit64·65를 실행할 새 고정 parent queue를 구성한다.
- [ ] 각 reviewed child SHA와 output namespace가 새 queue와 일치하는지 확인한다.
- [ ] owner를 먼저 시작한 뒤 정확한 소유 곡만 연다.
- [ ] unit64를 완료하고 앱 종료·owner 종료·원본 파일 불변성을 확인한다.
- [ ] 결과와 actual elapsed·잔여 수·ETA를 사용자에게 체크리스트로 알린다.
- [ ] 다음 unit65를 새 fresh context에서 실행한다.

다음 command는 binding과 source 검토를 마친 뒤의 실행 entry다.
현재 pending 상태에서 실행하면 owner 시작 전에 거부돼야 한다.

```sh
python3 .local/next-state/unit64-chorder-chords-readonly/run_getter64.py
python3 .local/next-state/unit65-preset-remainder/run_presets65_batch.py
```

두 command를 동시에 실행하지 않는다.
가능하면 기존 유한 launcher의 새 검토된 queue로 순차 실행한다.
외부 터미널 session ID나 옛 MCP 객체를 되살리지 않는다.

## 12. Chorder 저장 코드와 나머지261개 writable 역할

현재 fixed provider는908개 이름, writable268개와 read-only640개다.
선정 configuration5개와 GUI learning2개 뒤에261개의 별도 writable 역할이 남는다.

| 역할 | 수 | 남은 검증 |
|---|---:|---|
| per-output learnChord | 128 | 선택한 입력 chord의 membership 변경·읽기·정확 복원 |
| per-key guiInput | 128 | note-on velocity와 note-off, held 상태·정상 종료 lifecycle |
| global command·audition | 5 | playLearnChord, clear, clearAll, copy, paste의 owning receiver와 결과 |

640개 indicator는 playingLive·playingScheduled·outputPlayingLive·outputPlayingScheduled·chordAssigned 계열이다.
indicator의 metadata를 읽은 사실은 그 이벤트나 image 내용을 전부 검증했다는 뜻이 아니다.
per-pitch copy나 clear가 따로 존재한다고 이름을 만들어 호출하지 않는다.

다음 편집 단계는 unit64의 typed 저장 모델을 독립 witness로 사용한다.
닫힌 `chord_note` intent에 expected_input_pitch, output_pitch와 enabled를 갖는 설계가 준비돼 있다.
GUI Mode1과 정확히 일치하는 선택 입력을 begin 직전 및 관련 guard에서 요구해야 한다.
expected_input_pitch는 비교 전제다. 암묵적으로 입력을 선택하는 명령이 아니다.

고정 child probe는 성공했지만 MIDI0..127과 pitch1..128의 일반 매핑은 별도 경계 관찰이 필요하다.
처음에는 입력60의 한 출력 membership만 바꾼 새 유한 단위를 계획한다.
변경 모델·전체908개 changed pair를 확인한 뒤 같은 membership을 원본으로 복원한다.
다른 입력 chord와 configuration이 바뀌지 않았는지 검사한다.
그 다음 별도 소유 copy에서 변경·복원 각각의 정상 Save와 완전 재시작을 확인한다.

IStorable codec writer는 GUI receiver가 요구 편집을 할 수 없을 때의 별도 대안이다.
현재 getter에 임의 XML import나 일반 load 기능을 추가하면 안 된다.
codec writer가 필요하면 전체 알려진 schema와 expected capture digest를 먼저 검증한다.
한 chord만 변경한 내부 bytes를 구성하고 비모델 bytes 보존과 semantic 차이를 확인해야 한다.
load가 시작된 뒤 실패하면 unknown으로 끝내며 자동 repair나 retry를 보내지 않는다.

기존 Auto Fill의 GUI Save 성공과 legacy DSP facade Save 실패를 모두 유지한다.
Native32 Mode의 동일581바이트 관찰과 no-Save reset을 stored-chord 편집 성공으로 바꾸지 않는다.

## 13. 프리셋 이후에도 남은 전체 작업

### 13.1 Vocal Tune bypass와 전용 callback

root와12개 음 mask의13개 current-value 사례는 이전에 완료했다.
host bypass는 별도 역할이며 capsule이 복원한다고 가정하지 않았다.
fresh26 pair와 owning parameter/observer를 읽는 preflight가 필요하다.
검토한 direct-only exactFloat64 `0 → 1 → 0` 두 setter pilot을 별도로 만든다.
변경 pair·원본 pair·최종 독립 pair를 통과하면 현재값 한 이름만 수락한다.
오디오 bypass·latency·저장이나 다른 클래스 bypass로 확대하지 않는다.

### 13.2 악기·macro·Fender 모델

| 영역 | 남은 역할과 구현 |
|---|---|
| Mojito | MIDI carrier3개와 이벤트 lifecycle |
| Mai Tai | MIDI6개, GUI edit-view binding, 조건부 character/model |
| SampleOne | MIDI6개, sampler load/model 상태 |
| Presence | MIDI6개, script/program에 묶인 macro16개 |
| Multi Instrument | macro20개의 실제 target map, facade130개 assignment·child identity |
| Ampire | alias12·linked mix3·routing1·tuner GUI2·host bypass1 |
| Pedalboard | alias11과 host bypass callback |
| Mustang·Rumble | model facade, amp 위치·stomp graph·routing·tuner·bypass |
| dynamics·meter | resetmin, Scope Oneshot/Retrigger, Level Meter resethold/resetloudness |

Mustang·Rumble의 정상 GUI model 목록은 기존 관찰에서 비어 있었다.
50개 effect placeholder는0..0 sentinel이었고 두 ampmodel facade는 빈 표시였다.
빈 목록이 모든 모델을 지원했다거나 실제 rack이 비어 있다는 증거는 아니다.
라이선스나 private initialization을 바꿔 선택값을 강제로 채우지 않았다.
실제 model domain·workspace route와 assignment를 먼저 확인한다.

Scope·meter command는 일반 numeric scalar sweep에 넣지 않는다.
action·capture·autoreset·audition의 정상 lifecycle을 닫힌 operation으로 검토한다.
이미 완료한 세 filter swap action을 persistent parameter 수에 다시 더하지 않는다.

### 13.3 CV·External 하드웨어

CV Instrument97개는 config86, calibration2, monitor2, MIDI carrier3, generator selector4로 분리한다.
네 CV Gate/Pitch·modulation 포트의 연결과 격리 증거가 먼저다.
Main0은 독립 CV 출력 차단이나 물리 무음의 증거가 아니다.
실제 장치가 필요한 calibration·monitor lifecycle은 하드웨어 조건으로 구분한다.
External Mono·Stereo에는 latencyTotal2, unit/UI 문자열4, physical ping2가 별도로 남아 있다.
기존 disconnected 모드·gain·mix·latency·phase·문자 결과는 그대로 재사용한다.
하드웨어가 없으면 제한을 정확히 기록하고 독립적으로 가능한 다른 작업을 진행한다.

### 13.4 일반 중첩 그래프

기존 Splitter42개 제어와 두 Mixtool whole-Inserts 왕복은 완료 증거다.
일반 그래프 create/connect/reorder/delete를 제공하는 API는 아직 별도 구현이 필요하다.
graph·node·slot·Device의 정확한 identity와 소유권 모델부터 만든다.
작은 소유 fixture에서 전체 topology와 plugin state의 전후·복원을 비교한다.
handle·token 무효화와 다른 sibling 보존을 확인한다.
partial·unknown 뒤 blind folder load를 자동 복구로 보내지 않는다.
과거 Pro EQ의 `default*`와 `default` 표시 차이를 strict display 복원 성공으로 바꾸지 않는다.

### 13.5 합집합 감사·지원표·릴리스

native class UID·source·canonical name 기준으로 최신 distinct-name 집합을 다시 계산한다.
완료 Focus33·MIDI2·Vocal13·Chorder current-value를 기존 고정 감사에 연결한다.
실패 단위, duplicate, sibling facade와 같은 선택값의 반복을 별도 이름으로 더하지 않는다.
read-only·opaque·unavailable·조건부 상태를 분리한다.
지원표는 read, write, current readback, state restore, Save/reopen, event와 audio를 구분한다.
README·검증 문서·최초 계획·PR 설명을 최종 구현에 맞춘다.
CI는 실제 실행 결과를 확인하고 검증하지 않은 플랫폼은 그대로 제한한다.
완료 범위가 명확해질 때까지 PR #4는 Draft로 유지하고 자동 merge를 시작하지 않는다.

### 13.6 최초 P0–P5의 장기 범위

내장 플러그인 우선순위 때문에 다음 최초 범위를 삭제하지 않는다.
이미 구현한 부분은 VALIDATION과 현재 도구에서 찾아 중복 구현을 피한다.

- no-document wakeup, 재연결, 대기 중 문서 전환, 중복 이름, 부분 mailbox와 lock 경쟁.
- 대규모 fixture, queue 취소·deadline, 손상 archive와 capability·지원표 정합성.
- transport·tempo·loop·metronome·mixer·send·bus/output routing의 조합과 복원.
- MIDI part/event 편집, 빈 part의 첫 note, batch·duplicate·경계·단위·partial 처리.
- plugin insert/reorder/delete, stale handle과 third-party AU/VST3의 노출 한계.
- automation 기존 곡선 읽기, direct point write 또는 제한된 recording의 실제 지원.
- mix/plugin snapshot 필드, 녹음·bounce·export job start/status/cancel과 artifact.
- Core Audio process tap 권한, callback 제약, 상태 복원과 buffer·메모리 관찰.
- mixdown·stems command와 실제 파일 format·길이·channel 확인.
- WAV loudness·peak·clipping·band·stereo 분석의 기준 신호와 독립 비교.
- 설치·제거·업데이트·복구·platform packaging·사용법과 지원 범위에 맞는 배포.

## 14. 재사용할 이전 완료 증거와 알려진 실패

현재 inventory는60 class fixture의8,668개 record,59 Device provider와 별도 Splitter다.
Impact의 서로 다른 현재 이름4,262개는 direct4,227·pad0 Focus33·MIDI 현재값2다.
ordinary 악기755개와 추가 ordinary51개, Vocal Tune13개도 완료된 단위만 보존한다.
Arpeggiator107·Repeater141 현재 writable와 Surround Panner47개는 재개 때 다시 sweep하지 않는다.
모델 선택자23개·비기본298선택, catalog59종 수명 검사는 실제 recall46종 계획과 별도다.

과거 SampleOne·Impact의 앱 종료 단위는 최종 체크포인트가 없어서0개로 남겼다.
그 뒤 새 단위에서 완료한 결과만 수락했다.
GC 조사에서는 JS proxy finalizer·SongDocument destructor·forced GC 재진입 경로를 확인했다.
native document context로 bridge/EditTask의 JS Song proxy 생성 경로를 피했다.
Native30의 새 Focus20단위·문서 전환·EditTask 회귀는 종료 재발 없이 완료했다.
이것이 전역 GC crash fix 보장이라는 주장은 하지 않는다.

예전 Variant padding 수정도 GC 해결로 승격하지 않는다.
Ampire read-only meter 변화로 실패한 단위는 명시적인 새 observation 정책으로 분리했다.
Chorder의 legacy facade persistence 실패와 초기 direct learning-bit 실패도 지우지 않는다.

## 15. 실제 실행·정리 규칙

1. 실제 DAW 요청은 프로세스 간 단일 owner로 직렬화한다.
2. owner를 먼저 시작하고 앱과 정확한 소유 fixture를 연다.
3. fresh session·document·instance·class·source를 확인한다.
4. setter·preset·state mutation 전 Main numeric0과 존재하는 output:null을 확인한다.
5. bool0, missing output, 모호한 Main이나 다른 routed output은 허용하지 않는다.
6. Audio/Music fixture의 빈 event·mute·arm·monitor·stopped 조건을 해당 계획대로 확인한다.
7. MCP Validation Scratch에서는 Play를 누르지 않는다. 기존 MCP Keys에 synthetic part가 있다.
8. Chorder diagnostic은 전역 빈 한 Music 트랙이며 mute0의 별도 고정 fixture다.
9. unknown mutation·read error·예상 밖 peer·schema·context 차이 뒤 write·load·retry를 보내지 않는다.
10. 실패한 단위는 소유 unsaved scratch를 정상 폐기하는 경로로 정리한다.
11. quiescence가 불명확하면 앱 cleanup을 승인하지 않는다.
12. 앱의 `isRunning:false`를 실제 관찰한 뒤 owner를 마지막에 종료한다.
13. 종료 확인에 CUA `getApp`을 사용하면 앱이 다시 열릴 수 있으므로 `listApps`로 확인한다.
14. UI index는 fresh AX state에서 얻는다. 옛 index와 PTY 번호를 재사용하지 않는다.
15. 설치나 active release binary 변경은 진행 중 owner가 없을 때 수행한다.

소유 곡은 `MCP Validation Scratch`, `MCP Chorder Processing Diagnostic`,
`MCP Chorder Transient Restart Diagnostic`이다.
원본 저장 해시는 각각 `ed0b86bc4551321bccbb2662ab5ed3601023d2696a710e019c5f44b4f2f1c443`,
`7f62f3c0b5166d0f22e5134444e3d47e8e8aec60ae69e77d041347083a3de1e5`,
`5e91af30ab9c2d53a9bde053286541866bb72544b559b94985361c0396bb2b55`다.
절대 경로와 raw song·app log·token·vendor resource는 공개 Git에 넣지 않는다.
로컬 증거는 `.local`에 보존하며 Git clone만으로 따라오지 않는다.

## 16. 비공개 증거 지도

다음 경로는 저장소 루트 기준이다. raw 내용은 공개 문서에 복사하지 않는다.

| 범위 | 경로 |
|---|---|
| 재개 확인·pitch | `.local/next-state/unit48-resume-preflight.json`, `unit48a-pitch-roundtrip-result.json` |
| Mode 측정·폐기 | `.local/next-state/unit48b-mode-measurement-stop-result.json`, `unit48b-discard-recovery-proof.json` |
| Native32 한도 수정 | `.local/next-state/unit49-learning-bound-fix/`, `unit49-learning-bound-review/` |
| Mode 고정 계획 | `.local/next-state/unit49-mode-state-plan/` |
| Mode 완료 | `.local/next-state/unit50-mode-roundtrip-result.json`, `.local/live/unit50-mode-batch/` |
| 최초 추가 프리셋 | `.local/next-state/unit51-presets-partial-result.json`, `.local/live/unit51-preset-batch/` |
| Save As·Mode-only Save | `.local/next-state/unit52-transient-worker/run/`, `unit53-transient-worker/run/` |
| no-Save reset | `.local/next-state/unit55-no-save-reset-result.json` |
| Gate 최초 실패 | `.local/live/unit54-preset-batch/` |
| Expander hybrid1 | `.local/next-state/unit56-expander-hybrid-result.json` |
| strict typed 정책·기존 감사 | `.local/next-state/unit57-preset-preparation/`, `actual51-typed-final-audit.json` |
| Tricomp·Multiband | `.local/live/unit57-preset-batch/`, `.local/next-state/unit57-multiband-restoration-differences.json` |
| 변조9종·Surround 실패 | `.local/live/unit58-preset-batch/`, `.local/next-state/unit64-surround-measurement.json` |
| Expander2·Gate1 | `.local/live/unit59-expander-index2/`, `.local/live/unit60-gate-hybrid/` |
| Native33 구현·수명 검토 | `.local/next-state/native33-learning-child-probe/`, `native33-lifetime-review/`, `native33-wire-review/` |
| Native33 실제 읽기 | `.local/next-state/unit61-learning-child-readonly/run/` |
| terminal response 반례·수정 | `.local/next-state/unit61-independent-review/` |
| Gate2·Analog·Room | `.local/live/unit62-gate-index2/`, `.local/live/unit63-preset-remainder/` |
| unit62·63 독립 검토 | `.local/next-state/unit62-63-independent-review/` |
| 저장 모델 설계 | `.local/next-state/chorder-stored-model-design/REPORT.md`, `chorder34-contract.json` |
| Native34 native·R2 freeze | `.local/next-state/chorder34-native/`, `source-freeze-r2.json` |
| Native34 wire | `.local/next-state/chorder34-wire/` |
| Native34 독립 검토 | `.local/next-state/chorder34-native-review/`, `chorder34-wire-review/` |
| 정상 Native34 패키지 | `.local/next-state/unit64-artifacts/native-artifact-manifest.json`와 package·dSYM |
| 최종 설치·종료·곡 보존 관찰 | `.local/next-state/pause-2026-11-12-installed-observation.json` |
| 다음 unit64 | `.local/next-state/unit64-chorder-chords-readonly/`, `unit64-independent-review/` |
| 다음 unit65·Room 측정 | `.local/next-state/unit65-preset-remainder/`, `unit65-independent-review/` |
| 전체 프리셋 분류 | `.local/next-state/remaining-preset-recalls/classification.json`와 controls |
| 전용 역할 분류·합집합 | `.local/next-state/remaining-work3072/`, `progress-union-review/` |
| Vocal Tune bypass 계획 | `.local/next-state/vocal-bypass-plan/` |

제품 source의 주요 변경 지점은 `src/operation.rs`, `src/server.rs`, `src/runtime.rs`,
`src/runtime/chorder_chords.rs`, bridge 세 파일과 native `request/chorder_chords.rs`다.
GUI child는 `gui_parameter/learning_child.rs`, 기존 global learning은 `gui_model.rs`에 있다.
제품 source 수정이 이미 설치한 binary나 frozen verifier를 자동으로 갱신하지 않는다.

## 17. Git와 완료 시점 기록

| 커밋 | 역할 |
|---|---|
| `486d611` | 이전 Native31 learning getter/setter 제품 source |
| `48bf48c` | Native32 learning 전용144뷰 한도 |
| `045c458` | Native33 고정 learning child 메타데이터와 당시 프리셋 기록 |
| `3611031` | 실제 Native33 unit61 결과와 다음 작업 문서 |
| `3815714` | Native34 저장 코드 getter와 unit62·63까지의 문서 |

이 인수인계와 최종 paused 상태는 뒤의 별도 문서 commit에 기록한다.
제품 source commit을 amend하거나 실패 증거를 삭제하지 않는다.
공개 PR은 Draft이며 merge·release 완료를 주장하지 않는다.
최종 push와 깨끗한 작업 트리는 root의 종료 체크포인트에서 확인한다.

## 18. 중단 완료 조건과 보고 방식

사용자 요청 뒤에는 unit64·65의 owner나 실제 DAW 호출을 시작하지 않았다.
이미 진행 중인 오프라인 검토와 Native34 설치 상태 확인을 마무리했다.
최종 설치·곡 해시 관찰 시각은2026-11-12 09:50:26 UTC다.
이후 자세한 인수인계·paused 상태·재개 진입점을 commit하고 push한다.
자동 재개, 예약 실행이나 새 검증 loop를 만들지 않는다.

재개 이후 단위 완료 보고는 다음 정보를 포함한다.

- 완료한 입력 범위와 최종 읽기·원본 복원 여부.
- 실제 검증 실행 시간과 UI·정리를 포함한 시간의 구분.
- 해당 유한 계획의 완료 수와 잔여 수.
- 다음에 진행할 단위와 알려진 미확인 결과.
- 전체 남은 시간 범위와 이전 추정에서 달라진 근거.

중단 직전 전체 예상 잔여 시간은6~17시간이며 신뢰도는 낮다.
이 범위는 재개 후 실제 작업 시간 추정이다. 이동·중단 시간은 포함하지 않는다.
stored-chord 편집·저장·중첩 그래프·전용 역할이 구현 불확실성의 큰 부분이다.
실제 장치가 필요한 CV·External 검증은 별도 조건이다.
이 숫자를 완료 보장이나 전체 지원률로 표현하지 않는다.

## 부록A. 잔여28종의 클래스 식별값

아래 class UID는 catalog에서 관찰한 클래스 식별값이다. 만료되는 runtime object UID가 아니다.
catalog 수와 provider record 수는 과거 고정 분류의 값이다.
새 실기에서는 fresh catalog·instance·schema를 확인하고 발급된 index만 사용한다.

| 클래스 | class UID | catalog 수 | provider record 수 |
|---|---|---:|---:|
| Multiband Dynamics | 701812B1-7B20-4C95-BECB-D7C2ACF954ED | 28 | 71 |
| Surround Delay | 47273102-4C60-4D74-A700-674C38FDD154 | 74 | 79 |
| Room Reverb | 29C71194-B29A-40C0-9A35-9053DB6F596C | 73 | 19 |
| Mixverb | FEB6F7CB-36CA-4CBA-9090-42B7C76F4158 | 24 | 9 |
| Open AIR | 507EFD36-8B20-4A37-A8C2-36FFC4D30ED1 | 1 | 94 |
| Spectrum Meter | 0C6C94DF-CCE0-465C-886A-F1F01A142104 | 32 | 9 |
| Level Meter | DF958D90-0196-434F-ADC3-1FC2BB7F4AF6 | 6 | 7 |
| Scope | 36322412-AD90-4EE0-B63C-37929566B081 | 1 | 31 |
| Tone Generator | A1C299EA-7586-4B51-95A0-9BCFE682DE9C | 6 | 10 |
| Ampire | B6407C28-0F92-4538-9E7F-9B867B3FEA74 | 114 | 94 |
| Pedalboard | BC9129B4-EC67-41B8-96DE-EC128D5FE54D | 53 | 28 |
| Rumble Native | B9D7536C-A010-457D-A182-36E5531164C9 | 111 | 39 |
| Studio Verb | 1E1F2DB4-47C7-4766-8A8D-D3309A4CF500 | 121 | 151 |
| Compressor | 36F3F4D1-CBB4-4BF7-A7E3-EBCCED53718B | 89 | 154 |
| Tuner | 4F748A80-0D49-4E8A-A558-B120D824512B | 5 | 13 |
| Voice FX | 9F3AF9D1-3820-46CF-A675-2F382D6D334E | 12 | 5 |
| Rotor | 072BAC58-596A-4D6A-8F01-978B96E4BF36 | 18 | 29 |
| De-Esser | A7036AE3-E3A3-4C6F-A3DD-F84CB5C9BDCB | 14 | 10 |
| Vocoder | AB032906-D099-466C-903C-DC3409459F99 | 27 | 117 |
| VU Meter | B05B181C-6417-4441-A6F4-CC3E780548AD | 8 | 11 |
| Vocal Tune | 90D01A1C-EB13-4C5F-8C45-904F4AD0AFC0 | 13 | 26 |
| Multi Instrument | B9D410AD-E47D-495E-8AEE-10D2B2987CC7 | 50 | 150 |
| SampleOne | C37BC9D1-6BD1-46A7-A60C-B13438666448 | 1 | 235 |
| Impact | 3713E26C-2FCA-4024-9F25-17E9D2BE2B9B | 2 | 4262 |
| CV Instrument | EDCD22A5-A01E-4EA6-9FE4-2BB1AB243E85 | 1 | 110 |
| Mojito | 19C6BEAB-1A48-4338-BB04-14A9AC1FCC05 | 50 | 37 |
| Chorder | D2D9B002-12FC-4F1D-AFDA-70336D1DC108 | 24 | 908 |
| Repeater | 2F791B7D-E24D-0A4D-93E1-7D497CD6399F | 15 | 141 |

이 표는 기존 classification의 nonempty46종에서 완료18종을 뺀 목록이다.
classification 파일의 옛 nonempty_without_positive label은 현재 완료 여부를 나타내지 않는다.
현재 완료 여부는 RESUME_STATE와 최신 public preset evidence를 기준으로 판단한다.

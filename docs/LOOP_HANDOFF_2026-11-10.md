# Fender Studio Pro MCP 루프 인수인계

작성일은 2026-11-10이다. 사용자가 이동을 위해 **현재 단위까지만 마친 뒤 루프를 종료**하도록 요청했다.
이 문서는 장시간 진행한 내장 플러그인 확장·검증 작업의 완료 내용, 실패 이력, 현재 설치 상태와 남은 실행 순서를 보존한다.
작업은 사용자 요청으로 중단됐다. 자동 재개, 예약 작업과 백그라운드 검증은 없다.

## 1. 재개 방법과 문서 우선순위

사용자가 **“진행 재개해”**라고 하면 이 문서와 [RESUME_STATE.json](RESUME_STATE.json)을 먼저 읽고
아래의 재개 순서에서 이어간다. 저장소 `AGENTS.md`에도 같은 진입점을 기록했다.

현재 상태를 판단하는 순서는 다음과 같다.

1. 사용자의 최신 지시와 저장소 규칙.
2. 이 문서의 중단 상태·다음 작업, 실제 Git와 설치 파일 상태.
3. [플러그인 상태 검증](PLUGIN_STATE_VALIDATION.md)의 개별 실험 결과와 제한.
4. 로컬 비공개 증거의 완료 체크포인트·해시·소스 바인딩.
5. [구현 계획](IMPLEMENTATION_PLAN.md)의 최초 P0–P5 계획.

최초 계획에는 이미 다른 형태로 구현한 항목과 과거의 미체크 상자가 함께 남아 있다.
체크박스가 오래됐다는 이유로 완료된 기능을 다시 구현하거나 전수 검사를 처음부터 돌리지 않는다.
반대로 코드가 존재하거나 요청이 성공했다는 이유만으로 실제 검증 완료를 선언하지 않는다.

이 문서의 기능·숫자는 중단 시점의 제한된 증거다. 전체 내장 플러그인, 모든 선택 상태,
모든 프리셋, 실제 소리와 파일 저장까지 완료했다는 뜻은 아니다.

## 2. 사용자 목표와 유지할 결정

- macOS를 우선 지원하는 Rust MCP 서버를 만든다.
- Fender Studio Pro의 가능한 많은 작업을 LLM이 수행하게 한다.
- 브리지·EditTask와 내장 플러그인 제어를 실제 **8.1.1 Build 113103**에서 검증한다.
- 대표 값 몇 개에 그치지 않고 쓰기 가능한 전체 파라미터의 변경·읽기·복원을 확대한다.
- Splitter, Surround Panner, External Effect, 모델·프리셋·중첩 그래프를 포함한다.
- 공개 저장소 `dev-universe/fender-studio-pro-mcp`에서 버전 관리한다.
- Hopper는 체험 기간이 끝났다. 추가 분석은 **Ghidra**를 사용한다.
- 다른 노래와 설정을 보존한다. 테스트는 이름이 명확한 소유 fixture에서만 한다.
- 단위 작업이 끝날 때마다 완료·진행·남은 작업을 체크리스트로 알린다.
- 질문이나 상태 요청이 들어왔다고 기존 작업을 중단하지 않는다. 이번에는 사용자가 명시적으로 중단을 요청했다.
- branch 이름과 PR 제목에 `codex`를 사용하지 않는다.

참고 프로젝트 조사와 출처는 [RESEARCH.md](RESEARCH.md)에 보존돼 있다.
기존 Rust 서버를 버리고 새 언어·별도 서버로 다시 시작하는 작업은 필요 없다.

## 3. 중단 시점의 실제 상태

| 항목 | 확인한 상태 |
|---|---|
| 제품 소스 커밋 | `486d6115dcecbcbb96cbbbfd083e014af8eddbb0` |
| 작업 branch | `feature/plugin-state-and-exhaustive-validation` |
| 공개 PR | [#4 Add guarded plugin state and full parameter checks](https://github.com/dev-universe/fender-studio-pro-mcp/pull/4), Draft 유지 |
| Rust MCP 표면 | 도구 42개 |
| 설치한 네이티브 단위 | Native31 |
| Native31 UUID | `411316DF-1CAE-3CE2-86CA-72C39C7E1B0A` |
| Rust release | 위 제품 소스에서 빌드·설치 완료 |
| 설치 파일 검증 | native 4개 파일, 변경 bridge JS, 공유 문서 helper, EditTask package 원본 일치 |
| 마지막 실제 단위 | Chorder Mode-off learning getter 및 전체 908개 원본 읽기 2회 |
| 마지막 getter | 256 한도 안에서 125뷰 완전 탐색, 유일한 바인딩 1개 |
| 학습 Mode / 입력 pitch | `0` / `-1`로 관찰 |
| 새 learning setter 실기 호출 | 0회 |
| 마지막 단위 transport action | 0회 |
| Studio Pro | 정상 종료, `isRunning:false` 확인 |
| MCP owner46·47 | 모두 종료, 프로세스 조회에서도 남은 serve/owner 없음 |
| 소유한 테스트 곡 | 저장하지 않고 닫음, 기존 저장 파일 해시 일치 |
| Ghidra/Hopper | 이번 조사의 Ghidra 세션 정리 완료, Hopper 사용 안 함 |

설치된 확장과 bridge 자체는 남아 있다. 앱이나 owner가 계속 실행되는 상태는 아니다.
인수인계 문서 커밋은 위 제품 소스 이후에 추가된다. 문서만 바뀌면 다시 빌드할 필요는 없다.

로컬 바인딩은 `.local/next-state/unit47-artifact-binding.json`과
`unit47-driver-source-binding.json`에 있다. 마지막 종료 증거는
`unit47-final-pause-state.json`, 마지막 읽기 결과는 `unit47a-readonly-stop-result.json`이다.
실제 재개 때는 이 파일을 현재 프로세스·파일 상태의 대체물로 취급하지 말고 다시 대조한다.

## 4. 완료 범위 요약

| 영역 | 완료한 유한 범위 | 아직 확대하면 안 되는 주장 |
|---|---|---|
| 기반 연결 | Rust MCP, DAW bridge, EditTask, 단일 owner, 문서·세션 가드 | 모든 P0–P5 시나리오 완료 |
| 현재 제공값 | 60종 fixture의 8,668개 record 읽기, 59 Device provider와 별도 Splitter | 조건부·숨은 값 전체 발견 |
| Impact | 직접 4,227 + 현재 pad0 Focus 33 + MIDI 현재값 2 = 서로 겹치지 않는 4,262개 이름 | 모든 pad 선택, 이벤트 수명, 저장·오디오 |
| 일반 악기 | Mojito32, Mai Tai275, SampleOne229, Presence219, 합계755 | 각 악기의 전용 이벤트·매크로·모델 전체 |
| 추가 일반 제어 | Ampire39, Mustang3, Pedalboard2, Rumble3, Vocal Tune4, 합계51 | 해당 클래스의 모든 별도 역할 |
| Vocal Tune 추가 | root 및 12개 음 마스크, 13개 역할 | host bypass 콜백·저장·소리 |
| Note FX | 선정257개, 그중 Arpeggiator107·Repeater141 현재 writable 전체 | Chorder packed model·learning 전체 |
| 모델 선택자 | 23개 선택자, 비기본 선택298건 및 상태 복원 | 모든 조합·숨은 모델 |
| 프리셋 목록 | 59종의 목록 발견·해제·만료 토큰 거부 | 59종 모두의 실제 프리셋 호출 |
| 추가 프리셋 호출 | Mixtool2건, Binaural Pan2건, 각 전체 원본 복원 | 각 클래스의 모든 프리셋 |
| Surround Panner | 현재 쓰기 가능한47개 | 다른 출력 레이아웃·소리 |
| External Effect | 연결되지 않은 상태의 모드12전이·연동6·일반18·문자4 | 실제 하드웨어·ping·보정 |
| 중첩 그래프 | 기존 Splitter42개 제어, 두 Mixtool의 whole-Inserts 왕복 | 일반 그래프 생성·연결·삭제 |
| Chorder Auto Fill GUI | 켜짐·꺼짐 각각 Save/완전 재시작, 최종908값 및 preset bytes 원복 | 다른 chord model·learning의 저장 |
| Chorder learning | 두 역할 소스 구현, Mode-off getter 실제 확인 | 새 setter의 실제 왕복 |

이 숫자를 단순히 더해 전체 지원률을 만들지 않는다. 같은 native class/source/name이 여러 실험에 나타날 수 있다.
마지막 독립 전역 이름 합집합 감사는 **Focus7 경계의 6,288/7,112**다.
이는 이후 Focus·MIDI·Vocal Tune 결과가 추가되기 전의 고정 경계다. 최신 전역 합집합은 재계산하지 않았다.
분모도 그때의 고정 fixture inventory이며 모든 모델·프리셋·오디오 작업의 분모가 아니다.

## 5. 이번 루프에서 확장한 구현

### 5.1 기본 서버와 경계

서버는 Rust native binary다. JavaScript는 DAW 안의 bridge와 EditTask에서 실행된다.
Node와 Python은 개발·검증 도구이며 사용자 서버의 별도 실행 runtime 요구가 아니다.
macOS CoreMIDI wakeup, 프로세스 간 단일 소유권, 파일 mailbox와 직렬 요청 경로를 사용한다.
임의 eval·shell·검사하지 않은 스크립트를 MCP로 실행하는 표면은 만들지 않는다.

요청·응답은 session/document와 대상 instance에 묶인다. 쓰기 결과는 `applied_verified`,
`applied_unverified`, `not_applied`, `unknown`을 구분한다. UI 값 확인을 오디오·저장 검증으로 승격하지 않는다.
저장 곡 읽기와 WAV 분석은 live DAW 관찰과 출처가 다르다.

### 5.2 native provider와 정확한 값

CCL 공개 인터페이스를 감싼 선택 설치형 Rust native 확장을 구현했다.
고정한 공식 CCL 5.1.2 ABI와 실제 앱 이미지 identity를 검사한다.
최근 ABI 검증 범위는 header hash106개·compiler-derived offset/call70개다.
native artifact는 네 파일 패키지와 dSYM·소스 manifest로 보관한다.

직접 Device DSP provider, controller provider와 metadata fallback을 구별한다.
페이지의 complete와 전체 provider 완료를 혼동하지 않고 `next_offset`을 같은 instance/source로 따라간다.
타입·범위·읽기 전용·enabled·native carrier·normalized bits·display text를 함께 읽는다.
Float64 원본 복원에는 native `bits_hex`를 보존하는 `float_bits` 경로가 있다.
불투명 값은 record metadata와 내용 검증을 구분한다.

native description, 제한된 owner reflection, explicit-value formatting과 full-provider digest를 추가했다.
digest는 노출된 값들의 순차 관찰이며 숨은 plugin state나 원자적 snapshot의 증거가 아니다.
Pro EQ199·Impact4262에서 Python 독립 projection과 native digest의 일치를 검증했다.

### 5.3 상태·프리셋·그래프 도구

메모리 안의 상태 capsule capture/restore/release와 발급된 preset catalog의 제한된 recall을 구현했다.
token은 service/document/Device와 관련 owner에 묶이며 임의 파일 import 권한이 아니다.
load 성공은 값 복원 성공이 아니다. 별도 전체 provider 읽기로 확인한다.
해제 토큰·다른 대상 토큰 거부, owner·문서 변경 후 무효화를 구분해 검사한다.

whole-Inserts 저장/복원은 실제 관찰한 폴더 소유권 범위에서만 제공한다.
registered filter swap action은 permutation과 inverse를 별도로 확인했다.
Fender 모델 목록은 현재 categories/effects의 읽기 전용 제한 목록이다.
빈 모델 목록으로부터 selector 선택 개수나 mutation 경로를 추측하지 않는다.

### 5.4 Chorder GUI 전용 경로

직접 DSP facade와 실제 GUI receiver가 다른 경우를 확인했다.
정확한 이미 열린 Chorder DeviceEditor 안의 고정 이름·canonical identity로 제어를 바인딩한다.
범용 GUI parameter 이름이나 caller-configurable 탐색 경로는 만들지 않았다.

`chorder_autofill_set`은 Boolean intent로 로컬 `autofill`을 편집한다.
DSP Float facade와 GUI Integer Toggle을 구분하고 phase-aware NumericEditLease로 한 번의 편집을 수행한다.
공개 결과는 GUI current-value 사실을 별도로 반환하며 계속 `applied_unverified`다.

`plugin_parameter_details`의 `compare_gui:true`와 `include_owner:true` 조합으로
GUI observer controller와 local `learnMode`, `learnInputPitch`를 읽는 진단을 추가했다.
Auto Fill과 같은 canonical GUI anchor, 같은 observer/controller인지를 확인한다.

새 도구 `chorder_learning_get`·`chorder_learning_set`을 구현해 도구 수가40에서42가 됐다.
두 도구 모두 `track`, Note FX `slot`, `expected_instance`, `expected_document`가 필요하다.
getter는 read-only지만 문서 바인딩이 필수다.
setter의 닫힌 입력 형태는 아래 두 가지다.

```json
{"kind":"learn_mode","enabled":true}
```

```json
{"kind":"input_pitch","pitch":0}
```

명시적인 `pitch:null`은 GUI 원본 -1을 뜻한다. pitch 생략, 문자열, bool, 소수와 범위 밖 값은 거부한다.
GUI pitch는 MIDI0..127이며 DSP facade의 별도 offset을 적용하지 않는다.
공통 `remove_nulls`가 null을 지우는 문제는 새 operation의 typed change만 복원하는 방식으로 해결했다.

getter의 고정 discovery ceiling은256뷰, setter는128뷰, 깊이는32다.
setter는 완전 탐색 후 Auto Fill까지의 안정된 ancestor path만 유지하고,
off-path view·leaf children·reflection·임시 witness 소유권을 `begin` 전에 실제로 해제한다.
Mode 전환으로 sibling subtree가 붙거나 떨어져도 이전 GUI tree 전체를 유지하지 않게 한 것이다.
예전 Auto Fill 전용 비교·편집 경로의 한도와 trace는 그대로 보존했다.

ExactInteger profile은 기존 NumericEditLease에 추가했다. 별도의 두 번째 편집 상태 머신은 만들지 않았다.
`learnInputPitch`, tag6, Integer carrier1, 범위-1..127, default-1을 초기·쓰기·최종 단계에서 검사한다.
최종 positive readback 뒤에는 foreign release가 값을 바꿀 수 있는 순서를 두지 않도록 borrowed association 검사를 사용한다.

JS와 Rust 경계 모두 닫힌 응답 DTO·phase·typed scalar를 검사한다.
문서 변경·soft error·malformed 응답에서는 GUI 값·바인딩·검증 사실을 통째로 제거한다.
새 성공도 GUI current-value만 뜻하며 저장·DSP propagation·effects·audio를 검증했다고 반환하지 않는다.

## 6. 실제 검증이 완료된 상세 단위

### 6.1 Impact 직접 값과 Focus

직접 파라미터4,227개는 마지막 전체 체크포인트가 완료된 단위만 수락했다.
지속 필드3,714개와 일반 pad 필드512개·bypass가 포함된다.
과거 중단된 단위의 개별 성공 이벤트는 완료 수에 넣지 않았다.

현재 선택 pad0 Focus33개는 다음과 같다.

- filter cutoff, sample 시작·끝.
- pitch envelope attack/hold/decay/sustain/release.
- filter envelope attack/hold/decay/sustain/release.
- amplifier envelope attack/hold/decay/sustain/release.
- wave transpose/tune.
- filter active/type/clip/drive/resonance/velocity-to-cutoff/compress.
- wave-amp pan/level, voiceAmp velocity amount.
- pitch velocity·pitch ADSR·filter ADSR modulation amount.

각 Focus는 alias와 현재 pad0 peer의 실제 대응을 바인딩했다.
변경 상태와 복원 상태에서 전체4,262개를 각각 두 번 관찰했다.
Native30의 새20단위는11,360 RPC·plugin setter40회를 종료 재발 없이 완료했다.
이전13개와 합쳐33개를 수락했다.

별도 MIDI current-value protocol은 Pitch Bend와 Sustain 각각 변경·원본 복원,
총 네 setter와 전체 provider 검사를 통과했다. 이벤트 전달·held voice·autoreset·저장은 별도다.
직접4,227·Focus33·MIDI2 이름이 겹치지 않고 현재4,262개와 정확히 같은 집합임을 확인했다.

### 6.2 일반 악기755개와 추가51개

Mojito32, Mai Tai275, SampleOne229, Presence219를 모두 변경·읽기·복원했다.
선택한 ordinary partition 전체755개이며, 전용 MIDI·macro·model 역할은 제외했다.
각 값은 전체 writable projection의 fresh pair와 fixture-end 체크포인트까지 요구했다.

Ampire39, Mustang3, Pedalboard2, Rumble3, Vocal Tune4를 별도 exact-name 계획으로 검증했다.
합계51개다. readonly meter 관찰 정책을 사용하는 Ampire를 제외한 수치·metadata 허용 오차는 추가하지 않았다.
SampleOne crash 및 Ampire meter 실패의 미완료 단위는 제외한 뒤 새 단위에서 완료했다.

### 6.3 Vocal Tune 연동 상태13개

Custom scale15, root C0, 12개 음 mask 모두1, bypass0인 고정 원본을 사용했다.
full provider는26개이며 writable19·readonly7이다.
첫 C mask pilot이 통과한 후 나머지11개 mask와 root1(C#)를 진행했다.
root0/1의 표시 문자열을 선택 없이 formatter로 먼저 확인했다.

각 case는 변경된 전체26개 projection 두 번을 확인한 뒤에만 계획된 capsule load를 한 번 허용했다.
모든 복원 gate에서 full typed·normalized·display 원본 pair를 요구했다.
readonly7개도 strict 비교에 포함했다. 해제 토큰 재사용 거부와 최종 독립 원본 pair도 통과했다.
총1,484 RPC·mutation-labelled117회를 사용했고 매번 Main0 및 명시적 output null을 먼저 확인했다.
이117회는 setter13회뿐 아니라 capture/load/release와 negative token call을 포함한다.
host bypass는 이 protocol에서 제외했다.

private preflight checker의 instance 비교를 보완했다.
필수 fresh inspect에 두 full read와 formatter를 묶고 immutable reference의 records는 바꾸지 않는다.
다른 UID의 readonly 비교 증명과 frozen runner 실행 호환성을 구분한다.
실제 이 fixture의 fresh UID는 저장 원본과 같아 frozen runner와 호환됐다.
12개 offline test·54개 CLI 사례로 checker를 검증했으며 protocol·기존 owner·원본은 수정하지 않았다.

### 6.4 프리셋·Panner·External·중첩 그래프

기존 selector 결과는23개·298 비기본 선택이다. 프리셋 catalog는59종에서 수명 주기를 확인했다.
기존 positive recall은 Pro EQ, Fat Channel, Mai Tai, Arpeggiator, Mustang Native, Console Shaper에 있다.
Pro EQ 재검사 등 반복 결과를 별도 parameter 수로 세지 않는다.

이번 말미에 fresh issued catalog를 확인한 뒤 Mixtool의 -24dB/-3dB와
Binaural Pan의 Make Mono/Narrow를 각각 호출·변화 확인·전체 원본 복원했다.
각2건, 추가4건이다. Mixtool provider72개, Binaural Pan5개가 대상이었다.
catalog-only59와 실제 recall 클래스 수는 서로 다른 지표다.

Surround Panner의 현재 writable47개와 capsule 왕복을 확인했다.
External Effect는 연결되지 않은 포트·모드에 한정해12전이·연동6·ordinary18·literal4를 검증했다.
빈 포트 목록은 그대로 빈 목록으로 보고했다. 실제 외부 장치·물리 calibration을 한 것은 아니다.

중첩 fixture에서는42개 Splitter 제어와 두 Mixtool의 whole-Inserts 왕복을 검증했다.
folder state2,859 bytes·두 part·두 load·두 setter·한 release의 고정 실험이다.
현재72/142 writable provider와 Splitter14 active field를 전체 원본과 비교했다.
이것이 일반 topology create/connect/delete의 증거는 아니다.

### 6.5 Chorder Auto Fill 지속성 및 새 getter

legacy DSP raw·normalized·`native_user` 편집은 live 값이 바뀌어도 Save/reopen에서 유지되지 않았다.
이 실패를 지우거나 GUI 결과로 legacy route를 성공 처리하지 않는다.

GUI 전용 Auto Fill enable 한 번 후 full908 두 번, Save와 완전 앱 재시작 후908 두 번을 확인했다.
원본 disable 한 번 후 같은 검사를 반복했고 마지막 Chorder preset member581 bytes가 원본과 같았다.
두 GUI setter·두 Save·양방향 완전 재시작의 유한 결과다.
학습 모드나 transport는 사용하지 않았다.

Native30에서는 GUI controller probe를 두 번 실행하고 앞뒤 full908 원본 pair를 비교했다.
ChorderComponent direct controller, same GUI Auto Fill canonical anchor,
`learnMode` Toggle Integer tag5 범위0..1, `learnInputPitch` Integer tag6 범위-1..127/default-1을 관찰했다.

사용자의 중단 요청 후 마지막 진행 단위만 마무리했다.
Native31과42-tool binary를 설치하고 소유 diagnostic을 열어 getter를 실행했다.
Mode0/pitch-1, visited125/max256, complete true/candidate1을 실제 확인했다.
전체908개를 두 번 읽고 Native30에서 보존한 저장 원본 projection과 정확히 같음을 확인했다.
그 뒤 저장하지 않고 닫고 앱·owner를 순서대로 종료했다.
Mode-on 탐색과 learning setter는 실행하지 않았다.

## 7. 종료·실패·수정 이력과 반복하면 안 되는 판단

### 7.1 SampleOne·Impact 앱 종료

SampleOne의 과거 단위는124개 중122개 individual event 뒤 앱 종료로 final checkpoint가 없었다.
그 단위는0개로 처리했다. 원본235개를 두 번 복구 확인한 뒤 disjoint224개와
별도 stepL12 한 개를 새 실험으로 검증해 ordinary229개를 완료했다.

Impact Focus amplifier Attack의 첫 실험도 두 setter는 verified였지만 마지막 full original read에서 종료됐다.
실패 단위는0개이며 setter·restore·retry를 이어서 보내지 않았다.
나중 Native30에서 새 단위로 같은 역할을 검증한 결과만 수락했다.

두 crash report는 처음24 frames와 fault instruction bytes가 같았다.
Ghidra로 정확한 설치 jsengine·main image를 묶어 조사했다.
fatal condition은 `MOZ_RELEASE_ASSERT(!JS::RuntimeHeapIsBusy())`였다.
JS proxy finalizer의 native release 뒤 SongDocument 소멸자가 global scripting GC를 강제 호출해
collector가 이미 바쁜 상태에서 forced context GC로 재진입하는 경로를 확인했다.
정확한 이전 proxy reference의 provenance와 모든 trigger를 찾은 것은 아니다.

Rust script-return Variant의 C++ padding 보존 문제는 별도로 수정했다.
그 수정이 설치된 뒤에도 GC crash가 재발했으므로 padding 수정을 GC 해결책으로 쓰지 않는다.
host output string copy/receipt 보관 경로를 조사했지만 Rust output string 소유권 오류를 crash 원인으로 확정하지 않았다.

### 7.2 native document context 수정

버전 있는 `documentContext()` native service를 추가했다.
service serial·generation과 canonical native document identity를 사용하며,
동시에 보존하는 마지막 문서 identity는 하나다. close 이후 다음 query/stop 전까지 마지막 native owner가 남을 수 있다.
bridge context, song metadata, EditTask lease의 세 JS document-proxy 생성 경로를 대체했다.
bridge와 EditTask는 같은 helper 소스를 패키징한다.

native API를 선택한 bridge는 실패 시 legacy JS proxy로 fallback하지 않는다.
문서 context 실패는 epoch와 dispatch 권한을 무효화한다.
API가 없는 host의 legacy 경로는 호환을 위해 남아 있다.
GC 전체를 비활성화하거나 vendor binary를 patch하지 않았다.

Native30에서 문서 전환 두 번, EditTask read probe, 잔여 Focus20개가 종료 없이 통과했다.
이는 관찰한 경로 회피와 제한된 회귀 증거다. 전역 crash fix 보장은 계속 false다.
Song이 없는 시작 페이지에서는 bridge clock이 움직이지 않아 요청 timeout이 있었다.
같은 상태로 반복하지 않았으며, no-document 응답을 항상 성공했다고 기록하지 않는다.

### 7.3 Ampire meter와 Chorder 기타 실패

strict Ampire pilot은 readonly peak meter 두 개만 변해 실패했다. 그 단위는0개다.
schema와 writable69개가 같다는 독립 진단 뒤 readonly를 observation으로 별도 기록하는
정책을 명시하고 새4+35 단위에서 완료했다. writable tolerance나 원본 rebase는 하지 않았다.

Chorder 초기 GUI 탐색은 depth limit 후75뷰에서 중단됐다.
고정 depth32로125뷰 완전 탐색을 얻었고, DSP full name 대신 실제 GUI local `autofill`을 정확히 바인딩했다.
호출자 임의 이름·fallback matching으로 우회하지 않았다.
과거 selector-to-core-pitch·learning-bit 실험의 미완료/불일치 결과도 유지한다.
이전 실패들을 새 learning getter 성공으로 완료 처리하지 않는다.

Native31 마지막 preflight에서는 include_context에 expected_instance가 없는 요청과,
editor-open에 지원하지 않는 expected_instance를 넣은 요청이 schema/argument 단계에서 거부됐다.
실제 plugin setter dispatch는0회였다. 올바른 fresh inspect와 editor DTO로 읽기 단위만 마무리했다.
재개할 때 같은 잘못된 요청 형식을 복사하지 않는다.

### 7.4 검증 도구의 오류·증거 형식

private owner의 오래된 batch filename 오류와 freshness 만료는 실제 parameter 쓰기 전에 멈췄다.
새 owner는 없는/잘못된 파일·경로·타입·재사용 label을 DAW dispatch 전에 거부한다.
검증 script 수정은 active driver나 frozen snapshot에 덮어쓰지 않고 새 namespace에서 한다.

Journal baseline SHA는 **마지막 개행을 제외한 canonical JSON**의 hash다.
실제 파일 bytes SHA와 다를 수 있다. 둘을 같은 것으로 비교해 증거가 손상됐다고 판단하지 않는다.
raw file hash와 canonical reference hash를 각각 검증한다.

설치 EditTask는 디렉터리이고 실제 archive는 그 안 `scripts/fender-studio-pro-mcp.package`다.
디렉터리에 read_bytes를 호출한 운영 script 오류가 한 번 있었고, 실제 package entries를 원본7개와 비교해 정정했다.
제품 오류나 앱 상태 변경은 없었다.

## 8. 최신 소스 검증과 독립 검토

학습 제어의 데이터 형태와 GUI tree 변경 문제를 두 설계로 비교했다.
안정된 path projection 설계를 선택하고 Mode 양쪽의 실제 탐색·복원 gate와
Mode-off `null → MIDI0 → null` 첫 pilot을 통합했다. getter 한도를 이유 없이 setter에 적용하지 않았다.

제품 소스 `486d611`에 대응하는 검증은 다음과 같다.

| 검사 | 결과와 근거 범위 |
|---|---|
| native add-on | Rust1.88 test241개 통과 |
| parameter library | test100개 + doc test4개 통과 |
| Rust server | test91개 통과 |
| JS 새 학습 그룹 | 보관된 targeted9개 통과 |
| JS 전체 | builder가273개 통과를 관찰, 전체 output은 별도 보존하지 않음 |
| 새 native 실제 함수 통합 | fake-host의 real capture/Reply/Receipt14개 통과, 실제 DAW 결과와 구별 |
| native 직렬화 fixture | 9개를 현재 JS 경계에 재주입해 호환 확인 |
| 독립 경계 검토 | 문서·session·open 변경/예외·reserved field 등35사례 통과 |
| 형식·lint | Root/add-on/parameter의 fmt 및 해당 all-target Clippy 통과 |
| source binding | 독립 검토17파일, native freeze9파일 및 로그 해시 일치 |

통합 검사는 sibling attach/detach, off-path owner의 begin 전 해제,
projection-release 재진입, 초기 Integer metadata drift 시 begin0회,
late value witness 무효화, peer/root 손실, null/0/127, getter>128 완전 탐색과 setter128 거부를 포함한다.
임시 사본에서 projection 해제를 없애거나 Integer profile을 General로 바꾸면 해당 검사가 실패했다.
JS admission check 제거도 targeted9개 중3개 실패를 일으켰다.
반증 실험 후 제품 소스가 복원·동결됐음을 해시로 확인했다.

Native document context의 이전 소스 검증은 add-on238·Root82·JS264와
실패 QI sentinel·String OUT 교체·busy 상태의 release/stop 재진입·늦은 문서 변경 검사를 포함했다.
새 학습 변경이 그 완료 증거를 삭제하지 않는다. 관련 코드가 바뀌지 않으면 같은 전체 검사를 반복하지 않는다.

remote CI를 실제 실행했다고 주장하지 않는다. 현재 수치는 로컬 결과다.
독립 검토는 현재 선택 모델을 상속한 에이전트로 수행했으며 여러 모델의 합의라고 표현하지 않는다.

## 9. 실기 실행 규칙

1. root 한 명만 DAW, MCP owner, CUA, REA와 설치를 조작한다. 다른 작업자는 소스·offline 검사만 맡는다.
2. 실제 DAW 입력은 직렬화한다. 여러 owner를 시작하거나 active release binary를 덮어쓰지 않는다.
3. owner를 먼저 시작하고 앱·테스트 곡을 연다. 앱을 먼저 닫고 owner를 마지막에 종료한다.
4. fresh session/document/instance/class/source, stopped transport, 정확한 fixture를 확인한다.
5. parameter setter와 계획된 state/preset mutation 전에 Main 채널 하나의 numeric volume0과 **output 키가 존재하며 null**임을 확인한다.
6. bool0, missing output, 다른 routed output, 모호한 Main, stale context는 허용하지 않는다.
7. 일반 Music fixture는 유일 선택·빈 event iterator·mute1/arm0/monitor0을 요구한다.
8. Chorder 전용 diagnostic은 전역적으로 빈 한 Music 트랙이며 기존 실험상 mute0/arm0/monitor0이다. Main0을 별도로 확인한다.
9. Main0은 물리 무음이나 원자적 보장이 아니다. CV·하드웨어 독립 출력의 격리 증거를 대신하지 않는다.
10. unknown mutation·read error·unexpected peer/schema/instance drift 이후 setter, load, retry, 자동 repair를 이어서 보내지 않는다.
11. 같은 read-only refusal을 반복하지 말고 schema와 실제 owning source를 확인한다.
12. 실패 시 복구는 마지막 상태를 추측해 덮어쓰는 것이 아니라 소유한 unsaved scratch의 폐기 경로로 제한한다.
13. 파일 저장은 실험에서 명시한 소유 fixture의 Save 단계에만 한다. 일반 반복 검증은 저장하지 않고 닫는다.
14. raw receipt, token, UID, 사용자 곡, vendor resource, research dump는 `.local` 밖 공개 Git에 넣지 않는다.
15. 기존 verifier snapshot은 immutable이다. protocol 변경은 새 사본·새 owner namespace·별도 review에서 한다.

특히 **MCP Validation Scratch에서 Play를 누르지 않는다.** 기존 MCP Keys에 synthetic part 세 개가 있다.
이번 후반 검증에는 transport Play를 사용하지 않았다. 다른 목적의 과거 diagnostic 재생 결과와 구별한다.

CUA는 `mcp__cua_repl`을 사용한다. 새 세션·context 복구 후 문서를 다시 읽는다.
앱을 종료한 뒤 `getApp`으로 상태를 조회하면 다시 실행될 수 있다. 종료 확인에는 `listApps`를 사용한다.
실제 UI 결과를 확인한 뒤 완료 요약을 한 번 제출한다. 이번 종료 과정은 완료했다.
File recent 항목·Save prompt는 fresh AX state에서 파생한다. 오래된 element index를 재사용하지 않는다.
과거 Go To/Open 키보드 경로는 제대로 동작하지 않았으므로 같은 키 입력을 반복하지 않는다.

## 10. 재개 직후 실행 순서

### 10.1 작업 상태 확인

- [ ] `AGENTS.md`, `RESUME.md`, 이 문서와 `RESUME_STATE.json`을 읽는다.
- [ ] `git status`, branch/HEAD/remote를 확인한다. 사용자 변경을 지우지 않는다.
- [ ] 마지막 제품 소스486d611과 문서 이후 변경을 구별한다.
- [ ] App/serve/owner 프로세스가 실제로 종료돼 있는지 확인한다.
- [ ] Native31·bridge·EditTask·release hash를 `unit47-artifact-binding.json`과 대조한다.
- [ ] source가 그대로면 불필요한 rebuild/reinstall/회귀 전수 검사를 하지 않는다.
- [ ] 사용자가 요청한 단위별 체크리스트 보고를 다시 시작한다.

### 10.2 새로운 owner와 바인딩

- [ ] owner48 같은 새 namespace를 사용한다. 기존 unit47 파일과 receipt label을 덮어쓰지 않는다.
- [ ] `live_owner_unit47.py`의 protocol과 frozen27을 그대로 재사용할 수 있는지 source binding을 확인한다.
- [ ] 새 driver/binding 파일을 쓸 경우 기존 active 파일을 수정하지 말고 차이를 검토한다.
- [ ] owner 하나만 시작한 뒤 앱을 열고 최근 파일의 **MCP Chorder Processing Diagnostic**을 선택한다.
- [ ] fresh host·독립 plugin inspect로 현재 instance를 얻는다. 오래된 document token·instance·capsule을 재사용하지 않는다.
- [ ] `include_context:true` inspect에는 explicit source와 **expected_instance**가 필요하다.
- [ ] `plugin_editor_open` DTO에는 expected_instance 필드가 없다. target/plugin과 공통 expected_document를 사용한다.
- [ ] plugin parameter/learning 도구에는 그 도구의 expected_instance와 expected_document를 넣는다.

owner47은 이미 종료됐다. PTY 번호나 이전 MCP client 객체를 살리려 하지 않는다.
frozen27 manifest는29개 source를 바인딩하며 기존 driver가 해시를 검증한다.
새 schema를 쓰는 raw learning 요청은 제품 tool 경계를 통과하고, 실제 setter 직전의 Root fixture/Main 검사도 필요하다.
기존 owner의 generic state suite를 새 learning protocol인 것처럼 사용하지 않는다.

### 10.3 첫 실제 쓰기 단위는 Mode-off pitch

- [ ] 전역 한 트랙·event0·선택·stopped·Main0/null output·arm0/monitor0을 fresh 확인한다.
- [ ] 전체908개를 두 번 읽고 저장 원본과 exact typed/schema/value/normalized/display projection을 비교한다.
- [ ] 이미 열린 editor에 getter를 호출해 complete discovery와 Mode0, pitch-1을 다시 확인한다.
- [ ] 같은 instance에서 fresh reverse binding이 가능함을 확인한다.
- [ ] 첫 setter는 `change:{kind:"input_pitch",pitch:0}`만 쓴다.
- [ ] GUI current-value와 전체908개 changed pair를 읽는다. 원래 chord bytes·할당·다른 역할이 예상 밖으로 바뀌면 멈춘다.
- [ ] DSP `chorder.learnInputPitch`의 offset/binding을 GUI pitch와 동일하다고 미리 가정하지 않는다.
- [ ] positive changed proof 후 새 Main/fixture guard를 확인한다.
- [ ] `change:{kind:"input_pitch",pitch:null}`로 같은 GUI 원본에 한 번 복원한다.
- [ ] getter·전체908개 원본 pair와 최종 체크포인트까지 통과해야 한 단위로 수락한다.
- [ ] 필요할 때만 같은 프로토콜로 pitch127 등 추가 경계를 별도 단위로 검증한다. 첫 결과를 모든 pitch로 확대하지 않는다.

### 10.4 Mode는 실제 양쪽 상태 측정이 먼저

- [ ] Mode-off의 complete getter 결과와 복원 가능한 새 바인딩을 만든다.
- [ ] 소유한 empty diagnostic에서 정상 UI를 이용한 Mode-on 관찰을 계획한다. 그 전에도 Main·fixture guard를 fresh 확인한다.
- [ ] Mode-on 전체 getter256 탐색 수와 schema, stable ancestor binding, null/selected pitch 상태를 관찰한다.
- [ ] UI 원본 복귀와 새 Mode-off 바인딩·full908 복원을 확인한다.
- [ ] Mode-on이128뷰 이하인지 실제 측정한 뒤에만 현재 setter128이 역방향을 찾는다고 판단한다.
- [ ] 128을 넘으면 setter를 호출하지 않는다. 측정 근거에 따라 고정 production bound 변경·관련 검사·새 build를 검토한다.
- [ ] 호출자 임의 budget이나 보지 않은 branch 생략으로 우회하지 않는다.
- [ ] 양방향 바인딩이 확인되면 한 번의 Mode on/off MCP pilot과 full changed/restored checks를 수행한다.

Mode·input pitch는 storable가 아닌 user 학습 제어로 관찰됐다.
Save/reopen 검사는 transient reset과 기존 learned chord bytes 보존을 **특성화**한다.
학습 Mode가 재시작 후 꺼지는 것을 Auto Fill과 같은 지속성 실패로 잘못 분류하지 않는다.

## 11. 남은 작업 전체 목록

### 11.1 Chorder 모델·학습·명령

- [ ] 위 pitch·Mode 실제 왕복.
- [ ] 입력 selector와 해당 core pitch 객체의 canonical binding.
- [ ] per-pitch chordAssigned/learnChord, held/audition 상태와 정상 이벤트 lifecycle.
- [ ] learn/copy/paste/clear/clearAll/playLearnChord 등 명령의 별도 결과·역방향 상태 증명.
- [ ] packed keyboard/chord data와 model content의 읽기·편집·원본 복원.
- [ ] sample event를 사용하는 검증은 새 소유 fixture·결과 정의·cleanup을 먼저 만든다.
- [ ] Auto Fill legacy facade 저장 실패는 별도로 유지하고 일반 API 지원표에 정확히 표기한다.

과거 잔여 분류의 Chorder/Input Filter391개는256 packed keyboard/model 영역과135 학습·명령·event 역할을 포함한다.
새 두 도구의 소스 완료만으로391개가 검증됐다고 처리하지 않는다. exact-name 합집합 감사에서 갱신한다.

### 11.2 추가 프리셋44종

기존 계획의46종 중 Mixtool·Binaural Pan은 각 두 recall/restore를 완료했다.
남은44종은 다음과 같다.

| 묶음 | 클래스 |
|---|---|
| 채널·다이내믹 | Channel Strip, Dual Pan, Limiter, Expander, Gate, Tricomp, Multiband Dynamics, Compressor, De-Esser |
| 왜곡·변조 | RedlightDist, Bitcrusher, Phaser, Flanger, Chorus, X-Trem, Autofilter, Rotor |
| delay·reverb | Beat Delay, Groove Delay, Surround Delay, Analog Delay, Room Reverb, Mixverb, Open AIR, Studio Verb |
| meter·generator | Spectrum Meter, Level Meter, Scope, Tone Generator, Tuner, VU Meter |
| amp·보컬 | Ampire, Pedalboard, Rumble Native, Voice FX, Vocoder, Vocal Tune |
| 악기·Note FX | Multi Instrument, SampleOne, Impact, CV Instrument, Mojito, Chorder, Repeater |

각 클래스는 최대 두 **새로 발급된 recallable index**를 사용한다. 과거 hint를 capability처럼 재생하지 않는다.
기존87 최대 attempt 계획에서 완료한 네 attempt를 뺀 잔여 상한은83이다. 이는 예상 성공 수가 아니다.
singleton·default-only·동일 값 preset은 contrast가 없을 수 있다. 성공을 만들려고 원본을 임의로 바꾸지 않는다.
두 개가 무변화면 추가 preset을 끝없이 순회하지 않고 미검증 사유를 남긴다.

적용 전 complete contiguous catalog, exact instance, same-instance capsule, 빈 fixture와 Main guard가 필요하다.
known recall 뒤 full changed pair에서 authoritative writable 변화가 있어야 positive로 센다.
schema만 변하거나 readonly만 변한 것은 positive가 아니다.
restore 뒤 exact 원본 pair·token release/refusal·final checkpoint를 확인한다.
상태가 불명확하면 load/retry를 금지한다.

CV Instrument preset은 Main0만으로 독립 CV output을 격리할 수 없다.
44개 template가 존재한다는 이유로 CV·routing gate를 건너뛰어 순차 자동 실행하지 않는다.
Music 선택 guard, model schema와 readonly 정책도 각 fixture에서 확인한다.
과거 empty7종과 positive6종을 잔여44종에 다시 넣지 않는다.

### 11.3 Vocal Tune host bypass

13개 연동 역할 완료 뒤 남은 별도 역할이다. 기존 미완료 scalar tail의 bypass event는 primary proof가 아니다.
direct DSP parameter와 slot component alias의 canonical equality, owning callback, capsule의 bypass 복원을 확정하지 않았다.

- [ ] fresh full26 pair·inspect·`plugin_parameter_details` bypass/include_owner pair·full26 pair.
- [ ] 정확한 concrete parameter/observer callback의 1/0 상태 경로를 조사한다. 필요하면 정확한 image에 Ghidra를 사용한다.
- [ ] 별도로 검토한 direct-only exactFloat64 `0→1→0` 두 setter pilot을 만든다.
- [ ] capsule로 host bypass를 복원한다고 가정하지 않는다.
- [ ] changed full pair·원본 full pair·최종 독립 pair가 모두 통과하면 현재값 한 이름만 수락한다.
- [ ] 오디오 bypass·latency·저장·다른 클래스의 bypass로 결과를 확대하지 않는다.

준비한 offline 계획과 template는 execution_ready false다. adapter나 owner dispatch 코드는 아직 없다.

### 11.4 나머지 악기·Amp·매크로·전용 값

| 영역 | 다음에 확인할 내용 |
|---|---|
| Mojito | 전용 MIDI carrier3개와 event lifecycle |
| Mai Tai | MIDI6개, GUI edit-view binding, 조건부 character/model 동작 |
| SampleOne | MIDI6개, sampler의 조건부 load/model 상태 |
| Presence | MIDI6개와 script/program에 묶인 macro16개 |
| Multi Instrument | macro20개의 실제 target map, native control facade130개의 assignment·child identity |
| Ampire | alias12·linked mix3·routing1·tuner GUI2·host bypass1의 별도 역할 |
| Pedalboard | alias11 및 host bypass callback |
| Mustang/Rumble | model facade·amp position/stomp graph·routing·tuner GUI·host bypass |
| Dynamics/Meter | Compressor/Limiter resetmin, Scope Oneshot/Retrigger, Level Meter resethold/resetloudness |

이 표의 분류는 당시 exact-name 계획의 역할 수다. 모델 선택자와 완료 ordinary 결과를 다시 세지 않는다.
Scope/Level Meter의 action/capture를 일반 numeric scalar로 제안하는 기존 classifier 문제가 남아 있다.
이를 일반 writable sweep으로 보내지 말고 closed action/lifecycle로 다룬다.
이미 완료한 세 filter swap action은 persistent scalar 수와 별도다.

Mustang/Rumble의 정상 GUI model 목록은 실제로 비어 있었다.
50개 effect slot은0..0 sentinel, 두 ampmodel facade는 빈 표시를 보였다.
빈 catalog를 근거로 유효 선택값을 만들어 쓰지 않는다. 실제 model domain/workspace route가 먼저다.
라이선스·설정·private initialization을 바꿔 목록을 강제로 채우는 작업은 하지 않았다.
Macro의0 값과 숫자 범위는 target이 없다는 증거가 아니므로 assignment를 먼저 읽는다.

### 11.5 CV·External 하드웨어

CV Instrument97개는 다음 문제를 분리해야 한다.

- config86개는 CV Gate/Pitch·modulation 네 포트의 연결·격리 증거가 먼저다.
- calibration2, monitor2는 실제 하드웨어 lifecycle이 필요하다.
- MIDI carrier3과 generator selector4는 각각 이벤트·component state binding이 필요하다.

Main fader0으로 CV output이 차단된다고 가정하지 않는다.
External Mono/Stereo에는 derived latencyTotal2, unit/UI 문자열4, physical ping2가 별도 역할로 남아 있다.
opaque image record의 metadata hash를 image 내용 검증이라고 부르지 않는다.
실제 장치가 없으면 제품 capability·제한을 명확히 남기고 hardware 검증을 완료로 표시하지 않는다.
물리 검증 부재 때문에 나머지 독립 소스·오프라인 작업까지 정지할 필요는 없다.

### 11.6 일반 중첩 그래프

- [ ] 정확한 graph/node/slot/Device identity와 소유권 모델을 만든다.
- [ ] 새 node/slot 생성, 연결·분기·채널 매핑, 순서 변경·삭제를 별도 closed operation으로 검토한다.
- [ ] 고정된 작은 소유 그래프에서 topology 전체와 plugin state의 before/after/restore를 비교한다.
- [ ] graph 변경 시 기존 handle·token 무효화와 다른 sibling 보존을 확인한다.
- [ ] 실패·partial/unknown 뒤 blind folder load를 하지 않도록 복구 규칙을 만든다.
- [ ] 예전 whole-folder Pro EQ 표시가 `default*→default`로 바뀐 결과를 strict display 복원 성공으로 바꾸지 않는다.

기존42개 Splitter 값과 whole-Inserts 왕복은 재사용할 증거이며 일반 graph API를 대신하지 않는다.

### 11.7 범위 감사·문서·릴리스

- [ ] native class UID/source/canonical name 기준으로 최신 distinct-name 합집합을 재계산한다.
- [ ] Focus33·MIDI2·Vocal13의 새 이름을 기존 Focus7 감사에 disjoint하게 연결한다.
- [ ] 실패·중복·sibling·같은 model choice의 반복 결과를 제외한다.
- [ ] readonly·불투명·unavailable·조건부 영역을 분리한다.
- [ ] 지원표에 read/write/current readback/state restore/save/reopen/event/audio 수준을 구분한다.
- [ ] README, PLUGIN_STATE_VALIDATION, IMPLEMENTATION_PLAN과 PR 설명을 최종 구현으로 맞춘다.
- [ ] 필요한 CI 실제 실행을 확인한다. 정의만 있는 target을 검증된 platform으로 표현하지 않는다.
- [ ] 지원 범위가 명확한 release와 설치·연결·unknown outcome 복구 예제를 준비한다.
- [ ] 미완료가 남은 동안 PR #4는 Draft로 둔다. 자동 merge를 시작하지 않는다.

### 11.8 최초 P0–P5 장기 계획 보존

현재 우선순위는 사용자가 다시 지정한 내장 플러그인·모델·프리셋·전체 파라미터다.
그 밖의 최초 계획도 삭제하지 않는다. 아래 항목은 계획과 실제 증거를 재대조한 뒤 확장해야 한다.

- no-document/wakeup·재연결·대기 중 문서 전환·중복 이름·부분 mailbox·lock 경쟁의 전체 edge scenario.
- 대규모 fixture, queue cancellation/deadline, archive 제한과 손상 입력, capability/지원표 정합성.
- transport·tempo·loop·metronome·mixer·send·bus/output routing의 폭넓은 조합과 복원.
- MIDI part/event·빈 part 첫 note·batch 편집·duplicate handling·경계·단위·cancel/partial 결과.
- plugin insert/reorder/delete와 stale handle, third-party AU/VST3의 실제 노출 한계.
- automation mode/기존 곡선 읽기, direct point write 또는 제한된 recording 경로의 실제 지원 여부.
- mix/plugin snapshot 포함·제외 필드, 캡처·녹음·bounce/export의 job start/status/cancel/artifact.
- Core Audio process tap의 권한·실시간 callback 제약·현재 상태 복원·buffer drop/메모리 측정.
- mixdown/stems 대화상자 또는 command 인자와 실제 출력 파일 format/길이/channel 검사.
- WAV loudness·peak·clipping·band/stereo 분석의 기준 신호·독립 비교 및 실제 분석 지원 범위 정리.
- 최종 설치/제거/업데이트·복구·platform packaging·사용법·지원 범위에 맞는 배포.

이미 구현·검증한 부분은 [VALIDATION.md](VALIDATION.md)와 현재 도구에서 먼저 찾는다.
계획의 성능 숫자는 제안 기준인 곳이 많다. 측정값·속도 향상·모든 lane 통과로 인용하지 않는다.

## 12. 로컬 증거·계획 파일 지도

아래 경로는 저장소 루트 기준이다. `.local`은 의도적으로 Git에서 제외한다.
public 문서에는 요약과 증거 hash만 남긴다. raw receipt·발급 token·곡·vendor resource는 공개하지 않는다.
이번 주요 비공개 증거18개의 hash는 [정제한 인수인계 manifest](evidence/2026-11-10-loop-handoff.json)에 버전 관리한다.
같은 Mac에서 재개할 때 이 폴더를 보존한다. 다른 Mac으로 Git만 clone하면 `.local` 증거와 설치 상태는 따라오지 않는다.
로컬 파일이 없으면 파일이 있는 것처럼 명령을 실행하지 말고 공개 요약과 새 preflight로 가능한 부분을 이어간다.

| 목적 | 경로와 사용법 |
|---|---|
| 최신 설치·제품 바인딩 | `.local/next-state/unit47-artifact-binding.json` |
| Native31 보관본 | `.local/next-state/unit47-artifacts/`의 binary/package/dSYM/native source manifest |
| 마지막 owner source | `.local/next-state/live_owner_unit47.py`, `unit47-driver-source-binding.json` |
| 마지막 실제 getter·full908 | `.local/next-state/unit47a-readonly-stop-result.json`, `unit47a-learning-get-off.json`, `unit47a-stop-readonly-proof.json` |
| 종료·저장 파일 보존 | `.local/next-state/unit47-final-pause-state.json`, `unit46-final-scratch-file-proof.json` |
| full908 helper | `.local/next-state/gui47_helpers.py`, prefix와 fresh host/instance 파일을 새 namespace에서 준비해야 함 |
| 설치 이전 Native30 바인딩 | `.local/next-state/unit46-artifact-binding.json`, `unit46-artifacts/` |
| Impact 직접4227 승인 | `.local/next-state/unit21-impact-accepted4227.json` |
| Focus33 승인 체인 | `.local/next-state/unit36-focus-accepted33.json`과 참조한 이전 increment manifest |
| Native30 잔여20 결과 | `.local/next-state/unit46-focus-complete33-result.json`, `.local/live/unit46-focus-01`부터 `20` |
| Impact4262 정확 합집합 | `.local/next-state/unit46-impact-current-provider-complete4262.json` |
| Focus 유한 실행 도구 | `.local/next-state/focus46_driver.js`, `focus46_case_status.py`, `accept_focus_unit.py`, `unit46-focus-remaining20-plan.json` |
| Impact MIDI현재값2 | `.local/next-state/unit40m-actual-result.json` |
| ordinary 악기755 | `.local/next-state/unit32-instruments-accepted755.json` |
| 추가 ordinary51 | `.local/next-state/unit38-extra-accepted51.json` |
| Vocal13 전체결과 | `.local/next-state/unit46v-all13-result.json` |
| Vocal pilot·remainder 원장 | `.local/live/unit46v-pilot-c/ledger.jsonl`, `.local/live/unit46v-remaining12/ledger.jsonl` |
| Vocal 고정 protocol·reference | `.local/next-state/vocal-tune-coordinated-plan/vocal_protocol.py`, 그 안의 REFERENCE/SHA |
| Vocal 최종 원본 proof | `.local/next-state/vocal-tune-coordinated-plan/unit46v-final-original-proof.json` |
| Vocal fresh inspect checker | `.local/next-state/vocal-tune-coordinated-plan/check_preflight.py`의 필수 `--inspect` |
| Vocal checker 독립검사 | `.local/next-state/unit46v-preflight-check-review/{REPORT.md,evidence.json,test.log}` |
| Vocal protocol/owner review | `.local/next-state/vocal-coordinated-review/`, `unit46-driver-review/` |
| Vocal bypass 다음계획 | `.local/next-state/vocal-bypass-plan/{PLAN.md,readonly-template.json,evidence.json}` |
| 프리셋 최초46종 계획 | `.local/next-state/remaining-preset-recalls/{plan.md,classification.json,controls/}` |
| 새 완료 프리셋 두종 | `.local/next-state/unit46p-mixtool-result.json`, `unit46p-binaural-result.json` |
| 해당 실제 원장 | `.local/live/unit46p-mixtool/ledger.jsonl`, `.local/live/unit46p-binaural/ledger.jsonl` |
| frozen verifier | `.local/next-state/unit27-verifier-snapshot/manifest.json`, 변경 금지 |
| 과거 전역 이름합집합 감사 | `.local/next-state/progress-union-review/{REPORT.md,check.py,check-result.json}` |
| 잔여 역할 exact-name 분류 | `.local/next-state/remaining-work3072/{REPORT.md,plan.json,check.py}`, 옛 숫자와 최신 완료를 대조 |
| Auto Fill 저장 왕복 최종 | `.local/next-state/unit44a-gui-persistent-roundtrip-result.json` |
| Native30 controller probe | `.local/next-state/unit46a-controller-probe-result.json`, `unit46a-afterprobe-proof.json` |
| 학습 제어 설계 판정 | `.local/next-state/chorder-gui-model-edit-design/{judgment.md,judgment-evidence.json,synthesis-recommendation.md}` |
| native 학습 builder | `.local/next-state/chorder-gui-model-edit-native/{REPORT.md,evidence.json,readiness.json,source-freeze.json}` |
| wire 학습 builder | `.local/next-state/chorder-gui-model-edit-wire/{REPORT.md,evidence.json}` |
| 독립 학습 검토 | `.local/next-state/chorder-learning-independent-review/{REPORT.md,evidence.json,reviewed-source-sha256.json}` |
| GC 경로 static proof | `.local/next-state/unit45-jsengine-binary-binding.json`, `unit45-nested-gc-path-result.json` |
| GC 독립 review | `.local/next-state/gc-crash-recurrence-audit/`, `gc-heap-busy-audit/` |
| Native 문서 context review | `.local/next-state/document-context-native-review/`, `document-context-wire-review/` |
| 실제 새 context·EditTask | `.local/next-state/unit46-native-context-first-proof.json`, `unit46-edittask-native-context-proof.json` |
| Ghidra 보존 결과 | `.local/next-state/jsengine-unit45-evidence.json`, `main-gc-unit45-evidence.json` 및 각 snapshot |
| Ampire readonly 진단 | `.local/next-state/ampire-meter-failure-review/` |

기존 control JSON이 `execution_ready:false` envelope면 그대로 owner에 보내지 않는다.
새 Root control 파일로 검토한 control 내용만 추출하고 output namespace를 새로 만든다.
unit46/47의 document/session/instance/token이 들어간 raw batch를 재개용 명령으로 재생하지 않는다.
hash가 같더라도 runtime capability는 앱·service 종료로 만료됐다.

## 13. 제품 코드와 향후 수정 지점

| 목적 | 주요 source |
|---|---|
| MCP 도구와 schema | `src/server.rs`, `src/operation.rs` |
| 요청·결과·검증 | `src/runtime.rs`, `src/runtime/chorder_learning.rs` |
| bridge dispatch/context | `assets/bridge/BridgeCore.js`, `BridgeComponent.js` |
| native request/receipt 경계 | `assets/bridge/NativePluginControls.js`, `native/addon/src/request.rs` |
| Chorder native learning | `native/addon/src/request/gui_model.rs` |
| GUI discovery·projection·pin | `native/addon/src/request/gui_parameter.rs`, `gui_parameter/controller_probe.rs` |
| numeric edit·ExactInteger | `native/parameter-api/src/numeric_edit.rs`, `numeric_edit_tests.rs`, `parameter.rs` |
| shared document lease | `assets/edit-tasks/package/McpDocumentContext.js` 및 bridge/task 포함 경로 |
| 설치·패키징 | `native/build.sh`, 서버 install/native-install 구현 |
| 고정 경계 JS 검사 | `tests/js/chorder-learning.test.mjs`와 기존 GUI/native groups |
| live verifier source | `scripts/verify_all_parameters.py`, `verify_plugin_states.py`, `verify_plugin_presets.py` 등 |

생산 코드와 `.local` frozen verifier는 별도다. 제품 파일을 수정했다고 이전 active runner가 새 동작을 사용하는 것이 아니다.
새 실기 lane은 정확한 binary·installed assets·driver·verifier manifest를 함께 바인딩한다.

## 14. 주요 Git 변경 이력

| 커밋 | 내용 |
|---|---|
| `c90208d` | Chorder GUI parameter receiver 비교 |
| `e687645` | bounded GUI walk failure 보고 |
| `7ccb35f` | 악기·Focus 완료 증거 정리 |
| `a777332` | C++ Variant padding 보존 수정 |
| `37884b0` | 고정 깊이로 Chorder tree 검사 |
| `b35a821` | 실제 GUI local parameter 이름 바인딩 |
| `a7503b4` | ordinary instrument scalar 완료 기록 |
| `5d7f258` | 완료 체크포인트 공개 문서화 |
| `e715683` | 실제 GUI Auto Fill receiver 편집 |
| `3cbe509` | Save/restart 양방향 지속성 결과 |
| `50c1900` | nested GC crash 재발 보존 |
| `3e344c5` | native 문서 context로 JS Song proxy 경로 회피 |
| `329f747` | Chorder GUI model receiver 진단 |
| `1de28ad` | Impact33/4262·Vocal13 완료 기록 |
| `486d611` | guarded Chorder learning get/set 구현 |

이 인수인계와 최종 종료 결과는 별도 문서 커밋으로 추가한다.
위 커밋을 amend하거나 실패 증거를 삭제하지 않는다.
PR은 생성돼 있지만 전체 지원이 끝난 상태가 아니므로 Draft로 유지한다.

## 15. 왜 오래 걸렸으며 어떻게 반복을 제한할 것인가

완료 전수 검사를 이유 없이 계속 재시작한 것은 아니다.
수천 개의 파라미터마다 변경·전체 비교·원본 복원을 수행했고,
두 번의 앱 종료 조사, GUI/DSP receiver 분리, 저장·재시작 검증,
새 수명 관리와 경계 검토가 추가됐다. 이 작업들은 source와 실제 host 결과를 구분해야 해서 단위가 많았다.
그러나 남은 모든 모델·하드웨어·이벤트의 종료 시간을 이 숫자만으로 추정할 수는 없다.

재개 후에는 다음 규칙으로 진행을 분명히 드러낸다.

- 한 단위의 입력 이름·목표·최종 완료 조건을 시작 전에 고정한다.
- 완료 단위의 source/hash/evidence를 보존하고 다음 미완료 단위로 넘어간다.
- 실패 단위는0개로 남기고 원인·수정·새 실험을 분리한다.
- 새 코드·실패·변경 영향이 없으면 동일한 광범위 테스트를 반복하지 않는다.
- 새 증거 없이 설계·review·counterfactual round를 늘리지 않는다.
- 끝난 단위마다 `[x] 완료 / [ ] 다음·잔여` 체크리스트와 실제 분모를 알린다.
- 하드웨어 등 외부 조건이 없으면 exact limitation을 남기고 독립적으로 가능한 다음 일을 한다.
- 전체 지원률·완료 ETA는 유효한 분모와 근거 없이 만들지 않는다.

## 16. 중단 완료 체크리스트

- [x] 사용자의 중단 요청 후 새 쓰기·프리셋·모델 단위를 시작하지 않았다.
- [x] 이미 진행 중이던 Native31 설치 후 readonly 단위만 마쳤다.
- [x] Chorder getter와 전체908개 원본 pair를 검증했다.
- [x] learning setter0회·transport0회를 확인했다.
- [x] 소유 diagnostic을 저장하지 않고 닫았다.
- [x] 실제 앱 완전 종료를 확인한 뒤 owner47을 종료했다.
- [x] owner46도 앞서 정상 종료했다.
- [x] 두 테스트 곡 저장 파일이 이전과 같음을 확인했다.
- [x] 추가 offline 작업자의 계획을 동결하고 종료했다.
- [x] 자동 재개·예약 실행을 만들지 않았다.
- [x] 재개 진입점·상태 파일·상세 다음 작업을 작성했다.
- [ ] 재개 후 fresh runtime guard와 첫 pitch pilot부터 진행한다.

GitHub push와 최종 repository 상태는 이번 종료 응답 직전에 확인한다.
그 뒤 사용자의 재개 지시가 올 때까지 기능 작업을 진행하지 않는다.

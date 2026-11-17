# 2026-11-17 재개 루프 인수인계

이 문서는 11월 17일 재개 이후의 구현, 실제 검증, 실패, 조사 결과와 미완료 대기열을 보존한다. 사용자가 오늘 오후 6시 종료를 지정했으며, 새 실제 단위는 5시 40분부터 시작하지 않고 앱과 분석 엔진을 정리한다. 최종 중단 시각은 2026-11-17T17:47:20+09:00이며 `RESUME_STATE.json`을 우선한다. R81 독립 수용과 마지막 정적 단위를 완료했으며 최종 정리 기록은 아래에 남긴다.

## 1. 한 문장으로 재개

사용자가 **진행 재개해**라고 하면 `AGENTS.md`, `RESUME.md`, `RESUME_STATE.json`, 이 문서와 [11월 17일 전체 진행 기록](PROGRESS_2026-11-17.md)을 읽는다. 이전 [11월 15일 인수인계](LOOP_HANDOFF_2026-11-15.md)는 이미 완료한 이전 근거와 정책을 위한 역사 문서다. 11월 10일 시작 단위나 완료한 대규모 검사를 다시 실행하지 않는다.

Git과 현재 설치, 프로세스, 두 잠금, 저장본과 OS 장치 상태를 새로 확인한다. 문서만 바뀌면 같은 제품의 빌드와 설치를 반복하지 않는다. 새 실제 단위에는 새 앱 문서와 bridge session, instance, owner와 원본 요청 기록을 만든다. 이전 세션과 문서 토큰, UI 번호, 프로세스 ID는 재사용하지 않는다. 닫힌 세션 56개에는 R81을 포함해야 한다. 중단 후 자동 재개나 예약 실행은 없다.

재개 후 첫 기술 작업은 CV의 구조형 선택지 준비·정리 계약과 원래 상태 보존 설계다. Native59에서 실제 지원 여부를 이미 확인했으므로 같은 존재 조회를 다시 수행하는 것을 첫 단위로 삼지 않는다. 마지막 정적 결과를 먼저 읽고, 그 결과가 닫아 준 범위 안에서 다음 유한 단위를 계획한다.

## 2. 사용자 목표와 유지할 결정

macOS 우선의 native Rust MCP로 실제 Fender Studio Pro 8.1.1 Build 113103의 bridge, EditTask, 내장 플러그인, 모델, 프리셋과 중첩 그래프를 제어한다. 대표 파라미터에 머무르지 않고 쓰기 가능한 전체 파라미터의 변경·읽기·원복을 검증하는 목표다. 내장 JavaScript는 DAW 내부에서만 실행한다.

공개 저장소는 [dev-universe/fender-studio-pro-mcp](https://github.com/dev-universe/fender-studio-pro-mcp), branch는 `feature/plugin-state-and-exhaustive-validation`, [PR 4](https://github.com/dev-universe/fender-studio-pro-mcp/pull/4)는 Draft다. 이 실행에서 merge나 release를 완료했다고 주장하지 않는다. branch와 PR 제목에 codex를 쓰지 않는다.

공식 8.1.1 설치 파일 비교는 사용자가 생략했다. 경로나 MyFender 로그인을 다시 묻거나 기다리지 않는다. 모델 모듈 서명 문제를 해결했다고 쓰지 않는다. Hopper는 사용하지 않는다. REA 6.9.0과 Ghidra 12.1.4 및 검증한 기존 Node/JDK를 사용했다. 설치된 도구를 매 단위마다 다시 다운로드하거나 등록하지 않는다.

Volt 276과 Keystation88 MK3는 사용자 지시상 분리 상태다. 과거에는 USB만 연결했고 loopback 배선은 없었다. 최신 OS inventory에 다른 무선 오디오 장치가 보인 사실을 이 두 장치의 재연결이나 물리 CV 검증으로 해석하지 않는다. 실제 CV, External mono/stereo와 건반의 네 완료물은 재연결과 필요한 배선·물리 관찰이 필요하다.

사용자 보고는 한국어, 내부 작업은 한국어 또는 영어만 쓴다. 단위 완료마다 체크리스트, 실제 경과, 남은 작업 수, 다음 유한 묶음 예상과 오늘 루프 종료까지 시간을 보고한다. 전체 연구 완료 시간을 모를 때는 미정이라고 표시한다. 폐기한 6~20시간 추정을 반복하지 않는다. 고정 43개의 하위 요청 수와 노력은 같지 않다.

## 3. 현재 제품과 검증 기준

| 항목 | 현재 기준 |
|---|---|
| 제품 소스 | `7f0cd1da13b18a0097344d1ad7a84c550d764b77` |
| 설치 | Native59 정상 설치 |
| native UUID | `684497D3-384F-3DD3-B4D2-1C788AF35F33` |
| native SHA256 | `67c5b25ccf3955a9a0b9fbd99f00e0b17a0064a6498325661f3bcd2bf29466d7` |
| Rust CLI SHA256 | `16cb6ff2ee1708df932d060474df8fa81bd94794f90527faeced2d0b23540342` |
| 소스·설치 대조 | 소스181, 설치13, source pairs12, native package4 |
| 공개 입력 스키마 | MCP46도구, 고정 버튼5동작, 상태7동작 |
| 새 관련 테스트 | addon448, parameter-api142 unit와4 doc-test, 모두 exit0 |
| 재사용한 변경 없는 검증 | main179, 선택 JS97; 새로 실행했다고 주장하지 않음 |
| 고정 완료물 | 43개 중15완료,28잔여: 부분23·미착수1·하드웨어4 |
| 프리셋 설정 | 40/46종,77수용사례,6종 잔여 |
| 현재값 이름 | 6866/7112,246개·18클래스 잔여 |

프리셋 종류, 이름, 고정 완료물을 서로 환산하지 않는다. 현재 CV host 포트 7행을 읽었다고 쓰기 가능한 이름 7개를 새로 수용하지 않는다. 전체 내장 플러그인 완료와 전체 저장·오디오·하드웨어 검증은 아직 아니다.

## 4. 이번 실행의 실제 단위

| 단위 | 실제 요청 | 결과와 제한 |
|---|---:|---|
| R73 |40|CV DSP110행과 쓰기 가능한97개, capsule A3900을 관찰. 프리셋 적용과 출력 할당 복원은 없음 |
| R74 |47|Glide 한 번 변경 뒤 A 한 번 복원, 전체97 독립 읽기와 UI 원복. factory preset 또는 host route 보존이 아님 |
| R75 |27|일반 host node 읽기. 부분 metadata를 출력 할당 의미로 수용하지 않음 |
| R76 / Native56 |27|PortSetup와7자식의 제한된 타입·공개 인터페이스 관찰 수용. 당시 두 단계의 Main UI 주장은 editor 원본뿐이어서 미검증으로 보존 |
| R77 / Native57 |27|직접 controller8개와 indexed List7행, 별도 CV/Main4쌍 수용. 선택지·setter·할당 의미는 미검증 |
| R78 / Native58 |1|첫 host_status 읽기 뒤 Root의 필수 하드웨어 관찰 파일 누락으로 중단. 미수용. 추가 요청 없음 |
| R79 / Native58 |16|16guard 통과 뒤 Root UI 기록의 surface와 직접 관찰 필드 누락으로 다음 intent 이전 중단. 전체 단위 미수용 |
| R80 / Native58 |27|7행의 고정 details49필드와 CV/Main4쌍 독립 수용. 실제14분, 검토 약4분 |
| R81 / Native59 |27|7행 모두 structured_interface=true, details56필드,60ms. 27guard와전체audit·NoSave·owner0·보존 및 독립 수용 완료 |

누적 실제 요청은239개다. R78·R79의17개는 계속 미수용이다. R81의27개까지 독립 수용해 현재222수용·17미수용·수용대기0이다. metadata3write/2원본, OS inventory와 doctor 조회는 DAW 요청 수에 더하지 않는다.

R78·R79의 실패 원본을 고치거나 해당 세션을 이어 쓰지 않는다. 이는 제품의 새 상세 조회 실패가 아니라 Root 준비 기록 오류였다. R79 종료 때 원본 캡처로 AX번호가 바뀐 뒤 옛 번호 클릭이 실제 동작 전에 거절된 사실도 남겼다. R80부터 검증한 typed 입력·CV/Main 기록 생성기를 사용해 수작업 누락을 막았다.

## 5. CV에서 현재 확정한 사실

PortSetup의 첫 타입은 AudioFXPortSetup, 입력 Input00은 AudioFXEndPointComponent, 출력 Output01..06은 AudioOutputNode이며 그 base는 AudioIONode다. 실제 controller는8개이고 PortSetup은 파라미터0개다. 입력 port1개와 출력 portAssignment 각1개가 직접 indexed로 관찰됐다.

모두 List이고 표시는 없음, min0/max-1이다. 입력 raw-1과 normalized bits3F800000, 출력 raw0과 normalized bits80000000인 음의0을 보존한다. read_only=false는 유효한 쓰기 범위나 setter 권한이 아니다.

Native58 실제7행은 precision1, storable·mutable·store_list_value·read_only·list_interface=false였다. Input00 can_undo=true, 출력6개는false였다. Native59 실제7행은 별도 structured_interface=true였다. 일반 IListParameter 부재와 중첩 IStructuredParameter 존재는 모순이 아니다. 구체 클래스 이름은 아직 unknown이다.

R76의 실제8노드에서 IStorable, IPresetCollector, IPresetMediator는 모두 엄격한 observed false였다. 이 원본을 재사용해 같은 node QI의 새 구현·재실기를 기각했다. parameter의 storable flag와 node의 IStorable QI를 구분한다. 다른 내부 serializer나 adapter의 존재까지 부정하지 않는다.

factory Default의 별도 PortSetup.data315byte에는 Output03의 비어 있지 않은 assignment가 있다. DSP capsule3900과 factory primary3991byte는 서로 바꿔 쓸 수 있다고 입증하지 못했다. 현재 DSP 복원만으로 host 출력 경로 보존을 주장하지 않는다. 직접 IPreset.restore는 AudioSynthSlot의 mediator·ambient owner·extra-content eligibility·host bookkeeping 문제 때문에 NOT_READY다. 공개 owner 계약이나 검증한 복원 범위를 먼저 확정해야 한다.

## 6. 구현의 안전 경계

cv_ports는 정확한 CV class, AudioSynth, instrument, source DSP, 현재 문서·instance, include_nodes와 문맥 조건을 요구한다. 숫자 한도는 node9/depth1/ancestor0/children8이며 반사 metadata의 부분성은 그대로 보고한다. controller나 property 호출 권한을 반사 메서드 이름으로 추정하지 않는다.

cv_port_parameters는8노드·노드당32·전체64파라미터·retained text48KiB와 기존 encoded응답256KiB, GUI2초 경계를 유지한다. 내부 callback 하나를 선점하거나 peak memory를 보장하는 한도로 설명하지 않는다.

현재 parameter witness를 첫 관찰 전에 등록한다. 모든 Description과 text한도를 끝낸 뒤 RowsComplete에서만 details와 새 structured QI를 읽는다. index/text한도에서는 추가 QI0회다. 조회와 성공 owner 해제 전후에 전체 inventory guard를 둔다. 실패nonnull·성공null·모호한 status는 오류이며 malformed 실패 포인터를 adopt·dereference·release하지 않는다.

마지막 검사는 모든 ID 복사 다음 borrowed controller count와 indexed pointer, retained Header, attachment, Context 순서를 유지한다. 늦은 최종 실패에서는 Reply 전체를 지운다. 임의 eval, shell, caller IID, unchecked script나 private ABI를 공개 도구에 노출하지 않는다. source·slot·자식 소유권 확인 없이 일반 graph write로 확장하지 않는다.

Native59의 새 helper는 CV complete rows에서만 사용한다. 일반 ParameterDetails 본문과 형태는 그대로다. 구조 준비·정리·자식 열거·메뉴·getController·getOriginal·setter는 이 구현과 R81에서 호출하지 않았다. assignment_values_observed, routing_verified, atomic_snapshot, mutation_capability는 false다.

## 7. 조사 경로와 다음 우선순위

일반 paired-entry100da8ffc와 소유 객체의 이름 없는 긴 chain은 구체 연결 없이 반복하지 않는다. 같은 배열 offset이 다른 함수에서 보인다는 사실만으로 같은 AudioEffect owner라고 주장하지 않는다. 실제 Input00 wrapper와 정적 AudioInputNode의 관계도 미검증이다.

정확한 AudioOutputNode 등록·생성 경로에서 AudioIONode base의 portAssignment/tag100 생성으로 연결했다. 그 생성 implementation의 정확한 QI분기를 공개 SDK의 IStructuredParameter IID와 대조했다. Native59 actual은 그 존재를7/7로 확인했다. 다음은 이 공개 인터페이스의 준비·정리와 유한 자식 관찰 계약이다. 마지막 정적 단위의 보고서가 새 출발점이며 단순 QI 존재 조회나 완료한 함수 본문을 반복하지 않는다.

선언 controller를 관찰하는 별도 persistence 후보도 검토했다. 기존 RawParameter::controller_owner를 고정 Output03에 한정하고 direct node와 다른 canonical에서만 IStorable QI하는 안이다. publication까지 owner를 유지하는 마지막 borrowed association 검사 설계가 선행조건이다. 현재 우선순위는 구조형 경로이며 이 안을 구현 완료나 실제 serializer 발견으로 쓰지 않는다.

Legacy Environment 소유권 seed는 다른 트랙으로 보존한다. 정확한 factory receiver와 cache item ABI, endpoint 방향 의미가 미확인인 상태에서 graph/port setter를 호출하지 않는다. Source에 없는 private C++ signature를 decompiler 표기만으로 확정하지 않는다.

## 8. 빠짐없이 이어갈 28개 완료물

다음 목록은 기존 고정43개 중 미완료28개를 ID 기준으로 보존한다. 원래 완료 조건은 기존 작업표에서 재사용하며, 최근 수용 범위가 더 좁거나 넓으면 본 문서와 상태 JSON의 최신 근거를 우선한다. 각 항목의 전체 완료를 현재값·반사·단순 accepted 응답으로 대신하지 않는다.

### PRESET-AMPIRE — Ampire

상태: 부분 완료.

기존 근거: issued catalog 발견·해제·만료 거부. 이46종 계획의 완료 사례는 아직 없음

남은 완료 조건: 고정 issued index의 실제 설정 변화·독립 안정쌍·전체 원래값/스키마 복원·token 해제/만료·NoSave 정리를 충족하는 사례를 기존 두 사례 기준으로 수용한다. 이미 완료한 사례는 재사용한다.

### PRESET-PEDALBOARD — Pedalboard

상태: 부분 완료.

기존 근거: issued catalog 발견·해제·만료 거부. 이46종 계획의 완료 사례는 아직 없음

남은 완료 조건: 고정 issued index의 실제 설정 변화·독립 안정쌍·전체 원래값/스키마 복원·token 해제/만료·NoSave 정리를 충족하는 사례를 기존 두 사례 기준으로 수용한다. 이미 완료한 사례는 재사용한다.

### PRESET-RUMBLE — Rumble Native

상태: 부분 완료.

기존 근거: 기존 issued111catalog 수명과 Native48새18DAQ/314.470s의3653byte aliascapture/release·전체39current/metadata 안정성·Root정리 확인. 실제presetapply/load/modelidentity는미검증.

남은 완료 조건: 고정 issued index의 실제 설정 변화·독립 안정쌍·전체 원래값/스키마 복원·token 해제/만료·NoSave 정리를 충족하는 사례를 기존 두 사례 기준으로 수용한다. 이미 완료한 사례는 재사용한다.

### PRESET-MULTI — Multi Instrument

상태: 부분 완료.

기존 근거: issued catalog 발견·해제·만료 거부. 이46종 계획의 완료 사례는 아직 없음

남은 완료 조건: 고정 issued index의 실제 설정 변화·독립 안정쌍·전체 원래값/스키마 복원·token 해제/만료·NoSave 정리를 충족하는 사례를 기존 두 사례 기준으로 수용한다. 이미 완료한 사례는 재사용한다.

### PRESET-IMPACT — Impact

상태: 부분 완료.

기존 근거: issued catalog 발견·해제·만료 거부. 이46종 계획의 완료 사례는 아직 없음

남은 완료 조건: 고정 issued index의 실제 설정 변화·독립 안정쌍·전체 원래값/스키마 복원·token 해제/만료·NoSave 정리를 충족하는 사례를 기존 두 사례 기준으로 수용한다. 이미 완료한 사례는 재사용한다.

### PRESET-CV — CV Instrument

상태: 부분 완료.

기존 근거: issued catalog 발견·해제·만료 거부. 이46종 계획의 완료 사례는 아직 없음

남은 완료 조건: 고정 issued index의 실제 설정 변화·독립 안정쌍·전체 원래값/스키마 복원·token 해제/만료·NoSave 정리를 충족하는 사례를 기존 두 사례 기준으로 수용한다. 이미 완료한 사례는 재사용한다.

### CONTROL-AMPIRE — Ampire

상태: 부분 완료.

기존 근거: 일반39개·catalog/owner 읽기

남은 완료 조건: alias·micmix/linked·bypass/tuner·stomp 순서의 남은19역할을 올바른 대상에 변경·읽기·복원하고 연동 peer/구조를 보존한다.

### CONTROL-CV — CV Instrument

상태: 하드웨어 대기.

기존 근거: provider/선택 범위와 역할97개 분류

남은 완료 조건: config86·generator4·MIDI3의 유한 변경/복원, 포트 격리와 calibration2/monitor2의 실제 효과·중지·복원을 구분해 입증한다.

### CONTROL-CHORDER — Chorder

상태: 부분 완료.

기존 근거: input60 출력128 모델·selector·고정Song저장 완료; guiInput1~128의128개 현재값/정확128이름 모두 수용. scalar128 한사례277.847초·131요청·변경2회·전체908/저장모델128 각6묶음/48페이지·자기playingLive/outputPlayingLive만1·다른905/unknown/공통 원본 보존. model_input/expected_outputs None·128모델입력 미연결·전체모델열거·capture/전체 정확복원·Root NoSave정리·저장6해시동일·MIDI0/0 완료. 남은이름action5개. 이벤트·다른입력모델접근·clipboard·미리듣기와 일반MIDI/모델대응·noteoff·audio·지속저장은별도

남은 완료 조건: guiInput128개와 clear/clearAll/copy/paste/playLearnChord5개 역할을 수명에 맞게 처리한다. 고정 현재값 복원과 실제 note 수명을 구분하고, 다른 입력 모델 접근·clipboard/미리듣기 결과를 독립 확인한다.

### CONTROL-EXTERNAL-MONO — External Effect Mono

상태: 하드웨어 대기.

기존 근거: 현재값·mode·opaque 메뉴 읽기 완료

남은 완료 조건: Send/Return 선택의 공개 global filter·포트 매핑·None 원복, 파생 latency/단위표시를 구분하고 Volt loopback에서 ping/왕복지연/오디오를 검증한다.

### CONTROL-EXTERNAL-STEREO — External Effect Stereo

상태: 하드웨어 대기.

기존 근거: 현재값·mode·메뉴·held-target provenance·checkOnly26 완료

남은 완료 조건: 두 포트의 실제 선택·None 원복과 global filter 경계를 검증하고, 파생 latency/단위표시·물리 stereo loopback/ping을 확인한다. checkOnly를 선택 성공으로 세지 않는다.

### CONTROL-INPUTFILTER — Input Filter

상태: 부분 완료.

기존 근거: 일반 필터 설정·provider/catalog; keyboard.pitch1~128.guiInput128개 현재값/128이름 모두 수용·전체388복원과Root정리 완료. 122~128의7사례406.206초·579요청·변경14회·42전체/294페이지·사전Mojito7관계확인·다른385/비값/unknown/공통 원본 보존. 전체 복원·Root NoSave정리·저장6해시동일·MIDI0/0 완료. 앞선114~121 R1은요청0·변경0·수용0 실패로보존. 남은이름0개이나 일반MIDI/음정대응·이벤트수명·noteoff·audio·지속저장은별도

남은 완료 조건: keyboard.pitch1..128.guiInput을 현재값·pitch 대응·press/release 결과로 분리해 확인하고 원본 상태와 종료를 복원한다.

### CONTROL-MAITAI — Mai Tai

상태: 부분 완료.

기존 근거: 일반 선택 설정·preset 계열 완료

남은 완료 조건: MIDI6 facade의 현재값/실제 전달과 character1.editchar의 GUI·조건부 모델 binding/복원을 검증한다.

### CONTROL-MOJITO — Mojito

상태: 부분 완료.

기존 근거: 일반 선택 설정 완료

남은 완료 조건: afterTouch/modWheel/pitchBend3역할의 현재값·수신·neutral 복원/종료를 검증한다. 기존 Mojito 범위 안에서 legacy stored pan의 실제 제어 경로 또는 무효 legacy 필드 근거도 구분한다. 고정7112 이름 분모는 바꾸지 않는다.

### CONTROL-MUSTANG — Mustang Native

상태: 부분 완료.

기존 근거: provider·RemoteRoot/workspace·selector 조회 및 빈 모델 목록 관측; 기존 preset recall 완료

남은 완료 조건: 현재 비어 있는 amp/effect 실제 domain과 선택·stomp/amp 위치·routing·tuner/bypass29역할의 변경/복원을 검증한다. 빈 목록을 전체지원으로 세지 않는다.

### CONTROL-PEDALBOARD — Pedalboard

상태: 부분 완료.

기존 근거: 일반2개·provider/catalog 완료

남은 완료 조건: alias11·host bypass의 대상/연동·실제 변경과 정확한 원복, 중첩 rack 제어를 검증한다.

### CONTROL-PRESENCE — Presence

상태: 부분 완료.

기존 근거: 일반 선택 설정 완료

남은 완료 조건: MIDI6와 script/program macro16개를 올바른 program/target에 연결하고 현재값·실제 효과·원복을 검증한다.

### CONTROL-RUMBLE — Rumble Native

상태: 부분 완료.

기존 근거: 일반3개·workspace/selector·빈 모델 목록 조회

남은 완료 조건: 실제 amp/effect domain과 선택·amp 위치/stomp routing·tuner/bypass29역할을 검증한다.

### CONTROL-SAMPLEONE — SampleOne

상태: 부분 완료.

기존 근거: 일반 선택 설정 완료

남은 완료 조건: MIDI6의 현재값·수명과 sampler load/model의 실제 상태·복원을 확인한다. 단순 preset ack와 구분한다.

### CONTROL-SCOPE — Scope

상태: 부분 완료.

기존 근거: 일반 설정·singleton catalog 완료

남은 완료 조건: trigOneshot의 armed/capture와 trigRetrig의 retrigger 효과·완료·원래 상태를 별도 검증한다.

### CONTROL-MULTI — Multi Instrument

상태: 부분 완료.

기존 근거: 현재값150개·8실기/3752요청·이름150 수용 및 controller34/36 제한 읽기 완료

남은 완료 조건: macro20 target map·controller130 facade assignment/전달·child identity·일반 중첩 그래프 제어를 검증한다. 현재값150은 반복하지 않는다.

### CONTROL-IMPACT — Impact

상태: 부분 완료.

기존 근거: 현재 노출4262개와 pad0 Focus33·MIDI현재값2 완료

남은 완료 조건: 다른 pad 선택의 Focus alias/peer 관계와 MIDI event lifecycle·저장/불러오기 상태를 검증한다. pad0 근거를 모든 pad로 확장하지 않는다.

### SHARED-GRAPH — 일반 중첩 그래프 API

상태: 부분 완료.

기존 근거: Splitter42·두 Mixtool Inserts 왕복과 Native48R3의 직접자식6개/두 Device manifest3회·전체72개 전후 대조 완료

남은 완료 조건: 닫힌 create/connect/reorder/delete API를 구현하고 소유권·stale handle·sibling 보존·전체 topology/state의 정확한 복원을 실제 작은 fixture에서 확인한다.

### SHARED-EVENT — MIDI/키 입력 수명 계약

상태: 미착수.

기존 근거: passive MIDI 조회·빈 관측창·일부 현재값만 확인

남은 완료 조건: 고정 press/release·neutral·중복 입력·문맥 소실·unknown 보유의 안전한 계약과 독립 event/held 증거를 구현·검증한다. 클래스별 적용은 CONTROL 항목에서 수용한다.

### SHARED-PERSISTENCE — 저장·재시작 보장 범위

상태: 부분 완료.

기존 근거: Chorder input60/output0의339요청·2Save·2restart property 완료

남은 완료 조건: 기존 property를 재사용하고 새 모델/그래프/설정에 주장할 Save/reopen 범위를 유한 사례로 검증해 지원표와 일치시킨다. 원래 곡을 덮어쓰지 않는다.

### SHARED-MACOS-PACKAGE — macOS 배포/설치·업데이트·복구

상태: 부분 완료.

기존 근거: 현재 Native45/86c5610/46도구 정상 설치·33descriptor 실제읽기 완료; main/bridge bd43f35 재사용

남은 완료 조건: 최종 승인 소스의 설치/업데이트/제거·복구와 설정 보존, CI/배포 artifact·사용법을 일치시킨다. 이미 검증한 Native41 설치를 반복 작업으로 재계상하지 않는다.

### SHARED-DOCS-RELEASE — 최종 지원표·증거·PR/재개 문서

상태: 부분 완료.

기존 근거: 6610 이름·30종/60사례 및 다수 독립 검토 완료

남은 완료 조건: read/write/current/state/Save/event/audio/hardware 지원표를 최종 증거와 맞추고 실패를 보존한다. 지원 범위가 닫힌 뒤 PR/릴리스 상태를 갱신한다. 완료 이름 재감사/재실기를 반복하지 않는다.

### SHARED-PHYSICAL-KEYS — Keystation88 MK3 실제 입력 연동

상태: 하드웨어 대기.

기존 근거: USB 연결 존재 및 빈 MIDI window만 과거 확인

남은 완료 조건: 장치 재연결 뒤 실제 key press/release와 DAW 수신·종료/설정 복원을 확인한다. USB 존재나 소프트웨어 현재값을 물리 연주로 세지 않는다.

## 9. 반복하지 않을 완료 증거와 보존할 제한

Chorder와 Input Filter의128 현재값, Chorder128 model role·selector240·저장339, Multi150 현재값과3752요청은 완료 근거다. 이름이 일부 집계에 남았다는 이유로 Gate·Expander·Compressor swapfreqs를 새 실기로 다시 세지 않는다. Multi의 일반 할당·graph와 다른 instrument MIDI 역할은 별도 미완료다.

SampleOne singleton 설정111요청·전체235와5222byte A/B, Compressor·Limiter·Level의 고정 reset 효과는 완료 범위에서 재사용한다. Scope R68의81요청·12원본은 단발 캡처 유지·한 번 retrigger·전체41설정 복원을 수용했지만 직접 armed/completion과 숨은 파형 메모리·비활성 C 표시 한계는 남는다. CONTROL-VOCAL은 기존 DSP와 R71 host/UI36요청·6원본·전체26원복으로 완료했다. 이를 다시 부분으로 되돌리거나 같은 실기를 재요구하지 않는다.

Splitter42와 두 Mixtool 왕복, 직접 FXInserts 소유 child6개와 전체72읽기는 일반 create/connect/reorder/delete 완료가 아니다. External checkOnly26은 실제 route 선택이나 None 복원·물리 ping을 뜻하지 않는다. Mustang/Rumble의 빈 domain과 모듈 서명 문제의 인과는 미입증이며 설치 파일 비교는 USER_SKIPPED다.

## 10. 실행·검증·Git 규칙

원래 곡과 설정을 보존한다. 실제 실행은 이름이 지정된 owned Scratch만 사용한다. 원본 곡·오디오·앱 로그·스크린샷·절대 로컬 경로·독점 리소스와 raw pseudocode는 Git에 넣지 않는다. 공개 문서는 요약·해시·제한을 기록한다. 실제 근거는 무시된 private 디렉터리에 남기고 manifest로 검증한다.

앱 초기화를 마치고 정확한 저장본을 연 뒤 owner를 시작하는 순서를 유지한다. 이 순서가 과거 모든 시작 정체의 인과적 해결이라고 단정하지 않는다. 문서·bridge session·instance를 맞춘 뒤에만 변경하고 두 프로세스 간 잠금을 유지한다. process-local queue만으로 충분하다고 하지 않는다.

각 요청은 preintent, oncewrite, 원본 receipt, 다음 요청 전 guard를 남긴다. 실제나 helper의 예상 밖 결과는 해당 세션의 추가 DAQ를 멈춘다. 재전송·자동 inverse·계획 없는 load/release나 실패 세션 재사용은 하지 않는다. 종료는 Root의 현재 UI에서 NoSave로 수행한다. screenshot 캡처가 AX 번호를 갱신하므로 최신 번호를 다시 사용한다. 앱 미실행 관찰 직후 CUA completion을 기록한다.

owner는 명시적 child.wait 영수증의 exit0을 확인한다. 단순 터미널 사라짐을 exit0으로 추정하지 않는다. 별도의 passive hardware 조회가 끝난 뒤 process부재와 두 잠금 해제를 검사한다. OS device metadata 동일성을 물리 연결 안정성이나 실제 오디오 경로로 세지 않는다.

제품 writer는 하나이며 Root가 앱·설치·Git·최종 판단을 소유한다. 기존 pstack과 Codex adapter, 같은 부모 모델·effort를 유지한다. 완료한 설계·테스트·검토를 동일한 근거로 다시 수행하는 절차를 만들지 않는다. 새 사실이 필요한 좁은 단위와 meaningful counterfactual·실제 원본을 사용한다. 시간 제한 초과와 검사기의 오류도 원본에 보존한다.

## 11. 다음 실행의 유한 계획

1. 최신 최종 정적 보고서와 Native59 R81 수용을 읽고 현재 source/install/저장본/장치/프로세스만 새로 확인한다. R81 지원 여부 조회를 반복하지 않는다.
2. IStructuredParameter 준비·정리가 변경하는 목록과 parameter 상태의 범위, callback 소유자, cleanup 실패와 문맥 소실 시 처리 계약을 설계한다. 선택지 열거를 callback 없는 일반 read로 분류하지 않는다.
3. 근거가 충분할 때만 정확한 CV·고정 child·깊이·행·문자열 한도를 가진 구조형 관찰을 구현한다. full inventory와 최종 tail, 임시 owner 해제, 원래 metadata 보존을 검사한다. 알려진 serializer 부재와 아직 모르는 output assignment 의미를 구분한다.
4. source·독립 검토·정상 설치·fresh metadata·현재 원본 UI·finite intent계약을 갖춘 새 실제 단위를 실행한다. partial/오류/unknown이면 보장을 넓히지 않는다. factory preset apply·port write를 이 조회에 끼워 넣지 않는다.
5. factory 전체 preset과 PortSetup extra content를 보존하는 public owner 계약 또는 typed endpoint snapshot/restore가 증명된 뒤에만 CV preset 사례를 수용한다.
6. 위의 다른27개 미완료 ID도 유지한다. SHARED-EVENT와 일반 graph, 6개 preset 종류·18클래스의246이름·모델·alias·MIDI·macro·Save 범위를 단계별로 닫는다. 하드웨어4개는 필요한 재연결·배선 조건을 별도로 관리한다.
7. 최종 지원표, macOS 설치/업데이트/제거/복구, Save/reopen, PR와 릴리스 상태를 실제 검증된 범위와 맞춘다. 고정 기능이 아직 남았는데 목표 달성이라고 종료하지 않는다.

전체 기능 완료 예상은 미정이다. 다음 묶음의 범위와 시간은 실제 구조 lifecycle 결과를 바탕으로 새로 산정한다. 오늘18시 마감은 이 실행의 정리 시각이며 이후 재개 실행의 새 마감으로 자동 재사용하지 않는다.

## 12. 최종 수용과 재개의 정확한 출발점

R81의독립검토273.821초가현재원본10장·실제stdout29원본·27intent/oncewrite/guard·fresh55세션제외·현재pins·설치Mach-O UUID를확인해제한을명시하고수용했다. 초기검토기manifest객체형식오류exit1과수정후exit0을보존했다. 17:41목표는19.821초넘겼으나hard300은지켰다. 실제단위620.268초, 누적239raw/222수용/17미수용, 고정15/43완료28잔여다. [R81실제근거](evidence/2026-11-17-cv-structured-native59-r81.json)를참조한다.

마지막정적단위297.327초에서새본문3개·근거10개를추가했다. 정확한공개구조형table에서prepare/cleanup/count/get을연결했다. prepare는객체와연결리스트를만들고내부필드를쓰며미식별owner인터페이스callback을호출한다. cleanup도필드를쓰고별도helper를호출한다. 순수읽기·멱등성·중첩수명·assignment보존을입증하지못했다. 엔진은175.469초에닫혔고export10/close/openfalse/cleanupfalse/reset/EOFwait0과원본6pins를확인했다. 한상수read의0바이트결과는부재가아닌미완전관측이다. [lifecycle정적근거](evidence/2026-11-17-cv-structured-lifecycle-static.json)를참조한다.

다음유한정적범위는owner IID를이미고정한공개SDK에서먼저식별한뒤cleanuphelper와공개count/get구현의최대3본문이다. 기존준비본문을반복하지않는다. SDK에서이름을찾지못하면UNKNOWN으로유지하며privateABI호출로대체하지않는다. 준비/정리가변경하는범위와cleanup실패·문맥소실·원래상태보존계약을검증한뒤기존사용자요청범위안에서구현과실기를이어간다. 이를새사용자승인요청으로바꾸지않는다.

다음정적단위는4~7분예상이다. 그뒤구현·실기전체시간은효과계약이닫히지않아미정이다. 오늘종료마감은이후명시적재개시새마감으로재사용하지않는다.

## 13. 최종 안전 종료 확인

18시 마감 전에 feature 실행을 중단하고 정리했다. 최종 readonly 확인에서source181/artifact13/saved10/Main이동일하고앱·MCP main프로세스가없으며두잠금을다시획득·해제했다. normal과actual owner,마지막정적엔진의명시적waitedchild0을재확인했다. doctorcurrent=true/live_verified=false와bridgeclosed를확인했다. OS오디오5개·MIDI source/destination0개이며물리안정성은미검증이다. 새DAQ·자동재개·예약작업은없다. [최종정리근거](evidence/2026-11-17-final-pause-native59.json)를참조한다.

남은28개ID와우선순위를보존했으며다음재개때완료한실험을다시시작하지않는다. 마지막제품소스는7f0cd1d이고후속문서commit은제품source authority를바꾸지않는다. GitHub PR4는Draft로유지하며merge/release/fullplugin완료를주장하지않는다.

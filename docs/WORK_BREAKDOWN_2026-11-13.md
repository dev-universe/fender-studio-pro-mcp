# 내장 플러그인 잔여 작업표 — 2026-11-13

**고정한43개 목록 중1개를 완료했고42개가 남았다: 프리셋16 + 프리셋 외 플러그인 기능20 + 공통6.** 남은 상태는부분37개·미착수1개·하드웨어대기4개다. PRESET-SPECTRUM은 두 고정설정 사례의 별도 수용을 마쳐 완료로 기록한다. 이43개는 목록을 만들기 전에 완료한 과거 작업을 제외한 기준이다.

과거 작업은 서로 다른 크기의 setter·실기·리뷰·기능으로 기록돼 같은 단위로 역사 전체 작업 수와 완료 수를 재집계하지 않았다. 따라서 **전체 작업 총수·전체 완료율·전체 종료 시각은 미확정**이다.43은 고정한 목록의 기준 개수이지 남은 실기 횟수나 반복 횟수가 아니다. 한 완료물에 필요한 하위 실험 수는 아직 모두 확정되지 않았다.

한 단위는 “플러그인 또는 공통 하위시스템의 독립 기능 완료물”이다. 예를 들어 프리셋 수용과 일반 그래프 편집은 별도 결과이므로 별도 ID지만, 같은 증거를 재사용하며 같은 실기를 두 번 요구하지 않는다. 파라미터 잔여 수와 프리셋 잔여 종 수를 같은 크기의 작업처럼 더하지 않는다.

| 기존 지표 | 완료/전체 | 남음 | 의미 |
|---|---:|---:|---|
| 고정 현재값 이름 |6733/7112|379개·19클래스|모델·이벤트·오디오·저장·전체지원률이 아님|
| 추가 프리셋 종 |30/46|16종|완료60사례. catalog59종과는 다른 지표|
| 고정한 완료물 목록 |43개 중1개 완료|42개|아래 ID 기준. 하위 요청 수/노력은 서로 다름|

## 상태와 완료 기준

- **완료**: 이 목록의 완료 조건을 실제 검증과 별도 수용으로 충족했다. 플러그인의 모든 기능이 끝났다는 뜻은 아니다.
- **부분 완료**: 관련 API·제한된 실제 증거가 있지만 해당 완료 조건 전체는 충족하지 못했다. 구현 여부와 실제 검증 범위는 아래에 별도 기록한다.
- **미착수**: 이 표의 독립 완료물 수준에서는 실행 가능한 계약을 아직 만들지 못했다.
- **하드웨어 대기**: 전체 완료에 재연결/배선/물리 관측이 필요하다. 해당 행의 소프트웨어 준비까지 모두 멈춘다는 뜻은 아니다.
- 완료는 실제 의미·독립 읽기·원래 상태 보존과 정상 정리의 증거로 판단한다. 소스 구현이나 accepted 응답만으로 완료하지 않는다.

## 고정 잔여 목록

### 프리셋

| ID | 완료물 | 상태 | 기존 구현/실제 근거 | 남은 완료 조건 |
|---|---|---|---|---|
|PRESET-ROOM|Room Reverb|부분 완료|쓰기16개 복원·새725바이트 직렬화 완료. 원래 읽기전용 표시 복원은 미확인|이미 완료한 writable/새직렬화 증거를 재사용하고, 원래 derived/read-only 상태와 프리셋 종 완료 기준의 간극을 소스 근거와 새 유한 조건으로 해결한다. 같은 실패 gate를 재시도하지 않는다.|
|PRESET-OPENAIR|Open AIR|부분 완료|쓰기43개 복원·새2033바이트 직렬화 완료. 읽기전용16개 원래 복원은 미확인|이미 완료한 writable/새직렬화 증거를 재사용하고, 원래 derived/read-only 상태와 프리셋 종 완료 기준의 간극을 소스 근거와 새 유한 조건으로 해결한다. 같은 실패 gate를 재시도하지 않는다.|
|PRESET-SPECTRUM|Spectrum Meter|완료|서로 다른 issued index1/2의 두 고정설정·각전체9복원·새584byte A/B·해제/만료·NoSave/owner0/저장6개 보존을 수용|두 실기를 반복하지 않는다. 모든32항목·factory origin·audio·지속저장·전체플러그인 지원의 완료는 아니다.|
|PRESET-LEVEL|Level Meter|부분 완료|issued catalog 발견·해제·만료 거부. 이46종 계획의 완료 사례는 아직 없음|고정 issued index의 실제 설정 변화·독립 안정쌍·전체 원래값/스키마 복원·token 해제/만료·NoSave 정리를 충족하는 사례를 기존 두 사례 기준으로 수용한다. 이미 완료한 사례는 재사용한다.|
|PRESET-SCOPE|Scope|부분 완료|issued catalog 발견·해제·만료 거부. 이46종 계획의 완료 사례는 아직 없음|고정 issued index의 실제 설정 변화·독립 안정쌍·전체 원래값/스키마 복원·token 해제/만료·NoSave 정리를 충족하는 사례를 기존 두 사례 기준으로 수용한다. 이미 완료한 사례는 재사용한다.|
|PRESET-TONEGEN|Tone Generator|부분 완료|issued catalog 발견·해제·만료 거부. 이46종 계획의 완료 사례는 아직 없음|고정 issued index의 실제 설정 변화·독립 안정쌍·전체 원래값/스키마 복원·token 해제/만료·NoSave 정리를 충족하는 사례를 기존 두 사례 기준으로 수용한다. 이미 완료한 사례는 재사용한다.|
|PRESET-AMPIRE|Ampire|부분 완료|issued catalog 발견·해제·만료 거부. 이46종 계획의 완료 사례는 아직 없음|고정 issued index의 실제 설정 변화·독립 안정쌍·전체 원래값/스키마 복원·token 해제/만료·NoSave 정리를 충족하는 사례를 기존 두 사례 기준으로 수용한다. 이미 완료한 사례는 재사용한다.|
|PRESET-PEDALBOARD|Pedalboard|부분 완료|issued catalog 발견·해제·만료 거부. 이46종 계획의 완료 사례는 아직 없음|고정 issued index의 실제 설정 변화·독립 안정쌍·전체 원래값/스키마 복원·token 해제/만료·NoSave 정리를 충족하는 사례를 기존 두 사례 기준으로 수용한다. 이미 완료한 사례는 재사용한다.|
|PRESET-RUMBLE|Rumble Native|부분 완료|issued catalog 발견·해제·만료 거부. 이46종 계획의 완료 사례는 아직 없음|고정 issued index의 실제 설정 변화·독립 안정쌍·전체 원래값/스키마 복원·token 해제/만료·NoSave 정리를 충족하는 사례를 기존 두 사례 기준으로 수용한다. 이미 완료한 사례는 재사용한다.|
|PRESET-VUMETER|VU Meter|부분 완료|issued catalog 발견·해제·만료 거부. 이46종 계획의 완료 사례는 아직 없음|고정 issued index의 실제 설정 변화·독립 안정쌍·전체 원래값/스키마 복원·token 해제/만료·NoSave 정리를 충족하는 사례를 기존 두 사례 기준으로 수용한다. 이미 완료한 사례는 재사용한다.|
|PRESET-MULTI|Multi Instrument|부분 완료|issued catalog 발견·해제·만료 거부. 이46종 계획의 완료 사례는 아직 없음|고정 issued index의 실제 설정 변화·독립 안정쌍·전체 원래값/스키마 복원·token 해제/만료·NoSave 정리를 충족하는 사례를 기존 두 사례 기준으로 수용한다. 이미 완료한 사례는 재사용한다.|
|PRESET-SAMPLEONE|SampleOne|부분 완료|issued catalog 발견·해제·만료 거부. 이46종 계획의 완료 사례는 아직 없음|고정 issued index의 실제 설정 변화·독립 안정쌍·전체 원래값/스키마 복원·token 해제/만료·NoSave 정리를 충족하는 사례를 기존 두 사례 기준으로 수용한다. 이미 완료한 사례는 재사용한다.|
|PRESET-IMPACT|Impact|부분 완료|issued catalog 발견·해제·만료 거부. 이46종 계획의 완료 사례는 아직 없음|고정 issued index의 실제 설정 변화·독립 안정쌍·전체 원래값/스키마 복원·token 해제/만료·NoSave 정리를 충족하는 사례를 기존 두 사례 기준으로 수용한다. 이미 완료한 사례는 재사용한다.|
|PRESET-CV|CV Instrument|부분 완료|issued catalog 발견·해제·만료 거부. 이46종 계획의 완료 사례는 아직 없음|고정 issued index의 실제 설정 변화·독립 안정쌍·전체 원래값/스키마 복원·token 해제/만료·NoSave 정리를 충족하는 사례를 기존 두 사례 기준으로 수용한다. 이미 완료한 사례는 재사용한다.|
|PRESET-MOJITO|Mojito|부분 완료|issued catalog 발견·해제·만료 거부. 이46종 계획의 완료 사례는 아직 없음|고정 issued index의 실제 설정 변화·독립 안정쌍·전체 원래값/스키마 복원·token 해제/만료·NoSave 정리를 충족하는 사례를 기존 두 사례 기준으로 수용한다. 이미 완료한 사례는 재사용한다.|
|PRESET-CHORDER|Chorder|부분 완료|issued catalog 발견·해제·만료 거부. 이46종 계획의 완료 사례는 아직 없음|고정 issued index의 실제 설정 변화·독립 안정쌍·전체 원래값/스키마 복원·token 해제/만료·NoSave 정리를 충족하는 사례를 기존 두 사례 기준으로 수용한다. 이미 완료한 사례는 재사용한다.|
|PRESET-REPEATER|Repeater|부분 완료|issued catalog 발견·해제·만료 거부. 이46종 계획의 완료 사례는 아직 없음|고정 issued index의 실제 설정 변화·독립 안정쌍·전체 원래값/스키마 복원·token 해제/만료·NoSave 정리를 충족하는 사례를 기존 두 사례 기준으로 수용한다. 이미 완료한 사례는 재사용한다.|

### 프리셋 외 플러그인 기능

| ID | 완료물 | 상태 | 기존 구현/실제 근거 | 남은 완료 조건 |
|---|---|---|---|---|
|CONTROL-AMPIRE|Ampire|부분 완료|일반39개·catalog/owner 읽기|alias·micmix/linked·bypass/tuner·stomp 순서의 남은19역할을 올바른 대상에 변경·읽기·복원하고 연동 peer/구조를 보존한다.|
|CONTROL-CV|CV Instrument|하드웨어 대기|provider/선택 범위와 역할97개 분류|config86·generator4·MIDI3의 유한 변경/복원, 포트 격리와 calibration2/monitor2의 실제 효과·중지·복원을 구분해 입증한다.|
|CONTROL-CHORDER|Chorder|부분 완료|input60 출력128 모델·selector·고정Song저장 완료; guiInput canonical1~74의74개 현재값/정확74이름 수용. 73 상단경계의 예측6반응·다른901과 74 첫 범위 밖의 예측2반응·다른905를 각각 확인. 전체908/저장128복원·Root정리 완료. 나머지59개·MIDI/모델대응·noteoff·audio는별도|guiInput128개와 clear/clearAll/copy/paste/playLearnChord5개 역할을 수명에 맞게 처리한다. 고정 현재값 복원과 실제 note 수명을 구분하고, 다른 입력 모델 접근·clipboard/미리듣기 결과를 독립 확인한다.|
|CONTROL-COMPRESSOR|Compressor|부분 완료|일반 설정/프리셋·swapfreqs action 완료|남은 resetmin의 실제 최소값 초기화 효과와 정상 lifecycle을 확인한다. 완료 swapfreqs는 재실행하지 않는다.|
|CONTROL-EXTERNAL-MONO|External Effect Mono|하드웨어 대기|현재값·mode·opaque 메뉴 읽기 완료|Send/Return 선택의 공개 global filter·포트 매핑·None 원복, 파생 latency/단위표시를 구분하고 Volt loopback에서 ping/왕복지연/오디오를 검증한다.|
|CONTROL-EXTERNAL-STEREO|External Effect Stereo|하드웨어 대기|현재값·mode·메뉴·held-target provenance·checkOnly26 완료|두 포트의 실제 선택·None 원복과 global filter 경계를 검증하고, 파생 latency/단위표시·물리 stereo loopback/ping을 확인한다. checkOnly를 선택 성공으로 세지 않는다.|
|CONTROL-INPUTFILTER|Input Filter|부분 완료|일반 필터 설정·provider/catalog; keyboard.pitch1~49.guiInput49개 현재값/49이름·전체388복원과Root정리 완료. 나머지79개·MIDI수명은미완료|keyboard.pitch1..128.guiInput을 현재값·pitch 대응·press/release 결과로 분리해 확인하고 원본 상태와 종료를 복원한다.|
|CONTROL-LEVEL|Level Meter|부분 완료|전체DSP7 읽기·controller2노드/1연결, useisl known miss 완료|resethold/resetloudness의 실제 reset 효과와 controller/숨은 설정 접근을 확인한다. 제한 metadata를 값/쓰기 권한으로 승격하지 않는다.|
|CONTROL-LIMITER|Limiter|부분 완료|일반 설정과 두 프리셋 사례 완료|resetmin 명령의 별도 효과·완료·원래 환경 보존을 확인한다.|
|CONTROL-MAITAI|Mai Tai|부분 완료|일반 선택 설정·preset 계열 완료|MIDI6 facade의 현재값/실제 전달과 character1.editchar의 GUI·조건부 모델 binding/복원을 검증한다.|
|CONTROL-MOJITO|Mojito|부분 완료|일반 선택 설정 완료|afterTouch/modWheel/pitchBend3역할의 현재값·수신·neutral 복원/종료를 검증한다.|
|CONTROL-MUSTANG|Mustang Native|부분 완료|provider·RemoteRoot/workspace·selector 조회 및 빈 모델 목록 관측; 기존 preset recall 완료|현재 비어 있는 amp/effect 실제 domain과 선택·stomp/amp 위치·routing·tuner/bypass29역할의 변경/복원을 검증한다. 빈 목록을 전체지원으로 세지 않는다.|
|CONTROL-PEDALBOARD|Pedalboard|부분 완료|일반2개·provider/catalog 완료|alias11·host bypass의 대상/연동·실제 변경과 정확한 원복, 중첩 rack 제어를 검증한다.|
|CONTROL-PRESENCE|Presence|부분 완료|일반 선택 설정 완료|MIDI6와 script/program macro16개를 올바른 program/target에 연결하고 현재값·실제 효과·원복을 검증한다.|
|CONTROL-RUMBLE|Rumble Native|부분 완료|일반3개·workspace/selector·빈 모델 목록 조회|실제 amp/effect domain과 선택·amp 위치/stomp routing·tuner/bypass29역할을 검증한다.|
|CONTROL-SAMPLEONE|SampleOne|부분 완료|일반 선택 설정 완료|MIDI6의 현재값·수명과 sampler load/model의 실제 상태·복원을 확인한다. 단순 preset ack와 구분한다.|
|CONTROL-SCOPE|Scope|부분 완료|일반 설정·singleton catalog 완료|trigOneshot의 armed/capture와 trigRetrig의 retrigger 효과·완료·원래 상태를 별도 검증한다.|
|CONTROL-MULTI|Multi Instrument|부분 완료|현재값150개·8실기/3752요청·이름150 수용 및 controller34/36 제한 읽기 완료|macro20 target map·controller130 facade assignment/전달·child identity·일반 중첩 그래프 제어를 검증한다. 현재값150은 반복하지 않는다.|
|CONTROL-IMPACT|Impact|부분 완료|현재 노출4262개와 pad0 Focus33·MIDI현재값2 완료|다른 pad 선택의 Focus alias/peer 관계와 MIDI event lifecycle·저장/불러오기 상태를 검증한다. pad0 근거를 모든 pad로 확장하지 않는다.|
|CONTROL-VOCAL|Vocal Tune|부분 완료|root/mask13·직접 DSP bypass 현재값·프리셋 완료|직접 DSP bypass와 별도 host bypass/owning callback의 실제 반영·복원 보장 범위를 확인한다. 현재값 완료나 callback 주소 부재를 일반 금지로 바꾸지 않는다.|

### 공통

| ID | 완료물 | 상태 | 기존 구현/실제 근거 | 남은 완료 조건 |
|---|---|---|---|---|
|SHARED-GRAPH|일반 중첩 그래프 API|부분 완료|Splitter42·두 Mixtool Inserts 왕복 완료|닫힌 create/connect/reorder/delete API를 구현하고 소유권·stale handle·sibling 보존·전체 topology/state의 정확한 복원을 실제 작은 fixture에서 확인한다.|
|SHARED-EVENT|MIDI/키 입력 수명 계약|미착수|passive MIDI 조회·빈 관측창·일부 현재값만 확인|고정 press/release·neutral·중복 입력·문맥 소실·unknown 보유의 안전한 계약과 독립 event/held 증거를 구현·검증한다. 클래스별 적용은 CONTROL 항목에서 수용한다.|
|SHARED-PERSISTENCE|저장·재시작 보장 범위|부분 완료|Chorder input60/output0의339요청·2Save·2restart property 완료|기존 property를 재사용하고 새 모델/그래프/설정에 주장할 Save/reopen 범위를 유한 사례로 검증해 지원표와 일치시킨다. 원래 곡을 덮어쓰지 않는다.|
|SHARED-MACOS-PACKAGE|macOS 배포/설치·업데이트·복구|부분 완료|현재 Native41/c733ce5/46도구의 정상 설치·실기 완료|최종 승인 소스의 설치/업데이트/제거·복구와 설정 보존, CI/배포 artifact·사용법을 일치시킨다. 이미 검증한 Native41 설치를 반복 작업으로 재계상하지 않는다.|
|SHARED-DOCS-RELEASE|최종 지원표·증거·PR/재개 문서|부분 완료|6651 이름·30종/60사례 및 다수 독립 검토 완료|read/write/current/state/Save/event/audio/hardware 지원표를 최종 증거와 맞추고 실패를 보존한다. 지원 범위가 닫힌 뒤 PR/릴리스 상태를 갱신한다. 완료 이름 재감사/재실기를 반복하지 않는다.|

### 하드웨어

| ID | 완료물 | 상태 | 기존 구현/실제 근거 | 남은 완료 조건 |
|---|---|---|---|---|
|SHARED-PHYSICAL-KEYS|Keystation88 MK3 실제 입력 연동|하드웨어 대기|USB 연결 존재 및 빈 MIDI window만 과거 확인|장치 재연결 뒤 실제 key press/release와 DAW 수신·종료/설정 복원을 확인한다. USB 존재나 소프트웨어 현재값을 물리 연주로 세지 않는다.|

## 이름 수와 완료 기능을 혼동하지 않는 기준

19클래스의 이름 잔여 목록에는 이미 실제 검증한 Gate/Expander/Compressor의 swapfreqs3개도 scalar 집계 방식 때문에 남아 있다. 이는 **장부 분류 항목이며 새 실행 작업이 아니다**. Gate와 Expander를 별도 잔여 플러그인 기능 행에 넣지 않았다. 반대로 현재값이 끝난 Multi·Impact·Vocal은 할당/다른 focus/host callback 등의 미검증 기능이 있어 위 기능 행에 남겼다. 이름 숫자만으로 전체 기능 잔여를 산출할 수 없는 이유다.

Chorder input60의 출력128 모델 역할·selector 경계·고정 Song 저장339요청, Multi150 현재값/3752요청, Vocal 직접 bypass 현재값, Native40 Level23, Native41 External26, Tuner 두127사례, Spectrum의 기존 설정 사례, 이미 수용한 프리셋30종은 재실행 목록이 아니다. 모델·현재값·저장·오디오 간에 성공을 자동 전이하지 않는다.

## 현재 실행과 장치

현재 실행 단위·성공과 실패·최근 종료 및 다음 순서는 `RESUME_STATE.json`을 우선한다. 이 표는 잔여 완료물의 기준이며 실행 중인 단위의 상태를 별도로 복제하지 않는다.

사용자는 Volt276와 Keystation88 MK3를 분리했다. 과거 연결은 USB만이었으며 loopback 배선은 없었다. 따라서 External의 물리 ping/왕복지연, CV calibration/monitor, 실제 건반 수신은 장치/필요 배선 재연결까지 별도 대기다. Main0을 CV 포트 격리나 물리 무음으로 보지 않는다.

## 근거와 유지 규칙

- [RESUME_STATE.json](RESUME_STATE.json): 현재 상태·accepted/next_units/hardware; stale remaining_preset array는 accepted에서 정규화.
- [PROGRESS_2026-11-13.md](PROGRESS_2026-11-13.md): 최신 Chorder 실패와 후속 준비 및 단위 이력.
- [PLUGIN_STATE_VALIDATION.md](PLUGIN_STATE_VALIDATION.md): 영역별 완료/미검증; 오래된 숫자는 현재JSON에 종속.
- [LOOP_HANDOFF_2026-11-12.md](LOOP_HANDOFF_2026-11-12.md): 13.2~13.5 기존 잔여 범위만 재사용; 완료 P0-P5 재큐잉 안함.
- [PARAMETER_NAME_UNION_CURRENT.md](PARAMETER_NAME_UNION_CURRENT.md): 6733/7112·379·19클래스의 현재값 전용 범위.

현재 JSON의 accepted와 이름 합집합을 우선했다. 오래된 검증 문서/인수인계의 과거18·25·28종 숫자나 완료된 P0–P5 단위를 새 backlog로 되살리지 않았다. stale Tuner 배열은 작성자가 accepted 기준으로 정규화했고, 이후 Root가 현재 RESUME_STATE 배열도17종으로 맞췄다. 일반 편집·automation·bounce·process tap 등 최초 장기 범위는 이번 내장 플러그인 잔여 분모에 새로 얹지 않았다. 이미 구현·검증된 범위는 별도 과거 근거로 유지한다.

이 고정 목록43개를 더 쪼개거나 범위를 발견하면 기존 ID와 변경 사유를 보존하고 분모 변경을 명시해야 한다. 새 장애 수정은 원래 완료물의 하위 작업으로 기록하며 조용히 새 목표로 늘리지 않는다. 장치 대기와 소프트웨어 잔여를 나눠 보고하고, 단위별 ETA만 측정값으로 제공한다.

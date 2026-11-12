# 작업 재개 안내

현재 작업은 사용자의 2026-11-13 `진행 재개해` 요청으로 진행 중이다.
최신 완료 단위와 다음 작업은 [현재 진행 기록](PROGRESS_2026-11-13.md)과 `RESUME_STATE.json`을 읽는다.
11월12일의 중단 인수인계는 이전 완료 증거와 전체 잔여 범위를 보존한다.
자동 재개나 예약 실행은 새로 만들지 않았다.

대화나 앱을 다시 열 때 다음 순서로 현재 상태를 대조한다.

1. 저장소 `AGENTS.md`와 사용자의 최신 지시를 읽는다.
2. [11월12일 상세 인수인계](LOOP_HANDOFF_2026-11-12.md)를 읽는다. 그 문서의 미실행 unit64는 이번에 완료했다.
3. [11월13일 진행 기록](PROGRESS_2026-11-13.md)과 [상태 JSON](RESUME_STATE.json)을 읽는다.
4. 실제 Git·프로세스·설치 파일·소유 곡을 대조하고 필요한 다음 단위에 fresh binding을 만든다.
5. 최신 `next_units_in_order`에서 이어간다. 완료한 unit64와 과거 대규모 sweep을 반복하지 않는다.
6. 단위 완료마다 실제 시간·완료와 잔여 수·전체 예상 잔여 시간과 변경 근거를 보고한다.

새로운 중단·범위 변경 지시는 이 안내보다 우선한다.
다시 중단한 뒤에는 명시적인 재개 지시 없이 실제 검증을 시작하지 않는다.

현재 체크포인트는 다음과 같다.

- Native34 R2 제품 소스 `3815714`와43개 도구 실행 파일로 unit64를 실제8.1.1에서 검증했다.
- 입력128개의 저장 모델을 두 pass·총16회 serializer read로 읽었다. 전후908개 원본 네 번과 GUI getter 네 번도 일치했다.
- unit64는117.091초였고 쓰기·state capsule·load·transport·Song Save는0회였다. 마지막 진단 문서는 저장 없이 닫았고 앱·owner가 종료됐다.
- 새 증거는 현재 저장 모델의 읽기다. 모델 편집·복원·persistence·audio나 GUI child 출력 음정 매핑은 아직 확정하지 않는다.
- 추가 프리셋46종 계획에서20종·40사례가 완료됐고26종이 남았다. unit64 때문에 이 수를 늘리지 않는다.
- unit65는 Mixverb의 strict typed 복원 실패로 종료·정리됐다. 추가 성공0종, 저장본 불변. unit66은 Studio Verb·Compressor2종·4사례를 추가하고 Voice FX의 무변화 관찰에서 멈췄다. 미실행5종·9사례는 unit68로 계획하며65·66을 재실행하지 않는다.
- 사용자는 Volt 276과 Keystation 88 MK3를 한국 시간11월13일 오전6시30분까지 USB로만 연결한다. 물리 loopback 배선은 없다.
- 실제 Audio Setup에서 재생·녹음 Volt276·48kHz·512샘플을 확인했다. 원래 MacBook 스피커·마이크 설정의 UI 복원이 하드웨어 단위 종료 또는 마감 전에 반드시 필요하다. DAW I/O는 INPUT1·2/MONITOR L·R을 관찰했고 입력 bus는 비어 있다. External 전체 provider·선택지 확인은 남아 있다. 첫 UI Main 최솟값은 exact0이 아니어서 중단했으며 새 단위는 API Main1→0 준비1회를 선언한다.
- Rust inventory·bounded observer의 소스·독립 검토·실제 CLI/MCP 검증이 완료됐고 main은45개 도구다. 세 수신 창은 event0과 정리status0이며 실제 건반 연주 증거는 아니다. 소스7dedec2와 [하드웨어 안내](HARDWARE.md)를 참고한다.
- REA6.9.0·portableNode26.12.1·Craft0.6.1 설치·실행과 Codex 등록 갱신, 새CLI+Ghidra12.1.4 분석을 확인했다. 기존 연결REA MCP는6.5.0이며 새CLI를 사용한다. Hopper는 사용하지 않았다.

오전6시30분 전에는 장치·DAW I/O·읽기 전용 External 선택지와 passive MIDI 검증을 우선한다.
USB 연결만으로 물리 ping이나 CV 출력 격리를 인정하지 않는다.
하드웨어 시간과 조건이 허용하는 동안 일반 프리셋을 이어가되 실패 뒤 retry·추가 load는 하지 않는다.

기존 launcher의 현재 source·검토된 child SHA·새 output namespace를 확인한 뒤 실행한다.
완료된 unit62·63 또는 unit64를 옛 binding으로 다시 큐에 넣지 않는다.
runtime token·instance·document·bridge session·UI flag는 다음 단위에서 재사용하지 않는다.
PR #4는 Draft로 유지하며 미완료 범위를 전체 지원으로 승격하지 않는다.

전체 예상 잔여 시간은6~18시간이며 신뢰도는 낮다.
하드웨어 연결 마감과 전체 작업 종료 추정은 별개다.
각 단위 종료 관측으로 추정을 갱신하고 하드웨어·배선이 필요한 항목을 따로 보고한다.

상세 범위는 [플러그인 상태 검증](PLUGIN_STATE_VALIDATION.md), 최초 P0~P5 계획은
[구현 계획](IMPLEMENTATION_PLAN.md)에 보존한다.
최초 계획이나 옛 인수인계의 오래된 체크박스만 보고 현재 완료 여부를 판단하지 않는다.

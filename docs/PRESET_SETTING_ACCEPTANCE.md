# 프리셋 설정 사례의 수용 기준

2026-11-14에 기존 일반 프리셋 기준과 Room Reverb/Open AIR의 더 엄격한 실험 기준을 대조했다. 사용자가 요청한 쓰기 가능한 전체 파라미터와 프리셋 제어를 기준으로, **프리셋 설정의 정확한 복원**과 **읽기 전용 현재값까지 포함한 전체 상태의 복원**을 별도 주장으로 기록한다. 후자는 두 플러그인에서 여전히 검증되지 않았다.

이번 결정은 기존 수용된 쓰기값·새 직렬화 실기의 범위를 명시적으로 일반 프리셋 설정 집계에 연결한다. 과거 실패 원장과 그 원래 전역 수용0을 수정하지 않는다. 실제 실험을 반복하거나 새로운 완료물을 추가하지 않았다.

## 프리셋 설정 사례

한 사례는 다음을 모두 충족해야 한다.

1. 정확한 문서·세션·class/instance와 issued catalog entry를 묶고, 실제 쓰기 가능 설정의 긍정적인 변화를 독립된 읽기로 확인한다. 요청 수락만으로 성공 처리하지 않는다.
2. 복원 뒤 모든 쓰기 가능 current value의 typed raw/normalized/text와 전체 provider의 schema·비값·알 수 없는 metadata가 원래와 정확히 같아야 한다. 쓰기값의 한 ULP 차이도 허용하지 않는다.
3. 읽기 전용 값의 관측은 authoritative access metadata가 complete/readOnly:true/writable:false인 고정 record에만 적용한다. schema·종류·태그·범위·지원 여부·소유 관계는 같은 비교 기준을 유지한다. 현재값 차이와 정확한 이름을 별도로 기록하며 meter나 비영구 상태라고 가정하지 않는다.
4. 원래 A와 복원 후 B는 서로 다른 요청과 token으로 새로 IStorable.save를 수행해야 한다. 같은 저장 범위·길이·바이트 서명이 같아야 한다. load 응답에 보존된 원래 서명만 다시 읽은 것은 B가 아니다.
5. state/catalog token 수명과 해제·만료 거부, 원래 환경·저장본 보존과 Root의 저장 없는 종료가 확인돼야 한다. 이미 완료한 독립 lifecycle 근거는 재사용한다.
6. 오디오·IR/리소스 의미·읽기 전용 직렬화 참여·hidden memory 전체·Song 저장/재시작·전체 플러그인 완료로 확장하지 않는다.

현재 class별 대표 사례 수는 **완전하고 비어 있지 않은 issued catalog에서 min(2, entry 수)**다. 선택지가0개면 완료로 처리하지 않는다. 두 개 이상이면 서로 다른 두 entry의 긍정 사례가 필요하다. 한 개뿐인 catalog에서 같은 실행을 반복해 다른 프리셋 두 개처럼 세지 않는다. 이 규칙은 모든 프리셋이나 factory origin 전체를 검증했다는 뜻이 아니다.

## 기존 증거의 별도 수용

| 대상 | 이번에 연결한 범위 | 남는 제한 |
|---|---|---|
| Open AIR | issued index0 default 하나의 긍정 Mix 반응, 쓰기43개와 전체94개 비값 metadata 원복, 별도 fresh A/B2033바이트 동일, 수명·Root 정리 확인. 실제 선택지1개이므로 대표 설정 class1/1 완료 |읽기 전용16개와 전체94개 현재값의 원래 복원은 false. tap 값을 단순 meter로 취급하지 않음. 리소스·오디오·저장 미검증|
| Room Reverb | issued index0 Arena의 쓰기16개와 전체19개 비값 metadata 원복, fresh A/B725바이트 동일, 수명·Root 정리 확인. class는 부분 완료 |두 번째 서로 다른 entry index1 BD - Inside가 남음. 읽기 전용 outdepth/outheight의 원래 bit 복원과 전체19개 현재값 원복은 false|

Open AIR의 readonly tanktap 값들은 기존 정적 preset에도 들어 있다. readonly라는 사실만으로 transient meter나 저장되지 않는 값으로 분류하지 않았다. Room의 두 readonly 차이는 한 ULP지만 쓰기값 비교 기준을 완화한 근거로 쓰지 않는다.

기존 독립 보고서·원시 관측과24개 불변 입력 핀을 Root가 대조했다. 새 실제 호출은0회다. 현재 일반 프리셋 집계는 **31/46종·완료 종의61사례, 별도 부분 Room1사례·15종 잔여**다. 고정43개 목록은 PRESET-OPENAIR를 추가로 마쳐 **2개 완료·41개 잔여**다. 이름6,866/7,112와246개 잔여는 변하지 않는다.

[별도 수용 증거](evidence/2026-11-14-preset-setting-scope-admission.json), [원래 Room 실기](evidence/2026-11-13-room-serialized-identity.json), [원래 Open AIR 실기](evidence/2026-11-13-openair-writable-serialization.json)를 함께 읽는다. 두 원래 실패의 전체 상태 복원 주장은 계속 false다.

## 후속 현재 결과 — Native46

이상의 결정 시점31/46종·부분 Room1사례는 역사 기록이다. 별도 새 index1 `BD - Inside`의 긍정 설정 변화·쓰기16개/전체19 metadata 정확 복원·fresh725바이트 A/B 동일·해제와 Root 정리를 확인해 Room의 서로 다른2/2 사례를 완료했다. 현재는 **32/46종·완료 종63사례·14종 잔여**, 고정43개 중 **3개 완료·40개 잔여**다. readonly outdepth/outheight/전체19 current와 프리셋 표시 이름 복원은 false다. 전체 상태·오디오·리소스·저장 검증으로 확장하지 않는다. [새 실제 증거](evidence/2026-11-14-room-index1-native46.json)를 참조한다.

## 후속 현재 결과 — Native47 Level

Level Meter는 기존 DSP7와 등록6설정의 union13을 사용해 issued index1 Movie Score Classic과 index2 Pop Rock을 각각 실제 적용·독립 관측·원복했다. 각51요청, capsule load1회/추가보정0회로 쓰기12개와 전체13 current/metadata가 정확히 돌아왔고 독립 fresh406바이트 A/B도 같았다. 수명과 Root 정리를 함께 확인해 대표2/2로 수용했다. 이 두 사례에서 readonly correlation 원래값도 같았다는 관측은 기록하되 직렬화 참여나 모든 상태의 증명으로 확장하지 않는다. 프리셋 표시 이름의 원래 복원은 false다.

현재는 **33/46종·완료 종65사례·13종 잔여**, 고정43개 중 **5개 완료·38개 잔여**다. [index1](evidence/2026-11-14-level-preset-index1-native47.json)과 [index2](evidence/2026-11-14-level-preset-index2-native47.json)는 서로 다른 새 문맥의 증거다. 과거 결정 시점31/46, 후속 Room32/46 및 첫 Level 부분1/2는 이력을 보존하며 재실행 지시가 아니다.

## Scope singleton의 실제 별도 적용

fresh complete issued catalog가 하나인 Scope는 최소 대표 사례 수가1개다. 최초 P에서 일반 `showSecondary`0→1을 한 번 준비한 O를 별도 기준으로 캡처했다. default 적용의 실제1→0 변화와 전체31개 O 정확 복원, 새758바이트 A/B 동일, 수명·정리를 확인했다. 이어 사전에 선언한 최초0 복원과 전체31개 P 독립쌍까지 통과했다. 쓰기31개·읽기 전용0이며 비값/unknown metadata를 모두 정확 비교했다. 표시 이름도 최초/최종 `default*`가 같았다.

[새 실제 증거](evidence/2026-11-14-scope-singleton-native47.json)는329.338초·46요청이고, 대표 설정1/1만 수용한다. Scope 두 트리거의 실제 효과·waveform/audio·저장/재시작은 별도다. 현재35/46종·완료 종68사례·11종 잔여, 고정43개 중7개 완료·36개 잔여다. 앞선 Room/Level/Tone 결정 시점의 집계는 이력으로 보존한다.

## VU Meter 대표2/2의 실제 적용

서로 다른 issued index1 Cosy Master와 index2 Fast Detection을 각39요청으로 수용했다. 한 load로 쓰기3개와 실제 관측한 readonly8개를 포함한 전체11 current/metadata/unknown을 정확 복원했고 새131바이트 A/B 동일·수명/Root정리를 확인했다. readonly 예외는 쓰지 않았지만 직렬화 참여/제외는 여전히 미검증이다. 표시이름의 원래 복원은 false다. [두 번째 증거](evidence/2026-11-14-vu-preset-index2-native47.json)가 첫 불변 증거를 연결한다. 현재36/46종·완료 종70사례·10종 잔여, 고정8/43 완료·35개 잔여다. 기존 첫 사례1/2 기록은 이력이며 반복하지 않는다.

## Mojito의 정확 비교 실패는 수용하지 않음

Native47 R2는 실제 격리와 프리셋 변화20개를 확인했지만 한 Aload 뒤 쓰기 LFO speed2개가 normalized bits3BA3838F에서3BA3838E로 달랐다. 표시5.00 Hz 일치는 정확 복원 기준을 대신하지 않는다. readonly lfoBeats 차이도 별도 보존한다.30요청·706.132초 뒤 추가 보정/B캡처/해제 없이 저장하지 않고 정리했다. [실패 증거](evidence/2026-11-14-mojito-preset-index0-native47-r2-failed.json)의 수용0을 유지한다. 기존 하이브리드 경로를 검토하더라도 새 실행 전 선언해야 하며 R2를 사후 합격 처리하지 않는다. 현재36/46종·70사례와 고정8/43 완료는 그대로다.

## Mojito의 사전 선언한 하이브리드 대표2/2 수용

Acid Dreams index0과 Basslead index1은 서로 다른 새 문맥에서 각57요청으로 수용했다. 한 Aload 뒤 사전에 고정한 두 LFO 속도만 정밀 setter1회씩으로 보정했고, 쓰기35개 모든 현재값 carrier와 전체37 비값/unknown metadata가 정확히 원래로 돌아왔다. 독립 full37 쌍은 첫 보정의 peer 보존부터 마지막 수명 종료 뒤까지 확인했다. 별도 fresh895바이트 A/B의 source/serialization/SHA256 일치와 RootNoSave/owner0/저장6/MIDI0도 검증했다.

이 결과는 capsule-only 복원이 아니다. readonly lfoBeats의 마지막 native current7/최초3와 전체37 current 원복false, 표시 이름 원복false, legacy pan provider 미확정을 남긴다. 기존 실패R1/R2는 사후 합격 처리하지 않는다. [두 번째 증거](evidence/2026-11-14-mojito-preset-index1-native47.json)가 첫 성공 사례를 연결한다. 현재37/46종·72사례·9종 잔여, 별도 부분0과 고정9/43 완료·34잔여다.

## Repeater의 첫 capsule-only 설정 사례

issued index0 80s Dream을75요청·365.674초로 수용했다. 실제15개 쓰기값 변화 뒤 전체141개 writable current와 모든 비값/unknown metadata가 한 Aload로 정확히 원래로 돌아왔다. 세 페이지64/64/13을 합친 독립 전체 관측14회/7쌍과 별도 새2989바이트 A/B 일치, 수명·Root 정리를 확인했다. scalar seed·보정·readonly 예외·허용 오차는0이다. 각 페이지 complete=false를 전체 provider 완전성으로 바꾸지 않고 모든141개 인덱스를 모아 대조했다.

[첫 실제 증거](evidence/2026-11-14-repeater-preset-index0-native47.json)는 대표1/2만 수용한다. 표시이름 원복false와 실제 MIDI 처리·오디오·저장 미검증을 남긴다. 두 번째 서로 다른 issued entry가 필요하므로 완료 종37/46·완료 종72사례·잔여9종은 그대로다. 별도 부분 Repeater1사례를 더한 총 수용은73사례이며 고정9/43 완료·34잔여도 유지한다. 앞선 부분0 집계는 Mojito 완료 당시의 이력이다.

## Repeater의 서로 다른 capsule-only 대표2/2 수용

새 문맥의 issued index1 Analog Machine을75요청·299.143초로 수용했다. 쓰기34개의 실제 변화 뒤 전체141개 writable current와 비값/unknown metadata를 한 Aload로 정확히 복원했다. 독립 전체 관측14회/7쌍과 새2989바이트 A/B 일치·수명·Root NoSave/owner0/저장6/MIDI0을 확인했다. scalar seed·보정·readonly 예외·허용 오차는0이다.

[두 번째 증거](evidence/2026-11-14-repeater-preset-index1-native47.json)가 첫 index0의 증거를 연결한다. 서로 다른 index0/1의 대표 설정2/2와 PRESET-REPEATER를 완료했다. 표시이름 원복false와 실제 MIDI 처리·오디오·저장 미검증은 유지한다. Repeater의 MIDI 역할은 별도다.

현재38/46종·완료 종74사례·잔여8종이며 별도 부분 사례0이다. capsule-only29종57사례, 하이브리드6종12사례, 명시적 설정scope3종5사례다. 고정43개 중10개 완료·33잔여이며 현재값 이름6866/7112·246잔여는 그대로다. 앞선 Repeater1/2와37/46 집계는 첫 사례 당시의 이력으로 보존한다.

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

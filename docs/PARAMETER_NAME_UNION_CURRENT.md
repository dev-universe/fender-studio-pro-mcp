# 현재 파라미터 이름 집계

고정된7,112개 쓰기 가능 이름 중 **6,619개**의 현재값 변경·읽기·복원 근거를 확인했다. 남은 이름은 **493개·19개 클래스**다. 전체 플러그인 지원률은 아니다.

이전6,460개는 [입력60의 출력128개를 반영한6,459개 기준](PARAMETER_NAME_UNION_128_2026-11-13.md)에 [Vocal Tune bypass 한 개](evidence/2026-11-13-vocal-bypass-value.json)를 더한 집합이다. 그 근거와 실패 기록은 보존한다.

이번에는 [Multi Instrument150개 이름 delta](evidence/2026-11-13-multi150-name-delta.json)를 중복 없이 추가했다. Float137개·Toggle8개·Integer5개를8개 한정 실제 단위에서 검증했고,3752개 성공 요청·300개 setter의 영수증을 정규(class UID,dsp,이름)150개에 연결했다. 각 변경과 복원 뒤 전체150개 값과 나머지149개·메타데이터를 대조했다. control128의 원래값5000도 정확히 복원했다.

독립 실제 감사들과 별도 이름delta 검토가 통과했다. 이름 집계 때문에 새 앱 sweep을 실행하지 않았다. 원래 pilot들의strict_increment0은 바꾸지 않고 별도 delta로만150개를 수용했다. 모델·프리셋·매크로 연결·MIDI·콜백·중첩 그래프·오디오·저장과 하드웨어는 별도 완료 기준을 따른다.

추가로 [Chorder guiInput60 한 이름 delta](evidence/2026-11-13-chorder-guiinput60-name-delta.json)를 수용했다. 실제173개 요청의0→5000→0 및 전체908/저장128 원본복원·정리 근거를 기존6610집합과 대조해 중복0인 정확한 canonical 이름1개만 더했다. 다른58개 클래스 partition은 같다. 원래 실행기의strict_increment0은 보존하며 새 실제 sweep은0이다. Chorder 기능 전체·MIDI 음정 매핑·noteoff·오디오·지속저장은 여전히 별도다.

[Chorder canonical1..8의8개 이름 delta](evidence/2026-11-13-chorder-guiinput1-8-name-delta.json)를추가수용했다. 실제915원장/부모915·16setter·48전체908/48저장128·NoSave/owner0/저장6보존을정확한8tuple에연결했고기존6611과중복0이다. Chorder잔여이름124개,다른58클래스partition은같다. 원래실행기strict0을바꾸거나기존실기를다시돌리지않았다. 표시연동은MIDI음정매핑·실제noteoff·오디오의증명이아니다.

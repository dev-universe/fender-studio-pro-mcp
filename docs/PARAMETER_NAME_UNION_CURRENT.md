# 현재 파라미터 이름 집계

고정된7,112개 쓰기 가능 이름 중 **6,709개**의 현재값 변경·읽기·복원 근거를 확인했다. 남은 이름은 **403개·19개 클래스**다. 전체 플러그인 지원률은 아니다.

이전6,460개는 [입력60의 출력128개를 반영한6,459개 기준](PARAMETER_NAME_UNION_128_2026-11-13.md)에 [Vocal Tune bypass 한 개](evidence/2026-11-13-vocal-bypass-value.json)를 더한 집합이다. 그 근거와 실패 기록은 보존한다.

이번에는 [Multi Instrument150개 이름 delta](evidence/2026-11-13-multi150-name-delta.json)를 중복 없이 추가했다. Float137개·Toggle8개·Integer5개를8개 한정 실제 단위에서 검증했고,3752개 성공 요청·300개 setter의 영수증을 정규(class UID,dsp,이름)150개에 연결했다. 각 변경과 복원 뒤 전체150개 값과 나머지149개·메타데이터를 대조했다. control128의 원래값5000도 정확히 복원했다.

독립 실제 감사들과 별도 이름delta 검토가 통과했다. 이름 집계 때문에 새 앱 sweep을 실행하지 않았다. 원래 pilot들의strict_increment0은 바꾸지 않고 별도 delta로만150개를 수용했다. 모델·프리셋·매크로 연결·MIDI·콜백·중첩 그래프·오디오·저장과 하드웨어는 별도 완료 기준을 따른다.

추가로 [Chorder guiInput60 한 이름 delta](evidence/2026-11-13-chorder-guiinput60-name-delta.json)를 수용했다. 실제173개 요청의0→5000→0 및 전체908/저장128 원본복원·정리 근거를 기존6610집합과 대조해 중복0인 정확한 canonical 이름1개만 더했다. 다른58개 클래스 partition은 같다. 원래 실행기의strict_increment0은 보존하며 새 실제 sweep은0이다. Chorder 기능 전체·MIDI 음정 매핑·noteoff·오디오·지속저장은 여전히 별도다.

[Chorder canonical1..8의8개 이름 delta](evidence/2026-11-13-chorder-guiinput1-8-name-delta.json)를추가수용했다. 실제915원장/부모915·16setter·48전체908/48저장128·NoSave/owner0/저장6보존을정확한8tuple에연결했고기존6611과중복0이다. Chorder잔여이름124개,다른58클래스partition은같다. 원래실행기strict0을바꾸거나기존실기를다시돌리지않았다. 표시연동은MIDI음정매핑·실제noteoff·오디오의증명이아니다.

[Chorder canonical9~16의8개 이름 delta](evidence/2026-11-13-chorder-guiinput9-16-name-delta.json)도 수용했다. 실제915개 요청과16번 변경·전체 복원·종료 확인을 기존6619집합과 대조했다. 정확한8개만 중복 없이 추가해6627/7112·잔여485개가 됐다. Chorder 잔여 이름은116개이며 다른58클래스의 분할은 같다. 원래 실행기의 이름 증가0은 유지했고 이름 집계 때문에 새 실기를 실행하지 않았다.

[Chorder canonical17~24의8개 이름 delta](evidence/2026-11-14-chorder-guiinput17-24-name-delta.json)를 수용했다. 실제915개 요청·변경16회·전체908/저장128 복원과 정상 종료를 기존6627집합에 연결했다. 정확한8개만 중복 없이 추가해6635/7112·잔여477개, Chorder 잔여108개다. 다른58클래스와 원래 실행기의 이름 증가0은 그대로다. 새 실기 없이 불변 근거를 별도로 수용했다.

[Chorder canonical25~32의8개 이름 delta](evidence/2026-11-14-chorder-guiinput25-32-name-delta.json)를 수용했다. 실제915개 요청·변경16회·전체908/저장128 복원과 정상 종료를 기존6635집합에 연결했다. 정확한8개만 중복 없이 추가해6643/7112·잔여469개, Chorder 잔여100개다. 다른58클래스와 원래 실행기의 이름 증가0은 그대로이며 새 실기는0회다.

[Chorder canonical33~40의8개 이름 delta](evidence/2026-11-14-chorder-guiinput33-40-name-delta.json)를 수용했다. 실제915개 요청·변경16회·전체908/저장128 복원과 정상 종료를 기존6643집합에 연결했다. 정확한8개만 중복 없이 추가해6651/7112·잔여461개, Chorder 잔여92개다. 다른58클래스와 원래 실행기의 이름 증가0은 그대로이며 새 실기는0회다.

[Chorder canonical41~48의8개 이름 delta](evidence/2026-11-14-chorder-guiinput41-48-name-delta.json)를 수용했다. 실제915개 요청·변경16회·전체908/저장128 복원과 정상 종료를 기존6651집합에 연결했다. 정확한8개만 중복 없이 추가해6659/7112·잔여453개, Chorder 잔여84개다. 다른58클래스와 원래 실행기의 이름 증가0은 그대로이며 새 실기는0회다.

[Input Filter 첫 guiInput 이름 delta](evidence/2026-11-14-inputfilter-guiinput1-name-delta.json)를 수용했다. 실제99개 요청·변경2회·전체388복원과 정상 종료를 기존6659집합에 연결했다. 정확한keyboard.pitch1.guiInput 한 개만 중복 없이 추가해6660/7112·잔여452개, Input Filter 잔여127개다. 다른58클래스와 원래 실행기의 이름 증가0은 그대로다. 이름 집계를 위한 새 실기는0회이며 MIDI/음정 대응·오디오·전체 기능 완료로 확대하지 않는다.

[Input Filter canonical2~9의8개 이름 delta](evidence/2026-11-14-inputfilter-guiinput2-9-name-delta.json)를 수용했다. 실제481.483초·659요청·변경16회·전체388개48묶음/336페이지와 정상 정리를 정확한8이름에 연결해6668/7112·잔여444개가 됐다. 다른58클래스 분할은 같고 원래 수집기의 이름 증가0도 보존했다.

[Chorder canonical49의1개 이름 delta](evidence/2026-11-14-chorder-guiinput49-name-delta.json)도 별도 수용했다. 실제393.829초·131요청·변경2회·전체908/저장128 복원과 사전에 예측한6개 읽기전용 반응을 확인했다. 정확한1이름만 더해6669/7112·잔여443개, Chorder83개·Input Filter119개 잔여다. 다른58클래스는 같다. 원래 실패batch07은완료0으로보존하며 일반MIDI/모델대응·noteoff·audio·전체Chorder완료는주장하지않는다.

[Input Filter canonical10~17의8개 이름 delta](evidence/2026-11-14-inputfilter-guiinput10-17-name-delta.json)를 수용했다. 실제372.987초·659원장/부모요청·변경16회·전체388개48묶음/336페이지·정확복원과Root정리를735개입력핀및기존6669집합에연결했다. 정확8이름만추가해6677/7112·잔여435개, Input Filter111개·Chorder83개잔여다. 다른58클래스는같고원래수집기이름증가0과실패R1완료0은유지한다. MIDI/음정대응·noteoff·audio·전체기능완료는별도다.

[Chorder canonical50~57의8개 이름 delta](evidence/2026-11-14-chorder-guiinput50-57-name-delta.json)를 수용했다. 실제786.797초·915원장/부모요청·변경16회·48전체908/48저장128·각6RO예측/다른901와unknown복원·Root정리를927입력핀과정확8tuple에연결했다. 기존6677집합과중복없이6685/7112·잔여427개, Chorder75개/Input Filter111개잔여다. 다른58클래스와원래수집기이름증가0은그대로다. 이름집계의새실기0이며일반MIDI/모델대응·noteoff·audio·전체플러그인완료는별도다.

[Input Filter canonical18~25의8개 이름 delta](evidence/2026-11-14-inputfilter-guiinput18-25-name-delta.json)를 수용했다. 실제501.314초·659원장/부모요청·파라미터 변경16회·전체388개48묶음/336페이지와 정확한 복원 및 정상 정리 근거를 기존6685집합에 연결했다. 정확한8개 이름만 중복 없이 추가해6693/7112·잔여419개가 됐다. Input Filter103개와 Chorder75개가 남으며 다른58클래스의 분할은 같다. 원래 수집기의 이름 증가0과 기존 실패 기록은 보존한다. 이름 집계를 위한 새 실제 호출은0회이며 MIDI·음정 대응, noteoff·오디오와 플러그인 전체 완료는 별도다.

[Chorder canonical58·59·61~66의8개 이름 delta](evidence/2026-11-14-chorder-guiinput58-66-name-delta.json)를 수용했다. 실제836.113초·915원장/부모요청·파라미터 변경16회·전체908개와 저장 모델128개 각48묶음/384페이지로 검증했다. 각 사례의 예측한6개 읽기 전용 반응, 다른901개 값과 비값·알 수 없는 필드 보존, 전체 복원과 Root 정리를 확인했다. 927개 입력 핀과 정확한8개 tuple을 기존6693집합에 중복 없이 연결해6701/7112·잔여411개로 수용했다. Chorder67개·Input Filter103개가 남으며 다른58클래스 분할은 같다. 완료한60은 재실행하지 않았다. 원래 수집기의 이름 증가0과 기존 실패 기록을 보존한다. 이름 집계를 위한 새 실제 호출0회이며 일반 MIDI·모델 대응, noteoff·오디오·지속저장과 플러그인 전체 완료는 별도다.

[Input Filter canonical26~33의8개 이름 delta](evidence/2026-11-14-inputfilter-guiinput26-33-name-delta.json)를 수용했다. 실제524.306초·659원장/부모요청·파라미터 변경16회·전체388개48묶음/336페이지와 정확한 복원 및 Root 정리를 확인했다. 같은 실행의744개 입력 핀과 정확한8개 tuple을 기존6701집합에 중복 없이 연결해6709/7112·잔여403개로 수용했다. Input Filter95개와 Chorder67개가 남으며 다른58클래스 분할은 같다. 원래 수집기의 이름 증가0과 기존 실패 기록을 보존한다. 이름 집계를 위한 새 실제 호출은0회이며 MIDI·음정 대응, noteoff·오디오·지속저장과 플러그인 전체 완료는 별도다.

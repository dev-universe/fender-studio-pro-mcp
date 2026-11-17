# 중첩 그래프 소유 관계 조회 구현 계획

상태는 소스 구현·독립 검토·정상 설치와 실제8.1.1의 제한된 직접 소유 관계 R3 검증 완료다. 일반 그래프 편집 완료를 뜻하지 않는다. 현재 설치 제품은 Native48 /2db35b2 /MCP46개다. 준비 오류 R1/R2를 실패로 보존하며 그 두 단위의 graph 조회는0이다. 새 문맥의 R3는32요청·348.714초로 통과했다.

## 이번 변경의 결과

기존 `plugin_inspect`의 `include_nodes:true`, `node_scope:"graph"` 요청에 `node_report.ownership_manifest`를 추가한다. 새 도구·요청 필드·영구 그래프 핸들은 만들지 않는다. 기존 부분 DFS 보고와 별개로 Graph의 모든 직접 자식과 고정 Slot→Device 대응을 읽는다. 조회마다 현재 문서·대상 인스턴스·Inserts 폴더와 소유 객체를 새로 검증하고 응답 확정까지 유지한다.

직접 자식은 최대32개, 고정 경로의 retained 객체는 최대97개다. 자식 순서는 CCL 열거 순서다. 오디오 연결·DSP 순서·원자적 스냅샷·변경 권한은 입증하지 않는다. 범위는 현재 확인한 primary FX Inserts 계열이며 Multi Instrument의 다른 그래프 계열은 별도다.

## 선택한 구조와 검토 결과

명시적 expected_folder/expected_graph 필드를 추가하는 안과 기존 조회 요청에서 새 관측을 자동으로 만드는 안을 비교했다. 기존 요청의 문서·인스턴스와 내부 소유 관계 검증으로 현재 조회를 제한할 수 있어 후자를 선택했다. 이후 변경 API는 별도의 최신 대상·그래프 사전조건을 요구해야 한다.

독립 검토에서 두 가지 필수 보완을 확인했다. 전체 상속 타입·메서드 목록을 모으면 기존64개 타입 예산을 넘을 수 있으므로 이번 경로는 식별자와 첫 native class만 읽고 생략된 reflection 범위를 명시한다. 기존 전역 한도를 늘리지 않는다. 또한 없던 Slot이 생기는 변화도 응답 직전까지 감지하도록 부재 관측을 유지하고 다시 대조한다. 마지막 문맥 검증 실패 시 전체 graph node_report를 제거한다.

Graph·직접 자식의 class UID는 없을 수 있으나 object UID는 필요하다. Slot UID는 없을 수 있으며 retained canonical identity와 고정 delegate가 근거다. Device의 object/class UID는 필요하다. Slot의 부모를 SlotNode로 추측하지 않는다. 알 수 없는 종류·누락된 대응·같은 개수에서의 순서 변경·객체 교체는 완전한 보고로 내보내지 않는다.

## 검증 조건

- 32개 자식 각각에 Slot과 Device가 있는97개 객체 사례,33개 자식 거부, 마지막 형제 누락 방지를 검사한다.
- 순서·객체 교체, 중복, 부재 Slot의 생성, Device 누락, 식별자·종류 오류, 재진입·스레드·문맥 변경과 해제 도중 변화를 검사한다.
- 성공·실패 경로의 참조 수명과 실제 Reply 무효화를 검사한다. 기존 DFS·폴더·다른 조회의 한도와 동작을 보존한다.
- 새 이름의 스크래치 사본에서 실제8.1.1 Routing 화면, 독립 전체 manifest, 두 Mixtool의 전체72개 provider 상태·메타데이터와 음소거·정지 문맥을 대조한다. 요청 수는 설치본과 최종 응답 형태 검토 후 고정하며, 시작·문맥·부정 대조 요청도 모두 센다.
- topology 변경·프리셋·capsule load 없이 확인하고 NoSave·owner 종료·저장본 보존·물리 장치 분리 상태로 마친다. 기존142개 writable 변경·Splitter42·전체 chain 왕복은 반복하지 않는다.

## Ghidra 조사에서 확인한 것과 남은 것

같은8.1.1 arm64 바이너리에서39개 정적 분석 기록을 보존했다. createFunctions 디스패치, AudioFXGraphEditFunctions의 두 별도 인터페이스와 this 조정, insertDevice/connectChannel/colorizeChannel 경로, beginMultiple/endMultiple과 journal·대상 참조 수명 경로를 확인했다. 예전 스냅샷은 현재 분석 프로필에서 거부돼 원본 바이너리를 새로 분석했으며 새 스냅샷과 전체 증거를 비공개로 저장한 뒤 엔진을 닫았다.

이는 실제 호출 성공이 아니다. 정확한 편집 객체 획득과 대상 연결, 각 변경 인자의 ABI·오류·완료 방식, 독립적인 오디오 포트·연결 열거는 더 확인해야 한다. 일반 create/connect/reorder/delete는 계속 미완료다. 정적 데이터 영역의 읽기0바이트 결과를 객체나 기능의 부재로 해석하지 않는다.

이 준비로 완료 수를 늘리지 않는다. 현재 고정43개 중10개 완료·33개 잔여, 프리셋38/46종·74사례, 현재값 이름6866/7112개·246개 잔여다. 하드웨어 대기4개와 전체 종료 ETA 미정도 유지한다.

## 소스 검토에서 보완한 수명 검증

마지막 그래프 연결 확인 뒤 외부 편집기 provider 검사가 새 객체를 만들고 해제할 수 있던 경로를 분리했다. 전체 provider 검사는 마지막 폴더 메타데이터 정리 뒤에 마치며, 이후에는 유지한 객체의 borrowed 연결과 문서·대상만 확인한다. 실제 Release 콜백으로 Slot 생성과 provider 연결 소실을 모사한 검사를 추가했고, 옛 검증 순서에서 잘못된 성공도 재현했다. 전체 graph 보고 제거와2초 기한을 유지한다. 이는 오프라인 수명 증거이며 실제 External Effect 편집기나 호스트 지연 검증을 대신하지 않는다.

## 실제8.1.1 R3 결과

직접자식6개는 Splitter2개·Slot2개·Merger2개이며, 두 Mixtool의 새 인스턴스가 정확히 한 번씩 대응됐다. 독립 manifest2회와 마지막1회의 전체 필드가 일치했다. 기존 DFS complete=false는 그대로다. 두 장치 각각72개 중71개 writable과readonly stereo1개를 읽었고, 초기·조회뒤·마지막의 독립6쌍 전체 값과 알려지지 않은 metadata까지 정확히 일치했다. 실제 Routing 위·아래 화면도 전후 대조했다. 마지막의 가짜 expected_instance는 예상한 not_applied로 거부됐고 이후 RPC 없이 NoSave로 닫았다.

Main0 격리1회 외 대상 setter·topology 변경·preset·load0이다. 앱 종료·owner0·두 잠금의 해제·저장10개·MIDI0/0을 확인했다. [실기 증거](evidence/2026-11-14-graph-ownership-native48-r3.json)는6개 자식의 이 경로에 한정한다. 오디오 포트/연결, 일반 create/connect/reorder/delete와 Multi Instrument의 다른 graph 계열은 남아 있다. 고정10/43개 완료·33잔여와 프리셋38/46종은 그대로다.

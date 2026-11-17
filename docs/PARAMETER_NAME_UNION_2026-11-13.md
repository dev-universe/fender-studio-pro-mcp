# 고정 파라미터 이름 합집합 감사

과거의 고정 목록 7,112개 이름 중 6,423개가 검증 합집합에 포함되고 689개가 남는다.
이 수치는 Chorder 출력 역할 92개까지 완료했던 경계다. 뒤에 완료한 역할은 포함하지 않는다.
전체 플러그인 지원률이나 현재 가능한 모든 모델·그래프·이벤트 상태의 비율을 뜻하지 않는다.

중복 제거 키는 native class UID, source, canonical parameter name이다.
과거 59개 class/source 쌍, 8,668개 record에서 readOnly=false, writable=true인 이름만 분모로 썼다.
현재값 검증, 선택한 pad0 alias, MIDI 역할, 입력60의 Chorder 모델 매핑 등 증거 방법은 서로 다르다.
프리셋 차이, 반복 읽기, callback 수, 다른 인스턴스와 조합 수를 이름 수에 더하지 않았다.

기존 독립 검증 6,288개에 Focus26, Impact MIDI2, Vocal Tune13, Chorder input1·Mode1·member92를 중복 없이 더했다.
실패한 원래 출력60은 제외하며, 옛 청크8의 출력57~59는 각자 완료한 증거만 사용했다.
각 member는 전체908개 파라미터와128개 모델의 before/changed/final 쌍, 선택한 실제 변경 영수증과 원본581바이트 capture 복원을 대조했다.
독립 검토는 전체 순수 감사 재실행, 16개 검사 그룹, raw 표본529개와 입력13,512개의 해시 보존을 확인했다.
새 앱 요청은0회다. 이전 실제 앱 종료와 원본 보존 증거를 재사용했다.

| 고정 경계의 클래스 | 합집합 밖 이름 |
|---|---:|
|Chorder|169|
|Multi Instrument|150|
|Input Filter|128|
|CV Instrument|97|
|Mustang Native|29|
|Rumble Native|29|
|Presence|22|
|Ampire|19|
|Pedalboard|12|
|Mai Tai|7|
|SampleOne|6|
|External Effect Mono|4|
|External Effect Stereo|4|
|Mojito|3|
|Compressor|2|
|Level Meter|2|
|Scope|2|
|Expander|1|
|Gate|1|
|Limiter|1|
|Vocal Tune|1|

이 목록은 미구현 기능 수와 같지 않다. 이벤트 입력, command, mapping과 일반 scalar는 서로 다른 제어·관찰이 필요하다.
완전한 choices, 소유 receiver, 모든 model/focus 조합, 숨은 상태, persistence, 물리 경로와 오디오는 별도 검증이다.
이후 합집합을 갱신할 때는 새 역할의 증거를 명시적으로 추가하고 검토한다. 쓰기 횟수만 더하지 않는다.

정량 증거와 검토 해시는 [JSON 증거](evidence/2026-11-13-name-union.json)에 보존했다.
현재 실행과 더 최신 역할 수는 [재개 상태](RESUME_STATE.json), [Chorder 기록](CHORDER_MEMBERSHIP_2026-11-13.md)을 우선한다.

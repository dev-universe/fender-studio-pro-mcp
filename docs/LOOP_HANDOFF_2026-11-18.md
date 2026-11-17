# 2026-11-18 공개 준비와 CV 후속 작업 인수인계

이 문서는 이번 11월18일 재개 이후의 수행 내용과 남은 대기열을 보존한다. **실제 검증은 종료했으며 공개 main의 검증본을 복원했다.** 이번 사용자 마감은 오전6시 한국 시간이다. 이전 실행의 오후6시 마감을 재사용하지 않는다.

## 1. 다음 재개에서 먼저 읽을 내용

사용자가 `진행 재개해`라고 하면 저장소 AGENTS.md, `docs/RESUME.md`, 이 문서, `docs/RESUME_STATE.json`의 current_execution과 next_units_in_order를 읽는다. 공개 README와 진행 기록의 완료 수를 소스 검사 수로 환산하지 않는다. 제품 소스·설치·프로세스·잠금·저장본·하드웨어를 새로 확인하고 새 문서/세션/인스턴스를 바인딩한다.

지금 가장 앞의 미완료 문제는 **CV 출력 경로의 구조형 자식 관찰을 실행하기 전, CV 채널 mute1 요청이0으로 남은 이유**다. Native60 lifecycle을 실제로 호출하지 못했다. 이전 세션의 요청을 다시 보내거나 기대값을 완화하지 않는다. R84에서는 GUI0→1/API1/1을 수용했지만 R85의 새 GUI 한 번은0에 남고 보이는 FX도 음소거 해제되어 다시 중단했다. 두 관측을 함께 보존하고 그룹·연결 상태와 입력/조작 문맥을 먼저 구분한다. 원인은 확정하지 못했다.

완료한 Native59 R81의 구조형 인터페이스 존재 조회, 공개 cleanup/count/get 함수 분석, 이미 수용한 전체 파라미터·Chorder·Vocal·프리셋 검사를 다시 처음부터 수행하지 않는다. 마지막 전체 CV 실앱 수용과 최종 복원 설치는 Native59다. Native60 소스·빌드·정상 설치 증거는 후보로 별도 보존한다.

## 2. 사용자 결정과 실행 경계

- macOS Apple Silicon, Fender Studio Pro8.1.1 Build113103을 우선한다. MCP 서버는 네이티브 Rust이고 JavaScript는 DAW 내부 bridge/EditTask에서만 사용한다.
- Volt276과 Keystation88 MK3는 이번 실행에서 USB로 연결했다. 루프백 배선은 없다. 실제 연주·오디오 왕복·외부 효과 ping을 장치 인식만으로 완료 처리하지 않는다.
- 오전5시30분 이후 새 실제 단위를 시작하지 않는다. 앱·owner·분석 엔진 정리는5시45분, 문서·Git 정리는6시까지 마친다. 다음 재개의 새 마감으로 이 시각을 자동 재사용하지 않는다.
- 사용자 노래와 설정을 보존한다. 실제 검증은 소유한 named Scratch에서만 하고 원본 저장은 하지 않는다. 실패 뒤 추가 요청·재전송·자동 역변경·계획하지 않은 state load/release를 하지 않는다.
- 공식 설치 파일 대조는 사용자가 생략했다. MyFender 로그인이나 설치 경로를 다시 요청하지 않는다. 기존 서명 문제를 해결했다고 주장하지 않는다. Hopper를 사용하지 않으며 필요한 정적 조사는 Ghidra/REA의 기존 고정 증거를 재사용한다.
- 사용자 보고는 한국어, 내부 지시는 한국어 또는 영어다. branch와 PR 제목에 codex를 쓰지 않는다. 원본 곡·오디오·앱 로그·개인 경로·독점 리소스를 Git에 올리지 않는다.
- 기존 범위 안의 구현·검증·공개 저장소 병합은 사용자에게 이미 허가받았다. 새 토큰·보안 권한을 만들지 않고 현재 도구와 기존 인증을 사용한다. 이번 공개 요청은 과거 PR4 Draft 유지 제한을 해당 검토·출시 범위에서 해제했다.
- 각 완료 단위에 체크리스트, 실제 경과, 남은 작업 수, 다음 유한 단위 예상과 루프 종료까지 시간을 보고한다. 전체 연구 완료 예상은 미정이다.

## 3. 공개 저장소 준비와 main 출시 완료

사용자가 가장 먼저 요청한 공개 저장소의 기본 보호, 검증 코드의 main 병합과 음악인용 README 첫 줄을 완료했다.

| 항목 | 확인한 결과 |
| --- | --- |
| 저장소 공개/관리자 | 공개 유지, 기존 단독 관리자 권한 유지 |
| main 보호 | PR 필수, 관리자에도 적용, 강제 push/삭제 불가, 최신 main 기준 필수 검사2개, 대화 해결 필수 |
| 필수 승인 | 단독 관리자 저장소라0명, 대신 독립 검증 결과를 PR에 남김 |
| Actions | 기본 토큰 읽기, PR 승인 불가, GitHub 소유 Actions만 허용, 모든 외부 기여자 실행 승인 필요 |
| 보안 | 비밀정보 검사/푸시 보호 유지, 취약점 알림/보안 수정/비공개 제보 활성화 |
| 병합 후 branch | 자동 삭제 활성화 |
| 기여/보안 안내 | CONTRIBUTING.md, SECURITY.md, CODEOWNERS 추가 |
| README 첫 줄 | **그래서 이게 뭔가요?** Fender Studio Pro에서 트랙과 이펙트를 AI에게 말로 부탁해 조작할 수 있게 해 주는 연결 도구입니다. |

설정을 변경한 뒤 원격 API에서 다시 조회했다. 기존 권한을 확장하거나 자격증명을 생성하지 않았다. GitHub connector의 쓰기403과 gh의 workflow 권한 부재를 확인했으므로 workflow 게시에만 이미 로그인된 브라우저를 사용했다.

[PR4](https://github.com/dev-universe/fender-studio-pro-mcp/pull/4)는 독립 PASS+NOTES와 필수 검사2개 통과 뒤 squash 병합했다. main은 `15304714c14cf8d765dd0acb08dbb59463924129`다. 검토한 head `7ab2f230282067f141f38e81363f33dcb5343b14`와 tree `4cea21a8b300b88274f0ac49826f9c84ad0f5309`가 같다. 병합은 원격 merged 상태와 main base로 다시 확인했고 메인 push 검사2개도 통과했다. 로그인 없는 README 조회가 HTTP200이며 한국어 첫 줄 바이트가 main과 같다. 사용자는 main 소스를 빌드해 사용할 수 있고, 빌드·bridge·장치 설정 순서는 README에 있다. 완성된 전체 플러그인 제어 제품으로 표시하지 않는다.

PR4에서는 일반 변경 요청의 문서 토큰 사전조건 누락을 고쳤다. 공통 runtime과 CLI의 owner 시작 이전에 누락·공백 토큰을 거절한다. 요청/세션/문서/인스턴스와 프로세스 간 잠금 조건을 유지한다. CLI21사례와 runtime의 stale/matching 조건을 독립 검증했다. ZIP을9.6.1로 고정하고 deflate-only와 Rust1.88을 유지했다. 이름 API의 오류 전파2곳만 맞췄고 GitHub 보안 경고4개는 모두 fixed로 확인했다. 네이티브 테스트의 문자열 포인터2비교를 내용 비교로 바꿨다. 실제 제한시간을 늘리지 않았고 당시 debug 제한시간 초과 기록도 남겼다.

CI는 Rust1.88, Node24, 고정 SHA의 checkout/setup-node와 contents:read를 사용한다. root fmt/clippy/tests/release, JS/Python 검사와 macOS arm64 라이브러리·최적화 직렬 addon·설치하지 않는 패키지 빌드를 수행한다. PR4의 root183, addon448, 라이브러리289와 doctest20 등의 통과는 실앱 전체 기능 인증이 아니다.

공개 근거는 `docs/evidence/2026-11-18-public-main-preview.json`에 있다. 원본 설정·원격 응답·개인 장치 ID는 비공개로 보관한다.

## 4. 이번 실행의 장치와 공개 main 실제 확인

초기 OS 조회에서 Volt276 입력2/출력2,48kHz/512frames와 Keystation의 일반·Transport MIDI 입력을 관측했다. 유한 passive 수신5초에서는 이벤트0개, 연결 해제0개, 정상 dispose를 관측했다. MIDI 송신·전달은0이며 실제 건반 연주를 입증하지 않는다. 이 CLI 묶음 전체는5.885초다.

공개 main의 새 CLI로 R82 context 한 번을 읽었다. 결과는 observed, 실제 요청 약0.06초이며 UI 준비 전체의 별도 시간은 기록하지 않았다. 앱은 NoSave로 종료했고 owner child의 명시적 exit0을 기다렸다. 저장본10개와 기존 native/bridge12파일을 보존했다. 후처리 검사기의 SHA 문자열/object 형식 차이를 고쳤으며 추가 DAQ는 없었다. 이 세션은 닫힌 이력이며 후속 runtime에 재사용하지 않는다.

이번 실행에서 CoreAudio.settings와 MusicDevices.settings를 보존했다. Studio Pro.settings 등 일반 UI 상태는 앱을 여닫으며 바뀔 수 있으므로 모든 profile 바이트가 같았다고 주장하지 않는다. DAW는 원래 MacBook Pro 오디오 설정을 유지했으며 OS의 Volt 연결 상태를 DAW 엔진 선택 증거로 바꾸지 않는다.

## 5. CV 구조형 파라미터 정적 조사와 설계

기존 Native59는 Output01 등7행에서 IStructuredParameter 지원을 관측했지만 준비/정리/자식 열거는 실행하지 않았다. 이번 정적 r2는 새 함수 본문3개와 약278.735초, r3는 참조6개·새 본문0개와234.897초다. 공용 child-list 구현은 구체 파라미터의+90 위치이고 내부 prepare/cleanup 캐시는+180 위치로 서로 다르다. 공개 소비자는 부모 retain, prepare, borrowed child count/get, cleanup 계약을 사용한다. 알 수 없는 owner IID의 이름을 임의로 추정하지 않는다.

설계 r4의 service_active=false를 객체 수명 종료로 본 부분은 잘못이었다. 정정 r6는 addon admission 중단과 CCL retained object 수명을 구분한다. 기존 public IUnknown retain/release와 원래 객체 cleanup을 사용하며 가상의 격리 수명 체계를 더하지 않는다. 서비스 중단이나 문맥 소실 뒤 응답은 unknown으로 처리한다. 프로세스 종료나 반환하지 않는 host callback의 정확히1회 cleanup을 보장한다고 주장하지 않는다. 원래 r4와 정정 r6는 별개로 남긴다.

공개 prepare/cleanup은 캐시 객체와 콜백을 바꾸므로 단순 읽기로 분류하지 않는다. 정적 결과로 현재 배선·None 보존·선택지 적용 또는 원상복구를 보장하지 않는다.

## 6. Native60 소스 후보와 검증 범위

[PR6](https://github.com/dev-universe/fender-studio-pro-mcp/pull/6)는 `feature/cv-structured-lifecycle`의 Draft다. 제품 소스 기준은 `921f8890597fbfbb20368b06a00a6e460b60345a`다.

새 `plugin_state.inspect_cv_port_structure`는 정확한 CV Instrument instrument class, Output01/index0/portAssignment/tag100 하나만 받는다. action은 변경으로 분류하며 문서·세션·인스턴스를 일치시킨다. 임의 source/port/index/IID/path/depth/slot/channel/token을 허용하지 않는다. 기존 읽기 전용 PluginInspect 동작은 유지한다. 고정 공개 ABI의 prepare/cleanup/count/get을 사용하고1회 prepare/cleanup, 깊이1, 최대32자식, 문자열/응답 크기 상한을 둔다. borrowed child는 즉시 retain하고 두 번째 canonical 관찰과 최종 부모/flags 검사를 거친다. temporary release가 부모 설명자를 바꾸는 경우도 최종 검사에서 거절한다. 준비 전 알려진 거절과 dispatch 후 unknown을 구분하며 `closed_checked`만 RuntimeValidator가 받는다.

자식0개는 유효할 수 있다.32개 초과는 bounded partial이며 이를 전체 열거로 표시하지 않는다. 선택지 handle, 포트 쓰기, 라우팅, 오디오, 저장 지속성 및 atomic snapshot 권한은 모두 없다. public receipt에 실제 관측하지 않은 get counter를 만들어 넣지 않는다.

작성43분46.888초와 독립 검토17분54.274초를 수행했다. 초기 수정본은 늦은 parent descriptor 변경을 놓쳤고 독립 재현으로 실패를 확인했다. 수정본에서 동일 사례가 거절됨을 확인했다. root190, 최적화 addon466, parameter API142와 doctest4, JS389, SDK34hash와 ABI32대조가 통과했다. 의도적으로 guard를 제거한 오프라인3사례의 실패와 원래 복원 후466통과를 확인했다. 이 근거는 실제 host lifecycle 수용이 아니다.

| 소스/검토 근거 | SHA256 |
| --- | --- |
| 소스20개 동결 manifest | d28d56a9876586f6c7b6d0097af6bb4ca78c62263096503a10850cd8104f2767 |
| 독립 R8 보고서 | 5a0672de6478668b4ca3f350cf997a6df0ea67fd474150179ff1940897882047 |
| 독립 ABI 보고서 | bab92aed6d95fb8971080e4e82da1a4eec6033f2835c3475c1e0547993059f8b |

## 7. 정상 빌드·설치·메타데이터와 되돌림 준비

정상 Native60의 CLI 빌드30.69초, native 빌드34.71초가 exit0이다. 관리 install/native-install 및 비실시간 doctor가 설치 일치를 확인했다. 새 MCP owner에 initialize/initialized/tools-list를3번 쓰고 원본2개를 얻었다. 도구46개/상태 동작8개 전체 JSON이 소스 생성 결과와 같다. owner child wait exit0을 명시적으로 확인했다. 준비를 포함한 정상 검증 묶음은1512.672초다.

제품 소스187개, CLI·설치13개와 vendor Main, 저장본10개 및 오디오/MIDI 설정을 고정했다. 새 네이티브 SHA는 `1af793aaaa01af9e5877e3460651884a34d1eaef7e8bed85da2261c347bec06c`, UUID는 `2E958F38-1BBE-3883-9279-7DA1D228887C`다. CLI SHA는 `672ac5e83fbe877377b576c190d121ab329d838ebd513ca1cb9faf7191a272e1`다.

기존 Native59의 관리 대상 파일과 설치 manifest를 되돌림 묶음으로 보존했다. vendor 앱과 사용자 곡을 백업 복사 대상으로 섞지 않았다. 정상 Native60 검증 당시에는 되돌림을 실행하지 않아 설치가 후보60이었다. 최종 종료 때에는15절처럼 공개 main/Native59로 복원했다. 실제 전체 CV 수용은 Native59다. 제품 소스가 바뀌지 않는 문서/CI 변경만으로 다시 빌드·설치하지 않는다.

## 8. R83 실제 계획, preflight 수정과 안전 중단

실제 계획은 metadata3write와 별도로31요청(IDs3~33)이다. 전체110행 중 쓰기 가능한97개 typed current/schema를 baseline/final에 모두 비교하고13읽기 전용 중 voltage2값 차이만 별도 표시한다. 직접 CV7행/56필드는 전후 exact 비교한다. lifecycle은 ID27 한 번이다. 실제 UI 경계는18/20/25/26/28/33이며 원본 CV/Main 쌍을 서로 다른 화면으로 직접 확인한다. 내부 Monitor Volume은 관측 불가로 유지한다.

첫 preflight는 원본 metadata까지만 얻고 실제0건으로 닫았다. 앱 초기화 때 CoreMIDI endpoint_ref가 바뀌어 raw handle 비교가 거절했다. r4는 direction+signed UID와 모든 실제 물리 장치 속성을 비교하고 raw handle 변화는 따로 남긴다. bridge의 제조사/모델/offline 관측 불가는 그대로 유지한다. 현재 hardware39검사와593.953초의 독립 읽기 전용 검토를 통과했다. 옛 r3의31/95/9검사는 역사적 비하드웨어 근거다. 이전 preflight의3→4 snapshot을3→4→3 종료 증거로 재해석하지 않는다.

새 R83-r2는 다른 owner와 metadata/장치/보존 원본을 만들었다. source187/artifact13/saved10와 source metadata 전체 일치를 확인하고 fresh runtime을 바인딩했다. 실제7요청(IDs3~9), guard6개가 통과했다. Main volume1→0은 확인했다. CV Instrument mute1은 before0/after0 및 applied_unverified로 돌아와 즉시 중단했다. target/session/document가 맞아도 변경 효과가 확인되지 않으면 성공으로 세지 않는다. 전체31수용과 lifecycle은0이며 기존97개 보존 주장도 추가하지 않는다.

Root 원본 화면에는 여러 FX mute가1에서0으로 바뀐 모습도 있다. 정확한 발생 사건과 원인은 아직 모른다. 이전 성공 R81과 이번 실패 R83의 대상/값/index55는 같았고 setChannel 구현과 SDK mute 이름도 같다. 새 lifecycle 분기는 호출하지 않았다. MusicDevices 설정에는 bridge만 있고 Keystation/Mackie/HUI mapping이 없어 특정 하드웨어 명령 충돌을 원인으로 확정할 수 없다.

Root가 추가 DAQ 없이 NoSave 종료 후 앱 미실행을 관측했고 바로 CUA completion을 남겼다. owner는 child wait exit0, 종료 요청 강제 없음이다. saved10/source187/art13/vendor Main/오디오MIDI 설정이 모두 같고 잠금2개는 다시 획득·해제했다. 새 하드웨어 triplet은3→4→3으로 확인했다. 이 단위는 시작부터 종료 검사까지162.545초다. 실패 세션은 닫힌 목록에 추가하고 다시 쓰지 않는다.

비공개 Root evidence의 캡처 helper가 처음 옛 경로를 참조해 EEXIST로 거절된 시도도 남긴다. 파일을 덮어쓰지 않고 새 디렉터리 함수를 고친 뒤 원본을 저장했다. metadata receipt 시각 파일의 이름을 실제 guard 규격에 맞췄으며 DAQ 이전이었다. 이러한 도구 수정은 제품 실기 성공 수가 아니다.

## 9. CI 서버 배정 문제와 후속 검토

PR6의 macOS15 검사2회가 각각 runner 미배정·0단계 상태로 capacity 제한에 취소됐다. Core 검사는 별개이며 native 테스트 실패로 해석하지 않는다. 같은 arm64·검사명·내용·읽기 권한을 유지해 표준 macOS26 runner로 옮겼다. 커밋은 `06bcb1bdbf4123d213b9f4c4c943df055d382321`이고 runner 한 줄과 마지막 빈 줄만 바뀌었다. GitHub 공식 문서에서 두 라벨이 무료 공개 표준 M1 arm64 범주임을 확인했다.

독립 후속 검토197.868초에서 product187개가 source921f889/새 head06bcb1b/현재 파일 모두 같음을 확인했다. workflow source는 PASS+NOTES지만 실제 lifecycle0과 CI 대기로 병합 판정은 아니다. 문서가 동시에 바뀌어 초기 전역 clean assertion이 실패했으나 원본을 남기고 제품187개로 좁혀 확인했다. 문서를 제품 변경으로 오인하거나 실패를 숨기지 않는다. 최신 CI 결과는 마지막 종료 절에 추가한다.

## 10. 프리셋 출력 경로 보존 조사에서 유지할 결론

CV 프리셋 readiness r7의 유한 소스 조사는 generic plugin_presets apply가 소스에서 일괄 금지된 것은 아님을 확인했다. AudioSynthSlot mediator, Presets IDataTarget, issued entry와 canInsertData/insertData 경로는 있다. 그러나 현재 출력 경로 보존 gate는 해제되지 않았다. factory Output03의 비어 있지 않은 상태, 발급 payload와 실제 적격성은 미확정이다. 구조형 lifecycle이 성공해도 프리셋을 바로 적용하지 않는다.

프리셋 설정의 유한 성공 조건과 전체3991개 host 상태 비교를 혼동하지 않는다. 소스 조사에서 제안된63요청 계획은 아직 실제 admission이 아니다. issued catalog, 현재 선택, 변경 의도, 기대하는 설정 차이, 출력 None 보존, typed 복원과 NoSave/저장 보존을 새 유한 계약으로 닫아야 한다. 이전 실패와 미관측 상태는 유지한다.

## 11. 빠짐없이 이어갈 28개 완료물


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


## 12. 바로 다음 단위와 재개 절차

1. 아래 최종 종료 절과 RESUME_STATE에서 새 GUI mute 조사 결과를 확인한다. R83과 같은 MCP setter를 다시 보내지 않는다.
2. 조사에서 실제 CV mute 화면/API 읽기 연결이 확인됐는지와 다른 채널 변화 여부를 구분한다. 불명확하면 source/host-control 경계 조사부터 계속한다. UI 한 번의 성공을 MCP setter 성공 또는 Native60 lifecycle admission으로 쓰지 않는다.
3. lifecycle을 다시 시험하려면 새 정확한 prep 계약, 원본-first 기록, 현 소스/설치/metadata와 독립 검토를 바인딩한다. 이전31계획의 실패 전제를 그대로 두고 namespace만 바꿔 반복하지 않는다.
4. 전체97쓰기 가능 값과 직접7행56필드의 전후 비교, 별도 CV/Main 원본, 계획된 public cleanup과 최종 borrow/descriptor tail, NoSave와 owner wait0/보존을 함께 확인한다.
5. 출력 None/선택지/포트 쓰기와 프리셋 경로 보존은 각각 독립 gate다. generic state/preset API 성공만으로 외부 출력 보존을 선언하지 않는다.
6. main 추가 출시 전에는 해당 PR 현재 head의 독립 판정, 필수 검사2개, 실제 필요한 검증 범위와 소스 동일성을 확인한다. merge 명령만으로 완료라 하지 않고 원격 merged와 main tree, 병합 후 검사를 확인한다.
7. 장치가 분리되었다면 하드웨어4개는 대기시키고 소프트웨어24개를 계속한다. 연결 재개 시 현재 장치·설정·새 owner를 확인한다. 입력 연주와 루프백이 없으면 이를 기다리며 같은 inventory를 반복하지 않는다.

소유한 raw evidence는 로컬 `.local`의 next-state 아래에 남긴다. 정상 Native60, r4 prep, closed R83 원본과 read-only 진단을 삭제하거나 덮어쓰지 않는다. Git에는 이 문서·진행 기록·정제한 근거와 검증된 코드만 올린다. 앞으로도 사용자의 `진행 재개해` 한마디로 이 대기열을 이어갈 수 있게 최신 pointer와 상태를 함께 갱신한다.

## 13. 최종 종료 확인

R84의8실제요청과 GUI 한 번, API 두 읽기1/1을 독립 수용했다. 별도 CV/Main 원본4장과 전체67행을 대조했다. 첫 API 읽기는 클릭 후28.217초이고 두 읽기는11.949초 간격이다. 즉시 반영 속도나 보편적 지연 보장은 아니다. MCP mute setter와 Native60 lifecycle 수용은 false다. 실제 준비부터 종료까지1143.969초, 독립 검토는 약165.589초다.

R84는 NoSave로 닫았고 앱 미실행 관측 직후 CUA completion을 제출했다. owner child의 명시적 wait exit0, source187/art13/saved10/vendor Main/전역 오디오MIDI 설정과 두 잠금, 장치3→4→3을 확인했다. 독립 보고서 SHA는 `0f527baaf80b940b6d6ef20f50afd0492c861b5e2a272974e5383b6eb0b2168f`다. 현재 앱과 owner는 닫혔다.

R85 준비는 R83의 실패한 MCP mute 쓰기를 반복하지 않고 ID8 뒤 GUI 준비의 원본을 확인한 다음 ID9 읽기로 상태와 전체 peer를 검증하는 새31요청 계약이다. 기존 구조형 lifecycle27과 전체110행25/33,7행56필드26/28과 나머지 UI/종료 조건은 유지한다. 별도 namespace·새 owner·현재 source/설치/metadata, R84의 실제 수용과 새 키트 독립 검토가 모두 필요하다. 키트가 시간 안에 닫히지 않으면 실제를 시작하지 않는다. 이 준비 시점에는 R85 actual0이었고, 최종6요청 중단 결과는14절에 있다.

macOS26의 필수 검사는 runner_id0·실행단계0과 capacity 안내로 취소되었고 Core는 통과했다. 동일 수동 재시도는 중단했다. 공개 main15304714의 이전 필수 검사2개 통과를 새 후보의 통과로 대신하지 않는다. R85의 최종 결과와 복원 상태는 아래 절에 기록했다. Git 최종 SHA와 원격 상태는 상태 파일과 최종 기록을 함께 확인한다.

### 별도로 보존한 도구 수정

R84 키트 작성465.812초, pre-click 분기 수정83.75초, 불필요한 Music 선택 요구 제거77.326초, 최종 키트 독립 검토126.419초다. 뒤의 세 시간은 일부 선행 지침 읽기를 제외한 구간이며 전체 wall time처럼 합산하지 않는다. 소스/검토/빌드/키트는 일부 병렬이므로 위 단위 시간을 더해 이번 루프 전체 경과로 쓰지 않는다.

처음 R84 캡처 helper가 R83의 옛 개인 디렉터리를 참조한 파일4개는 덮어쓰지 않고 보존했다. 실제 R84 수용 원본은 새 전용 함수와 명시적 목적지로 다시 캡처했고 R84 디렉터리의 경로/해시/시간을 독립 대조했다. 정적 crop 점검의 Python PIL 부재는 OS 이미지 도구로 검사만 대체했으며 crop을 원본 증거로 수용하지 않았다. 앱 bundle ID가 개인 분석 복사본과 중복돼 정상 vendor 앱의 전체 경로를 명시했다. 복사본은 실행하지 않았다. UI 메뉴의 Cancel과 현재 app binding을 새로 확인했고, 결국 실제 CV 오디오 strip의 악기 편집기 아이콘을 사용했다. 이 과정은 제품 파라미터 변경이나 추가 DAQ로 세지 않는다.


## 14. R85 최종 중단과 실제 범위

새31요청의 실제 Rust 변환·runtime 검증은1검사/31요청/잘못된 입력11개 거절로 통과했다. 계획 준비317.556초 뒤 새 runtime 키트를 연결했다. 작성 구간652.427초, 독립 검토181.762초에서 manifest53·계약49·외부17·현재 source187/art13과 Python12·모의 driver3·합성31단계/단계별 실제 Rust 검증이 통과했다. 이는 실제 DAW31요청의 성공이 아니다. 독립 보고서 SHA는 `c4192887c8729d039ba573058657a6f0e2459d2b187efd5a95bf002861ffe3c7`다.

Root가 새 owner·metadata3write/2원본·문서/세션·장치3→4·소스/설치/저장본을 바인딩했다. 실제는6요청(IDs3~8)만 보내 모두 guard를 통과했다. 전체67행에서 Main volume0 외의 변화가 없음을 확인했다. 클릭 전 별도 CV/Main 원본에서 CV audio mute0, Master 최소와 Stop, CV AudioIn/출력6개 None을 직접 확인했다.

CV 오디오 strip의 M을 한 번 누른 결과 ControlLink와 해당 M은0으로 남았고 화면에 보이는 FX 트랙3~7의 음소거도 풀렸다. Root는 성공 후 기록을 만들거나9번을 보내지 않았다. Native60 lifecycle27, 직접7행/56필드26/28, 전체110행25/33은 모두0회다. 추가 API 읽기를 하지 않았으므로 실패 후 전체67개 값이 바뀌었다고 주장하지 않는다. 보이는 peer 변화는 GUI 관측이다. 원인은 아직 모른다. 그룹이나 키 조합 가설을 확정하지 않는다. 공식 Groups 참고 문서도8.2이므로8.1.1 원인 증거로 쓰지 않는다.

R85는 시작부터 종료 검사까지1039.877초다. 이 시간에는 파일 준비·앱 초기화·키트 검토 대기와 GUI가 포함된다. Root가 NoSave로 앱 종료·미실행을 보고 바로 CUA completion을 남겼다. owner child wait0, 강제 종료 없음, source187/art13/saved10/vendor Main/CoreAudio·MusicDevices 보존과 두 잠금, 장치3→4→3을 확인했다. 독립 중단 감사는269.686초이며 원본 응답8개·실제6요청과 가드6개·화면3장을 대조했다. 감사의 현재 파일 검사는 **05:32:37.843 한국 시간, 아래 복원 이전** 기준이다. 보고서 SHA는 `8eb78622bff73764f9a461e71858e3cabb6b90b9cab7a615d3032eb785dd4eb8`다.

이번 실행의 실제 DAW 요청은 R82의1개, R83-r2의7개, R84의8개, R85의6개로 총22개다. 초기 R83 preflight와 모든 정상 metadata·복원 metadata는 실제 DAW0이다. Native60 lifecycle 실제 호출은 이번 실행 전체0회다. R84의 한 성공 관측을 반복 가능성·즉시 반영·MCP setter 정확성 또는 Native60 성공으로 넓히지 않는다.

## 15. 공개 main 사용 환경 복원

Native60을 실제 수용하지 못했으므로 새 후보의 관리 파일13개와 설치 manifest2개를 별도 보존한 뒤, 공개 main15304714의 CLI와 마지막 실앱 수용 Native59 관리 파일13개를 복원했다. 새 소스187개와 기존 모든 후보 원본은 그대로다. 원본 곡·vendor 앱·오디오/MIDI 설정을 바꾸지 않았다. 현재 CLI SHA는 `42b04424d069952a01e360cac28c5719d2b15b204275cc086d43bb8fb92230cb`, Native59 SHA는 `67c5b25ccf3955a9a0b9fbd99f00e0b17a0064a6498325661f3bcd2bf29466d7`, UUID는 `684497D3-384F-3DD3-B4D2-1C788AF35F33`다.

복원 후 비실시간 doctor의 installation_current와 native files_match_manifest가 true다. 앱을 다시 열지 않고 metadata3write/2원본에서 도구46개·상태 동작7개와 Native60 action 미노출을 확인했다. 이 정상 metadata owner의 child wait0은 Root가 확인했다. 첫 후처리기는 action이 직접 enum이라고 가정해 $ref에서 실패했으며 저장한 원본에서 $defs를 따라 수정했다. owner 재시작·재전송·추가 DAQ는 없다. 종료 PID의 별도 원본 receipt를 parser 이전에 저장한 실제 R84/R85와는 증거 형식을 구분한다. doctor live_verified는 false이며 복원 후 새로운 DAW 실기를 했다고 주장하지 않는다.

보관 blob의 실행 권한이 없어 첫 두 install 호출은 실행 전에 거절됐다. 해시를 다시 확인한 실행용 복사본으로 정상 관리 install/native-install을 수행하고 target CLI를 원자적으로 복원했다. 최초 실패와 실행 복사본, 설치 출력은 별도 남겼다. 이전 후보나 실패 원본을 덮어쓰지 않았다.

### 다음 재개에서 설치와 소스를 혼동하지 말 것

현재 checkout은 Native60 후보가 있는 feature 브랜치지만 실행 파일과 설치는 공개 main/Native59다. 기존 Native60 normal artifact map의13개 절대 경로는 이제 현재 설치와 다르므로 그대로 current라고 승인하면 안 된다. Native60 후보의13파일·manifest2개는 별도 보존 묶음에서 원래 SHA와 비교할 수 있다. artifacts의 Native60 패키지도 남겼다.

Native60을 다시 시험하기로 결정하면 먼저 현재 source187이 동결본과 같은지 확인한다. 그 다음 보존한 Native60 CLI/관리 자산·패키지로 정상 설치를 복구하거나 소스가 달라졌으면 새 빌드를 한다. target CLI를 main으로 복원했으므로 Cargo cache가 있다고 해서 Native60 바이너리라고 가정하지 않는다. 빌드 전 보존한 후보 CLI를 일치 확인해 복구하거나 명시적으로 새 컴파일하고 최종 CLI SHA·상태8·native UUID2E958F38을 다시 확인한다. 정상 metadata/설치·잠금·저장본·현재 장치와 새 세션/문서/인스턴스가 확인된 뒤에만 새 실제 계약을 사용한다.

R83·R84·R85는 모두 닫혔다. R85의31계획을 그대로 재전송하지 않는다. 다음 기술 단위는 실제 mute0/peer 변화의 조작·그룹/연결 문맥을 읽기 위주로 구분하고, 필요한 새 유한 진단과 독립 관측 조건을 설계하는 일이다. 그 전에는 Native60 lifecycle, 프리셋 적용·load, 외부 포트 쓰기를 시작하지 않는다. CV 프리셋 출력 경로 보존 gate와 나머지28개 ID는 모두 유지한다.

최종 파일·잠금·프로세스 확인은11월18일 오전5시45분에 통과했다. 현재 source187은 후보 그대로, 설치13은 main/Native59와 같고 저장본10·vendor Main·전역 오디오/MIDI 설정은 유지됐다. 장치5/3/3은 그 시각의 마지막 관측이며 오전6시 이후 연결을 보장하지 않는다. 자동 재개는 없고 `진행 재개해`로 이어간다.

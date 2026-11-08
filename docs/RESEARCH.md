# Fender Studio Pro MCP 참고 프로젝트 조사

이 문서는 구현 계획의 근거와 아직 확인하지 못한 범위를 기록한다. 세 저장소의 소스를 조사했다. DAW를 실행하거나 브리지를 설치하거나 참고 프로젝트의 테스트를 실행하지 않았다.

사용자는 macOS 우선을 선택했다. 로컬 `/Applications/Studio Pro 8.app/Contents/Info.plist`에서 읽은 버전은 `8.1.1 Build 113103`이다. 공개 매뉴얼은 8.2를 표시한다. 매뉴얼과 참고 프로젝트의 동작 보고를 로컬 8.1.1의 검증 결과로 취급하지 않는다.

## 소스 기준점을 고정한다

| 프로젝트 | 조사한 커밋 | 역할 |
| --- | --- | --- |
| NeanderthalMan/studio-one-mcp | `7d9711f351272478787521adebe100d9f4688358` | 파일 파서, DAW 내부 브리지, EditTask, 광범위한 편집 기능의 원형 |
| b4blue/fender-studio-mcp | `7f74f0a150df7bc178ad4cb2ae00c556994758dc` | 위 프로젝트의 Studio Pro 8 대응, 가상 MIDI, 오디오 캡처와 분석 |
| pajdarova/FenderStudioProMCP | `09bd3a36145d84be0b7764ff58d8b4cbf9933476` | MCU, 사용자 명령 카탈로그, 단축키 자동화의 비교 대상 |

세 프로젝트는 모두 MIT 라이선스다. 실제 코드나 브리지 자산을 가져오면 해당 저작권 표시와 라이선스를 함께 보존한다. 소스와 라이선스는 각 기준 커밋에 고정한다. [원본 LICENSE](https://github.com/NeanderthalMan/studio-one-mcp/blob/7d9711f351272478787521adebe100d9f4688358/LICENSE), [b4blue LICENSE](https://github.com/b4blue/fender-studio-mcp/blob/7f74f0a150df7bc178ad4cb2ae00c556994758dc/LICENSE), [pajdarova LICENSE](https://github.com/pajdarova/FenderStudioProMCP/blob/09bd3a36145d84be0b7764ff58d8b4cbf9933476/LICENSE).

## NeanderthalMan에서 편집 구조를 가져온다

Node MCP 서버는 저장된 `.song`을 읽는 `song_*` 도구와 실행 중인 DAW를 다루는 `live_*` 도구를 제공한다. `.song`은 ZIP 안의 XML과 UBJSON을 해석한다. 저장 파일의 상태와 미저장 실시간 상태는 서로 다른 데이터다. [README](https://github.com/NeanderthalMan/studio-one-mcp/blob/7d9711f351272478787521adebe100d9f4688358/README.md), [song.js](https://github.com/NeanderthalMan/studio-one-mcp/blob/7d9711f351272478787521adebe100d9f4688358/src/song.js).

서버가 `{id, op, args}`를 임시 파일에 쓴 뒤 `request.json`으로 바꾼다. MIDI note 119를 보내면 DAW의 control-surface JavaScript가 요청을 처리하고 `response.json`을 쓴다. 요청 대기는 프로세스 내부 큐가 직렬화한다. 같은 파일을 사용하는 다른 프로세스까지 직렬화하지는 않는다. [bridge.js](https://github.com/NeanderthalMan/studio-one-mcp/blob/7d9711f351272478787521adebe100d9f4688358/src/bridge.js), [midi.js](https://github.com/NeanderthalMan/studio-one-mcp/blob/7d9711f351272478787521adebe100d9f4688358/src/midi.js).

노트와 곡 구조 편집에는 별도 EditTask 확장이 필요하다. Control-surface가 작업 파일을 쓰고 `Musical Functions/MCP Edit` 또는 `Track/MCP Track Edit` 명령을 실행한다. EditTask는 host가 넘겨준 편집 함수를 사용한다. 이 구조가 Rust 서버에서도 JavaScript 자산을 남겨둘 이유다. [McpEdit.js](https://github.com/NeanderthalMan/studio-one-mcp/blob/7d9711f351272478787521adebe100d9f4688358/device/EditTasks/package/McpEdit.js), [McpTrackOps.js](https://github.com/NeanderthalMan/studio-one-mcp/blob/7d9711f351272478787521adebe100d9f4688358/device/EditTasks/package/McpTrackOps.js), [설치기](https://github.com/NeanderthalMan/studio-one-mcp/blob/7d9711f351272478787521adebe100d9f4688358/src/setup/device.js).

소스에는 트랙, 이벤트, MIDI 파트와 노트, 믹서, 버스와 VCA, 라우팅, 플러그인 삽입, 알려진 파라미터, 마커, 편곡 구간, 템포와 박자 편집이 있다. README의 개발 환경은 Studio One 5.5.2 macOS다. Pro 8과 Windows는 미검증이라고 명시한다. 코드 존재와 현재 버전의 동작 보장은 다르다. [기능과 검증 범위](https://github.com/NeanderthalMan/studio-one-mcp/blob/7d9711f351272478787521adebe100d9f4688358/README.md#testing).

중요한 제약은 다음과 같다.

- 파라미터 전체를 열거하지 못한다. preset과 remote-control map에서 이름을 찾은 뒤 host에서 조회한다. 타사 플러그인에는 빠진 이름이 생긴다.
- 기존 버스에 새 send를 연결하는 경로가 부족하다. 새 FX 채널과 send 생성은 별도 기능이다.
- Automation point를 직접 넣지 못해 실제 재생 중 Write 모드로 값을 기록한다.
- Mixdown과 stem export는 대화상자를 열기 때문에 제공하지 않는다. Bounce의 렌더 파일은 undo 이후에도 남는다.
- 마커나 템포 조회가 잠시 재생 위치를 움직일 수 있다. 일부 조회는 순수 읽기가 아니다.
- 저장본의 연속 tempo ramp를 step으로 근사한다. 정확한 초와 beat 변환의 근거로 삼지 않는다.
- Undo 기록 시점과 지원 범위가 다르다. 이름과 색상 변경에는 역방향 쓰기가 필요할 수 있다.
- Host 객체의 잘못된 호출이나 `throw`는 modal Scripting Error를 만들 수 있다고 기록한다.
- Host signal 구독 실험은 종료와 문서 해제 때 충돌이 발생해 폐기됐다고 기록한다. 안전한 snapshot 비교가 우선이다.

이 제약은 해당 구현과 버전에 관한 근거다. Fender Studio Pro 전체의 영구적인 한계로 확정하지 않는다. [README의 제약과 undo 설명](https://github.com/NeanderthalMan/studio-one-mcp/blob/7d9711f351272478787521adebe100d9f4688358/README.md), [automation.js](https://github.com/NeanderthalMan/studio-one-mcp/blob/7d9711f351272478787521adebe100d9f4688358/src/automation.js).

## b4blue에서 Studio Pro 8 대응을 가져온다

b4blue는 독립적인 새 제어 방식보다 위 구조를 확장한 fork다. README는 Studio Pro 8.1.2 macOS에서 사용한다고 보고한다. Fender profile과 프로세스 이름, `Session/Remove Track`, 이름이 같은 instrument note track과 audio channel 구분을 다룬다. 자체 CoreMIDI 가상 포트를 제공하고 다른 서버 프로세스에 MIDI wakeup을 중계한다. [README](https://github.com/b4blue/fender-studio-mcp/blob/7f74f0a150df7bc178ad4cb2ae00c556994758dc/README.md), [midi.js](https://github.com/b4blue/fender-studio-mcp/blob/7f74f0a150df7bc178ad4cb2ae00c556994758dc/src/midi.js), [tracks.js](https://github.com/b4blue/fender-studio-mcp/blob/7f74f0a150df7bc178ad4cb2ae00c556994758dc/src/tracks.js).

추가된 오디오 기능은 Swift Core Audio process tap으로 WAV를 캡처하고 Python으로 loudness, peak, spectrum, stereo 지표를 계산한다. Python 경로는 NumPy, SciPy, soundfile, pyloudnorm을 사용한다. Rust 구현에서 그대로 외부 실행 의존성으로 둘 필요는 없다. [audio.js](https://github.com/b4blue/fender-studio-mcp/blob/7f74f0a150df7bc178ad4cb2ae00c556994758dc/src/audio.js), [s1tap.swift](https://github.com/b4blue/fender-studio-mcp/blob/7f74f0a150df7bc178ad4cb2ae00c556994758dc/audio/tap/s1tap.swift), [analyze.py](https://github.com/b4blue/fender-studio-mcp/blob/7f74f0a150df7bc178ad4cb2ae00c556994758dc/audio/analyze.py).

Rust 설계에서 보완할 사항은 다음과 같다. 아래 문제는 소스에서 도출한 위험이며 재현한 장애는 아니다.

- MIDI 중계가 있어도 mailbox 큐는 각 프로세스에 남아 있다. 두 서버가 동일 요청 파일을 덮어쓸 수 있다. [bridge.js](https://github.com/b4blue/fender-studio-mcp/blob/7f74f0a150df7bc178ad4cb2ae00c556994758dc/src/bridge.js).
- 응답에 session이 있지만 클라이언트는 request ID만 비교한다. 요청에 예상 문서와 revision도 없다. [BridgeCore.js](https://github.com/b4blue/fender-studio-mcp/blob/7f74f0a150df7bc178ad4cb2ae00c556994758dc/device/StudioOneMCP/BridgeCore.js).
- JS의 마지막 ID 기억은 crash 이후 exactly-once를 보장하지 않는다. 실행 후 응답 유실은 실패로 단정할 수 없다. [BridgeCore.js](https://github.com/b4blue/fender-studio-mcp/blob/7f74f0a150df7bc178ad4cb2ae00c556994758dc/device/StudioOneMCP/BridgeCore.js).
- 지연 적용을 기다리는 `until()`은 조건이 끝내 맞지 않아도 마지막 값을 반환한다. 대기 종료를 결과 검증 성공과 분리해야 한다. [settle.js](https://github.com/b4blue/fender-studio-mcp/blob/7f74f0a150df7bc178ad4cb2ae00c556994758dc/src/settle.js).
- Surface mapping의 256 channels, 16 inserts, 8 sends는 구현 상한이다. DAW 자체의 상한으로 쓰지 않는다. 초과 결과를 조용히 자르면 안 된다. [surface.xml](https://github.com/b4blue/fender-studio-mcp/blob/7f74f0a150df7bc178ad4cb2ae00c556994758dc/device/StudioOneMCP/StudioOneMCP.surface.xml).
- 플러그인 snapshot은 발견한 파라미터 집합이다. 완전한 opaque plugin state 복제로 부르면 안 된다. [plugins.js](https://github.com/b4blue/fender-studio-mcp/blob/7f74f0a150df7bc178ad4cb2ae00c556994758dc/src/plugins.js).
- 녹음 전용 도구와 일반 transport 또는 command 경로의 제약이 다르다. 공통 실행 계층에 작업 분류를 둬야 한다. [server.js](https://github.com/b4blue/fender-studio-mcp/blob/7f74f0a150df7bc178ad4cb2ae00c556994758dc/src/server.js).
- 재생 시작 후 tap을 준비하면 캡처 앞부분이 빠질 수 있다. 복구 오류를 숨기면 실제로 복원되지 않아도 복원 성공으로 보고할 수 있다. [audio.js](https://github.com/b4blue/fender-studio-mcp/blob/7f74f0a150df7bc178ad4cb2ae00c556994758dc/src/audio.js).
- 합성 신호 테스트는 완전한 BS.1770 또는 EBU 적합성 검증이 아니다. 무음과 캡처 권한 오류도 구분해야 한다. [audio.test.js](https://github.com/b4blue/fender-studio-mcp/blob/7f74f0a150df7bc178ad4cb2ae00c556994758dc/test/audio.test.js).

## pajdarova에서 명령 탐색과 MCU의 경계를 확인한다

Python 서버는 MCU MIDI와 OS 키 입력을 사용한다. 사용자 `.keyscheme`에서 명령과 단축키를 읽는 방식은 유용하다. 페이더, 미터, LCD feedback은 일부 읽지만 mute, solo, record arm, bank는 전송 이력에 의존한다. Bank별 strip과 축약 이름은 안정적인 트랙 ID가 아니다. [midi_bridge.py](https://github.com/pajdarova/FenderStudioProMCP/blob/09bd3a36145d84be0b7764ff58d8b4cbf9933476/src/studio_pro_mcp/midi_bridge.py), [keyscheme.py](https://github.com/pajdarova/FenderStudioProMCP/blob/09bd3a36145d84be0b7764ff58d8b4cbf9933476/src/studio_pro_mcp/keyscheme.py).

과거 MIDI CC 명령 전달은 MCU V-Pot과 충돌해 폐기됐다고 README가 밝힌다. 현재 명령 도구는 단축키 또는 Ctrl+K 검색을 사용한다. Ctrl+K fallback은 소스에서 Windows 전용이며 검색 결과 선택도 미검증이라고 표시한다. 이를 macOS에서 검증된 범용 명령 실행기로 취급하면 안 된다. [README](https://github.com/pajdarova/FenderStudioProMCP/blob/09bd3a36145d84be0b7764ff58d8b4cbf9933476/README.md), [명령 도구](https://github.com/pajdarova/FenderStudioProMCP/blob/09bd3a36145d84be0b7764ff58d8b4cbf9933476/src/studio_pro_mcp/tools/commands.py), [fallback 구현](https://github.com/pajdarova/FenderStudioProMCP/blob/09bd3a36145d84be0b7764ff58d8b4cbf9933476/src/studio_pro_mcp/keystrokes.py#L565-L577).

플러그인 DB 탐색과 매크로 파일 생성은 있다. 그러나 매크로 파일을 썼다는 결과가 DAW에 플러그인이 삽입됐다는 뜻은 아니다. 현재 소스에는 `.song` 파서, 의미 단위 MIDI 노트 CRUD, arranger CRUD, 실시간 플러그인 파라미터 열거 계층이 없다. [plugin_db.py](https://github.com/pajdarova/FenderStudioProMCP/blob/09bd3a36145d84be0b7764ff58d8b4cbf9933476/src/studio_pro_mcp/plugin_db.py), [macro_tools.py](https://github.com/pajdarova/FenderStudioProMCP/blob/09bd3a36145d84be0b7764ff58d8b4cbf9933476/src/studio_pro_mcp/tools/macro_tools.py).

UCNET 문서는 명시적으로 연구 단계이며 실제 구현 근거가 아니다. JS bridge를 포기한 과거 문서는 Studio One 7.2.3의 특정 context와 timer 조건에 관한 기록이다. 다른 프로젝트의 MIDI callback 기반 Pro 8 bridge까지 불가능하다고 결론내리지 않는다. [UCNET 계획](https://github.com/pajdarova/FenderStudioProMCP/blob/09bd3a36145d84be0b7764ff58d8b4cbf9933476/docs/phase2-ucnet.md), [JS bridge 조사](https://github.com/pajdarova/FenderStudioProMCP/blob/09bd3a36145d84be0b7764ff58d8b4cbf9933476/docs/phase3-macro-bridge.md).

## 공식 문서와 커뮤니티 관찰을 구분한다

Fender 공식 매뉴얼은 Control Link, Mackie Control, MIDI 장치 설정과 매크로를 설명한다. 전체 DAW 객체를 원격으로 편집하는 공개 API 계약을 이 문서에서 확인한 것은 아니다. 내부 JS API의 호환성은 실제 버전별 probe로 확인한다. [Control Link](https://fenderstudiopromanual.fender.com/en/Content/Control_Link_Topics/Chapter-Control_Link.htm), [Mackie Control](https://fenderstudiopromanual.fender.com/en/Content/Control_Link_Topics/Mackie_Control.htm), [Macro Toolbar](https://fenderstudiopromanual.fender.com/en/Content/Editing_Topics/Macro_Toolbar.htm).

Rust MCP에는 공식 `rmcp` SDK를 사용한다. 버전과 feature flags는 구현 시작 시 게시된 release와 호환 클라이언트를 확인하고 고정한다. 현재 문서의 stdio, tools, resources와 구조화 스키마를 기반으로 한다. 최신 프로토콜 기능을 모든 클라이언트가 지원한다고 가정하지 않는다. [공식 Rust SDK](https://github.com/modelcontextprotocol/rust-sdk), [MCP tools 규격](https://modelcontextprotocol.io/specification/2026-07-28/server/tools).

Apple은 Core Audio process tap과 aggregate device를 이용한 캡처를 문서화한다. `NSAudioCaptureUsageDescription`과 시스템 오디오 녹음 권한이 필요하다. Apple sample의 최소 OS 설명과 b4blue의 macOS 14.4+ 요구가 다르므로, 제품의 최소 OS는 사용할 API availability와 실제 테스트 결과로 정한다. [Apple Core Audio taps](https://developer.apple.com/documentation/CoreAudio/capturing-system-audio-with-core-audio-taps).

## 계획에 적용할 결론

1. Rust 실행 파일이 MCP, 작업 실행, 검증, 파일 읽기, 분석, 설치와 진단을 맡는다.
2. DAW 안에는 검증된 연산만 받는 JavaScript control-surface와 EditTask를 둔다.
3. 우선 b4blue와 NeanderthalMan의 제어 경로를 8.1.1에서 증명한다. MCU와 Accessibility는 보완 경로로 둔다.
4. 명령 수보다 실제 작업의 완료, 상태 readback, 실패 시 결과 설명을 기준으로 범위를 넓힌다.
5. 직접 automation 편집, 기존 채널 send, 완전한 타사 플러그인 상태, 무인 mixdown과 stems는 별도 검증 과제로 관리한다.

실환경 검증, Rust 포팅, 설치와 패키징은 모두 다음 구현 작업이다. 이번 결과물은 조사와 계획이다.

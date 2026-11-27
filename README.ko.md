<h1 align="center">Fender Studio Pro MCP</h1>

<p align="center"><a href="README.md">English</a> · <strong>한국어</strong></p>

<h2 align="center">그래서 이게 뭔가요?</h2>
<p align="center">Fender Studio Pro에서 트랙을 편집하거나 믹서를 조절하는 일을 채팅으로 AI에게 부탁할 수 있게 해주는 도구입니다.<br>
실험적 프로젝트이며 Apple Silicon Mac의 Studio Pro 8.1.1 (빌드 113103)에서 검증했습니다.</p>

<p align="center"><a href="https://dev-universe.github.io/fender-studio-pro-mcp/ko/">프로젝트 사이트</a></p>

- [빠른 시작](#빠른-시작)
- [사용 방법](#사용-방법)
- [지원 플러그인](#지원-플러그인)
- [네이티브 애드온](#선택-설치-네이티브-애드온)
- [오프라인 파일 검사](#오프라인-파일-검사)
- [검증 범위](#검증-범위)
- [프로젝트 문서](#프로젝트-문서)
- [라이선스](#라이선스)

## 빠른 시작

Fender Studio Pro, Cargo가 포함된 Rust 1.88 이상, 로컬 stdio 서버를 지원하는 MCP 클라이언트가 필요합니다.

### 1. 빌드하고 브리지 설치하기

설치 프로그램은 기본으로 `~/Library/Application Support/Fender/Studio Pro 8`을 사용합니다. 사용자 지정 프로필을 쓰는 경우 설치와 연결 명령마다 `--profile /path/to/profile`을 넘기세요. 클라이언트의 `serve` 인수에도 같은 경로를 넣으세요.

Studio Pro를 종료한 뒤 서버를 빌드하고 브리지와 EditTask를 설치하세요.

```sh
git clone https://github.com/dev-universe/fender-studio-pro-mcp.git
cd fender-studio-pro-mcp
cargo build --locked --release
./target/release/fender-studio-pro-mcp install
```

### 2. 컨트롤 서피스 설정하기

가상 MIDI 소스를 시작하세요. 컨트롤 서피스 설정을 끝낼 때까지 이 명령을 실행한 상태로 두세요.

```sh
./target/release/fender-studio-pro-mcp hold
```

`"ready": true`가 표시되면 Studio Pro를 여세요. **External Devices**에서 다음 값으로 **Control Surface**를 추가하세요.

| 설정 | 값 |
| --- | --- |
| Vendor | `fender-studio-pro-mcp` |
| Device | `Fender MCP Bridge` |
| Receive From | `fender-studio-pro-mcp` |
| Send To | `None` |

이름을 붙인 시험 송을 열고 재생과 녹음을 정지하세요. 검증된 연결 방식은 열린 송이 필요합니다.

### 3. 연결 확인하고 클라이언트 시작하기

Ctrl+C로 `hold`를 종료한 뒤 연결을 진단하세요.

```sh
./target/release/fender-studio-pro-mcp doctor --live
```

출력에서 `"installation_current": true`와 `"live_verified": true`를 모두 확인하세요.

빌드한 실행 파일의 절대 경로로 `serve`를 실행하도록 MCP 클라이언트를 설정하세요. 아래 예시를 쓰는 클라이언트의 설정 형식에 맞게 적용하세요.

```json
{
  "mcpServers": {
    "fender-studio-pro": {
      "command": "/path/to/fender-studio-pro-mcp/target/release/fender-studio-pro-mcp",
      "args": ["serve"]
    }
  }
}
```

클라이언트를 시작하고 `host_status`를 호출해 열린 송을 읽을 수 있는지 확인하세요.

## 사용 방법

열린 송의 상태를 읽고, 트랙과 MIDI 노트를 편집하고, 믹서와 재생을 제어할 수 있습니다. 지원되는 내장 플러그인 설정도 변경할 수 있습니다. 시험 송에서 아래처럼 요청해 보세요.

- “이 송에서 음소거된 믹서 채널이 뭐야?”
- “템포를 100 BPM으로 바꾼 다음 실제 적용된 값을 확인해 줘.”

편집할 때 클라이언트는 `host_status`의 현재 `document` 토큰을 `expected_document`로 보냅니다. 송을 바꾸거나 다시 열었으면 토큰을 새로 읽으세요. 사용 가능한 동작과 인수는 `operations_list`에서 확인할 수 있습니다.

`applied_verified`에는 동작별로 다시 읽어 확인한 결과가 포함됩니다. `applied_unverified`는 호스트가 변경을 접수했지만 검증은 일부 남은 상태입니다. `partially_applied`는 일부 변경만 적용된 상태입니다. `outcome_unknown`을 받으면 같은 요청을 다시 보내기 전에 현재 상태를 확인하세요.

트랙의 악기를 제어하려면 `tracks_list`로 정확한 트랙 이름을 읽으세요. 트랙 이름과 현재 문서 토큰으로 `track_select`를 호출하세요. 악기를 조회하거나 변경하기 전에 `tracks_list`에서 대상 트랙만 선택됐는지 확인하세요.

### 플러그인과 라우팅 가이드

| 작업 | 가이드 |
| --- | --- |
| Pedalboard 슬롯 교환과 Ampire 조회 | [Pedalboard와 Ampire](docs/PLUGIN_WORKFLOWS.md#pedalboard-and-ampire) |
| Serum 2와 Serum 2 FX 프리셋 적용과 MIDI 라우팅 | [Serum 2와 Serum 2 FX](docs/PLUGIN_WORKFLOWS.md#serum-2-and-serum-2-fx) |
| FabFilter 프리셋 적용과 MIDI 매핑 | [FabFilter 사용 방법](docs/PLUGIN_WORKFLOWS.md#fabfilter-preset-library) |
| Multi Instrument, Chorder, CV, Impact, External Effect 제어 | [플러그인 사용 방법](docs/PLUGIN_WORKFLOWS.md) |
| 중첩 Splitter와 Scope Send 편집 | [인서트 라우팅과 Scope 사이드체인](docs/INSERT_ROUTING.md) |

## 지원 플러그인

아래 플러그인은 Apple Silicon의 Studio Pro 8.1.1 (빌드 113103)에서 MCP 동작을 검증했습니다. 지원하는 동작은 플러그인별로 다릅니다. 시험한 설정, 빌드, 제약은 각 가이드에 나와 있습니다. 일부 동작에는 [네이티브 애드온](#선택-설치-네이티브-애드온)이 필요합니다.

### Studio Pro 기본 플러그인

실앱에서 내장 플러그인 클래스 60개를 검증했습니다. Phase Meter는 인서트 슬롯 바이패스를 지원합니다. [기본 플러그인 검증표](docs/BUILTIN_PLUGINS.md)

| 분류 | 플러그인 |
| --- | --- |
| 악기 | CV Instrument, Impact, Mai Tai, Mojito, Multi Instrument, Presence, SampleOne |
| 노트 FX | Arpeggiator, Chorder, Input Filter, Repeater |
| 앰프와 페달 | Ampire, Mustang Native, Pedalboard, Rumble Native |
| EQ, 다이내믹스, 디스토션 | Bitcrusher, Channel Strip, Compressor, De-Esser, Expander, Fat Channel, Gate, Limiter, Multiband Dynamics, Pro EQ, RedlightDist, Tricomp |
| 필터, 모듈레이션, 보컬 효과 | Autofilter, Chorus, Flanger, Phaser, Rotor, Vocoder, Voice FX, Vocal Tune, X-Trem |
| 딜레이와 리버브 | Analog Delay, Beat Delay, Groove Delay, IR Maker, Mixverb, Open AIR, Room Reverb, Studio Verb, Surround Delay |
| 미터링과 유틸리티 | Binaural Pan, Dual Pan, Level Meter, Mixtool, Phase Meter, Scope, Spectrum Meter, Tone Generator, Tuner, VU Meter |
| 외부 효과 | External Effect Mono, External Effect Stereo |
| 라우팅, Mix FX, 공간 제어 | Console Shaper, Splitter, Surround Panner |

### 써드파티 VST3 플러그인

#### FabFilter

아래 23종에서 프리셋 작업을 검증했습니다. 파라미터와 MIDI의 검증 범위는 제품마다 다릅니다. [FabFilter 프리셋 라이브러리](docs/PLUGIN_WORKFLOWS.md#fabfilter-preset-library) · [MIDI 컨트롤러 매핑](docs/PLUGIN_WORKFLOWS.md#fabfilter-midi-controller-mapping)

- Micro
- One
- Pro-C 2
- Pro-C 3
- Pro-DS
- Pro-G
- Pro-L
- Pro-L 2
- Pro-MB
- Pro-Q 2
- Pro-Q 3
- Pro-Q 4
- Pro-R
- Pro-R 2
- Saturn
- Saturn 2
- Simplon
- Timeless 2
- Timeless 3
- Twin 2
- Twin 3
- Volcano 2
- Volcano 3

#### Xfer Records Serum 2

두 제품 모두 2.1.5에서 프리셋 적용과 MIDI 라우팅을 검증했습니다. [Serum 2와 Serum 2 FX 사용 방법](docs/PLUGIN_WORKFLOWS.md#serum-2-and-serum-2-fx)

- Serum 2
- Serum 2 FX

검증이 끝난 플러그인을 이 목록에 계속 추가할 예정입니다.

## 선택 설치 네이티브 애드온

네이티브 파라미터, 지원되는 플러그인 상태·프리셋, 인서트 라우팅을 다루려면 애드온을 설치하세요. Apple Silicon의 Studio Pro 8.1.1 빌드 113103을 대상으로 합니다. 설치 전에 Studio Pro를 닫고 서버 또는 `hold`를 종료하세요. 빌드에는 Rust 1.88.0, Python 3, macOS 명령줄 도구가 필요합니다.

```sh
sh native/build.sh
./target/release/fender-studio-pro-mcp native-install artifacts/native/fender-studio-pro-mcp.native.mac
```

문제 해결, 업데이트, 롤백, 제거, 개발자 프리뷰 패키지는 설치 가이드에서 확인하세요. [설치와 유지보수](docs/INSTALLATION.md)

## 오프라인 파일 검사

아래 명령은 Studio Pro에 연결하지 않고 실행할 수 있습니다.

```sh
./target/release/fender-studio-pro-mcp song-read /path/to/Scratch.song
./target/release/fender-studio-pro-mcp audio-analyze /path/to/render.wav
./target/release/fender-studio-pro-mcp local-catalog --plugin 'Pro EQ'
```

`song-read`는 마지막으로 저장한 송을 읽습니다. `audio-analyze`는 WAV의 라우드니스, 피크, RMS, 스테레오 레벨을 측정합니다. `local-catalog`는 설치된 앱의 메타데이터에서 파라미터 이름 후보를 찾습니다. CLI로 오디오와 MIDI 장치 목록도 확인할 수 있습니다. [하드웨어 명령](docs/HARDWARE.md)

## 검증 범위

내장 플러그인 클래스 60개 중 59개는 대표 파라미터 또는 그래프 변경과 독립 읽기, 원복을 검증했습니다. Phase Meter는 인서트 슬롯 바이패스를 검증했습니다. [기본 플러그인 검증표](docs/BUILTIN_PLUGINS.md)

Studio Pro 8.1.1에서 확인한 상태 읽기, 저장과 재열기, 이벤트, 신호 시험 결과를 검증표에 기록했습니다. 실앱 연동은 Apple Silicon Mac에서 확인했으며 Windows, Intel Mac, Linux는 아직 검증하지 않았습니다. [기능별 검증 현황](docs/BUILTIN_PLUGINS.md#current-workflow-verification) · [실앱 검증 보고서](docs/VALIDATION.md) · [플러그인 상태 검증](docs/PLUGIN_STATE_VALIDATION.md)

실험 결과와 남은 과제, 근거 자료는 상세 기록에서 확인할 수 있습니다. [기본 플러그인 실험 기록](docs/BUILTIN_PLUGIN_EXPERIMENTS.md)

## 프로젝트 문서

- Rust 서버, DAW 내부 JavaScript 브리지, CoreMIDI 연결 방식. [아키텍처](docs/ARCHITECTURE.md)
- 기존 작업 재개와 새 플러그인 지원 추가. [재개 안내](docs/RESUME.md) · [MCP 개발 절차](docs/MCP_DEVELOPMENT.md)
- Impact 공장 프리셋 적용과 믹서 원복. [Impact 프리셋 검증 절차](docs/IMPACT_PRESETS.md)
- 개발 환경, 오프라인 검사, 보안 문제 비공개 제보. [기여 안내](CONTRIBUTING.md) · [보안](SECURITY.md)

## 라이선스

이 독립 프로젝트는 MIT 라이선스를 사용하며 가져온 자산의 고지문을 보존합니다. Fender Studio Pro와 Studio One은 각 소유자의 상표입니다. [MIT 라이선스](LICENSE) · [써드파티 고지](THIRD_PARTY_NOTICES.md)

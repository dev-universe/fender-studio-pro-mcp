<h1 align="center">Fender Studio Pro MCP</h1>

<p align="center"><strong>English</strong> · <a href="README.ko.md">Korean</a></p>

<h2 align="center">What is this?</h2>
<p align="center">Control Fender Studio Pro from an MCP-compatible AI client.<br>
Experimental · Live-tested on Studio Pro 8.1.1 (Build 113103) on Apple Silicon Macs</p>

<p align="center"><a href="https://dev-universe.github.io/fender-studio-pro-mcp/">Project website</a></p>

- [Quick start](#quick-start)
- [Usage](#usage)
- [Supported plugins](#supported-plugins)
- [Native add-on](#optional-native-add-on)
- [Inspect files offline](#inspect-files-offline)
- [Validation](#validation)
- [Project docs](#project-docs)
- [License](#license)

## Quick start

You need Fender Studio Pro, Rust 1.88 or later with Cargo, and an MCP client that supports local stdio servers.

### 1. Build and install the bridge

The installer uses `~/Library/Application Support/Fender/Studio Pro 8` by default. For a custom profile, pass `--profile /path/to/profile` with each install and connection command. Add it to the client's `serve` arguments too.

Quit Studio Pro, then build the server and install the bridge and EditTasks.

```sh
git clone https://github.com/dev-universe/fender-studio-pro-mcp.git
cd fender-studio-pro-mcp
cargo build --locked --release
./target/release/fender-studio-pro-mcp install
```

### 2. Set up the control surface

Start the virtual MIDI source. Leave this command running until the control surface is configured.

```sh
./target/release/fender-studio-pro-mcp hold
```

When the command reports `"ready": true`, open Studio Pro. In **External Devices**, add a **Control Surface** with these settings.

| Setting | Value |
| --- | --- |
| Vendor | `fender-studio-pro-mcp` |
| Device | `Fender MCP Bridge` |
| Receive From | `fender-studio-pro-mcp` |
| Send To | `None` |

Open a named scratch song with playback and recording stopped. An open song is required for the verified connection path.

### 3. Verify the connection and start your client

Stop `hold` with Ctrl+C, then run the live diagnostic.

```sh
./target/release/fender-studio-pro-mcp doctor --live
```

Look for both `"installation_current": true` and `"live_verified": true` in the output.

Configure your MCP client to start `serve` with the absolute path to your built binary. Adapt this example to your client's configuration format.

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

Start the client and ask it to call `host_status` to confirm it can see the open song.

## Usage

The client can inspect the open song, edit tracks and MIDI notes, adjust the mixer and transport, and change supported built-in plugin settings. Try these prompts in your scratch song.

- "Which mixer channels are muted in this song?"
- "Set the tempo to 100 BPM and read it back."

For edits, the client passes the current `document` token from `host_status` as `expected_document`. Read a fresh token after switching or reopening a song. Call `operations_list` to see the available operations and their arguments.

`applied_verified` includes operation-specific readback. `applied_unverified` records an acknowledged edit with incomplete verification. `partially_applied` means some changes took effect. If the client receives `outcome_unknown`, check the current state before repeating the request.

To control an instrument on a track, read its exact name with `tracks_list`. Call `track_select` with that name and the current document token. Check that `tracks_list` shows only that track selected before inspecting or editing the instrument.

### Plugin and routing guides

| Task | Guide |
| --- | --- |
| Exchange Pedalboard slots or inspect Ampire | [Pedalboard and Ampire](docs/PLUGIN_WORKFLOWS.md#pedalboard-and-ampire) |
| Apply presets or route MIDI in Serum 2 and Serum 2 FX | [Serum 2 and Serum 2 FX](docs/PLUGIN_WORKFLOWS.md#serum-2-and-serum-2-fx) |
| Apply FabFilter presets or set up MIDI mapping | [FabFilter workflows](docs/PLUGIN_WORKFLOWS.md#fabfilter-preset-library) |
| Control Multi Instrument, Chorder, CV, Impact or External Effect | [Plugin workflows](docs/PLUGIN_WORKFLOWS.md) |
| Edit nested Splitters and Scope Sends | [Insert routing and Scope sidechain](docs/INSERT_ROUTING.md) |

## Supported plugins

These plugins have verified MCP operations in Studio Pro 8.1.1 (Build 113103) on Apple Silicon. The available operations vary by plugin. The linked guides list the tested settings, builds, and limitations. Some operations require the [native add-on](#optional-native-add-on).

### Studio Pro built-ins

The live tests covered 60 built-in classes. Phase Meter support covers insert-slot bypass. [Built-in plugin matrix](docs/BUILTIN_PLUGINS.md)

| Category | Plugins |
| --- | --- |
| Instruments | CV Instrument, Impact, Mai Tai, Mojito, Multi Instrument, Presence, SampleOne |
| Note FX | Arpeggiator, Chorder, Input Filter, Repeater |
| Amps and pedals | Ampire, Mustang Native, Pedalboard, Rumble Native |
| EQ, dynamics, and distortion | Bitcrusher, Channel Strip, Compressor, De-Esser, Expander, Fat Channel, Gate, Limiter, Multiband Dynamics, Pro EQ, RedlightDist, Tricomp |
| Filter, modulation, and vocal effects | Autofilter, Chorus, Flanger, Phaser, Rotor, Vocoder, Voice FX, Vocal Tune, X-Trem |
| Delay and reverb | Analog Delay, Beat Delay, Groove Delay, IR Maker, Mixverb, Open AIR, Room Reverb, Studio Verb, Surround Delay |
| Metering and utilities | Binaural Pan, Dual Pan, Level Meter, Mixtool, Phase Meter, Scope, Spectrum Meter, Tone Generator, Tuner, VU Meter |
| External effects | External Effect Mono, External Effect Stereo |
| Routing, Mix FX, and spatial | Console Shaper, Splitter, Surround Panner |

### Third-party VST3 plugins

#### FabFilter

The 23 products below have verified preset workflows. Parameter and MIDI coverage varies by product. [FabFilter preset library](docs/PLUGIN_WORKFLOWS.md#fabfilter-preset-library) · [MIDI controller mapping](docs/PLUGIN_WORKFLOWS.md#fabfilter-midi-controller-mapping)

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

Preset application and MIDI routing were verified on version 2.1.5. [Serum 2 and Serum 2 FX workflows](docs/PLUGIN_WORKFLOWS.md#serum-2-and-serum-2-fx)

- Serum 2
- Serum 2 FX

This list will grow as more plugins are verified.

## Optional native add-on

Install the add-on for native parameters, supported plugin state and presets,
and insert routing. It targets Studio Pro 8.1.1 build 113103 on Apple Silicon.
Quit Studio Pro and stop the server or `hold` before installing. Build with
Rust 1.88.0, Python 3, and the macOS command-line tools.

```sh
sh native/build.sh
./target/release/fender-studio-pro-mcp native-install artifacts/native/fender-studio-pro-mcp.native.mac
```

The installation guide covers troubleshooting, update, rollback, uninstall,
and the developer preview archive. [Installation and maintenance](docs/INSTALLATION.md)

## Inspect files offline

These commands run without connecting to Studio Pro.

```sh
./target/release/fender-studio-pro-mcp song-read /path/to/Scratch.song
./target/release/fender-studio-pro-mcp audio-analyze /path/to/render.wav
./target/release/fender-studio-pro-mcp local-catalog --plugin 'Pro EQ'
```

`song-read` reads the last saved song. `audio-analyze` measures WAV loudness, peaks, RMS, and stereo levels. `local-catalog` finds candidate parameter names in the installed app's metadata. Audio and MIDI device inventory is also available from the CLI. [Hardware commands](docs/HARDWARE.md)

## Validation

A live run checked 60 built-in plugin classes. Of those, 59 passed a representative parameter or graph edit with readback and restoration. Phase Meter passed an insert-slot bypass check. [Built-in plugin matrix](docs/BUILTIN_PLUGINS.md)

The workflow matrix tracks state readback, Save and reopen, events, and signal checks on Studio Pro 8.1.1. Live DAW integration has been tested on Apple Silicon Macs. Windows, Intel Mac, and Linux remain unverified. [Current workflow verification](docs/BUILTIN_PLUGINS.md#current-workflow-verification) · [Live validation report](docs/VALIDATION.md) · [Plugin-state validation](docs/PLUGIN_STATE_VALIDATION.md)

The detailed notes include run outcomes, remaining gaps, and links to evidence. [Built-in plugin experiment notes](docs/BUILTIN_PLUGIN_EXPERIMENTS.md)

## Project docs

- How the Rust server, in-DAW JavaScript bridge, and CoreMIDI connection work. [Architecture](docs/ARCHITECTURE.md)
- Continue an existing work queue or add support for another plugin. [Resume guide](docs/RESUME.md) · [MCP development guide](docs/MCP_DEVELOPMENT.md)
- Impact factory preset application and mixer recovery. [Verified Impact preset procedure](docs/IMPACT_PRESETS.md)
- Development setup, offline checks, and private security reports. [Contributing](CONTRIBUTING.md) · [Security](SECURITY.md)

## License

This independent project uses the MIT license and retains notices for copied assets. Fender Studio Pro and Studio One are trademarks of their respective owners. [MIT license](LICENSE) · [Third-party notices](THIRD_PARTY_NOTICES.md)

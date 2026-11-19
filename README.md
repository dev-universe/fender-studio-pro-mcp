<h1 align="center">Fender Studio Pro MCP</h1>

**그래서 이게 뭔가요?** 펜더 스튜디오 프로에서 트랙을 만들거나 믹서를 조절하는 일을 채팅으로 AI에게 부탁할 수 있게 해주는 도구예요.

<p align="center">
  <strong>Control Fender Studio Pro from an MCP-compatible AI client.</strong><br>
  Experimental · Live-tested on Studio Pro 8.1.1 (Build 113103) on Apple Silicon Macs
</p>

<p align="center">
  <a href="#quick-start">Quick start</a> ·
  <a href="#usage">Usage</a> ·
  <a href="#optional-native-add-on">Native add-on</a> ·
  <a href="#validation">Validation</a>
</p>

---

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

### Troubleshooting

| Symptom | What to check |
| --- | --- |
| `controller_busy` | Stop the other MCP server, `hold` process, or live CLI command using the same profile. |
| MIDI source disconnected after a restart | Reselect **Receive From** while the server is running. |
| The bridge request times out on the Home screen | Open a scratch song and retry the connection check. |

### Update or uninstall

Quit Studio Pro and stop the server first. After updating the source, rebuild and run `install` again. If the native add-on changed, rebuild and reinstall it separately. To remove the installed assets, run `uninstall` for the bridge and `native-uninstall` for the add-on. Both preserve modified files.

## Optional native add-on

Install the add-on to enumerate native plugin parameters and use supported state capture, presets, and deeper plugin inspection. It requires the tested Studio Pro 8.1.1 arm64 build, Python 3, and the macOS command-line tools for the build.

Quit Studio Pro and stop the MCP server or `hold` before installing. From the repository root, run the build with the Rust 1.88.0 toolchain installed.

```sh
sh native/build.sh
./target/release/fender-studio-pro-mcp native-install artifacts/native/fender-studio-pro-mcp.native.mac
```

Open Studio Pro and the scratch song, then start the MCP client. For native reads and writes, carry the returned `instance.object_uid` as `expected_instance`. Enumerate parameters again after changing models, pads, or plugin focus. [Native add-on guide](native/addon/README.md)

### CV Instrument preset settings

Use `plugin_presets` with `kind: "instrument"` and `plugin: "CV Instrument"`
on the selected track. Carry `expected_document`, `expected_instance`, and
the index and `catalog_token` issued by `list`. `inspect_cv_preset_settings`
returns a byte count and SHA-256. `apply_cv_preset_settings` loads the primary
settings without loading the preset's port-setup record.

If you want to restore the original settings, capture them with `plugin_state`
before applying. After an `applied_unverified` receipt, read parameters and
port assignments to check the result. The 8.1.1 trial verified 97 writable
values, seven unassigned ports, and restoration. Testing with assigned
physical ports is pending. [Live evidence](docs/evidence/2026-11-20-cv-settings-apply-r106.json)

### FabFilter Twin 3 program changes

Use `midi_program_part_add` with the exact Twin 3 VST3 track, its current
`expected_instance` and `expected_document`, and MIDI `bank_msb`, `bank_lsb`,
and `program` numbers from 0 to 127. The track must be empty and exclusively
selected, with record arm and monitoring off. Stop at the song start and
disable looping. The tool adds a program-change part without musical notes.

With Twin 3's **Enable MIDI Program Changes** option active, playback of the
part can change the preset. Check the bank and program numbers in the current
preset browser, since they can change with the preset library. Actual Studio
Pro 8.1.1 playback of MSB 0, LSB 0, program 5 loaded **Ambient Mover** and
matched 1,509 independently observed settings. [R25 evidence](docs/evidence/2026-11-20-twin3-midi-r25.json) · [FabFilter documentation](https://www.fabfilter.com/help/twin/presets/loadingpresets)

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

The reports record the tested settings and the remaining gaps in model, preset, and hardware coverage. Live DAW integration has been tested on Apple Silicon Macs. Windows, Intel Mac, and Linux remain unverified. [Live validation report](docs/VALIDATION.md), [Plugin-state validation](docs/PLUGIN_STATE_VALIDATION.md)

The detailed notes include run outcomes, remaining gaps, and links to evidence. [Built-in plugin experiment notes](docs/BUILTIN_PLUGIN_EXPERIMENTS.md)

## Project docs

- How the Rust server, in-DAW JavaScript bridge, and CoreMIDI connection work. [Architecture](docs/ARCHITECTURE.md)
- The development queue and latest handoff. [Project status](docs/RESUME.md)
- Development setup, offline checks, and private security reports. [Contributing](CONTRIBUTING.md) · [Security](SECURITY.md)

## License

This independent project uses the MIT license and retains notices for copied assets. Fender Studio Pro and Studio One are trademarks of their respective owners. [MIT license](LICENSE) · [Third-party notices](THIRD_PARTY_NOTICES.md)

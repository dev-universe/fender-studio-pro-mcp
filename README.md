<h1 align="center">Fender Studio Pro MCP</h1>

**그래서 이게 뭔가요?**\
Fender Studio Pro에서 하는 대부분(이 되도록 열심히 노력 중)의 작업들을 채팅으로 AI에게 부탁할 수 있게 해주는 도구입니다.

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

### Verify a parameter through a separate read

Set `readback` to `true` on `plugin_parameter_set` with the canonical name from
`plugin_parameters`, the current `expected_document` and `expected_instance`,
and an explicit `dsp` or `controller` source. A verified native write keeps its
immediate `after` value and triggers one separate read of the same parameter
after a 150 ms wait. `readback.status` is `matched`, `mismatch`, or `unavailable`.
A mismatch or unavailable check returns `applied_unverified` and includes the
observed parameter or the reason the read could not be confirmed. A match
describes the value at that observation time. This option applies to ordinary
scalar writes. Studio Pro 8.1.1 was verified to report both a stable match and a
transient mismatch from the DSP source on FabFilter Pro-G 1.42.
[Live verification](docs/evidence/2026-11-20-plugin-parameter-readback.json)

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

A second 8.1.1 trial changed Gate mode, the first LFO waveform and the first
ADSR attack before applying `default`. Two independent reads confirmed that
the settings-only apply reset all three and matched all 97 writable values.
The trial restored the prior values and checked all 67 mixer channels and
seven unassigned CV ports throughout. [Three-setting verification](docs/evidence/2026-11-20-cv-default-three-settings.json)

### Live Impact outputs

Call `plugin_inspect` with `kind: "instrument"`, `plugin: "Impact"`, the
track name, `source: "dsp"`, `include_nodes: true`, and
`node_scope: "impact_outputs"`. Supply the current `expected_document` from
`host_status` and the `expected_instance` from an initial `plugin_inspect`.
The 8.1.1 macOS arm64 native add-on reports all 32 output names and whether
each activation is requested and acknowledged. You can use it with the Impact
editor closed. Use `channels_list` to see the associated mixer channels.

Use `impact_output_set` to change one of the 31 secondary outputs at indices
1 through 31. Provide the track, current
`expected_document` and `expected_instance`, `output_index` and `output_name`
from the inspected row, its current `expected_active` state, and the desired
`active` state. The tool checks both reported activation flags and preserves
the last active output. Disabling an output removes its mixer channel. Verify
your mixer routing with `channels_list` after a change.

### FabFilter preset library

Call `fabfilter_presets_list` with `{"product":"Pro-Q 4"}` to see preset names and their folders. The tool covers the 23 FabFilter products and versions in the compatibility checks. It reads names under `~/Documents/FabFilter/Presets` by default. For a library stored elsewhere, set `FSP_FABFILTER_PRESET_LIBRARY` to the absolute parent of the per-product directories when starting the MCP server. [FabFilter preset folders](https://www.fabfilter.com/help/pro-q/presets/howpresetsarestored)

For another page, send `offset` set to `next_offset` and `expected_catalog` set to the first page's `catalog_sha256`. The digest covers the listed names and folders. FabFilter assigns its own MIDI program numbers, and those can change as its library changes. Use `plugin_presets` for presets issued by Studio Pro. [FabFilter program numbering](https://www.fabfilter.com/help/pro-q/presets/loadingpresets)

### FabFilter MIDI controller mapping

Call `fabfilter_midi_mapping` with `{"product":"Micro","controller":7}` to look up the VST3 SDK mapping for CC 7. Values 128 and 129 select channel pressure and pitch bend. Program change (130) has no parameter in this SDK mapping. The result includes the tested VST3 version and the corresponding `studio_pro_name`. Compare the version and confirm the name with `plugin_parameters` on the current instance. The lookup itself sends no MIDI. [VST3 MIDI mapping interface](https://steinbergmedia.github.io/vst3_doc/vstinterfaces/classSteinberg_1_1Vst_1_1IMidiMapping.html)

FabFilter Micro 1.32 was verified in Studio Pro 8.1.1 on macOS arm64. Use `plugin_parameters` to read the bound instance and `plugin_parameter_set` to change a writable value. All 145 writable values were changed, read back, and restored individually. The separate program value `vstp1073741959` was verified as `0 → 1 → 0`. That round trip also required restoring a companion value and ended with all 146 reported values at baseline. [Micro verification details](docs/evidence/2026-11-20-fabfilter-micro-controls.json)

FabFilter Pro-Q 3 3.29 passed individual change, independent readback and restoration of all 518 writable parameters in Studio Pro 8.1.1. The final readback matched all 519 reported parameters. [Pro-Q 3 verification details](docs/evidence/2026-11-20-fabfilter-proq3-controls.json)

FabFilter Pro-Q 4 4.13 passed individual change, independent readback and restoration of all 736 writable parameters in Studio Pro 8.1.1. The final readback matched all 737 reported parameters. [Pro-Q 4 verification details](docs/evidence/2026-11-20-fabfilter-proq4-controls.json)

FabFilter Pro-R 2 2.06 passed individual change, independent readback and restoration of all 272 writable parameters in Studio Pro 8.1.1. The final readback matched all 273 reported parameters. [Pro-R 2 verification](docs/evidence/2026-11-20-fabfilter-pror2-controls.json)

FabFilter Saturn 1.31 passed individual change, independent readback and restoration of all 1,035 writable parameters in Studio Pro 8.1.1. The full restored snapshot matched all 1,036 reported parameters. [Saturn verification](docs/evidence/2026-11-20-fabfilter-saturn-controls.json)

FabFilter Saturn 2 2.13 passed individual change, independent readback and restoration of all 1,087 writable parameters in Studio Pro 8.1.1. The full restored snapshot matched all 1,088 reported parameters. [Saturn 2 verification](docs/evidence/2026-11-20-fabfilter-saturn2-controls.json)

FabFilter Simplon 1.42 passed individual change, independent readback and restoration of all 155 writable parameters in Studio Pro 8.1.1. Four virtual MIDI CC writes also changed a linked filter control. Restoring those controls and the affected internal value returned all 156 reported parameters to baseline. [Simplon verification](docs/evidence/2026-11-20-fabfilter-simplon-controls.json)

FabFilter One 3.52 passed individual change, independent readback and restoration for 164 of 167 writable parameters in Studio Pro 8.1.1. The verified values include all 31 named controls and 127 of 128 virtual MIDI CC values. Direct writes to the modulation wheel row, program list and legacy CC remain unverified. [One verification](docs/evidence/2026-11-20-fabfilter-one-controls.json)

Some parameters share the display title `MIDI CC`. Use the SDK mapping to choose the parameter ID for a particular CC number. The legacy row `vstp1073741828` briefly accepted a value in Pro-G, Pro-C 2, Pro-C 3 and Pro-DS, then returned to baseline on an independent `plugin_parameters` call. Treat that row as transient in these versions. In a separate SDK test, the Pro-G processor received the input and emitted its own value under the same ID. [Legacy MIDI parameter verification](docs/evidence/2026-11-20-fabfilter-legacy-midi.json)

FabFilter Pro-Q 2 2.30 passed individual change, independent readback and restoration of all 327 writable parameters in Studio Pro 8.1.1. The legacy `vstp1073741828` row also passed on this version. [Pro-Q 2 verification details](docs/evidence/2026-11-20-fabfilter-proq2-controls.json)

FabFilter Timeless 2 2.41 passed individual change, independent readback and restoration for all 841 writable parameters in Studio Pro 8.1.1. The checks covered 128 virtual MIDI CC values and confirmed that a separate read observed the changed legacy controller value before restoration. [Timeless 2 verification](docs/evidence/2026-11-20-fabfilter-timeless2-controls.json)

FabFilter Twin 2 2.41 passed individual change, independent readback and restoration for 1,017 of 1,019 writable parameters in Studio Pro 8.1.1. The verified values include all 883 named controls and all 128 virtual MIDI CC values. During MIDI CC changes, 20 linked named controls moved and 15 needed separate restoration. Direct writes to the program list and legacy CC remain unverified. [Twin 2 verification](docs/evidence/2026-11-20-fabfilter-twin2-controls.json)

FabFilter Timeless 3 3.10 passed individual change, independent readback and restoration for all 1,143 writable parameters in Studio Pro 8.1.1. The checks covered 128 virtual MIDI CC values and confirmed that a separate read observed the changed legacy controller value before restoration. [Timeless 3 verification](docs/evidence/2026-11-20-fabfilter-timeless3-controls.json)

FabFilter Twin 3 3.10 passed individual change, independent readback and restoration of all 1,509 writable parameters in Studio Pro 8.1.1, including all 128 virtual MIDI CC values. A CC write can also change a parameter linked by MIDI Learn. Use `plugin_parameters` to check the linked value on the current instance. The verification restored the original values and matched all 1,510 reported parameters. [Full Twin 3 verification](docs/evidence/2026-11-20-fabfilter-twin3-controls.json) · [Virtual CC details](docs/evidence/2026-11-20-fabfilter-twin3-midi-cc.json) · [FabFilter factory MIDI CC table](https://www.fabfilter.com/help/twin/support/midi)

FabFilter Pro-R 1.20 passed individual change, independent readback and restoration of all 221 writable parameters in Studio Pro 8.1.1. Restoration matched all 222 reported parameters, including the independent check of the legacy MIDI CC value. [Pro-R verification](docs/evidence/2026-11-20-fabfilter-pror-controls.json)

### FabFilter Twin 3 factory preset file

Twin 3 3.10 loaded the official `Ambient Mover.ffp` using **Open Other Preset** in the plug-in editor. The reader observed 433 changed parameters in Studio Pro 8.1.1. Restoring the original state with `plugin_state` returned 1,509 of 1,510 values. One verified `plugin_parameter_set` write returned the remaining internal float to zero. The complete typed readback, independent DSP digest and recaptured host component fingerprint all matched the baseline. This verification covers the named file and version. [Verification details](docs/evidence/2026-11-20-fabfilter-twin3-vendor-preset.json) · [FabFilter loading instructions](https://prod.fabfilter.com/help/twin/presets/loadingpresets)

### FabFilter Pro-Q 4 factory preset file

Pro-Q 4 4.13 loaded the official `Musical Low Cut Filter.ffp` using **Open Other Preset** in the plug-in editor. Studio Pro 8.1.1 reported 110 changed parameters. Restoring the original state with `plugin_state` returned 736 of 737 values. A single `plugin_parameter_set` write returned the remaining internal value to normalized zero. The complete typed readback, independent DSP digest and recaptured host component fingerprint matched the baseline. This verification covers the named file and version. [Verification details](docs/evidence/2026-11-20-fabfilter-proq4-vendor-preset.json) · [FabFilter loading instructions](https://prod.fabfilter.com/help/pro-q/presets/loadingpresets)

### FabFilter instrument program changes

Use `midi_program_part_add` with an exact FabFilter VST3 track, its current
`expected_instance` and `expected_document`, and MIDI `bank_msb`, `bank_lsb`,
and `program` numbers from 0 to 127. Choose `instrument` as `one`, `twin_2`,
or `twin_3`. Leaving it out selects Twin 3. The track must be empty and
exclusively selected, with record arm and monitoring off. Stop at the song
start and disable looping. The tool adds a program-change part without notes.

Enable **MIDI Program Changes** in the instrument's preset menu. Read the bank
and program numbers from that menu, since the library can change the mapping.
The part receipt verifies the part and instrument route. Read the plugin after
playback to confirm the preset. [FabFilter One manual](https://www.fabfilter.com/help/one/presets/loadingpresets) · [FabFilter Twin manual](https://www.fabfilter.com/help/twin/presets/loadingpresets)

| Studio Pro 8.1.1 trial | Program at bank 0/0 | Preset shown after playback | Matching writable values |
| --- | ---: | --- | ---: |
| One 3.52 | 1 | Sawtooth | 165/167 |
| Twin 2.41 | 5 | By Random Trance DJM | 1,017/1,019 |
| Twin 3.10 | 5 | Ambient Mover | 1,509/1,509 |

One and Twin 2 differed from a manual preset selection in the program parameter
and one internal parameter. In a separate One trial, program 0 created the part
and playback left **Default Sound** selected. [One and Twin 2 evidence](docs/evidence/2026-11-20-fabfilter-program-parts.json) · [Twin 3 evidence](docs/evidence/2026-11-20-twin3-midi-r25.json)

### External Effect Stereo Send

With the effect editor open, use `plugin_inspect` to identify the current instance and `external_port_menu` with `port: "send"` to read the current menu. The checked row identifies the logical Send choice. In the Studio Pro 8.1.1 test with Main's physical output unassigned, choosing Main in the editor changed the checkmark and controller display while the raw port value stayed `-1`. Selecting None in the editor restored all 24 controller rows. [Live verification](docs/evidence/2026-11-20-external-stereo-send-observation.json)

## Inspect files offline

These commands run without connecting to Studio Pro.

```sh
./target/release/fender-studio-pro-mcp song-read /path/to/Scratch.song
./target/release/fender-studio-pro-mcp audio-analyze /path/to/render.wav
./target/release/fender-studio-pro-mcp local-catalog --plugin 'Pro EQ'
```

`song-read` reads the last saved song. `audio-analyze` measures WAV loudness, peaks, RMS, and stereo levels. `local-catalog` finds candidate parameter names in the installed app's metadata. Audio and MIDI device inventory is also available from the CLI. [Hardware commands](docs/HARDWARE.md)

For a supported saved Impact instance, `song_read` and `song-read` include `instrument_outputs` on the plugin. Each output reports its saved enabled state, speaker layout and mixer channel ID and name when connected. The reader checks both the saved output descriptors and mixer membership. If those disagree, it reports a warning and leaves the output graph absent. The same fields were verified in Studio Pro 8.1.1 scratch copies with `Impact St 2` enabled and restored after reopening.

For saved pre-fader insert racks with supported Splitter routing, `song_read` and `song-read` include `mixer[].insert_routing.nodes`. The ordered `successors` show where each node sends audio. Slot nodes include their saved rack index, slot ID and device name. The file reader reflects the last save. A warning identifies a rack whose links or slots could not be verified. [Studio Pro 8.1.1 verification](docs/evidence/2026-11-20-saved-insert-routing.json)

For the current insert rack, use `insert_chain_state` with `action: "capture"` and `include_routing: true`. Supply the mixer channel and current `expected_document`. Use `expected_folder` to pin the exact folder from `plugin_inspect`. `routing.status` is `observed` when the host capture includes supported directed links, `absent` for a capture without graph metadata, or `unavailable` when the metadata cannot be verified. Slot nodes show the captured plugin class name and slot ID. Release the capture token when finished. [Live 8.1.1 verification](docs/evidence/2026-11-20-live-insert-routing.json)

After `insert_chain_state` restores a captured chain, read the current routing and bind the current slot and plugin instance IDs again. Studio Pro may replace both IDs. The `applied_unverified` receipt confirms the host accepted the load. Compare the current connections and plugin values to verify the result. [Nested restoration verification](docs/evidence/2026-11-20-insert-chain-restore.json)

## Validation

A live run checked 60 built-in plugin classes. Of those, 59 passed a representative parameter or graph edit with readback and restoration. Phase Meter passed an insert-slot bypass check. [Built-in plugin matrix](docs/BUILTIN_PLUGINS.md)

The reports record the tested settings and the remaining gaps in model, preset, and hardware coverage. Live DAW integration has been tested on Apple Silicon Macs. Windows, Intel Mac, and Linux remain unverified. [Live validation report](docs/VALIDATION.md), [Plugin-state validation](docs/PLUGIN_STATE_VALIDATION.md)

The detailed notes include run outcomes, remaining gaps, and links to evidence. [Built-in plugin experiment notes](docs/BUILTIN_PLUGIN_EXPERIMENTS.md)

## Project docs

- How the Rust server, in-DAW JavaScript bridge, and CoreMIDI connection work. [Architecture](docs/ARCHITECTURE.md)
- Completed checks and the remaining built-in and FabFilter work. [Latest handoff](https://github.com/dev-universe/fender-studio-pro-mcp/blob/feature/cv-structured-lifecycle/docs/LOOP_HANDOFF_2026-11-20_LIVE.md) · [Resume guide](docs/RESUME.md)
- Impact factory preset application and mixer recovery. [Verified Impact preset procedure](docs/IMPACT_PRESETS.md)
- Development setup, offline checks, and private security reports. [Contributing](CONTRIBUTING.md) · [Security](SECURITY.md)

## License

This independent project uses the MIT license and retains notices for copied assets. Fender Studio Pro and Studio One are trademarks of their respective owners. [MIT license](LICENSE) · [Third-party notices](THIRD_PARTY_NOTICES.md)

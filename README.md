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

For a Pedalboard insert, use `plugin_inspect` with `source: "controller"`, `include_nodes: true`, and `node_scope: "plugin_controller"`. Bind the exact channel, slot, plugin, instance, and document. The bounded controller graph can show its `StompItemModel` under `stomps`. [Pedalboard controller verification](docs/evidence/2026-11-24-pedalboard-controller-inspection.json)

In Studio Pro 8.1.1 on Apple Silicon, `pedalboard_slot_swap` exchanges two positions in the standalone Pedalboard. Stop playback and recording. Read `opt.stompmodel1` through `opt.stompmodel8` with `source: "dsp"` and pass the current document, exact instance, and both displayed model names. Use the literal `" - "` for an empty slot. Insert slots start at zero and pedal positions start at one.

- Adjacent occupied positions with different models can be exchanged directly. Tests covered both directions at positions 1–2 and 7–8, and separately verified Save, reopen, and Undo. [Adjacent exchange](docs/evidence/2026-11-24-pedalboard-slot-swap.json) · [Save and reopen](docs/evidence/2026-11-24-pedalboard-swap-persistence.json) · [Host Undo](docs/evidence/2026-11-24-pedalboard-slot-swap-undo.json)
- An empty position, two identical model names, or nonadjacent positions require a fresh `plugin_state` capture for the exact insert. Confirm that `pedalboard_rack` shows two distinct payloads and pass the capture's `sha256` as `expected_rack_sha256`. Capture again after the exchange and compare all eight slots, then release both tokens. Identical-model DSP rows and a vacant-slot active alias can lag behind the serialized rack, so use payload fingerprints for this check. [Guarded exchanges](docs/evidence/2026-11-25-pedalboard-guarded-exchange.md) · [Empty slot restart](docs/evidence/2026-11-25-pedalboard-empty-slot-persistence.md) · [Same-model restart](docs/evidence/2026-11-25-pedalboard-same-model-persistence.md)

Ampire has a separate pre/post divider controlled by `opt.stomporder`. The standalone Pedalboard has a continuous rack. [Divider verification](docs/evidence/2026-11-25-pedalboard-ampire-divider.md)

The client can inspect the open song, edit tracks and MIDI notes, adjust the mixer and transport, and change supported built-in plugin settings. Try these prompts in your scratch song.

- "Which mixer channels are muted in this song?"
- "Set the tempo to 100 BPM and read it back."

For edits, the client passes the current `document` token from `host_status` as `expected_document`. Read a fresh token after switching or reopening a song. Call `operations_list` to see the available operations and their arguments.

`applied_verified` includes operation-specific readback. `applied_unverified` records an acknowledged edit with incomplete verification. `partially_applied` means some changes took effect. If the client receives `outcome_unknown`, check the current state before repeating the request.

To control an instrument on a track, read its exact name with `tracks_list`. Call `track_select` with that name and the current document token. Check that `tracks_list` shows only that track selected before inspecting or editing the instrument.

For a Multi Instrument with one stock Mai Tai, stop playback and recording and keep its uniquely named Music track selected. Use `plugin_inspect` with `kind: "instrument"` and `track: "Your track name"` to get the Multi Instrument's native instance UID. Repeat with the observed `plugin` and `expected_instance`, plus `include_nodes: true` and `node_scope: "plugin_controller"`, to read the Instruments folder, Graph, original `Inst01` slot, and its Mai Tai Device UIDs. Pass those five UIDs to `plugin_multi_add_mai_tai` with the current document and track. After `applied_unverified` or `outcome_unknown`, inspect the current children and mixer before issuing any further edit. After an explicit Undo, repeat the inspection before issuing a new add request. [Studio Pro 8.1.1 validation](docs/evidence/2026-11-26-multi-add-mai-tai-live.md).

### Troubleshooting

| Symptom | What to check |
| --- | --- |
| `controller_busy` | Stop the other MCP server, `hold` process, or live CLI command using the same profile. |
| MIDI source disconnected after a restart | Reselect **Receive From** while the server is running. |
| The bridge request times out on the Home screen | Open a scratch song and retry the connection check. |

### Update or uninstall

Quit Studio Pro and stop the server first. After updating the source, rebuild and run `install` again. If the native add-on changed, rebuild and reinstall it separately. The installers report backup paths for the files they replace.

To restore an earlier build, run its `install` command and pass its native package to `native-install`. Use the same Studio Pro profile and MCP data directory. Run `doctor` from the restored build to check the files.

If either the installed bundle or the package contains `CodeResources`, use a CLI that supports signed bundle resources for native install, restore and uninstall. A CLI built before that support may refuse to restore a legacy bundle from a signed installation. Use the newer CLI's `native-install` with the legacy package to preserve the signature resource in a backup.

To remove the installed assets, run `uninstall` for the bridge and `native-uninstall` for the add-on. Both preserve modified files. [macOS installer lifecycle verification](docs/evidence/2026-11-25-macos-installer-lifecycle.md)

### Build a macOS arm64 archive

On Apple Silicon, run `sh scripts/package-macos-arm64.sh`. It writes a
versioned archive and a SHA-256 sidecar to `artifacts/macos-arm64`. You can
pass a different output directory as the first argument. To check a copied
archive, run `shasum -a 256 -c` with its `.sha256` file from that directory.
The archive contains the CLI, its native add-on, and a build manifest. The
archive's native bundle has an ad-hoc resource signature. Developer ID and
notarization are separate from this build.

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
transient mismatch from the DSP source on FabFilter Pro-G 1.42. External Effect
Stereo's phase switch matched the controller read after changing and restoring it.
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
Set `include_color: true` to also read each strip's signed 32-bit
`color_argb` value. Studio Pro can link an instrument's primary strip
color to its arrangement track. [Impact color verification](docs/evidence/2026-11-21-impact-primary-mixer-color.json)

Set `include_selection: true` to read `console_selected` for each uniquely
identified audio strip. The value is `null` when the Console cannot match
the strip. Use `tracks_list` to check arrangement selection. In Studio Pro
8.1.1, selecting an instrument channel in the Console can change arrangement
selection, record arm, and monitoring. [Impact Console verification](docs/evidence/2026-11-21-impact-console-selection.json)

Use `impact_output_set` to change one of the 31 secondary outputs at indices
1 through 31. Provide the track, current
`expected_document` and `expected_instance`, `output_index` and `output_name`
from the inspected row, its current `expected_active` state, and the desired
`active` state. The tool checks both reported activation flags and preserves
the last active output. Disabling an output removes its mixer channel. Verify
your mixer routing with `channels_list` after a change.

### Serum 2 and Serum 2 FX

Call `host_status` for the current document token. Use `serum2_load` with
`target: {"variant":"instrument","track":"Serum Keys"}` to create a Music
track, or `target: {"variant":"fx","channel":"Guitar"}` to add the effect to
an existing mixer channel. Include `expected_document` with either request.
The tool selects the exact Xfer VST3 class. Read the track or inserts after
the operation to confirm placement.

Select the instrument track before inspecting it. Use `plugin_inspect` to
get the native instance ID, then use `plugin_parameters` and
`plugin_parameter_set` on that exact instance. Serum 2 and Serum 2 FX 2.1.5
were tested in Studio Pro 8.1.1 on Apple Silicon. In the Serum editor, you
can use **Menu → Load Preset** to open compatible `.SerumPreset` files.
[Studio Pro verification](docs/evidence/2026-11-25-serum2-studio-pro.md)

Use `serum2_preset_apply` to apply a processor-only `.vstpreset` file for
Serum 2 or Serum 2 FX 2.1.5. Supply the absolute local `path`, file
`expected_sha256` in lowercase, current `expected_document`, and native
`expected_instance`. Use the same instrument selector as `serum2_load`. For
FX, include the zero-based `slot` in the `target`. Stop playback and recording
before applying a preset. Use a separate `plugin_parameters` call to confirm
any change. For an `unknown` receipt, read the target before retrying.
Studio Pro displays the temporary file name for this processor-only import.
[Direct preset verification](docs/evidence/2026-11-25-serum2-mcp-vst3-preset.md)

### FabFilter preset library

Call `fabfilter_presets_list` with `{"product":"Pro-Q 4"}` to see preset names and their folders. The tool covers the 23 FabFilter products and versions in the compatibility checks. It reads names under `~/Documents/FabFilter/Presets` by default. For a library stored elsewhere, set `FSP_FABFILTER_PRESET_LIBRARY` to the absolute parent of the per-product directories when starting the MCP server. [FabFilter preset folders](https://www.fabfilter.com/help/pro-q/presets/howpresetsarestored)

For another page, send `offset` set to `next_offset` and `expected_catalog` set to the first page's `catalog_sha256`. The digest covers the listed names and folders. FabFilter assigns its own MIDI program numbers, and those can change as its library changes. Use `plugin_presets` for presets issued by Studio Pro. [FabFilter program numbering](https://www.fabfilter.com/help/pro-q/presets/loadingpresets)

Use `fabfilter_preset_apply` for local `.ffp` files in Studio Pro 8.1.1 on Apple Silicon. The supported VST3 versions are Micro 1.32, One 3.52, Pro-C 2 2.22, Pro-DS 1.32, Pro-G 1.42, Pro-L 1.37, Pro-L 2 2.26, Pro-MB 1.33, Pro-Q 2 2.30, Pro-Q 3 3.29, Pro-Q 4 4.13, Pro-C 3 3.02, Pro-R 1.20, Pro-R 2 2.06, Saturn 1.31, Saturn 2 2.13, Simplon 1.42, Timeless 2 2.41, Timeless 3 3.10, Twin 2 2.41, Twin 3 3.10, Volcano 2 2.41, and Volcano 3 3.09. Micro reads its FMic version 1 factory format. One reads FFPr versions 0 and 1. Pro-C 2 reads FC2p version 2 and legacy FPrC version 1. Pro-DS reads its FPSr version 2 factory format. Pro-G reads its FPGr version 2 factory format. Pro-L reads its FPLr version 3 factory format. Pro-L 2 reads its FL2p version 2 factory format. Pro-MB reads its FPMb version 3 factory format. Pro-Q 2 reads its FQ2p version 2 factory format. Pro-Q 3 reads its FQ3p version 4 factory format. Pro-R reads its version 4 binary factory format. Pro-R 2 reads its current text factory format and the legacy binary format included in its factory library. Saturn reads FSat versions 1, 2, and 4. Saturn 2 reads FS2a versions 2–4 and legacy FSat version 4. Simplon reads its FSim version 1 factory format. Timeless 2 reads FTms versions 3–5, FTim version 2, and FDel version 1. Timeless 3 reads F3Ts versions 9 and 10. Twin 2 reads FT2i versions 6 and 8 and FTwi versions 1, 3, and 4. Volcano 2 reads FV2l versions 1 and 2 and FVol version 1. Volcano 3 reads FV3l versions 3–5. Open the intended plug-in's editor and supply its `expected_instance` and `expected_document`. For Micro, Pro-C 2, Pro-DS, Pro-G, Pro-L, Pro-L 2, Pro-MB, Pro-Q 2, Pro-Q 3, Pro-Q 4, Pro-C 3, Pro-R, Pro-R 2, Saturn, Saturn 2, Simplon, Timeless 2, Timeless 3, Volcano 2, or Volcano 3, supply the effect's `channel` and `slot`. For One, Twin 2, or Twin 3, select only the intended instrument track and supply its name as `track`. Set `expected_catalog` to the listing's `catalog_sha256` and `index` to the listing's `offset` plus the entry's zero-based position. The result carries the selected file's SHA-256. Use a separate `plugin_parameters` or `plugin_parameter_digest` request to confirm the change. [Micro verification](docs/evidence/2026-11-24-fabfilter-micro-library-preset-apply.json) · [One verification](docs/evidence/2026-11-24-fabfilter-one-library-preset-apply.json) · [Pro-C 2 verification](docs/evidence/2026-11-24-fabfilter-proc2-library-preset-apply.json) · [Pro-DS verification](docs/evidence/2026-11-24-fabfilter-prods-library-preset-apply.json) · [Pro-G verification](docs/evidence/2026-11-24-fabfilter-prog-library-preset-apply.json) · [Pro-L verification](docs/evidence/2026-11-24-fabfilter-prol-library-preset-apply.json) · [Pro-L 2 verification](docs/evidence/2026-11-24-fabfilter-prol2-library-preset-apply.json) · [Pro-MB verification](docs/evidence/2026-11-24-fabfilter-promb-library-preset-apply.json) · [Pro-Q 2 verification](docs/evidence/2026-11-24-fabfilter-proq2-library-preset-apply.json) · [Pro-Q 3 verification](docs/evidence/2026-11-24-fabfilter-proq3-library-preset-apply.json) · [Pro-Q 4 verification](docs/evidence/2026-11-21-fabfilter-proq4-library-preset-apply.json) · [Pro-C 3 verification](docs/evidence/2026-11-21-fabfilter-proc3-library-preset-apply.json) · [Pro-R verification](docs/evidence/2026-11-21-fabfilter-pror-library-preset-apply.json) · [Pro-R 2 verification](docs/evidence/2026-11-21-fabfilter-pror2-library-preset-apply.json) · [Saturn verification](docs/evidence/2026-11-24-fabfilter-saturn-library-preset-apply.json) · [Saturn 2 verification](docs/evidence/2026-11-24-fabfilter-saturn2-library-preset-apply.json) · [Simplon verification](docs/evidence/2026-11-24-fabfilter-simplon-library-preset-apply.json) · [Timeless 2 verification](docs/evidence/2026-11-24-fabfilter-timeless2-library-preset-apply.json) · [Timeless 3 verification](docs/evidence/2026-11-24-fabfilter-timeless3-library-preset-apply.json) · [Twin 2 verification](docs/evidence/2026-11-24-fabfilter-twin2-library-preset-apply.json) · [Twin 3 verification](docs/evidence/2026-11-21-fabfilter-twin3-library-preset-apply.json) · [Volcano 2 verification](docs/evidence/2026-11-24-fabfilter-volcano2-library-preset-apply.json) · [Volcano 3 verification](docs/evidence/2026-11-24-fabfilter-volcano3-library-preset-apply.json)

For Pro-Q 4 VST3 4.13 on Apple Silicon, `plugin_inspect` with the expected instance can report `native.capabilities.fabfilter_editor_owner`. A `matching_attached_editor_count` of 1 means Studio Pro attached the editor to that instance. A count of 0 means that instance currently has no attached editor. `other_same_product_attached_editors` counts attached editors for other Pro-Q 4 instances in the host. [Actual editor ownership verification](docs/evidence/2026-11-21-fabfilter-proq4-editor-ownership.json)

The same observation covers FabFilter Twin 3 VST3 3.10. Select only the intended instrument track before using `plugin_inspect` with its expected instance. `other_same_product_attached_editors` then counts other Twin 3 editors. [Twin 3 editor ownership verification](docs/evidence/2026-11-21-fabfilter-twin3-editor-ownership.json)

### FabFilter MIDI controller mapping

Call `fabfilter_midi_mapping` with `{"product":"Micro","controller":7}` to look up the VST3 SDK mapping for CC 7. Values 128 and 129 select channel pressure and pitch bend. Program change (130) has no parameter in this SDK mapping. The result includes the tested VST3 version and the corresponding `studio_pro_name`. Compare the version and confirm the name with `plugin_parameters` on the current instance. The lookup itself sends no MIDI. [VST3 MIDI mapping interface](https://steinbergmedia.github.io/vst3_doc/vstinterfaces/classSteinberg_1_1Vst_1_1IMidiMapping.html)

FabFilter Micro 1.32 was verified in Studio Pro 8.1.1 on macOS arm64. Use `plugin_parameters` to read the bound instance and `plugin_parameter_set` to change a writable value. All 145 writable values were changed, read back, and restored individually. The separate program value `vstp1073741959` was verified as `0 → 1 → 0`. That round trip also required restoring a companion value and ended with all 146 reported values at baseline. [Micro verification details](docs/evidence/2026-11-20-fabfilter-micro-controls.json)

FabFilter Pro-Q 3 3.29 passed individual change, independent readback and restoration of all 518 writable parameters in Studio Pro 8.1.1. The final readback matched all 519 reported parameters. [Pro-Q 3 verification details](docs/evidence/2026-11-20-fabfilter-proq3-controls.json)

FabFilter Pro-Q 4 4.13 passed individual change, independent readback and restoration of all 736 writable parameters in Studio Pro 8.1.1. The final readback matched all 737 reported parameters. [Pro-Q 4 verification details](docs/evidence/2026-11-20-fabfilter-proq4-controls.json)

FabFilter Pro-R 2 2.06 passed individual change, independent readback and restoration of all 272 writable parameters in Studio Pro 8.1.1. The final readback matched all 273 reported parameters. [Pro-R 2 verification](docs/evidence/2026-11-20-fabfilter-pror2-controls.json)

FabFilter Saturn 1.31 passed individual change, independent readback and restoration of all 1,035 writable parameters in Studio Pro 8.1.1. The full restored snapshot matched all 1,036 reported parameters. [Saturn verification](docs/evidence/2026-11-20-fabfilter-saturn-controls.json)

FabFilter Saturn 2 2.13 passed individual change, independent readback and restoration of all 1,087 writable parameters in Studio Pro 8.1.1. The full restored snapshot matched all 1,088 reported parameters. [Saturn 2 verification](docs/evidence/2026-11-20-fabfilter-saturn2-controls.json)

FabFilter Simplon 1.42 passed individual change, independent readback and restoration of all 155 writable parameters in Studio Pro 8.1.1. Four virtual MIDI CC writes also changed a linked filter control. Restoring those controls and the affected internal value returned all 156 reported parameters to baseline. [Simplon verification](docs/evidence/2026-11-20-fabfilter-simplon-controls.json)

FabFilter One 3.52 passed individual change, independent readback and restoration for all 167 writable parameters in Studio Pro 8.1.1. These include all 31 named controls, all 128 virtual MIDI CC values, the program list and the legacy CC. With the transport stopped, the program list and its read-only counter changed from 0 to 1 and returned to 0. The program change and modulation wheel each required a separate companion correction. All 168 reported values returned to baseline in both trials. [One writable controls](docs/evidence/2026-11-20-fabfilter-one-controls.json) · [Direct program list verification](docs/evidence/2026-11-20-fabfilter-one-direct-program.json) · [Legacy MIDI CC verification](docs/evidence/2026-11-20-fabfilter-one-legacy-control.json) · [Modulation wheel verification](docs/evidence/2026-11-20-fabfilter-one-modulation-wheel.json)

Some parameters share the display title `MIDI CC`. Use the SDK mapping to choose the parameter ID for a particular CC number. For Pro-G 1.42, Pro-C 2 2.22, Pro-C 3 3.02, Pro-DS 1.32, Pro-L 1.37, Pro-L 2 2.26 and Pro-MB 1.33, the legacy row `vstp1073741828` briefly accepted a value in Studio Pro 8.1.1 and returned to zero by the 150 ms readback. An independent parameter read confirmed zero. Use `plugin_parameter_set` with `readback: true` to detect this behavior and treat `applied_unverified` as a transient result for this row. Pro-R 1.20 kept the changed value through both readings and an explicit write restored it. [Studio Pro verification](docs/evidence/2026-11-24-fabfilter-legacy-readback.json) · [Separate VST3 processor experiment](docs/evidence/2026-11-20-fabfilter-legacy-midi.json)

FabFilter Pro-Q 2 2.30 passed individual change, independent readback and restoration of all 327 writable parameters in Studio Pro 8.1.1. The legacy `vstp1073741828` row also passed on this version. [Pro-Q 2 verification details](docs/evidence/2026-11-20-fabfilter-proq2-controls.json)

FabFilter Timeless 2 2.41 passed individual change, independent readback and restoration for all 841 writable parameters in Studio Pro 8.1.1. The checks covered 128 virtual MIDI CC values and confirmed that a separate read observed the changed legacy controller value before restoration. [Timeless 2 verification](docs/evidence/2026-11-20-fabfilter-timeless2-controls.json)

FabFilter Twin 2 2.41 passed individual change, independent readback and restoration for all 1,019 writable parameters in Studio Pro 8.1.1. The verified values include all 883 named controls, all 128 virtual MIDI CC values, the program list and the legacy CC. During MIDI CC changes, 20 linked named controls moved and 15 needed separate restoration. The final legacy CC check restored all 1,020 reported values. [Twin 2 writable controls](docs/evidence/2026-11-20-fabfilter-twin2-controls.json) · [Direct program list verification](docs/evidence/2026-11-20-fabfilter-twin2-direct-program.json) · [Legacy controller verification](docs/evidence/2026-11-20-fabfilter-twin2-legacy-control.json)

FabFilter Timeless 3 3.10 passed individual change, independent readback and restoration for all 1,143 writable parameters in Studio Pro 8.1.1. The checks covered 128 virtual MIDI CC values and confirmed that a separate read observed the changed legacy controller value before restoration. [Timeless 3 verification](docs/evidence/2026-11-20-fabfilter-timeless3-controls.json)

FabFilter Twin 3 3.10 passed individual change, independent readback and restoration of all 1,509 writable parameters in Studio Pro 8.1.1, including all 128 virtual MIDI CC values. A CC write can also change a parameter linked by MIDI Learn. Use `plugin_parameters` to check the linked value on the current instance. The verification restored the original values and matched all 1,510 reported parameters. [Full Twin 3 verification](docs/evidence/2026-11-20-fabfilter-twin3-controls.json) · [Virtual CC details](docs/evidence/2026-11-20-fabfilter-twin3-midi-cc.json) · [FabFilter factory MIDI CC table](https://www.fabfilter.com/help/twin/support/midi)

FabFilter Volcano 2 2.41 passed individual change, independent readback and restoration for all 893 writable parameters in Studio Pro 8.1.1. The checks covered 128 virtual MIDI CC values and confirmed that a separate read observed the changed legacy controller value before restoration. [Volcano 2 verification](docs/evidence/2026-11-20-fabfilter-volcano2-controls.json)

FabFilter Volcano 3 3.09 passed individual change, independent readback and restoration for all 1,032 writable parameters in Studio Pro 8.1.1. The checks covered 128 virtual MIDI CC values and confirmed that a separate read observed the changed legacy controller value before restoration. [Volcano 3 verification](docs/evidence/2026-11-20-fabfilter-volcano3-controls.json)

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

On Studio Pro 8.1.1 for Apple Silicon, `plugin_inspect` can report `native.capabilities.external_stereo_send_route.assigned` for the exact External Effect Stereo instance. `true` means the effect holds a logical Send route. `false` means Send is unassigned. A missing field means the native check was unavailable. The result covers routing assignment. Audio output and latency require a separate loopback test. [Live assignment verification](docs/evidence/2026-11-21-external-stereo-send-assignment.json)

With the effect editor open, use `plugin_inspect` to identify the current instance and `external_port_menu` with `port: "send"` to read the current menu. The checked row identifies the logical Send choice. In the Studio Pro 8.1.1 test with Main at zero gain, choosing Main in the editor changed the checkmark and controller display while the raw port value stayed `-1`. Selecting None in the editor restored all 24 controller rows. [Live verification](docs/evidence/2026-11-20-external-stereo-send-observation.json)

On the verified Apple Silicon build, the Stereo Send menu can include `held_send_route`. This records the logical assignment observed at the native lifecycle checks and confirms that the route was stable through the checked menu operations and cleanup. [Live menu lifecycle verification](docs/evidence/2026-11-21-external-stereo-send-menu-guard.json)

Use `external_stereo_send_set` to change the logical Send between `none` (없음) and `main` (메인). This tool requires the matching native extension on Studio Pro 8.1.1 for Apple Silicon with the Korean interface. Open the exact External Effect Stereo editor, stop transport, and disable click and preroll. Use a muted, disarmed audio track with input monitoring off and a Main channel at zero gain. The mixer must report a `null` Main output for this tool. In Studio Pro 8.1.1, the Audio I/O console can still show a physical assignment when the mixer reports `null`. The target track must route to Main. Return must be unassigned.

Supply the current document from `host_status`, the channel name and zero-based insert slot, and the current `expected_instance` from `plugin_inspect`. Use `external_port_menu` to check the current route before sending the change.

```json
{
  "channel": "Scratch Audio",
  "slot": 1,
  "expected_document": "<current document from host_status>",
  "expected_instance": "<exact effect UUID from plugin_inspect>",
  "expected_current": "none",
  "destination": "main"
}
```

A verified receipt covers one selection, the new menu checkmark and logical assignment, and a complete read of all 24 controller rows. An unknown outcome requires a fresh observation before any further change. Return, physical routing, audible output, and persistence require separate verification. [Live change and restore verification](docs/evidence/2026-11-21-external-stereo-send-set.json)

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

`insert_slot_move` moves an effect immediately after another slot in the same supported Inserts graph. Open that rack's Routing tab. Use `plugin_inspect` with `node_scope: "graph"` to get the graph, slot-node and effect UUIDs. Use a fresh capture with `include_routing: true` to identify the destination slot's single current successor. Supply the source channel, rack slot, plugin and instance UUID, plus the graph UUID, source node, destination node, and destination's current successor UUID. Send canonical uppercase UUIDs and the current `expected_document`. The move is available on the tested Apple Silicon build of Studio Pro 8.1.1 with the matching native extension. After `applied_unverified`, read the graph and plugin values independently. One host Undo restored the two parallel branches, all graph and effect IDs, and all 144 reported values in the nested Splitter trial. Studio Pro kept the moved effect selected after Undo. [Nested slot move and Undo verification](docs/evidence/2026-11-20-insert-slot-move.json)

A separate save and restart trial kept both Mixtools in series in the inner Splitter branch. The reopened Routing screen, seven directed links and 144 reported values matched the saved song. [Saved nested slot move verification](docs/evidence/2026-11-24-insert-slot-move-persistence.json)

To move an existing effect into an empty branch of a supported two-way Splitter, use the same tool with the Splitter UUID in `after_node` and its merger UUID in `following_node`. Set `branch` to `0` for the first branch or `1` for the second. In a fresh routing capture, the selected branch must lead directly to that merger. In the actual 8.1.1 trial, the second branch gained the effect, and one host Undo restored the original location, all graph and effect IDs, and all 144 reported values. Studio Pro kept the moved effect selected. [Empty branch move and Undo verification](docs/evidence/2026-11-20-insert-slot-empty-branch.json)

The first-branch move was also verified through one Save and a fresh Studio Pro process. All eight nodes, ten ordered links and 144 Mixtool values matched, and the previous document binding was refused. [First-branch persistence](docs/evidence/2026-11-26-splitter-first-lane-persistence.md)

After reopening a song, obtain a fresh document binding and read the current insert folder and graph UUIDs before editing. In the Studio Pro 8.1.1 restart trial, the saved links, slot and effect IDs, and all 144 values survived. The live folder and graph UUIDs changed, and the MCP rejected requests with the prior document or prior graph UUID before applying a change. [Save, restart and stale request verification](docs/evidence/2026-11-20-insert-routing-reopen.json)

`insert_splitter_add` inserts a default two-branch Splitter and its merger on the existing link from a supported effect slot to its immediate merger. Both new branches start empty and rejoin before reaching the original merger. Open the exact rack's Routing tab. Use `plugin_inspect` with `node_scope: "graph"` and a fresh routing capture to get the effect and graph UUIDs. Supply `channel`, `slot`, `plugin`, `expected_instance`, `expected_graph`, the effect slot-node UUID in `after_node`, the immediate merger UUID in `following_node`, and the current `expected_document`. Use canonical uppercase UUIDs. The matching native extension checks that a recorded Undo is available on the tested Apple Silicon build of Studio Pro 8.1.1. After `applied_unverified`, inspect the new links and plugin values. One host Undo restored the original graph and all 144 Mixtool values in the nested trial. [Splitter insertion and Undo verification](docs/evidence/2026-11-20-insert-splitter-add.json)

In Studio Pro 8.1.1, an added Splitter and both branches survived saving and restarting Studio Pro. The reopened rack matched all eight nodes, ten directed links, and 144 Mixtool values. Studio Pro issued new live document, insert folder, and graph bindings. Requests with the old document or graph were refused. [Splitter insertion persistence](docs/evidence/2026-11-24-insert-splitter-add-persistence.json)

`insert_splitter_remove` removes a supported inner Splitter and its merger. Each of its two ordered branches must contain one effect slot, and the other outer branch must lead directly to the outer merger. After removal the two effects remain connected in series. Use `plugin_inspect` with `node_scope: "graph"` and a fresh `insert_chain_state` capture with routing to bind the graph and ordered branch nodes. Choose either branch effect as the anchor and supply its `channel`, `slot`, `plugin` and `expected_instance`. Also supply `expected_graph`, `splitter_node`, `merger_node`, `first_slot_node`, `second_slot_node` and the current `expected_document`. Use canonical uppercase UUIDs. The tested Apple Silicon build of Studio Pro 8.1.1 and matching native extension support the operation with the Routing editor closed. Check links and effect values independently after `applied_unverified`. One host Undo restored the original parallel branches, IDs and all 144 reported values in the Mixtool trial. Studio Pro selected the second effect after Undo. [Nested Splitter removal and Undo verification](docs/evidence/2026-11-20-insert-splitter-remove.json)

The saved removal trial reopened the same song in a fresh Studio Pro session. The Routing editor showed the two Mixtools connected in series, and all 144 reported values matched the saved graph. Requests using the previous document or graph binding were refused before mutation. [Saved nested Splitter removal verification](docs/evidence/2026-11-24-insert-splitter-remove-persistence.json)

`insert_splitter_remove_empty_nested` removes the third Splitter and its paired merger in the supported nested Mixtool graph. Both lanes of the third Splitter must lead directly to its merger. Stop playback and recording, open the rack’s Routing tab, and bind slot 0 Mixtool with `plugin_inspect` and a fresh routing capture. Supply `channel`, `slot`, `plugin`, `expected_instance`, `expected_graph`, `splitter_node`, `merger_node`, `predecessor_slot_node`, `successor_merger_node`, and the current `expected_document`. Use canonical uppercase UUIDs. Inspect the links and effect values independently after `applied_unverified` or an unknown outcome before attempting any further edit. In Studio Pro 8.1.1 for Apple Silicon, the two Mixtools stayed in parallel and all 144 reported values survived one Save and a fresh process. [Empty nested Splitter removal verification](docs/evidence/2026-11-26-empty-nested-splitter-public.md)

After `insert_chain_state` restores a captured chain, read the current routing and bind the current slot and plugin instance IDs again. Studio Pro may replace both IDs. The `applied_unverified` receipt confirms the host accepted the load. Compare the current connections and plugin values to verify the result. [Nested restoration verification](docs/evidence/2026-11-20-insert-chain-restore.json)

## Validation

A live run checked 60 built-in plugin classes. Of those, 59 passed a representative parameter or graph edit with readback and restoration. Phase Meter passed an insert-slot bypass check. [Built-in plugin matrix](docs/BUILTIN_PLUGINS.md)

The reports record the tested settings and the remaining gaps in model, preset, and hardware coverage. Live DAW integration has been tested on Apple Silicon Macs. Windows, Intel Mac, and Linux remain unverified. [Live validation report](docs/VALIDATION.md), [Plugin-state validation](docs/PLUGIN_STATE_VALIDATION.md)

The detailed notes include run outcomes, remaining gaps, and links to evidence. [Built-in plugin experiment notes](docs/BUILTIN_PLUGIN_EXPERIMENTS.md)

## Project docs

- How the Rust server, in-DAW JavaScript bridge, and CoreMIDI connection work. [Architecture](docs/ARCHITECTURE.md)
- Continue an existing work queue or add support for another plugin. [Resume guide](docs/RESUME.md) · [MCP development guide](docs/MCP_DEVELOPMENT.md)
- Impact factory preset application and mixer recovery. [Verified Impact preset procedure](docs/IMPACT_PRESETS.md)
- Development setup, offline checks, and private security reports. [Contributing](CONTRIBUTING.md) · [Security](SECURITY.md)

## License

This independent project uses the MIT license and retains notices for copied assets. Fender Studio Pro and Studio One are trademarks of their respective owners. [MIT license](LICENSE) · [Third-party notices](THIRD_PARTY_NOTICES.md)

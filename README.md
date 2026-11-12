# Fender Studio Pro MCP

The user resumed the run on 2026-11-13. See the
[current progress](docs/PROGRESS_2026-11-13.md), [resume guide](docs/RESUME.md)
and [latest detailed handoff](docs/LOOP_HANDOFF_2026-11-12.md) for completed work and remaining tasks.

A native Rust MCP server that lets an LLM read and control Fender Studio Pro through typed tools. The server connects to a JavaScript bridge and EditTasks inside the DAW through a local mailbox and CoreMIDI.

Actual DAW checks use **Fender Studio Pro 8.1.1 Build 113103 on macOS Apple Silicon**. The final native-extension run covered **60 built-in classes**, enumerated **8,668 indexed parameter records across 59 current providers**, and resolved every reported DSP parameter name. Splitter adds 16 fixed graph controls. Fifty-nine classes passed a parameter or graph change/readback/restore; Phase Meter passed slot bypass. External Effect Mono and Stereo passed gain, mix, manual latency, phase and Unicode-note roundtrips. The same build passed the bridge/EditTask regression. Read the [validation report](docs/VALIDATION.md) and [plugin matrix](docs/BUILTIN_PLUGINS.md) for scope and evidence. Every GUI action, model and physical hardware path has not been certified.

The 45 MCP tools cover session and transport state, tracks and mixer channels, MIDI parts and notes, native parameter pages, supported list choices, existing Splitter controls, and finite editing operations. They also expose bounded native state capsules, issued preset catalogs, parameter details, explicit-value formatting, fresh parameter digests, registered button actions and current-selector model lists. Saved-song inspection and WAV analysis run without a DAW connection. [Hardware inventory and bounded MIDI input observation](docs/HARDWARE.md) also have standalone CLI paths that bypass the DAW runtime. Current surface mappings cover at most 256 mixer channels, 16 insert slots and 8 sends per channel; note-edit batches cover at most 128 parts. Windows, Intel Mac, and Linux DAW integration have no actual-environment validation.

The [plugin-state validation](docs/PLUGIN_STATE_VALIDATION.md) separately records the newer model, preset, nested-fixture and full-parameter work. It includes successful actual-host cases and stopped restoration tests. Complete current-provider enumeration does not mean every writable parameter or model has passed a roundtrip. A new Chorder check found that live native parameter/model changes can fail to persist through Song Save/reopen; an opt-in edit-lifecycle pilot reproduced that persistence failure, and the receiver/model path remains under investigation.

Build with Rust 1.88 or newer and Cargo. The installed server runs as a native binary. Node.js and Python are development and validation tools. The server does not launch either runtime.

Follow these steps for the first connection.

1. Clone, build, and install the bridge and EditTasks.

   ```sh
   git clone https://github.com/dev-universe/fender-studio-pro-mcp.git
   cd fender-studio-pro-mcp
   cargo build --locked --release
   ./target/release/fender-studio-pro-mcp install
   ```

   The default DAW profile is `~/Library/Application Support/Fender/Studio Pro 8`. For a different profile, pass `--profile /path/to/profile` to each command. Installation adds the managed device and extension assets and preserves unrelated settings.

2. Start the MCP server through your client using the configuration below. For initial device setup without an MCP client, keep this command running in a terminal.

   ```sh
   ./target/release/fender-studio-pro-mcp hold
   ```

   The `hold` command prints `"ready": true` and publishes the virtual MIDI source. Keep the owner process running while you configure the DAW.

3. Quit and restart Studio Pro after installation, with the server or `hold` process already running.

4. Open **External Devices**, add a **Control Surface**, and select these settings.

   | Setting | Value |
   | --- | --- |
   | Vendor | `fender-studio-pro-mcp` |
   | Device | `Fender MCP Bridge` |
   | Receive From | `fender-studio-pro-mcp` |
   | Send To | `None` |

5. Open a session. Use a named disposable scratch session for the first editing checks, with playback and recording stopped. The validated request path requires an open session. The Home screen can time out.

6. To check the installation and current connection, first disconnect the MCP client or stop `hold` with Ctrl-C. Then run the standalone diagnostic.

   ```sh
   ./target/release/fender-studio-pro-mcp doctor --live
   ```

   Look for `"installation_current": true` and `"live_verified": true`. A stored `status_file` alone reports a previous bridge load. It does not verify the current connection.

7. Start your MCP client again. Only one server, `hold`, live diagnostic, or CLI controller can own the same profile at a time. A second owner returns `controller_busy`.

Use an absolute binary path in a client that supports stdio MCP servers. Adapt this generic configuration to your client's settings format.

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

The client starts `serve` and exchanges MCP JSON-RPC through stdin and stdout. The server writes diagnostics to stderr. Keep the server alive during the DAW session. If the input source disconnects after a server restart, reselect **Receive From** with the server running. Restarting the DAW with the owner already running is the first-connection sequence above. Reconnection across every launch order remains unverified.

Ask the LLM to call `host_status` before editing. Mutations require its current `document` token as `expected_document`. Reopening or switching a document invalidates earlier editing intent. Call `operations_list` to inspect the finite operation schema and tool capabilities.

For standalone CLI calls, stop any other owner first. Read the current context.

```sh
./target/release/fender-studio-pro-mcp call context
```

For a scratch channel named `Scratch` with Mixtool in insert slot 0, read the parameter before setting it. Replace the names and document token with the values from your own scratch session. Insert slots start at zero, and normalized values range from 0 to 1.

```sh
./target/release/fender-studio-pro-mcp call pluginParams \
  '{"kind":"effect","channel":"Scratch","slot":0,"plugin":"Mixtool","names":["gain"]}'

./target/release/fender-studio-pro-mcp call setPluginParam \
  '{"kind":"effect","channel":"Scratch","slot":0,"plugin":"Mixtool","param":"gain","normalized":0.5}' \
  --expected-document 'DOCUMENT_TOKEN_FROM_CONTEXT'
```

Plugin writes require the expected plugin class or instance name. With the native extension, carry the returned `instance.object_uid` as `expected_instance` for subsequent reads/writes and the receipt's document token as `expected_document`. Model/pad/focus changes still require fresh discovery. Without the extension, candidate discovery is partial and class-only matching cannot identify a same-class replacement.

For native enumeration and External Effect's controller, quit Studio Pro, stop the owner, then build and install the optional Rust extension.

```sh
sh native/build.sh
./target/release/fender-studio-pro-mcp native-install artifacts/native/fender-studio-pro-mcp.native.mac
```

Start the owner before reopening Studio Pro. The extension accepts only the tested **8.1.1 arm64 / CCL 5.1.2** image identities. It uses the normal user extension loader and makes no changes to the vendor app bundle. Building requires Rust 1.88 and the usual macOS command-line tools; no SDK download or Ghidra installation is needed for an ordinary build. Optional SDK ABI checks and native tests are documented in the [native add-on guide](native/addon/README.md).

Call `plugin_parameters` with `names` omitted and `limit:64`. Follow `next_offset` until null, keeping `expected_document`, `expected_instance` and the returned `source` unchanged. `page_complete` covers a page; `complete` is true only when one page covers the current provider. Counts include read-only meters and UI state, so they are not writable-control counts. Impact exposed 4,262 records; Pro EQ exposed 199. Re-enumerate after changing a model, pad or nested context.

For External Effect, open its expected editor with `plugin_editor_open`, then query `plugin_parameters` with `source:"controller"`. Both modes exposed 24 controller records. `plugin_parameter_choices` reads integer-indexed native list labels without selecting a value. The tested send/receive lists had zero configured hardware ports; positive physical routing, ping/automatic latency and audible loopback remain unverified. Floating-scale DSP list facades do not use this index-list query; their ordinary raw/normalized/display-text parameter path remains available.

`plugin_parameter_details` reads precision, state flags and the bounded original-parameter chain. `plugin_parameter_format` formats up to 32 explicit numeric inputs without selecting them. For example, the tested Ampire `opt.ampmodel` inputs `[0,1,2,3,4,5]` returned distinct amplifier labels without changing the current provider. A Float List facade, its precision or its maximum does not by itself prove a complete choice domain.

For Chorder `chorder.autofill`, `plugin_parameter_details` also accepts `compare_gui:true` with explicit `source:"dsp"` and `expected_instance`. It compares the direct parameter and its declared observer with a unique exactly named control in the already-open editor. The bounded read opens no UI and grants no mutation access. Missing, ambiguous or incomplete bindings remain unresolved. Identity equality does not establish visible-control semantics, saved-state persistence or audible processing; original-parameter equality remains unobserved.

The experimental `chorder_autofill_set` edits the source-defined local GUI
receiver instead of the direct DSP facade. Supply `track`, `slot`, Boolean
`enabled`, `expected_instance` and `expected_document`, with the normal Chorder
editor already open. Read its `gui_parameter` descriptor through the comparison
first. A successful call remains `applied_unverified`; `gui_value_verified`
covers only the typed GUI current value. DSP propagation, persistence and effects
need independent observations. This finite operation accepts no arbitrary GUI
name or path. One actual 8.1.1 on/off roundtrip passed Save and full app restart
in both directions, with all 908 exposed values checked twice and the original
Chorder preset member restored byte for byte. Other chord-model edits remain
separate work.

The experimental `chorder_learning_get` reads the already-open Chorder editor's
learning mode and selected input pitch. `chorder_learning_set` also admits one
bounded chord-note membership intent. Both
require `track`, Note FX `slot`, `expected_instance` and `expected_document`.
The setter's closed `change` accepts `{"kind":"learn_mode","enabled":true}`,
`{"kind":"input_pitch","pitch":0}`, or
`{"kind":"chord_note","expected_input_pitch":60,"output_pitch":0,"enabled":true}`.
The last intent requires Learn Mode1 and an already selected matching input.
It changes no selector or mode implicitly and reports only GUI current-value
verification. Stored-model mapping, restoration, persistence and audio need
separate evidence. One actual input60/output0 add/remove pilot passed all908
provider values and all128stored-input model comparisons in179requests.
Other pitches and persistence remain separate checks.
An explicit `pitch:null` clears the
selection; omitting `pitch` is an error. Pitches are MIDI 0..127. The getter's
read-only discovery ceiling is 256 views; learning setter discovery is limited to
144 views after an actual 131-view Mode-on measurement. The separate Auto Fill
setter remains limited to 128 views. Incomplete bindings refuse the request. Successful edits remain
`applied_unverified` with a separate GUI-current-value fact. These transient
controls do not establish learned chord contents, persistence or audio results.
The Mode-off getter completed a 125-view actual-host walk. One input-pitch
null-to-MIDI0-to-null setter roundtrip passed complete 908-record changed and
original comparisons. Native32 also passed one Mode-on, Mode-off and explicit
input-null roundtrip. Eighteen complete provider reads covered its phases;
the final 908 records and GUI descriptors matched the original. Five state
captures retained identical stored chord bytes. Song Save/restart and learned
chord editing remain separate checks. A separate full restart without Save
reset Mode and input selection to the original, with four complete 908-record
reads and unchanged stored chord bytes. Mode-only editing left the normal Save
menu disabled in that experiment.

`plugin_models` reads the current `categories` or `effects` flat list for an expected Mustang Native or Rumble Native instance through the matching native extension. Use the standard plugin target, `plugin`, `expected_instance`, `catalog`, and optional `offset`0..4096/`limit`1..32. Follow `next_offset` with the same document, instance and catalog. Titles can be unavailable; `complete` describes the requested page, while `all_flat_rows_observed` also requires reading the entire current list from offset0. Lists may be filtered by category and edit context. They do not establish every model, an atomic snapshot or permission to select/apply one. This tool performs no selection, preview or confirmation and issues no token.

Capture `plugin_state` before changing a model or recalling a preset. Keep the returned opaque token, document token and native instance identity. `plugin_presets` lists an issued catalog and accepts only an index issued by that catalog; callers cannot supply preset paths or imported state bytes. State load and preset recall remain `applied_unverified` until independent reads establish the intended result. A capsule load can also introduce representational drift, so exact restoration must be checked. Release unused state and catalog tokens.

The experimental `chorder_chords_get` reads stored chords through the matching
native extension. Supply `track`, Note FX `slot`, `expected_instance`,
`expected_document`, `offset` 0..127 and `limit` 1..16. Pages include empty input
keys and sorted output MIDI pitches. Each page captures fresh state; compare
the capture and model digests across pages. The bounded decoder accepts only
the known Chorder serializer schema and exposes no XML, file path or state
token. It grants no mutation authority or persistence/audio claim. Native34
passed actual 8.1.1 unit64 with two complete 128-input passes, 16 serializer
reads and four unchanged complete 908-record reads. The run made zero parameter
writes, capsule actions, loads, transport actions or Song Saves. See the
[read-only model evidence](docs/evidence/2026-11-13-chorder-chords-readonly.json).
One input60/output0 membership edit and exact restoration passed independently
through this getter. Other pitches, Save/restart and audio remain pending.

Native33's fixed Chorder child probe also passed in the actual 8.1.1 host.
It observed `pitch61` and its local `learnChord` and `chordAssigned` metadata.
Four complete 908-record reads stayed original before and after the probe.
The probe grants no mutation authority or output-pitch mapping. Stored-chord
editing, restoration and persistence remain pending.

The finite additional-preset plan has completed 18 of 46 classes in the actual
8.1.1 host. Sixteen classes passed two capsule-only recall/restore cases each.
Expander and Gate each passed two separate hybrid cases, each using one capsule
load and one preplanned original attack setter. These are 36 positive cases.
Multiband Dynamics, Surround Delay and Room Reverb stopped at exact restoration
checks and contribute no accepted cases. Twenty-eight classes remain.
The [preset evidence](docs/evidence/2026-11-12-preset-recalls.json) separates
completed classes, partial cases and failed units. The selected issued indices
do not establish that every available preset, hidden value, saved state or
audio result has been verified.

`insert_chain_state` adds a separate, capability-gated whole-Inserts lifecycle for one exact mixer channel. Every capture, restore and release requires `expected_document`; restore and release accept only its issued token, with optional `expected_folder` from the capture response. The native owner keeps the host-authored preset file private. Callers supply no file path or imported bytes. Restore remains `applied_unverified` and requires fresh graph and per-part parameter/model observations; a load may replace Device UIDs. A simple actual-host chain passed changed-value restoration with full exposed-state confirmation. A two-Mixtool nested fixture also passed changed child/Splitter restoration and visible connection checks. Initial Pro EQ preset-display preservation failed (`default*` became `default`); dirty/UI state, hidden values and generic topology editing remain unverified. See the [live validation checklist](docs/PLUGIN_STATE_VALIDATION.md).

`plugin_parameter_set` accepts `float_bits` with an explicit native source to preserve the finite f64 bits returned in `native_value.data.bits_hex`. Its optional `edit_lifecycle:"native_user"` selector brackets one native edit; the current pilot admits only Chorder `chorder.autofill` raw 0/1 with explicit `source:"dsp"` and `expected_instance`. The returned edit phase and numeric readback do not establish saved-state persistence. `plugin_parameter_digest` reads the complete bounded provider afresh and hashes its exposed schema and typed values. It reports sequential observations, not an atomic snapshot or decoded opaque state. These hashes do not establish audible signal flow or model coverage.

Use `track_formats` to read the host's exact audio format names, then `tracks_edit` with `{"ops":[{"op":"addAudioTrack","name":"Scratch 5.1","format":"5.1"}]}` and the current `expected_document`, using a returned format name. Mono and stereo names can be localized. The operation refuses unknown/non-audio formats and duplicate track names. `panner_get` includes the channel's readable `speakerFormat` descriptor.

For an existing Splitter, `splitters_list` returns its native node name, label and active parameter descriptors; pass that identity to `splitter_set`. Counts 2..5, modes 0..2, five gains, five mutes and four crossovers passed change/readback/restore checks. Graph creation, connection, deletion and nested graph editing remain unfinished. Surround Panner uses `plugin_parameters` and `plugin_parameter_set` with `kind:"panner"`, `slot:0` and `plugin:"Surround Panner"`. Its current 5.1 provider exposed 47 records; all 47 writable controls passed changed/readback/capsule-restore checks. Read native descriptors or use normalized writes: some raw scales are 0..10000.

Read each outcome receipt. `applied_verified` includes operation-specific readback. `applied_unverified` acknowledges an edit without complete verification. `partially_applied` preserves successful earlier work. `outcome_unknown` can mean the edit happened before a timeout. Inspect the actual state before deciding whether to repeat an operation.

These commands inspect local files without connecting to the DAW or taking its controller lock.

```sh
./target/release/fender-studio-pro-mcp local-catalog --plugin 'Pro EQ'
./target/release/fender-studio-pro-mcp song-read /path/to/Scratch.song
./target/release/fender-studio-pro-mcp audio-analyze /path/to/render.wav
```

`local-catalog` reads candidate parameter names from the installed app's metadata. `song-read` reports the last saved archive, including partial plugin snapshots, rather than unsaved live edits. `audio-analyze` reports WAV loudness, peak, RMS, and stereo measurements with explicit format and measurement limits. Dedicated DAW capture and export workflows remain unfinished.

After a source update, quit Studio Pro, stop the active owner, rebuild and run `install`. Rebuild/reinstall the optional native package separately when its sources change. The installers refuse unknown files and preserve backups when explicitly replacing modified managed files with `--force`. `doctor` reports whether managed assets match their manifests. `uninstall` removes unchanged bridge/EditTask assets; `native-uninstall` separately removes unchanged native files. Modified files are preserved. Restart the DAW with the owner already running after installation changes.

Use these documents for details.

- [Validation report and known limits](docs/VALIDATION.md).
- [Built-in plugin matrix](docs/BUILTIN_PLUGINS.md).
- [Implementation plan](docs/IMPLEMENTATION_PLAN.md).
- [Architecture](docs/ARCHITECTURE.md).
- [Reference project research](docs/RESEARCH.md).
- [Local checks and CI status](ci/README.md).

This independent project is not affiliated with Fender or PreSonus. Fender Studio Pro and Studio One are trademarks of their respective owners. The project uses the [MIT license](LICENSE). Copied upstream assets retain their notices in [third-party notices](THIRD_PARTY_NOTICES.md).


For the exact External Effect Mono/Stereo classes, explicit `source:"controller"`
reads require a matching retained open editor. The new actual Stereo check read
its complete24-parameter provider before and after without changes. Its current
None value is integer-1, but parameter choice lists are empty even when the
visible menu has a Main bus row. These lists do not enumerate the popup menu.
Explicit controller sendPort/receivePort writes are refused until a separate
GUI port writer is verified. No physical loopback, ping or audio claim follows
from these metadata reads.

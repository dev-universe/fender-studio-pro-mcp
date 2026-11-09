# Validation on Studio Pro 8.1.1

Validated on 9 November 2026 against **Fender Studio Pro 8.1.1 Build 113103 on macOS arm64**. The native Rust MCP server, embedded bridge and both EditTasks ran in the actual DAW. The [original public evidence](evidence/2026-11-09.json) records the earlier session and plugin inventory at clean revision `afff1fa`. The [native-controls follow-up](evidence/2026-11-09-native-controls.json) separately records the updated feature binary, Splitter/Surround Panner acceptance and session regression. Source provenance remained unchanged during that regression and installed assets matched their manifest; the feature working tree had uncommitted changes. The earlier 55 representative plugin parameter passes were not all rerun on this binary.

The P0 bridge and note-creation paths work. These results do not complete every P0 acceptance or later feature gate in the [implementation plan](IMPLEMENTATION_PLAN.md).

| Environment | Validated scope |
| --- | --- |
| Host | One Apple Silicon Mac running macOS |
| DAW | Fender Studio Pro 8.1.1 Build 113103 |
| Session | Named synthetic scratch sessions: 64 tracks in the original run, 65 in the native-controls follow-up |
| Client path | MCP over stdio, native Rust server, installed bridge and EditTasks |
| MCP discovery | 30 tools in the updated server; the original evidence recorded 27. Discovery alone does not verify every workflow. |
| Other platforms | No Windows, Intel Mac or Linux DAW execution evidence |

## Observed DAW behavior

| Check | Result and evidence boundary |
| --- | --- |
| Bridge and installation | Open-session context, dispatch and state reads succeeded. Timer and MIDI wakeup paths operated with an active session. The original run passed `doctor --live`; the follow-up confirmed installed asset hashes matched the manifest and exercised the updated bridge. |
| Track EditTask | `createPart` and `addNotes` created an empty instrument part and inserted notes with matching pitch, beat, length and velocity readback. Named-part creation also passed: a part began at 16 seconds, and its synthetic pitch-72 note began at absolute quarter-note beat 32 with a 0.5-second length. |
| Musical EditTask | The updated feature binary again transposed 4 notes across 3 parts up one semitone, verified complete readback, then transposed down and verified exact original part and note data. |
| Plugin control | The [60-class matrix](BUILTIN_PLUGINS.md) combines earlier 55 representative parameter and 3 bypass passes with the two native-control results below: 57 classes with parameter/native graph control, 3 bypass-only and none without verified control. This is not all-controls coverage or a fresh full-inventory run. |
| Splitter | 21 cases passed: counts 2..5, modes 0..2, five gains, five mutes and four crossovers on one existing top-level node. At five branches all 16 fixed controls were readable. Changes and restorations were read back; final original active values, mode and count matched. Hidden branch values before expansion were not observed. |
| Surround Panner | All 15 known candidates changed, were read back and restored in a 5.1 fixture. Access uses native `Panner/AudioPanFX/FXSlot/Device`, `kind:panner`, slot 0 and expected class identity. Other formats, complete enumeration and audible spatial behavior remain unverified. |
| Audio track formats | Read-only `track_formats` returned 16 native audio formats. `tracks_edit` / `addAudioTrack` created the named 5.1 fixture using an exact returned format. Localized mono/stereo names must be taken from discovery. `panner_get` reports the readable native `speakerFormat`. |
| Native channel panner | Five values were readable, including the read-only `pannerType` string `Simple`; four controls were writable. All four (`panX1`, `panX2`, `panWidth`, `stereoMode`) changed with `applied_verified` readback and all original descriptors were restored; width included 1 → 0.75 → 1. |
| Plugin editor | The Phase Meter editor window opened and was observed in the actual app. This verifies that editor-opening path, not arbitrary editor interaction. |
| Cross-process ownership | A second controller for the same profile/mailbox exited unsuccessfully with `controller_busy` while the MCP owner remained alive. |
| Stale document intent | A channel mutation with a stale expected-document token returned `not_applied` / `document_changed`; channel readback remained unchanged. |
| MIDI wakeup recording probe | Recording was observed on an empty Multi Instrument track with Note FX slot 0 confirmed absent. Sixteen ping operations ran; readback found zero recorded notes. Recording stopped, and the fixture, transport position and Main mute were restored. Input routing was not inspected, so this establishes no global All Inputs or all-routing guarantee. |

The note-edit receipts remain `applied_unverified` at the generic operation level. The validator independently compared every returned part and note field, including pitch, velocity and timing, to the expected transposed state and original state. This does not promote future edits or command acknowledgments to verified workflow completion.

The final native-controls run passed **36 cases with 902 receipts**: 21 Splitter and 15 Surround Panner cases, all with restoration confirmed. Each write required a stable named scratch document, unique empty Audio fixtures, muted channels and stopped transport. Some panner raw values use 0..10000 scales; descriptors and normalized writes avoid assuming that raw values are display units or 0..1.

## Session latency

The table below is the **historical baseline from the original evidence**, measured in its 64-track scratch session. Client wall time used a monotonic clock and excluded report-file writes. Snapshots included event data, with at most 50 returned events per track. The private report also retains server `elapsed_ms` values and full receipts.

| Operation | Samples | p50 wall time | p95 wall time | Errors | Timeouts |
| --- | ---: | ---: | ---: | ---: | ---: |
| Ping | 100 | 72.14 ms | 78.31 ms | 0 | 0 |
| Track snapshot | 20 | 67.79 ms | 75.41 ms | 0 | 0 |

p50 is the median; p95 uses nearest rank. This is one warm run on one host, with no baseline ratio or throughput claim. The limiting resource was not profiled, and other machines, larger sessions, cold starts and sustained-load behavior remain unmeasured.

The updated feature binary separately completed 100 pings and 20 track snapshots in the 65-track session with zero errors or timeouts. Its second-owner rejection, stale-document refusal and complete note restoration also passed; those results do not replace the historical timing values above.

## Local checks

The updated feature checks passed **45 Rust tests and 119 JavaScript tests**, formatting, Clippy, the release build and MSRV 1.88.0. The original evidence retains its earlier 38 Rust / 99 JavaScript results and Rust 1.99.0 release compiler; these are separate build records.

Contract tests cover typed input validation, receipt handling, archive bounds, audio fixtures, the actual bridge and both EditTask scripts. Integrated fixtures check one-shot lease consumption, replay rejection, expired and mismatched requests, internal-task command blocking and check-only behavior. Multi-part fixtures cover reversed iteration, repeated names, fresh per-part leases, cleanup and preservation of earlier applied evidence when a later part refuses. These simulated host objects do not prove real host callback scheduling.

The native WAV analyzer has synthetic-file coverage. No EBU conformance suite or end-to-end DAW capture/loudness workflow has been certified.

## Reproducing the script checks

Run local checks from the repository root:

```sh
./scripts/check.sh
cargo +1.88.0 check --locked --all-targets
```

With the bridge installed, open a disposable scratch session and stop playback and recording. Replace the title and fixture names below with exact existing names. The session script requires a unique audio fixture channel; `--note-track` opts into the reversible note edit. Without it, notes are not edited. Reports stay in the ignored private evidence directory.

```sh
target/release/fender-studio-pro-mcp doctor --live
python3 scripts/verify_live_session.py \
  --binary target/release/fender-studio-pro-mcp \
  --scratch-title "YOUR SCRATCH SESSION" \
  --audio-channel "FSP Verify FX Pro EQ" \
  --note-track "MCP Keys"
python3 scripts/verify_live_plugins.py \
  --binary target/release/fender-studio-pro-mcp \
  --scratch-title "YOUR SCRATCH SESSION" \
  --kinds effect,instrument,note_effect
python3 scripts/verify_native_controls.py \
  --binary target/release/fender-studio-pro-mcp \
  --scratch-title "YOUR SCRATCH SESSION" \
  --output .local/live/native-controls.json
```

The [session script](../scripts/verify_live_session.py) records provenance, ownership and stale-token checks, actual track count, timing distributions and optional complete note restoration. It never blindly repeats a transpose after an ambiguous response. The [plugin script](../scripts/verify_live_plugins.py) reproduces the earlier inventory checks, creates or reuses scratch fixtures and restores tested values; fixture tracks and inserts can remain.

The [native-controls script](../scripts/verify_native_controls.py) requires existing unique, empty, muted Audio fixtures named `FSP Verify FX Splitter` and `FSP Verify Surround Panner`. The first must contain one top-level Splitter; the second must already use 5.1 with Surround Panner. It creates neither fixture nor graph nodes. It checkpoints each call, checks document and node/class identity, reads state after uncertain writes and restores original raw values only while the document remains stable. Its 16 fixed Splitter controls and 15 panner names do not constitute full native parameter enumeration. The Mix FX, Simple panner, editor-window and MIDI-recording observations were checked separately.

## Receipt contract and remaining limits

The [receipt model](../src/model.rs) separates observation, verified application, unverified application, refusal, partial application and unknown outcomes. A command acknowledgment establishes dispatch, not completion. A timeout after dispatch can mean an edit occurred without a reply; non-idempotent operations must not be repeated automatically.

The [runtime](../src/runtime.rs) matches request, bridge session and document context and serializes DAW access through an owner with a process-shared profile lock. The task lease is consumed before mutation and ordinary replay is rejected. This does not provide crash-safe exactly-once execution.

- **Mapping bounds:** up to 256 mapped mixer channels, 16 insert slots and 8 sends per channel; one note-edit request supports at most 128 parts. Larger mappings and nested rack addressing are not established.
- **Plugin completeness:** native parameter enumeration is incomplete. Modulation, sampler focus, presets, file loading and all-controls coverage remain open. External Effect gain/mix/routing remain unverified despite successful slot bypass. Splitter support covers the 16 fixed controls of an existing top-level node, not graph construction, connection, deletion, plugin placement or nesting. Surround Panner covers 15 named candidates in 5.1 only.
- **Document lifetime at the task boundary:** the outer bridge tracks a lifetime token, but the task guard independently compares path and title. A mocked same-path reopen between arming and task execution passed that weaker check. Post-operation context detects the changed lifetime and Rust reports unknown after the possible edit. Real Studio Pro scheduling of that switch during synchronous dispatch has not been demonstrated; prevention at this boundary remains unresolved.
- **Plugin instance lifetime:** class or instance-label matching cannot distinguish a same-class replacement in the same slot. No stable instance UID/generation contract is implemented.
- **No-document state:** the original release run observed `bridge_timeout` on the Home/no-document screen. Open a session for the validated path; reliable no-document status remains unresolved.
- **MIDI lifecycle and routing:** source recreation after server restart can require reconnecting the control-surface input when the DAW was already open. All launch orders and input-routing combinations remain unverified.
- **Saved songs:** `song_read` reports last-saved state, which can differ from live unsaved state. Plugin snapshots are partial; binary note/automation content is not fully decoded.
- **Capture and output:** dedicated audio capture, managed recording and export-job workflows are not implemented. The bounded MIDI recording probe does not establish those workflows.
- **Platforms and CI:** Windows and Intel Mac live behavior remain untested. Publishing the GitHub Actions workflow was blocked by the current credential's workflow authorization. The [Rust workflow template](../ci/rust.yml) is tracked outside the Actions directory; no remote CI pass is asserted.

## Evidence handling

Live runs used scratch data. Raw receipts, session/document identifiers, local paths, user audio and proprietary resource or preset dumps remain local and excluded from Git. The linked public evidence contains sanitized outcomes and reproducibility hashes. Source review, simulated contracts and actual DAW observations retain their separate scopes.

Native path investigation used Ghidra and shipped/public-source evidence. A private Rust CCL service probe loaded through Studio Pro 8.1.1's normal module loader and completed initialization and shutdown without MIDI, document or UI access. Native parameter enumeration remains unfinished research and is not shipped; the loader check adds no plugin control coverage.

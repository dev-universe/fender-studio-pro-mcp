# Validation on Studio Pro 8.1.1

Validated on 9 November 2026 against **Fender Studio Pro 8.1.1 Build 113103 on macOS arm64**. The native Rust MCP server, embedded bridge and both EditTasks ran in the actual DAW. Source revision `afff1fa` was clean during the final session run; installed assets matched their manifest. The [public evidence JSON](evidence/2026-11-09.json) records the revision, binary/source/asset hashes, aggregate results and limits.

The P0 bridge and note-creation paths work. These results do not complete every P0 acceptance or later feature gate in the [implementation plan](IMPLEMENTATION_PLAN.md).

| Environment | Validated scope |
| --- | --- |
| Host | One Apple Silicon Mac running macOS |
| DAW | Fender Studio Pro 8.1.1 Build 113103 |
| Session | Named synthetic scratch session with 64 tracks |
| Client path | MCP over stdio, native Rust server, installed bridge and EditTasks |
| MCP discovery | 27 tools listed by the actual server; discovery alone does not verify every workflow |
| Other platforms | No Windows, Intel Mac or Linux DAW execution evidence |

## Observed DAW behavior

| Check | Result and evidence boundary |
| --- | --- |
| Bridge and installation | Open-session context, dispatch and state reads succeeded. Timer and MIDI wakeup paths operated with an active session. `doctor --live` passed with the current installed assets. |
| Track EditTask | `createPart` and `addNotes` created an empty instrument part and inserted notes with matching pitch, beat, length and velocity readback. Named-part creation also passed: a part began at 16 seconds, and its synthetic pitch-72 note began at absolute quarter-note beat 32 with a 0.5-second length. |
| Musical EditTask | The final session validator transposed 4 notes across 3 parts up one semitone, verified the complete readback, then transposed down and verified exact original part and note data. |
| Plugin control | 55 representative parameter roundtrips and 3 slot-bypass roundtrips passed across the [60-class matrix](BUILTIN_PLUGINS.md). Splitter graph control and Surround Panner remain outside verified coverage. |
| Native channel panner | Five values were readable, including the read-only `pannerType` string `Simple`; four controls were writable. All four (`panX1`, `panX2`, `panWidth`, `stereoMode`) changed with `applied_verified` readback and all original descriptors were restored; width included 1 → 0.75 → 1. |
| Plugin editor | The Phase Meter editor window opened and was observed in the actual app. This verifies that editor-opening path, not arbitrary editor interaction. |
| Cross-process ownership | A second controller for the same profile/mailbox exited unsuccessfully with `controller_busy` while the MCP owner remained alive. |
| Stale document intent | A channel mutation with a stale expected-document token returned `not_applied` / `document_changed`; channel readback remained unchanged. |
| MIDI wakeup recording probe | Recording was observed on an empty Multi Instrument track with Note FX slot 0 confirmed absent. Sixteen ping operations ran; readback found zero recorded notes. Recording stopped, and the fixture, transport position and Main mute were restored. Input routing was not inspected, so this establishes no global All Inputs or all-routing guarantee. |

The note-edit receipts remain `applied_unverified` at the generic operation level. The validator independently compared every returned part and note field, including pitch, velocity and timing, to the expected transposed state and original state. This does not promote future edits or command acknowledgments to verified workflow completion.

## Session latency

One warm run measured complete MCP roundtrips in the actual 64-track scratch session. Client wall time used a monotonic clock and excluded report-file writes. Snapshots included event data, with at most 50 returned events per track. The private report also retains server `elapsed_ms` values and full receipts.

| Operation | Samples | p50 wall time | p95 wall time | Errors | Timeouts |
| --- | ---: | ---: | ---: | ---: | ---: |
| Ping | 100 | 72.14 ms | 78.31 ms | 0 | 0 |
| Track snapshot | 20 | 67.79 ms | 75.41 ms | 0 | 0 |

p50 is the median; p95 uses nearest rank. This is one warm run on one host, with no baseline ratio or throughput claim. The limiting resource was not profiled, and other machines, larger sessions, cold starts and sustained-load behavior remain unmeasured.

## Local checks

The final local checks passed **38 Rust tests and 99 JavaScript tests**, formatting, Clippy and the release build. MSRV 1.88.0 passed; the release compiler was Rust 1.99.0. Results are recorded in the [public evidence](evidence/2026-11-09.json).

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
```

The [session script](../scripts/verify_live_session.py) records provenance, ownership and stale-token checks, actual track count, timing distributions and optional complete note restoration. It never blindly repeats a transpose after an ambiguous response. The [plugin script](../scripts/verify_live_plugins.py) creates or reuses scratch fixtures and restores tested values; fixture tracks and inserts can remain. The additional Mix FX, native panner, editor-window and MIDI-recording observations above were checked separately and are not all reproduced by these commands.

## Receipt contract and remaining limits

The [receipt model](../src/model.rs) separates observation, verified application, unverified application, refusal, partial application and unknown outcomes. A command acknowledgment establishes dispatch, not completion. A timeout after dispatch can mean an edit occurred without a reply; non-idempotent operations must not be repeated automatically.

The [runtime](../src/runtime.rs) matches request, bridge session and document context and serializes DAW access through an owner with a process-shared profile lock. The task lease is consumed before mutation and ordinary replay is rejected. This does not provide crash-safe exactly-once execution.

- **Mapping bounds:** up to 256 mapped mixer channels, 16 insert slots and 8 sends per channel; one note-edit request supports at most 128 parts. Larger mappings and nested rack addressing are not established.
- **Plugin completeness:** candidate discovery is partial. Nested graphs, modulation, sampler focus, presets, file loading and all-controls coverage remain incomplete. External Effect gain/mix/routing are unverified despite successful slot bypass. Splitter graph editing is explicitly unsupported; Surround Panner requires a multichannel fixture.
- **Document lifetime at the task boundary:** the outer bridge tracks a lifetime token, but the task guard independently compares path and title. A mocked same-path reopen between arming and task execution passed that weaker check. Post-operation context detects the changed lifetime and Rust reports unknown after the possible edit. Real Studio Pro scheduling of that switch during synchronous dispatch has not been demonstrated; prevention at this boundary remains unresolved.
- **Plugin instance lifetime:** class or instance-label matching cannot distinguish a same-class replacement in the same slot. No stable instance UID/generation contract is implemented.
- **No-document state:** the final release run still observed `bridge_timeout` on the Home/no-document screen. Open a session for the validated path; reliable no-document status remains unresolved.
- **MIDI lifecycle and routing:** source recreation after server restart can require reconnecting the control-surface input when the DAW was already open. All launch orders and input-routing combinations remain unverified.
- **Saved songs:** `song_read` reports last-saved state, which can differ from live unsaved state. Plugin snapshots are partial; binary note/automation content is not fully decoded.
- **Capture and output:** dedicated audio capture, managed recording and export-job workflows are not implemented. The bounded MIDI recording probe does not establish those workflows.
- **Platforms and CI:** Windows and Intel Mac live behavior remain untested. Publishing the GitHub Actions workflow was blocked by the current credential's workflow authorization. The [Rust workflow template](../ci/rust.yml) is tracked outside the Actions directory; no remote CI pass is asserted.

## Evidence handling

Live runs used scratch data. Raw receipts, session/document identifiers, local paths, user audio and proprietary resource or preset dumps remain local and excluded from Git. The linked public evidence contains sanitized outcomes and reproducibility hashes. Source review, simulated contracts and actual DAW observations retain their separate scopes.

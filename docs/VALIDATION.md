# Validation on Studio Pro 8.1.1

The final native-provider build ran against **Fender Studio Pro 8.1.1 Build 113103 on macOS 26.7.1 arm64**, using one 65-track synthetic scratch song. The Rust MCP server, installed JavaScript bridge, both EditTasks and optional Rust CCL extension ran in the actual DAW. [Sanitized final evidence](evidence/2026-11-09-native-provider.json) records artifact hashes and scope. The [earlier baseline](evidence/2026-11-09.json) and [spatial follow-up](evidence/2026-11-09-native-controls.json) remain separate historical runs.

The final server exposed 31 MCP tools. The complete matrix was rerun on the final server/native binaries: 60 classes, 59 indexed providers and one Splitter graph. Fifty-nine classes passed changed parameter or graph control; Phase Meter passed slot bypass. This does not certify every GUI action, all model/preset states or physical audio behavior.

## Final observed results

| Check | Actual result |
| --- | --- |
| Native enumeration | 8,668 indexed records across 59 current providers; unique complete index coverage, stable document/instance/class/source/count and no unresolved DSP names. Canonical parameter names were unique within each provider. |
| Representative controls | 56 representative scalar change/readback/restore passes and one Phase Meter slot-bypass pass. |
| External Effect | Mono and Stereo each exposed all 24 controller records. Each passed input gain, output gain and mix with exact restoration of the finite nonzero raw f64 value; integer manual latency, phase toggle and Unicode notes also changed, independently read back and restored. |
| Native list choices | Each exposed send/receive hardware list returned zero entries. Empty-list query passed. Floating-scale DSP list facades explicitly reject this integer-index-list query. Positive physical port selection and audible signal flow were not tested. |
| Splitter | 21 cases passed: counts 2..5, modes 0..2, five gains, five mutes and four crossovers. Sixteen fixed controls were read at five branches; original active values/count/mode were restored. |
| Surround Panner | 47 indexed records read in a 5.1 fixture; all 15 previously named controls changed/read back/restored. |
| Spatial acceptance | 36 cases, 908 receipts, restoration confirmed on the same final build. |
| EditTask regression | Four notes across three parts transposed +1, complete part/note readback matched, then -1 restored exact original note/part data. Generic edit receipts remained applied_unverified; the validator established workflow completion independently. |
| Process ownership | A second controller was rejected with controller_busy while the sole MCP owner remained active. |
| Stale document | A stale-token mutation was refused as not_applied/document_changed; channel state stayed unchanged. |
| Warm request checks | 100 pings and 20 complete track snapshots, all successful, zero timeouts; observed track count 65. |
| Normal native shutdown | Service instances, factory refs, owned metadata refs and receipt leases reached zero; module registration detached. Own code/bundle pins deliberately last until process exit. |
| Installation | Bridge/EditTask assets and the four native payload files matched their managed installation manifests. |

The native provider run recorded 1,169 receipts, plus its separate spatial and session regression ledgers. Some read-only preflight probes intentionally checked unsupported sources; the 60-class acceptance and regression summaries passed. A preceding run used normalized 0.37 for four binary switches, which rounded to their unchanged zero state. The validator was corrected to request a different raw switch value; all four passed in the final full rerun. This was a test-target correction, not evidence that an unchanged value counted as a changed control.

Earlier independent checks also established track/part/note creation, exact named 5.1 audio-track creation, 16 audio format names, four writable Simple-panner controls and plugin-editor opening. An earlier bounded MIDI recording probe found no wakeup notes in one fixture; it is not a guarantee for every input routing configuration. Those creation/routing observations were not repeated as new fixture creation during the final run.

## Timing and local checks

One warm run, client monotonic wall time excluding report-file writes:

| Operation | Samples | p50 | p95 | Errors/timeouts |
| --- | ---: | ---: | ---: | ---: |
| Ping | 100 | 69.36 ms | 77.76 ms | 0 / 0 |
| Track snapshot | 20 | 73.40 ms | 86.89 ms | 0 / 0 |

These are one-machine observations, not throughput, cold-start or improvement claims. No limiting resource was profiled.

Final local checks cover 53 root Rust tests, 131 JavaScript tests, 60 native-crate mocks and six native compile-fail documentation checks. Formatting, Clippy with warnings denied, release build, Rust 1.88 checks and optional exact-SDK ABI checks passed. The native mocks are split across parameter access (24), DSP records (11), editor identity (12) and add-on/lifecycle/schema (13). SDK/compiler checks validate 54 add-on calls, 23 parameter calls and 18 editor calls plus layouts and pinned headers. They do not replace actual host evidence.

The root release SHA256 is `24353e08beb7223e148a459a41e69d8c7cbdc445a1cc7fd1e731027ef5dc7213`; native add-on SHA256 is `67be3695fb00780b684498e52e920cca8ad924a1ec7de6fe948deb8d881c1bfd`. The working feature sources were uncommitted during the run. Content/installation hashes, rather than an assertion of clean-head execution, bind the evidence.

## Reproduction

Run the ordinary checks and the separate native-crate checks described in the [add-on guide](../native/addon/README.md). Native tests/build do not install or launch the DAW.

```sh
./scripts/check.sh
cargo +1.88.0 test --locked
sh native/build.sh
```

Install bridge and native package while the DAW/owner are stopped, then start the acceptance runner first so its MIDI source exists before Studio Pro starts. Its fixture JSON is an array of existing plugin targets, optionally with a previously checked test_parameter or test_bypass flag. For example, a target can contain channel, slot, plugin and test_parameter; instrument/Note FX targets use kind and track. Only named FSP Verify scratch fixtures are accepted. The runner creates no tracks or media and saves no song.

```sh
python3 scripts/verify_native_parameters.py \
  --binary target/release/fender-studio-pro-mcp \
  --fixtures .local/live/native-parameter-fixtures.json \
  --scratch-title 'MCP Validation Scratch' \
  --output .local/live/native-parameters.json \
  --pause --roundtrips --spatial-controls \
  --prepare-midi-fixtures --external-controls --session-checks
```

Open exactly the named disposable song, then press Enter in the runner. The supplied test fixture inventory must already exist locally; no raw song or machine-specific fixture inventory is distributed. The spatial option requires the existing named Splitter/5.1 fixtures; session checks require the existing MCP Keys note fixture. Raw output must resolve inside the repository's ignored .local directory.

Selection in the tested DAW auto-armed and enabled monitoring even when reselecting the same Music track. The opt-in MIDI preparation proves unique empty-track identity and exclusive selection, reads finite switch states, sets mute on/arm off/monitor off through fixed observed commands and verifies each update. It avoids reselecting an already exclusive target. These changes affect only the disposable scratch song and were discarded on normal quit.

Every mutation has an in-flight checkpoint, current document and target checks, and a restoration path. A restoration failure aborts further mutation acceptance. The existing [session](../scripts/verify_live_session.py), [candidate-based plugin](../scripts/verify_live_plugins.py) and [spatial](../scripts/verify_native_controls.py) runners remain available for their narrower scopes.

## Receipt and compatibility boundaries

- **Enumeration scope:** complete current DSP/controller provider indices; read-only meters, headers, UI state and opaque records are included. It is not every possible model, selected pad, unloaded program, modulation route, file action or nested plugin. Opaque image/object contents are not exported or generically writable.
- **Native ABI:** exact tested app/cclgui arm64 image identities and CCL 5.1.2 declarations. One exploratory prototype used 5.2 method offsets and crashed the disposable scratch process. It was retired, the saved scratch reopened, and note/spatial baselines verified. Corrected and product builds use source/compiler/runtime guards; no final product run crashed. This continuation used Ghidra and authoritative public source.
- **Lifecycle:** one bounded completed-receipt lease retains native document/model/provider owners through output publication. It drains on the next request or stop. Own code and bundle metadata intentionally remain pinned until process exit; hot unloading is unsupported.
- **Typed transport:** raw numeric JSON writes are bounded to the JavaScript safe integer magnitude. Larger native integers retain exact decimal metadata and do not advertise generic numeric writes. Unsupported types, disabled/read-only parameters and empty invalid ranges are refused. A successful API call is distinct from a verified requested value.
- **Hardware:** the exposed send/receive lists contained no entries. Positive routing, ping/automatic latency and audible loopback remain unverified; no hardware test signal was generated. Manual latency means a parameter-state roundtrip, not measured acoustic/electrical latency accuracy.
- **Graph/spatial:** existing top-level Splitter controls and a 5.1 Surround Panner fixture only. Graph construction/connection/deletion, nested graphs, other speaker formats and audible spatial behavior remain unverified.
- **Document/task lifetime:** native data requests retain canonical document/model identities. The existing EditTask lease independently compares path/title, so a mocked same-path reopen at that boundary can pass its weaker check; the outer bridge detects the changed object afterward and reports unknown. Prevention at that task boundary remains unresolved.
- **Instance/focus:** expected_instance binds the resolved native Device within a document, and document tokens must accompany cross-request intent. This does not freeze every selected-pad/model/focus context. Class-only fallback without the add-on cannot distinguish same-class replacement.
- **No-document/MIDI lifecycle:** an open song is required for the validated path. Home/no-document timeouts and all server/DAW launch orders remain unresolved; source recreation can require reconnecting Receive From. Start the owner before the DAW.
- **Mapping/platforms:** 256 mixer channels, 16 insert slots and 8 sends per channel; 128 parts per note-edit request. Other mappings, Windows and Intel Mac live behavior are untested.
- **Other workflows:** saved-song inspection is last-saved and partial; binary notes/automation are not fully decoded. Dedicated capture, managed recording and export-job workflows remain unfinished. WAV tests are synthetic, not EBU conformance certification.
- **CI:** local checks passed. The current GitHub credential cannot publish Actions workflows; ci/rust.yml remains a template outside .github/workflows. No remote CI pass is asserted.

Only synthetic scratch data was edited, and its unsaved edits were discarded at the end. Raw receipts, values, session identifiers, local paths, audio and proprietary resources stay outside Git. The public records contain names, counts, tested fields, outcomes and reproducibility hashes, with source review, mocks and real host observations kept distinct.

# Built-in plugin validation

Validated on 9 November 2026 against Fender Studio Pro 8.1.1 Build 113103 on macOS arm64. Across the separate runs below, the 60-class matrix contains **57 classes with changed parameter or native graph controls, 3 bypass-only classes and 0 classes without verified control**. Full control of every built-in plugin has not been established.

The [original public evidence](evidence/2026-11-09.json) records 55 representative parameter passes and 3 bypass passes, including the 58-class inventory and a separate Console Shaper check. The [native-controls follow-up](evidence/2026-11-09-native-controls.json) records 36 successful change/readback/restore cases: 21 for Splitter and 15 for Surround Panner, with 902 receipts. The older inventory was not rerun in full on this follow-up binary. See [session validation](VALIDATION.md) for the test environment and remaining limits.

## Reading the matrix

- **Parameter pass** means a named parameter changed to a different finite raw value on the live instance, that value was read back, and the original raw value was restored and matched within `1e-6 × (1 + |original|)`. Normalized requests can quantize; the test does not require exact equality with the requested normalized value.
- **Bypass pass** means the insert-slot bypass changed, was read back, and was restored with readback. It does not establish DSP parameter or signal-path control.
- **Native graph pass** covers the fixed controls of one existing top-level Splitter. It does not establish graph construction or arbitrary routing edits.
- **Readable** counts candidate names resolved on that instance. **Missing** counts queried candidate names that did not resolve. Candidate discovery is partial; these are not counts of all plugin controls.
- **—** means not measured. Zero missing candidates does not prove complete enumeration. Phase Meter had an empty candidate list; its zero counts establish no DSP parameter coverage. Splitter's 16 controls were read with five active branches.
- Counts depend on the loaded model and selected focus. A representative pass does not certify every control, preset or plugin action.

## Audio effects

| Plugin | Kind | Readable | Missing | Tested parameter | Status |
| --- | --- | ---: | ---: | --- | --- |
| Splitter | Graph | 16 | 0 | Count, mode, gains, mutes, crossovers | Native graph pass |
| External Effect Mono | Effect | 1 | 11 | — | Bypass pass |
| External Effect Stereo | Effect | 1 | 11 | — | Bypass pass |
| Channel Strip | Effect | 14 | 0 | `autogain` | Parameter pass |
| Mixtool | Effect | 7 | 0 | `gain` | Parameter pass |
| Binaural Pan | Effect | 4 | 0 | `stereowidth` | Parameter pass |
| Dual Pan | Effect | 8 | 0 | `linkedwidth` | Parameter pass |
| Limiter | Effect | 10 | 6 | `ingain` | Parameter pass |
| Expander | Effect | 13 | 2 | `attack` | Parameter pass |
| Gate | Effect | 16 | 0 | `attack` | Parameter pass |
| Tricomp | Effect | 15 | 2 | `ingain` | Parameter pass |
| Multiband Dynamics | Effect | 65 | 0 | `attackband1` | Parameter pass |
| RedlightDist | Effect | 10 | 0 | `drive` | Parameter pass |
| Bitcrusher | Effect | 11 | 0 | `autoGain` | Parameter pass |
| Phaser | Effect | 12 | 0 | `lfoWidth` | Parameter pass |
| Flanger | Effect | 8 | 0 | `lfowidth` | Parameter pass |
| Chorus | Effect | 13 | 0 | `hifreq` | Parameter pass |
| X-Trem | Effect | 24 | 0 | `depth` | Parameter pass |
| Autofilter | Effect | 40 | 2 | `cutoff` | Parameter pass |
| Beat Delay | Effect | 13 | 1 | `mix` | Parameter pass |
| Groove Delay | Effect | 53 | 1 | `cutoff1` | Parameter pass |
| Surround Delay | Effect | 76 | 1 | `drive` | Parameter pass |
| Analog Delay | Effect | 20 | 1 | `lfotime` | Parameter pass |
| Room Reverb | Effect | 16 | 1 | `mix` | Parameter pass |
| Mixverb | Effect | 9 | 1 | `mix` | Parameter pass |
| Open AIR | Effect | 51 | 18 | `gain` | Parameter pass |
| IR Maker | Effect | 4 | 0 | `irlen` | Parameter pass |
| Spectrum Meter | Effect | 8 | 13 | `maxfreq` | Parameter pass |
| Phase Meter | Effect | 0 | 0 | — | Bypass pass |
| Level Meter | Effect | 5 | 7 | `togglesize` | Parameter pass |
| Scope | Effect | 31 | 0 | `crossLevel` | Parameter pass |
| Tone Generator | Effect | 10 | 0 | `freq` | Parameter pass |
| Ampire | Effect | 68 | 47 | `cab.micmixA` | Parameter pass |
| Pedalboard | Effect | 21 | 46 | `input.level` | Parameter pass |
| Mustang Native | Effect | 31 | 2 | `input.level` | Parameter pass |
| Rumble Native | Effect | 31 | 2 | `input.level` | Parameter pass |
| Studio Verb | Effect | 19 | 2 | `algoreverb.centerlevel` | Parameter pass |
| Fat Channel | Effect | 49 | 36 | `comp.attack` | Parameter pass |
| Pro EQ | Effect | 53 | 40 | `highcut.hcfreq` | Parameter pass |
| Compressor | Effect | 20 | 22 | `studiocomp.attack` | Parameter pass |
| Tuner | Effect | 3 | 0 | `calibration` | Parameter pass |
| Voice FX | Effect | 1 | 0 | `voicefxopt.fxmodel` | Parameter pass |
| Rotor | Effect | 13 | 0 | `drive` | Parameter pass |
| De-Esser | Effect | 7 | 0 | `freq` | Parameter pass |
| Vocoder | Effect | 74 | 0 | `carrierfreq` | Parameter pass |
| VU Meter | Effect | 3 | 0 | `scale` | Parameter pass |
| Vocal Tune | Effect | 19 | 0 | `a` | Parameter pass |

External Effect Mono and Stereo each exposed one readable candidate and 11 unresolved candidates. Their slot-bypass roundtrips passed; gain, mix, hardware routing, roundtrip latency and audible signal flow remain unverified.

Phase Meter's slot bypass passed. Its editor window was also observed in the actual app through `plugin_editor_open`; this does not certify meter accuracy or control of every editor element.

Splitter is a graph node. The follow-up verified counts 2..5, modes 0/1/2, `gain[1..5]`, `mute[1..5]` and `splitFreq[1..4]`, including native value changes, readback and restoration. `splitters_list` and `splitter_set` address the exact node name and expected label under the channel's insert graph. The five-branch baseline was captured after expansion; hidden branch values before expansion were not observed. Creating, connecting or deleting graph nodes, placing plugins in branches, and nested graph control remain unverified.

## Instruments

| Plugin | Kind | Readable | Missing | Tested parameter | Status |
| --- | --- | ---: | ---: | --- | --- |
| Multi Instrument | Instrument | 20 | 0 | `bool-1` | Parameter pass |
| Mai Tai | Instrument | 219 | 17 | `adsr2.attack` | Parameter pass |
| SampleOne | Instrument | 146 | 20 | `adsrMain.attack` | Parameter pass |
| Impact | Instrument | 36 | 43 | `midi-cc.bendRange` | Parameter pass |
| Presence | Instrument | 131 | 2 | `adsr2.attack` | Parameter pass |
| CV Instrument | Instrument | 106 | 23 | `cvGate.time` | Parameter pass |
| Mojito | Instrument | 36 | 2 | `ampEnvAttack` | Parameter pass |

Multi Instrument's result covers a macro value. Assignments and audible effects in nested instruments were not verified. Impact and other sample instruments can expose controls for the selected pad or sample; the matrix does not certify every pad, sample, file-loading action or model-specific control. CV Instrument's parameter result does not certify external CV hardware output.

## Note FX

| Plugin | Kind | Readable | Missing | Tested parameter | Status |
| --- | --- | ---: | ---: | --- | --- |
| Input Filter | Note FX | 4 | 3 | `hiKey` | Parameter pass |
| Arpeggiator | Note FX | 108 | 3 | `gateTime` | Parameter pass |
| Chorder | Note FX | 12 | 7 | `chorder.autofill` | Parameter pass |
| Repeater | Note FX | 13 | 4 | `timeFreq` | Parameter pass |

All four Note FX passed a representative parameter roundtrip on identified live instances. Generated-note behavior, chord content, timing and all mode-specific controls require separate musical acceptance checks.

## Additional native paths

| Plugin | Kind | Readable | Missing | Tested parameter | Status |
| --- | --- | ---: | ---: | --- | --- |
| Console Shaper | Mix FX | 6 | 0 | `drive` | Parameter pass |
| Surround Panner | Panner | 15 | 0 | 15 known controls below | Parameter pass: 5.1 fixture |

Console Shaper was inserted through the native AudioBusMixFX path. Six candidates were readable, none of those candidates was missing, and `drive` changed with readback before exact raw restoration. This verifies one Mix FX class and parameter, not every Mix FX rack operation.

Surround Panner's verified native device path is `Panner/AudioPanFX/FXSlot/Device`. Access uses `kind:"panner"`, `slot:0` and the expected `plugin:"Surround Panner"` identity. In one 5.1 fixture, all 15 known names changed with native readback and original raw restoration: `editMode`, `azimuth`, `altitude`, `spread`, `size`, `balanceAzimuth`, `balanceAmount`, `lfeLevel`, `centerLevel`, `disableCenter`, `muteLfe`, `sendLfe`, `snap`, `sizeLock` and `soloLfe`. These are named candidates, not complete parameter enumeration. Some native raw ranges use 0..10000; consult descriptors or use normalized writes. Other speaker formats and audible spatial behavior remain unverified.

The separate native **Simple** channel panner exposed five values, including the read-only string `pannerType`, and four writable controls. All four passed change/readback/restore checks. `panner_get` now also reports the readable `speakerFormat` descriptor. `track_formats` discovers supported audio format names, and `tracks_edit` can create an explicitly named audio track using an exact returned format; mono/stereo names can be localized.

## Target identity and scope

Writes require the expected plugin class or instance name. Current checks cannot distinguish replacement by another instance of the same class in the same slot; a stable instance UID or generation contract remains a gap.

Presets, nested graphs, modulation assignments, sample and file selection, model switching, metering and hardware-dependent behavior need their own verification. The matrix contains no claim of complete native parameter enumeration or all-controls support.

Only plugin names, candidate counts, tested parameter names and outcomes are published here. Raw DAW receipts, proprietary metadata, preset values, user audio and session identifiers remain local and are excluded from Git. The [plugin verification script](../scripts/verify_live_plugins.py) reproduces the earlier 58-class inventory checks. The [native-controls script](../scripts/verify_native_controls.py) reproduces the Splitter and 5.1 Surround Panner checks using existing empty, muted scratch fixtures.

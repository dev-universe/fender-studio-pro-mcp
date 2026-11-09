# Built-in plugin validation

Validated on Fender Studio Pro **8.1.1 Build 113103, macOS arm64**. The final native-extension run covered **60 built-in classes**: **59 classes with a changed parameter or graph control and one bypass-only class**. It enumerated **8,668 indexed parameter records across 59 current providers**, with every reported index accounted for and no unresolved DSP parameter names. The existing Splitter graph adds 16 fixed controls at five branches.

The same server/native build passed 56 representative parameter roundtrips, Phase Meter slot bypass, six gain/mix roundtrips across the two External Effects, six additional External Effect control roundtrips, and 36 Splitter/5.1 Surround Panner cases. Every tested original was restored with readback. This establishes current-provider enumeration and the listed controls; it does not establish every GUI action, model, preset, nested device or physical signal path. See [sanitized evidence](evidence/2026-11-09-native-provider.json) and [validation details](VALIDATION.md).

## Reading the results

- **Native records** is the provider-reported count, verified against unique indices `0..count-1` across pages with a stable document, instance UID, class UID and source. Counts include meters, bypass/header fields and UI state. They are not writable-control counts.
- **Parameter pass** changes a finite value, reads it independently, restores the original and checks readback. Ordinary DSP restoration uses the documented numeric tolerance; the six External Effect gain/mix restores additionally matched the original finite, nonzero raw f64 value exactly.
- **Graph pass** covers one existing top-level Splitter. **Bypass pass** covers the native insert-slot switch. Neither establishes arbitrary graph construction or meter accuracy.
- Native controller descriptions report enabled/read-only/support flags. Opaque images/objects are described without exporting their contents or offering generic writes.

| Plugin | Kind | Native records | Verified control |
| --- | --- | ---: | --- |
| Splitter | Graph | 16 fixed | Count, mode, five gains/mutes, four crossovers |
| External Effect Mono | Effect | 24 | Input/output gain, mix, manual latency, phase, Unicode notes |
| External Effect Stereo | Effect | 24 | Input/output gain, mix, manual latency, phase, Unicode notes |
| Channel Strip | Effect | 19 | `autogain` |
| Mixtool | Effect | 72 | `gain` |
| Binaural Pan | Effect | 5 | `stereowidth` |
| Dual Pan | Effect | 8 | `linkedwidth` |
| Limiter | Effect | 13 | `ingain` |
| Expander | Effect | 14 | `attack` |
| Gate | Effect | 19 | `attack` |
| Tricomp | Effect | 17 | `ingain` |
| Multiband Dynamics | Effect | 71 | `attackband1` |
| RedlightDist | Effect | 10 | `drive` |
| Bitcrusher | Effect | 12 | `autoGain` |
| Phaser | Effect | 16 | `lfoWidth` |
| Flanger | Effect | 12 | `lfowidth` |
| Chorus | Effect | 14 | `hifreq` |
| X-Trem | Effect | 27 | `depth` |
| Autofilter | Effect | 44 | `cutoff` |
| Beat Delay | Effect | 14 | `mix` |
| Groove Delay | Effect | 55 | `cutoff1` |
| Surround Delay | Effect | 79 | `drive` |
| Analog Delay | Effect | 24 | `lfotime` |
| Room Reverb | Effect | 19 | `mix` |
| Mixverb | Effect | 9 | `mix` |
| Open AIR | Effect | 94 | `gain` |
| IR Maker | Effect | 5 | `irlen` |
| Spectrum Meter | Effect | 9 | `maxfreq` |
| Phase Meter | Effect | 2 | Insert-slot bypass |
| Level Meter | Effect | 7 | `togglesize` |
| Scope | Effect | 31 | `crossLevel` |
| Tone Generator | Effect | 10 | `freq` |
| Ampire | Effect | 94 | `cab.micmixA` |
| Pedalboard | Effect | 28 | `input.level` |
| Mustang Native | Effect | 39 | `input.level` |
| Rumble Native | Effect | 39 | `input.level` |
| Studio Verb | Effect | 151 | `algoreverb.centerlevel` |
| Fat Channel | Effect | 56 | `comp.attack` |
| Pro EQ | Effect | 199 | `highcut.hcfreq` |
| Compressor | Effect | 154 | `studiocomp.attack` |
| Tuner | Effect | 13 | `calibration` |
| Voice FX | Effect | 5 | `voicefxopt.fxmodel` |
| Rotor | Effect | 29 | `drive` |
| De-Esser | Effect | 10 | `freq` |
| Vocoder | Effect | 117 | `carrierfreq` |
| VU Meter | Effect | 11 | `scale` |
| Vocal Tune | Effect | 26 | `a` |
| Multi Instrument | Instrument | 150 | `bool-1` |
| Mai Tai | Instrument | 283 | `adsr2.attack` |
| SampleOne | Instrument | 235 | `adsrMain.attack` |
| Impact | Instrument | 4262 | `midi-cc.bendRange` |
| Presence | Instrument | 241 | `adsr2.attack` |
| CV Instrument | Instrument | 110 | `cvGate.time` |
| Mojito | Instrument | 37 | `ampEnvAttack` |
| Input Filter | Note FX | 388 | `hiKey` |
| Arpeggiator | Note FX | 108 | `gateTime` |
| Chorder | Note FX | 908 | `chorder.autofill` |
| Repeater | Note FX | 141 | `timeFreq` |
| Console Shaper | Mix FX | 8 | `drive` |
| Surround Panner | Panner | 47 | 15 controls, 5.1 fixture |

## Native access

`plugin_parameters` without `names` returns one native page when the optional extension is installed. Follow `next_offset` until it is null. Carry the receipt document token as `expected_document`, the first page's `instance.object_uid` as `expected_instance`, and the returned source on later requests. Verify index coverage and an unchanged count/class UID. `page_complete` describes one page; `complete` is true only when a single page covers the whole current provider. `value_read_complete` separately identifies unresolved scripting values.

`source:"dsp"` enumerates the exact Device provider. `source:"controller"` reads the currently exposed native controller. DSP devices can report zero controller entries while still resolving a known parameter name dynamically; use DSP enumeration for their full list. Named controller reads expose native types, formatter/support flags and read-only state. Native raw values can use scales such as 0..10000; consult descriptors or use normalized/display-text writes.

The native extension is pinned to the tested arm64 app and CCL 5.1.2 image identities. A different native ABI is refused. Without it, the ordinary bridge remains available and installed-metadata candidate discovery remains explicitly partial. The older [candidate-based evidence](evidence/2026-11-09.json) and [spatial follow-up](evidence/2026-11-09-native-controls.json) remain historical records.

## External Effect

Open the expected editor with `plugin_editor_open` before querying its controller. The resolver matches the actual Device identity, window, slot and controller; matching a window title alone is insufficient. Mono and Stereo each exposed all 24 controller records.

Both modes passed actual MCP change/readback/restore for `inputGain`, `outputGain`, `wetdry`, integer `latencyOffset`, toggle `phase` and Unicode `deviceNotes`. The generic controller path supports the descriptor's enabled numeric/string operations; that does not certify the audio meaning of every UI field.

`plugin_parameter_choices` reads labels for integer-indexed native lists without changing their selection. The tested `sendPort` and `receivePort` lists each contained **zero configured ports**. Empty-list reads passed; positive hardware-port selection, automatic latency/ping and audible roundtrip flow remain unverified. Disabled controls remain non-writable. Floating-scale DSP list facades are rejected by this index-list query; their ordinary raw/normalized/display-text parameter path remains available.

## Graph, spatial and musical scope

Splitter addresses the exact existing node name and label under a channel's top-level insert graph. Counts 2..5, modes 0..2, five gains, five mutes and four crossovers passed 21 cases. Creating, connecting or deleting nodes, branch plugin placement and nested graphs remain unverified.

Surround Panner uses `kind:"panner"`, `slot:0` and the expected plugin identity. Its current 5.1 provider reported 47 indexed records. Fifteen controls passed change/readback/restore: `editMode`, `azimuth`, `altitude`, `spread`, `size`, `balanceAzimuth`, `balanceAmount`, `lfeLevel`, `centerLevel`, `disableCenter`, `muteLfe`, `sendLfe`, `snap`, `sizeLock`, `soloLfe`. Other speaker formats and audible spatial behavior require separate checks. The standard Simple panner is a separate path through `panner_get`/`panner_set`.

Instrument and Note FX targets require their unique selected Music track. The acceptance runner uses only existing empty validation tracks and explicitly prepares their MIDI mute/arm/monitor state; selection can auto-arm and enable monitoring. Those scratch changes were discarded on normal quit. Sample loading, nested instruments, modulation/chord content, selected-pad aliases, model switching and preset/file actions are outside the listed roundtrips. Enumerate again after a selection/model change; the current provider is not a claim about every possible state.

Writes require an expected plugin class or instance name and a current document token. Carry `expected_instance` for native instance consistency. This binds the resolved device within that document; it does not freeze every mutable pad/model/focus context.

Raw receipts, values, user audio, session identifiers and proprietary resources remain under ignored `.local` paths. Only names, counts, tested fields, artifact digests and outcomes are published. [The acceptance script](../scripts/verify_native_parameters.py) reproduces the checks with existing named scratch fixtures.

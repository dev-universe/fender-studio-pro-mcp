# Plugin workflows

These recipes apply to the verified Studio Pro 8.1.1 build on Apple Silicon with
the matching [native add-on](INSTALLATION.md#optional-native-add-on). Use a
scratch song and bind the current document and exact plugin instance before
each edit. See the [verification matrix](BUILTIN_PLUGINS.md) for tested limits.

- [Pedalboard and Ampire](#pedalboard-and-ampire)
- [Multi Instrument](#multi-instrument)
- [Fender models, Chorder, CV and Impact](#mustang-native-processor-state)
- [Serum 2 and Serum 2 FX](#serum-2-and-serum-2-fx)
- [FabFilter presets and MIDI](#fabfilter-preset-library)
- [External Effect Stereo Send](#external-effect-stereo-send)
- [Saved plugin routing](#saved-plugin-routing)

## Pedalboard and Ampire

For a Pedalboard insert, use `plugin_inspect` with `source: "controller"`, `include_nodes: true`, and `node_scope: "plugin_controller"`. Bind the exact channel, slot, plugin, instance, and document. The bounded controller graph can show its `StompItemModel` under `stomps`. [Pedalboard controller verification](evidence/2026-11-24-pedalboard-controller-inspection.json)

In Studio Pro 8.1.1 on Apple Silicon, `pedalboard_slot_swap` exchanges two positions in the standalone Pedalboard. Stop playback and recording. Read `opt.stompmodel1` through `opt.stompmodel8` with `source: "dsp"` and pass the current document, exact instance, and both displayed model names. Use the literal `" - "` for an empty slot. Insert slots start at zero and pedal positions start at one.

- Adjacent occupied positions with different models can be exchanged directly. Tests covered both directions at positions 1–2 and 7–8, and separately verified Save, reopen, and Undo. [Adjacent exchange](evidence/2026-11-24-pedalboard-slot-swap.json) · [Save and reopen](evidence/2026-11-24-pedalboard-swap-persistence.json) · [Host Undo](evidence/2026-11-24-pedalboard-slot-swap-undo.json)
- An empty position, two identical model names, or nonadjacent positions require a fresh `plugin_state` capture for the exact insert. Confirm that `pedalboard_rack` shows two distinct payloads and pass the capture's `sha256` as `expected_rack_sha256`. Capture again after the exchange and compare all eight slots, then release both tokens. Identical-model DSP rows and a vacant-slot active alias can lag behind the serialized rack, so use payload fingerprints for this check. [Guarded exchanges](evidence/2026-11-25-pedalboard-guarded-exchange.md) · [Empty slot restart](evidence/2026-11-25-pedalboard-empty-slot-persistence.md) · [Same-model restart](evidence/2026-11-25-pedalboard-same-model-persistence.md)
- A Studio Pro 8.1.1 test saved Compressor after moving it to the empty slot. Fresh open kept the moved payload and cleared the vacated slot’s temporary active alias. [Moved-to-vacant restart](evidence/2026-11-26-pedalboard-vacant-moved-persistence.md)

Ampire has a separate pre/post divider controlled by `opt.stomporder`. The standalone Pedalboard has a continuous rack. [Divider verification](evidence/2026-11-25-pedalboard-ampire-divider.md)

## Multi Instrument

For a Multi Instrument with one stock Mai Tai, stop playback and recording and keep its uniquely named Music track selected. Use `plugin_inspect` with `kind: "instrument"` and `track: "Your track name"` to get the Multi Instrument's native instance UID. Repeat with the observed `plugin` and `expected_instance`, plus `include_nodes: true` and `node_scope: "plugin_controller"`, to read the Instruments folder, Graph, original `Inst01` slot, and its Mai Tai Device UIDs. Pass those five UIDs to `plugin_multi_add_mai_tai` with the current document and track. After `applied_unverified` or `outcome_unknown`, inspect the current children and mixer before issuing any further edit. After an explicit Undo, repeat the inspection before issuing a new add request. [Studio Pro 8.1.1 validation](evidence/2026-11-26-multi-add-mai-tai-live.md).

## Mustang Native processor state

Call `plugin_state` with `action: "read_mustang_state"`, the insert's
`channel` and `slot`, `plugin: "Mustang Native"`, `expected_instance`
from `plugin_inspect`, and `expected_document` from `host_status`.
This reads the serialized processor class IDs and numeric settings for
all 26 slots through the matching native extension. Compare
`models_sha256` and `settings_sha256` across reads. The
`capture_sha256` identifies one serialization and can differ when
the two projections match. The read issues no token.
[8.1.1 verification](evidence/2026-11-26-mustang-processor-state.md)

## Verify a parameter through a separate read

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
[Live verification](evidence/2026-11-20-plugin-parameter-readback.json)

## Set a Mai Tai child Filter Env amount

On Studio Pro 8.1.1 with the matching macOS arm64 add-on, stop playback and
recording and select the Multi Instrument Music track. Use `plugin_inspect`
with `include_nodes: true` and `node_scope: "plugin_controller"` to find the
parent, the `Inst01` or `Inst02` slot, and its live Mai Tai Device. Read
`modulationTable.cell1.amount` with `plugin_multi_child_parameters`.

Pass `track`, `expected_document`, `expected_instance`, `child_slot`,
`expected_child_slot`, and `expected_child_instance` to
`plugin_multi_child_filter_env_set`. Set `expected_float_bits` to the
observed `native_value.data.bits_hex` and pass the new value as `float_bits`.
Both strings encode distinct Float64 values in 0 through 10000.

The add-on compares the original bits before the setter. After
`applied_unverified`, read the same child again. If the reply is
`outcome_unknown`, read the current state before making another change.
The receipt covers the one child amount. Inspect the parent or the other
child separately when those values matter.
[Live recovery verification](evidence/2026-11-25-multi-child-filter-env.md)

## Chorder audition

On Studio Pro 8.1.1 with the matching native add-on, `chorder_audition` auditions MIDI 60 on one Chorder Note FX. Stop playback and recording, turn Chorder Learn off, then select the target Music track and inspect the exact Note FX slot and native instance UID. Use the current document token from `host_status` and keep the same MCP server session for all three actions.

1. Call `chorder_audition` with `action: "prepare"`, the `track`, zero-based `slot`, `expected_instance`, and `expected_document`. The owner returns a permit valid for 15 seconds.
2. Call again with `action: "play"`, the `permit` and the same `expected_document`. The owner issues one guarded press and one guarded release and separately observes the held and neutral DSP states. The receipt verifies the local Chorder key lane.
3. To retrieve an interrupted reply, call with `action: "status"`, the same `permit`, and `expected_document`. Status reads the owner's record even after the active song changes. A replay returns history with no new key event. A new successful prepare supersedes the last permit.

A missing press acknowledgement yields `outcome_unknown`. Inspect the original Chorder and clear the key using Studio Pro if necessary. The owner requires a fresh neutral observation of the original binding before permitting another audition. A later neutral observation preserves the recorded unknown outcome, and an owner restart discards permit history and requires a fresh neutral preflight on the requested target.

## CV Instrument preset settings

Use `plugin_presets` with `kind: "instrument"` and `plugin: "CV Instrument"`
on the selected track. Carry `expected_document`, `expected_instance`, and
the index and `catalog_token` issued by `list`. `inspect_cv_preset_settings`
returns a byte count and SHA-256. `apply_cv_preset_settings` loads the primary
settings without loading the preset's port-setup record.

If you want to restore the original settings, capture them with `plugin_state`
before applying. After an `applied_unverified` receipt, read parameters and
port assignments to check the result. The 8.1.1 trial verified 97 writable
values, seven unassigned ports, and restoration. Testing with assigned
physical ports is pending. [Live evidence](evidence/2026-11-20-cv-settings-apply-r106.json)

A second 8.1.1 trial changed Gate mode, the first LFO waveform and the first
ADSR attack before applying `default`. Two independent reads confirmed that
the settings-only apply reset all three and matched all 97 writable values.
The trial restored the prior values and checked all 67 mixer channels and
seven unassigned CV ports throughout. [Three-setting verification](evidence/2026-11-20-cv-default-three-settings.json)

## Live Impact outputs

Call `plugin_inspect` with `kind: "instrument"`, `plugin: "Impact"`, the
track name, `source: "dsp"`, `include_nodes: true`, and
`node_scope: "impact_outputs"`. Supply the current `expected_document` from
`host_status` and the `expected_instance` from an initial `plugin_inspect`.
The 8.1.1 macOS arm64 native add-on reports all 32 output names and whether
each activation is requested and acknowledged. You can use it with the Impact
editor closed. Use `channels_list` to see the associated mixer channels.
Set `include_color: true` to also read each strip's signed 32-bit
`color_argb` value. Studio Pro can link an instrument's primary strip
color to its arrangement track. [Impact color verification](evidence/2026-11-21-impact-primary-mixer-color.json)

Set `include_selection: true` to read `console_selected` for each uniquely
identified audio strip. The value is `null` when the Console cannot match
the strip. Use `tracks_list` to check arrangement selection. In Studio Pro
8.1.1, selecting an instrument channel in the Console can change arrangement
selection, record arm, and monitoring. [Impact Console verification](evidence/2026-11-21-impact-console-selection.json)

Use `impact_output_set` to change one of the 31 secondary outputs at indices
1 through 31. Provide the track, current
`expected_document` and `expected_instance`, `output_index` and `output_name`
from the inspected row, its current `expected_active` state, and the desired
`active` state. The tool checks both reported activation flags and preserves
the last active output. Disabling an output removes its mixer channel. Verify
your mixer routing with `channels_list` after a change.

## Serum 2 and Serum 2 FX

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
[Studio Pro verification](evidence/2026-11-25-serum2-studio-pro.md)
[Studio Pro editor audition](evidence/2026-11-26-serum2-instrument-audition.md)

Use `serum2_midi_mapping` with `variant` set to `instrument` or `fx`,
`controller` set to 0–129, and the optional zero-based `channel` set to
0–15. The default is the first MIDI channel, `channel: 0`. The arm64
2.1.5 VST3 reference covers CC 0, CC 2–127 and channel pressure 128 on
all 16 channels of input bus 0. CC 1 and pitch bend 129 are supported
on the first channel. Resolve the returned `studio_pro_name` with
`plugin_parameters` on the actual instance.
[MIDI channel verification](evidence/2026-11-26-serum2-midi-channels.md)

Set `event_input: 2` and `channel: 1`, or `event_input: 16` and
`channel: 15`, to look up the separately verified Studio Pro 8.1.1
route for CC 1, CC 74 or pitch bend 129. The corresponding MIDI wire
channels are 2 and 16. Both routes use the global rows for
Mod Wheel and Pitch Bend. CC 74 uses the selected port's own row.

### Change a Music track's Event Input

This is the Music track's **Out** route to an already connected Serum instance.
The same number can appear on several Serum instances. For Serum 2 FX, use
the Music source track in the route commands and inspect the FX on its
receiver's Audio channel.

1. In Studio Pro, select only that Music track. Stop playback and recording,
   and turn off the track's record arm and monitor.
2. Read the current document with `host_status`. Call
   `track_event_input_get` with the Music track name and that
   `expected_document`. Keep the returned `event_input`.
3. Get the destination's `native.instance.object_uid` with `plugin_inspect`.
   For the FX case, match the Music source's destination UUID to the
   insert's UUID as described below.
4. Call `serum2_event_input_set` once. For example, to move the currently
   routed instrument from Event Input 1 to 2, replace the two placeholder
   strings with the values just read.

   ```json
   {
     "track": "Serum Keys",
     "variant": "instrument",
     "expected_document": "<current document>",
     "expected_instance": "<destination UUID>",
     "expected_event_input": 1,
     "event_input": 2
   }
   ```

   For an FX destination, use `"variant": "fx"` and the Music source track's
   exact name. Choose an Event Input from 1 through 16.
5. Read the route again with `track_event_input_get` and confirm the
   destination UUID with `plugin_inspect`. If the request returns an
   unknown outcome, use those same fresh reads before any new request.

[Live route lookup](evidence/2026-11-26-serum-mcp-live-event-input.md)
[FX route lookup](evidence/2026-11-26-serum2-fx-mcp-live-event-input.md)
[Instrument and FX setter verification](evidence/2026-11-26-serum-mcp-event-input-set.md)

For Macro 1–8 in Serum 2 and Serum 2 FX 2.1.5, resolve
`vstp7000000` through `vstp7007000` in steps of 1000 on the exact DSP
instance. Use `plugin_parameter_set` with `normalized` and `readback: true`.
[Macro readback and restoration](evidence/2026-11-26-serum2-macro-readback.md)
[Assigned Macro 1 audio verification](evidence/2026-11-26-serum2-macro-routing.md)

To check a Music track routed to Serum 2 FX, call `plugin_inspect` with
`kind: "instrument"`, the exact `track`, `source: "dsp"`, and the current
`expected_document`. Inspect the candidate FX insert by its exact channel
and slot. Compare the two `native.instance.object_uid` values and use the
matching UID as `expected_instance` on a fresh track inspection. Use the
track's **Out** menu when choosing a different destination instance, then
repeat the inspection. [Live destination verification](evidence/2026-11-26-serum2-fx-live-destination.md)

To check the destination in a saved song, call `song_read` on the
`.song` file and look for `midi_destination` on the source track. A
resolved destination identifies the original Serum 2 instrument or the
specific Serum 2 FX insert, even with multiple FX copies in the song.
[Save and reopen verification](evidence/2026-11-26-serum2-fx-midi-persistence.md)

Use `serum2_factory_presets_list` to search the installed Xfer Factory
library. Filter with `variant: "instrument"` or `"fx"` and `search`.
Follow `next_offset` with the first page's `catalog_sha256` as
`expected_catalog`. Results include the source version and schema from the
preset header and the local path for **Menu → Load Preset** in Serum.
If you moved the library, start the MCP server with
`FSP_SERUM_FACTORY_PRESET_LIBRARY` set to the absolute `Presets/Factory`
directory.

Use `serum2_preset_inspect` with the absolute local `path` to identify a
Serum 2.1.5 `.SerumPreset` or processor-only `.vstpreset`. The tool reports
the instrument or FX variant, file SHA-256, and the matching load method.
`serum_editor` means use **Menu → Load Preset**. This reads the container
and state header. The effect of loading the preset still needs host readback.
[Preset file verification](evidence/2026-11-26-serum2-preset-inspection.md)

Use `serum2_preset_apply` to apply a processor-only `.vstpreset` file for
Serum 2 or Serum 2 FX 2.1.5. Supply the absolute local `path`, file
`expected_sha256` in lowercase, current `expected_document`, and native
`expected_instance`. Use the same instrument selector as `serum2_load`. For
FX, include the zero-based `slot` in the `target`. Stop playback and recording
before applying a preset. Use a separate `plugin_parameters` call to confirm
any change. For an `unknown` receipt, read the target before retrying.
Studio Pro displays the temporary file name for this processor-only import.
[Direct preset verification](evidence/2026-11-25-serum2-mcp-vst3-preset.md)
[Serum 2 FX demo preset recovery after reopen](evidence/2026-11-26-serum2-fx-demo-delay-recovery.md)
[Serum 2 FX internal signal](evidence/2026-11-26-serum2-fx-signal.md)
[Serum 2 FX assigned Macro 1 audio verification](evidence/2026-11-26-serum2-fx-macro-routing.md)
[Serum 2 instrument virtual MIDI and pitch-bend center](evidence/2026-11-26-serum2-live-midi.md)

Keep the source preset when using the Serum 2 instrument demo. In Studio
Pro 8.1.1, a saved song reopened with the original instrument and notes,
but its saved demo state omitted the applied delay rack.
[Demo song save and reopen verification](evidence/2026-11-26-serum2-demo-delay-reopen.md)

## FabFilter preset library

Call `fabfilter_presets_list` with `{"product":"Pro-Q 4"}` to see preset names and their folders. The tool covers the 23 FabFilter products and versions in the compatibility checks. It reads names under `~/Documents/FabFilter/Presets` by default. For a library stored elsewhere, set `FSP_FABFILTER_PRESET_LIBRARY` to the absolute parent of the per-product directories when starting the MCP server. [FabFilter preset folders](https://www.fabfilter.com/help/pro-q/presets/howpresetsarestored)

For another page, send `offset` set to `next_offset` and `expected_catalog` set to the first page's `catalog_sha256`. The digest covers the listed names and folders. FabFilter assigns its own MIDI program numbers, and those can change as its library changes. Use `plugin_presets` for presets issued by Studio Pro. [FabFilter program numbering](https://www.fabfilter.com/help/pro-q/presets/loadingpresets)

Use `fabfilter_preset_apply` for local `.ffp` files in Studio Pro 8.1.1 on Apple Silicon. The supported VST3 versions are Micro 1.32, One 3.52, Pro-C 2 2.22, Pro-DS 1.32, Pro-G 1.42, Pro-L 1.37, Pro-L 2 2.26, Pro-MB 1.33, Pro-Q 2 2.30, Pro-Q 3 3.29, Pro-Q 4 4.13, Pro-C 3 3.02, Pro-R 1.20, Pro-R 2 2.06, Saturn 1.31, Saturn 2 2.13, Simplon 1.42, Timeless 2 2.41, Timeless 3 3.10, Twin 2 2.41, Twin 3 3.10, Volcano 2 2.41, and Volcano 3 3.09. Micro reads its FMic version 1 factory format. One reads FFPr versions 0 and 1. Pro-C 2 reads FC2p version 2 and legacy FPrC version 1. Pro-DS reads its FPSr version 2 factory format. Pro-G reads its FPGr version 2 factory format. Pro-L reads its FPLr version 3 factory format. Pro-L 2 reads its FL2p version 2 factory format. Pro-MB reads its FPMb version 3 factory format. Pro-Q 2 reads its FQ2p version 2 factory format. Pro-Q 3 reads its FQ3p version 4 factory format. Pro-R reads its version 4 binary factory format. Pro-R 2 reads its current text factory format and the legacy binary format included in its factory library. Saturn reads FSat versions 1, 2, and 4. Saturn 2 reads FS2a versions 2–4 and legacy FSat version 4. Simplon reads its FSim version 1 factory format. Timeless 2 reads FTms versions 3–5, FTim version 2, and FDel version 1. Timeless 3 reads F3Ts versions 9 and 10. Twin 2 reads FT2i versions 6 and 8 and FTwi versions 1, 3, and 4. Volcano 2 reads FV2l versions 1 and 2 and FVol version 1. Volcano 3 reads FV3l versions 3–5. Open the intended plug-in's editor and supply its `expected_instance` and `expected_document`. For Micro, Pro-C 2, Pro-DS, Pro-G, Pro-L, Pro-L 2, Pro-MB, Pro-Q 2, Pro-Q 3, Pro-Q 4, Pro-C 3, Pro-R, Pro-R 2, Saturn, Saturn 2, Simplon, Timeless 2, Timeless 3, Volcano 2, or Volcano 3, supply the effect's `channel` and `slot`. For One, Twin 2, or Twin 3, select only the intended instrument track and supply its name as `track`. Set `expected_catalog` to the listing's `catalog_sha256` and `index` to the listing's `offset` plus the entry's zero-based position. The result carries the selected file's SHA-256. Use a separate `plugin_parameters` or `plugin_parameter_digest` request to confirm the change. [Micro verification](evidence/2026-11-24-fabfilter-micro-library-preset-apply.json) · [One verification](evidence/2026-11-24-fabfilter-one-library-preset-apply.json) · [Pro-C 2 verification](evidence/2026-11-24-fabfilter-proc2-library-preset-apply.json) · [Pro-DS verification](evidence/2026-11-24-fabfilter-prods-library-preset-apply.json) · [Pro-G verification](evidence/2026-11-24-fabfilter-prog-library-preset-apply.json) · [Pro-L verification](evidence/2026-11-24-fabfilter-prol-library-preset-apply.json) · [Pro-L 2 verification](evidence/2026-11-24-fabfilter-prol2-library-preset-apply.json) · [Pro-MB verification](evidence/2026-11-24-fabfilter-promb-library-preset-apply.json) · [Pro-Q 2 verification](evidence/2026-11-24-fabfilter-proq2-library-preset-apply.json) · [Pro-Q 3 verification](evidence/2026-11-24-fabfilter-proq3-library-preset-apply.json) · [Pro-Q 4 verification](evidence/2026-11-21-fabfilter-proq4-library-preset-apply.json) · [Pro-C 3 verification](evidence/2026-11-21-fabfilter-proc3-library-preset-apply.json) · [Pro-R verification](evidence/2026-11-21-fabfilter-pror-library-preset-apply.json) · [Pro-R 2 verification](evidence/2026-11-21-fabfilter-pror2-library-preset-apply.json) · [Saturn verification](evidence/2026-11-24-fabfilter-saturn-library-preset-apply.json) · [Saturn 2 verification](evidence/2026-11-24-fabfilter-saturn2-library-preset-apply.json) · [Simplon verification](evidence/2026-11-24-fabfilter-simplon-library-preset-apply.json) · [Timeless 2 verification](evidence/2026-11-24-fabfilter-timeless2-library-preset-apply.json) · [Timeless 3 verification](evidence/2026-11-24-fabfilter-timeless3-library-preset-apply.json) · [Twin 2 verification](evidence/2026-11-24-fabfilter-twin2-library-preset-apply.json) · [Twin 3 verification](evidence/2026-11-21-fabfilter-twin3-library-preset-apply.json) · [Volcano 2 verification](evidence/2026-11-24-fabfilter-volcano2-library-preset-apply.json) · [Volcano 3 verification](evidence/2026-11-24-fabfilter-volcano3-library-preset-apply.json)

For Pro-Q 4 VST3 4.13 on Apple Silicon, `plugin_inspect` with the expected instance can report `native.capabilities.fabfilter_editor_owner`. A `matching_attached_editor_count` of 1 means Studio Pro attached the editor to that instance. A count of 0 means that instance currently has no attached editor. `other_same_product_attached_editors` counts attached editors for other Pro-Q 4 instances in the host. [Actual editor ownership verification](evidence/2026-11-21-fabfilter-proq4-editor-ownership.json)

The same observation covers FabFilter Twin 3 VST3 3.10. Select only the intended instrument track before using `plugin_inspect` with its expected instance. `other_same_product_attached_editors` then counts other Twin 3 editors. [Twin 3 editor ownership verification](evidence/2026-11-21-fabfilter-twin3-editor-ownership.json)

## FabFilter MIDI controller mapping

Call `fabfilter_midi_mapping` with `{"product":"Micro","controller":7}` to look up the VST3 SDK mapping for CC 7. Values 128 and 129 select channel pressure and pitch bend. Program change (130) has no parameter in this SDK mapping. The result includes the tested VST3 version and the corresponding `studio_pro_name`. Compare the version and confirm the name with `plugin_parameters` on the current instance. The lookup itself sends no MIDI. [VST3 MIDI mapping interface](https://steinbergmedia.github.io/vst3_doc/vstinterfaces/classSteinberg_1_1Vst_1_1IMidiMapping.html)

FabFilter Micro 1.32 was verified in Studio Pro 8.1.1 on macOS arm64. Use `plugin_parameters` to read the bound instance and `plugin_parameter_set` to change a writable value. All 145 writable values were changed, read back, and restored individually. The separate program value `vstp1073741959` was verified as `0 → 1 → 0`. That round trip also required restoring a companion value and ended with all 146 reported values at baseline. [Micro verification details](evidence/2026-11-20-fabfilter-micro-controls.json)

FabFilter Pro-Q 3 3.29 passed individual change, independent readback and restoration of all 518 writable parameters in Studio Pro 8.1.1. The final readback matched all 519 reported parameters. [Pro-Q 3 verification details](evidence/2026-11-20-fabfilter-proq3-controls.json)

FabFilter Pro-Q 4 4.13 passed individual change, independent readback and restoration of all 736 writable parameters in Studio Pro 8.1.1. The final readback matched all 737 reported parameters. [Pro-Q 4 verification details](evidence/2026-11-20-fabfilter-proq4-controls.json)

FabFilter Pro-R 2 2.06 passed individual change, independent readback and restoration of all 272 writable parameters in Studio Pro 8.1.1. The final readback matched all 273 reported parameters. [Pro-R 2 verification](evidence/2026-11-20-fabfilter-pror2-controls.json)

FabFilter Saturn 1.31 passed individual change, independent readback and restoration of all 1,035 writable parameters in Studio Pro 8.1.1. The full restored snapshot matched all 1,036 reported parameters. [Saturn verification](evidence/2026-11-20-fabfilter-saturn-controls.json)

FabFilter Saturn 2 2.13 passed individual change, independent readback and restoration of all 1,087 writable parameters in Studio Pro 8.1.1. The full restored snapshot matched all 1,088 reported parameters. [Saturn 2 verification](evidence/2026-11-20-fabfilter-saturn2-controls.json)

FabFilter Simplon 1.42 passed individual change, independent readback and restoration of all 155 writable parameters in Studio Pro 8.1.1. Four virtual MIDI CC writes also changed a linked filter control. Restoring those controls and the affected internal value returned all 156 reported parameters to baseline. [Simplon verification](evidence/2026-11-20-fabfilter-simplon-controls.json)

FabFilter One 3.52 passed individual change, independent readback and restoration for all 167 writable parameters in Studio Pro 8.1.1. These include all 31 named controls, all 128 virtual MIDI CC values, the program list and the legacy CC. With the transport stopped, the program list and its read-only counter changed from 0 to 1 and returned to 0. The program change and modulation wheel each required a separate companion correction. All 168 reported values returned to baseline in both trials. [One writable controls](evidence/2026-11-20-fabfilter-one-controls.json) · [Direct program list verification](evidence/2026-11-20-fabfilter-one-direct-program.json) · [Legacy MIDI CC verification](evidence/2026-11-20-fabfilter-one-legacy-control.json) · [Modulation wheel verification](evidence/2026-11-20-fabfilter-one-modulation-wheel.json)

Some parameters share the display title `MIDI CC`. Use the SDK mapping to choose the parameter ID for a particular CC number. For Pro-G 1.42, Pro-C 2 2.22, Pro-C 3 3.02, Pro-DS 1.32, Pro-L 1.37, Pro-L 2 2.26 and Pro-MB 1.33, the legacy row `vstp1073741828` briefly accepted a value in Studio Pro 8.1.1 and returned to zero by the 150 ms readback. An independent parameter read confirmed zero. Use `plugin_parameter_set` with `readback: true` to detect this behavior and treat `applied_unverified` as a transient result for this row. Pro-R 1.20 kept the changed value through both readings and an explicit write restored it. [Studio Pro verification](evidence/2026-11-24-fabfilter-legacy-readback.json) · [Separate VST3 processor experiment](evidence/2026-11-20-fabfilter-legacy-midi.json)

FabFilter Pro-Q 2 2.30 passed individual change, independent readback and restoration of all 327 writable parameters in Studio Pro 8.1.1. The legacy `vstp1073741828` row also passed on this version. [Pro-Q 2 verification details](evidence/2026-11-20-fabfilter-proq2-controls.json)

FabFilter Timeless 2 2.41 passed individual change, independent readback and restoration for all 841 writable parameters in Studio Pro 8.1.1. The checks covered 128 virtual MIDI CC values and confirmed that a separate read observed the changed legacy controller value before restoration. [Timeless 2 verification](evidence/2026-11-20-fabfilter-timeless2-controls.json)

FabFilter Twin 2 2.41 passed individual change, independent readback and restoration for all 1,019 writable parameters in Studio Pro 8.1.1. The verified values include all 883 named controls, all 128 virtual MIDI CC values, the program list and the legacy CC. During MIDI CC changes, 20 linked named controls moved and 15 needed separate restoration. The final legacy CC check restored all 1,020 reported values. [Twin 2 writable controls](evidence/2026-11-20-fabfilter-twin2-controls.json) · [Direct program list verification](evidence/2026-11-20-fabfilter-twin2-direct-program.json) · [Legacy controller verification](evidence/2026-11-20-fabfilter-twin2-legacy-control.json)

FabFilter Timeless 3 3.10 passed individual change, independent readback and restoration for all 1,143 writable parameters in Studio Pro 8.1.1. The checks covered 128 virtual MIDI CC values and confirmed that a separate read observed the changed legacy controller value before restoration. [Timeless 3 verification](evidence/2026-11-20-fabfilter-timeless3-controls.json)

FabFilter Twin 3 3.10 passed individual change, independent readback and restoration of all 1,509 writable parameters in Studio Pro 8.1.1, including all 128 virtual MIDI CC values. A CC write can also change a parameter linked by MIDI Learn. Use `plugin_parameters` to check the linked value on the current instance. The verification restored the original values and matched all 1,510 reported parameters. [Full Twin 3 verification](evidence/2026-11-20-fabfilter-twin3-controls.json) · [Virtual CC details](evidence/2026-11-20-fabfilter-twin3-midi-cc.json) · [FabFilter factory MIDI CC table](https://www.fabfilter.com/help/twin/support/midi)

FabFilter Volcano 2 2.41 passed individual change, independent readback and restoration for all 893 writable parameters in Studio Pro 8.1.1. The checks covered 128 virtual MIDI CC values and confirmed that a separate read observed the changed legacy controller value before restoration. [Volcano 2 verification](evidence/2026-11-20-fabfilter-volcano2-controls.json)

FabFilter Volcano 3 3.09 passed individual change, independent readback and restoration for all 1,032 writable parameters in Studio Pro 8.1.1. The checks covered 128 virtual MIDI CC values and confirmed that a separate read observed the changed legacy controller value before restoration. [Volcano 3 verification](evidence/2026-11-20-fabfilter-volcano3-controls.json)

FabFilter Pro-R 1.20 passed individual change, independent readback and restoration of all 221 writable parameters in Studio Pro 8.1.1. Restoration matched all 222 reported parameters, including the independent check of the legacy MIDI CC value. [Pro-R verification](evidence/2026-11-20-fabfilter-pror-controls.json)

## FabFilter Twin 3 factory preset file

Twin 3 3.10 loaded the official `Ambient Mover.ffp` using **Open Other Preset** in the plug-in editor. The reader observed 433 changed parameters in Studio Pro 8.1.1. Restoring the original state with `plugin_state` returned 1,509 of 1,510 values. One verified `plugin_parameter_set` write returned the remaining internal float to zero. The complete typed readback, independent DSP digest and recaptured host component fingerprint all matched the baseline. This verification covers the named file and version. [Verification details](evidence/2026-11-20-fabfilter-twin3-vendor-preset.json) · [FabFilter loading instructions](https://prod.fabfilter.com/help/twin/presets/loadingpresets)

## FabFilter Pro-Q 4 factory preset file

Pro-Q 4 4.13 loaded the official `Musical Low Cut Filter.ffp` using **Open Other Preset** in the plug-in editor. Studio Pro 8.1.1 reported 110 changed parameters. Restoring the original state with `plugin_state` returned 736 of 737 values. A single `plugin_parameter_set` write returned the remaining internal value to normalized zero. The complete typed readback, independent DSP digest and recaptured host component fingerprint matched the baseline. This verification covers the named file and version. [Verification details](evidence/2026-11-20-fabfilter-proq4-vendor-preset.json) · [FabFilter loading instructions](https://prod.fabfilter.com/help/pro-q/presets/loadingpresets)

## FabFilter instrument program changes

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
and playback left **Default Sound** selected. [One and Twin 2 evidence](evidence/2026-11-20-fabfilter-program-parts.json) · [Twin 3 evidence](evidence/2026-11-20-twin3-midi-r25.json)

## External Effect Stereo Send

On Studio Pro 8.1.1 for Apple Silicon, `plugin_inspect` can report `native.capabilities.external_stereo_send_route.assigned` for the exact External Effect Stereo instance. `true` means the effect holds a logical Send route. `false` means Send is unassigned. A missing field means the native check was unavailable. The result covers routing assignment. Audio output and latency require a separate loopback test. [Live assignment verification](evidence/2026-11-21-external-stereo-send-assignment.json)

With the effect editor open, use `plugin_inspect` to identify the current instance and `external_port_menu` with `port: "send"` to read the current menu. The checked row identifies the logical Send choice. In the Studio Pro 8.1.1 test with Main at zero gain, choosing Main in the editor changed the checkmark and controller display while the raw port value stayed `-1`. Selecting None in the editor restored all 24 controller rows. [Live verification](evidence/2026-11-20-external-stereo-send-observation.json)

On the verified Apple Silicon build, the Stereo Send menu can include `held_send_route`. This records the logical assignment observed at the native lifecycle checks and confirms that the route was stable through the checked menu operations and cleanup. [Live menu lifecycle verification](evidence/2026-11-21-external-stereo-send-menu-guard.json)

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

A verified receipt covers one selection, the new menu checkmark and logical assignment, and a complete read of all 24 controller rows. An unknown outcome requires a fresh observation before any further change. Return, physical routing, audible output, and persistence require separate verification. [Live change and restore verification](evidence/2026-11-21-external-stereo-send-set.json)

## Saved plugin routing

For a saved Music track with a supported plug-in destination, `song_read` and `song-read` include `tracks[].midi_destination`. The result identifies the saved plug-in instance and port, plus the channel and zero-based slot for an insert. The reader omits the field when the saved reference is ambiguous or unsupported. For Serum 2 instruments and Serum 2 FX pre-fader inserts, the saved reader recognizes Event Input 1, 2 and 16. [Serum 2 instrument saved ports](evidence/2026-11-26-serum2-saved-event-destinations.md) · [Serum 2 FX saved ports](evidence/2026-11-26-serum2-fx-saved-event-destinations.md) · [Serum 2 FX save and reopen verification](evidence/2026-11-26-serum2-fx-midi-persistence.md)

For a supported saved Impact instance, `song_read` and `song-read` include `instrument_outputs` on the plugin. Each output reports its saved enabled state, speaker layout and mixer channel ID and name when connected. The reader checks both the saved output descriptors and mixer membership. If those disagree, it reports a warning and leaves the output graph absent. The same fields were verified in Studio Pro 8.1.1 scratch copies with `Impact St 2` enabled and restored after reopening.

use rmcp::{
    ErrorData, RoleServer, ServerHandler,
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, Implementation, ListToolsResult,
        PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool, ToolAnnotations,
    },
    service::RequestContext,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    fabfilter::{MidiMappingQuery, PresetApply, PresetQuery},
    operation::{self, Operation, Request, TrackEdit, TrackTask, schema},
    runtime::RuntimeHandle,
    serum::{PresetApply as SerumPresetApply, Variant as SerumVariant},
};

#[derive(Clone)]
pub struct McpServer {
    runtime: RuntimeHandle,
}

struct ToolSpec {
    tool: Tool,
    operation: Option<&'static str>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct FileQuery {
    path: std::path::PathBuf,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct HardwareQuery {}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Serum2Load {
    target: Serum2Target,
}

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "variant", rename_all = "snake_case", deny_unknown_fields)]
enum Serum2Target {
    Instrument {
        #[schemars(length(min = 1, max = 256))]
        track: String,
    },
    Fx {
        #[schemars(length(min = 1, max = 256))]
        channel: String,
    },
}

impl Serum2Load {
    fn into_operation(self) -> Result<Operation, ErrorData> {
        let edit = match self.target {
            Serum2Target::Instrument { track } => TrackEdit::AddInstrumentTrack {
                instrument: SerumVariant::Instrument.plugin_name().into(),
                name: Some(serum2_destination(track)?),
                class_id: Some(format!("{{{}}}", SerumVariant::Instrument.class_uid())),
            },
            Serum2Target::Fx { channel } => TrackEdit::AddPlugin {
                channel: serum2_destination(channel)?,
                plugin: SerumVariant::Fx.plugin_name().into(),
                class_id: Some(format!("{{{}}}", SerumVariant::Fx.class_uid())),
            },
        };
        Ok(Operation::TrackTask(TrackTask { ops: vec![edit] }))
    }
}

fn serum2_destination(name: String) -> Result<String, ErrorData> {
    if name.trim().is_empty() || name.len() > 256 || name.chars().any(char::is_control) {
        return Err(ErrorData::invalid_params(
            "A Serum destination must contain 1 to 256 UTF-8 bytes without control characters",
            None,
        ));
    }
    Ok(name)
}

fn hardware_query(args: serde_json::Map<String, Value>) -> Result<(), ErrorData> {
    serde_json::from_value::<HardwareQuery>(Value::Object(args))
        .map(|_| ())
        .map_err(|error| ErrorData::invalid_params(error.to_string(), None))
}

impl McpServer {
    pub fn new(runtime: RuntimeHandle) -> Self {
        Self { runtime }
    }

    fn specs() -> Vec<ToolSpec> {
        use operation::*;
        let mut specs = vec![
            named::<Empty>(
                "host_status",
                "context",
                "Read the current bridge session and document epoch. Call before editing.",
                false,
            ),
            named::<Empty>(
                "session_get",
                "song",
                "Read the active live session and transport, including unsaved state.",
                false,
            ),
            named::<TrackQuery>(
                "tracks_list",
                "tracks",
                "Read tracks, selection, layers and events from the active session.",
                false,
            ),
            named::<SelectTrack>(
                "track_select",
                "selectTrack",
                "Select a track by its exact name in the current document. Selection is exclusive by default. For duplicate names, use mediaType from tracks_list to identify one track. Read tracks_list again to verify selection before controlling an instrument.",
                true,
            ),
            named::<ChannelList>(
                "channels_list",
                "channels",
                "Read mixer channels and their actual state. Track and audio channel names may differ. Set include_color to read signed ARGB colors and include_selection to read Console selection when the channel can be identified.",
                false,
            ),
            named::<PannerQuery>(
                "panner_get",
                "panner",
                "Read standard channel panner values and native ranges. The pannerType label is read-only. This does not inspect a Surround Panner plugin.",
                false,
            ),
            named::<PannerSet>(
                "panner_set",
                "setPanner",
                "Set one standard channel panner parameter using its native raw range and read it back. This does not control a Surround Panner plugin.",
                true,
            ),
            named::<SplittersQuery>(
                "splitters_list",
                "splitters",
                "Read native Splitter nodes and parameter values from a channel's insert graph. Use the returned node name and label when editing.",
                false,
            ),
            named::<SplitterSet>(
                "splitter_set",
                "setSplitter",
                "Set one native Splitter parameter on the exact node and expected label. Values use native raw ranges; inactive branches and out-of-range values are refused. Matching readback determines verification.",
                true,
            ),
            named::<Empty>(
                "track_formats",
                "trackFormats",
                "Read supported native audio track format names before tracks_edit addAudioTrack. Speaker types are native masks, not channel counts.",
                false,
            ),
            named::<ChannelSet>(
                "channel_set",
                "setChannel",
                "Set a channel's target state. Volume is linear gain, pan is 0..1 with 0.5 center; switches are booleans.",
                true,
            ),
            named::<TransportSet>(
                "transport_set",
                "setTransport",
                "Set BPM, seconds or bar position and loop/precount/preroll target state.",
                true,
            ),
            named::<TransportAction>(
                "transport_action",
                "transport",
                "Execute a transport action. Record writes a take; toggles are explicit non-idempotent actions.",
                true,
            ),
            named::<NotesQuery>(
                "notes_read",
                "notes",
                "Read MIDI notes and part positions. Beat positions use quarter-note beats.",
                false,
            ),
            named::<EditNotes>(
                "notes_edit",
                "editNotes",
                "Edit selected track notes through the in-DAW EditTask. Filters limit the target notes.",
                true,
            ),
            named::<MidiProgramPartAdd>(
                "midi_program_part_add",
                "midiProgramPartAdd",
                "Create a note-free MIDI bank and program part at song start on the empty, exclusively selected Music track routed to the exact requested FabFilter VST3 instrument. Use one for One, twin_2 for Twin 2, or twin_3 for Twin 3. Omit instrument to target Twin 3. Requires an empty arrangement event selection and stopped transport at song start with loop, record arm, and input monitoring disabled. Verifies part and route only. If the outcome is unknown, inspect the track before retrying.",
                true,
            ),
            named::<TrackTask>(
                "tracks_edit",
                "trackTask",
                "Run finite EditTask operations for parts, routing, instruments, events and arranger sections. For addPlugin and addInstrumentTrack, pass the exact name and optional classId from a complete plugin_catalog result to select a specific class. Results are per item and batches are not atomic.",
                true,
            ),
            named::<Serum2Load>(
                "serum2_load",
                "trackTask",
                "Load Xfer Records Serum 2 VST3 as a new Music track with target.variant instrument and target.track, or add Serum 2 FX VST3 to an existing mixer channel's Inserts with target.variant fx and target.channel. Uses one exact-class track edit. Read tracks_list or the channel inserts after the receipt. Inspect the current song before retrying an unknown outcome.",
                true,
            ),
            named::<SerumPresetApply>(
                "serum2_preset_apply",
                "serum2PresetApply",
                "Apply a processor-only VST3 preset for Serum 2 or Serum 2 FX 2.1.5 from a local absolute path with playback and recording stopped. Supply the lowercase file SHA-256, current document, and exact native instance. Use target.variant instrument with track, or fx with channel and zero-based insert slot. The host receipt confirms acceptance only. Read plugin state before retrying any unknown outcome.",
                true,
            ),
            named::<PluginQuery>(
                "plugins_list",
                "plugins",
                "Discover names of host-exposed effects or instruments. Installed, licensed and tested sets may differ.",
                false,
            ),
            named::<PluginCatalogQuery>(
                "plugin_catalog",
                "pluginCatalog",
                "Discover live plugin class IDs and names by host category.",
                false,
            ),
            named::<PluginParams>(
                "plugin_parameters",
                "pluginParams",
                "Read named parameters, or enumerate a native page when names is empty. Follow next_offset until null with the same expected_document, expected_instance and source. complete means one page covers its current provider. source level_meter enumerates only six registered Level Meter GUI settings and requires Level Meter, expected_document and expected_instance. Native enumeration requires the matching optional extension; metadata fallback is partial.",
                false,
            ),
            named::<PluginParams>(
                "plugin_inspect",
                "pluginInspect",
                "Inspect a live plugin's available parameter enumeration interfaces without arbitrary script execution. With include_context=true, a bound Music instrument/note-effect target and explicit source add strict before/after fixture observations; these are not an atomic snapshot and cannot include nodes. With include_nodes=true, node_scope selects target, graph, the declared plugin_controller, cv_ports, cv_port_parameters, or cv_preset_target. CV scopes require exact CV Instrument instrument, dsp source, expected_document, expected_instance and no include_context. cv_ports observes bounded host PortSetup metadata. cv_port_parameters discovers bounded direct indexed host parameters. cv_preset_target observes the slot mediator target identity relationship and IStorable availability. The latter two scopes accept no names or pagination. The CV scopes establish no routing or mutation capability. impact_outputs reads the 32 live Impact output names and requested and acknowledged activation states. It uses the exact instrument track, dsp source, expected document and native instance. The controller scope requires an expected plugin and expected_instance and admits only the exact Mustang Native and Rumble Native classes or the exact Multi Instrument AudioSynth identity or exact Level Meter AudioEffect identity or exact Pedalboard AudioEffect identity. The alias provider may return no controller; its bounded read-only metadata may be partial and establishes no model catalog or mutation capability.",
                false,
            ),
            named::<PluginChoices>(
                "plugin_parameter_choices",
                "pluginChoices",
                "Read a native list parameter's index/label choices without changing its selection. Requires the matching native extension and a controller parameter; open the External Effect editor first. Follow next_offset with expected_document and expected_instance. source level_meter addresses only the registered Level Meter List settings. Empty hardware port lists remain explicit.",
                false,
            ),
            named::<PluginParameterDetails>(
                "plugin_parameter_details",
                "pluginParameterDetails",
                "Read native parameter precision, state flags and a bounded original-parameter chain. include_owner=true also observes bounded controller metadata. compare_gui=true compares the exact already-open Chorder editor's chorder.autofill parameter and declared observer with the direct dsp receiver. With both flags true, a fixed GUI observer-controller probe also reads local learnMode and learnInputPitch descriptors when canonically anchored. These are sequential observations with no mutation authority. It requires source dsp and expected_instance, opens no UI and grants no mutation access. Missing or ambiguous controls remain unresolved. Other details require an explicit dsp or controller source. List precision does not imply a choice count or selection index.",
                false,
            ),
            named::<PluginParameterFormat>(
                "plugin_parameter_format",
                "pluginParameterFormat",
                "Read the native display text for 1..32 explicitly supplied finite raw values without selecting or changing them. Requires an expected plugin, expected_instance and an explicit dsp, controller or level_meter source. level_meter admits only raw Integer values for its six registered settings and requires expected_document. The native parameter range and numeric type further constrain values. Returned labels do not establish a complete model choice list or count.",
                false,
            ),
            named::<PluginParameterDigest>(
                "plugin_parameter_digest",
                "pluginParameterDigest",
                "Read fresh native schema and value digests for the expected plugin's exposed parameter projection. Requires expected_instance and an explicit dsp or controller source. Native scan errors return no digest. Compare only digests with matching version, source, instance and scope. This is a sequential observation, not an atomic snapshot or complete plugin state; opaque contents remain unverified.",
                false,
            ),
            named::<PluginModels>(
                "plugin_models",
                "pluginModels",
                "Read bounded current-selector category or effect model titles for an expected Mustang Native or Rumble Native instance. Requires the matching native extension. Follow next_offset with the same expected_document, expected_instance and catalog. Lists may be filtered by category/edit context; they do not establish every model, an atomic snapshot or mutation authority. No selection, preview or confirmation occurs.",
                false,
            ),
            named::<PluginAction>(
                "plugin_action",
                "pluginAction",
                "Run a fixed native button action on the expected instance. swap_filter_frequencies supports only the exact Expander, Gate and Compressor classes. reset_meter_hold and reset_meter_loudness support only the exact Level Meter class. reset_minimum_reduction supports only the exact Limiter and Compressor classes. retrigger_scope supports only the exact Scope class. Its pulse does not verify waveform acquisition. A pulse or its own Toggle reset never verifies the separate effect. Meter reset effects require a separate independent non-neutral accumulator witness; the action receipt does not establish that effect. Never retry an unknown outcome.",
                true,
            ),
            named::<PedalboardSlotSwap>(
                "pedalboard_slot_swap",
                "pedalboardSlotSwap",
                "Exchange two occupied, directly adjacent positions containing different Pedalboard models. Read opt.stompmodel1 through opt.stompmodel8 from the exact DSP instance. Use their current display labels as the expected model names. Insert slot is zero-based and pedal positions are one-based. Stop playback and recording before use. Compare independent parameter reads after an applied_unverified receipt and stop on unknown outcomes.",
                true,
            ),
            named::<ImpactOutputSet>(
                "impact_output_set",
                "impactOutputSet",
                "Enable or disable one secondary output on the exact Impact instrument (indices 1 through 31). Read plugin_inspect with node_scope impact_outputs first. Supply that track, document, instance, zero-based output index, exact name, and expected acknowledged state. Use only when the requested and acknowledged states agree. Disabling removes the corresponding mixer channel. The last active output cannot be disabled. Never retry an unknown outcome.",
                true,
            ),
            named::<PluginState>(
                "plugin_state",
                "pluginState",
                "Run inspect_cv_port_structure for one fixed CV Instrument Output01 nested-parameter observation. It prepares and cleans a callback-sensitive native cache; it grants no routing or setter authority. Otherwise capture, restore or release a bounded in-memory plugin state, or read_rumble_named_parameters for bounded native Rumble descriptors associated with serialized sibling names, read_rumble_parameters for bounded native Rumble controller metadata, read_rumble_state for direct serialized Rumble processor IDs/numeric settings and inspect_rumble_serialization for a bounded structure summary. These reads take no token and grant no native write authority. Requires the matching native extension. Inspect first and provide expected_instance. Capture returns an opaque token bound to this service and document. Restore accepts only an issued token and requires independent parameter/model readback; a successful native load is applied_unverified. No preset file is written or imported.",
                true,
            ),
            named::<InsertChainState>(
                "insert_chain_state",
                "insertChainState",
                "Capture, restore or release the whole Inserts folder of one exact mixer channel with the matching native extension. Every action requires expected_document. Capture issues a private opaque token; restore and release require that same token. Optional expected_folder pins its native folder UUID. Set include_routing=true on capture to read the host-authored Splitter, merger and slot links when present. A successful load remains applied_unverified and requires independent graph and per-part parameter/model readback. No caller file path, URL, bytes or plugin slot is accepted.",
                true,
            ),
            named::<InsertSlotMove>(
                "insert_slot_move",
                "insertSlotMove",
                "Move one bound effect slot in the current Inserts graph with its Routing tab open. For a move after an effect, supply the destination slot as after_node and its current successor as following_node. For an empty two-way Splitter branch, supply the Splitter as after_node, its merger as following_node, and the branch index (0 or 1). Read the new graph and parameters independently.",
                true,
            ),
            named::<InsertSplitterAdd>(
                "insert_splitter_add",
                "insertSplitterAdd",
                "Insert a two-branch Splitter on the live link from a bound effect slot to its immediate merger. Open the exact rack’s Routing tab. Supply the effect, its instance UUID, the graph UUID, the effect slot node UUID in after_node, and the merger UUID in following_node. Independently inspect the new links after acceptance.",
                true,
            ),
            named::<InsertSplitterRemove>(
                "insert_splitter_remove",
                "insertSplitterRemove",
                "Remove one nested two-way Splitter with a single effect on each branch and a shared merger. Supply the mixer channel, one branch effect and instance UUID, the graph UUID, the Splitter and merger UUIDs, and the two slot-node UUIDs in branch order. Inspect and capture the graph first, then independently verify the affected links and effects.",
                true,
            ),
            named::<PluginPresets>(
                "plugin_presets",
                "pluginPresets",
                "List, apply or release presets from the expected native plugin slot. List issues a bounded catalog token; follow next with the same token, document and instance. Apply accepts only an index returned by that catalog and remains applied_unverified until independent parameter/model readback. Capture plugin_state before applying when restoration is needed. For the exact CV Instrument, inspect_cv_preset_settings reads the primary settings byte count and SHA-256 from an issued index without loading the preset. apply_cv_preset_settings loads those primary settings once on the bound CV component and reports applied_unverified on acceptance. No caller-supplied preset path, URL or state data is accepted; excluded preset sources remain explicit.",
                true,
            ),
            named::<PresetApply>(
                "fabfilter_preset_apply",
                "fabfilterPresetApply",
                "Apply a local FabFilter preset selected by the zero-based index and catalog SHA-256 from fabfilter_presets_list. Supports Micro 1.32, One 3.52, Pro-C 2 2.22, Pro-DS 1.32, Pro-G 1.42, Pro-L 1.37, Pro-L 2 2.26, Pro-MB 1.33, Pro-Q 2 2.30, Pro-Q 3 3.29, Pro-Q 4 4.13, Pro-C 3 3.02, Pro-R 1.20, Pro-R 2 2.06, Saturn 1.31, Saturn 2 2.13, Simplon 1.42, Timeless 2 2.41, Timeless 3 3.10, and Twin 3 3.10 VST3 on Studio Pro 8.1.1 for Apple Silicon. Open the target editor and supply the exact instance and document. Supply channel and slot for an effect, or the exclusively selected instrument track for One or Twin 3. Confirm effects with a separate readback.",
                true,
            ),
            named::<PluginEditor>(
                "plugin_editor_open",
                "pluginEditorOpen",
                "Request the expected plugin editor to open. The acknowledgment does not verify window visibility.",
                true,
            ),
            named::<ChorderAutofillSet>(
                "chorder_autofill_set",
                "editChorderAutofill",
                "Edit only Auto Fill on an already-open exact Chorder editor. The finite GUI receiver is distinct from the DSP facade. Successful receipts remain applied_unverified and report GUI current-value evidence separately; independently verify the full provider and saved-state persistence. Never retry an unknown outcome.",
                true,
            ),
            named::<ChorderLearningGet>(
                "chorder_learning_get",
                "chorderLearningGet",
                "Read only Learn Mode and the learning input pitch on an already-open exact Chorder editor. Requires the current document and native instance. The fixed GUI discovery ceiling is 256 views; the report exposes measured coverage and sequential GUI values, with no mutation authority or DSP/persistence/audio verification.",
                false,
            ),
            named::<ExternalPortMenu>(
                "external_port_menu",
                "externalPortMenu",
                "Observe one Send or Return menu on an already-open exact External Effect editor. Requires the current document and native instance. Menu prepare/build/cleanup callbacks may reenter; zero selection does not mean callback-free access. Returns bounded opaque row metadata only after positive cleanup. Optional include_check_only=true requires provenance=true and exact Stereo Send; it queries the two fixed Param leaves using public checkOnly flags1, never flags0 execution or selection, and does not check the global command filter. Optional provenance=true is restricted to Stereo Send and its fixed first two Param leaves; it reports sequential public identity relations with current mapped-bank target checks before and after the native call. This read-only bracket does not prove atomicity or continuous active-mixer membership; menuData typed values remain unobserved. No port setter, command, numeric mapping, physical routing, audio or persistence authority is granted. Never retry an unknown outcome.",
                true,
            ),
            named::<ExternalStereoSendSet>(
                "external_stereo_send_set",
                "externalStereoSendSet",
                "Set External Effect Stereo Send to main or none on Studio Pro 8.1.1 Apple Silicon with a Korean interface. Requires a current document, exact instance, channel, slot, expected_current and destination. Open the editor, stop transport, disable click and preroll, mute and disarm the target audio channel, and use Main at zero gain with the mixer reporting a null Main output. Reads the logical route, menu checkmark and 24 controller rows after one selection. Return, physical routing, audio and persistence require separate verification. Never retry an unknown outcome.",
                true,
            ),
            named::<ChorderChordsGet>(
                "chorder_chords_get",
                "chorderChordsGet",
                "Read a page of stored Chorder input chords from the exact Note FX instance. Requires the current document, instance, offset 0..127 and limit 1..16. Empty inputs are explicit. Compare model and capture digests across pages because each page is a fresh sequential observation. No GUI, imported XML, file path, mutation, audio or persistence claim is supplied.",
                false,
            ),
            named::<ChorderLearningSet>(
                "chorder_learning_set",
                "chorderLearningSet",
                "Change one Chorder learning control through a fresh GUI binding. Use change kind learn_mode with a Boolean enabled, or input_pitch with a required pitch that is null or MIDI integer 0..127. Null selects no learning key; zero selects MIDI 0. chord_note compares required expected_input_pitch and changes output_pitch membership with Boolean enabled; both pitches are MIDI integers 0..127. It requires Mode1 and the exact selected input, and never selects one implicitly. Receipts remain applied_unverified with GUI current-value evidence; output mapping, stored model, persistence and audio require independent checks. Never retry an unknown outcome.",
                true,
            ),
            named::<PluginSet>(
                "plugin_parameter_set",
                "setPluginParam",
                "Set exactly one raw, normalized, display-text or exact finite f64 float_bits value on the expected plugin instance. An explicit source uses native access and type checks; float_bits requires it and preserves the 16 hexadecimal digits from native_value.data.bits_hex. Set readback=true with an explicit dsp or controller source, expected_instance and a canonical parameter name to request a separate read of the same instance. The result reports matched, mismatch or unavailable. Optional edit_lifecycle=native_user invokes a finite Chorder Auto Fill raw 0/1 pilot with explicit dsp and expected_instance. Live readback measures current value at the observation time.",
                true,
            ),
            named::<InsertBypass>(
                "plugin_bypass",
                "setInsertBypass",
                "Set insert or entire insert-rack bypass to a target state.",
                true,
            ),
            named::<SendSet>(
                "send_set",
                "setSend",
                "Set an existing send's normalized level and mute target state.",
                true,
            ),
            named::<AutomationSet>(
                "automation_mode_set",
                "setAutomation",
                "Set channel automation mode. Write mode can replace existing automation during playback.",
                true,
            ),
            named::<CommandQuery>(
                "commands_search",
                "listCommands",
                "Discover actual command names and optional current enabled state.",
                false,
            ),
            named::<Command>(
                "command_run",
                "command",
                "Run a discovered contextual command or check its enabled state. Acknowledgment does not prove its workflow completed.",
                true,
            ),
        ];
        let empty = json!({"type":"object","properties":{},"additionalProperties":false});
        specs.push(ToolSpec { operation: None,
            tool: Tool::new("hardware_devices", "Read bounded operating-system CoreAudio devices/defaults and CoreMIDI endpoints. This does not identify the DAW-selected device, open streams, change settings, send MIDI, or verify physical key events or routing. MIDI unique IDs are signed integers; re-resolve them before use.", empty.as_object().unwrap().clone())
                .with_annotations(ToolAnnotations::default().read_only(true)),
        });
        let midi_input_schema = serde_json::to_value(schemars::schema_for!(
            crate::hardware::midi_input::ObserveMidiInput
        ))
        .unwrap();
        specs.push(ToolSpec { operation: None,
            tool: Tool::new("midi_input_observe", "Observe one exact currently-online MIDI source for a bounded duration and event cap without selecting a destination or forwarding/sending messages. Requires signed nonzero source_uid, duration_ms 100..5000 and max_events 1..256. Empty observations do not verify physical key performance. Inspect cleanup and source stability before accepting events.", midi_input_schema.as_object().unwrap().clone())
                .with_annotations(ToolAnnotations::default().read_only(true)),
        });
        specs.push(ToolSpec { operation: None, tool: Tool::new("operations_list", "Get the finite operation schema, including less common operations. No arbitrary eval or raw bridge escape hatch is available.", empty.as_object().unwrap().clone()) });
        specs.push(ToolSpec { operation: None, tool: Tool::new("operation_run", "Run a typed operation from operations_list. Supply expected_document for mutations. Receipts distinguish observed, verified, unverified, partial and unknown outcomes.", schema().as_object().unwrap().clone()) });
        let fabfilter_schema = serde_json::to_value(schemars::schema_for!(PresetQuery)).unwrap();
        specs.push(ToolSpec {
            operation: None,
            tool: Tool::new("fabfilter_presets_list", "List .ffp names and relative folders for one FabFilter product in the local preset library. Follow next_offset with expected_catalog set to the first page's catalog_sha256. The result lists files on disk. For eligible products, use the catalog digest and zero-based entry index with fabfilter_preset_apply.", fabfilter_schema.as_object().unwrap().clone())
                .with_annotations(ToolAnnotations::default().read_only(true)),
        });
        let mapping_schema = serde_json::to_value(schemars::schema_for!(MidiMappingQuery)).unwrap();
        specs.push(ToolSpec {
            operation: None,
            tool: Tool::new("fabfilter_midi_mapping", "Look up the reference VST3 SDK mapping for a FabFilter controller on bus 0, channel 0. Use CC 0–127, channel pressure 128, pitch bend 129, or program change 130. Compare the reference version with your VST3 bundle and resolve the parameter on the current instance with plugin_parameters. This lookup does not inspect a live instance or send MIDI.", mapping_schema.as_object().unwrap().clone())
                .with_annotations(ToolAnnotations::default().read_only(true)),
        });
        let saved_schema = serde_json::to_value(schemars::schema_for!(FileQuery)).unwrap();
        specs.push(ToolSpec {
            operation: None,
            tool: Tool::new("song_read", "Read a saved .song archive without editing it. Results are saved state, not unsaved live state; plugin parameter snapshots may be partial.", saved_schema.as_object().unwrap().clone()),
        });
        specs.push(ToolSpec {
            operation: None,
            tool: Tool::new("audio_analyze", "Analyze a WAV file with native Rust loudness, peak, RMS and stereo measurements. Loudness requires known mono/stereo layout; results include format and measurement limitations.", saved_schema.as_object().unwrap().clone()),
        });
        specs
    }
}

fn named<T: JsonSchema>(
    name: &'static str,
    operation: &'static str,
    description: &'static str,
    mutation: bool,
) -> ToolSpec {
    let mut schema = serde_json::to_value(schemars::schema_for!(T)).expect("tool schema");
    if mutation
        || matches!(operation, "chorderLearningGet" | "chorderChordsGet")
        || matches!(
            operation,
            "pluginParams"
                | "pluginInspect"
                | "pluginChoices"
                | "pluginParameterDetails"
                | "pluginParameterFormat"
                | "pluginParameterDigest"
                | "pluginModels"
        )
    {
        schema["properties"]["expected_document"] = json!({"type":"string","description":"Document token returned by host_status. Refuses stale document intents."});
    }
    if mutation || matches!(operation, "chorderLearningGet" | "chorderChordsGet") {
        let object = schema.as_object_mut().unwrap();
        object
            .entry("required")
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .unwrap()
            .push(json!("expected_document"));
    }
    if operation == "pluginInspect" {
        schema["allOf"] = json!([{
            "if":{"properties":{"node_scope":{"enum":["cv_ports","cv_port_parameters","cv_preset_target"]}},"required":["node_scope"]},
            "then":{"required":["expected_document","expected_instance","kind","track","plugin","source","include_nodes"],
                "properties":{"expected_document":{"type":"string","minLength":1},"expected_instance":{"type":"string","minLength":36},
                    "kind":{"const":"instrument"},"plugin":{"const":"CV Instrument"},"source":{"const":"dsp"},
                    "include_nodes":{"const":true},"include_context":{"const":false}}}
        }, {"if":{"properties":{"node_scope":{"enum":["cv_port_parameters","cv_preset_target"]}},"required":["node_scope"]},
            "then":{"properties":{"names":{"maxItems":0}},"not":{"anyOf":[{"required":["offset"]},{"required":["limit"]}]}}
        }, {"if":{"properties":{"node_scope":{"const":"impact_outputs"}},"required":["node_scope"]},
            "then":{"required":["expected_document","expected_instance","kind","track","plugin","source","include_nodes"],
                "properties":{"expected_document":{"type":"string","minLength":1},"expected_instance":{"type":"string","minLength":36},
                    "kind":{"const":"instrument"},"plugin":{"const":"Impact"},"source":{"const":"dsp"},
                    "track":{"type":"string","minLength":1,"maxLength":512},
                    "include_nodes":{"const":true},"include_context":{"const":false},"names":{"maxItems":0}},
                "not":{"anyOf":[{"required":["channel"]},{"required":["offset"]},{"required":["limit"]}]}}
        }]);
    }
    if operation == "pluginPresets" {
        schema["allOf"] = json!([{
            "if":{"properties":{"action":{"enum":["inspect_cv_preset_settings","apply_cv_preset_settings"]}},"required":["action"]},
            "then":{"required":["kind","track","catalog_token","index"],
                "properties":{"kind":{"const":"instrument"},"plugin":{"const":"CV Instrument"},
                    "track":{"type":"string","minLength":1,"maxLength":512},
                    "slot":{"const":0},"catalog_token":{"type":"string","pattern":"^[0-9A-F]{64}$"},
                    "index":{"type":"integer","minimum":0,"maximum":4095}},
                "not":{"anyOf":[{"required":["channel"]},{"required":["offset"]},{"required":["limit"]}]}}
        }]);
    }
    if operation == "pluginState" {
        crate::operation::cv_structure_schema(&mut schema);
    }
    let mut tool = Tool::new(name, description, schema.as_object().unwrap().clone());
    if matches!(operation, "chorderLearningGet" | "chorderChordsGet") {
        tool = tool.with_annotations(ToolAnnotations::default().read_only(true));
    } else if operation == "externalPortMenu" {
        tool = tool.with_annotations(ToolAnnotations::default().read_only(false));
    }
    ToolSpec {
        operation: Some(operation),
        tool,
    }
}

impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")))
            .with_instructions("Read host_status before editing and pass its document token. Volume uses linear gain, normalized plugin values use 0..1. Treat applied_unverified and outcome_unknown honestly; never retry ambiguous non-idempotent operations automatically. Only real live reports establish host-version and plugin support.")
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        Self::specs()
            .into_iter()
            .find(|spec| spec.tool.name == name)
            .map(|spec| spec.tool)
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult {
            tools: Self::specs().into_iter().map(|spec| spec.tool).collect(),
            ..Default::default()
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let mut args = request.arguments.unwrap_or_default();
        let spec = Self::specs()
            .into_iter()
            .find(|spec| spec.tool.name == request.name)
            .ok_or_else(|| ErrorData::invalid_params("Unknown tool", None))?;
        if request.name == "operations_list" {
            return Ok(CallToolResult::structured(json!({"schema":schema(),"provenance":"server implementation; live support is version dependent","arbitrary_eval":false})).into());
        }
        if request.name == "hardware_devices" {
            hardware_query(args)?;
            let result = tokio::task::spawn_blocking(crate::hardware::inspect)
                .await
                .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;
            return Ok(match result {
                Ok(value) => CallToolResult::structured(value),
                Err(message) => CallToolResult::structured_error(json!({"scope":"operating_system","error":{"code":"hardware_inventory_unavailable","message":message}})),
            }.into());
        }
        if request.name == "midi_input_observe" {
            let query: crate::hardware::midi_input::ObserveMidiInput =
                serde_json::from_value(Value::Object(args))
                    .map_err(|error| ErrorData::invalid_params(error.to_string(), None))?;
            crate::hardware::midi_input::validate(&query)
                .map_err(|error| ErrorData::invalid_params(error.to_string(), None))?;
            let result =
                tokio::task::spawn_blocking(move || crate::hardware::midi_input::observe(&query))
                    .await
                    .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;
            return Ok(match result {
                Ok(value) => CallToolResult::structured(value),
                Err(error) => CallToolResult::structured_error(
                    json!({"scope":"passive_midi_source","error":error}),
                ),
            }
            .into());
        }
        if request.name == "fabfilter_presets_list" {
            let query: PresetQuery = serde_json::from_value(Value::Object(args))
                .map_err(|error| ErrorData::invalid_params(error.to_string(), None))?;
            query
                .validate()
                .map_err(|error| ErrorData::invalid_params(error, None))?;
            let result = tokio::task::spawn_blocking(move || crate::fabfilter::list(&query))
                .await
                .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;
            return Ok(match result {
                Ok(page) => CallToolResult::structured(serde_json::to_value(page).unwrap()),
                Err(error) => CallToolResult::structured_error(json!({"scope":"local_fabfilter_preset_library","error":{"code":error.code(),"message":error.message()}})),
            }.into());
        }
        if request.name == "fabfilter_midi_mapping" {
            let query: MidiMappingQuery = serde_json::from_value(Value::Object(args))
                .map_err(|error| ErrorData::invalid_params(error.to_string(), None))?;
            let mapping = query
                .lookup()
                .map_err(|error| ErrorData::invalid_params(error, None))?;
            return Ok(CallToolResult::structured(serde_json::to_value(mapping).unwrap()).into());
        }
        if request.name == "song_read" {
            let query: FileQuery = serde_json::from_value(Value::Object(args))
                .map_err(|error| ErrorData::invalid_params(error.to_string(), None))?;
            let result = tokio::task::spawn_blocking(move || crate::song::read_song(&query.path))
                .await
                .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;
            return Ok(match result {
                Ok(saved) => CallToolResult::structured(serde_json::to_value(saved).unwrap()),
                Err(message) => CallToolResult::structured_error(
                    json!({"source":"saved","error":{"code":"song_read_failed","message":message}}),
                ),
            }
            .into());
        }
        if request.name == "audio_analyze" {
            let query: FileQuery = serde_json::from_value(Value::Object(args))
                .map_err(|error| ErrorData::invalid_params(error.to_string(), None))?;
            let result =
                tokio::task::spawn_blocking(move || crate::audio::analyze_file(&query.path))
                    .await
                    .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;
            return Ok(match result {
                Ok(analysis) => CallToolResult::structured(serde_json::to_value(analysis).unwrap()),
                Err(message) => CallToolResult::structured_error(json!({"source":"file","error":{"code":"audio_analyze_failed","message":message}})),
            }.into());
        }
        let operation = if let Some(name) = spec.operation {
            let expected_document = args
                .remove("expected_document")
                .map(|v| {
                    v.as_str().map(str::to_owned).ok_or_else(|| {
                        ErrorData::invalid_params("expected_document must be a string", None)
                    })
                })
                .transpose()?;
            let operation = if request.name == "serum2_load" {
                serde_json::from_value::<Serum2Load>(Value::Object(args))
                    .map_err(|error| ErrorData::invalid_params(error.to_string(), None))?
                    .into_operation()?
            } else {
                Operation::from_parts(name, Value::Object(args))
                    .map_err(|error| ErrorData::invalid_params(error.to_string(), None))?
            };
            Request {
                operation,
                expected_document,
            }
        } else {
            serde_json::from_value::<Request>(Value::Object(args))
                .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?
        };
        if (!operation.operation.is_read_only() || operation.operation.requires_explicit_document())
            && operation
                .expected_document
                .as_ref()
                .is_none_or(String::is_empty)
        {
            return Ok(CallToolResult::structured_error(json!({"outcome":"not_applied","error":{"code":"missing_precondition","message":"Read host_status and supply expected_document"}})).into());
        }
        let result = match self.runtime.execute(operation).await {
            Ok(receipt) => serde_json::to_value(receipt).unwrap(),
            Err(error) => json!({"outcome":"not_applied","error":error}),
        };
        let failed = result.get("error").is_some_and(|error| !error.is_null());
        Ok(if failed {
            CallToolResult::structured_error(result)
        } else {
            CallToolResult::structured(result)
        }
        .into())
    }
}

#[cfg(test)]
mod tests {
    const EXPECTED_TOOL_COUNT: usize = 59;

    use super::*;

    #[test]
    fn serum2_load_reuses_exact_class_track_edits() {
        let spec = McpServer::specs()
            .into_iter()
            .find(|spec| spec.tool.name == "serum2_load")
            .unwrap();
        let schema = &spec.tool.input_schema;
        assert_eq!(spec.operation, Some("trackTask"));
        assert_eq!(schema["required"], json!(["target", "expected_document"]));
        assert_eq!(schema["additionalProperties"], false);
        for (target, edit) in [
            (
                json!({"variant":"instrument","track":"My Serum"}),
                json!({"op":"addInstrumentTrackByClass","instrument":"Serum 2","name":"My Serum","classId":"{56534558-6673-5073-6572-756D20320000}"}),
            ),
            (
                json!({"variant":"fx","channel":"Mix"}),
                json!({"op":"addPluginByClass","plugin":"Serum 2 FX","channel":"Mix","classId":"{56534558-6673-5173-6572-756D20322066}"}),
            ),
        ] {
            let load: Serum2Load = serde_json::from_value(json!({"target":target})).unwrap();
            let operation = load.into_operation().unwrap();
            assert!(!operation.is_read_only());
            assert!(operation.validate().is_ok());
            assert_eq!(
                operation.wire(),
                ("trackTask".into(), json!({"ops":[edit]}))
            );
        }
    }

    #[test]
    fn serum2_load_refuses_extra_keys_wrong_targets_and_invalid_labels() {
        for args in [
            json!({"target":{"variant":"fx","channel":"Mix","classId":"arbitrary"}}),
            json!({"target":{"variant":"instrument","track":"Keys","channel":"Mix"}}),
            json!({"target":{"variant":"fx","track":"Keys"}}),
            json!({"target":{"variant":"au","channel":"Mix"}}),
            json!({"target":{"variant":"fx","channel":"Mix"},"plugin":"Other"}),
        ] {
            assert!(serde_json::from_value::<Serum2Load>(args).is_err());
        }
        for label in [
            String::from("  "),
            String::from("Mix\u{0000}"),
            "악".repeat(86),
        ] {
            let load: Serum2Load =
                serde_json::from_value(json!({"target":{"variant":"fx","channel":label}})).unwrap();
            assert!(load.into_operation().is_err());
        }
    }

    #[test]
    fn serum_preset_apply_is_exact_document_bound_and_local_file_only() {
        let spec = McpServer::specs()
            .into_iter()
            .find(|spec| spec.tool.name == "serum2_preset_apply")
            .unwrap();
        let tool_schema = &spec.tool.input_schema;
        assert_eq!(tool_schema["additionalProperties"], false);
        assert_eq!(
            tool_schema["required"],
            json!([
                "target",
                "expected_instance",
                "path",
                "expected_sha256",
                "expected_document"
            ])
        );
        assert_eq!(
            tool_schema["properties"]["expected_sha256"]["pattern"],
            "^[0-9a-f]{64}$"
        );
        let args = json!({
            "target":{"variant":"fx","channel":"FX Return","slot":15},
            "expected_instance":"DF52AB13-A21E-4EDA-855C-CA68851BACA2",
            "path":"/private/fixture.vstpreset",
            "expected_sha256":"a".repeat(64),
        });
        let operation = Operation::from_parts(spec.operation.unwrap(), args.clone()).unwrap();
        assert!(!operation.is_read_only());
        assert!(operation.requires_explicit_document());
        assert_eq!(
            operation.wire(),
            (
                "serum2PresetApply".into(),
                json!({
                    "target":args["target"], "expected_instance":args["expected_instance"],
                })
            )
        );
        for alias in ["serum2PresetApply", "serum2_preset_apply"] {
            let mut request: Request = serde_json::from_value(json!({
                "operation":alias, "args":args,
            }))
            .unwrap();
            assert_eq!(
                crate::runtime::validate_request(&request).unwrap_err().code,
                "missing_precondition"
            );
            request.expected_document = Some("current".into());
            assert!(crate::runtime::validate_request(&request).is_ok());
        }
        for (key, value) in [
            (
                "target",
                json!({"variant":"fx","channel":"FX Return","slot":16}),
            ),
            (
                "expected_instance",
                json!("df52ab13-a21e-4eda-855c-ca68851baca2"),
            ),
            (
                "expected_instance",
                json!("00000000-0000-0000-0000-000000000000"),
            ),
            ("expected_instance", json!("short")),
            ("path", json!("relative.vstpreset")),
            ("path", json!("/private/raw.SerumPreset")),
            ("expected_sha256", json!("A".repeat(64))),
        ] {
            let mut invalid = args.clone();
            invalid[key] = value;
            assert!(
                Operation::from_parts(spec.operation.unwrap(), invalid)
                    .unwrap()
                    .validate()
                    .is_err(),
                "{key}"
            );
        }
        let mut mixed_target = args;
        mixed_target["target"]["track"] = json!("Keys");
        assert!(Operation::from_parts(spec.operation.unwrap(), mixed_target).is_err());
    }

    #[test]
    fn cv_structure_schema_keeps_tool_count_and_a_mutating_fixed_action() {
        let specs = McpServer::specs();
        assert_eq!(specs.len(), EXPECTED_TOOL_COUNT);
        let tool = &specs
            .iter()
            .find(|s| s.tool.name == "plugin_state")
            .unwrap()
            .tool;
        let named = Value::Object((*tool.input_schema).clone());
        assert_eq!(
            named["allOf"][0]["then"]["properties"]["kind"]["const"],
            "instrument"
        );
        assert_eq!(
            named["allOf"][0]["then"]["not"]["anyOf"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
        assert_eq!(schema()["$defs"]["PluginState"]["allOf"], named["allOf"]);
        assert_eq!(
            named["$defs"]["PluginStateAction"]["enum"]
                .as_array()
                .unwrap()
                .len(),
            8
        );
        assert!(
            named["required"]
                .as_array()
                .unwrap()
                .contains(&json!("expected_document"))
        );
        assert!(tool.annotations.is_none());
    }

    #[test]
    fn named_rumble_schema_adds_only_one_action_and_preserves_tool_annotations() {
        let specs = McpServer::specs();
        assert_eq!(specs.len(), EXPECTED_TOOL_COUNT);
        let spec = specs
            .iter()
            .find(|s| s.tool.name == "plugin_state")
            .unwrap();
        assert!(spec.tool.annotations.is_none());
        let schema = Value::Object((*spec.tool.input_schema).clone());
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(
            schema["$defs"]["PluginStateAction"]["enum"],
            json!([
                "inspect_cv_port_structure",
                "read_rumble_state",
                "read_rumble_parameters",
                "read_rumble_named_parameters",
                "inspect_rumble_serialization",
                "capture",
                "restore",
                "release"
            ])
        );
        let fields = schema["properties"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            fields,
            [
                "channel",
                "slot",
                "kind",
                "track",
                "plugin",
                "expected_instance",
                "action",
                "token",
                "expected_document"
            ]
            .into_iter()
            .collect()
        );
    }

    #[test]
    fn stereo_send_schema_needs_exact_document_current_route_and_target() {
        let spec = McpServer::specs()
            .into_iter()
            .find(|spec| spec.tool.name == "external_stereo_send_set")
            .unwrap();
        assert_eq!(spec.operation, Some("externalStereoSendSet"));
        let schema = &spec.tool.input_schema;
        let required = schema["required"].as_array().unwrap();
        for field in [
            "expected_document",
            "channel",
            "slot",
            "expected_instance",
            "expected_current",
            "destination",
        ] {
            assert!(required.contains(&json!(field)), "{field}");
        }
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(schema["properties"]["slot"]["maximum"], 15);
        assert_eq!(
            schema["$defs"]["ExternalStereoSendDestination"]["enum"],
            json!(["main", "none"])
        );
    }

    #[test]
    fn external_menu_schema_is_closed_and_completion_sensitive() {
        assert_eq!(McpServer::specs().len(), EXPECTED_TOOL_COUNT);
        let spec = McpServer::specs()
            .into_iter()
            .find(|s| s.tool.name == "external_port_menu")
            .unwrap();
        assert_eq!(spec.operation, Some("externalPortMenu"));
        assert_eq!(
            spec.tool.annotations.as_ref().unwrap().read_only_hint,
            Some(false)
        );
        let schema = Value::Object((*spec.tool.input_schema).clone());
        for key in [
            "channel",
            "slot",
            "plugin",
            "expected_instance",
            "port",
            "expected_document",
        ] {
            assert!(
                schema["required"].as_array().unwrap().contains(&json!(key)),
                "{key}"
            );
        }
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(schema["properties"]["provenance"]["type"], "boolean");
        assert_eq!(
            schema["properties"]["include_check_only"]["type"],
            "boolean"
        );
        assert!(
            !schema["required"]
                .as_array()
                .unwrap()
                .contains(&json!("include_check_only"))
        );
        let union = McpServer::specs()
            .into_iter()
            .find(|s| s.tool.name == "operation_run")
            .unwrap();
        assert!(
            serde_json::to_string(&*union.tool.input_schema)
                .unwrap()
                .contains("include_check_only")
        );
        assert!(
            !schema["required"]
                .as_array()
                .unwrap()
                .contains(&json!("provenance"))
        );
        for key in [
            "action", "source", "param", "path", "value", "token", "max_rows", "kind", "track",
        ] {
            assert!(schema["properties"].get(key).is_none(), "{key}");
        }
    }

    #[test]
    fn hardware_inventory_is_a_closed_local_read() {
        let spec = McpServer::specs()
            .into_iter()
            .find(|spec| spec.tool.name == "hardware_devices")
            .unwrap();
        assert!(spec.operation.is_none());
        assert_eq!(spec.tool.annotations.unwrap().read_only_hint, Some(true));
        let schema = Value::Object((*spec.tool.input_schema).clone());
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(schema["properties"], json!({}));
        assert!(hardware_query(serde_json::Map::new()).is_ok());
        for (key, value) in [
            ("path", json!("/unchecked")),
            ("command", json!("anything")),
            ("source_uid", json!(1)),
            ("expected_document", json!("stale")),
            ("include_audio", json!(true)),
        ] {
            assert!(hardware_query(serde_json::Map::from_iter([(key.into(), value)])).is_err());
        }
    }

    #[test]
    fn passive_midi_tool_keeps_the_finite_source_only_contract() {
        let spec = McpServer::specs()
            .into_iter()
            .find(|spec| spec.tool.name == "midi_input_observe")
            .unwrap();
        assert!(spec.operation.is_none());
        assert_eq!(spec.tool.annotations.unwrap().read_only_hint, Some(true));
        let schema = Value::Object((*spec.tool.input_schema).clone());
        assert_eq!(schema["additionalProperties"], false);
        for field in ["source_uid", "duration_ms", "max_events"] {
            assert!(
                schema["required"]
                    .as_array()
                    .unwrap()
                    .contains(&json!(field))
            );
        }
        for field in [
            "destination",
            "path",
            "message",
            "forward",
            "expected_document",
        ] {
            assert!(schema["properties"].get(field).is_none());
        }
    }

    #[test]
    fn chorder_learning_tools_keep_getter_readonly_and_both_document_guards_required() {
        for (name, operation) in [
            ("chorder_learning_get", "chorderLearningGet"),
            ("chorder_learning_set", "chorderLearningSet"),
        ] {
            let spec = McpServer::specs()
                .into_iter()
                .find(|spec| spec.tool.name == name)
                .unwrap();
            assert_eq!(spec.operation, Some(operation));
            let schema = Value::Object((*spec.tool.input_schema).clone());
            for field in ["track", "slot", "expected_instance", "expected_document"] {
                assert!(
                    schema["required"]
                        .as_array()
                        .unwrap()
                        .contains(&json!(field))
                );
            }
            for forbidden in [
                "source",
                "param",
                "path",
                "max_views",
                "budget",
                "kind",
                "plugin",
                "channel",
            ] {
                assert!(schema["properties"].get(forbidden).is_none());
            }
            if name.ends_with("get") {
                assert_eq!(spec.tool.annotations.unwrap().read_only_hint, Some(true));
                assert!(schema["properties"].get("change").is_none());
            } else {
                assert!(
                    schema["required"]
                        .as_array()
                        .unwrap()
                        .contains(&json!("change"))
                );
            }
        }
    }

    #[test]
    fn chorder_chords_tool_requires_all_paging_fields_and_explicit_document() {
        let spec = McpServer::specs()
            .into_iter()
            .find(|spec| spec.tool.name == "chorder_chords_get")
            .unwrap();
        assert_eq!(spec.operation, Some("chorderChordsGet"));
        assert_eq!(
            spec.tool.annotations.as_ref().unwrap().read_only_hint,
            Some(true)
        );
        let schema = Value::Object((*spec.tool.input_schema).clone());
        for field in [
            "track",
            "slot",
            "expected_instance",
            "offset",
            "limit",
            "expected_document",
        ] {
            assert!(
                schema["required"]
                    .as_array()
                    .unwrap()
                    .contains(&json!(field))
            );
        }
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(schema["properties"]["offset"]["maximum"], 127);
        assert_eq!(schema["properties"]["limit"]["maximum"], 16);
        for forbidden in [
            "source", "path", "xml", "token", "change", "kind", "channel", "plugin",
        ] {
            assert!(schema["properties"].get(forbidden).is_none());
        }
    }

    #[test]
    fn named_mutations_require_the_document_token() {
        let specs = McpServer::specs();
        for name in [
            "track_select",
            "serum2_load",
            "serum2_preset_apply",
            "chorder_autofill_set",
            "channel_set",
            "plugin_editor_open",
            "plugin_presets",
            "plugin_action",
            "impact_output_set",
            "pedalboard_slot_swap",
            "insert_chain_state",
            "insert_slot_move",
            "insert_splitter_remove",
            "insert_splitter_add",
            "panner_set",
            "splitter_set",
        ] {
            let tool = &specs.iter().find(|s| s.tool.name == name).unwrap().tool;
            assert!(
                tool.input_schema["required"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("expected_document"))
            );
        }
        let names: std::collections::BTreeSet<_> = specs.iter().map(|s| &s.tool.name).collect();
        assert_eq!(names.len(), specs.len());
        assert_eq!(specs.len(), EXPECTED_TOOL_COUNT);
    }

    #[test]
    fn track_selection_reuses_the_document_bound_bridge_operation() {
        let spec = McpServer::specs()
            .into_iter()
            .find(|spec| spec.tool.name == "track_select")
            .unwrap();
        let schema = &spec.tool.input_schema;
        assert_eq!(schema["required"], json!(["name", "expected_document"]));
        assert_eq!(schema["additionalProperties"], false);
        let args = json!({"name":"Twin 3","exclusive":true,"mediaType":"Music"});
        let selection = Operation::from_parts(spec.operation.unwrap(), args.clone()).unwrap();
        assert_eq!(selection.wire(), ("selectTrack".into(), args));

        let mut request = Request {
            operation: selection,
            expected_document: None,
        };
        assert_eq!(
            crate::runtime::validate_request(&request).unwrap_err().code,
            "missing_precondition"
        );
        request.expected_document = Some("session:1".into());
        crate::runtime::validate_request(&request).unwrap();
    }

    #[test]
    fn button_action_schema_is_the_closed_five_action_set() {
        let specs = McpServer::specs();
        let tool = &specs
            .iter()
            .find(|s| s.tool.name == "plugin_action")
            .unwrap()
            .tool;
        assert_eq!(
            tool.input_schema["$defs"]["PluginButtonAction"]["enum"],
            json!([
                "swap_filter_frequencies",
                "reset_meter_hold",
                "reset_meter_loudness",
                "reset_minimum_reduction",
                "retrigger_scope"
            ])
        );
        assert_eq!(tool.input_schema["additionalProperties"], false);
    }

    #[test]
    fn insert_chain_state_requires_document_and_a_separate_folder_payload() {
        let specs = McpServer::specs();
        let spec = specs
            .iter()
            .find(|spec| spec.tool.name == "insert_chain_state")
            .unwrap();
        assert_eq!(spec.operation, Some("insertChainState"));
        let required = spec.tool.input_schema["required"].as_array().unwrap();
        for field in ["channel", "action", "expected_document"] {
            assert!(required.contains(&json!(field)));
        }
        for field in ["token", "expected_folder"] {
            assert!(!required.contains(&json!(field)));
            assert_eq!(
                spec.tool.input_schema["properties"][field]["type"],
                json!("string")
            );
        }
        assert_eq!(spec.tool.input_schema["additionalProperties"], json!(false));
        for field in [
            "plugin",
            "slot",
            "kind",
            "expected_instance",
            "path",
            "url",
            "bytes",
            "xml",
            "script",
            "bridge_session",
            "document_epoch",
        ] {
            assert!(spec.tool.input_schema["properties"].get(field).is_none());
        }
    }

    #[test]
    fn presets_require_document_instance_and_a_finite_action() {
        let specs = McpServer::specs();
        let preset = specs
            .iter()
            .find(|spec| spec.tool.name == "plugin_presets")
            .unwrap();
        assert_eq!(preset.operation, Some("pluginPresets"));
        let required = preset.tool.input_schema["required"].as_array().unwrap();
        for field in ["plugin", "expected_instance", "action", "expected_document"] {
            assert!(required.contains(&json!(field)));
        }
        assert_eq!(
            preset.tool.input_schema["additionalProperties"],
            json!(false)
        );
        for field in ["path", "url", "bytes", "method"] {
            assert!(preset.tool.input_schema["properties"].get(field).is_none());
        }
    }

    #[test]
    fn formatting_requires_explicit_values_and_has_no_mutation_fields() {
        let specs = McpServer::specs();
        let format = specs
            .iter()
            .find(|spec| spec.tool.name == "plugin_parameter_format")
            .unwrap();
        assert_eq!(format.operation, Some("pluginParameterFormat"));
        let required = format.tool.input_schema["required"].as_array().unwrap();
        for field in ["plugin", "param", "source", "expected_instance", "values"] {
            assert!(required.contains(&json!(field)));
        }
        assert!(!required.contains(&json!("expected_document")));
        assert_eq!(
            format.tool.input_schema["additionalProperties"],
            json!(false)
        );
        for field in ["path", "method", "index", "normalized", "text"] {
            assert!(format.tool.input_schema["properties"].get(field).is_none());
        }
    }

    #[test]
    fn digests_accept_only_target_and_provider_fields() {
        let specs = McpServer::specs();
        let digest = specs
            .iter()
            .find(|spec| spec.tool.name == "plugin_parameter_digest")
            .unwrap();
        assert_eq!(digest.operation, Some("pluginParameterDigest"));
        let required = digest.tool.input_schema["required"].as_array().unwrap();
        for field in ["plugin", "source", "expected_instance"] {
            assert!(required.contains(&json!(field)));
        }
        assert!(!required.contains(&json!("expected_document")));
        assert_eq!(
            digest.tool.input_schema["additionalProperties"],
            json!(false)
        );
        assert_eq!(
            digest.tool.input_schema["$defs"]["ParameterDigestSource"]["enum"],
            json!(["dsp", "controller"])
        );
        let properties = digest.tool.input_schema["properties"].as_object().unwrap();
        let fields: std::collections::BTreeSet<_> = properties.keys().map(String::as_str).collect();
        assert_eq!(
            fields,
            std::collections::BTreeSet::from([
                "channel",
                "slot",
                "kind",
                "track",
                "plugin",
                "source",
                "expected_instance",
                "expected_document"
            ])
        );
    }

    #[test]
    fn model_lists_expose_a_finite_readonly_target_and_catalog_schema() {
        let specs = McpServer::specs();
        let spec = specs
            .iter()
            .find(|spec| spec.tool.name == "plugin_models")
            .unwrap();
        assert_eq!(spec.operation, Some("pluginModels"));
        let schema = &spec.tool.input_schema;
        let required = schema["required"].as_array().unwrap();
        for field in ["plugin", "expected_instance", "catalog"] {
            assert!(required.contains(&json!(field)));
        }
        assert!(!required.contains(&json!("expected_document")));
        assert_eq!(schema["additionalProperties"], json!(false));
        assert_eq!(
            schema["$defs"]["ModelCatalog"]["enum"],
            json!(["categories", "effects"])
        );
        assert_eq!(schema["properties"]["offset"]["maximum"], 4096);
        assert_eq!(schema["properties"]["limit"]["minimum"], 1);
        assert_eq!(schema["properties"]["limit"]["maximum"], 32);
        let fields: std::collections::BTreeSet<_> = schema["properties"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            fields,
            std::collections::BTreeSet::from([
                "channel",
                "slot",
                "kind",
                "track",
                "plugin",
                "expected_instance",
                "catalog",
                "offset",
                "limit",
                "expected_document"
            ])
        );
    }

    #[test]
    fn splitter_named_tools_match_the_finite_operations() {
        let specs = McpServer::specs();
        let read = specs
            .iter()
            .find(|s| s.tool.name == "splitters_list")
            .unwrap();
        let write = specs
            .iter()
            .find(|s| s.tool.name == "splitter_set")
            .unwrap();
        assert_eq!(read.operation, Some("splitters"));
        assert_eq!(write.operation, Some("setSplitter"));
        assert_eq!(read.tool.input_schema["required"], json!(["channel"]));
        assert!(
            read.tool.input_schema["properties"]
                .get("expected_document")
                .is_none()
        );
        for name in [
            "channel",
            "node",
            "label",
            "param",
            "value",
            "expected_document",
        ] {
            assert!(
                write.tool.input_schema["required"]
                    .as_array()
                    .unwrap()
                    .contains(&json!(name))
            );
        }
    }

    #[test]
    fn midi_program_part_schema_exposes_only_typed_bank_program_and_identity() {
        let spec = McpServer::specs()
            .into_iter()
            .find(|entry| entry.tool.name == "midi_program_part_add")
            .unwrap();
        assert_eq!(spec.operation, Some("midiProgramPartAdd"));
        let schema = spec.tool.input_schema;
        let fields = [
            "bank_lsb",
            "bank_msb",
            "expected_document",
            "expected_instance",
            "program",
            "track",
        ];
        assert_eq!(
            schema["properties"].as_object().unwrap().len(),
            fields.len() + 1
        );
        for field in fields {
            assert!(
                schema["required"]
                    .as_array()
                    .unwrap()
                    .contains(&json!(field))
            );
        }
        for field in ["bank_lsb", "bank_msb", "program"] {
            let value = &schema["properties"][field];
            assert_eq!(value["minimum"], 0);
            assert_eq!(value["maximum"], 127);
        }
        assert!(schema["properties"].get("instrument").is_some());
        assert!(
            !schema["required"]
                .as_array()
                .unwrap()
                .contains(&json!("instrument"))
        );
        assert_eq!(
            schema["$defs"]["MidiProgramInstrument"]["enum"],
            json!(["one", "twin_2", "twin_3"])
        );
        assert_eq!(schema["additionalProperties"], false);
    }

    #[test]
    fn inspection_description_covers_exact_multi_readonly_metadata_without_expanding_schema() {
        let specs = McpServer::specs();
        assert_eq!(specs.len(), EXPECTED_TOOL_COUNT);
        let spec = specs
            .iter()
            .find(|s| s.tool.name == "plugin_inspect")
            .unwrap();
        assert_eq!(spec.operation, Some("pluginInspect"));
        let description = spec.tool.description.as_deref().unwrap();
        for phrase in [
            "exact Mustang Native and Rumble Native classes",
            "exact Multi Instrument AudioSynth identity",
            "exact Level Meter AudioEffect identity",
            "exact Pedalboard AudioEffect identity",
            "provider may return no controller",
            "read-only metadata may be partial",
            "no model catalog or mutation capability",
        ] {
            assert!(description.contains(phrase), "{phrase}");
        }
        assert!(spec.tool.annotations.is_none());
        let schema = Value::Object((*spec.tool.input_schema).clone());
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(
            schema["$defs"]["NodeScope"]["enum"],
            json!([
                "target",
                "graph",
                "plugin_controller",
                "cv_ports",
                "cv_port_parameters",
                "cv_preset_target",
                "impact_outputs"
            ])
        );
        for field in [
            "expected_document",
            "expected_instance",
            "kind",
            "track",
            "plugin",
        ] {
            assert!(schema["properties"].get(field).is_some(), "{field}");
        }
        for field in ["value", "eval", "script", "action", "path"] {
            assert!(schema["properties"].get(field).is_none(), "{field}");
        }
    }

    #[test]
    fn cv_ports_schema_keeps_its_conditional_document_and_exact_target_gate() {
        let spec = McpServer::specs()
            .into_iter()
            .find(|spec| spec.tool.name == "plugin_inspect")
            .unwrap();
        let schema = Value::Object((*spec.tool.input_schema).clone());
        let rule = &schema["allOf"][0];
        assert_eq!(
            rule["if"]["properties"]["node_scope"]["enum"],
            json!(["cv_ports", "cv_port_parameters", "cv_preset_target"])
        );
        for field in [
            "expected_document",
            "expected_instance",
            "kind",
            "track",
            "plugin",
            "source",
            "include_nodes",
        ] {
            assert!(
                rule["then"]["required"]
                    .as_array()
                    .unwrap()
                    .contains(&json!(field)),
                "{field}"
            );
        }
        assert_eq!(
            rule["then"]["properties"]["include_context"]["const"],
            false
        );
        assert_eq!(
            rule["then"]["properties"]["plugin"]["const"],
            "CV Instrument"
        );
        assert_eq!(McpServer::specs().len(), EXPECTED_TOOL_COUNT);
    }

    #[test]
    fn native_queries_offer_an_optional_document_guard_for_pagination() {
        let specs = McpServer::specs();
        for name in [
            "plugin_parameters",
            "plugin_inspect",
            "plugin_parameter_choices",
            "plugin_parameter_details",
            "plugin_parameter_format",
            "plugin_parameter_digest",
        ] {
            let spec = specs.iter().find(|s| s.tool.name == name).unwrap();
            if name == "plugin_inspect" {
                assert_eq!(
                    spec.tool.input_schema["properties"]["include_context"]["type"],
                    "boolean"
                );
                assert!(
                    !spec
                        .tool
                        .input_schema
                        .get("required")
                        .and_then(Value::as_array)
                        .is_some_and(|fields| fields.contains(&json!("include_context")))
                );
            }
            assert!(
                spec.tool.input_schema["properties"]
                    .get("expected_document")
                    .is_some()
            );
            assert!(
                !spec
                    .tool
                    .input_schema
                    .get("required")
                    .and_then(Value::as_array)
                    .is_some_and(|r| r.contains(&json!("expected_document")))
            );
        }
    }

    #[test]
    fn chord_note_extends_existing_tool_without_new_backend_or_tool_version() {
        let specs = McpServer::specs();
        assert_eq!(specs.len(), EXPECTED_TOOL_COUNT);
        let spec = specs
            .iter()
            .find(|s| s.tool.name == "chorder_learning_set")
            .unwrap();
        assert_eq!(spec.operation, Some("chorderLearningSet"));
        assert!(spec.tool.annotations.is_none());
        let schema = Value::Object((*spec.tool.input_schema).clone());
        assert_eq!(schema["additionalProperties"], false);
        let variants = schema["$defs"]["ChorderLearningChange"]["oneOf"]
            .as_array()
            .unwrap();
        assert!(
            variants
                .iter()
                .any(|v| v["properties"]["kind"]["const"] == "chord_note")
        );
        assert_eq!(env!("CARGO_PKG_VERSION"), "0.1.0");
    }
}

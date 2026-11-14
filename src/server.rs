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
    operation::{self, Operation, Request, schema},
    runtime::RuntimeHandle,
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
            named::<Empty>(
                "channels_list",
                "channels",
                "Read mixer channels and their actual state. Track and audio channel names may differ.",
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
            named::<TrackTask>(
                "tracks_edit",
                "trackTask",
                "Run finite EditTask operations for parts, routing, instruments, events and arranger sections. Results are per item and batches are not atomic.",
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
                "Inspect a live plugin's available parameter enumeration interfaces without arbitrary script execution. With include_context=true, a bound Music instrument/note-effect target and explicit source add strict before/after fixture observations; these are not an atomic snapshot and cannot include nodes. With include_nodes=true, node_scope selects target, graph, or the declared plugin_controller. The controller scope requires an expected plugin and expected_instance and admits only the exact Mustang Native and Rumble Native classes or the exact Multi Instrument AudioSynth identity or exact Level Meter AudioEffect identity. The alias provider may return no controller; its bounded read-only metadata may be partial and establishes no model catalog or mutation capability.",
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
                "Run a fixed native button action on the expected instance. swap_filter_frequencies supports only the exact Expander, Gate and Compressor classes. reset_meter_hold and reset_meter_loudness support only the exact Level Meter class. A pulse or its own Toggle reset never verifies the separate effect. Meter reset effects require an independent non-neutral accumulator witness, which is not currently exposed. Never retry an unknown outcome.",
                true,
            ),
            named::<PluginState>(
                "plugin_state",
                "pluginState",
                "Capture, restore or release a bounded in-memory plugin state, or inspect_rumble_serialization for a read-only bounded Rumble structure summary without a token. Requires the matching native extension. Inspect first and provide expected_instance. Capture returns an opaque token bound to this service and document. Restore accepts only an issued token and requires independent parameter/model readback; a successful native load is applied_unverified. No preset file is written or imported.",
                true,
            ),
            named::<InsertChainState>(
                "insert_chain_state",
                "insertChainState",
                "Capture, restore or release the whole Inserts folder of one exact mixer channel with the matching native extension. Every action requires expected_document. Capture issues a private opaque token; restore and release require that same token. Optional expected_folder pins its native folder UUID. A successful load remains applied_unverified and requires independent graph and per-part parameter/model readback. No caller file path, URL, bytes or plugin slot is accepted.",
                true,
            ),
            named::<PluginPresets>(
                "plugin_presets",
                "pluginPresets",
                "List, apply or release presets from the expected native plugin slot. List issues a bounded catalog token; follow next with the same token, document and instance. Apply accepts only an index returned by that catalog and remains applied_unverified until independent parameter/model readback. Capture plugin_state before applying when restoration is needed. No caller-supplied preset path, URL or state data is accepted; excluded preset sources remain explicit.",
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
                "Set exactly one raw, normalized, display-text or exact finite f64 float_bits value on the expected plugin instance. An explicit source uses native access and type checks; float_bits requires it and preserves the 16 hexadecimal digits from native_value.data.bits_hex. Readback determines verification. Optional edit_lifecycle=native_user invokes a finite Chorder Auto Fill raw 0/1 pilot with explicit dsp and expected_instance. Editing phase and live readback do not prove persistence.",
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
            Request {
                operation: Operation::from_parts(name, Value::Object(args))
                    .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?,
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
    use super::*;

    #[test]
    fn external_menu_schema_is_closed_and_completion_sensitive() {
        assert_eq!(McpServer::specs().len(), 46);
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
            "chorder_autofill_set",
            "channel_set",
            "plugin_editor_open",
            "plugin_presets",
            "plugin_action",
            "insert_chain_state",
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
        assert_eq!(specs.len(), 46);
    }

    #[test]
    fn button_action_schema_is_the_closed_three_action_set() {
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
                "reset_meter_loudness"
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
    fn inspection_description_covers_exact_multi_readonly_metadata_without_expanding_schema() {
        let specs = McpServer::specs();
        assert_eq!(specs.len(), 46);
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
            json!(["target", "graph", "plugin_controller"])
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
        assert_eq!(specs.len(), 46);
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

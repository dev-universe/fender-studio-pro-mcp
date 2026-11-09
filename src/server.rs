use rmcp::{
    ErrorData, RoleServer, ServerHandler,
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, Implementation, ListToolsResult,
        PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool,
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
                "Read named parameters, or enumerate a native page when names is empty. Follow next_offset until null with the same expected_document, expected_instance and source. complete means one page covers its current provider. Native enumeration requires the matching optional extension; metadata fallback is partial.",
                false,
            ),
            named::<PluginParams>(
                "plugin_inspect",
                "pluginInspect",
                "Inspect a live plugin's available parameter enumeration interfaces without arbitrary script execution.",
                false,
            ),
            named::<PluginChoices>(
                "plugin_parameter_choices",
                "pluginChoices",
                "Read a native list parameter's index/label choices without changing its selection. Requires the matching native extension and a controller parameter; open the External Effect editor first. Follow next_offset with expected_document and expected_instance. Empty hardware port lists remain explicit.",
                false,
            ),
            named::<PluginParameterDetails>(
                "plugin_parameter_details",
                "pluginParameterDetails",
                "Read native parameter precision, state flags and a bounded original-parameter chain. include_owner=true also observes bounded controller metadata without invoking methods or granting mutation access. Requires an explicit dsp or controller source and expected_instance. List interface presence and precision do not imply a choice count or selection index.",
                false,
            ),
            named::<PluginParameterFormat>(
                "plugin_parameter_format",
                "pluginParameterFormat",
                "Read the native display text for 1..32 explicitly supplied finite raw values without selecting or changing them. Requires an expected plugin, expected_instance and an explicit dsp or controller source. The native parameter range and numeric type further constrain values. Returned labels do not establish a complete model choice list or count.",
                false,
            ),
            named::<PluginParameterDigest>(
                "plugin_parameter_digest",
                "pluginParameterDigest",
                "Read fresh native schema and value digests for the expected plugin's exposed parameter projection. Requires expected_instance and an explicit dsp or controller source. Native scan errors return no digest. Compare only digests with matching version, source, instance and scope. This is a sequential observation, not an atomic snapshot or complete plugin state; opaque contents remain unverified.",
                false,
            ),
            named::<PluginState>(
                "plugin_state",
                "pluginState",
                "Capture, restore or release a bounded in-memory plugin state using the matching native extension. Inspect first and provide expected_instance. Capture returns an opaque token bound to this service and document. Restore accepts only an issued token and requires independent parameter/model readback; a successful native load is applied_unverified. No preset file is written or imported.",
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
            named::<PluginSet>(
                "plugin_parameter_set",
                "setPluginParam",
                "Set exactly one raw, normalized, display-text or exact finite f64 float_bits value on the expected plugin instance. An explicit source uses native access and type checks; float_bits requires it and preserves the 16 hexadecimal digits from native_value.data.bits_hex. Readback determines verification.",
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
        || matches!(
            operation,
            "pluginParams"
                | "pluginInspect"
                | "pluginChoices"
                | "pluginParameterDetails"
                | "pluginParameterFormat"
                | "pluginParameterDigest"
        )
    {
        schema["properties"]["expected_document"] = json!({"type":"string","description":"Document token returned by host_status. Refuses stale document intents."});
    }
    if mutation {
        let object = schema.as_object_mut().unwrap();
        object
            .entry("required")
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .unwrap()
            .push(json!("expected_document"));
    }
    ToolSpec {
        operation: Some(operation),
        tool: Tool::new(name, description, schema.as_object().unwrap().clone()),
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
        if !operation.operation.is_read_only() && operation.expected_document.is_none() {
            return Ok(CallToolResult::structured_error(json!({"outcome":"not_applied","error":{"code":"missing_precondition","message":"Read host_status and supply expected_document for mutations"}})).into());
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
    fn named_mutations_require_the_document_token() {
        let specs = McpServer::specs();
        for name in [
            "channel_set",
            "plugin_editor_open",
            "plugin_presets",
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
        assert_eq!(specs.len(), 36);
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
}

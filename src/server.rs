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
            named::<PluginEditor>(
                "plugin_editor_open",
                "pluginEditorOpen",
                "Request the expected plugin editor to open. The acknowledgment does not verify window visibility.",
                true,
            ),
            named::<PluginSet>(
                "plugin_parameter_set",
                "setPluginParam",
                "Set exactly one raw, normalized or display-text value on the expected plugin instance. Readback determines verification.",
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
            "pluginParams" | "pluginInspect" | "pluginChoices"
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

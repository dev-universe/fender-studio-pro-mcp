use std::{
    sync::mpsc::{self, SyncSender},
    thread,
    time::Instant,
};

use serde_json::{Value, json};
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::{
    bridge::{Bridge, OwnerLock, WireResponse},
    config::Config,
    midi::MidiWakeup,
    model::{HostContext, OperationError, Outcome, Receipt},
    operation::{NodeScope, Operation, PluginParams, Request},
};

mod chorder_chords;
mod chorder_learning;
mod cv_port_structure;
mod cv_preset_apply;
mod external_menu;
mod midi_program_part;
mod plugin_parameter_readback;
mod rumble_named_parameters;
mod rumble_parameters;
mod rumble_processor_state;
mod rumble_serialization;
mod serum_preset_apply;

#[derive(Clone)]
pub struct RuntimeHandle {
    sender: SyncSender<Envelope>,
}

struct Envelope {
    request: Request,
    reply: oneshot::Sender<Result<Receipt, OperationError>>,
}

impl RuntimeHandle {
    pub fn start(config: Config) -> Result<Self, OperationError> {
        let (sender, receiver) = mpsc::sync_channel::<Envelope>(16);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        thread::Builder::new()
            .name("studio-pro-owner".into())
            .spawn(move || {
                let setup = (|| {
                    let lock = OwnerLock::acquire(&config)?;
                    let midi = MidiWakeup::new(&config.profile().to_string_lossy())?;
                    let bridge = Bridge::new(&config)?;
                    Ok::<_, OperationError>((lock, midi, bridge))
                })();
                let (_lock, mut midi, mut bridge) = match setup {
                    Ok(owner) => {
                        let _ = ready_tx.send(Ok(()));
                        owner
                    }
                    Err(error) => {
                        let _ = ready_tx.send(Err(error));
                        return;
                    }
                };
                let mut local_catalog = None;
                while let Ok(envelope) = receiver.recv() {
                    if envelope.reply.is_closed() {
                        continue;
                    }
                    let result =
                        execute(&mut bridge, &mut midi, &mut local_catalog, envelope.request);
                    let _ = envelope.reply.send(result);
                }
            })
            .map_err(|e| OperationError::new("owner_start", e.to_string()))?;
        ready_rx.recv().map_err(|_| {
            OperationError::new("owner_start", "Owner thread ended during startup")
        })??;
        Ok(Self { sender })
    }

    pub async fn execute(&self, request: Request) -> Result<Receipt, OperationError> {
        validate_request(&request)?;
        let (reply, result) = oneshot::channel();
        self.sender
            .try_send(Envelope { request, reply })
            .map_err(|error| match error {
                mpsc::TrySendError::Full(_) => {
                    OperationError::new("queue_full", "The bounded DAW operation queue is full")
                }
                mpsc::TrySendError::Disconnected(_) => {
                    OperationError::new("owner_stopped", "The DAW owner stopped")
                }
            })?;
        result.await.map_err(|_| {
            OperationError::new(
                "outcome_unknown",
                "The owner stopped before returning the operation result",
            )
        })?
    }
}

pub(crate) fn validate_request(request: &Request) -> Result<(), OperationError> {
    request.operation.validate()?;
    require_learning_document(request)
}

fn require_learning_document(request: &Request) -> Result<(), OperationError> {
    if (!request.operation.is_read_only() || request.operation.requires_explicit_document())
        && request
            .expected_document
            .as_deref()
            .is_none_or(|document| document.trim().is_empty())
    {
        return Err(OperationError::new(
            "missing_precondition",
            "Read host_status and supply expected_document for mutations and context-bound native controls",
        ));
    }
    Ok(())
}

fn execute(
    bridge: &mut Bridge,
    midi: &mut MidiWakeup,
    local_catalog: &mut Option<crate::plugin_catalog::LocalCatalog>,
    request: Request,
) -> Result<Receipt, OperationError> {
    let started = Instant::now();
    let request_id = Uuid::new_v4().to_string();
    let mut operation = request.operation;
    let read_only = operation.is_read_only();
    let context_response = bridge.transact("context", &json!({}), None, None, midi);
    let context_response = match context_response {
        Ok(response) if response.ok => response,
        Ok(response) => {
            return Ok(rejected(
                &operation,
                request_id,
                started,
                host_error(&response),
                None,
            ));
        }
        Err(failure) => {
            return Ok(rejected(
                &operation,
                request_id,
                started,
                failure.error,
                None,
            ));
        }
    };
    let context: HostContext =
        serde_json::from_value(context_response.result.clone()).map_err(|error| {
            bounded_cv_preset_error(
                &operation,
                OperationError::new("invalid_context", error.to_string()),
            )
        })?;
    if request
        .expected_document
        .as_ref()
        .is_some_and(|expected| *expected != context.document)
    {
        return Ok(rejected(
            &operation,
            request_id,
            started,
            OperationError::new(
                "document_changed",
                "The request targets a different document epoch",
            ),
            Some(&context),
        ));
    }
    if matches!(operation, Operation::Context(_)) {
        return Ok(Receipt {
            request_id,
            outcome: Outcome::Observed,
            session: Some(context.session),
            document: Some(context.document),
            result: context_response.result,
            error: None,
            elapsed_ms: started.elapsed().as_millis(),
        });
    }
    if let Operation::Serum2PresetApply(query) = &operation {
        return Ok(serum_preset_apply::execute(
            bridge, midi, query, &context, request_id, started,
        ));
    }
    let (op, mut args) = operation.wire();
    if let Operation::FabfilterPresetApply(query) = &operation {
        let asset = match crate::fabfilter::load_for_apply(query) {
            Ok(asset) => asset,
            Err(reason) => {
                return Ok(rejected(
                    &operation,
                    request_id,
                    started,
                    OperationError::new(reason.code(), reason.message()),
                    Some(&context),
                ));
            }
        };
        let data_hex = asset.wire_data();
        args = json!({"product":query.product,"channel":query.channel,"slot":query.slot,"track":query.track,
            "expected_instance":query.expected_instance,"filename":asset.filename,
            "sha256":asset.sha256,"data_hex":data_hex});
    }
    let staged_program = if let Operation::MidiProgramPartAdd(change) = &operation {
        match bridge.stage_midi_program(&midi_program_part::smf(change)) {
            Ok(staged) => {
                args["fixture"] = json!(staged.basename);
                Some(staged)
            }
            Err(error) => {
                return Ok(rejected(
                    &operation,
                    request_id,
                    started,
                    error,
                    Some(&context),
                ));
            }
        }
    } else {
        None
    };
    let response = bridge.transact(
        &op,
        &args,
        Some(&context.session),
        Some(&context.document),
        midi,
    );
    let mut response = match response {
        Ok(response) => response,
        Err(failure) => {
            let mut result = Value::Null;
            if let (Operation::MidiProgramPartAdd(change), Some(staged)) =
                (&operation, staged_program)
            {
                if failure.dispatched {
                    result = midi_program_part::unverified(change, &staged.basename, "dispatch");
                } else {
                    staged.remove();
                }
            }
            if rumble_named_parameters::requested(&operation) {
                rumble_named_parameters::invalidate(&mut result);
            }
            return Ok(Receipt {
                request_id,
                outcome: if !read_only && failure.dispatched {
                    Outcome::Unknown
                } else {
                    Outcome::NotApplied
                },
                session: Some(context.session),
                document: Some(context.document),
                result,
                error: Some(bounded_cv_preset_error(&operation, failure.error)),
                elapsed_ms: started.elapsed().as_millis(),
            });
        }
    };
    if let Operation::PluginMultiAddMaiTai(change) = &operation {
        return Ok(multi_add_mai_tai_receipt(
            change, &context, response, request_id, started,
        ));
    }
    let nested_splitter_tool = match &operation {
        Operation::InsertSplitterRemoveEmptyNested(_) => {
            Some("insert_splitter_remove_empty_nested")
        }
        Operation::InsertSplitterRemoveOneLaneNested(_) => {
            Some("insert_splitter_remove_one_lane_nested")
        }
        _ => None,
    };
    if let Some(tool) = nested_splitter_tool {
        let (outcome, result, error) =
            nested_splitter_remove_result(tool, &args, &context, &response);
        return Ok(Receipt {
            request_id,
            outcome,
            session: Some(response.session),
            document: Some(response.document),
            result,
            error,
            elapsed_ms: started.elapsed().as_millis(),
        });
    }
    if let (Operation::MidiProgramPartAdd(change), Some(staged)) = (&operation, staged_program) {
        let (outcome, result, error) =
            midi_program_part::classify(change, &staged.basename, &context, &response);
        // If the operation is uncertain, the host callback may still be reading
        // the exclusively created fixture. Leave it in the private mailbox.
        if outcome != Outcome::Unknown {
            staged.remove();
        }
        return Ok(Receipt {
            request_id,
            outcome,
            session: Some(response.session),
            document: Some(response.document),
            result,
            error,
            elapsed_ms: started.elapsed().as_millis(),
        });
    }
    if let Operation::PluginParams(parameters) = &mut operation
        && parameters.names.is_empty()
        && let Some(plugin) = parameters.plugin.as_deref()
        && response.ok
        && response
            .result
            .get("params")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
        && response.result.get("complete") != Some(&Value::Bool(true))
        && response.result.get("parameter_discovery").is_none()
    {
        if local_catalog.is_none() {
            let app = std::env::var_os("FSP_APP")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| {
                    std::path::PathBuf::from("/Applications/Studio Pro 8.app/Contents")
                });
            let app = if app.extension().is_some_and(|extension| extension == "app") {
                app.join("Contents")
            } else {
                app
            };
            *local_catalog = crate::plugin_catalog::discover(&app).ok();
        }
        if let Some(catalog) = local_catalog {
            let names = catalog.candidate_names(plugin, None);
            if !names.is_empty() {
                let mut fallback = args.clone();
                fallback["names"] = json!(names);
                match bridge.transact(
                    &op,
                    &fallback,
                    Some(&context.session),
                    Some(&context.document),
                    midi,
                ) {
                    Ok(found) => response = found,
                    Err(failure) => {
                        return Ok(rejected(
                            &operation,
                            request_id,
                            started,
                            failure.error,
                            Some(&context),
                        ));
                    }
                }
            }
            if response.result.is_object() {
                response.result["parameter_discovery"] = json!({
                    "source":"installed_plugin_metadata", "completeness":"partial",
                    "note":"Candidate names are checked against the live instance. Missing names and selected-focus controls are not complete plugin enumeration."
                });
            }
        } else if response.result.is_object() {
            response.result["parameter_discovery"] =
                json!({"source":"metadata_unavailable","completeness":"unknown"});
        }
    }
    let parameter_readback_requested =
        matches!(&operation, Operation::SetPluginParam(change) if change.readback == Some(true));
    if response.document != context.document
        || (matches!(
            operation,
            Operation::ExternalPortMenu(_) | Operation::ExternalStereoSendSet(_)
        ) || parameter_readback_requested
            || cv_port_structure::requested(&operation)
            || cv_preset_target_requested(&operation)
            || operation.inspects_impact_outputs()
            || operation.inspects_cv_preset_settings()
            || operation.applies_cv_preset_settings()
            || rumble_serialization::requested(&operation)
            || rumble_processor_state::requested(&operation)
            || rumble_parameters::requested(&operation)
            || rumble_named_parameters::requested(&operation))
            && response.session != context.session
    {
        let cv_apply_refused =
            operation.applies_cv_preset_settings() && cv_preset_apply::refused(&response);
        if operation.applies_cv_preset_settings() || parameter_readback_requested {
            response.result = Value::Null;
        } else {
            invalidate_gui_evidence(&operation, &mut response.result);
        }
        return Ok(Receipt {
            request_id,
            outcome: if read_only || cv_apply_refused {
                Outcome::NotApplied
            } else {
                Outcome::Unknown
            },
            session: Some(response.session),
            document: Some(response.document),
            result: response.result,
            error: Some(OperationError::new(
                "document_changed",
                "The document changed before verification",
            )),
            elapsed_ms: started.elapsed().as_millis(),
        });
    }
    if let Operation::EditChorderAutofill(change) = &operation
        && response.ok
        && !valid_gui_edit_receipt(change, &response.result)
    {
        invalidate_gui_edit(&mut response.result, false);
        response.result["error"] = json!({"code":"invalid_gui_edit_receipt", "message":"The GUI edit receipt did not establish the finite receiver contract"});
    }
    if let Operation::ChorderLearningGet(query) = &operation
        && response.ok
        && !chorder_learning::valid_get(query, &response.result)
    {
        chorder_learning::invalidate_get(&mut response.result);
    }
    if let Operation::PluginState(query) = &operation
        && cv_port_structure::requested(&operation)
        && (!response.ok || !cv_port_structure::valid_get(query, &response.result))
    {
        cv_port_structure::invalidate(&mut response.result, true);
    }
    if let Operation::ExternalPortMenu(query) = &operation
        && response.ok
        && !external_menu::valid_get(query, &response.result)
    {
        external_menu::invalidate(&mut response.result, true);
    }
    if let Operation::ChorderChordsGet(query) = &operation
        && response.ok
        && !chorder_chords::valid_get(query, &response.result)
    {
        chorder_chords::invalidate_get(&mut response.result);
    }
    if let Operation::ChorderLearningSet(change) = &operation
        && response.ok
        && !chorder_learning::valid_set(change, &response.result)
    {
        chorder_learning::invalidate_set(&mut response.result, false);
    }
    if let Operation::PluginState(query) = &operation
        && rumble_serialization::requested(&operation)
        && (!response.ok || !rumble_serialization::valid_get(query, &response.result))
    {
        rumble_serialization::invalidate(&mut response.result);
    }
    if let Operation::PluginState(query) = &operation
        && rumble_processor_state::requested(&operation)
        && (!response.ok || !rumble_processor_state::valid_get(query, &response.result))
    {
        rumble_processor_state::invalidate(&mut response.result);
    }
    if let Operation::PluginState(query) = &operation
        && rumble_parameters::requested(&operation)
        && (!response.ok || !rumble_parameters::valid_get(query, &response.result))
    {
        rumble_parameters::invalidate(&mut response.result);
    }
    if let Operation::PluginState(query) = &operation
        && rumble_named_parameters::requested(&operation)
        && (!response.ok || !rumble_named_parameters::valid_get(query, &response.result))
    {
        rumble_named_parameters::invalidate(&mut response.result);
    }
    if let Operation::PluginPresets(args) = &operation
        && operation.applies_cv_preset_settings()
    {
        cv_preset_apply::normalize_result(&mut response, args.index);
    }
    if let Operation::PluginInspect(query) = &operation
        && operation.inspects_impact_outputs()
    {
        if !response.ok {
            response.result = Value::Null;
        } else if !valid_impact_outputs(query, &response.result) {
            response.result = json!({"error":"Impact output inspection did not establish the bound 32 output ports"});
        }
    }
    if matches!(operation, Operation::ImpactOutputSet(_))
        && response.ok
        && response.result["native_outcome"] != "unknown"
        && !impact_output_refused(&response.result)
        && !verify(&operation, &response.result)
    {
        response.result = json!({"ok":false,"native_outcome":"unknown", "errors":[{
            "code":"impact_output_invalid_receipt",
            "message":"The Impact output response did not establish the requested state change"
        }]});
    }
    if matches!(operation, Operation::ExternalStereoSendSet(_))
        && (!response.ok || !verify(&operation, &response.result))
    {
        response.result = normalize_stereo_send_set_failure(&response.result);
    }
    let soft_error = response.result.get("error").filter(|v| !v.is_null());
    let mut error = if !response.ok {
        Some(host_error(&response))
    } else {
        soft_error.map(|error| OperationError::new("host_rejected", error.to_string()))
    };
    let mut outcome = if let Some(error) = &error {
        if cv_port_structure::requested(&operation) {
            cv_port_structure::failure_outcome(&response.result)
        } else if matches!(operation, Operation::ExternalPortMenu(_)) {
            external_menu::failure_outcome(&response.result)
        } else if read_only
            || (matches!(operation, Operation::ImpactOutputSet(_))
                && response.ok
                && impact_output_refused(&response.result))
            || [
                "session_changed",
                "document_changed",
                "missing_precondition",
                "context_required",
                "protocol_mismatch",
            ]
            .contains(&error.code.as_str())
        {
            Outcome::NotApplied
        } else {
            Outcome::Unknown
        }
    } else if read_only
        || matches!(operation, Operation::ExternalPortMenu(_))
        || cv_port_structure::requested(&operation)
    {
        Outcome::Observed
    } else {
        let outcome = mutation_outcome(&response.result);
        if outcome == Outcome::AppliedUnverified && verify(&operation, &response.result) {
            Outcome::AppliedVerified
        } else {
            outcome
        }
    };
    if parameter_readback_requested && error.is_none() && response.result.is_object() {
        if outcome == Outcome::AppliedVerified {
            let readback =
                plugin_parameter_readback::observe(&args, &context, &response.result, |query| {
                    thread::sleep(std::time::Duration::from_millis(
                        plugin_parameter_readback::SETTLE_MILLIS,
                    ));
                    bridge
                        .transact(
                            "pluginParameterDetails",
                            &query,
                            Some(&context.session),
                            Some(&context.document),
                            midi,
                        )
                        .ok()
                });
            if readback["status"] != "matched" {
                outcome = Outcome::AppliedUnverified;
            }
            response.result["readback"] = readback;
        } else if outcome == Outcome::AppliedUnverified {
            response.result["readback"] =
                plugin_parameter_readback::unavailable("write_unverified");
        }
    }
    if error.is_none() {
        error = match outcome {
            Outcome::PartiallyApplied => Some(OperationError::new(
                "partial_application",
                "Some host edits failed after other work may have applied; inspect the per-item results",
            )),
            Outcome::Unknown => Some(OperationError::new(
                "outcome_unknown",
                "The host reported failures without proving that no changes applied",
            )),
            Outcome::NotApplied if has_failure(&response.result) => Some(OperationError::new(
                "native_rejected",
                "The native request was rejected before applying a change; inspect the result errors",
            )),
            _ => None,
        };
    }
    if error.is_some() {
        if cv_port_structure::requested(&operation) {
            cv_port_structure::invalidate(&mut response.result, true);
        } else if matches!(operation, Operation::ExternalPortMenu(_)) {
            external_menu::invalidate(&mut response.result, true);
        } else {
            invalidate_gui_evidence(&operation, &mut response.result);
        }
    }
    Ok(Receipt {
        request_id,
        outcome,
        session: Some(response.session),
        document: Some(response.document),
        result: response.result,
        error: error.map(|error| bounded_cv_preset_error(&operation, error)),
        elapsed_ms: started.elapsed().as_millis(),
    })
}

fn multi_add_mai_tai_receipt(
    change: &crate::operation::PluginMultiAddMaiTai,
    context: &HostContext,
    response: WireResponse,
    request_id: String,
    started: Instant,
) -> Receipt {
    let (outcome, result, error) = multi_add_mai_tai_result(change, context, &response);
    Receipt {
        request_id,
        outcome,
        session: Some(response.session),
        document: Some(response.document),
        result,
        error,
        elapsed_ms: started.elapsed().as_millis(),
    }
}

fn multi_add_mai_tai_result(
    change: &crate::operation::PluginMultiAddMaiTai,
    context: &HostContext,
    response: &WireResponse,
) -> (Outcome, Value, Option<OperationError>) {
    let value = &response.result;
    let bound = response.session == context.session && response.document == context.document;
    let clean = response.ok && response.code.is_none() && response.error.is_none();
    if !response.ok
        && value.is_null()
        && matches!(
            response.code.as_deref(),
            Some(
                "session_changed"
                    | "document_changed"
                    | "missing_precondition"
                    | "context_required"
                    | "protocol_mismatch"
            )
        )
    {
        return (Outcome::NotApplied, Value::Null, Some(host_error(response)));
    }
    if bound
        && clean
        && exact_gui_object(
            value,
            &[
                "ok",
                "outcome",
                "native_outcome",
                "dispatch_attempted",
                "error",
            ],
        )
        && value["ok"] == false
        && value["outcome"] == "not_applied"
        && value["native_outcome"] == "not_applied"
        && value["dispatch_attempted"] == false
        && exact_gui_object(&value["error"], &["code", "message"])
        && value["error"]["code"] == "multi_add_mai_tai_refused"
        && let Some(message) = value["error"]["message"].as_str()
        && !message.is_empty()
        && message.encode_utf16().count() <= 512
    {
        return (
            Outcome::NotApplied,
            value.clone(),
            Some(OperationError::new("multi_add_mai_tai_refused", message)),
        );
    }
    let instance = &value["instance"];
    if bound
        && clean
        && exact_gui_object(
            value,
            &[
                "ok",
                "outcome",
                "native_outcome",
                "operation",
                "instance",
                "dispatch_attempted",
                "instrument",
                "graph_attachment_verified",
                "undo_verified",
            ],
        )
        && value["ok"] == true
        && value["outcome"] == "applied_unverified"
        && value["native_outcome"] == "applied_unverified"
        && value["operation"] == "multi_add_mai_tai"
        && value["instrument"] == "Mai Tai"
        && value["dispatch_attempted"] == true
        && value["graph_attachment_verified"] == false
        && value["undo_verified"] == false
        && exact_gui_object(
            instance,
            &["object_uid", "class_uid", "native_class", "object_id"],
        )
        && instance["object_uid"] == change.expected_instance
        && instance["class_uid"] == "B9D410AD-E47D-495E-8AEE-10D2B2987CC7"
        && instance["native_class"] == "AudioSynth"
        && instance["object_id"]
            .as_str()
            .is_some_and(|id| !id.is_empty() && id.encode_utf16().count() <= 512)
    {
        return (Outcome::AppliedUnverified, value.clone(), None);
    }
    let message = "The native Mai Tai insertion could not be confirmed. Read the current Multi Instrument before making another edit.";
    let dispatch_attempted =
        (bound && clean && value["dispatch_attempted"] == true).then_some(true);
    (
        Outcome::Unknown,
        json!({"ok":false,"outcome":"unknown","native_outcome":"unknown","dispatch_attempted":dispatch_attempted,
            "error":{"code":"multi_add_mai_tai_unknown","message":message}}),
        Some(OperationError::new("multi_add_mai_tai_unknown", message)),
    )
}

fn exact_gui_object(value: &Value, fields: &[&str]) -> bool {
    value.as_object().is_some_and(|object| {
        object.len() == fields.len() && fields.iter().all(|field| object.contains_key(*field))
    })
}

fn valid_nested_splitter_remove_receipt(
    tool: &str,
    args: &Value,
    context: &HostContext,
    response: &WireResponse,
) -> bool {
    if response.session != context.session
        || response.document != context.document
        || response.error.is_some()
        || response.code.is_some()
    {
        return false;
    }
    let result = &response.result;
    if result["ok"] == false {
        return exact_gui_object(result, &["ok", "native_outcome", "errors"])
            && matches!(
                result["native_outcome"].as_str(),
                Some("not_applied" | "unknown")
            )
            && result["errors"].as_array().is_some_and(|errors| {
                matches!(errors.as_slice(), [error] if exact_gui_object(error, &["code", "message"])
                && matches!(error["code"].as_str().and_then(|code| code.strip_prefix(tool)),
                    Some("_refused" | "_failed" | "_context_changed"))
                && error["message"].as_str().is_some_and(|s| {
                    !s.is_empty() && s.encode_utf16().count() <= 512
                }))
            });
    }
    let mut identifiers = vec![
        "expected_instance",
        "expected_graph",
        "splitter_node",
        "merger_node",
        "predecessor_slot_node",
        "successor_merger_node",
    ];
    if tool == "insert_splitter_remove_one_lane_nested" {
        identifiers.push("occupied_slot_node");
    }
    let mut fields = vec![
        "ok",
        "native_outcome",
        "outcome",
        "operation",
        "verified",
        "instance",
    ];
    fields.extend_from_slice(&identifiers);
    let instance = &result["instance"];
    exact_gui_object(result, &fields)
        && result["ok"] == true
        && result["outcome"] == "applied_unverified"
        && result["native_outcome"] == "applied_unverified"
        && result["operation"] == tool
        && result["verified"] == false
        && identifiers.iter().all(|&key| result[key] == args[key])
        && exact_gui_object(
            instance,
            &["object_id", "native_class", "class_uid", "object_uid"],
        )
        && instance["native_class"] == "AudioEffect"
        && instance["class_uid"] == "E4D7D911-0608-4B46-ABA3-2E345399A5AC"
        && instance["object_uid"] == args["expected_instance"]
        && instance["object_id"] == "Mixtool"
}

fn nested_splitter_remove_result(
    tool: &str,
    args: &Value,
    context: &HostContext,
    response: &WireResponse,
) -> (Outcome, Value, Option<OperationError>) {
    let result = &response.result;
    if !response.ok
        && result.is_null()
        && response.session == context.session
        && matches!(
            response.code.as_deref(),
            Some(
                "document_changed"
                    | "missing_precondition"
                    | "context_required"
                    | "protocol_mismatch"
            )
        )
        && response
            .error
            .as_ref()
            .and_then(Value::as_str)
            .is_some_and(|s| !s.is_empty() && s.encode_utf16().count() <= 512)
    {
        return (Outcome::NotApplied, Value::Null, Some(host_error(response)));
    }
    if response.ok && valid_nested_splitter_remove_receipt(tool, args, context, response) {
        if result["ok"] == true {
            return (Outcome::AppliedUnverified, result.clone(), None);
        }
        if result["native_outcome"] == "not_applied" {
            return (
                Outcome::NotApplied,
                result.clone(),
                Some(OperationError::new(
                    "native_rejected",
                    if tool == "insert_splitter_remove_empty_nested" {
                        "The exact empty nested Splitter was refused before removal"
                    } else {
                        "The exact one-lane nested Splitter was refused before removal"
                    },
                )),
            );
        }
    }
    let message = "Read the exact insert graph before making another edit";
    (
        Outcome::Unknown,
        json!({"ok":false,"native_outcome":"unknown","errors":[{
            "code": if tool == "insert_splitter_remove_empty_nested" {
                "empty_splitter_remove_unconfirmed"
            } else {
                "one_lane_splitter_remove_unconfirmed"
            },
            "message":message
        }]}),
        Some(OperationError::new("outcome_unknown", message)),
    )
}

fn gui_number(value: &Value) -> Option<f64> {
    if !exact_gui_object(value, &["variant_tag", "data"]) {
        return None;
    }
    let tag = i16::try_from(value["variant_tag"].as_i64()?).ok()?;
    let data = &value["data"];
    match (tag & 15, data["kind"].as_str()?) {
        (1, "integer") if exact_gui_object(data, &["kind", "value"]) => {
            let number = data["value"].as_i64()?;
            (number.unsigned_abs() <= 9_007_199_254_740_991).then_some(number as f64)
        }
        (2, "float") if exact_gui_object(data, &["kind", "value", "bits_hex"]) => {
            let number = data["value"].as_f64()?;
            let text = data["bits_hex"].as_str()?;
            if text.len() != 16
                || !text
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b))
            {
                return None;
            }
            let bits = u64::from_str_radix(text, 16).ok()?;
            (number.is_finite() && number.to_bits() == bits).then_some(number)
        }
        _ => None,
    }
}

fn gui_default_scalar(typed: &Value, scalar: &Value) -> bool {
    if typed.is_null() {
        return scalar.is_null();
    }
    if let Some(number) = gui_number(typed) {
        return scalar.as_f64() == Some(number);
    }
    if !exact_gui_object(typed, &["variant_tag", "data"]) {
        return false;
    }
    let Some(tag) = typed["variant_tag"]
        .as_i64()
        .and_then(|tag| i16::try_from(tag).ok())
    else {
        return false;
    };
    let data = &typed["data"];
    match (tag & 15, data["kind"].as_str()) {
        (0, Some("empty")) => scalar.is_null() && exact_gui_object(data, &["kind"]),
        (1, Some("integer")) => {
            scalar.is_null()
                && exact_gui_object(data, &["kind", "exact_decimal"])
                && data["exact_decimal"].as_str().is_some_and(|s| {
                    s.parse::<i64>().is_ok_and(|n| {
                        n.to_string() == s && n.unsigned_abs() > 9_007_199_254_740_991
                    })
                })
        }
        (2, Some("non_finite_float")) => {
            scalar.is_null()
                && exact_gui_object(data, &["kind", "bits_hex"])
                && data["bits_hex"].as_str().is_some_and(|s| {
                    s.len() == 16
                        && s.bytes()
                            .all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b))
                        && u64::from_str_radix(s, 16)
                            .is_ok_and(|bits| !f64::from_bits(bits).is_finite())
                })
        }
        (3, Some("text")) => {
            exact_gui_object(data, &["kind", "value"])
                && data["value"]
                    .as_str()
                    .is_some_and(|s| s.encode_utf16().count() <= 4096)
                && *scalar == data["value"]
        }
        (4, Some("opaque_object")) => {
            scalar.is_null()
                && exact_gui_object(data, &["kind", "present"])
                && data["present"].is_boolean()
        }
        (kind, Some("unsupported")) => {
            kind > 4
                && scalar.is_null()
                && exact_gui_object(data, &["kind", "tag"])
                && data["tag"] == kind
        }
        _ => false,
    }
}

fn gui_toggle_description(value: &Value) -> bool {
    let fields = [
        "name",
        "title",
        "tag",
        "value",
        "text",
        "min",
        "max",
        "default",
        "normalized",
        "enabled",
        "readOnly",
        "writable",
        "native_kind",
        "native_value",
        "native_min",
        "native_max",
        "native_default",
        "native_normalized",
        "native_support",
        "support",
        "display_parser",
    ];
    if !exact_gui_object(value, &fields)
        || value["name"] != "autofill"
        || value["title"] != "autofill"
        || value["native_kind"] != "Toggle"
        || value["enabled"] != true
        || value["readOnly"] != false
        || value["writable"] != true
        || value["support"]["raw"] != true
        || value["tag"]
            .as_i64()
            .and_then(|tag| i32::try_from(tag).ok())
            .is_none()
        || (!value["text"].is_null()
            && value["text"]
                .as_str()
                .is_none_or(|s| s.encode_utf16().count() > 4096))
        || value["display_parser"]
            .as_str()
            .is_none_or(|s| s.len() > 96)
        || !exact_gui_object(
            &value["native_support"],
            &["raw_number", "normalized", "display_text"],
        )
        || !exact_gui_object(&value["support"], &["raw", "normalized", "text"])
    {
        return false;
    }
    for (native, scalar) in [
        ("native_value", "value"),
        ("native_min", "min"),
        ("native_max", "max"),
    ] {
        let Some(number) = gui_number(&value[native]) else {
            return false;
        };
        if value[scalar]
            .as_f64()
            .is_none_or(|scalar| scalar.to_bits() != number.to_bits())
        {
            return false;
        }
    }
    if !gui_default_scalar(&value["native_default"], &value["default"]) {
        return false;
    }
    for (native, scalar) in [
        ("raw_number", "raw"),
        ("normalized", "normalized"),
        ("display_text", "text"),
    ] {
        if value["native_support"][native].as_bool().is_none()
            || value["native_support"][native] != value["support"][scalar]
        {
            return false;
        }
    }
    let raw = gui_number(&value["native_value"]);
    if !matches!(value["native_value"]["variant_tag"].as_i64(), Some(1 | 2))
        || !matches!(raw, Some(0.0 | 1.0))
        || ["native_value", "native_min", "native_max"]
            .iter()
            .any(|key| {
                value[*key]["data"]["kind"] == "float"
                    && value[*key]["data"]["bits_hex"]
                        != if value[*key]["data"]["value"].as_f64() == Some(1.0) {
                            "3FF0000000000000"
                        } else {
                            "0000000000000000"
                        }
            })
        || gui_number(&value["native_min"]) != Some(0.0)
        || gui_number(&value["native_max"]) != Some(1.0)
        || value["native_value"]["variant_tag"] != value["native_min"]["variant_tag"]
        || value["native_value"]["variant_tag"] != value["native_max"]["variant_tag"]
    {
        return false;
    }
    let normalized = &value["native_normalized"];
    if normalized["kind"] == "not_applicable" {
        return exact_gui_object(normalized, &["kind"]) && value["normalized"].is_null();
    }
    if !matches!(
        normalized["kind"].as_str(),
        Some("float32" | "non_finite_float32" | "invalid_float32")
    ) || !exact_gui_object(normalized, &["kind", "bits_hex"])
    {
        return false;
    }
    let Some(text) = normalized["bits_hex"].as_str() else {
        return false;
    };
    if text.len() != 8
        || !text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b))
    {
        return false;
    }
    let Ok(bits) = u32::from_str_radix(text, 16) else {
        return false;
    };
    let number = f32::from_bits(bits);
    if normalized["kind"] != "float32" {
        return !number.is_finite() && value["normalized"].is_null();
    }
    number.is_finite()
        && value["normalized"]
            .as_f64()
            .is_some_and(|v| v.to_bits() == f64::from(number).to_bits())
}

fn gui_errors(value: &Value) -> bool {
    value.as_array().is_some_and(|errors| {
        !errors.is_empty()
            && errors.len() <= 8
            && errors.iter().all(|error| {
                exact_gui_object(error, &["code", "message"])
                    && error["code"].as_str().is_some_and(|s| {
                        !s.is_empty()
                            && s.len() <= 64
                            && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                    })
                    && error["message"]
                        .as_str()
                        .is_some_and(|s| s.encode_utf16().count() <= 512)
            })
    })
}

fn valid_gui_edit_receipt(change: &crate::operation::ChorderAutofillSet, value: &Value) -> bool {
    if value["ok"] == false {
        let allowed = [
            "ok",
            "outcome",
            "native_outcome",
            "instance",
            "plugin",
            "errors",
            "edit_phase",
            "gui_value_verified",
            "persistence_verified",
            "effects_verified",
            "dsp_propagation_verified",
        ];
        if value
            .as_object()
            .is_none_or(|object| object.keys().any(|key| !allowed.contains(&key.as_str())))
        {
            return false;
        }
        let phase = value.get("edit_phase");
        return matches!(
            value["native_outcome"].as_str(),
            Some("not_applied" | "unknown")
        ) && value
            .get("outcome")
            .is_none_or(|outcome| *outcome == value["native_outcome"])
            && (value["native_outcome"] != "not_applied"
                || phase.is_none()
                || phase == Some(&json!("prepared")))
            && phase.is_none_or(|p| {
                matches!(
                    p.as_str(),
                    Some(
                        "prepared"
                            | "begin_dispatched"
                            | "value_dispatched"
                            | "end_dispatched"
                            | "closed_checked"
                    )
                )
            })
            && [
                "gui_value_verified",
                "persistence_verified",
                "effects_verified",
                "dsp_propagation_verified",
            ]
            .iter()
            .all(|key| value.get(*key).is_none_or(|flag| *flag == false))
            && gui_errors(&value["errors"]);
    }
    let fields = [
        "ok",
        "outcome",
        "native_outcome",
        "operation",
        "instance",
        "binding_source",
        "logical_parameter",
        "receiver",
        "requested_enabled",
        "edit_phase",
        "gui_parameter",
        "gui_value_verified",
        "verification_scope",
        "persistence_verified",
        "effects_verified",
        "dsp_propagation_verified",
        "plugin",
    ];
    if !exact_gui_object(value, &fields)
        || value["ok"] != true
        || value["outcome"] != "applied_unverified"
        || value["native_outcome"] != "applied_unverified"
        || value["operation"] != "edit_chorder_autofill"
        || value["plugin"] != "Chorder"
        || value["binding_source"] != "dsp"
        || value["logical_parameter"] != "chorder.autofill"
        || value["receiver"] != "editor_control"
        || value["requested_enabled"] != change.enabled
        || value["edit_phase"] != "closed_checked"
        || value["verification_scope"] != "gui_receiver_current_value"
        || value["instance"]["object_uid"] != change.expected_instance
        || value["instance"]["class_uid"] != "D2D9B002-12FC-4F1D-AFDA-70336D1DC108"
        || [
            "persistence_verified",
            "effects_verified",
            "dsp_propagation_verified",
        ]
        .iter()
        .any(|key| value[*key] != false)
        || !exact_gui_object(&value["gui_parameter"], &["name", "before", "after"])
        || value["gui_parameter"]["name"] != "autofill"
    {
        return false;
    }
    let before = &value["gui_parameter"]["before"];
    let after = &value["gui_parameter"]["after"];
    if !gui_toggle_description(before)
        || !gui_toggle_description(after)
        || before["native_value"]["variant_tag"] != after["native_value"]["variant_tag"]
    {
        return false;
    }
    if before.as_object().unwrap().iter().any(|(key, v)| {
        ![
            "value",
            "text",
            "normalized",
            "native_value",
            "native_normalized",
        ]
        .contains(&key.as_str())
            && *v != after[key]
    }) {
        return false;
    }
    let desired = if change.enabled { 1.0_f64 } else { 0.0_f64 };
    let exact = gui_number(&after["native_value"])
        .is_some_and(|number| number.to_bits() == desired.to_bits());
    value["gui_value_verified"].as_bool() == Some(exact)
}

fn invalidate_gui_edit(value: &mut Value, preserve_refusal: bool) {
    let errors = value
        .get("errors")
        .filter(|errors| gui_errors(errors))
        .cloned();
    let phase = value
        .get("edit_phase")
        .and_then(Value::as_str)
        .filter(|phase| {
            matches!(
                *phase,
                "prepared"
                    | "begin_dispatched"
                    | "value_dispatched"
                    | "end_dispatched"
                    | "closed_checked"
            )
        })
        .map(str::to_owned);
    let refused = preserve_refusal
        && value["native_outcome"] == "not_applied"
        && phase.as_deref().is_none_or(|p| p == "prepared");
    let outcome = if refused { "not_applied" } else { "unknown" };
    *value = json!({"ok":false,"outcome":outcome,"native_outcome":outcome,"gui_value_verified":false,
        "persistence_verified":false,"effects_verified":false,"dsp_propagation_verified":false});
    if let Some(phase) = phase {
        value["edit_phase"] = json!(phase)
    }
    if let Some(errors) = errors {
        value["errors"] = errors
    }
}

fn cv_preset_target_requested(operation: &Operation) -> bool {
    matches!(operation, Operation::PluginInspect(query) if matches!(query.node_scope, Some(NodeScope::CvPresetTarget)))
}

fn valid_impact_outputs(query: &PluginParams, result: &Value) -> bool {
    let native = &result["native"];
    let instance = &native["instance"];
    let report = &native["node_report"];
    result["plugin"] == "Impact"
        && native["ok"] == true
        && native["outcome"] == "observed"
        && native["source"] == "dsp"
        && instance["object_id"] == "Impact"
        && instance["native_class"] == "AudioSynth"
        && instance["class_uid"] == "3713E26C-2FCA-4024-9F25-17E9D2BE2B9B"
        && query
            .expected_instance
            .as_deref()
            .is_some_and(|expected| instance["object_uid"] == expected)
        && report["scope"] == "impact_outputs"
        && report["value_semantics"] == "sequential_snapshot"
        && report["complete"] == true
        && report["output_count"] == 32
        && report["outputs"].as_array().is_some_and(|rows| {
            rows.len() == 32
                && rows.iter().enumerate().all(|(index, row)| {
                    row.as_object().is_some_and(|fields| fields.len() == 4)
                        && row["index"].as_u64() == Some(index as u64)
                        && row["name"].as_str().is_some_and(|name| {
                            !name.trim().is_empty()
                                && name.encode_utf16().count() <= 128
                                && !name.chars().any(char::is_control)
                        })
                        && row["requested"].is_boolean()
                        && row["acknowledged"].is_boolean()
                })
        })
}
fn invalidate_gui_evidence(operation: &Operation, result: &mut Value) {
    if cv_preset_target_requested(operation)
        || operation.inspects_cv_preset_settings()
        || operation.inspects_impact_outputs()
    {
        *result = Value::Null;
        return;
    }
    if cv_port_structure::requested(operation) {
        cv_port_structure::invalidate(result, false);
        return;
    }
    if rumble_named_parameters::requested(operation) {
        rumble_named_parameters::invalidate(result);
        return;
    }
    if rumble_parameters::requested(operation) {
        rumble_parameters::invalidate(result);
        return;
    }
    if rumble_processor_state::requested(operation) {
        rumble_processor_state::invalidate(result);
        return;
    }
    if rumble_serialization::requested(operation) {
        rumble_serialization::invalidate(result);
        return;
    }
    if matches!(operation, Operation::ExternalPortMenu(_)) {
        external_menu::invalidate(result, false);
        return;
    }
    if matches!(operation, Operation::ChorderChordsGet(_)) {
        chorder_chords::invalidate_get(result);
        return;
    }
    if matches!(operation, Operation::ChorderLearningGet(_)) {
        chorder_learning::invalidate_get(result);
        return;
    }
    if matches!(operation, Operation::ChorderLearningSet(_)) {
        chorder_learning::invalidate_set(result, true);
        return;
    }
    if matches!(operation, Operation::EditChorderAutofill(_)) {
        invalidate_gui_edit(result, true);
        return;
    }
    if matches!(operation, Operation::PluginParameterDetails(details) if details.compare_gui) {
        *result = json!({
            "ok":false,"outcome":"not_applied","native_outcome":"not_applied","mutation_capability":false,
            "gui_comparison":{
                "status":"unresolved","scope":"exact_device_named_control_candidate",
                "parameter_name":"chorder.autofill","semantic_binding":"named_candidate_only",
                "mutation_capability":false,"reason":"stale_association",
                "coverage":{"complete":false,"candidate_count":0,"visited_views":0,"max_views":128,"max_depth":32}
            }
        });
    }
}

fn mutation_outcome(value: &Value) -> Outcome {
    if let Some(outcome) = value.get("native_outcome").and_then(Value::as_str) {
        match outcome {
            "not_applied" => return Outcome::NotApplied,
            "unknown" => return Outcome::Unknown,
            _ => {}
        }
    }
    if has_failure(value) {
        if has_applied_evidence(value)
            || value
                .get("results")
                .and_then(Value::as_array)
                .is_some_and(|results| results.iter().any(|result| !has_failure(result)))
        {
            Outcome::PartiallyApplied
        } else {
            Outcome::Unknown
        }
    } else if value.get("executed") == Some(&Value::Bool(false)) {
        Outcome::NotApplied
    } else {
        Outcome::AppliedUnverified
    }
}

fn has_failure(value: &Value) -> bool {
    value.get("error").is_some_and(|error| !error.is_null())
        || value
            .get("errors")
            .and_then(Value::as_array)
            .is_some_and(|errors| !errors.is_empty())
        || value
            .get("results")
            .and_then(Value::as_array)
            .is_some_and(|results| results.iter().any(has_failure))
}

fn has_applied_evidence(value: &Value) -> bool {
    value
        .get("added")
        .and_then(Value::as_u64)
        .is_some_and(|count| count > 0)
        || value
            .get("notes")
            .and_then(Value::as_u64)
            .is_some_and(|count| count > 0)
        || value.get("part").is_some_and(Value::is_object)
        || value.get("done").is_some_and(|done| {
            done.as_bool() == Some(true) || done.as_array().is_some_and(|items| !items.is_empty())
        })
        || ["results", "applied"].iter().any(|key| {
            value
                .get(key)
                .and_then(Value::as_array)
                .is_some_and(|results| results.iter().any(has_applied_evidence))
        })
}

const STEREO_SEND_CHANGE_PHASES: &[&str] = &[
    "bridge_precondition",
    "bridge_dispatch",
    "request",
    "pre_fixture",
    "pre_attachment",
    "controller_baseline",
    "native_wiring",
    "native_reply",
    "native_admission",
    "native_allocator",
    "source_parameter",
    "builder_factory",
    "construct",
    "build",
    "source_profile",
    "check_only",
    "source_rebind",
    "select",
    "cleanup",
    "typed_destination",
    "fresh_destination_menu",
    "native_post_attachment",
    "native_publish",
    "native_reply_context",
    "post_native_attachment",
    "post_native_fixture",
    "independent_controller",
    "independent_menu",
    "independent_attachment",
    "independent_fixture",
    "bridge_publish",
    "receipt",
];

const STEREO_SEND_CHANGE_FLAGS: [&str; 5] = [
    "dispatch_attempted",
    "menu_callbacks_began",
    "select_entered",
    "select_returned",
    "cleanup_proven",
];

fn stereo_send_set_flags(trace: &Value) -> Option<[Option<bool>; 5]> {
    if trace.as_object()?.len() != 7 || trace["version"] != "stereo_send_v1" {
        return None;
    }
    let failure = trace.get("failure_phase")?;
    if !failure.is_null()
        && !failure
            .as_str()
            .is_some_and(|phase| STEREO_SEND_CHANGE_PHASES.contains(&phase))
    {
        return None;
    }
    let mut flags = [None; 5];
    for (index, key) in STEREO_SEND_CHANGE_FLAGS.iter().enumerate() {
        let value = trace.get(key)?;
        if !value.is_null() {
            flags[index] = Some(value.as_bool()?);
        }
    }
    let [dispatch, callbacks, entered, returned, cleanup] = flags;
    let native = [callbacks, entered, returned, cleanup];
    if match dispatch {
        Some(false) => !native.iter().all(|flag| *flag == Some(false)),
        None => !native.iter().all(Option::is_none),
        Some(true) => !native.iter().all(Option::is_none) && !native.iter().all(Option::is_some),
    } || (entered == Some(true) && callbacks != Some(true))
        || (returned == Some(true) && entered != Some(true))
        || (cleanup == Some(true) && callbacks != Some(true))
    {
        return None;
    }
    Some(flags)
}

fn normalize_stereo_send_set_failure(result: &Value) -> Value {
    let mut diagnostic = result["diagnostic"].clone();
    let flags = stereo_send_set_flags(&diagnostic);
    if flags.is_none() {
        diagnostic = json!({
            "version":"stereo_send_v1", "failure_phase":"receipt", "dispatch_attempted":null,
            "menu_callbacks_began":null, "select_entered":null, "select_returned":null, "cleanup_proven":null,
        });
    } else if diagnostic["failure_phase"].is_null() {
        diagnostic["failure_phase"] = json!("receipt");
    }
    let not_applied = result["ok"] == false
        && result["native_outcome"] == "not_applied"
        && result["diagnostic"]["failure_phase"].is_string()
        && flags.is_some_and(|[dispatch, callbacks, entered, returned, cleanup]| {
            dispatch.is_some() && [callbacks, entered, returned, cleanup] == [Some(false); 4]
        });
    json!({
        "ok":false, "native_outcome":if not_applied { "not_applied" } else { "unknown" },
        "diagnostic":diagnostic,
        "errors":[{"code":"external_stereo_send_unconfirmed","message":"The logical Stereo Send change could not be verified"}],
    })
}

fn verify(operation: &Operation, result: &Value) -> bool {
    match operation {
        Operation::ExternalStereoSendSet(change) => {
            let (old, new) = change.destination.display();
            let before = &result["before"];
            let after = &result["after"];
            let mut expected_after = before.clone();
            let object = match expected_after.as_object_mut() {
                Some(object) => object,
                None => return false,
            };
            object.insert("text".into(), json!(new));
            result["ok"] == true
                && stereo_send_set_flags(&result["diagnostic"]) == Some([Some(true); 5])
                && result["diagnostic"]["failure_phase"].is_null()
                && result["operation"] == "external_stereo_send_set"
                && result["native_outcome"] == "applied_unverified"
                && result["channel"] == change.channel
                && result["slot"] == change.slot
                && result["expected_current"] == change.expected_current.name()
                && result["destination"] == change.destination.name()
                && result["assigned"]
                    == matches!(
                        change.destination,
                        crate::operation::ExternalStereoSendDestination::Main
                    )
                && result["persistence_verified"] == false
                && result["audio_verified"] == false
                && result["selection_calls"] == 1
                && result["controller_rows"] == 24
                && result["fixture_after_matches"] == true
                && result["native_checkmark_verified"] == true
                && result["atomic_snapshot"] == false
                && result["instance"]["class_uid"] == "889DD9F8-14F0-44A5-BCEE-0F2B157FC5D4"
                && external_menu::matches_object_id(
                    &result["instance"]["object_id"],
                    "External Effect Stereo",
                )
                && result["instance"]["native_class"] == "AudioEffect"
                && result["instance"]["object_uid"] == change.expected_instance
                && before["name"] == "sendPort"
                && before["tag"] == 102
                && before["native_kind"] == "List"
                && before["value"] == -1
                && before["native_value"]
                    == json!({"variant_tag":1,"data":{"kind":"integer","value":-1}})
                && before["normalized"] == 1
                && before["native_normalized"] == json!({"kind":"float32","bits_hex":"3F800000"})
                && before["text"] == old
                && after == &expected_after
        }
        Operation::ImpactOutputSet(change) => {
            let row = |active| {
                json!({"index":change.output_index,"name":change.output_name,
                "requested":active,"acknowledged":active})
            };
            result["ok"] == true
                && result["operation"] == "impact_output_set"
                && result["outcome"] == "applied_verified"
                && result["native_outcome"] == "applied_verified"
                && result["plugin"] == "Impact"
                && result["instance"]["object_uid"] == change.expected_instance
                && result["instance"]["class_uid"] == "3713E26C-2FCA-4024-9F25-17E9D2BE2B9B"
                && result["instance"]["native_class"] == "AudioSynth"
                && result["output_count"] == 32
                && result["before"] == row(change.expected_active)
                && result["after"] == row(change.active)
        }
        Operation::SetChannel(change) => equivalent(&change.value, &result["after"]),
        Operation::SetPanner(change) => equivalent(&json!(change.value), &result["after"]["value"]),
        Operation::SetSplitter(change) => {
            equivalent(&json!(change.value), &result["after"]["value"])
        }
        Operation::SetInsertBypass(change) => equivalent(&json!(change.bypassed), &result["after"]),
        Operation::SetPluginParam(change) => {
            if change.edit_lifecycle.is_some() {
                return result.get("native_verified").and_then(Value::as_bool) == Some(true);
            }
            if let Some(verified) = result.get("native_verified").and_then(Value::as_bool) {
                return verified;
            }
            if let Some(value) = change.value {
                equivalent(&json!(value), &result["after"]["value"])
            } else if let Some(value) = change.normalized {
                equivalent(&json!(value), &result["after"]["normalized"])
            } else if let Some(text) = &change.text {
                result["after"]["text"].as_str() == Some(text.as_str())
            } else {
                false
            }
        }
        Operation::SetChannelLabel(change) => {
            result["after"].as_str() == Some(change.name.as_str())
        }
        Operation::SetChannelColor(change) => {
            result["after"].as_i64() == Some(i64::from(change.argb))
        }
        _ => false,
    }
}

fn impact_output_refused(value: &Value) -> bool {
    value["ok"] == false
        && value["native_outcome"] == "not_applied"
        && value.get("before").is_none()
        && value.get("after").is_none()
        && has_failure(value)
}

#[cfg(test)]
#[test]
fn impact_output_receipts_require_the_bound_reported_change() {
    let uid = "12345678-1234-1234-1234-123456789ABC";
    let operation = Operation::from_parts(
        "impact_output_set",
        json!({
            "track":"Scratch Impact","expected_instance":uid,"output_index":1,
            "output_name":"Impact St 2","expected_active":false,"active":true
        }),
    )
    .unwrap();
    let result = json!({"ok":true,"operation":"impact_output_set","native_outcome":"applied_verified",
        "outcome":"applied_verified","plugin":"Impact","output_count":32,
        "instance":{"object_uid":uid,"class_uid":"3713E26C-2FCA-4024-9F25-17E9D2BE2B9B","native_class":"AudioSynth"},
        "before":{"index":1,"name":"Impact St 2","requested":false,"acknowledged":false},
        "after":{"index":1,"name":"Impact St 2","requested":true,"acknowledged":true}});
    assert!(verify(&operation, &result));
    let mut pending = result.clone();
    pending["after"]["acknowledged"] = json!(false);
    assert!(!verify(&operation, &pending));
    let mut wrong_instance = result.clone();
    wrong_instance["instance"]["object_uid"] = json!("wrong");
    assert!(!verify(&operation, &wrong_instance));
    let mut refusal =
        json!({"ok":false,"native_outcome":"not_applied","error":{"code":"context_guard_failed"}});
    assert!(impact_output_refused(&refusal));
    refusal["after"] = result["after"].clone();
    assert!(!impact_output_refused(&refusal));
}

fn equivalent(expected: &Value, actual: &Value) -> bool {
    if expected == actual {
        return true;
    }
    let numeric = |value: &Value| {
        value
            .as_f64()
            .or_else(|| value.as_bool().map(|b| if b { 1.0 } else { 0.0 }))
    };
    match (numeric(expected), numeric(actual)) {
        (Some(a), Some(b)) => (a - b).abs() <= 1e-6 * (1.0 + a.abs()),
        _ => false,
    }
}

fn rejected(
    operation: &Operation,
    request_id: String,
    started: Instant,
    error: OperationError,
    context: Option<&HostContext>,
) -> Receipt {
    let mut result = Value::Null;
    if rumble_named_parameters::requested(operation) {
        rumble_named_parameters::invalidate(&mut result);
    }
    Receipt {
        request_id,
        outcome: Outcome::NotApplied,
        session: context.map(|c| c.session.clone()),
        document: context.map(|c| c.document.clone()),
        result,
        error: Some(bounded_cv_preset_error(operation, error)),
        elapsed_ms: started.elapsed().as_millis(),
    }
}

fn bounded_cv_preset_error(operation: &Operation, error: OperationError) -> OperationError {
    let applying = operation.applies_cv_preset_settings();
    if !operation.inspects_cv_preset_settings() && !applying {
        return error;
    }
    let (code, message) = match error.code.as_str() {
        "document_changed" => ("document_changed", "The active document changed"),
        "session_changed" => ("session_changed", "The bridge session changed"),
        "protocol_mismatch" => ("protocol_mismatch", "The bridge protocol is incompatible"),
        "native_context_lost" | "invalid_context" => (
            "native_context_lost",
            "The native document context is unavailable",
        ),
        "outcome_unknown" if applying => (
            "outcome_unknown",
            "The CV preset settings load outcome could not be established",
        ),
        "native_rejected" if applying => (
            "native_rejected",
            "CV preset settings were refused before loading",
        ),
        _ if applying => (
            "cv_preset_settings_apply_failed",
            "The CV preset settings apply request failed",
        ),
        _ => (
            "cv_preset_settings_unavailable",
            "CV preset settings inspection is unavailable",
        ),
    };
    OperationError::new(code, message)
}

fn host_error(response: &WireResponse) -> OperationError {
    OperationError::new(
        response.code.clone().unwrap_or_else(|| "host_error".into()),
        response
            .error
            .as_ref()
            .map(|v| {
                v.as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| v.to_string())
            })
            .unwrap_or_else(|| "Host rejected the operation".into()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_splitter_remove_requires_exact_anchors_and_a_bound_receipt() {
        for (tool, wire, occupied) in [
            (
                "insert_splitter_remove_empty_nested",
                "insertSplitterRemoveEmptyNested",
                None,
            ),
            (
                "insert_splitter_remove_one_lane_nested",
                "insertSplitterRemoveOneLaneNested",
                Some("60000000-1234-1234-1234-123456789ABC"),
            ),
        ] {
            check_nested_splitter_remove(tool, wire, occupied);
        }
    }

    fn check_nested_splitter_remove(tool: &str, wire: &str, occupied: Option<&str>) {
        let mut fixture = json!({
            "channel":"Scratch", "slot":0, "plugin":"Mixtool",
            "expected_instance":"12345678-1234-1234-1234-123456789ABC",
            "expected_graph":"10000000-1234-1234-1234-123456789ABC",
            "splitter_node":"20000000-1234-1234-1234-123456789ABC",
            "merger_node":"30000000-1234-1234-1234-123456789ABC",
            "predecessor_slot_node":"40000000-1234-1234-1234-123456789ABC",
            "successor_merger_node":"50000000-1234-1234-1234-123456789ABC"
        });
        if let Some(slot) = occupied {
            assert!(Operation::from_parts(tool, fixture.clone()).is_err());
            fixture["occupied_slot_node"] = fixture["predecessor_slot_node"].clone();
            assert!(
                Operation::from_parts(tool, fixture.clone())
                    .unwrap()
                    .validate()
                    .is_err()
            );
            fixture["occupied_slot_node"] = json!(slot);
        }
        let operation = Operation::from_parts(tool, fixture.clone()).unwrap();
        operation.validate().unwrap();
        assert!(operation.requires_explicit_document() && !operation.is_read_only());
        assert_eq!(operation.wire(), (wire.into(), fixture.clone()));
        for (key, value) in [
            ("slot", json!(1)),
            ("plugin", json!("Mixtool 2")),
            ("predecessor_slot_node", fixture["splitter_node"].clone()),
            (
                "successor_merger_node",
                json!("50000000-1234-1234-1234-123456789abc"),
            ),
        ] {
            let mut wrong = fixture.clone();
            wrong[key] = value;
            assert!(
                Operation::from_parts(tool, wrong)
                    .unwrap()
                    .validate()
                    .is_err()
            );
        }
        assert!(Operation::from_parts("insert_splitter_remove", fixture.clone()).is_err());
        let (_, args) = operation.wire();
        let context = HostContext {
            session: "session".into(),
            document: "session:1".into(),
            open: true,
            title: None,
            file_url: None,
            protocol: 2,
        };
        let mut success = fixture;
        for key in ["slot", "channel", "plugin"] {
            success.as_object_mut().unwrap().remove(key);
        }
        success["ok"] = json!(true);
        success["verified"] = json!(false);
        success["operation"] = json!(tool);
        success["outcome"] = json!("applied_unverified");
        success["native_outcome"] = json!("applied_unverified");
        success["instance"] = json!({"object_uid":args["expected_instance"],
            "class_uid":"E4D7D911-0608-4B46-ABA3-2E345399A5AC","object_id":"Mixtool",
            "native_class":"AudioEffect"});
        let mut response = WireResponse {
            protocol: 2,
            id: "receipt".into(),
            session: context.session.clone(),
            document: context.document.clone(),
            ok: true,
            result: success.clone(),
            error: None,
            code: None,
        };
        assert!(valid_nested_splitter_remove_receipt(
            tool, &args, &context, &response
        ));
        let (outcome, result, error) =
            nested_splitter_remove_result(tool, &args, &context, &response);
        assert_eq!(outcome, Outcome::AppliedUnverified);
        assert_eq!(result, success);
        assert!(error.is_none());

        for (key, value) in [
            ("verified", json!(true)),
            ("successor_merger_node", success["merger_node"].clone()),
            ("native_outcome", json!("applied_verified")),
            ("instance", json!({"object_uid":args["expected_instance"]})),
        ] {
            response.result = success.clone();
            response.result[key] = value;
            assert!(!valid_nested_splitter_remove_receipt(
                tool, &args, &context, &response
            ));
        }
        if occupied.is_some() {
            response.result = success.clone();
            response
                .result
                .as_object_mut()
                .unwrap()
                .remove("occupied_slot_node");
            assert!(!valid_nested_splitter_remove_receipt(
                tool, &args, &context, &response
            ));
            response.result["occupied_slot_node"] = success["predecessor_slot_node"].clone();
            assert!(!valid_nested_splitter_remove_receipt(
                tool, &args, &context, &response
            ));
        }
        response.result = success;
        response.document = "session:2".into();
        let (outcome, result, error) =
            nested_splitter_remove_result(tool, &args, &context, &response);
        assert_eq!(outcome, Outcome::Unknown);
        assert_eq!(result["ok"], false);
        assert!(result.get("splitter_node").is_none() && result.get("instance").is_none());
        assert_eq!(error.unwrap().code, "outcome_unknown");
        response.ok = false;
        response.code = Some("document_changed".into());
        response.error = Some(json!("The active document changed"));
        let (outcome, result, _) = nested_splitter_remove_result(tool, &args, &context, &response);
        assert_eq!(outcome, Outcome::Unknown);
        assert!(result.get("splitter_node").is_none());
        response.result = Value::Null;
        let (outcome, result, error) =
            nested_splitter_remove_result(tool, &args, &context, &response);
        assert_eq!(outcome, Outcome::NotApplied);
        assert!(result.is_null());
        assert_eq!(error.unwrap().code, "document_changed");

        response.document = context.document.clone();
        response.ok = true;
        response.code = None;
        response.error = None;
        response.result = json!({"ok":false,"native_outcome":"not_applied","errors":[{
            "code":format!("{tool}_failed"),"message":"Topology changed"
        }]});
        let (outcome, result, error) =
            nested_splitter_remove_result(tool, &args, &context, &response);
        assert_eq!(outcome, Outcome::NotApplied);
        assert_eq!(result, response.result);
        assert_eq!(error.unwrap().code, "native_rejected");
        response.result["errors"] = json!([]);
        assert!(!valid_nested_splitter_remove_receipt(
            tool, &args, &context, &response
        ));
    }

    #[test]
    fn impact_outputs_require_a_bound_instrument_and_ordered_complete_observation() {
        let uid = "12345678-1234-1234-1234-123456789ABC";
        let args = json!({"kind":"instrument","track":"Scratch Impact","plugin":"Impact",
            "source":"dsp","include_nodes":true,"node_scope":"impact_outputs","expected_instance":uid});
        let operation = Operation::from_parts("pluginInspect", args.clone()).unwrap();
        operation.validate().unwrap();
        assert!(operation.requires_explicit_document() && operation.is_read_only());
        for (field, wrong) in [
            ("plugin", json!("Presence")),
            (
                "expected_instance",
                json!("00000000-0000-0000-0000-000000000000"),
            ),
            ("channel", json!("Impact St 1")),
            ("include_context", json!(true)),
            ("offset", json!(0)),
        ] {
            let mut misplaced = args.clone();
            misplaced[field] = wrong;
            assert!(
                Operation::from_parts("pluginInspect", misplaced)
                    .unwrap()
                    .validate()
                    .is_err(),
                "{field}"
            );
        }
        let Operation::PluginInspect(query) = operation else {
            panic!("expected plugin inspection");
        };
        let outputs: Vec<_> = (0..32)
            .map(|index| {
                json!({"index":index,"name":format!("Output {index}"),
                "requested":index==0,"acknowledged":index==0})
            })
            .collect();
        let observed = json!({"plugin":"Impact","native":{"ok":true,"outcome":"observed","source":"dsp",
            "instance":{"object_uid":uid,"class_uid":"3713E26C-2FCA-4024-9F25-17E9D2BE2B9B",
                "native_class":"AudioSynth","object_id":"Impact"},
            "node_report":{"scope":"impact_outputs","value_semantics":"sequential_snapshot",
                "complete":true,"output_count":32,"outputs":outputs}}});
        assert!(valid_impact_outputs(&query, &observed));
        for (index, field, wrong) in [
            (1, "index", json!(0)),
            (31, "name", json!("")),
            (0, "acknowledged", json!(1)),
        ] {
            let mut malformed = observed.clone();
            malformed["native"]["node_report"]["outputs"][index][field] = wrong;
            assert!(!valid_impact_outputs(&query, &malformed), "{field}");
        }
    }

    fn mutating_operations() -> Vec<Operation> {
        [
            ("setPanner", json!({"channel":"Scratch","param":"panX1","value":0.5})),
            ("setSplitter", json!({"channel":"Scratch","node":"Splitter","label":"Splitter","param":"numOutputs","value":2})),
            ("setSend", json!({"channel":"Scratch","index":0,"level":0.5})),
            ("setInsertBypass", json!({"channel":"Scratch","slot":0,"plugin":"Phase Meter","bypassed":true})),
            ("setAutomation", json!({"channel":"Scratch","mode":"read"})),
            ("metronome", json!({"click":true})),
            ("takes", json!({"track":"Scratch","action":"next"})),
            ("trackState", json!({"track":"Scratch","action":"hide"})),
            ("editEvents", json!({"track":"Scratch","action":"mute"})),
            ("externalPortMenu", json!({"channel":"Scratch","slot":0,"plugin":"External Effect Stereo","expected_instance":"12345678-1234-1234-1234-123456789ABC","port":"send"})),
            ("externalStereoSendSet", json!({"channel":"Scratch","slot":1,"expected_instance":"12345678-1234-1234-1234-123456789ABC","expected_current":"none","destination":"main"})),
            ("pluginEditorOpen", json!({"channel":"Scratch","slot":0,"plugin":"Phase Meter"})),
            ("pluginState", json!({"channel":"Scratch","slot":0,"plugin":"Phase Meter","expected_instance":"12345678-1234-1234-1234-123456789ABC","action":"capture"})),
            ("insertChainState", json!({"channel":"Scratch","action":"capture"})),
            ("insertSlotMove", json!({"channel":"Scratch","slot":1,"plugin":"Mixtool","expected_instance":"12345678-1234-1234-1234-123456789ABC",
                "expected_graph":"10000000-1234-1234-1234-123456789ABC","source_node":"20000000-1234-1234-1234-123456789ABC",
                "after_node":"30000000-1234-1234-1234-123456789ABC","following_node":"40000000-1234-1234-1234-123456789ABC"})),
            ("pluginMultiAddMaiTai", json!({"track":"Selected music track","expected_instance":"12345678-1234-1234-1234-123456789ABC",
                "expected_folder":"10000000-1234-1234-1234-123456789ABC","expected_graph":"20000000-1234-1234-1234-123456789ABC",
                "expected_child_slot":"30000000-1234-1234-1234-123456789ABC","expected_child_instance":"40000000-1234-1234-1234-123456789ABC"})),
            ("insertSplitterAdd", json!({"channel":"Scratch","slot":0,"plugin":"Mixtool","expected_instance":"12345678-1234-1234-1234-123456789ABC",
                "expected_graph":"10000000-1234-1234-1234-123456789ABC","after_node":"20000000-1234-1234-1234-123456789ABC",
                "following_node":"30000000-1234-1234-1234-123456789ABC"})),
            ("insertSplitterRemove", json!({"channel":"Scratch","slot":0,"plugin":"Mixtool","expected_instance":"12345678-1234-1234-1234-123456789ABC",
                "expected_graph":"10000000-1234-1234-1234-123456789ABC","splitter_node":"20000000-1234-1234-1234-123456789ABC",
                "merger_node":"30000000-1234-1234-1234-123456789ABC","first_slot_node":"40000000-1234-1234-1234-123456789ABC",
                "second_slot_node":"50000000-1234-1234-1234-123456789ABC"})),
            ("insertSplitterRemoveEmptyNested", json!({"channel":"Scratch","slot":0,"plugin":"Mixtool","expected_instance":"12345678-1234-1234-1234-123456789ABC",
                "expected_graph":"10000000-1234-1234-1234-123456789ABC","splitter_node":"20000000-1234-1234-1234-123456789ABC",
                "merger_node":"30000000-1234-1234-1234-123456789ABC","predecessor_slot_node":"40000000-1234-1234-1234-123456789ABC",
                "successor_merger_node":"50000000-1234-1234-1234-123456789ABC"})),
            ("pluginPresets", json!({"channel":"Scratch","slot":0,"plugin":"Phase Meter","expected_instance":"12345678-1234-1234-1234-123456789ABC","action":"list"})),
            ("pluginAction", json!({"channel":"Scratch","slot":0,"plugin":"Phase Meter","expected_instance":"12345678-1234-1234-1234-123456789ABC","action":"reset_meter_hold"})),
            ("fabfilterPresetApply", json!({"product":"Pro-Q 4","index":0,"expected_catalog":"0".repeat(64),"channel":"Scratch","slot":0,"expected_instance":"12345678-1234-1234-1234-123456789ABC"})),
            ("serum2PresetApply", json!({"target":{"variant":"instrument","track":"Scratch"},"path":"/private/fixture.vstpreset",
                "expected_instance":"12345678-1234-1234-1234-123456789ABC","expected_sha256":"0".repeat(64)})),
            ("pedalboardSlotSwap", json!({"channel":"Scratch","slot":0,"expected_instance":"12345678-1234-1234-1234-123456789ABC",
                "first_slot":1,"second_slot":2,"expected_first_model":"Compressor","expected_second_model":"Reverb"})),
            ("impactOutputSet", json!({"track":"Scratch Impact","expected_instance":"12345678-1234-1234-1234-123456789ABC","output_index":1,"output_name":"Impact St 2","expected_active":false,"active":true})),
            ("editChorderAutofill", json!({"track":"Scratch","slot":0,"expected_instance":"12345678-1234-1234-1234-123456789ABC","enabled":true})),
            ("chorderLearningSet", json!({"track":"Scratch","slot":0,"expected_instance":"12345678-1234-1234-1234-123456789ABC","change":{"kind":"learn_mode","enabled":true}})),
            ("setPluginParam", json!({"channel":"Scratch","slot":0,"plugin":"Phase Meter","param":"gain","value":0})),
            ("setChannel", json!({"channel":"Scratch","field":"mute","value":true})),
            ("setChannelLabel", json!({"channel":"Scratch","name":"Scratch"})),
            ("setChannelColor", json!({"channel":"Scratch","argb":0})),
            ("setTransport", json!({"tempo":120})),
            ("transport", json!({"action":"stop"})),
            ("setLoop", json!({"start":0,"end":4,"enabled":true})),
            ("addTrack", json!({"type":"audioMono"})),
            ("selectTrack", json!({"name":"Scratch"})),
            ("command", json!({"category":"Transport","name":"Stop","checkOnly":false})),
            ("trackTask", json!({"ops":[{"op":"focusTrack","track":"Scratch"}]})),
            ("editNotes", json!({"track":"Scratch","ops":[{"op":"transpose","semitones":1}]})),
            ("midiProgramPartAdd", json!({"track":"Scratch","expected_instance":"12345678-1234-1234-1234-123456789ABC","bank_msb":0,"bank_lsb":0,"program":5})),
            ("markers", json!({})),
            ("addMarker", json!({"position":1})),
            ("deleteMarker", json!({"number":1})),
            ("selectEvents", json!({"track":"Scratch","all":true})),
            ("save", json!({})),
            ("undo", json!({"steps":1})),
            ("redo", json!({"steps":1})),
        ]
        .into_iter()
        .map(|(name, args)| {
            let operation = Operation::from_parts(name, args).unwrap();
            operation.validate().unwrap_or_else(|error| panic!("{name}: {error:?}"));
            assert!(!operation.is_read_only(), "{name}");
            operation
        })
        .collect()
    }

    #[test]
    fn mutation_document_fixtures_cover_every_registered_operation() {
        let schema = crate::operation::schema();
        let registered = schema["oneOf"]
            .as_array()
            .unwrap()
            .iter()
            .map(|variant| {
                variant["properties"]["operation"]["const"]
                    .as_str()
                    .unwrap()
            })
            .collect::<std::collections::BTreeSet<_>>();
        let operations = mutating_operations();
        let mut covered = operations
            .iter()
            .map(|operation| operation.wire().0)
            .collect::<std::collections::BTreeSet<_>>();
        covered.extend(
            [
                "ping",
                "context",
                "hostProbe",
                "song",
                "tracks",
                "trackFormats",
                "channels",
                "panner",
                "splitters",
                "meters",
                "notes",
                "plugins",
                "pluginCatalog",
                "inserts",
                "sends",
                "pluginParams",
                "pluginMultiChildParameters",
                "pluginChoices",
                "pluginParameterDetails",
                "chorderLearningGet",
                "chorderChordsGet",
                "pluginParameterFormat",
                "pluginParameterDigest",
                "pluginModels",
                "pluginInspect",
                "listCommands",
            ]
            .into_iter()
            .map(str::to_owned),
        );
        assert_eq!(
            covered
                .iter()
                .map(String::as_str)
                .collect::<std::collections::BTreeSet<_>>(),
            registered
        );
    }

    #[test]
    fn stock_mai_tai_serialized_receipt_distinguishes_refusal_from_uncertain_dispatch() {
        let Operation::PluginMultiAddMaiTai(change) = mutating_operations()
            .into_iter()
            .find(|operation| matches!(operation, Operation::PluginMultiAddMaiTai(_)))
            .unwrap()
        else {
            unreachable!()
        };
        let context = HostContext {
            session: "bridge-session".into(),
            document: "bridge-session:1".into(),
            open: true,
            title: None,
            file_url: None,
            protocol: 2,
        };
        let acknowledged = json!({
            "ok": true, "operation": "multi_add_mai_tai", "outcome": "applied_unverified",
            "native_outcome": "applied_unverified","dispatch_attempted": true,
            "instrument": "Mai Tai", "graph_attachment_verified": false, "undo_verified": false,
            "instance": {
                "object_uid":change.expected_instance,"class_uid":"B9D410AD-E47D-495E-8AEE-10D2B2987CC7",
                "native_class":"AudioSynth","object_id":"Multi"
            },
        });
        let refused = json!({
            "ok":false, "outcome":"not_applied", "native_outcome":"not_applied", "dispatch_attempted":false,
            "error":{"code":"multi_add_mai_tai_refused","message":"the expected native plugin instance is unavailable"}
        });
        let receipt = |result, session: &str, document: &str| {
            serde_json::to_value(multi_add_mai_tai_receipt(
                &change,
                &context,
                WireResponse {
                    protocol: 2,
                    id: "wire-id".into(),
                    session: session.into(),
                    document: document.into(),
                    ok: true,
                    result,
                    error: None,
                    code: None,
                },
                "owner-id".into(),
                Instant::now(),
            ))
            .unwrap()
        };
        let ack = receipt(acknowledged.clone(), &context.session, &context.document);
        assert_eq!(ack["request_id"], "owner-id");
        assert_eq!(ack["outcome"], "applied_unverified");
        assert!(ack.get("error").is_none());
        let rejected = receipt(refused.clone(), &context.session, &context.document);
        assert_eq!(rejected["outcome"], "not_applied");
        assert_eq!(rejected["error"]["code"], "multi_add_mai_tai_refused");
        assert_eq!(rejected["result"]["dispatch_attempted"], false);
        let mut contradictory = refused.clone();
        contradictory["dispatch_attempted"] = json!(true);
        let mut malformed = acknowledged.clone();
        malformed["graph_attachment_verified"] = json!(true);
        for reply in [
            receipt(contradictory, &context.session, &context.document),
            receipt(malformed, &context.session, &context.document),
            receipt(acknowledged, &context.session, "bridge-session:2"),
            receipt(refused, "unbound-session", &context.document),
            receipt(Value::Null, &context.session, &context.document),
        ] {
            assert_eq!(reply["outcome"], "outcome_unknown");
            assert_eq!(reply["error"]["code"], "multi_add_mai_tai_unknown");
            assert!(reply["result"].get("instance").is_none());
        }
    }

    #[test]
    fn all_mutations_require_nonblank_document_before_owner_enqueue() {
        let (sender, receiver) = mpsc::sync_channel(1);
        let runtime = RuntimeHandle { sender };
        let executor = tokio::runtime::Runtime::new().unwrap();
        for operation in mutating_operations() {
            for expected_document in [None, Some(String::new()), Some(" \t\n".into())] {
                let error = executor
                    .block_on(async {
                        tokio::time::timeout(
                            std::time::Duration::from_secs(1),
                            runtime.execute(Request {
                                operation: operation.clone(),
                                expected_document,
                            }),
                        )
                        .await
                        .expect("A rejected request must not wait for the DAW owner")
                    })
                    .unwrap_err();
                assert_eq!(error.code, "missing_precondition", "{}", operation.wire().0);
                assert!(matches!(
                    receiver.try_recv(),
                    Err(mpsc::TryRecvError::Empty)
                ));
            }
        }
    }

    #[test]
    fn valid_mutation_documents_and_ordinary_reads_reach_owner_admission() {
        let (sender, receiver) = mpsc::sync_channel(1);
        drop(receiver);
        let runtime = RuntimeHandle { sender };
        let executor = tokio::runtime::Runtime::new().unwrap();
        let mutations = mutating_operations().into_iter().map(|operation| Request {
            operation,
            expected_document: Some("session:1".into()),
        });
        let reads = [
            ("context", json!({})),
            ("song", json!({})),
            ("tracks", json!({})),
            ("metronome", json!({})),
            ("takes", json!({"track":"Scratch","action":"list"})),
            (
                "command",
                json!({"category":"Transport","name":"Stop","checkOnly":true}),
            ),
        ]
        .into_iter()
        .map(|(name, args)| {
            let operation = Operation::from_parts(name, args).unwrap();
            assert!(operation.is_read_only());
            Request {
                operation,
                expected_document: None,
            }
        });
        for request in mutations.chain(reads) {
            let name = request.operation.wire().0;
            let error = executor.block_on(runtime.execute(request)).unwrap_err();
            assert_eq!(error.code, "owner_stopped", "{name}");
        }
    }

    #[test]
    fn context_bound_reads_still_require_nonblank_document_before_enqueue() {
        let target = json!({"channel":"Scratch","slot":0,"plugin":"Level Meter","source":"level_meter","expected_instance":"12345678-1234-1234-1234-123456789ABC"});
        let (sender, receiver) = mpsc::sync_channel(1);
        drop(receiver);
        let runtime = RuntimeHandle { sender };
        let executor = tokio::runtime::Runtime::new().unwrap();
        let mut operations = vec![Operation::from_parts("chorderLearningGet", json!({"track":"Scratch","slot":0,"expected_instance":"12345678-1234-1234-1234-123456789ABC"})).unwrap()];
        operations.push(Operation::from_parts("chorderChordsGet", json!({"track":"Scratch","slot":0,"expected_instance":"12345678-1234-1234-1234-123456789ABC","offset":0,"limit":1})).unwrap());
        operations.push(
            Operation::from_parts(
                "pluginMultiChildParameters",
                json!({
                    "track": "Scratch",
                    "expected_instance": "12345678-1234-1234-1234-123456789ABC",
                    "child_slot": "Inst01",
                    "expected_child_slot": "22345678-1234-1234-1234-123456789ABC",
                    "expected_child_instance": "32345678-1234-1234-1234-123456789ABC"
                }),
            )
            .unwrap(),
        );
        for action in [
            "read_rumble_state",
            "read_rumble_parameters",
            "read_rumble_named_parameters",
            "inspect_rumble_serialization",
        ] {
            operations.push(Operation::from_parts("pluginState", json!({"channel":"Scratch","slot":0,"plugin":"Rumble Native","expected_instance":"12345678-1234-1234-1234-123456789ABC","action":action})).unwrap());
        }
        for node_scope in ["cv_ports", "cv_port_parameters", "cv_preset_target"] {
            operations.push(Operation::from_parts("pluginInspect", json!({"kind":"instrument","track":"Scratch","plugin":"CV Instrument","source":"dsp","expected_instance":"12345678-1234-1234-1234-123456789ABC","include_nodes":true,"node_scope":node_scope})).unwrap());
        }
        for (name, extra) in [
            ("pluginParams", json!({"names":[]})),
            ("pluginChoices", json!({"param":"mode"})),
            (
                "pluginParameterFormat",
                json!({"param":"mode","values":[0,1]}),
            ),
        ] {
            let mut args = target.clone();
            args.as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            operations.push(Operation::from_parts(name, args).unwrap());
        }
        for operation in operations {
            assert!(operation.is_read_only());
            assert!(operation.requires_explicit_document());
            for expected_document in [None, Some(String::new()), Some(" \t\n".into())] {
                let error = executor
                    .block_on(runtime.execute(Request {
                        operation: operation.clone(),
                        expected_document,
                    }))
                    .unwrap_err();
                assert_eq!(error.code, "missing_precondition", "{}", operation.wire().0);
            }
            let error = executor
                .block_on(runtime.execute(Request {
                    operation,
                    expected_document: Some("session:1".into()),
                }))
                .unwrap_err();
            assert_eq!(error.code, "owner_stopped");
        }
    }

    #[test]
    fn named_rumble_guards_run_before_enqueue_and_rejected_context_has_closed_failure() {
        let (sender, receiver) = mpsc::sync_channel(1);
        let runtime = RuntimeHandle { sender };
        let operation=Operation::from_parts("plugin_state",json!({"channel":"Scratch","slot":0,"plugin":"Rumble Native","expected_instance":"12345678-1234-1234-1234-123456789ABC","action":"read_rumble_named_parameters"})).unwrap();
        let executor = tokio::runtime::Runtime::new().unwrap();
        for expected_document in [None, Some(String::new())] {
            let error = executor
                .block_on(runtime.execute(Request {
                    operation: operation.clone(),
                    expected_document,
                }))
                .unwrap_err();
            assert_eq!(error.code, "missing_precondition");
            assert!(receiver.try_recv().is_err());
        }
        let mut invalid = operation.clone();
        if let Operation::PluginState(ref mut q) = invalid {
            q.expected_instance = "00000000-0000-0000-0000-000000000000".into();
        }
        let error = executor
            .block_on(runtime.execute(Request {
                operation: invalid,
                expected_document: Some("session:1".into()),
            }))
            .unwrap_err();
        assert_eq!(error.code, "invalid_argument");
        assert!(receiver.try_recv().is_err());
        for token in ["", "issued"] {
            let mut invalid = operation.clone();
            if let Operation::PluginState(ref mut query) = invalid {
                query.token = Some(token.into());
            }
            let error = executor
                .block_on(runtime.execute(Request {
                    operation: invalid,
                    expected_document: Some("session:1".into()),
                }))
                .unwrap_err();
            assert_eq!(error.code, "invalid_argument");
            assert!(receiver.try_recv().is_err());
        }
        let receipt = rejected(
            &operation,
            "test".into(),
            Instant::now(),
            OperationError::new("document_changed", "PRIVATE"),
            None,
        );
        assert_eq!(receipt.result.as_object().unwrap().len(), 10);
        assert_eq!(
            receipt.result["error"]["code"],
            "rumble_named_parameters_unavailable"
        );
        assert!(!receipt.result.to_string().contains("PRIVATE"));
    }

    #[test]
    fn cv_preset_settings_errors_discard_text_and_codes_from_an_older_bridge() {
        let mut args = json!({"kind":"instrument","track":"Scratch","plugin":"CV Instrument",
            "expected_instance":"12345678-1234-1234-1234-123456789ABC",
            "catalog_token":"A".repeat(64),"index":7});
        let private = "/private/mock/path.preset";
        for action in ["inspect_cv_preset_settings", "apply_cv_preset_settings"] {
            args["action"] = json!(action);
            let operation = Operation::from_parts("plugin_presets", args.clone()).unwrap();
            let error = bounded_cv_preset_error(&operation, OperationError::new(private, private));
            assert!(!error.code.contains(private));
            assert!(!error.message.contains(private));
            let error = bounded_cv_preset_error(
                &operation,
                OperationError::new("document_changed", private),
            );
            assert_eq!(error.code, "document_changed");
            assert!(!error.message.contains(private));
        }
        args["action"] = json!("apply");
        let ordinary = Operation::from_parts("plugin_presets", args).unwrap();
        assert_eq!(
            bounded_cv_preset_error(&ordinary, OperationError::new("host_error", private)).message,
            private
        );
    }

    #[test]
    fn changed_cv_preset_settings_receipt_discards_the_digest() {
        let args = json!({"kind":"instrument","track":"Scratch","plugin":"CV Instrument",
            "expected_instance":"12345678-1234-1234-1234-123456789ABC",
            "action":"inspect_cv_preset_settings","catalog_token":"A".repeat(64),"index":7});
        let op = Operation::from_parts("plugin_presets", args).unwrap();
        let mut result = json!({"cv_preset_settings":{"byte_count":3991,"sha256":"a".repeat(64)}});
        invalidate_gui_evidence(&op, &mut result);
        assert!(result.is_null());
    }

    #[test]
    fn changed_cv_preset_target_receipt_discards_all_native_evidence() {
        let mut args = json!({"kind":"instrument","track":"Scratch","plugin":"CV Instrument",
            "source":"dsp","expected_instance":"12345678-1234-1234-1234-123456789ABC",
            "include_nodes":true,"node_scope":"cv_preset_target"});
        let operation = Operation::from_parts("plugin_inspect", args.clone()).unwrap();
        let stale = json!({"plugin":"CV Instrument","native":{"ok":true,
            "instance":{"object_uid":args["expected_instance"]},"node_report":{"scope":"cv_preset_target",
            "target_istorable":"present"}}});
        let mut result = stale.clone();
        invalidate_gui_evidence(&operation, &mut result);
        assert_eq!(result, Value::Null);

        args["node_scope"] = json!("cv_ports");
        let ordinary = Operation::from_parts("plugin_inspect", args).unwrap();
        let mut result = stale.clone();
        invalidate_gui_evidence(&ordinary, &mut result);
        assert_eq!(result, stale);
    }

    #[test]
    fn changed_document_gui_reads_discard_all_identity_claims() {
        let mut args = json!({"kind":"note_effect","track":"Scratch","slot":0,"plugin":"Chorder",
            "param":"chorder.autofill","source":"dsp","expected_instance":"12345678-1234-1234-1234-123456789ABC"});
        let mut result = json!({"ok":true,"gui_comparison":{"status":"compared","topParameterSame":true,
            "declaredObserverSame":true,"controller_probe":{"status":"observed",
                "interfaces":{"controller":{"status":"present"}},
                "binding":{"autofill_same_gui":true},
                "parameters":{"learnInputPitch":{"value":-1}},
                "observer_metadata":{"status":"observed","report":{"nodes":[{"reflection":{"types":[{"name":"stale"}]}}]}}}},
            "controller_report":{"stale":true}});
        result["gui_comparison"]["controller_probe"]["learning_child"] =
            fixed_learning_child_observation();
        let legacy = Operation::from_parts("plugin_parameter_details", args.clone()).unwrap();
        let before = result.clone();
        invalidate_gui_evidence(&legacy, &mut result);
        assert_eq!(result, before);
        args["compare_gui"] = json!(true);
        args["include_owner"] = json!(true);
        let operation = Operation::from_parts("plugin_parameter_details", args).unwrap();
        invalidate_gui_evidence(&operation, &mut result);
        assert_eq!(result["native_outcome"], "not_applied");
        assert_eq!(result["gui_comparison"]["status"], "unresolved");
        assert_eq!(result["gui_comparison"]["coverage"]["max_views"], 128);
        assert_eq!(result["gui_comparison"]["coverage"]["max_depth"], 32);
        assert!(result["gui_comparison"].get("topParameterSame").is_none());
        assert!(
            result["gui_comparison"]
                .get("declaredObserverSame")
                .is_none()
        );
        assert!(result["gui_comparison"].get("controller_probe").is_none());
        assert!(result.get("controller_report").is_none());
        assert!(!result.to_string().contains("learning_child"));
    }

    fn gui_description(value: i64, float: bool) -> Value {
        let typed = |n: i64| {
            if float {
                json!({"variant_tag":2,"data":{"kind":"float","value":n as f64,"bits_hex":format!("{:016X}",(n as f64).to_bits())}})
            } else {
                json!({"variant_tag":1,"data":{"kind":"integer","value":n}})
            }
        };
        json!({"name":"autofill","title":"autofill","tag":4,"value":value,"text":value.to_string(),"min":0,"max":1,"default":0,
            "normalized":value,"enabled":true,"readOnly":false,"writable":true,"native_kind":"Toggle","native_value":typed(value),
            "native_min":typed(0),"native_max":typed(1),"native_default":typed(0),
            "native_normalized":{"kind":"float32","bits_hex":if value==0{"00000000"}else{"3F800000"}},
            "native_support":{"raw_number":true,"normalized":true,"display_text":true},
            "support":{"raw":true,"normalized":true,"text":true},"display_parser":"PlainNumeric"})
    }

    fn fixed_learning_child_observation() -> Value {
        let mut member = gui_description(0, false);
        member["name"] = json!("learnChord");
        member["title"] = json!("learnChord");
        let mut assigned = gui_description(1, false);
        assigned["name"] = json!("chordAssigned");
        assigned["title"] = json!("chordAssigned");
        assigned["native_kind"] = json!("Integer");
        assigned["readOnly"] = json!(true);
        assigned["writable"] = json!(false);
        assigned["max"] = json!(2);
        assigned["native_max"]["data"]["value"] = json!(2);
        assigned["normalized"] = json!(0.5);
        assigned["native_normalized"]["bits_hex"] = json!("3F000000");
        assigned["native_support"] =
            json!({"raw_number":false,"normalized":false,"display_text":false});
        assigned["support"] = json!({"raw":false,"normalized":false,"text":false});
        json!({"status":"observed","scope":"fixed_chorder_pitch61_learning_parameters","child_id":"pitch61",
            "mutation_capability":false,"value_semantics":"sequential_snapshot","output_pitch_mapping":"not_established",
            "interfaces":{"root_node":{"status":"present"},"controller":{"status":"present"}},"root_child_count":128,
            "binding":{"root_same_observer":true,"parent_same_root":true,"controller_same_child":true},
            "parameters":{"learnChord":member,"chordAssigned":assigned}})
    }

    fn gui_edit_success(float: bool) -> (crate::operation::ChorderAutofillSet, Value) {
        let change = crate::operation::ChorderAutofillSet {
            track: "Scratch".into(),
            slot: 0,
            enabled: true,
            expected_instance: "12345678-1234-1234-1234-123456789ABC".into(),
        };
        let value = json!({"ok":true,"outcome":"applied_unverified","native_outcome":"applied_unverified","operation":"edit_chorder_autofill",
            "instance":{"object_uid":change.expected_instance,"class_uid":"D2D9B002-12FC-4F1D-AFDA-70336D1DC108"},"binding_source":"dsp",
            "logical_parameter":"chorder.autofill","receiver":"editor_control","requested_enabled":true,"edit_phase":"closed_checked",
            "gui_parameter":{"name":"autofill","before":gui_description(0,float),"after":gui_description(1,float)},"gui_value_verified":true,
            "verification_scope":"gui_receiver_current_value","persistence_verified":false,"effects_verified":false,"dsp_propagation_verified":false,"plugin":"Chorder"});
        (change, value)
    }

    #[test]
    fn gui_edit_receipts_never_promote_gui_value_to_dsp_or_persistence_verification() {
        for float in [false, true] {
            let (change, value) = gui_edit_success(float);
            assert!(valid_gui_edit_receipt(&change, &value));
            assert_eq!(mutation_outcome(&value), Outcome::AppliedUnverified);
            assert!(!verify(
                &Operation::EditChorderAutofill(change.clone()),
                &value
            ));
            for (key, forged) in [
                ("gui_value_verified", json!("true")),
                ("outcome", json!("applied_verified")),
                ("persistence_verified", json!(true)),
                ("effects_verified", json!(true)),
                ("native_verified", json!(true)),
                ("dsp_propagation_verified", json!(true)),
                ("edit_phase", json!("end_dispatched")),
                ("receiver", json!("dsp")),
                ("requested_enabled", json!(false)),
            ] {
                let mut invalid = value.clone();
                invalid[key] = forged;
                assert!(!valid_gui_edit_receipt(&change, &invalid), "{key}");
            }
            let mut invalid = value.clone();
            invalid["gui_parameter"]["after"]["native_value"]["data"]["value"] = json!(0);
            assert!(!valid_gui_edit_receipt(&change, &invalid));
            let mut invalid = value.clone();
            invalid["gui_parameter"]["after"]["name"] = json!("chorder.autofill");
            assert!(!valid_gui_edit_receipt(&change, &invalid));
            let mut invalid = value.clone();
            invalid["gui_parameter"]["after"]["native_normalized"]["bits_hex"] = json!("00000000");
            assert!(!valid_gui_edit_receipt(&change, &invalid));
            if float {
                let mut invalid = value.clone();
                invalid["gui_parameter"]["after"]["native_value"]["data"]["bits_hex"] =
                    json!("0000000000000000");
                assert!(!valid_gui_edit_receipt(&change, &invalid));
            }
        }
    }

    #[test]
    fn gui_edit_failure_and_late_publication_remove_all_positive_receiver_evidence() {
        let (change, mut value) = gui_edit_success(false);
        invalidate_gui_evidence(&Operation::EditChorderAutofill(change.clone()), &mut value);
        assert_eq!(value["native_outcome"], "unknown");
        assert_eq!(value["gui_value_verified"], false);
        assert_eq!(value["edit_phase"], "closed_checked");
        assert!(value.get("gui_parameter").is_none());
        for phase in [
            "prepared",
            "begin_dispatched",
            "value_dispatched",
            "end_dispatched",
            "closed_checked",
        ] {
            let mut failure = json!({"ok":false,"native_outcome":"not_applied","edit_phase":phase,
                "errors":[{"code":"refused","message":"Unavailable"}]});
            assert_eq!(
                valid_gui_edit_receipt(&change, &failure),
                phase == "prepared"
            );
            invalidate_gui_evidence(
                &Operation::EditChorderAutofill(change.clone()),
                &mut failure,
            );
            assert_eq!(
                failure["native_outcome"],
                if phase == "prepared" {
                    "not_applied"
                } else {
                    "unknown"
                }
            );
            assert_eq!(failure["gui_value_verified"], false);
            assert_eq!(failure["errors"][0]["code"], "refused");
        }
    }

    #[test]
    fn readback_comparison_handles_host_boolean_numbers() {
        assert!(equivalent(&json!(true), &json!(1)));
        assert!(!equivalent(&json!(true), &json!(0)));
        assert!(equivalent(&json!(0.5), &json!(0.50000001)));
        assert!(!equivalent(&json!(0.5), &json!(0.6)));
    }

    #[test]
    fn numeric_edit_phase_does_not_promote_native_value_verification() {
        let operation = Operation::from_parts(
            "plugin_parameter_set",
            json!({"kind":"note_effect",
            "track":"Scratch","slot":0,"plugin":"Chorder","param":"chorder.autofill",
            "source":"dsp","value":1,"expected_instance":"12345678-1234-1234-1234-123456789ABC",
            "edit_lifecycle":"native_user"}),
        )
        .unwrap();
        assert!(!verify(
            &operation,
            &json!({"edit_phase":"closed_checked","after":{"value":1},"native_verified":false})
        ));
        assert!(!verify(
            &operation,
            &json!({"edit_phase":"closed_checked","after":{"value":1}})
        ));
        assert!(verify(
            &operation,
            &json!({"edit_phase":"closed_checked","native_verified":true})
        ));
        assert_eq!(
            mutation_outcome(&json!({"edit_phase":"begin_dispatched","native_outcome":"unknown"})),
            Outcome::Unknown
        );
        assert_eq!(
            mutation_outcome(&json!({"edit_phase":"prepared","native_outcome":"not_applied"})),
            Outcome::NotApplied
        );
    }

    #[test]
    fn native_plugin_verification_and_unknown_outcomes_are_authoritative() {
        let operation = Operation::from_parts("setPluginParam", json!({"channel":"Scratch","slot":0,"plugin":"External Effect Mono","param":"inputGain","value":0.5})).unwrap();
        assert!(!verify(
            &operation,
            &json!({"after":{"value":0.50000001},"native_verified":false})
        ));
        let text = Operation::from_parts("setPluginParam", json!({"channel":"Scratch","slot":0,"plugin":"External Effect Mono","param":"inputGain","text":"-6db"})).unwrap();
        assert!(verify(
            &text,
            &json!({"after":{"text":"-6.0 dB"},"native_verified":true})
        ));
        assert_eq!(
            mutation_outcome(&json!({"native_outcome":"not_applied","errors":["read only"]})),
            Outcome::NotApplied
        );
        assert_eq!(
            mutation_outcome(
                &json!({"native_outcome":"unknown","before":{"value":1},"after":{"value":0.5}})
            ),
            Outcome::Unknown
        );
    }

    #[test]
    fn panner_and_bypass_verification_require_matching_readback() {
        let panner = Operation::from_parts(
            "setPanner",
            json!({"channel":"Scratch","param":"panWidth","value":0.5}),
        )
        .unwrap();
        assert!(verify(&panner, &json!({"after":{"value":0.5}})));
        assert!(!verify(&panner, &json!({"after":{"value":0.6}})));
        assert!(!verify(&panner, &json!({"requested":true})));
        let bypass = Operation::from_parts(
            "setInsertBypass",
            json!({"channel":"Scratch","slot":0,"plugin":"Phase Meter","bypassed":true}),
        )
        .unwrap();
        assert!(verify(&bypass, &json!({"after":true})));
        assert!(!verify(&bypass, &json!({"after":false})));
        assert!(!verify(&bypass, &json!({"after":null})));
    }

    #[test]
    fn splitter_verification_requires_matching_native_raw_readback() {
        let operation = Operation::from_parts(
            "setSplitter",
            json!({"channel":"Scratch","node":"Node01","label":"Splitter 1","param":"gain[1]","value":0.5}),
        ).unwrap();
        assert!(verify(&operation, &json!({"after":{"value":0.5}})));
        for result in [
            json!({"after":{"value":0.6}}),
            json!({"requested":0.5}),
            json!({"after":{"normalized":0.5}}),
            json!({"after":{"value":"0.5"}}),
            json!({"after":{"value":null}}),
        ] {
            assert!(!verify(&operation, &result));
        }
        let mute = Operation::from_parts(
            "setSplitter",
            json!({"channel":"Scratch","node":"Node01","label":"Splitter 1","param":"mute[1]","value":1}),
        ).unwrap();
        assert!(verify(&mute, &json!({"after":{"value":true}})));
        assert!(!verify(&mute, &json!({"after":{"value":false}})));
    }

    #[test]
    fn a_part_with_some_failed_notes_is_not_a_clean_success() {
        let result = json!({"results":[{"op":"createPart","part":{"name":"Part"},"added":2,"errors":["third note failed"]}]});
        assert_eq!(mutation_outcome(&result), Outcome::PartiallyApplied);
    }

    #[test]
    fn a_failed_task_does_not_prove_that_no_edit_happened() {
        let result = json!({"results":[{"op":"addInstrumentTrack","error":"connection failed after creation"}]});
        assert_eq!(mutation_outcome(&result), Outcome::Unknown);
    }
}

#[cfg(test)]
mod stereo_send_set_contract {
    use super::*;

    #[test]
    fn accepts_only_closed_instance_destination_and_exact_independent_proof() {
        let id = "AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE";
        let make = |args| Operation::from_parts("external_stereo_send_set", args);
        let request = json!({
            "expected_instance":id, "expected_current":"none", "destination":"main",
            "channel":"Audio Track", "slot":1
        });
        let operation = make(request.clone()).unwrap();
        operation.validate().unwrap();
        assert!(operation.requires_explicit_document() && !operation.is_read_only());
        let mut bad = request.clone();
        bad["path"] = json!([1]);
        assert!(make(bad).is_err());
        let mut bad = request.clone();
        bad["destination"] = json!("return");
        assert!(make(bad).is_err());
        let mut bad = request.clone();
        bad["expected_current"] = json!("main");
        assert!(make(bad).unwrap().validate().is_err());
        let mut bad = request;
        bad["slot"] = json!(16);
        assert!(make(bad).unwrap().validate().is_err());
        let before = json!({
            "name":"sendPort", "tag":102, "native_kind":"List", "value":-1, "normalized":1,
            "native_value":{"variant_tag":1,"data":{"kind":"integer","value":-1}},
            "native_normalized":{"kind":"float32","bits_hex":"3F800000"}, "text":"없음"
        });
        let after = before
            .as_object()
            .unwrap()
            .clone()
            .into_iter()
            .map(|(k, v)| {
                let value = if k == "text" { json!("메인") } else { v };
                (k, value)
            })
            .collect::<serde_json::Map<String, Value>>();
        let result = json!({
            "ok":true,"operation":"external_stereo_send_set","native_outcome":"applied_unverified",
            "channel":"Audio Track","slot":1,"expected_current":"none","destination":"main","assigned":true,
            "selection_calls":1,"controller_rows":24,"fixture_after_matches":true,
            "native_checkmark_verified":true,"atomic_snapshot":false,"persistence_verified":false,"audio_verified":false,
            "diagnostic":{"version":"stereo_send_v1","failure_phase":null,"dispatch_attempted":true,
                          "menu_callbacks_began":true,"select_entered":true,"select_returned":true,"cleanup_proven":true},
            "instance":{"class_uid":"889DD9F8-14F0-44A5-BCEE-0F2B157FC5D4","object_id":"External Effect Stereo 2",
                        "native_class":"AudioEffect","object_uid":id},
            "before":before,"after":after,
        });
        assert!(verify(&operation, &result));
        let mut wrong = result.clone();
        wrong["after"]["text"] = json!("없음");
        assert!(!verify(&operation, &wrong));
        let mut wrong = result.clone();
        wrong["native_checkmark_verified"] = json!(false);
        assert!(!verify(&operation, &wrong));
        let mut wrong = result.clone();
        wrong["fixture_after_matches"] = json!(false);
        assert!(!verify(&operation, &wrong));
        for (field, value) in [
            ("assigned", json!(false)),
            ("audio_verified", json!(true)),
            ("persistence_verified", json!(true)),
            ("channel", json!("Other Track")),
            ("slot", json!(0)),
            ("expected_current", json!("main")),
        ] {
            let mut wrong = result.clone();
            wrong[field] = value;
            assert!(!verify(&operation, &wrong), "{field}");
        }
    }
}

#[cfg(test)]
mod stereo_send_set_failure_contract {
    use super::*;

    #[test]
    fn all_fixed_phases_roundtrip_without_raw_host_values() {
        for phase in STEREO_SEND_CHANGE_PHASES {
            let trace = json!({"version":"stereo_send_v1", "failure_phase":phase, "dispatch_attempted":true,
                "menu_callbacks_began":true,"select_entered":true,"select_returned":true,"cleanup_proven":false});
            let result = json!({"ok":false,"native_outcome":"unknown","raw_path":"/user/private","diagnostic":trace});
            let outcome = normalize_stereo_send_set_failure(&result);
            assert_eq!(outcome["native_outcome"], "unknown");
            assert_eq!(outcome["diagnostic"], trace);
            assert!(!outcome.to_string().contains("/user/private"));
        }
    }

    #[test]
    fn provenance_cannot_turn_ambiguity_into_known_refusal() {
        let before = json!({"ok":false,"native_outcome":"not_applied","diagnostic":{
            "version":"stereo_send_v1","failure_phase":"native_admission","dispatch_attempted":true,"menu_callbacks_began":false,
            "select_entered":false,"select_returned":false,"cleanup_proven":false}});
        assert_eq!(
            normalize_stereo_send_set_failure(&before)["native_outcome"],
            "not_applied"
        );
        for (key, value) in [
            ("failure_phase", json!("/private/path")),
            ("select_returned", json!(true)),
            ("dispatch_attempted", Value::Null),
        ] {
            let mut malformed = before.clone();
            malformed["diagnostic"][key] = value;
            let sanitized = normalize_stereo_send_set_failure(&malformed);
            assert_eq!(sanitized["native_outcome"], "unknown");
            assert_eq!(sanitized["diagnostic"]["failure_phase"], "receipt");
            assert!(sanitized["diagnostic"]["select_entered"].is_null());
        }
        let mut after_callback = before;
        after_callback["diagnostic"]["failure_phase"] = json!("check_only");
        after_callback["diagnostic"]["menu_callbacks_began"] = json!(true);
        let sanitized = normalize_stereo_send_set_failure(&after_callback);
        assert_eq!(sanitized["native_outcome"], "unknown");
        assert_eq!(sanitized["diagnostic"]["failure_phase"], "check_only");
        assert_eq!(sanitized["diagnostic"]["select_entered"], false);
    }
}

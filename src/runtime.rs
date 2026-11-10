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
    operation::{Operation, Request},
};

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
        request.operation.validate()?;
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
        Ok(response) => return Ok(rejected(request_id, started, host_error(&response), None)),
        Err(failure) => return Ok(rejected(request_id, started, failure.error, None)),
    };
    let context: HostContext = serde_json::from_value(context_response.result.clone())
        .map_err(|error| OperationError::new("invalid_context", error.to_string()))?;
    if request
        .expected_document
        .as_ref()
        .is_some_and(|expected| *expected != context.document)
    {
        return Ok(rejected(
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
    let (op, args) = operation.wire();
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
            return Ok(Receipt {
                request_id,
                outcome: if !read_only && failure.dispatched {
                    Outcome::Unknown
                } else {
                    Outcome::NotApplied
                },
                session: Some(context.session),
                document: Some(context.document),
                result: Value::Null,
                error: Some(failure.error),
                elapsed_ms: started.elapsed().as_millis(),
            });
        }
    };
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
                        return Ok(rejected(request_id, started, failure.error, Some(&context)));
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
    if response.document != context.document {
        invalidate_gui_evidence(&operation, &mut response.result);
        return Ok(Receipt {
            request_id,
            outcome: if read_only {
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
    let soft_error = response.result.get("error").filter(|v| !v.is_null());
    let mut error = if !response.ok {
        Some(host_error(&response))
    } else {
        soft_error.map(|error| OperationError::new("host_rejected", error.to_string()))
    };
    let outcome = if let Some(error) = &error {
        if read_only
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
    } else if read_only {
        Outcome::Observed
    } else {
        let outcome = mutation_outcome(&response.result);
        if outcome == Outcome::AppliedUnverified && verify(&operation, &response.result) {
            Outcome::AppliedVerified
        } else {
            outcome
        }
    };
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
        invalidate_gui_evidence(&operation, &mut response.result);
    }
    Ok(Receipt {
        request_id,
        outcome,
        session: Some(response.session),
        document: Some(response.document),
        result: response.result,
        error,
        elapsed_ms: started.elapsed().as_millis(),
    })
}

fn exact_gui_object(value: &Value, fields: &[&str]) -> bool {
    value.as_object().is_some_and(|object| {
        object.len() == fields.len() && fields.iter().all(|field| object.contains_key(*field))
    })
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

fn invalidate_gui_evidence(operation: &Operation, result: &mut Value) {
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

fn verify(operation: &Operation, result: &Value) -> bool {
    match operation {
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
    request_id: String,
    started: Instant,
    error: OperationError,
    context: Option<&HostContext>,
) -> Receipt {
    Receipt {
        request_id,
        outcome: Outcome::NotApplied,
        session: context.map(|c| c.session.clone()),
        document: context.map(|c| c.document.clone()),
        result: Value::Null,
        error: Some(error),
        elapsed_ms: started.elapsed().as_millis(),
    }
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

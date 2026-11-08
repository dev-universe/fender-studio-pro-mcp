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
            _ => None,
        };
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

fn mutation_outcome(value: &Value) -> Outcome {
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
        Operation::SetPluginParam(change) => {
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
    fn readback_comparison_handles_host_boolean_numbers() {
        assert!(equivalent(&json!(true), &json!(1)));
        assert!(!equivalent(&json!(true), &json!(0)));
        assert!(equivalent(&json!(0.5), &json!(0.50000001)));
        assert!(!equivalent(&json!(0.5), &json!(0.6)));
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

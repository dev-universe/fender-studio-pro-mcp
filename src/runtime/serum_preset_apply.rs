use std::time::Instant;

use serde_json::{Value, json};

use crate::{
    bridge::{Bridge, WireResponse},
    midi::MidiWakeup,
    model::{HostContext, OperationError, Outcome, Receipt},
    serum::PresetApply,
};

pub(super) fn execute(
    bridge: &mut Bridge,
    midi: &mut MidiWakeup,
    request: &PresetApply,
    context: &HostContext,
    request_id: String,
    started: Instant,
) -> Receipt {
    let receipt = |outcome, result, error| Receipt {
        request_id,
        outcome,
        session: Some(context.session.clone()),
        document: Some(context.document.clone()),
        result,
        error,
        elapsed_ms: started.elapsed().as_millis(),
    };
    if !context.open {
        return receipt(
            Outcome::NotApplied,
            Value::Null,
            Some(OperationError::new(
                "document_required",
                "Open a Studio Pro song before applying a Serum preset",
            )),
        );
    }
    let staged = match request.read_preset().and_then(|bytes| {
        bridge.stage_serum_preset(&bytes).map_err(|_| {
            OperationError::new(
                "serum_preset_stage_failed",
                "Could not stage the Serum preset in the private mailbox",
            )
        })
    }) {
        Ok(staged) => staged,
        Err(error) => return receipt(Outcome::NotApplied, Value::Null, Some(error)),
    };
    let reply = bridge.transact(
        "serum2PresetApply",
        &json!({
            "target": request.target,
            "expected_instance": request.expected_instance,
            "sha256": request.expected_sha256,
            "fixture": staged.basename,
        }),
        Some(&context.session),
        Some(&context.document),
        midi,
    );
    match reply {
        Ok(reply) if applied(request, context, &reply) => {
            staged.remove();
            receipt(
                Outcome::AppliedUnverified,
                json!({
                    "ok":true, "native_outcome":"applied_unverified", "verified":false,
                    "plugin":request.target.variant().plugin_name(), "sha256":request.expected_sha256,
                    "instance":{
                        "object_uid":request.expected_instance, "class_uid":request.target.variant().class_uid(),
                    },
                }),
                None,
            )
        }
        Ok(reply) if refused(context, &reply) => {
            let message = refusal_message(&reply);
            staged.remove();
            receipt(
                Outcome::NotApplied,
                json!({"ok":false,"native_outcome":"not_applied"}),
                Some(OperationError::new("serum2_preset_apply_failed", message)),
            )
        }
        Err(failure) if !failure.dispatched => {
            staged.remove();
            receipt(
                Outcome::NotApplied,
                Value::Null,
                Some(OperationError::new(
                    "serum_preset_dispatch_failed",
                    "The Serum preset request could not be published to the bridge",
                )),
            )
        }
        // The host may still be reading the fixture. Keep it in the private
        // mailbox and let the caller inspect the target before retrying.
        _ => receipt(
            Outcome::Unknown,
            json!({
                "ok":false,"native_outcome":"unknown","fixture":staged.basename,
                "sha256":request.expected_sha256,
            }),
            Some(OperationError::new(
                "outcome_unknown",
                "The Serum preset application could not be confirmed. Read the target before retrying",
            )),
        ),
    }
}

fn matches_context(context: &HostContext, reply: &WireResponse) -> bool {
    reply.session == context.session && reply.document == context.document
}

fn applied(request: &PresetApply, context: &HostContext, reply: &WireResponse) -> bool {
    let result = &reply.result;
    matches_context(context, reply)
        && reply.ok
        && reply.error.is_none()
        && reply.code.is_none()
        && result["ok"] == true
        && result["native_outcome"] == "applied_unverified"
        && result["verified"] == false
        && result["sha256"] == request.expected_sha256
        && result["plugin"] == request.target.variant().plugin_name()
        && result["instance"]["object_uid"] == request.expected_instance
        && result["instance"]["class_uid"] == request.target.variant().class_uid()
}

fn refused(context: &HostContext, reply: &WireResponse) -> bool {
    if !matches_context(context, reply) {
        return false;
    }
    if !reply.ok {
        return matches!(
            reply.code.as_deref(),
            Some("protocol_mismatch" | "context_required" | "missing_precondition")
        );
    }
    let result = &reply.result;
    reply.error.is_none()
        && reply.code.is_none()
        && result["ok"] == false
        && result["native_outcome"] == "not_applied"
        && result["errors"].as_array().is_some_and(|errors| {
            errors.len() == 1
                && errors[0]["code"] == "serum2_preset_apply_failed"
                && errors[0]["message"].is_string()
        })
}

fn refusal_message(reply: &WireResponse) -> &'static str {
    match reply.result["errors"][0]["message"].as_str() {
        Some("Stop playback and recording before loading a Serum preset") => {
            "Stop playback and recording before loading a Serum preset"
        }
        Some("The matching control surface is required") => {
            "Install or reload the matching Studio Pro MCP control surface"
        }
        Some("The requested Serum target is unavailable") => {
            "The requested Serum target is unavailable"
        }
        _ => "The bridge refused the Serum preset before applying it",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{bridge::PROTOCOL, serum::PresetTarget};

    fn context() -> HostContext {
        HostContext {
            session: "session".into(),
            document: "session:3".into(),
            open: true,
            title: None,
            file_url: None,
            protocol: PROTOCOL,
        }
    }

    fn envelope(context: &HostContext, result: Value) -> WireResponse {
        WireResponse {
            protocol: PROTOCOL,
            id: "test".into(),
            session: context.session.clone(),
            document: context.document.clone(),
            ok: true,
            result,
            error: None,
            code: None,
        }
    }

    #[test]
    fn accepts_only_the_expected_document_digest_variant_and_native_instance() {
        let request = PresetApply {
            target: PresetTarget::Instrument {
                track: "Keys".into(),
            },
            expected_instance: "DF52AB13-A21E-4EDA-855C-CA68851BACA2".into(),
            path: "/private/test.vstpreset".into(),
            expected_sha256: "a".repeat(64),
        };
        let context = context();
        let variant = request.target.variant();
        let result = json!({
            "ok":true,"native_outcome":"applied_unverified","verified":false,
            "plugin":variant.plugin_name(),"sha256":request.expected_sha256,
            "instance":{"object_uid":request.expected_instance,"class_uid":variant.class_uid()}
        });
        let mut valid = envelope(&context, result.clone());
        assert!(applied(&request, &context, &valid));
        valid.document = "session:4".into();
        assert!(!applied(&request, &context, &valid));
        for (key, wrong) in [
            ("ok", json!(false)),
            ("native_outcome", json!("applied_verified")),
            ("verified", json!(true)),
            ("plugin", json!("Serum 2 FX")),
            ("sha256", json!("b".repeat(64))),
            (
                "instance",
                json!({"object_uid":"peer","class_uid":variant.class_uid()}),
            ),
            (
                "instance",
                json!({"object_uid":request.expected_instance,"class_uid":"different"}),
            ),
        ] {
            let mut reply = envelope(&context, result.clone());
            reply.result[key] = wrong;
            assert!(!applied(&request, &context, &reply), "{key}");
            assert!(!refused(&context, &reply), "{key}");
        }
    }

    #[test]
    fn distinguishes_explicit_refusal_from_unknown_without_echoing_private_errors() {
        let context = context();
        let mut reply = envelope(
            &context,
            json!({
                "ok":false,"native_outcome":"not_applied",
                "errors":[{"code":"serum2_preset_apply_failed","message":"/private/test.vstpreset"}]
            }),
        );
        assert!(refused(&context, &reply));
        assert_eq!(
            refusal_message(&reply),
            "The bridge refused the Serum preset before applying it"
        );
        reply.result["native_outcome"] = json!("unknown");
        assert!(!refused(&context, &reply));
        reply.result = Value::Null;
        reply.ok = false;
        reply.code = Some("host_rejected".into());
        assert!(!refused(&context, &reply));
        reply.code = Some("missing_precondition".into());
        assert!(refused(&context, &reply));
        reply.session = "another-session".into();
        assert!(!refused(&context, &reply));
    }
}

use super::{WireResponse, exact_gui_object};
use serde_json::{Value, json};

pub(super) fn normalize_result(response: &mut WireResponse, index: Option<u32>) {
    if !response.ok {
        response.result = Value::Null;
    } else if !valid(&response.result, index) {
        response.result = unknown();
    } else if let Some(failure) = bounded_failure(&response.result) {
        response.result = failure;
    }
}

fn valid(value: &Value, index: Option<u32>) -> bool {
    valid_failure(value)
        || (exact_gui_object(
            value,
            &[
                "action",
                "index",
                "plugin",
                "cv_preset_settings",
                "ok",
                "native_outcome",
                "load_accepted",
                "verified",
            ],
        ) && value["ok"] == true
            && value["action"] == "apply_cv_preset_settings"
            && index.map(u64::from) == value["index"].as_u64()
            && index.is_some()
            && value["plugin"] == "CV Instrument"
            && value["native_outcome"] == "applied_unverified"
            && value["load_accepted"] == true
            && value["verified"] == false
            && valid_settings(&value["cv_preset_settings"]))
}

fn valid_settings(value: &Value) -> bool {
    exact_gui_object(value, &["byte_count", "sha256"])
        && value["byte_count"]
            .as_u64()
            .is_some_and(|count| (1..=262144).contains(&count))
        && value["sha256"].as_str().is_some_and(|hash| {
            hash.len() == 64
                && hash
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
}

fn valid_failure(value: &Value) -> bool {
    exact_gui_object(value, &["ok", "native_outcome", "errors"])
        && value["ok"] == false
        && matches!(
            value["native_outcome"].as_str(),
            Some("not_applied" | "unknown")
        )
        && value["errors"].as_array().is_some_and(|errors| {
            errors.len() == 1
                && exact_gui_object(&errors[0], &["code", "message"])
                && matches!(
                    errors[0]["code"].as_str(),
                    Some("cv_preset_settings_apply_failed" | "cv_preset_settings_context_changed")
                )
                && errors[0]["message"]
                    .as_str()
                    .is_some_and(|message| !message.is_empty() && message.len() <= 2048)
        })
}

pub(super) fn refused(response: &WireResponse) -> bool {
    if response.ok {
        valid_failure(&response.result) && response.result["native_outcome"] == "not_applied"
    } else {
        matches!(
            response.code.as_deref(),
            Some(
                "document_changed"
                    | "session_changed"
                    | "context_required"
                    | "protocol_mismatch"
                    | "missing_precondition"
            )
        )
    }
}

fn bounded_failure(value: &Value) -> Option<Value> {
    if !valid_failure(value) {
        return None;
    }
    let outcome = if value["native_outcome"] == "not_applied" {
        "not_applied"
    } else {
        "unknown"
    };
    let (code, message) = if value["errors"][0]["code"] == "cv_preset_settings_context_changed" {
        (
            "cv_preset_settings_context_changed",
            "The CV preset settings document lease changed before publication",
        )
    } else if outcome == "not_applied" {
        (
            "cv_preset_settings_apply_failed",
            "CV preset settings were refused before loading",
        )
    } else {
        (
            "cv_preset_settings_apply_failed",
            "The CV preset settings load outcome could not be established",
        )
    };
    Some(json!({"ok":false,"native_outcome":outcome,"errors":[{"code":code,"message":message}]}))
}

fn unknown() -> Value {
    json!({"ok":false,"native_outcome":"unknown","errors":[{
        "code":"cv_preset_settings_apply_failed",
        "message":"The CV preset settings load outcome could not be established"
    }]})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_apply_failure_projection_never_publishes_transport_error_text() {
        let private = "/private/mock/path.preset";
        for outcome in ["not_applied", "unknown"] {
            for code in [
                "cv_preset_settings_apply_failed",
                "cv_preset_settings_context_changed",
            ] {
                let value = json!({"ok":false,"native_outcome":outcome,"errors":[{"code":code,"message":private}]});
                let safe = bounded_failure(&value).unwrap();
                assert_eq!(safe["native_outcome"], outcome);
                assert_eq!(safe["errors"][0]["code"], code);
                assert!(!safe.to_string().contains(private));
            }
        }
    }

    #[test]
    fn failed_wire_reply_discards_its_result_even_with_an_expected_index() {
        let mut response = WireResponse {
            protocol: 2,
            id: "id".into(),
            session: "session".into(),
            document: "session:1".into(),
            ok: false,
            result: json!({"index":7,"path":"/private/mock/path.preset"}),
            error: Some("A failed wire request".into()),
            code: Some("document_changed".into()),
        };
        normalize_result(&mut response, Some(7));
        assert!(response.result.is_null());
        assert!(refused(&response));
    }

    #[test]
    fn accepts_only_the_bound_unverified_load_receipt_or_a_shaped_failure() {
        let accepted = json!({
            "action":"apply_cv_preset_settings","index":7,"plugin":"CV Instrument",
            "cv_preset_settings":{"byte_count":3991,"sha256":"a".repeat(64)},
            "ok":true,"native_outcome":"applied_unverified","load_accepted":true,"verified":false,
        });
        assert!(valid(&accepted, Some(7)));
        assert!(!valid(&accepted, Some(8)));
        assert!(!valid(&accepted, None));
        for (key, value) in [
            ("verified", json!(true)),
            ("load_accepted", json!(false)),
            ("native_outcome", json!("observed")),
            ("raw_bytes", json!("ignored")),
        ] {
            let mut invalid = accepted.clone();
            invalid[key] = value;
            assert!(!valid(&invalid, Some(7)), "{key}");
        }
        assert!(valid(&unknown(), Some(7)));
        let mut response = WireResponse {
            protocol: 2,
            id: "id".into(),
            session: "new".into(),
            document: "new:2".into(),
            ok: true,
            result: unknown(),
            error: None,
            code: None,
        };
        assert!(!refused(&response));
        response.result["native_outcome"] = json!("not_applied");
        assert!(refused(&response));
        response.result["cv_preset_settings"] = accepted["cv_preset_settings"].clone();
        assert!(!refused(&response));
        response.ok = false;
        response.code = Some("document_changed".into());
        assert!(refused(&response));
        response.code = Some("host_rejected".into());
        assert!(!refused(&response));
    }
}

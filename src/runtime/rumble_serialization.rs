use super::*;
use crate::operation::{PluginState, PluginStateAction};

pub(super) fn requested(operation: &Operation) -> bool {
    matches!(operation, Operation::PluginState(query) if matches!(query.action, PluginStateAction::InspectRumbleSerialization))
}
pub(super) fn invalidate(value: &mut Value) {
    *value = json!({"ok":false,"outcome":"not_applied","native_outcome":"not_applied","mutation_capability":false,
        "model_identity_verified":false,"audio_verified":false,"persistence_verified":false,
        "error":{"code":"rumble_serialization_unavailable","message":"The bounded Rumble serialization observation was not established"}});
}
pub(super) fn valid_get(query: &PluginState, value: &Value) -> bool {
    let fields = [
        "ok",
        "outcome",
        "native_outcome",
        "operation",
        "schema",
        "serialization",
        "instance",
        "source",
        "capture_sha256",
        "capture_byte_count",
        "format",
        "parse_complete",
        "reason",
        "root_kind",
        "counts",
        "value_semantics",
        "mutation_capability",
        "model_identity_verified",
        "audio_verified",
        "persistence_verified",
        "plugin",
    ];
    if !exact_gui_object(value, &fields)
        || value.to_string().len() > 4096
        || value["ok"] != true
        || value["outcome"] != "observed"
        || value["native_outcome"] != "observed"
        || value["operation"] != "rumble_serialization_inspect"
        || value["schema"] != "rumble_alias_serialization_summary_v1"
        || value["serialization"] != "component_alias_IStorable"
        || value["source"] != "component_alias"
        || value["plugin"] != "Rumble Native"
        || value["value_semantics"] != "sequential_snapshot"
        || [
            "mutation_capability",
            "model_identity_verified",
            "audio_verified",
            "persistence_verified",
        ]
        .iter()
        .any(|key| value[*key] != false)
        || !exact_gui_object(
            &value["instance"],
            &["object_uid", "class_uid", "native_class", "object_id"],
        )
        || value["instance"]["object_uid"] != query.expected_instance
        || value["instance"]["class_uid"] != "B9D7536C-A010-457D-A182-36E5531164C9"
        || value["instance"]["native_class"] != "AudioEffect"
        || value["instance"]["object_id"] != "Rumble Native"
        || !value["capture_sha256"].as_str().is_some_and(|s| {
            s.len() == 64
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
        || value["capture_byte_count"]
            .as_u64()
            .is_none_or(|n| n > 16 * 1024 * 1024)
    {
        return false;
    }
    let format = value["format"].as_str().unwrap_or("");
    if value["parse_complete"] == false {
        if !value["root_kind"].is_null() || !value["counts"].is_null() {
            return false;
        }
        let reason = value["reason"].as_str().unwrap_or("");
        return match format {
            "utf8_json" => matches!(
                reason,
                "syntax" | "duplicate_key" | "trailing_document" | "limit" | "invalid_utf8"
            ),
            "utf8_xml" => matches!(
                reason,
                "syntax"
                    | "duplicate_attribute"
                    | "unsupported_xml"
                    | "invalid_utf8"
                    | "trailing_document"
                    | "limit"
            ),
            "zip_signature_only" => reason == "unsupported_container",
            "opaque" => matches!(reason, "opaque" | "invalid_utf8" | "invalid_bom" | "limit"),
            _ => false,
        };
    }
    if value["parse_complete"] != true || !value["reason"].is_null() {
        return false;
    }
    if !match format {
        "utf8_json" => matches!(
            value["root_kind"].as_str(),
            Some("json_object" | "json_array" | "json_primitive")
        ),
        "utf8_xml" => matches!(
            value["root_kind"].as_str(),
            Some(
                "audio_effect_preset"
                    | "music_effect_preset"
                    | "attributes"
                    | "attribute"
                    | "other_xml"
            )
        ),
        _ => false,
    } {
        return false;
    }
    let counts = &value["counts"];
    if !exact_gui_object(
        counts,
        &[
            "known_tags",
            "other_tags",
            "known_ids",
            "classids_total",
            "classids_valid_guid",
            "classids_zero_guid",
            "classids_invalid",
        ],
    ) {
        return false;
    }
    let mut units = 0;
    for (key, len) in [("known_tags", 4), ("known_ids", 30)] {
        let Some(items) = counts[key].as_array().filter(|a| a.len() == len) else {
            return false;
        };
        for item in items {
            let Some(n) = item.as_u64().filter(|n| *n <= 8192) else {
                return false;
            };
            units += n;
        }
    }
    for key in [
        "other_tags",
        "classids_total",
        "classids_valid_guid",
        "classids_zero_guid",
        "classids_invalid",
    ] {
        if counts[key].as_u64().is_none_or(|n| n > 8192) {
            return false;
        }
    }
    units += counts["other_tags"].as_u64().unwrap() + counts["classids_total"].as_u64().unwrap();
    let root = value["root_kind"].as_str().unwrap_or("");
    let root_count = match root {
        "audio_effect_preset" => Some(0),
        "music_effect_preset" => Some(1),
        "attributes" => Some(2),
        "attribute" => Some(3),
        _ => None,
    };
    if root_count.is_some_and(|index| counts["known_tags"][index].as_u64().unwrap() == 0)
        || root == "other_xml" && counts["other_tags"] == 0
        || root == "json_primitive" && units != 0
    {
        return false;
    }
    units <= 8192
        && counts["classids_total"].as_u64().unwrap()
            == counts["classids_valid_guid"].as_u64().unwrap()
                + counts["classids_invalid"].as_u64().unwrap()
        && counts["classids_zero_guid"].as_u64() <= counts["classids_valid_guid"].as_u64()
        && (format != "utf8_json"
            || (counts["other_tags"] == 0
                && counts["known_tags"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|n| *n == 0)))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn query() -> PluginState {
        serde_json::from_value(json!({"channel":"Scratch","slot":0,"plugin":"Rumble Native","expected_instance":"12345678-1234-1234-1234-123456789ABC","action":"inspect_rumble_serialization"})).unwrap()
    }
    fn receipt() -> Value {
        json!({"ok":true,"outcome":"observed","native_outcome":"observed","operation":"rumble_serialization_inspect","schema":"rumble_alias_serialization_summary_v1","serialization":"component_alias_IStorable","instance":{"object_uid":query().expected_instance,"class_uid":"B9D7536C-A010-457D-A182-36E5531164C9","native_class":"AudioEffect","object_id":"Rumble Native"},"source":"component_alias","capture_sha256":"a".repeat(64),"capture_byte_count":20,"format":"utf8_json","parse_complete":true,"reason":null,"root_kind":"json_object","counts":{"known_tags":vec![0;4],"other_tags":0,"known_ids":vec![0;30],"classids_total":0,"classids_valid_guid":0,"classids_zero_guid":0,"classids_invalid":0},"value_semantics":"sequential_snapshot","mutation_capability":false,"model_identity_verified":false,"audio_verified":false,"persistence_verified":false,"plugin":"Rumble Native"})
    }
    fn no_positive(value: &Value) {
        for key in [
            "counts",
            "format",
            "root_kind",
            "instance",
            "schema",
            "capture_sha256",
            "capture_byte_count",
            "source",
            "token",
        ] {
            assert!(value.get(key).is_none(), "{key}");
        }
        assert_eq!(value["ok"], false);
    }
    #[test]
    fn rumble_serialization_receipt_admits_complete_and_honest_unsupported() {
        assert!(valid_get(&query(), &receipt()));
        let mut value = receipt();
        value["format"] = json!("zip_signature_only");
        value["parse_complete"] = json!(false);
        value["reason"] = json!("unsupported_container");
        value["root_kind"] = Value::Null;
        value["counts"] = Value::Null;
        assert!(valid_get(&query(), &value));
    }
    #[test]
    fn rumble_serialization_forged_and_late_results_erase_all_positive_fields() {
        for (key, bad) in [
            ("model_identity_verified", json!(true)),
            ("capture_sha256", json!("secret")),
            ("counts", json!({})),
            ("parse_complete", json!(null)),
            ("format", json!("malformed")),
            ("reason", json!("secret")),
            ("root_kind", json!("private-root")),
            ("token", json!("private")),
            ("instance", json!({})),
            ("capture_byte_count", json!(16777217)),
            ("plugin", json!("Other")),
        ] {
            let mut value = receipt();
            value[key] = bad;
            assert!(!valid_get(&query(), &value), "{key}");
            invalidate(&mut value);
            no_positive(&value);
        }
        let operation = Operation::PluginState(query());
        let mut value = receipt();
        super::super::invalidate_gui_evidence(&operation, &mut value);
        no_positive(&value);
        let mut value = receipt();
        value["counts"]["classids_zero_guid"] = json!(1);
        assert!(!valid_get(&query(), &value));
    }
    #[test]
    fn rumble_serialization_document_guard_precedes_owner_queue() {
        use std::{
            future::Future,
            task::{Context, Poll, Waker},
        };
        for expected_document in [None, Some(String::new())] {
            let request = Request {
                operation: Operation::PluginState(query()),
                expected_document,
            };
            let (sender, receiver) = mpsc::sync_channel(1);
            let handle = RuntimeHandle { sender };
            let mut future = std::pin::pin!(handle.execute(request));
            assert!(matches!(
                future
                    .as_mut()
                    .poll(&mut Context::from_waker(Waker::noop())),
                Poll::Ready(Err(_))
            ));
            assert!(receiver.try_recv().is_err());
        }
    }
}

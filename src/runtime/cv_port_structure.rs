use super::*;
use crate::operation::{PluginState, PluginStateAction};

const FALSE_FIELDS: &[&str] = &[
    "assignment_values_observed",
    "routing_verified",
    "selection_authority",
    "can_set_via_this_action",
    "mutation_capability",
    "atomic_snapshot",
    "audio_verified",
    "persistence_verified",
];
pub(super) fn requested(operation: &Operation) -> bool {
    matches!(operation, Operation::PluginState(query) if matches!(query.action, PluginStateAction::InspectCvPortStructure))
}
fn integer(v: &Value, min: i64, max: i64) -> bool {
    v.as_i64().is_some_and(|n| (min..=max).contains(&n))
}
fn hex(v: &Value, length: usize) -> bool {
    v.as_str().is_some_and(|s| {
        s.len() == length
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b))
    })
}
fn text(v: &Value, cap: usize) -> bool {
    v.as_str()
        .is_some_and(|s| s.encode_utf16().count() <= cap && !s.contains('\0'))
}
fn typed(v: &Value) -> bool {
    if v.is_null() {
        return true;
    }
    if !exact_gui_object(v, &["variant_tag", "data"]) || !integer(&v["variant_tag"], -32768, 32767)
    {
        return false;
    }
    let d = &v["data"];
    let tag = v["variant_tag"].as_i64().unwrap_or(-1) & 15;
    let coherent = match d["kind"].as_str() {
        Some("empty") => tag == 0,
        Some("integer") => tag == 1,
        Some("float" | "non_finite_float") => tag == 2,
        Some("text") => tag == 3,
        Some("opaque_object") => tag == 4,
        Some("unsupported") => tag > 4 && d["tag"] == tag,
        _ => false,
    };
    if !coherent {
        return false;
    }
    match d["kind"].as_str() {
        Some("empty") => exact_gui_object(d, &["kind"]),
        Some("integer") => {
            if exact_gui_object(d, &["kind", "value"]) {
                d["value"]
                    .as_i64()
                    .is_some_and(|n| n.abs_diff(0) <= 9_007_199_254_740_991)
            } else {
                exact_gui_object(d, &["kind", "exact_decimal"])
                    && d["exact_decimal"].as_str().is_some_and(|s| {
                        s.parse::<i64>().is_ok_and(|n| {
                            n.to_string() == s && n.abs_diff(0) > 9_007_199_254_740_991
                        })
                    })
            }
        }
        Some("float") => {
            exact_gui_object(d, &["kind", "value", "bits_hex"])
                && hex(&d["bits_hex"], 16)
                && d["value"].as_f64().is_some_and(|n| {
                    n.is_finite()
                        && u64::from_str_radix(d["bits_hex"].as_str().unwrap_or(""), 16)
                            .is_ok_and(|bits| n.to_bits() == bits)
                })
        }
        Some("non_finite_float") => {
            exact_gui_object(d, &["kind", "bits_hex"])
                && hex(&d["bits_hex"], 16)
                && u64::from_str_radix(d["bits_hex"].as_str().unwrap_or(""), 16)
                    .is_ok_and(|bits| !f64::from_bits(bits).is_finite())
        }
        Some("text") => exact_gui_object(d, &["kind", "value"]) && text(&d["value"], 4096),
        Some("opaque_object") => {
            exact_gui_object(d, &["kind", "present"]) && d["present"].is_boolean()
        }
        Some("unsupported") => {
            exact_gui_object(d, &["kind", "tag"]) && integer(&d["tag"], -32768, 32767)
        }
        _ => false,
    }
}
fn parameter(v: &Value, index: usize) -> bool {
    exact_gui_object(
        v,
        &[
            "index",
            "name",
            "tag",
            "kind",
            "enabled",
            "read_only",
            "value",
            "min",
            "max",
            "default",
            "normalized",
            "display",
        ],
    ) && v["index"] == index
        && v["name"]
            .as_str()
            .is_some_and(|s| s.len() <= 512 && !s.contains('\0'))
        && integer(&v["tag"], i32::MIN as i64, i32::MAX as i64)
        && v["kind"].as_str().is_some_and(|kind| {
            [
                "Toggle",
                "Integer",
                "Float",
                "String",
                "List",
                "Command",
                "Color",
                "Image",
                "Segments",
                "Scroll",
                "TextModel",
            ]
            .contains(&kind)
                || kind
                    .strip_prefix("Unknown(")
                    .and_then(|s| s.strip_suffix(')'))
                    .is_some_and(|s| s.parse::<i32>().is_ok())
        })
        && v["enabled"].is_boolean()
        && v["read_only"].is_boolean()
        && text(&v["display"], 4096)
        && ["value", "min", "max", "default"]
            .iter()
            .all(|key| typed(&v[*key]))
        && match v["normalized"]["kind"].as_str() {
            Some("not_applicable") => exact_gui_object(&v["normalized"], &["kind"]),
            Some(kind @ ("float32" | "non_finite_float32")) => {
                exact_gui_object(&v["normalized"], &["kind", "bits_hex"])
                    && hex(&v["normalized"]["bits_hex"], 8)
                    && u32::from_str_radix(v["normalized"]["bits_hex"].as_str().unwrap_or(""), 16)
                        .is_ok_and(|bits| f32::from_bits(bits).is_finite() == (kind == "float32"))
            }
            _ => false,
        }
}
fn text_bytes(v: &Value) -> usize {
    v["name"].as_str().map_or(0, str::len)
        + v["display"].as_str().map_or(0, str::len)
        + ["value", "min", "max", "default"]
            .iter()
            .map(|key| {
                if v[*key]["data"]["kind"] == "text" {
                    v[*key]["data"]["value"].as_str().map_or(0, str::len)
                } else {
                    0
                }
            })
            .sum::<usize>()
}
pub(super) fn valid_get(query: &PluginState, value: &Value) -> bool {
    let mut fields = vec![
        "ok",
        "outcome",
        "native_outcome",
        "plugin",
        "operation",
        "source",
        "instance",
        "scope",
        "phase",
        "prepare_calls",
        "cleanup_calls",
        "cleanup_returned",
        "cleanup_elapsed_ms",
        "parent_before",
        "parent_after",
        "parent_details",
        "child_count",
        "children",
        "observation_complete",
        "status",
        "retained_text_bytes",
        "limits",
    ];
    fields.extend_from_slice(FALSE_FIELDS);
    if !exact_gui_object(value, &fields)
        || value.to_string().len() > 262144
        || value["ok"] != true
        || value["outcome"] != "observed"
        || value["native_outcome"] != "observed"
        || value["plugin"] != "CV Instrument"
        || value["operation"] != "cv_port_structure_inspect"
        || value["source"] != "dsp"
        || value["scope"] != "fixed_output01_direct_nested_parameters"
        || value["phase"] != "closed_checked"
        || value["prepare_calls"] != 1
        || value["cleanup_calls"] != 1
        || value["cleanup_returned"] != true
        || value["cleanup_elapsed_ms"]
            .as_u64()
            .is_none_or(|ms| ms > 2000)
        || FALSE_FIELDS.iter().any(|key| value[*key] != false)
        || !exact_gui_object(
            &value["instance"],
            &["object_uid", "class_uid", "native_class", "object_id"],
        )
        || value["instance"]["object_uid"] != query.expected_instance
        || value["instance"]["class_uid"] != "EDCD22A5-A01E-4EA6-9FE4-2BB1AB243E85"
        || value["instance"]["native_class"] != "AudioSynth"
        || value["instance"]["object_id"] != "CV Instrument"
        || value["limits"]
            != json!({"deadline_ms":2000,"parents":1,"depth":1,"children":32,"name_bytes":512,"text_utf16_units":4096,"total_child_text_bytes":32768,"encoded_response_bytes":262144})
        || !parameter(&value["parent_before"], 0)
        || value["parent_before"] != value["parent_after"]
        || value["parent_before"]["name"] != "portAssignment"
        || value["parent_before"]["tag"] != 100
        || value["parent_before"]["kind"] != "List"
        || !exact_gui_object(
            &value["parent_details"],
            &[
                "precision",
                "storable",
                "mutable",
                "store_list_value",
                "read_only",
                "can_undo",
                "list_interface",
                "structured_interface",
            ],
        )
        || !integer(
            &value["parent_details"]["precision"],
            i32::MIN as i64,
            i32::MAX as i64,
        )
        || value["parent_details"]["structured_interface"] != true
        || [
            "storable",
            "mutable",
            "store_list_value",
            "read_only",
            "can_undo",
            "list_interface",
        ]
        .iter()
        .any(|key| !value["parent_details"][*key].is_boolean())
    {
        return false;
    }
    let Some(count) = value["child_count"]
        .as_i64()
        .filter(|n| *n >= 0 && *n <= i32::MAX as i64)
    else {
        return false;
    };
    let Some(children) = value["children"].as_array() else {
        return false;
    };
    if count > 32 {
        return children.is_empty()
            && value["status"] == "child_limit"
            && value["observation_complete"] == false
            && value["retained_text_bytes"] == 0;
    }
    children.len() == count as usize
        && value["status"] == "complete"
        && value["observation_complete"] == true
        && children
            .iter()
            .enumerate()
            .all(|(index, child)| parameter(child, index))
        && children.iter().map(text_bytes).sum::<usize>() <= 32768
        && value["retained_text_bytes"] == children.iter().map(text_bytes).sum::<usize>()
}
pub(super) fn failure_outcome(value: &Value) -> Outcome {
    if value["ok"] == false
        && value["outcome"] == "not_applied"
        && value["native_outcome"] == "not_applied"
        && value["phase"] == "before_prepare"
        && exact_gui_object(&value["error"], &["code", "message"])
        && value.as_object().is_some_and(|object| {
            object.keys().all(|key| {
                [
                    "ok",
                    "outcome",
                    "native_outcome",
                    "plugin",
                    "phase",
                    "error",
                ]
                .contains(&key.as_str())
            })
        })
        && text(&value["error"]["code"], 64)
        && text(&value["error"]["message"], 512)
    {
        Outcome::NotApplied
    } else {
        Outcome::Unknown
    }
}
pub(super) fn invalidate(value: &mut Value, preserve_refusal: bool) {
    let refusal = preserve_refusal && failure_outcome(value) == Outcome::NotApplied;
    let outcome = if refusal { "not_applied" } else { "unknown" };
    *value = json!({"ok":false,"outcome":outcome,"native_outcome":outcome,"phase":if refusal {"before_prepare"} else {"unknown"},"error":{"code":"cv_structure_unconfirmed","message":"The exact closed CV structure observation was not established"}});
}

#[cfg(test)]
mod tests;

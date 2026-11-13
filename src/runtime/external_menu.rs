use super::*;
use crate::operation::{ExternalPort, ExternalPortMenu};

const NESTED_CALLBACKS: &str = "not counted; prepare/build/extension/cleanup may reenter";

fn uid(value: &Value) -> bool {
    value.as_str().is_some_and(|text| {
        text.len() == 36
            && text != "00000000-0000-0000-0000-000000000000"
            && text.bytes().enumerate().all(|(i, b)| {
                if [8, 13, 18, 23].contains(&i) {
                    b == b'-'
                } else {
                    b.is_ascii_digit() || (b'A'..=b'F').contains(&b)
                }
            })
    })
}

pub(super) fn valid_get(query: &ExternalPortMenu, value: &Value) -> bool {
    let (name, tag, port) = match query.port {
        ExternalPort::Send => ("sendPort", 102, "send"),
        ExternalPort::Return => ("receivePort", 103, "return"),
    };
    let class = match query.plugin.as_str() {
        "External Effect Mono" => "873281D9-58DE-4B2D-B3BD-528F549B1AD9",
        "External Effect Stereo" => "889DD9F8-14F0-44A5-BCEE-0F2B157FC5D4",
        _ => return false,
    };
    if !exact_gui_object(
        value,
        &[
            "ok",
            "outcome",
            "native_outcome",
            "operation",
            "instance",
            "source",
            "port",
            "parameter",
            "menu_class",
            "rows",
            "coverage",
            "menu_phase",
            "lifecycle",
            "selection_authority",
            "selection_calls",
            "scope",
            "persistence_verified",
            "audio_verified",
            "value_semantics",
            "plugin",
        ],
    ) || value["ok"] != true
        || value["outcome"] != "observed"
        || value["native_outcome"] != "observed"
        || value["operation"] != "external_port_menu"
        || value["plugin"] != query.plugin
        || value["source"] != "controller"
        || value["port"] != port
        || value["menu_phase"] != "closed_checked"
        || value["scope"] != "exact_external_parameter_menu_read"
        || value["selection_authority"] != false
        || value["selection_calls"] != 0
        || value["persistence_verified"] != false
        || value["audio_verified"] != false
        || value["value_semantics"] != "sequential_menu_materialization"
        || !uid(&value["menu_class"])
        || !exact_gui_object(
            &value["instance"],
            &["object_uid", "class_uid", "native_class", "object_id"],
        )
        || value["instance"]["object_uid"] != query.expected_instance
        || !uid(&value["instance"]["object_uid"])
        || value["instance"]["class_uid"] != class
        || value["instance"]["native_class"] != "AudioEffect"
        || value["instance"]["object_id"] != query.plugin
        || value["parameter"] != json!({"name":name,"tag":tag})
        || value["lifecycle"]
            != json!({"construct_calls":1,"build_calls":1,"owned_menu_references_released":true,"root_menu_final_release_count":0,"builder_final_release_count":0,"post_cleanup_binding_checked":true,"nested_callbacks":NESTED_CALLBACKS})
    {
        return false;
    }
    let Some(rows) = value["rows"].as_array().filter(|r| r.len() <= 256) else {
        return false;
    };
    if value["coverage"]
        != json!({"complete":true,"max_rows":256,"max_depth":4,"max_total_text_bytes":32768,"max_text_bytes_each":1024,"row_count":rows.len()})
    {
        return false;
    }
    let mut parents = std::collections::BTreeMap::<Vec<u64>, (u64, bool)>::new();
    parents.insert(vec![], (0, true));
    let mut previous = Vec::new();
    let mut text_bytes = 0usize;
    for row in rows {
        if !exact_gui_object(
            row,
            &[
                "path",
                "name",
                "title",
                "category",
                "checked",
                "enabled",
                "has_submenu",
                "handler_present",
                "kind",
            ],
        ) {
            return false;
        }
        let Some(path) = row["path"]
            .as_array()
            .filter(|p| (1..=4).contains(&p.len()))
        else {
            return false;
        };
        let Some(path) = path.iter().map(Value::as_u64).collect::<Option<Vec<_>>>() else {
            return false;
        };
        if path.iter().any(|n| *n > 255) || (!previous.is_empty() && path <= previous) {
            return false;
        }
        let Some((next, submenu)) = parents.get_mut(&path[..path.len() - 1]) else {
            return false;
        };
        if !*submenu || path[path.len() - 1] != *next {
            return false;
        }
        *next += 1;
        let Some(submenu) = row["has_submenu"].as_bool() else {
            return false;
        };
        if row["kind"] != if submenu { "submenu" } else { "opaque" } {
            return false;
        }
        parents.insert(path.clone(), (0, submenu));
        previous = path;
        for key in ["checked", "enabled", "handler_present"] {
            if !row[key].is_null() && !row[key].is_boolean() {
                return false;
            }
        }
        for key in ["name", "title", "category"] {
            if row[key].is_null() {
                continue;
            }
            let Some(text) = row[key].as_str().filter(|s| s.len() <= 1024) else {
                return false;
            };
            text_bytes += text.len();
            if text_bytes > 32768 {
                return false;
            }
        }
    }
    true
}

pub(super) fn failure_outcome(value: &Value) -> Outcome {
    let refusal = value["ok"] == false
        && value["native_outcome"] == "not_applied"
        && value["outcome"] == "not_applied"
        && exact_gui_object(&value["error"], &["code", "message"])
        && value["error"]["code"]
            .as_str()
            .is_some_and(|s| !s.is_empty() && s.len() <= 64)
        && value["error"]["message"]
            .as_str()
            .is_some_and(|s| s.encode_utf16().count() <= 512)
        && value.as_object().is_some_and(|o| {
            o.keys().all(|k| {
                matches!(
                    k.as_str(),
                    "ok" | "outcome" | "native_outcome" | "plugin" | "error" | "menu_phase"
                )
            })
        })
        && value.get("menu_phase").is_none_or(|v| v == "prepared");
    if refusal {
        Outcome::NotApplied
    } else {
        Outcome::Unknown
    }
}

pub(super) fn invalidate(value: &mut Value, preserve_refusal: bool) {
    let outcome = if preserve_refusal && failure_outcome(value) == Outcome::NotApplied {
        "not_applied"
    } else {
        "unknown"
    };
    *value = json!({"ok":false,"outcome":outcome,"native_outcome":outcome,
        "error":{"code":"external_menu_unconfirmed","message":"The exact External menu lifecycle observation was not established"}});
}

#[cfg(test)]
mod tests;

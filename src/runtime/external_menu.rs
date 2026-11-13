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
    let mut fields = vec![
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
    ];
    if query.provenance {
        fields.push("provenance");
    }
    if query.include_check_only {
        fields.push("check_only");
    }
    if !exact_gui_object(value, &fields)
        || value["ok"] != true
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
    if query.provenance
        && (query.plugin != "External Effect Stereo"
            || !matches!(query.port, ExternalPort::Send)
            || !valid_provenance(&value["provenance"], rows))
    {
        return false;
    }
    if query.include_check_only
        && (!query.provenance
            || query.plugin != "External Effect Stereo"
            || !matches!(query.port, ExternalPort::Send)
            || !valid_check_only(&value["check_only"], &value["provenance"]))
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

fn valid_provenance(value: &Value, rows: &[Value]) -> bool {
    if !exact_gui_object(
        value,
        &[
            "leaves",
            "menu_data_typed_values_observed",
            "selection_authority",
            "selection_calls",
            "proof_scope",
            "read_only_intent",
            "atomic_snapshot",
            "continuous_active_mixer_membership_verified",
        ],
    ) || value["menu_data_typed_values_observed"] != false
        || value["proof_scope"] != "bracketed_current_mapped_bank_sequential_read_only"
        || value["read_only_intent"] != true
        || value["atomic_snapshot"] != false
        || value["continuous_active_mixer_membership_verified"] != false
        || value["selection_authority"] != false
        || value["selection_calls"] != 0
        || rows.len() != 4
    {
        return false;
    }
    let Some(leaves) = value["leaves"].as_array().filter(|a| a.len() == 2) else {
        return false;
    };
    for i in 2..4 {
        let r = &rows[i];
        if r["path"] != json!([i])
            || r["name"] != if i == 2 { "" } else { "Setup" }
            || r["title"] != if i == 2 { "-" } else { "오디오 I/O 설정" }
            || r["category"] != if i == 2 { "" } else { "Console" }
            || r["checked"] != false
            || r["enabled"] != true
            || r["handler_present"] != (i == 3)
            || r["has_submenu"] != false
        {
            return false;
        }
    }
    for (i, p) in leaves.iter().enumerate() {
        let r = &rows[i];
        if r["path"] != json!([i])
            || r["name"] != i.to_string()
            || r["title"] != if i == 0 { "없음" } else { "메인 - 1 + 2" }
            || r["category"] != "Param"
            || r["checked"] != (i == 0)
            || r["enabled"] != true
            || r["handler_present"] != true
            || r["has_submenu"] != false
            || !exact_gui_object(
                p,
                &[
                    "path",
                    "handler_matches_builder",
                    "data_handler_matches_builder",
                    "data_builder_matches_builder",
                    "data_parameter_is_selected",
                    "original_is_selected",
                    "original_end",
                ],
            )
            || p["path"] != json!([i])
            || [
                "handler_matches_builder",
                "data_handler_matches_builder",
                "data_builder_matches_builder",
                "data_parameter_is_selected",
            ]
            .iter()
            .any(|k| !p[k].is_boolean())
        {
            return false;
        }
        let Some(chain) = p["original_is_selected"]
            .as_array()
            .filter(|a| (1..=8).contains(&a.len()) && a.iter().all(Value::is_boolean))
        else {
            return false;
        };
        let end = &p["original_end"];
        let valid = match end["kind"].as_str() {
            Some("cycle") => {
                exact_gui_object(end, &["kind", "index"])
                    && end["index"]
                        .as_u64()
                        .is_some_and(|n| n < chain.len().saturating_sub(1) as u64)
            }
            Some("limit") => exact_gui_object(end, &["kind"]) && chain.len() == 8,
            Some("self_reference" | "missing_original") => {
                exact_gui_object(end, &["kind"]) && chain.len() < 8
            }
            _ => false,
        };
        if !valid {
            return false;
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

fn valid_check_only(value: &Value, provenance: &Value) -> bool {
    if !exact_gui_object(
        value,
        &[
            "leaves",
            "flags",
            "category_query_calls",
            "check_only_calls",
            "selection_calls",
            "execution_calls",
            "scope",
            "read_only_intent",
            "atomic_snapshot",
            "global_command_filter_checked",
            "menu_data_typed_values_observed",
        ],
    ) || value["flags"] != 1
        || value["category_query_calls"] != 2
        || value["check_only_calls"] != 2
        || value["selection_calls"] != 0
        || value["execution_calls"] != 0
        || value["scope"] != "fixed_stereo_send_public_check_only"
        || value["read_only_intent"] != true
        || value["atomic_snapshot"] != false
        || value["global_command_filter_checked"] != false
        || value["menu_data_typed_values_observed"] != false
    {
        return false;
    }
    let Some(leaves) = value["leaves"].as_array().filter(|v| v.len() == 2) else {
        return false;
    };
    let Some(proofs) = provenance["leaves"].as_array().filter(|v| v.len() == 2) else {
        return false;
    };
    leaves.iter().zip(proofs).enumerate().all(|(i, (leaf, p))| {
        exact_gui_object(leaf, &["path", "raw_category", "raw_check_only"])
            && leaf["path"] == json!([i])
            && leaf["raw_category"]
                .as_u64()
                .is_some_and(|n| (1..=255).contains(&n))
            && leaf["raw_check_only"].as_u64().is_some_and(|n| n <= 255)
            && [
                "handler_matches_builder",
                "data_handler_matches_builder",
                "data_builder_matches_builder",
                "data_parameter_is_selected",
            ]
            .iter()
            .all(|k| p[*k] == true)
            && p["original_is_selected"] == json!([true])
            && p["original_end"] == json!({"kind":"self_reference"})
    })
}

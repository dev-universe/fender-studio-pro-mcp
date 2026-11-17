use super::rumble_parameters::{description, guid, integer};
use super::*;
use crate::operation::{PluginState, PluginStateAction};
use sha2::{Digest, Sha256};

pub(super) fn requested(operation: &Operation) -> bool {
    matches!(operation, Operation::PluginState(q) if matches!(q.action, PluginStateAction::ReadRumbleNamedParameters))
}

pub(super) fn invalidate(value: &mut Value) {
    *value = json!({"ok":false,"outcome":"not_applied","native_outcome":"not_applied","mutation_capability":false,"can_set_via_this_action":false,"selection_authority":false,"atomic_snapshot":false,"audio_verified":false,"persistence_verified":false,"error":{"code":"rumble_named_parameters_unavailable","message":"The bounded Rumble named-parameter observation was not established"}});
}

pub(super) fn valid_get(query: &PluginState, value: &Value) -> bool {
    if !exact_gui_object(
        value,
        &[
            "ok",
            "outcome",
            "operation",
            "schema",
            "instance",
            "source",
            "scope",
            "controllers",
            "models_sha256",
            "before_after_model_ids_equal",
            "before_after_name_sets_equal",
            "name_set_complete",
            "controller_counts_complete",
            "named_metadata_complete",
            "access_metadata_complete",
            "current_values_complete",
            "expected_name_count",
            "matched_name_count",
            "value_semantics",
            "mutation_capability",
            "can_set_via_this_action",
            "selection_authority",
            "atomic_snapshot",
            "audio_verified",
            "persistence_verified",
            "plugin",
            "native_outcome",
        ],
    ) || value.to_string().len() > 262144
        || value["ok"] != true
        || value["outcome"] != "observed"
        || value["native_outcome"] != "observed"
        || value["operation"] != "rumble_named_parameters_read"
        || value["schema"] != "rumble_named_parameters_v1"
        || value["plugin"] != "Rumble Native"
        || value["source"] != "component_alias_fixed_controllers"
        || value["scope"] != "fixed_immediate_serialized_sibling_names"
        || value["before_after_model_ids_equal"] != true
        || value["before_after_name_sets_equal"] != true
        || value["name_set_complete"] != true
        || value["value_semantics"] != "sequential_snapshot"
        || [
            "mutation_capability",
            "can_set_via_this_action",
            "selection_authority",
            "atomic_snapshot",
            "audio_verified",
            "persistence_verified",
        ]
        .iter()
        .any(|k| value[*k] != false)
        || !exact_gui_object(
            &value["instance"],
            &["object_uid", "class_uid", "native_class", "object_id"],
        )
        || value["instance"]["object_uid"] != query.expected_instance
        || value["instance"]["class_uid"] != "B9D7536C-A010-457D-A182-36E5531164C9"
        || value["instance"]["native_class"] != "AudioEffect"
        || value["instance"]["object_id"] != "Rumble Native"
    {
        return false;
    }
    validate_rows(value).is_some()
}

fn validate_rows(value: &Value) -> Option<()> {
    let rows = value["controllers"].as_array()?;
    if rows.len() != 26 {
        return None;
    }
    let mut models = b"fsp.rumble.models.v1\0".to_vec();
    let mut expected = 0usize;
    let mut matched = 0usize;
    let mut counts_complete = true;
    let mut metadata_complete = true;
    let mut currents_complete = true;
    for (index, row) in rows.iter().enumerate() {
        let slot = if index == 25 {
            "amp".to_owned()
        } else {
            format!("effect{}", index + 1)
        };
        if !exact_gui_object(
            row,
            &[
                "slot",
                "serialized_class_uid",
                "status",
                "parameter_count",
                "expected_names",
                "parameters",
            ],
        ) || row["slot"] != slot
        {
            return None;
        }
        models.push(slot.len() as u8);
        models.extend_from_slice(slot.as_bytes());
        models.extend(guid(&row["serialized_class_uid"])?);
        let names = row["expected_names"].as_array()?;
        if names.len() > 64 {
            return None;
        }
        let mut previous = "";
        for name in names {
            let name = name.as_str()?;
            let bytes = name.as_bytes();
            if bytes.is_empty()
                || bytes.len() > 64
                || !bytes[0].is_ascii_alphabetic()
                || !bytes
                    .iter()
                    .all(|b| b.is_ascii_alphanumeric() || *b == b'_')
                || name <= previous
            {
                return None;
            }
            previous = name;
        }
        expected += names.len();
        if expected > 256 {
            return None;
        }
        if row["status"] != "present" {
            if !matches!(
                row["status"].as_str(),
                Some(
                    "node_absent"
                        | "controller_absent"
                        | "qi_failed"
                        | "malformed_success"
                        | "malformed_absence"
                        | "identity_mismatch"
                        | "negative_count"
                        | "count_limit"
                )
            ) || !row["parameter_count"].is_null()
                || !row["parameters"].is_null()
            {
                return None;
            }
            counts_complete = false;
            metadata_complete = false;
            continue;
        }
        if !integer(&row["parameter_count"], 0, 128) {
            return None;
        }
        let parameters = row["parameters"].as_array()?;
        if parameters.len() != names.len() {
            return None;
        }
        for (param, name) in parameters.iter().zip(names) {
            if !exact_gui_object(param, &["name", "status", "description"])
                || param["name"] != *name
            {
                return None;
            }
            if param["status"] == "observed" {
                if param["description"]["name"] != *name {
                    return None;
                }
                currents_complete &= description(&param["description"])?;
                matched += 1;
            } else {
                if !matches!(
                    param["status"].as_str(),
                    Some("parameter_absent" | "description_unavailable" | "unsupported_metadata")
                ) || !param["description"].is_null()
                {
                    return None;
                }
                metadata_complete = false;
            }
        }
    }
    if value["models_sha256"] != format!("{:x}", Sha256::digest(models))
        || value["expected_name_count"].as_u64() != Some(expected as u64)
        || value["matched_name_count"].as_u64() != Some(matched as u64)
        || value["controller_counts_complete"] != counts_complete
        || value["named_metadata_complete"] != metadata_complete
        || value["access_metadata_complete"] != metadata_complete
        || value["current_values_complete"] != (metadata_complete && currents_complete)
    {
        return None;
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn query() -> PluginState {
        serde_json::from_value(json!({"channel":"Scratch","slot":0,"plugin":"Rumble Native","expected_instance":"12345678-1234-1234-1234-123456789ABC","action":"read_rumble_named_parameters"})).unwrap()
    }
    fn receipt() -> Value {
        let rows=(0..26).map(|i|json!({"slot":if i==25 {"amp".to_owned()}else{format!("effect{}",i+1)},"serialized_class_uid":"00000000-0000-0000-0000-000000000000","status":"present","parameter_count":0,"expected_names":[],"parameters":[]})).collect::<Vec<_>>();
        json!({"ok":true,"outcome":"observed","native_outcome":"observed","operation":"rumble_named_parameters_read","schema":"rumble_named_parameters_v1","instance":{"object_uid":query().expected_instance,"class_uid":"B9D7536C-A010-457D-A182-36E5531164C9","native_class":"AudioEffect","object_id":"Rumble Native"},"source":"component_alias_fixed_controllers","scope":"fixed_immediate_serialized_sibling_names","controllers":rows,"models_sha256":"a7995d1e57ccc8f7983c4c45a2c6e8a0931cdda5617d1a7453d34d716302269b","before_after_model_ids_equal":true,"before_after_name_sets_equal":true,"name_set_complete":true,"controller_counts_complete":true,"named_metadata_complete":true,"access_metadata_complete":true,"current_values_complete":true,"expected_name_count":0,"matched_name_count":0,"value_semantics":"sequential_snapshot","mutation_capability":false,"can_set_via_this_action":false,"selection_authority":false,"atomic_snapshot":false,"audio_verified":false,"persistence_verified":false,"plugin":"Rumble Native"})
    }
    fn descriptor() -> Value {
        let number = json!({"status":"observed","variant_tag":1,"kind":"integer","decimal":"9223372036854775807"});
        json!({"name":"gain","tag":7,"kind":1,"enabled":true,"read_only":false,"current":number,"min":number,"max":number,"default":number,"normalized":{"status":"observed","bits_hex":"40000000"},"support":{"raw_number":true,"normalized":true}})
    }
    fn positive() -> Value {
        let mut value = receipt();
        value["controllers"][0]["expected_names"] = json!(["gain"]);
        value["controllers"][0]["parameters"] =
            json!([{"name":"gain","status":"observed","description":descriptor()}]);
        value["expected_name_count"] = json!(1);
        value["matched_name_count"] = json!(1);
        value
    }
    fn incomplete(value: &mut Value) {
        for key in [
            "named_metadata_complete",
            "access_metadata_complete",
            "current_values_complete",
        ] {
            value[key] = json!(false);
        }
    }
    fn set_names(value: &mut Value, index: usize, names: Vec<String>) {
        value["controllers"][index]["parameters"] = json!(
            names
                .iter()
                .map(|name| json!({"name":name,"status":"parameter_absent","description":null}))
                .collect::<Vec<_>>()
        );
        value["controllers"][index]["expected_names"] = json!(names);
        value["expected_name_count"] = json!(
            value["controllers"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r["expected_names"].as_array().unwrap().len())
                .sum::<usize>()
        );
        incomplete(value);
    }
    #[test]
    fn zero_count_and_large_indexed_aggregate_do_not_suppress_named_descriptors() {
        assert!(valid_get(&query(), &receipt()));
        let mut value = positive();
        assert!(valid_get(&query(), &value));
        for row in value["controllers"].as_array_mut().unwrap() {
            row["parameter_count"] = json!(128);
        }
        assert!(valid_get(&query(), &value));
    }
    #[test]
    fn unavailable_controllers_and_named_statuses_have_independent_completeness() {
        for status in [
            "node_absent",
            "controller_absent",
            "qi_failed",
            "malformed_success",
            "malformed_absence",
            "identity_mismatch",
            "negative_count",
            "count_limit",
        ] {
            let mut value = positive();
            let row = &mut value["controllers"][0];
            row["status"] = json!(status);
            row["parameters"] = Value::Null;
            row["parameter_count"] = Value::Null;
            value["controller_counts_complete"] = json!(false);
            value["matched_name_count"] = json!(0);
            incomplete(&mut value);
            assert!(valid_get(&query(), &value), "{status}");
            value["controllers"][0]["parameter_count"] = json!(0);
            assert!(!valid_get(&query(), &value));
        }
        for status in [
            "parameter_absent",
            "description_unavailable",
            "unsupported_metadata",
        ] {
            let mut value = positive();
            value["controllers"][0]["parameters"][0]["status"] = json!(status);
            value["controllers"][0]["parameters"][0]["description"] = Value::Null;
            value["matched_name_count"] = json!(0);
            incomplete(&mut value);
            assert!(valid_get(&query(), &value));
            value["named_metadata_complete"] = json!(true);
            assert!(!valid_get(&query(), &value));
        }
    }
    #[test]
    fn strict_names_and_exact_order_reject_entire_unsupported_domains() {
        for names in [
            vec!["".to_owned()],
            vec!["a".repeat(65)],
            vec!["gain.x".to_owned()],
            vec!["gain/x".to_owned()],
            vec!["1gain".to_owned()],
            vec!["gain\n".to_owned()],
            vec!["é".to_owned()],
            vec!["gain".to_owned(), "gain".to_owned()],
            vec!["b".to_owned(), "a".to_owned()],
            (0..65).map(|i| format!("n{i:02}")).collect(),
        ] {
            let mut value = receipt();
            set_names(&mut value, 0, names);
            assert!(!valid_get(&query(), &value));
        }
        let mut value = receipt();
        set_names(&mut value, 0, vec!["a".repeat(64)]);
        assert!(valid_get(&query(), &value));
        for i in 0..4 {
            set_names(&mut value, i, (0..64).map(|j| format!("n{j:02}")).collect());
        }
        assert!(valid_get(&query(), &value));
        set_names(&mut value, 4, vec!["extra".to_owned()]);
        assert!(!valid_get(&query(), &value));
        let mut value = positive();
        value["controllers"][0]["parameters"][0]["name"] = json!("other");
        assert!(!valid_get(&query(), &value));
    }
    #[test]
    fn derived_counts_model_hash_closed_shapes_and_privacy_are_verified() {
        for key in [
            "controller_counts_complete",
            "named_metadata_complete",
            "access_metadata_complete",
            "current_values_complete",
            "before_after_name_sets_equal",
            "name_set_complete",
            "before_after_model_ids_equal",
        ] {
            let mut value = positive();
            value[key] = json!(false);
            assert!(!valid_get(&query(), &value), "{key}");
        }
        for key in ["expected_name_count", "matched_name_count"] {
            let mut value = positive();
            value[key] = json!(2);
            assert!(!valid_get(&query(), &value));
        }
        for key in [
            "token",
            "source_path",
            "value",
            "display",
            "formatter",
            "pointer",
        ] {
            let mut value = positive();
            value["controllers"][0]["parameters"][0]["description"][key] = json!("PRIVATE");
            assert!(!valid_get(&query(), &value));
        }
        let mut value = positive();
        value["controllers"][0]["serialized_class_uid"] =
            json!("10000000-0000-0000-0000-000000000000");
        assert!(!valid_get(&query(), &value));
        let mut value = positive();
        value["controllers"].as_array_mut().unwrap().swap(0, 1);
        assert!(!valid_get(&query(), &value));
        let mut value = positive();
        value["controllers"][0]["parameters"][0]["description"]["name"] = json!("other");
        assert!(!valid_get(&query(), &value));
        let mut value = positive();
        value["controllers"][0]["parameters"][0]["index"] = json!(0);
        assert!(!valid_get(&query(), &value));
    }
    #[test]
    fn precision_list_minus_one_missing_default_and_support_policy_are_preserved() {
        for current in [
            json!({"status":"observed","variant_tag":1,"kind":"integer","decimal":"9007199254740993"}),
            json!({"status":"observed","variant_tag":2,"kind":"float64","bits_hex":"8000000000000000"}),
            json!({"status":"observed","variant_tag":1,"kind":"integer","decimal":"-1"}),
        ] {
            let mut value = positive();
            let d = &mut value["controllers"][0]["parameters"][0]["description"];
            d["kind"] = json!(4);
            d["current"] = current;
            d["default"] = json!({"status":"unavailable","variant_tag":null,"reason":"missing"});
            assert!(valid_get(&query(), &value));
        }
        let mut value = positive();
        let d = &mut value["controllers"][0]["parameters"][0]["description"];
        d["current"] = json!({"status":"unavailable","variant_tag":2,"reason":"non_finite"});
        d["support"] = json!({"raw_number":false,"normalized":false});
        value["current_values_complete"] = json!(false);
        assert!(valid_get(&query(), &value));
        for field in ["enabled", "read_only", "kind"] {
            let mut value = positive();
            value["controllers"][0]["parameters"][0]["description"][field] = if field == "kind" {
                json!(999)
            } else {
                json!(field == "read_only")
            };
            assert!(!valid_get(&query(), &value));
            value["controllers"][0]["parameters"][0]["description"]["support"] =
                json!({"raw_number":false,"normalized":false});
            assert!(valid_get(&query(), &value));
        }
    }
    #[test]
    fn oversized_adapted_results_fail_without_truncation() {
        let mut value = receipt();
        for i in 0..4 {
            let names = (0..64)
                .map(|j| format!("n{j:02}{}", "x".repeat(61)))
                .collect::<Vec<_>>();
            set_names(&mut value, i, names.clone());
            value["controllers"][i]["parameters"] = json!(
                names
                    .iter()
                    .map(|name| {
                        let mut d = descriptor();
                        d["name"] = json!(name);
                        json!({"name":name,"status":"observed","description":d})
                    })
                    .collect::<Vec<_>>()
            );
        }
        for key in [
            "named_metadata_complete",
            "access_metadata_complete",
            "current_values_complete",
        ] {
            value[key] = json!(true);
        }
        value["matched_name_count"] = json!(256);
        assert!(valid_get(&query(), &value));
        value["controllers"][0]["parameters"][0]["description"]["name"] =
            json!("PRIVATE".repeat(50000));
        assert!(value.to_string().len() > 262144);
        assert!(!valid_get(&query(), &value));
    }
    #[test]
    fn trusted_action_erases_forged_operation_schema_and_all_private_fields() {
        let operation = Operation::PluginState(query());
        assert!(requested(&operation));
        for discriminator in ["operation", "schema"] {
            let mut value = positive();
            value[discriminator] = json!("state_capture");
            value["token"] = json!("PRIVATE");
            assert!(!valid_get(&query(), &value));
            super::super::invalidate_gui_evidence(&operation, &mut value);
            assert_eq!(value.as_object().unwrap().len(), 10);
            assert_eq!(
                value["error"]["code"],
                "rumble_named_parameters_unavailable"
            );
            assert!(!value.to_string().contains("PRIVATE"));
        }
    }
}

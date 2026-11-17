use super::*;
use crate::operation::{PluginState, PluginStateAction};
use sha2::{Digest, Sha256};

pub(super) fn requested(operation: &Operation) -> bool {
    matches!(operation, Operation::PluginState(q) if matches!(q.action, PluginStateAction::ReadRumbleParameters))
}
pub(super) fn invalidate(value: &mut Value) {
    *value = json!({"ok":false,"outcome":"not_applied","native_outcome":"not_applied","mutation_capability":false,"can_set_via_this_action":false,"selection_authority":false,"atomic_snapshot":false,"audio_verified":false,"persistence_verified":false,"error":{"code":"rumble_controller_parameters_unavailable","message":"The bounded Rumble controller-parameter observation was not established"}});
}
pub(super) fn integer(value: &Value, min: i64, max: i64) -> bool {
    value.as_i64().is_some_and(|n| (min..=max).contains(&n))
}
fn hex(value: &Value, len: usize) -> bool {
    value.as_str().is_some_and(|s| {
        s.len() == len
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
fn number(value: &Value) -> Option<bool> {
    let tag = &value["variant_tag"];
    if value["status"] == "unavailable" {
        if !exact_gui_object(value, &["status", "variant_tag", "reason"]) {
            return None;
        }
        return match value["reason"].as_str()? {
            "missing" if tag.is_null() => Some(false),
            "not_numeric" | "non_finite" if integer(tag, -32768, 32767) => Some(false),
            _ => None,
        };
    }
    if value["status"] != "observed" || !integer(tag, -32768, 32767) {
        return None;
    }
    match value["kind"].as_str()? {
        "integer" if exact_gui_object(value, &["status", "variant_tag", "kind", "decimal"]) => {
            let s = value["decimal"].as_str()?;
            (s.parse::<i64>().ok()?.to_string() == s).then_some(true)
        }
        "float64"
            if exact_gui_object(value, &["status", "variant_tag", "kind", "bits_hex"])
                && hex(&value["bits_hex"], 16) =>
        {
            f64::from_bits(u64::from_str_radix(value["bits_hex"].as_str()?, 16).ok()?)
                .is_finite()
                .then_some(true)
        }
        _ => None,
    }
}
fn normalized(value: &Value) -> Option<bool> {
    if value["status"] == "observed"
        && exact_gui_object(value, &["status", "bits_hex"])
        && hex(&value["bits_hex"], 8)
    {
        return f32::from_bits(u32::from_str_radix(value["bits_hex"].as_str()?, 16).ok()?)
            .is_finite()
            .then_some(true);
    }
    (value["status"] == "unavailable"
        && exact_gui_object(value, &["status", "reason"])
        && matches!(
            value["reason"].as_str(),
            Some("not_applicable" | "non_finite")
        ))
    .then_some(false)
}
pub(super) fn description(value: &Value) -> Option<bool> {
    if !exact_gui_object(
        value,
        &[
            "name",
            "tag",
            "kind",
            "enabled",
            "read_only",
            "current",
            "min",
            "max",
            "default",
            "normalized",
            "support",
        ],
    ) || !value["name"]
        .as_str()
        .is_some_and(|s| !s.is_empty() && s.len() <= 128 && !s.contains('\0'))
        || !integer(&value["tag"], i32::MIN as i64, i32::MAX as i64)
        || !integer(&value["kind"], i32::MIN as i64, i32::MAX as i64)
        || value["enabled"].as_bool().is_none()
        || value["read_only"].as_bool().is_none()
        || !exact_gui_object(&value["support"], &["raw_number", "normalized"])
        || value["support"]["raw_number"].as_bool().is_none()
        || value["support"]["normalized"].as_bool().is_none()
    {
        return None;
    }
    let current = number(&value["current"])?;
    for k in ["min", "max", "default"] {
        number(&value[k])?;
    }
    let norm = normalized(&value["normalized"])?;
    let eligible = current
        && matches!(value["kind"].as_i64(), Some(0 | 1 | 2 | 4 | 9))
        && value["enabled"] == true
        && value["read_only"] == false;
    if value["support"]["raw_number"] == true && !eligible
        || value["support"]["normalized"] == true && !(eligible && norm)
    {
        return None;
    }
    Some(current)
}
pub(super) fn guid(value: &Value) -> Option<Vec<u8>> {
    let s = value.as_str()?;
    if s.len() != 36
        || s.bytes().enumerate().any(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b != b'-'
            } else {
                !b.is_ascii_digit() && !(b'A'..=b'F').contains(&b)
            }
        })
    {
        return None;
    }
    let compact = s.replace('-', "");
    (0..16)
        .map(|i| u8::from_str_radix(&compact[i * 2..i * 2 + 2], 16).ok())
        .collect()
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
            "census_complete",
            "parameter_lists_complete",
            "access_metadata_complete",
            "current_values_complete",
            "serialized_name_coverage_complete",
            "reported_parameter_count",
            "enumerated_parameter_count",
            "descriptor_budget_exceeded",
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
        || value["operation"] != "rumble_parameters_read"
        || value["schema"] != "rumble_controller_parameters_v1"
        || value["plugin"] != "Rumble Native"
        || value["source"] != "component_alias_fixed_controllers"
        || value["scope"] != "fixed_immediate_controller_parameter_lists"
        || value["before_after_model_ids_equal"] != true
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
    let mut reported = 0u64;
    let mut census = true;
    for (i, row) in rows.iter().enumerate() {
        let slot = if i == 25 {
            "amp".to_owned()
        } else {
            format!("effect{}", i + 1)
        };
        if !exact_gui_object(
            row,
            &[
                "slot",
                "serialized_class_uid",
                "status",
                "parameter_count",
                "enumeration_status",
                "parameters",
                "serialized_name_count",
                "matched_serialized_name_count",
            ],
        ) || row["slot"] != slot
            || !integer(&row["serialized_name_count"], 0, 8192)
            || !integer(
                &row["matched_serialized_name_count"],
                0,
                row["serialized_name_count"].as_i64()?,
            )
        {
            return None;
        }
        models.push(slot.len() as u8);
        models.extend_from_slice(slot.as_bytes());
        models.extend(guid(&row["serialized_class_uid"])?);
        if row["status"] == "present" {
            if !integer(&row["parameter_count"], 0, 128) {
                return None;
            }
            reported += row["parameter_count"].as_u64()?;
        } else {
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
                || row["enumeration_status"] != "unavailable"
                || row["matched_serialized_name_count"] != 0
            {
                return None;
            }
            census = false;
        }
    }
    let budget = reported > 256;
    let mut lists = census && !budget;
    let mut currents = true;
    let mut coverage = true;
    let mut enumerated = 0u64;
    for row in rows {
        if row["status"] != "present" {
            continue;
        }
        if budget {
            if !row["parameters"].is_null()
                || row["enumeration_status"] != "descriptor_total_limit"
                || row["matched_serialized_name_count"] != 0
            {
                return None;
            }
            continue;
        }
        let params = row["parameters"].as_array()?;
        if params.len() as u64 != row["parameter_count"].as_u64()? {
            return None;
        }
        let mut complete = true;
        let mut names = std::collections::HashSet::new();
        for (i, param) in params.iter().enumerate() {
            if !exact_gui_object(param, &["index", "status", "description"])
                || param["index"].as_u64() != Some(i as u64)
            {
                return None;
            }
            if param["status"] == "observed" {
                currents &= description(&param["description"])?;
                enumerated += 1;
                names.insert(param["description"]["name"].as_str()?);
            } else {
                if !matches!(
                    param["status"].as_str(),
                    Some("parameter_absent" | "description_unavailable" | "unsupported_metadata")
                ) || !param["description"].is_null()
                {
                    return None;
                }
                complete = false;
            }
        }
        if row["enumeration_status"] != if complete { "complete" } else { "incomplete" }
            || row["matched_serialized_name_count"].as_u64()? > names.len() as u64
        {
            return None;
        }
        lists &= complete;
        coverage &= row["matched_serialized_name_count"] == row["serialized_name_count"];
    }
    if enumerated > 256
        || value["models_sha256"] != format!("{:x}", Sha256::digest(models))
        || value["reported_parameter_count"].as_u64() != Some(reported)
        || value["enumerated_parameter_count"].as_u64() != Some(enumerated)
        || value["descriptor_budget_exceeded"] != budget
        || value["census_complete"] != census
        || value["parameter_lists_complete"] != lists
        || value["access_metadata_complete"] != lists
        || value["current_values_complete"] != (lists && currents)
        || value["serialized_name_coverage_complete"] != (lists && coverage)
    {
        return None;
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn query() -> PluginState {
        serde_json::from_value(json!({"channel":"Scratch","slot":0,"plugin":"Rumble Native","expected_instance":"12345678-1234-1234-1234-123456789ABC","action":"read_rumble_parameters"})).unwrap()
    }
    fn receipt() -> Value {
        let rows=(0..26).map(|i|json!({"slot":if i==25 {"amp".to_owned()}else{format!("effect{}",i+1)},"serialized_class_uid":"00000000-0000-0000-0000-000000000000","status":"present","parameter_count":0,"enumeration_status":"complete","parameters":[],"serialized_name_count":0,"matched_serialized_name_count":0})).collect::<Vec<_>>();
        json!({"ok":true,"outcome":"observed","native_outcome":"observed","operation":"rumble_parameters_read","schema":"rumble_controller_parameters_v1","instance":{"object_uid":query().expected_instance,"class_uid":"B9D7536C-A010-457D-A182-36E5531164C9","native_class":"AudioEffect","object_id":"Rumble Native"},"source":"component_alias_fixed_controllers","scope":"fixed_immediate_controller_parameter_lists","controllers":rows,"models_sha256":"a7995d1e57ccc8f7983c4c45a2c6e8a0931cdda5617d1a7453d34d716302269b","before_after_model_ids_equal":true,"census_complete":true,"parameter_lists_complete":true,"access_metadata_complete":true,"current_values_complete":true,"serialized_name_coverage_complete":true,"reported_parameter_count":0,"enumerated_parameter_count":0,"descriptor_budget_exceeded":false,"value_semantics":"sequential_snapshot","mutation_capability":false,"can_set_via_this_action":false,"selection_authority":false,"atomic_snapshot":false,"audio_verified":false,"persistence_verified":false,"plugin":"Rumble Native"})
    }
    fn description_fixture() -> Value {
        let number = json!({"status":"observed","variant_tag":1,"kind":"integer","decimal":"9223372036854775807"});
        json!({"name":"gain","tag":7,"kind":1,"enabled":true,"read_only":false,"current":number,"min":number,"max":number,"default":number,"normalized":{"status":"observed","bits_hex":"80000000"},"support":{"raw_number":true,"normalized":true}})
    }
    fn with_parameter() -> Value {
        let mut v = receipt();
        v["controllers"][0]["parameter_count"] = json!(1);
        v["controllers"][0]["parameters"] =
            json!([{"index":0,"status":"observed","description":description_fixture()}]);
        v["controllers"][0]["serialized_name_count"] = json!(1);
        v["controllers"][0]["matched_serialized_name_count"] = json!(1);
        v["reported_parameter_count"] = json!(1);
        v["enumerated_parameter_count"] = json!(1);
        v
    }
    fn incomplete(v: &mut Value) {
        for k in [
            "census_complete",
            "parameter_lists_complete",
            "access_metadata_complete",
            "current_values_complete",
            "serialized_name_coverage_complete",
        ] {
            v[k] = json!(false);
        }
    }
    #[test]
    fn accepts_zero_positive_partial_and_budget_census() {
        assert!(valid_get(&query(), &receipt()));
        assert!(valid_get(&query(), &with_parameter()));
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
            let mut v = receipt();
            let row = &mut v["controllers"][0];
            row["status"] = json!(status);
            row["parameter_count"] = Value::Null;
            row["parameters"] = Value::Null;
            row["enumeration_status"] = json!("unavailable");
            incomplete(&mut v);
            assert!(valid_get(&query(), &v), "{status}");
            v["controllers"][0]["parameter_count"] = json!(0);
            assert!(!valid_get(&query(), &v));
        }
        for status in [
            "parameter_absent",
            "description_unavailable",
            "unsupported_metadata",
        ] {
            let mut v = with_parameter();
            v["controllers"][0]["parameters"][0]["status"] = json!(status);
            v["controllers"][0]["parameters"][0]["description"] = Value::Null;
            v["controllers"][0]["enumeration_status"] = json!("incomplete");
            v["controllers"][0]["matched_serialized_name_count"] = json!(0);
            incomplete(&mut v);
            v["census_complete"] = json!(true);
            v["enumerated_parameter_count"] = json!(0);
            assert!(valid_get(&query(), &v));
        }
        let mut v = receipt();
        for i in 0..26 {
            v["controllers"][i]["parameter_count"] = json!(if i < 3 { 128 } else { 0 });
            v["controllers"][i]["parameters"] = Value::Null;
            v["controllers"][i]["enumeration_status"] = json!("descriptor_total_limit");
        }
        incomplete(&mut v);
        v["census_complete"] = json!(true);
        v["descriptor_budget_exceeded"] = json!(true);
        v["reported_parameter_count"] = json!(384);
        assert!(valid_get(&query(), &v));
        v["controllers"][25]["parameters"] = json!([]);
        assert!(!valid_get(&query(), &v));
    }
    #[test]
    fn numeric_carriers_preserve_integer_and_float_bits() {
        for decimal in [
            "0",
            "9007199254740993",
            "9223372036854775807",
            "-9223372036854775808",
        ] {
            assert_eq!(
                number(
                    &json!({"status":"observed","variant_tag":1,"kind":"integer","decimal":decimal})
                ),
                Some(true)
            );
        }
        for decimal in ["-0", "01", "9223372036854775808", "-9223372036854775809"] {
            assert!(
                number(
                    &json!({"status":"observed","variant_tag":1,"kind":"integer","decimal":decimal})
                )
                .is_none()
            );
        }
        assert_eq!(
            number(
                &json!({"status":"observed","variant_tag":2,"kind":"float64","bits_hex":"8000000000000000"})
            ),
            Some(true)
        );
        for bits in ["7ff0000000000000", "fff8000000000000"] {
            assert!(
                number(
                    &json!({"status":"observed","variant_tag":2,"kind":"float64","bits_hex":bits})
                )
                .is_none()
            );
        }
        let mut v = with_parameter();
        v["controllers"][0]["parameters"][0]["description"]["current"] =
            json!({"status":"unavailable","variant_tag":2,"reason":"non_finite"});
        v["controllers"][0]["parameters"][0]["description"]["support"] =
            json!({"raw_number":false,"normalized":false});
        v["current_values_complete"] = json!(false);
        assert!(valid_get(&query(), &v));
    }
    #[test]
    fn rejects_forged_shape_identity_digest_counts_and_metadata() {
        for (key, value) in [
            ("operation", json!("state_capture")),
            ("schema", json!("other")),
            ("models_sha256", json!("b".repeat(64))),
            ("atomic_snapshot", json!(true)),
            ("token", json!("PRIVATE")),
            ("reported_parameter_count", json!(1)),
            ("access_metadata_complete", json!(false)),
        ] {
            let mut v = receipt();
            v[key] = value;
            assert!(!valid_get(&query(), &v), "{key}");
        }
        for (key, value) in [
            ("kind", json!(999)),
            ("read_only", json!(true)),
            ("enabled", json!(false)),
            ("name", json!("x".repeat(129))),
            ("formatter", json!("PRIVATE")),
        ] {
            let mut v = with_parameter();
            v["controllers"][0]["parameters"][0]["description"][key] = value;
            assert!(!valid_get(&query(), &v), "{key}");
        }
        let mut v = receipt();
        v["controllers"][0]["serialized_class_uid"] = json!("11111111-1111-4111-8111-111111111111");
        assert!(!valid_get(&query(), &v));
    }
    #[test]
    fn unknown_kinds_and_nonfinite_normalized_are_observations_only() {
        let mut v = with_parameter();
        let d = &mut v["controllers"][0]["parameters"][0]["description"];
        d["kind"] = json!(999);
        d["support"] = json!({"raw_number":false,"normalized":false});
        assert!(valid_get(&query(), &v));
        let d = &mut v["controllers"][0]["parameters"][0]["description"];
        d["kind"] = json!(4);
        d["current"]["decimal"] = json!("-1");
        d["normalized"] = json!({"status":"unavailable","reason":"non_finite"});
        d["default"] = json!({"status":"unavailable","variant_tag":null,"reason":"missing"});
        d["support"] = json!({"raw_number":true,"normalized":false});
        assert!(valid_get(&query(), &v));
    }
    #[test]
    fn compact_adapted_body_limit_erases_instead_of_truncating() {
        let mut v = receipt();
        let mut d = description_fixture();
        d["name"] = json!("\u{1}".repeat(128));
        for i in 0..2 {
            v["controllers"][i]["parameter_count"] = json!(128);
            v["controllers"][i]["parameters"] = json!(
                (0..128)
                    .map(|j| json!({"index":j,"status":"observed","description":d}))
                    .collect::<Vec<_>>()
            );
        }
        v["reported_parameter_count"] = json!(256);
        v["enumerated_parameter_count"] = json!(256);
        assert!(v.to_string().len() > 262144);
        assert!(!valid_get(&query(), &v));
    }
    #[test]
    fn requested_policy_erases_even_forged_discriminator() {
        let mut v = with_parameter();
        v["operation"] = json!("state_capture");
        super::super::invalidate_gui_evidence(&Operation::PluginState(query()), &mut v);
        let mut expected = Value::Null;
        invalidate(&mut expected);
        assert_eq!(v, expected);
        assert_eq!(v.as_object().unwrap().len(), 10);
    }
    #[test]
    fn document_and_target_guards_precede_owner_queue() {
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
        let mut q = query();
        assert!(Operation::PluginState(q.clone()).is_read_only());
        assert!(Operation::PluginState(q.clone()).requires_explicit_document());
        q.kind = Some(crate::operation::PluginKind::Instrument);
        assert!(Operation::PluginState(q).validate().is_err());
    }
}

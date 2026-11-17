use super::*;
fn query() -> PluginState {
    serde_json::from_value(json!({"kind":"instrument","track":"Scratch","plugin":"CV Instrument","expected_instance":"12345678-1234-1234-1234-123456789ABC","action":"inspect_cv_port_structure"})).unwrap()
}
fn positive() -> Value {
    let mut v: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/cv-port-structure-native-positive.json"
    ))
    .unwrap();
    v["plugin"] = json!("CV Instrument");
    v["native_outcome"] = json!("observed");
    v
}
#[test]
fn only_exact_closed_receipts_admit_observed_including_zero_and_bounded_partial() {
    assert!(valid_get(&query(), &positive()));
    let mut v = positive();
    v["child_count"] = json!(33);
    v["observation_complete"] = json!(false);
    v["status"] = json!("child_limit");
    assert!(valid_get(&query(), &v));
    v["children"] = json!([v["parent_before"].clone()]);
    assert!(!valid_get(&query(), &v));
}
#[test]
fn phase_identity_authority_and_late_extra_fields_fail_closed() {
    for (key, bad) in [
        ("phase", json!("prepared")),
        ("cleanup_calls", json!(2)),
        ("prepare_calls", json!(0)),
        ("cleanup_returned", json!(false)),
        ("source", json!("controller")),
        ("scope", json!("choice_ids")),
        ("token", json!("handle")),
        ("child_count", json!(-1)),
        ("retained_text_bytes", json!(1)),
        ("status", json!("complete_wrong")),
    ] {
        let mut v = positive();
        v[key] = bad;
        assert!(!valid_get(&query(), &v), "{key}");
        invalidate(&mut v, false);
        assert_eq!(v["outcome"], "unknown");
        assert!(v.get("children").is_none());
    }
    for key in FALSE_FIELDS {
        let mut v = positive();
        v[*key] = json!(true);
        assert!(!valid_get(&query(), &v), "{key}");
    }
    for key in ["object_uid", "class_uid", "native_class", "object_id"] {
        let mut v = positive();
        v["instance"][key] = json!("other");
        assert!(!valid_get(&query(), &v));
    }
    let mut v = positive();
    v["parent_after"]["tag"] = json!(101);
    assert!(!valid_get(&query(), &v));
}
#[test]
fn children_are_exact_nested_descriptions_without_support_or_choice_handles() {
    let mut v = positive();
    let mut child = v["parent_before"].clone();
    child["name"] = json!("nested");
    v["child_count"] = json!(1);
    v["children"] = json!([child]);
    v["retained_text_bytes"] = json!(10);
    assert!(valid_get(&query(), &v));
    for (key, bad) in [
        ("index", json!(1)),
        ("can_set", json!(true)),
        ("path", json!([0])),
        ("choice_id", json!(0)),
        ("kind", json!("OutputChoice")),
        ("support", json!({"raw_number":true})),
    ] {
        let mut badv = v.clone();
        badv["children"][0][key] = bad;
        assert!(!valid_get(&query(), &badv), "{key}");
    }
    v["children"][0]["value"]["variant_tag"] = json!(3);
    assert!(!valid_get(&query(), &v));
}
#[test]
fn only_exact_before_prepare_failure_preserves_not_applied() {
    let mut v = json!({"ok":false,"outcome":"not_applied","native_outcome":"not_applied","phase":"before_prepare","error":{"code":"unavailable","message":"No structure"}});
    assert_eq!(failure_outcome(&v), Outcome::NotApplied);
    invalidate(&mut v, true);
    assert_eq!(v["outcome"], "not_applied");
    for phase in [
        "prepare_dispatched",
        "prepared",
        "cleanup_dispatched",
        "closed_checked",
        "unknown",
    ] {
        let mut v = v.clone();
        v["phase"] = json!(phase);
        assert_eq!(failure_outcome(&v), Outcome::Unknown);
        invalidate(&mut v, true);
        assert_eq!(v["outcome"], "unknown");
    }
    let mut v = positive();
    v["ok"] = json!(false);
    v["outcome"] = json!("not_applied");
    assert_eq!(failure_outcome(&v), Outcome::Unknown);
}

#[test]
fn nonnumeric_nested_values_are_metadata_without_pointer_or_choice_authority() {
    for (tag, data) in [
        (0, json!({"kind":"empty"})),
        (3, json!({"kind":"text","value":"nested label"})),
        (4, json!({"kind":"opaque_object","present":true})),
        (7, json!({"kind":"unsupported","tag":7})),
    ] {
        let mut v = positive();
        let mut child = v["parent_before"].clone();
        child["name"] = json!("nested");
        child["kind"] = json!("Unknown(11)");
        child["value"] = json!({"variant_tag":tag,"data":data});
        let bytes = text_bytes(&child);
        v["children"] = json!([child]);
        v["child_count"] = json!(1);
        v["retained_text_bytes"] = json!(bytes);
        assert!(valid_get(&query(), &v), "tag {tag}");
        v["children"][0]["value"]["data"]["pointer"] = json!("native");
        assert!(!valid_get(&query(), &v));
    }
}

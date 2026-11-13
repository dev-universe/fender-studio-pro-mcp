use super::*;
fn query() -> ExternalPortMenu {
    ExternalPortMenu {
        channel: "Scratch".into(),
        slot: 0,
        plugin: "External Effect Stereo".into(),
        expected_instance: "12345678-1234-1234-1234-123456789ABC".into(),
        port: ExternalPort::Send,
    }
}
fn row(path: Value, submenu: bool) -> Value {
    json!({"path":path,"name":null,"title":"Main","category":null,"checked":false,"enabled":true,"has_submenu":submenu,"handler_present":true,"kind":if submenu{"submenu"}else{"opaque"}})
}
fn good() -> Value {
    let q = query();
    let mut value: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/external-menu-native-positive.json"
    )))
    .unwrap();
    value["native_outcome"] = json!("observed");
    value["plugin"] = json!(q.plugin);
    value
}
#[test]
fn complete_external_menu_is_an_observation_with_no_selection_authority() {
    let q = query();
    assert!(valid_get(&q, &good()));
    let mut v = good();
    v["rows"] = json!([
        row(json!([0]), true),
        row(json!([0, 0]), false),
        row(json!([1]), false)
    ]);
    v["coverage"]["row_count"] = json!(3);
    assert!(valid_get(&q, &v));
    let mut q = q;
    q.plugin = "External Effect Mono".into();
    q.port = ExternalPort::Return;
    v["plugin"] = json!(q.plugin);
    v["instance"]["object_id"] = json!(q.plugin);
    v["instance"]["class_uid"] = json!("873281D9-58DE-4B2D-B3BD-528F549B1AD9");
    v["port"] = json!("return");
    v["parameter"] = json!({"name":"receivePort","tag":103});
    assert!(valid_get(&q, &v));
}
#[test]
fn external_menu_refuses_forged_claims_and_nonterminal_cleanup() {
    let q = query();
    for (key, bad) in [
        ("ok", json!(1)),
        ("native_outcome", json!("unknown")),
        ("port", json!("return")),
        ("selection_calls", json!(1)),
        ("selection_authority", json!(true)),
        ("persistence_verified", json!(true)),
        ("audio_verified", json!(true)),
        ("menu_phase", json!("release_dispatched")),
        ("token", json!("fake")),
        ("value_semantics", json!("atomic")),
        ("source", json!("dsp")),
    ] {
        let mut v = good();
        v[key] = bad;
        assert!(!valid_get(&q, &v), "{key}");
    }
    for key in [
        "root_menu_final_release_count",
        "builder_final_release_count",
    ] {
        let mut v = good();
        v["lifecycle"][key] = json!(1);
        assert!(!valid_get(&q, &v));
    }
    let mut v = good();
    v["instance"]["extra"] = json!(true);
    assert!(!valid_get(&q, &v));
    v = good();
    v["parameter"]["tag"] = json!(103);
    assert!(!valid_get(&q, &v));
}
#[test]
fn external_menu_bounds_typed_paths_and_utf8_without_value_row_inference() {
    let q = query();
    for path in [
        json!([]),
        json!([true]),
        json!([0.5]),
        json!([256]),
        json!([0, 0]),
        json!([0, 0, 0, 0, 0]),
    ] {
        let mut v = good();
        v["rows"][0]["path"] = path;
        assert!(!valid_get(&q, &v));
    }
    let mut v = good();
    v["rows"][0]["kind"] = json!("parameter_value");
    assert!(!valid_get(&q, &v));
    v = good();
    v["rows"][0]["title"] = json!("한".repeat(342));
    assert!(!valid_get(&q, &v));
    v["rows"][0]["title"] = json!("한".repeat(341));
    assert!(valid_get(&q, &v));
    v = good();
    v["rows"] = json!(vec![row(json!([0]), false); 257]);
    v["coverage"]["row_count"] = json!(257);
    assert!(!valid_get(&q, &v));
}
#[test]
fn external_menu_invalidation_removes_even_false_menu_evidence() {
    let mut v = good();
    invalidate(&mut v, false);
    assert_eq!(v["native_outcome"], "unknown");
    assert_eq!(v.as_object().unwrap().len(), 4);
    let mut v = json!({"ok":false,"outcome":"not_applied","native_outcome":"not_applied","error":{"code":"unavailable","message":"No editor"},"menu_phase":"prepared"});
    invalidate(&mut v, true);
    assert_eq!(v["native_outcome"], "not_applied");
    let mut v = json!({"ok":false,"outcome":"not_applied","native_outcome":"not_applied","error":{"code":"unavailable","message":"No editor"},"selection_authority":false});
    invalidate(&mut v, true);
    assert_eq!(v["native_outcome"], "unknown");
}
#[test]
fn external_menu_request_is_closed_and_lifecycle_sensitive() {
    let q = query();
    let raw = json!({"channel":q.channel,"slot":0,"plugin":q.plugin,"expected_instance":q.expected_instance,"port":"send"});
    let op = Operation::from_parts("external_port_menu", raw.clone()).unwrap();
    assert!(!op.is_read_only());
    assert!(op.requires_explicit_document());
    assert!(op.validate().is_ok());
    for key in [
        "source", "param", "path", "command", "action", "max_rows", "kind", "track",
    ] {
        let mut v = raw.clone();
        v[key] = json!("unchecked");
        assert!(Operation::from_parts("external_port_menu", v).is_err());
    }
    for (key, bad) in [
        ("port", json!("select")),
        ("slot", json!(true)),
        ("expected_instance", json!(null)),
    ] {
        let mut v = raw.clone();
        v[key] = bad;
        assert!(Operation::from_parts("external_port_menu", v).is_err());
    }
    let mut v = raw;
    v["plugin"] = json!("Chorder");
    assert!(
        Operation::from_parts("external_port_menu", v)
            .unwrap()
            .validate()
            .is_err()
    );
}

#[test]
fn external_menu_failure_outcome_needs_exact_preconstruct_evidence() {
    let base = json!({"ok":false,"outcome":"not_applied","native_outcome":"not_applied","error":{"code":"unavailable","message":"No editor"},"menu_phase":"prepared"});
    assert_eq!(failure_outcome(&base), Outcome::NotApplied);
    for (key, value) in [
        ("menu_phase", json!("construct_dispatched")),
        ("rows", json!([])),
        ("native_outcome", json!("unknown")),
        ("ok", json!(true)),
        ("selection_authority", json!(false)),
        ("lifecycle", json!({})),
    ] {
        let mut v = base.clone();
        v[key] = value;
        assert_eq!(failure_outcome(&v), Outcome::Unknown, "{key}");
    }
    let mut v = base;
    v["error"]["code"] = json!("");
    assert_eq!(failure_outcome(&v), Outcome::Unknown);
}

#[test]
fn external_menu_requires_exact_contract_per_text_byte_bound() {
    let q = query();
    assert_eq!(good()["coverage"]["max_text_bytes_each"], json!(1024));
    assert!(valid_get(&q, &good()));
    for bad in [
        json!(null),
        json!(true),
        json!("1024"),
        json!(1024.0),
        json!(1023),
        json!(1025),
    ] {
        let mut v = good();
        v["coverage"]["max_text_bytes_each"] = bad;
        assert!(!valid_get(&q, &v));
    }
    let mut v = good();
    v["coverage"]
        .as_object_mut()
        .unwrap()
        .remove("max_text_bytes_each");
    assert!(!valid_get(&q, &v));
    v = good();
    v["coverage"]["unchecked"] = json!(1024);
    assert!(!valid_get(&q, &v));
}

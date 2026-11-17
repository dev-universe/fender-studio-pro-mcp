use super::*;
fn query() -> ExternalPortMenu {
    ExternalPortMenu {
        channel: "Scratch".into(),
        slot: 0,
        plugin: "External Effect Stereo".into(),
        expected_instance: "12345678-1234-1234-1234-123456789ABC".into(),
        port: ExternalPort::Send,
        provenance: false,
        include_check_only: false,
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

fn provenance_good() -> Value {
    let mut v = good();
    let rows = [
        ("0", "없음", "Param", true, true),
        ("1", "메인 - 1 + 2", "Param", false, true),
        ("", "-", "", false, false),
        ("Setup", "오디오 I/O 설정", "Console", false, true),
    ];
    v["rows"] = Value::Array(rows.into_iter().enumerate().map(|(i,(name,title,category,checked,handler))| json!({"path":[i],"name":name,"title":title,"category":category,"checked":checked,"enabled":true,"has_submenu":false,"handler_present":handler,"kind":"opaque"})).collect());
    v["coverage"]["row_count"] = json!(4);
    v["provenance"] = json!({"leaves":([0,1].map(|i|json!({"path":[i],"handler_matches_builder":true,"data_handler_matches_builder":false,"data_builder_matches_builder":false,"data_parameter_is_selected":false,"original_is_selected":[false,true],"original_end":{"kind":"self_reference"}}))),"menu_data_typed_values_observed":false,"selection_authority":false,"selection_calls":0,"proof_scope":"bracketed_current_mapped_bank_sequential_read_only","read_only_intent":true,"atomic_snapshot":false,"continuous_active_mixer_membership_verified":false});
    v
}
#[test]
fn external_menu_provenance_is_typed_bounded_and_never_value_or_selection_authority() {
    let mut q = query();
    q.provenance = true;
    let good = provenance_good();
    assert!(valid_get(&q, &good));
    for (pointer, bad) in [
        ("/provenance/selection_calls", json!(1)),
        ("/provenance/menu_data_typed_values_observed", json!(true)),
        ("/provenance/leaves/0/handler_matches_builder", json!(1)),
        ("/provenance/leaves/0/original_is_selected", json!([0])),
        (
            "/provenance/leaves/0/original_end",
            json!({"kind":"cycle","index":false}),
        ),
        (
            "/provenance/leaves/0/original_end",
            json!({"kind":"cycle","index":u64::MAX}),
        ),
        ("/provenance/leaves/0/original_end", json!({"kind":"limit"})),
        ("/rows/3/title", json!("Changed")),
        ("/rows/2/handler_present", json!(true)),
    ] {
        let mut v = good.clone();
        *v.pointer_mut(pointer).unwrap() = bad;
        assert!(!valid_get(&q, &v), "{pointer}");
    }
    let mut v = good.clone();
    v["provenance"]["pointer"] = json!(123);
    assert!(!valid_get(&q, &v));
    q.provenance = false;
    assert!(!valid_get(&q, &good));
    q.provenance = true;
    q.port = ExternalPort::Return;
    assert!(!valid_get(&q, &good));
}
#[test]
fn external_menu_optional_provenance_preserves_old_wire_and_rejects_bad_intents() {
    let q = query();
    let wire = serde_json::to_value(&q).unwrap();
    assert!(wire.get("provenance").is_none());
    assert!(
        !serde_json::from_value::<ExternalPortMenu>(wire.clone())
            .unwrap()
            .provenance
    );
    for bad in [json!(0), json!(1), json!(null), json!("true")] {
        let mut v = wire.clone();
        v["provenance"] = bad;
        assert!(serde_json::from_value::<ExternalPortMenu>(v).is_err());
    }
    let mut q = q;
    q.provenance = true;
    assert!(
        crate::operation::Operation::ExternalPortMenu(q.clone())
            .validate()
            .is_ok()
    );
    q.port = ExternalPort::Return;
    assert!(
        crate::operation::Operation::ExternalPortMenu(q.clone())
            .validate()
            .is_err()
    );
    q.port = ExternalPort::Send;
    q.plugin = "External Effect Mono".into();
    assert!(
        crate::operation::Operation::ExternalPortMenu(q)
            .validate()
            .is_err()
    );
}

#[test]
fn external_menu_provenance_scope_needs_bridge_bracket_and_rejects_stronger_claims() {
    let mut q = query();
    q.provenance = true;
    let good = provenance_good();
    assert!(valid_get(&q, &good));
    for (key, bad) in [
        ("proof_scope", json!("held_target_sequential_read_only")),
        ("proof_scope", json!("atomic")),
        ("read_only_intent", json!(false)),
        ("read_only_intent", json!(1)),
        ("atomic_snapshot", json!(true)),
        ("atomic_snapshot", json!(0)),
        ("continuous_active_mixer_membership_verified", json!(true)),
        ("continuous_active_mixer_membership_verified", json!(0)),
    ] {
        let mut value = good.clone();
        value["provenance"][key] = bad;
        assert!(!valid_get(&q, &value), "{key}");
    }
}
#[test]
fn check_only_closed_schema_and_operation_union_flags() {
    let base = json!({"channel":"Scratch","slot":0,"plugin":"External Effect Stereo","expected_instance":"12345678-1234-1234-1234-123456789ABC","port":"send","provenance":true,"include_check_only":true});
    let op = Operation::from_parts("external_port_menu", base.clone()).unwrap();
    assert!(op.validate().is_ok());
    assert!(!op.is_read_only());
    assert!(op.requires_explicit_document());
    assert_eq!(op.wire().1["include_check_only"], true);
    for (key, value) in [
        ("include_check_only", json!(0)),
        ("include_check_only", json!(1)),
        ("include_check_only", Value::Null),
    ] {
        let mut bad = base.clone();
        bad[key] = value;
        assert!(Operation::from_parts("external_port_menu", bad).is_err());
    }
    for (key, value) in [
        ("provenance", json!(false)),
        ("port", json!("return")),
        ("plugin", json!("External Effect Mono")),
    ] {
        let mut bad = base.clone();
        bad[key] = value;
        let op = Operation::from_parts("external_port_menu", bad).unwrap();
        assert!(op.validate().is_err());
    }
    let good = json!({"leaves":[{"path":[0],"raw_category":1,"raw_check_only":0},{"path":[1],"raw_category":255,"raw_check_only":255}],"flags":1,"category_query_calls":2,"check_only_calls":2,"selection_calls":0,"execution_calls":0,"scope":"fixed_stereo_send_public_check_only","read_only_intent":true,"atomic_snapshot":false,"global_command_filter_checked":false,"menu_data_typed_values_observed":false});
    let provenance = json!({"leaves":[{"handler_matches_builder":true,"data_handler_matches_builder":true,"data_builder_matches_builder":true,"data_parameter_is_selected":true,"original_is_selected":[true],"original_end":{"kind":"self_reference"}},{"handler_matches_builder":true,"data_handler_matches_builder":true,"data_builder_matches_builder":true,"data_parameter_is_selected":true,"original_is_selected":[true],"original_end":{"kind":"self_reference"}}]});
    assert!(valid_check_only(&good, &provenance));
    for (key, val) in [
        ("flags", json!(0)),
        ("flags", json!(3)),
        ("execution_calls", json!(1)),
        ("global_command_filter_checked", json!(true)),
    ] {
        let mut bad = good.clone();
        bad[key] = val;
        assert!(!valid_check_only(&bad, &provenance));
    }
    let mut bad = good.clone();
    bad["leaves"][0]["raw_check_only"] = json!(false);
    assert!(!valid_check_only(&bad, &provenance));
}

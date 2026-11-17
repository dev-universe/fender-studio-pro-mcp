use super::*;

fn query(offset: u32, limit: u32) -> ChorderChordsGet {
    ChorderChordsGet {
        track: "Scratch".into(),
        slot: 0,
        expected_instance: "12345678-1234-1234-1234-123456789ABC".into(),
        offset,
        limit,
    }
}

fn page(query: &ChorderChordsGet) -> Value {
    let count = query.limit.min(128 - query.offset);
    let next = query.offset + count;
    json!({"ok":true,"outcome":"observed","native_outcome":"observed","operation":"chorder_chords_get",
        "scope":"component_alias_IStorable_chorder_chords","schema":"chorder_music_effect_preset_v1","serialization":"component_alias_IStorable",
        "instance":{"object_uid":query.expected_instance,"class_uid":"D2D9B002-12FC-4F1D-AFDA-70336D1DC108","native_class":"Chorder","object_id":"Chorder"},
        "source":"component_alias","total":128,"offset":query.offset,"limit":query.limit,"count":count,
        "next":if next<128 {json!(next)} else {Value::Null},
        "chords":(query.offset..next).map(|pitch| json!({"input_pitch":pitch,"output_pitches":[]})).collect::<Vec<_>>(),
        "capture_sha256":"1".repeat(64),"capture_byte_count":512,"model_sha256":"a".repeat(64),
        "value_semantics":"sequential_snapshot","mutation_capability":false,"persistence_verified":false,"audio_verified":false,"plugin":"Chorder"})
}

fn no_model(value: &Value) {
    for key in [
        "chords",
        "capture_sha256",
        "capture_byte_count",
        "model_sha256",
        "instance",
        "source",
        "next",
        "scope",
        "total",
        "token",
    ] {
        assert!(value.get(key).is_none(), "{key}");
    }
    assert_eq!(value["ok"], false);
    assert_eq!(value["native_outcome"], "not_applied");
    for key in [
        "mutation_capability",
        "persistence_verified",
        "audio_verified",
    ] {
        assert_eq!(value[key], false);
    }
}

#[test]
fn chorder_chord_pages_admit_all_empty_inputs_and_maximum_output_sets() {
    for offset in 0..128 {
        for limit in [1, 16] {
            let query = query(offset, limit);
            let mut value = page(&query);
            assert!(valid_get(&query, &value));
            for chord in value["chords"].as_array_mut().unwrap() {
                chord["output_pitches"] = json!((0..128).collect::<Vec<_>>());
            }
            assert!(valid_get(&query, &value));
        }
    }
}

#[test]
fn chorder_chords_reject_forged_identity_paging_payload_and_authority() {
    let query = query(0, 16);
    let mut mutations = Vec::new();
    for (key, changed) in [
        ("ok", json!(1)),
        ("outcome", json!("applied_verified")),
        ("native_outcome", json!("unknown")),
        ("operation", json!("chorder_learning_get")),
        ("scope", json!("other")),
        ("schema", json!("unknown")),
        ("serialization", json!("dsp")),
        ("source", json!("dsp")),
        ("total", json!(127)),
        ("offset", json!(1)),
        ("limit", json!(1)),
        ("count", json!(15)),
        ("next", json!(17)),
        ("capture_sha256", json!("x".repeat(64))),
        ("model_sha256", json!("A".repeat(64))),
        ("capture_byte_count", json!(0)),
        ("capture_byte_count", json!(1_048_577)),
        ("capture_byte_count", json!(true)),
        ("capture_byte_count", json!(1.5)),
        ("value_semantics", json!("atomic_snapshot")),
        ("mutation_capability", json!(true)),
        ("persistence_verified", json!(true)),
        ("audio_verified", json!(true)),
        ("token", json!("unchecked")),
        ("error", json!({"code":"late","message":"failure"})),
        ("plugin", json!("Other")),
    ] {
        let mut value = page(&query);
        value[key] = changed;
        mutations.push(value);
    }
    for key in ["class_uid", "object_uid"] {
        let mut value = page(&query);
        value["instance"][key] = json!("00000000-0000-0000-0000-000000000000");
        mutations.push(value);
    }
    for outputs in [
        json!([-1]),
        json!([128]),
        json!([true]),
        json!([null]),
        json!(["0"]),
        json!([0.5]),
        json!([1, 1]),
        json!([2, 1]),
        json!((0..129).collect::<Vec<_>>()),
    ] {
        let mut value = page(&query);
        value["chords"][0]["output_pitches"] = outputs;
        mutations.push(value);
    }
    let mut value = page(&query);
    value["chords"][0]["input_pitch"] = json!(1);
    mutations.push(value);
    let mut value = page(&query);
    value["chords"][0]["extra"] = json!(true);
    mutations.push(value);
    let mut value = page(&query);
    value["chords"].as_array_mut().unwrap().pop();
    mutations.push(value);
    let mut value = page(&query);
    value.as_object_mut().unwrap().remove("model_sha256");
    mutations.push(value);
    for mut value in mutations {
        assert!(!valid_get(&query, &value), "{value}");
        invalidate_get(&mut value);
        no_model(&value);
    }
}

#[test]
fn chorder_chords_late_failure_replaces_every_positive_model_field() {
    let query = query(112, 16);
    let mut value = page(&query);
    super::super::invalidate_gui_evidence(&Operation::ChorderChordsGet(query), &mut value);
    no_model(&value);
    let mut value = json!({"error":{"code":"chorder_schema_unsupported","message":"Unknown field"},"chords":[1],"model_sha256":"forged"});
    invalidate_get(&mut value);
    assert_eq!(value["error"]["code"], "chorder_schema_unsupported");
    no_model(&value);
}

#[test]
fn chorder_chords_document_guard_runs_before_the_owner_queue() {
    use std::future::Future;
    use std::task::{Context, Poll, Waker};
    for expected_document in [None, Some(String::new())] {
        let request = Request {
            operation: Operation::ChorderChordsGet(query(0, 16)),
            expected_document,
        };
        let (sender, receiver) = mpsc::sync_channel(1);
        let handle = RuntimeHandle { sender };
        let mut future = std::pin::pin!(handle.execute(request));
        let Poll::Ready(Err(error)) = future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        else {
            panic!("missing document entered owner queue");
        };
        assert_eq!(error.code, "missing_precondition");
        assert!(receiver.try_recv().is_err());
    }
    assert!(
        super::super::require_learning_document(&Request {
            operation: Operation::ChorderChordsGet(query(0, 16)),
            expected_document: Some("session:1".into())
        })
        .is_ok()
    );
}

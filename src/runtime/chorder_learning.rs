use super::*;
use crate::operation::{ChorderLearningChange, ChorderLearningGet, ChorderLearningSet};

fn instance(value: &Value, expected: &str) -> bool {
    exact_gui_object(
        value,
        &["object_uid", "class_uid", "native_class", "object_id"],
    ) && value["object_uid"] == expected
        && Uuid::parse_str(expected).is_ok_and(|uid| {
            !uid.is_nil() && uid.hyphenated().to_string().to_uppercase() == expected
        })
        && value["class_uid"] == "D2D9B002-12FC-4F1D-AFDA-70336D1DC108"
        && ["native_class", "object_id"].iter().all(|key| {
            value[*key]
                .as_str()
                .is_some_and(|s| s.encode_utf16().count() <= 512)
        })
}

fn normalized(value: &Value) -> bool {
    let native = &value["native_normalized"];
    if native["kind"] == "not_applicable" {
        return exact_gui_object(native, &["kind"]) && value["normalized"].is_null();
    }
    if !exact_gui_object(native, &["kind", "bits_hex"])
        || !matches!(
            native["kind"].as_str(),
            Some("float32" | "invalid_float32" | "non_finite_float32")
        )
    {
        return false;
    }
    let Some(text) = native["bits_hex"].as_str() else {
        return false;
    };
    if text.len() != 8
        || !text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b))
    {
        return false;
    }
    let Ok(bits) = u32::from_str_radix(text, 16) else {
        return false;
    };
    let number = f32::from_bits(bits);
    if native["kind"] != "float32" {
        return !number.is_finite() && value["normalized"].is_null();
    }
    number.is_finite()
        && value["normalized"]
            .as_f64()
            .is_some_and(|v| v.to_bits() == f64::from(number).to_bits())
}

fn descriptor(value: &Value, name: &str) -> bool {
    let fields = [
        "name",
        "title",
        "tag",
        "value",
        "text",
        "min",
        "max",
        "default",
        "normalized",
        "enabled",
        "readOnly",
        "writable",
        "native_kind",
        "native_value",
        "native_min",
        "native_max",
        "native_default",
        "native_normalized",
        "native_support",
        "support",
        "display_parser",
    ];
    if !exact_gui_object(value, &fields)
        || value["name"] != name
        || value["title"] != name
        || value["tag"]
            .as_i64()
            .and_then(|v| i32::try_from(v).ok())
            .is_none()
        || (!value["text"].is_null()
            && value["text"]
                .as_str()
                .is_none_or(|s| s.encode_utf16().count() > 4096))
        || ["enabled", "readOnly", "writable"]
            .iter()
            .any(|key| !value[*key].is_boolean())
        || !exact_gui_object(
            &value["native_support"],
            &["raw_number", "normalized", "display_text"],
        )
        || !exact_gui_object(&value["support"], &["raw", "normalized", "text"])
    {
        return false;
    }
    let Some(kind) = value["native_kind"].as_str() else {
        return false;
    };
    let unknown = kind
        .strip_prefix("Unknown(")
        .and_then(|s| s.strip_suffix(')'))
        .is_some_and(|s| s.parse::<i32>().is_ok_and(|v| v.to_string() == s));
    if !matches!(
        kind,
        "Toggle"
            | "Integer"
            | "Float"
            | "String"
            | "List"
            | "Command"
            | "Color"
            | "Image"
            | "Segments"
            | "Scroll"
            | "TextModel"
    ) && !unknown
    {
        return false;
    }
    let Some(parser) = value["display_parser"].as_str() else {
        return false;
    };
    let formatter = parser
        .strip_prefix("NativeFormatter { flags: ")
        .and_then(|s| s.strip_suffix(" }"))
        .is_some_and(|s| s.parse::<i32>().is_ok_and(|v| v.to_string() == s));
    if !matches!(parser, "Unsupported" | "LiteralString" | "PlainNumeric") && !formatter {
        return false;
    }
    for (native, scalar) in [
        ("native_value", "value"),
        ("native_min", "min"),
        ("native_max", "max"),
        ("native_default", "default"),
    ] {
        if !gui_default_scalar(&value[native], &value[scalar]) {
            return false;
        }
        if value[native]["data"]["kind"] == "float"
            && gui_number(&value[native]).is_some_and(|number| {
                value[scalar]
                    .as_f64()
                    .is_none_or(|v| v.to_bits() != number.to_bits())
            })
        {
            return false;
        }
    }
    let safe = ["native_value", "native_min", "native_max", "native_default"]
        .iter()
        .all(|key| value[*key]["data"].get("exact_decimal").is_none());
    for (native, scalar) in [
        ("raw_number", "raw"),
        ("normalized", "normalized"),
        ("display_text", "text"),
    ] {
        let Some(supported) = value["native_support"][native].as_bool() else {
            return false;
        };
        if value["support"][scalar].as_bool() != Some(safe && supported) {
            return false;
        }
    }
    let opaque = matches!(
        value["native_value"]["data"]["kind"].as_str(),
        Some("opaque_object" | "unsupported" | "non_finite_float")
    );
    let writable = safe
        && !opaque
        && ["raw_number", "normalized", "display_text"]
            .iter()
            .any(|key| value["native_support"][*key] == true);
    value["writable"].as_bool() == Some(writable) && normalized(value)
}

fn toggle(value: &Value) -> bool {
    descriptor(value, "learnMode")
        && value["native_kind"] == "Toggle"
        && value["enabled"] == true
        && value["readOnly"] == false
        && value["writable"] == true
        && value["support"]["raw"] == true
        && matches!(value["native_value"]["variant_tag"].as_i64(), Some(1 | 2))
        && matches!(gui_number(&value["native_value"]), Some(0.0 | 1.0))
        && gui_number(&value["native_min"]) == Some(0.0)
        && gui_number(&value["native_max"]) == Some(1.0)
        && value["native_value"]["variant_tag"] == value["native_min"]["variant_tag"]
        && value["native_value"]["variant_tag"] == value["native_max"]["variant_tag"]
        && ["native_value", "native_min", "native_max"]
            .iter()
            .all(|key| {
                value[*key]["data"]["kind"] != "float"
                    || value[*key]["data"]["bits_hex"]
                        == if value[*key]["data"]["value"].as_f64() == Some(1.0) {
                            "3FF0000000000000"
                        } else {
                            "0000000000000000"
                        }
            })
}

fn pitch(value: &Value) -> bool {
    descriptor(value, "learnInputPitch")
        && value["native_kind"] == "Integer"
        && value["tag"] == 6
        && value["enabled"] == true
        && value["readOnly"] == false
        && value["writable"] == true
        && value["support"]["raw"] == true
        && value["value"]
            .as_i64()
            .is_some_and(|v| (-1..=127).contains(&v))
        && value["min"] == -1
        && value["max"] == 127
        && value["default"] == -1
        && ["native_value", "native_min", "native_max", "native_default"]
            .iter()
            .all(|key| {
                value[*key]["variant_tag"] == 1
                    && exact_gui_object(&value[*key]["data"], &["kind", "value"])
                    && value[*key]["data"]["kind"] == "integer"
            })
}

pub(super) fn valid_get(query: &ChorderLearningGet, value: &Value) -> bool {
    exact_gui_object(
        value,
        &[
            "ok",
            "outcome",
            "native_outcome",
            "operation",
            "instance",
            "receiver",
            "mutation_capability",
            "value_semantics",
            "binding",
            "coverage",
            "parameters",
            "storage_semantics",
            "plugin",
        ],
    ) && value["ok"] == true
        && value["outcome"] == "observed"
        && value["native_outcome"] == "observed"
        && value["operation"] == "chorder_learning_get"
        && value["plugin"] == "Chorder"
        && value["receiver"] == "editor_controller"
        && value["mutation_capability"] == false
        && value["value_semantics"] == "sequential_snapshot"
        && value["storage_semantics"] == "transient_learning_controls"
        && instance(&value["instance"], &query.expected_instance)
        && exact_gui_object(
            &value["binding"],
            &[
                "editor_match",
                "autofill_same_gui",
                "controller_same_observer",
            ],
        )
        && value["binding"]["editor_match"] == "canonical_iunknown"
        && value["binding"]["autofill_same_gui"] == true
        && value["binding"]["controller_same_observer"] == true
        && exact_gui_object(
            &value["coverage"],
            &[
                "complete",
                "candidate_count",
                "visited_views",
                "max_views",
                "max_depth",
            ],
        )
        && value["coverage"]["complete"] == true
        && value["coverage"]["candidate_count"] == 1
        && value["coverage"]["max_views"] == 256
        && value["coverage"]["max_depth"] == 32
        && value["coverage"]["visited_views"]
            .as_u64()
            .is_some_and(|n| (1..=256).contains(&n))
        && exact_gui_object(&value["parameters"], &["learnMode", "learnInputPitch"])
        && descriptor(&value["parameters"]["learnMode"], "learnMode")
        && descriptor(&value["parameters"]["learnInputPitch"], "learnInputPitch")
}

fn failure(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    let fields = [
        "ok",
        "outcome",
        "native_outcome",
        "errors",
        "edit_phase",
        "instance",
        "plugin",
        "gui_value_verified",
        "persistence_verified",
        "effects_verified",
        "dsp_propagation_verified",
    ];
    object.keys().all(|key| fields.contains(&key.as_str()))
        && value["ok"] == false
        && matches!(
            value["native_outcome"].as_str(),
            Some("not_applied" | "unknown")
        )
        && value
            .get("outcome")
            .is_none_or(|outcome| *outcome == value["native_outcome"])
        && value.get("plugin").is_none_or(|name| *name == "Chorder")
        && value.get("instance").is_none_or(|value| {
            value["object_uid"]
                .as_str()
                .is_some_and(|uid| instance(value, uid))
        })
        && [
            "gui_value_verified",
            "persistence_verified",
            "effects_verified",
            "dsp_propagation_verified",
        ]
        .iter()
        .all(|key| value.get(*key).is_none_or(|v| *v == false))
        && value.get("edit_phase").is_none_or(|phase| {
            phase.as_str().is_some_and(|s| {
                matches!(
                    s,
                    "prepared"
                        | "begin_dispatched"
                        | "value_dispatched"
                        | "end_dispatched"
                        | "closed_checked"
                )
            })
        })
        && (value["native_outcome"] != "not_applied"
            || value
                .get("edit_phase")
                .is_none_or(|phase| *phase == "prepared"))
        && gui_errors(&value["errors"])
}

pub(super) fn valid_set(change: &ChorderLearningSet, value: &Value) -> bool {
    if value["ok"] == false {
        return failure(value)
            && value
                .get("instance")
                .is_none_or(|instance| instance["object_uid"] == change.expected_instance);
    }
    if !exact_gui_object(
        value,
        &[
            "ok",
            "outcome",
            "native_outcome",
            "operation",
            "instance",
            "binding_source",
            "receiver",
            "requested_change",
            "edit_phase",
            "gui_parameter",
            "gui_value_verified",
            "verification_scope",
            "persistence_verified",
            "effects_verified",
            "dsp_propagation_verified",
            "storage_semantics",
            "plugin",
        ],
    ) || value["ok"] != true
        || value["outcome"] != "applied_unverified"
        || value["native_outcome"] != "applied_unverified"
        || value["operation"] != "edit_chorder_model"
        || value["plugin"] != "Chorder"
        || value["binding_source"] != "dsp"
        || value["receiver"] != "editor_controller"
        || value["edit_phase"] != "closed_checked"
        || value["requested_change"]
            != serde_json::to_value(&change.change).expect("typed learning change")
        || !instance(&value["instance"], &change.expected_instance)
        || value["verification_scope"] != "gui_receiver_current_value"
        || value["storage_semantics"] != "transient_learning_controls"
        || [
            "persistence_verified",
            "effects_verified",
            "dsp_propagation_verified",
        ]
        .iter()
        .any(|key| value[*key] != false)
        || !exact_gui_object(&value["gui_parameter"], &["name", "before", "after"])
    {
        return false;
    }
    let (name, desired) = match &change.change {
        ChorderLearningChange::LearnMode { enabled } => ("learnMode", i64::from(*enabled)),
        ChorderLearningChange::InputPitch { pitch } => {
            ("learnInputPitch", pitch.0.map_or(-1, i64::from))
        }
    };
    if value["gui_parameter"]["name"] != name {
        return false;
    }
    let before = &value["gui_parameter"]["before"];
    let after = &value["gui_parameter"]["after"];
    let profile = if name == "learnMode" { toggle } else { pitch };
    if !profile(before)
        || !profile(after)
        || before["native_value"]["variant_tag"] != after["native_value"]["variant_tag"]
    {
        return false;
    }
    if before.as_object().unwrap().iter().any(|(key, value)| {
        ![
            "value",
            "text",
            "normalized",
            "native_value",
            "native_normalized",
        ]
        .contains(&key.as_str())
            && *value != after[key]
    }) {
        return false;
    }
    let exact = gui_number(&after["native_value"])
        .is_some_and(|number| number.to_bits() == (desired as f64).to_bits());
    value["gui_value_verified"].as_bool() == Some(exact)
}

pub(super) fn invalidate_get(value: &mut Value) {
    let error = value.get("error").filter(|error| exact_gui_object(error, &["code", "message"])
        && error["code"].as_str().is_some_and(|s| s.len() <= 64) && error["message"].as_str().is_some_and(|s| s.len() <= 512)).cloned()
        .unwrap_or_else(|| json!({"code":"invalid_chorder_learning_read","message":"The fixed GUI learning observation was not established"}));
    *value = json!({"ok":false,"outcome":"not_applied","native_outcome":"not_applied","mutation_capability":false,"error":error});
}

pub(super) fn invalidate_set(value: &mut Value, preserve_refusal: bool) {
    let refused = preserve_refusal && value["native_outcome"] == "not_applied" && failure(value);
    let phase = value
        .get("edit_phase")
        .and_then(Value::as_str)
        .filter(|phase| {
            matches!(
                *phase,
                "prepared"
                    | "begin_dispatched"
                    | "value_dispatched"
                    | "end_dispatched"
                    | "closed_checked"
            )
        })
        .map(str::to_owned);
    let errors = value.get("errors").filter(|errors| gui_errors(errors)).cloned()
        .unwrap_or_else(|| json!([{"code":"invalid_chorder_learning_edit","message":"The fixed GUI learning edit was not established"}]));
    let outcome = if refused { "not_applied" } else { "unknown" };
    *value = json!({"ok":false,"outcome":outcome,"native_outcome":outcome,"gui_value_verified":false,
        "persistence_verified":false,"effects_verified":false,"dsp_propagation_verified":false,"errors":errors});
    if let Some(phase) = phase {
        value["edit_phase"] = json!(phase);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operation::{ChorderLearningChange, ChorderLearningPitch};
    const UID: &str = "12345678-1234-1234-1234-123456789ABC";
    fn target() -> ChorderLearningGet {
        ChorderLearningGet {
            track: "Scratch".into(),
            slot: 0,
            expected_instance: UID.into(),
        }
    }
    fn change(change: ChorderLearningChange) -> ChorderLearningSet {
        ChorderLearningSet {
            track: "Scratch".into(),
            slot: 0,
            expected_instance: UID.into(),
            change,
        }
    }
    fn id() -> Value {
        json!({"object_uid":UID,"class_uid":"D2D9B002-12FC-4F1D-AFDA-70336D1DC108","native_class":"Chorder","object_id":"Chorder"})
    }
    fn description(name: &str, value: i64, float: bool) -> Value {
        let pitch = name == "learnInputPitch";
        let typed = |n: i64| {
            if float {
                json!({"variant_tag":2,"data":{"kind":"float","value":n as f64,"bits_hex":format!("{:016X}",(n as f64).to_bits())}})
            } else {
                json!({"variant_tag":1,"data":{"kind":"integer","value":n}})
            }
        };
        let normalized = if pitch {
            (value + 1) as f32 / 128.0
        } else {
            value as f32
        };
        json!({"name":name,"title":name,"tag":if pitch{6}else{5},"value":value,"text":value.to_string(),"min":if pitch{-1}else{0},
            "max":if pitch{127}else{1},"default":if pitch{-1}else{0},"normalized":f64::from(normalized),"enabled":true,"readOnly":false,"writable":true,
            "native_kind":if pitch{"Integer"}else{"Toggle"},"native_value":typed(value),"native_min":typed(if pitch{-1}else{0}),"native_max":typed(if pitch{127}else{1}),
            "native_default":typed(if pitch{-1}else{0}),"native_normalized":{"kind":"float32","bits_hex":format!("{:08X}",normalized.to_bits())},
            "native_support":{"raw_number":true,"normalized":true,"display_text":true},"support":{"raw":true,"normalized":true,"text":true},"display_parser":"PlainNumeric"})
    }
    fn get() -> Value {
        json!({"ok":true,"outcome":"observed","native_outcome":"observed","operation":"chorder_learning_get","instance":id(),"receiver":"editor_controller",
            "mutation_capability":false,"value_semantics":"sequential_snapshot","binding":{"editor_match":"canonical_iunknown","autofill_same_gui":true,"controller_same_observer":true},
            "coverage":{"complete":true,"candidate_count":1,"visited_views":125,"max_views":256,"max_depth":32},
            "parameters":{"learnMode":description("learnMode",0,false),"learnInputPitch":description("learnInputPitch",-1,false)},
            "storage_semantics":"transient_learning_controls","plugin":"Chorder"})
    }
    fn set(change: &ChorderLearningSet, float: bool) -> Value {
        let (name, value) = match &change.change {
            ChorderLearningChange::LearnMode { enabled } => ("learnMode", i64::from(*enabled)),
            ChorderLearningChange::InputPitch { pitch } => {
                ("learnInputPitch", pitch.0.map_or(-1, i64::from))
            }
        };
        let before = if name == "learnMode" {
            1 - value
        } else if value == -1 {
            0
        } else {
            -1
        };
        json!({"ok":true,"outcome":"applied_unverified","native_outcome":"applied_unverified","operation":"edit_chorder_model","instance":id(),
            "binding_source":"dsp","receiver":"editor_controller","requested_change":change.change,"edit_phase":"closed_checked","gui_parameter":{"name":name,
                "before":description(name,before,float),"after":description(name,value,float)},"gui_value_verified":true,"verification_scope":"gui_receiver_current_value",
            "persistence_verified":false,"effects_verified":false,"dsp_propagation_verified":false,"storage_semantics":"transient_learning_controls","plugin":"Chorder"})
    }
    fn no_positive(value: &Value) {
        for key in [
            "parameters",
            "binding",
            "coverage",
            "gui_parameter",
            "requested_change",
            "storage_semantics",
        ] {
            assert!(value.get(key).is_none(), "{key}");
        }
        assert_ne!(value["gui_value_verified"], true);
    }
    #[test]
    fn chorder_learning_receipts_keep_nullable_pitch_and_gui_only_current_value() {
        assert!(valid_get(&target(), &get()));
        for intent in [
            ChorderLearningChange::LearnMode { enabled: false },
            ChorderLearningChange::LearnMode { enabled: true },
            ChorderLearningChange::InputPitch {
                pitch: ChorderLearningPitch(None),
            },
            ChorderLearningChange::InputPitch {
                pitch: ChorderLearningPitch(Some(0)),
            },
            ChorderLearningChange::InputPitch {
                pitch: ChorderLearningPitch(Some(127)),
            },
        ] {
            let change = change(intent);
            for float in [false, true] {
                if matches!(change.change, ChorderLearningChange::InputPitch { .. }) && float {
                    continue;
                }
                let value = set(&change, float);
                assert!(valid_set(&change, &value), "{value}");
                assert!(!super::super::verify(
                    &Operation::ChorderLearningSet(change.clone()),
                    &value
                ));
                assert_eq!(mutation_outcome(&value), Outcome::AppliedUnverified);
            }
        }
    }
    #[test]
    fn chorder_learning_unknown_read_fields_or_bits_cannot_survive_invalidation() {
        for (path, replacement) in [
            ("/source", json!("dsp")),
            ("/binding/autofill_same_gui", json!(false)),
            ("/coverage/max_views", json!(128)),
            ("/coverage/visited_views", json!(257)),
            (
                "/parameters/learnInputPitch/native_value/data/value",
                json!(0),
            ),
            (
                "/parameters/learnMode/native_normalized/bits_hex",
                json!("3F800000"),
            ),
            (
                "/instance/object_uid",
                json!("87654321-4321-4321-4321-CBA987654321"),
            ),
            ("/error", json!({"code":"failed","message":"late read"})),
        ] {
            let mut value = get();
            if let Some(slot) = value.pointer_mut(path) {
                *slot = replacement;
            } else {
                value[path.trim_start_matches('/')] = replacement;
            }
            assert!(!valid_get(&target(), &value));
            invalidate_get(&mut value);
            no_positive(&value);
            assert_eq!(value["native_outcome"], "not_applied");
        }
    }
    #[test]
    fn chorder_learning_forged_set_claims_are_unknown_with_no_restore_or_promotion() {
        let change = change(ChorderLearningChange::InputPitch {
            pitch: ChorderLearningPitch(Some(0)),
        });
        for (path, replacement) in [
            ("/gui_parameter/after/tag", json!(7)),
            ("/gui_parameter/after/native_kind", json!("Toggle")),
            ("/gui_parameter/after/native_value/variant_tag", json!(17)),
            ("/requested_change/pitch", json!(1)),
            ("/edit_phase", json!("end_dispatched")),
            ("/persistence_verified", json!(true)),
            ("/effects_verified", json!(true)),
            ("/dsp_propagation_verified", json!(true)),
            ("/gui_value_verified", json!(false)),
            ("/outcome", json!("applied_verified")),
        ] {
            let mut value = set(&change, false);
            *value.pointer_mut(path).unwrap() = replacement;
            assert!(!valid_set(&change, &value));
            invalidate_set(&mut value, false);
            no_positive(&value);
            assert_eq!(value["native_outcome"], "unknown");
        }
        let mut mismatch = set(&change, false);
        mismatch["gui_parameter"]["after"] = description("learnInputPitch", 1, false);
        mismatch["gui_value_verified"] = json!(false);
        assert!(valid_set(&change, &mismatch));
    }
    #[test]
    fn chorder_learning_late_failures_keep_only_known_prebegin_refusal_or_unknown_phase() {
        let change = change(ChorderLearningChange::LearnMode { enabled: true });
        for phase in [
            "prepared",
            "begin_dispatched",
            "value_dispatched",
            "end_dispatched",
            "closed_checked",
        ] {
            let refused = phase == "prepared";
            let mut value = json!({"ok":false,"native_outcome":if refused{"not_applied"}else{"unknown"},"edit_phase":phase,
                "errors":[{"code":"stopped","message":"context changed"}]});
            assert!(valid_set(&change, &value));
            invalidate_set(&mut value, true);
            no_positive(&value);
            assert_eq!(
                value["native_outcome"],
                if refused { "not_applied" } else { "unknown" }
            );
        }
        let mut contradiction = json!({"ok":false,"native_outcome":"not_applied","edit_phase":"value_dispatched",
            "errors":[{"code":"stopped","message":"contradictory phase"}]});
        assert!(!valid_set(&change, &contradiction));
        invalidate_set(&mut contradiction, true);
        assert_eq!(contradiction["native_outcome"], "unknown");
        let mut positive = set(&change, false);
        super::super::invalidate_gui_evidence(
            &Operation::ChorderLearningSet(change),
            &mut positive,
        );
        no_positive(&positive);
        assert_eq!(positive["native_outcome"], "unknown");
        let mut positive = get();
        super::super::invalidate_gui_evidence(
            &Operation::ChorderLearningGet(target()),
            &mut positive,
        );
        no_positive(&positive);
    }
    #[test]
    fn chorder_learning_document_guard_runs_before_the_owner_queue() {
        use std::future::Future;
        use std::task::{Context, Poll, Waker};

        for operation in [
            Operation::ChorderLearningGet(target()),
            Operation::ChorderLearningSet(change(ChorderLearningChange::InputPitch {
                pitch: ChorderLearningPitch(None),
            })),
        ] {
            for expected in [None, Some(String::new())] {
                let request = Request {
                    operation: operation.clone(),
                    expected_document: expected,
                };
                assert_eq!(
                    super::super::require_learning_document(&request)
                        .unwrap_err()
                        .code,
                    "missing_precondition"
                );
                let (sender, receiver) = mpsc::sync_channel(1);
                let handle = RuntimeHandle { sender };
                let mut future = std::pin::pin!(handle.execute(request));
                let Poll::Ready(Err(error)) = future
                    .as_mut()
                    .poll(&mut Context::from_waker(Waker::noop()))
                else {
                    panic!("missing document must refuse before queuing an operation");
                };
                assert_eq!(error.code, "missing_precondition");
                assert!(receiver.try_recv().is_err());
            }
            assert!(
                super::super::require_learning_document(&Request {
                    operation,
                    expected_document: Some("session:1".into())
                })
                .is_ok()
            );
        }
    }
}

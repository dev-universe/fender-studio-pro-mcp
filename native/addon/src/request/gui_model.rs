use super::*;

pub(super) const API: &str = "chorder_learning_controls_v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Role {
    LearnMode,
    InputPitch,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum InputPitch {
    Unselected,
    Note(u8),
}
impl InputPitch {
    fn raw(self) -> i64 {
        match self {
            Self::Unselected => -1,
            Self::Note(pitch) => i64::from(pitch),
        }
    }
}
impl<'de> Deserialize<'de> for InputPitch {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match Value::deserialize(deserializer)? {
            Value::Null => Ok(Self::Unselected),
            Value::Number(number) => number
                .as_u64()
                .filter(|n| *n <= 127)
                .map(|n| Self::Note(n as u8))
                .ok_or_else(|| serde::de::Error::custom("pitch must be null or an integer0..127")),
            _ => Err(serde::de::Error::custom(
                "pitch must be null or an integer0..127",
            )),
        }
    }
}
impl Serialize for InputPitch {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Unselected => serializer.serialize_none(),
            Self::Note(pitch) => serializer.serialize_u8(*pitch),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub(super) struct MidiPitch(u8);
impl MidiPitch {
    fn value(self) -> u8 {
        self.0
    }
}
impl<'de> Deserialize<'de> for MidiPitch {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match Value::deserialize(deserializer)? {
            Value::Number(number) => number
                .as_u64()
                .filter(|value| *value <= 127)
                .map(|value| Self(value as u8))
                .ok_or_else(|| serde::de::Error::custom("MIDI pitch must be an integer0..127")),
            _ => Err(serde::de::Error::custom(
                "MIDI pitch must be an integer0..127",
            )),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Change {
    LearnMode {
        enabled: bool,
    },
    InputPitch {
        pitch: InputPitch,
    },
    ChordNote {
        expected_input_pitch: MidiPitch,
        output_pitch: MidiPitch,
        enabled: bool,
    },
}
impl Change {
    fn role(self) -> Option<Role> {
        match self {
            Self::LearnMode { .. } => Some(Role::LearnMode),
            Self::InputPitch { .. } => Some(Role::InputPitch),
            Self::ChordNote { .. } => None,
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::LearnMode { .. } => "learnMode",
            Self::InputPitch { .. } => "learnInputPitch",
            Self::ChordNote { .. } => "learnChord",
        }
    }
}
pub(super) const PITCH_PROFILE: parameters::IntegerEditProfile = parameters::IntegerEditProfile {
    name: "learnInputPitch",
    tag: 6,
    min: -1,
    max: 127,
    default: -1,
};
enum Binding {
    Controls(Box<gui_parameter::ModelBinding>),
    Note(Box<gui_parameter::NoteBinding>),
}
impl Binding {
    fn check(&self, ctx: &Context<'_>) -> Result<(), String> {
        match self {
            Self::Controls(binding) => binding.check(ctx),
            Self::Note(binding) => binding.check(ctx),
        }
    }
    fn check_pinned(&self, ctx: &Context<'_>) -> Result<(), String> {
        match self {
            Self::Controls(binding) => binding.check_pinned(ctx),
            Self::Note(binding) => binding.check_pinned(ctx),
        }
    }
}
pub(super) struct EditLease {
    numeric: parameters::NumericEditLease,
    binding: Binding,
}
impl EditLease {
    pub(super) fn phase(&self) -> &'static str {
        self.numeric.phase().as_str()
    }
    pub(super) fn began(&self) -> bool {
        self.numeric.phase().effect_may_have_started()
    }
    pub(super) fn check(&self, ctx: &Context<'_>) -> Result<(), String> {
        self.numeric
            .check(&mut || self.binding.check(ctx))
            .map_err(|e| e.to_string())?;
        self.binding.check_pinned(ctx)
    }
}
pub(super) fn edit(ctx: &Context<'_>, change: Change) -> (Value, Option<EditLease>) {
    let prepared = (|| {
        let binding = match change {
            Change::ChordNote {
                expected_input_pitch,
                output_pitch,
                ..
            } => Binding::Note(Box::new(gui_parameter::NoteBinding::capture(
                ctx,
                expected_input_pitch.value(),
                output_pitch.value(),
            )?)),
            _ => Binding::Controls(Box::new(gui_parameter::ModelBinding::capture_set(ctx)?)),
        };
        let parameter = match &binding {
            Binding::Controls(controls) => controls.parameter(
                ctx,
                change.role().ok_or("learning control role is missing")?,
            )?,
            Binding::Note(note) => note.parameter(ctx)?,
        };
        Ok::<_, String>((binding, parameter))
    })();
    let (binding, parameter) = match prepared {
        Ok(prepared) => prepared,
        Err(reason) => {
            return (
                error("chorder_learning_unavailable", &reason, "not_applied"),
                None,
            );
        }
    };
    let mut guard = || binding.check(ctx);
    let attempt = match change {
        Change::LearnMode { enabled } => {
            parameter.edit_raw_toggle("learnMode", enabled, &mut guard)
        }
        Change::InputPitch { pitch } => {
            parameter.edit_raw_integer(PITCH_PROFILE, pitch.raw(), &mut guard)
        }
        Change::ChordNote { enabled, .. } => match &binding {
            Binding::Note(note) => note.edit(parameter, enabled, ctx),
            Binding::Controls(_) => {
                unreachable!("closed membership intent requires a note binding")
            }
        },
    };
    let lease = EditLease {
        numeric: attempt.lease,
        binding,
    };
    let mut value = match attempt.result {
        Ok(readback) => {
            json!({"ok":true,"outcome":"applied_unverified","operation":"edit_chorder_model",
            "binding_source":"dsp","receiver":"editor_controller","requested_change":change,
            "gui_parameter":{"name":change.name(),"before":description(&readback.before),"after":description(&readback.after)},
            "gui_value_verified":readback.verified,"verification_scope":"gui_receiver_current_value",
            "persistence_verified":false,"effects_verified":false,"dsp_propagation_verified":false,
            "storage_semantics":"transient_learning_controls"})
        }
        Err(failure) => error(
            "chorder_learning_edit_failed",
            &failure.error.to_string(),
            if lease.began() || failure.outcome == parameters::Outcome::Unknown {
                "unknown"
            } else {
                "not_applied"
            },
        ),
    };
    if value["ok"] == true && matches!(change, Change::ChordNote { .. }) {
        value["storage_semantics"] = json!("selected_chord_membership_gui_current_value");
        value["stored_model_verified"] = json!(false);
        value["output_pitch_mapping"] = json!("source_declared_unverified");
        value["selection_semantics"] = json!("synchronous_guarded_comparison_not_atomic_snapshot");
    }
    value["edit_phase"] = json!(lease.phase());
    (value, Some(lease))
}
pub(super) fn get(
    ctx: &Context<'_>,
) -> Result<(Value, gui_parameter::ComparisonRead, Controller), String> {
    let binding = gui_parameter::ModelBinding::capture_get(ctx)?;
    let (parameters, coverage) = binding.snapshot()?;
    let value = json!({"ok":true,"outcome":"observed","operation":"chorder_learning_get","receiver":"editor_controller",
        "mutation_capability":false,"value_semantics":"sequential_snapshot",
        "binding":{"editor_match":"canonical_iunknown","autofill_same_gui":true,"controller_same_observer":true},
        "coverage":coverage,"parameters":parameters,"storage_semantics":"transient_learning_controls"});
    binding.check(ctx)?;
    let (comparison, direct) = binding.into_read();
    Ok((value, comparison, direct))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chord_note_intent_is_closed_bounded_and_never_implicitly_selects_an_input() {
        let base =
            json!({"kind":"chord_note","expected_input_pitch":60,"output_pitch":0,"enabled":true});
        for input in [0, 60, 127] {
            for output in [0, 60, 127] {
                for enabled in [false, true] {
                    let mut value = base.clone();
                    value["expected_input_pitch"] = json!(input);
                    value["output_pitch"] = json!(output);
                    value["enabled"] = json!(enabled);
                    let change: Change = serde_json::from_value(value.clone()).unwrap();
                    assert_eq!(serde_json::to_value(change).unwrap(), value);
                    assert_eq!(change.role(), None);
                    assert_eq!(change.name(), "learnChord");
                }
            }
        }
        for field in ["expected_input_pitch", "output_pitch"] {
            for bad in [
                Value::Null,
                json!(-1),
                json!(128),
                json!(60.0),
                json!(true),
                json!("60"),
                json!([]),
            ] {
                let mut value = base.clone();
                value[field] = bad;
                assert!(serde_json::from_value::<Change>(value).is_err());
            }
        }
        for field in ["expected_input_pitch", "output_pitch", "enabled"] {
            let mut value = base.clone();
            value.as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<Change>(value).is_err());
        }
        for field in [
            "pitch",
            "child",
            "path",
            "param",
            "source",
            "select_input",
            "value",
            "mode",
            "max_views",
        ] {
            let mut value = base.clone();
            value[field] = json!("forbidden");
            assert!(serde_json::from_value::<Change>(value).is_err());
        }
        for bad in [Value::Null, json!(1), json!("true")] {
            let mut value = base.clone();
            value["enabled"] = bad;
            assert!(serde_json::from_value::<Change>(value).is_err());
        }
        let request:Request=serde_json::from_value(json!({"operation":"edit_chorder_model","expected_instance":"61D64D66-2A46-4BEE-A623-EC5CEB898313","change":base})).unwrap();
        assert!(request.validate().is_ok());
        assert_eq!(request.source(), Source::Dsp);
    }
    #[test]
    fn late_chord_note_receipt_failure_removes_all_gui_selection_model_and_mapping_claims() {
        let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
        let mut host = guard_tests::Fixture::new();
        let ctx = host.context();
        let service = ctx.service;
        let mut reply = Reply::unbound(
            json!({"ok":true,"outcome":"applied_unverified","operation":"edit_chorder_model",
            "requested_change":{"kind":"chord_note","expected_input_pitch":60,"output_pitch":0,"enabled":true},
            "gui_value_verified":true,"gui_parameter":{"name":"learnChord","after":{"value":1}},
            "edit_phase":"closed_checked","stored_model_verified":false,"output_pitch_mapping":"source_declared_unverified",
            "selection_semantics":"synchronous_guarded_comparison_not_atomic_snapshot","persistence_verified":false,"effects_verified":false}),
        );
        reply.context = Some(ctx);
        reply.mutation = true;
        host.switch_document();
        assert!(!reply.final_check(service));
        assert_eq!(reply.value["outcome"], "unknown");
        assert_eq!(reply.value["ok"], false);
        for field in [
            "requested_change",
            "gui_value_verified",
            "gui_parameter",
            "stored_model_verified",
            "output_pitch_mapping",
            "selection_semantics",
            "persistence_verified",
            "effects_verified",
        ] {
            assert!(reply.value.get(field).is_none(), "{field}");
        }
        assert_eq!(reply.value["edit_phase"], "closed_checked");
        reply.release_context();
    }
    #[test]
    fn native_learning_operations_are_finite_and_require_instance() {
        let uid = "61D64D66-2A46-4BEE-A623-EC5CEB898313";
        for body in [
            json!({"operation":"chorder_learning_get","expected_instance":uid}),
            json!({"operation":"edit_chorder_model","expected_instance":uid,"change":{"kind":"input_pitch","pitch":null}}),
        ] {
            let parsed: Request = serde_json::from_value(body.clone()).unwrap();
            assert!(parsed.validate().is_ok());
            assert_eq!(parsed.source(), Source::Dsp);
            for key in ["source", "param", "mode", "max_views", "path"] {
                let mut extra = body.clone();
                extra[key] = json!("forbidden");
                assert!(serde_json::from_value::<Request>(extra).is_err());
            }
            let mut missing = body.clone();
            missing.as_object_mut().unwrap().remove("expected_instance");
            assert!(serde_json::from_value::<Request>(missing).is_err());
            let mut zero = body;
            zero["expected_instance"] = json!(ZERO_UID);
            assert!(
                serde_json::from_value::<Request>(zero)
                    .unwrap()
                    .validate()
                    .is_err()
            );
        }
    }
    #[test]
    fn late_learning_failures_remove_all_positive_evidence() {
        for operation in ["chorder_learning_get", "edit_chorder_model"] {
            let mut value = json!({"ok":false,"outcome":"unknown","error":{"code":"changed","message":"context changed"},
                "operation":operation,"instance":{"class_uid":"fixture"},"edit_phase":"closed_checked",
                "parameters":{"learnMode":{"value":1}},"binding":{"autofill_same_gui":true},"coverage":{"complete":true},
                "gui_parameter":{"after":{"value":1}},"gui_value_verified":true,"requested_change":{"kind":"learn_mode","enabled":true},
                "storage_semantics":"transient_learning_controls","persistence_verified":false});
            gui_parameter::invalidate(&mut value);
            assert_eq!(value.as_object().unwrap().len(), 5);
            assert_eq!(value["ok"], false);
            assert_eq!(value["outcome"], "unknown");
            for key in [
                "parameters",
                "binding",
                "coverage",
                "gui_parameter",
                "gui_value_verified",
                "requested_change",
                "operation",
                "storage_semantics",
            ] {
                assert!(value.get(key).is_none(), "{key}");
            }
        }
    }
    #[test]
    fn learning_change_is_closed_and_nullable_pitch_is_required() {
        for change in [
            json!({"kind":"learn_mode","enabled":false}),
            json!({"kind":"learn_mode","enabled":true}),
            json!({"kind":"input_pitch","pitch":null}),
            json!({"kind":"input_pitch","pitch":0}),
            json!({"kind":"input_pitch","pitch":127}),
        ] {
            let parsed: Change = serde_json::from_value(change.clone()).unwrap();
            assert_eq!(serde_json::to_value(parsed).unwrap(), change);
        }
        for change in [
            json!({"kind":"input_pitch"}),
            json!({"kind":"learn_mode"}),
            json!({"kind":"input_pitch","pitch":-1}),
            json!({"kind":"input_pitch","pitch":128}),
            json!({"kind":"input_pitch","pitch":true}),
            json!({"kind":"input_pitch","pitch":1.0}),
            json!({"kind":"learn_mode","enabled":1}),
            json!({"kind":"learn_mode","enabled":null}),
            json!({"kind":"learn_mode","enabled":true,"pitch":null}),
            json!({"kind":"input_pitch","pitch":0,"name":"other"}),
            json!({"kind":"learn_chord","enabled":true}),
            Value::Null,
        ] {
            assert!(
                serde_json::from_value::<Change>(change.clone()).is_err(),
                "{change}"
            );
        }
        assert_eq!(InputPitch::Unselected.raw(), -1);
        assert_eq!(InputPitch::Note(0).raw(), 0);
        assert_eq!(InputPitch::Note(127).raw(), 127);
    }
}

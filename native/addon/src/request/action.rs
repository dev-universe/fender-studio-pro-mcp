use super::*;
use std::ffi::CStr;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum Action {
    SwapFilterFrequencies,
    ResetMeterHold,
    ResetMeterLoudness,
    ResetMinimumReduction,
    RetriggerScope,
    ClearChorderSelected,
}
impl Action {
    fn name(self) -> &'static str {
        match self {
            Self::SwapFilterFrequencies => "swap_filter_frequencies",
            Self::ResetMeterHold => "reset_meter_hold",
            Self::ResetMeterLoudness => "reset_meter_loudness",
            Self::ResetMinimumReduction => "reset_minimum_reduction",
            Self::RetriggerScope => "retrigger_scope",
            Self::ClearChorderSelected => "clear_chorder_selected",
        }
    }
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ClearGuard {
    expected_input_pitch: u8,
    expected_model_sha256: String,
}

pub(super) fn supplied_clear_guard<'de, D>(deserializer: D) -> Result<Option<ClearGuard>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    ClearGuard::deserialize(deserializer).map(Some)
}

pub(super) fn validate_clear_intent(
    action: Action,
    guard: Option<&ClearGuard>,
) -> Result<(), String> {
    match (action, guard) {
        (Action::ClearChorderSelected, Some(guard))
            if guard.expected_input_pitch <= 127
                && guard.expected_model_sha256.len() == 64
                && guard
                    .expected_model_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) =>
        {
            Ok(())
        }
        (Action::ClearChorderSelected, _) => {
            Err("Chorder Clear requires an expected MIDI pitch and lowercase model SHA-256".into())
        }
        (_, Some(_)) => Err("A Chorder guard is only valid for Clear Chorder Selected".into()),
        (_, None) => Ok(()),
    }
}

#[derive(Clone, Copy)]
struct Spec {
    parameter: &'static CStr,
    effects: &'static [&'static str],
    expected_tag: Option<i32>,
}
fn registered(class_uid: &str, action: Action) -> Result<Spec, String> {
    match (class_uid, action) {
        (
            "831A0046-BD71-45BF-A419-6FC90D6E9F2F" | "132C0E1F-06FD-4C96-BD65-678C04FE1C92",
            Action::SwapFilterFrequencies,
        ) => Ok(Spec {
            parameter: c"swapfreqs",
            effects: &["filterlow", "filterhigh"],
            expected_tag: None,
        }),
        ("36F3F4D1-CBB4-4BF7-A7E3-EBCCED53718B", Action::SwapFilterFrequencies) => Ok(Spec {
            parameter: c"studiocomp.swapfreqs",
            effects: &[
                "studiocomp.sidechainfreqlow",
                "studiocomp.sidechainfreqhigh",
            ],
            expected_tag: None,
        }),
        ("DF958D90-0196-434F-ADC3-1FC2BB7F4AF6", Action::ResetMeterHold) => Ok(Spec {
            parameter: c"resethold",
            effects: &[],
            expected_tag: Some(1383298120),
        }),
        ("DF958D90-0196-434F-ADC3-1FC2BB7F4AF6", Action::ResetMeterLoudness) => Ok(Spec {
            parameter: c"resetloudness",
            effects: &[],
            expected_tag: Some(1383298124),
        }),
        ("61B18D53-26FA-4220-8614-89944A1990EC", Action::ResetMinimumReduction) => Ok(Spec {
            parameter: c"resetmin",
            effects: &[],
            expected_tag: Some(1383288146),
        }),
        ("36F3F4D1-CBB4-4BF7-A7E3-EBCCED53718B", Action::ResetMinimumReduction) => Ok(Spec {
            parameter: c"studiocomp.resetmin",
            effects: &[],
            expected_tag: Some(88),
        }),
        ("36322412-AD90-4EE0-B63C-37929566B081", Action::RetriggerScope) => Ok(Spec {
            parameter: c"trigRetrig",
            effects: &[],
            expected_tag: Some(1398043218),
        }),
        ("D2D9B002-12FC-4F1D-AFDA-70336D1DC108", Action::ClearChorderSelected) => Ok(Spec {
            parameter: c"chorder.clear",
            effects: &[],
            expected_tag: Some(9),
        }),
        _ => Err("button action is not registered for this exact native class".into()),
    }
}

pub(super) struct ActionLease {
    parameter: RawParameter,
    owner: Option<parameters::ControllerOwner>,
    name: &'static CStr,
    expected_tag: Option<i32>,
}
impl ActionLease {
    fn capture<G: FnMut() -> Result<(), String>>(
        parameter: RawParameter,
        spec: Spec,
        guard: &mut G,
    ) -> Result<Self, String> {
        match parameter.controller_owner(guard) {
            Ok(owner) => Ok(Self {
                parameter,
                owner,
                name: spec.parameter,
                expected_tag: spec.expected_tag,
            }),
            Err(error) => {
                let before = guard();
                drop(parameter);
                let after = guard();
                before.and(after)?;
                Err(error.to_string())
            }
        }
    }
    pub(super) fn check<G: FnMut() -> Result<(), String>>(
        &self,
        controller: &Controller,
        guard: &mut G,
    ) -> Result<(), String> {
        self.check_owner(guard)?;
        guard()?;
        let name = PlainCString {
            text: self.name.as_ptr(),
            owner: ptr::null_mut(),
        };
        let find: unsafe extern "C" fn(*mut c_void, *const PlainCString) -> *mut c_void =
            unsafe { method(controller.handle.0, 5) };
        let current = unsafe { find(controller.handle.0, &name) };
        guard()?;
        if current != self.parameter.as_borrowed_raw().as_ptr() {
            return Err("registered button parameter binding changed".into());
        }
        self.check_owner(guard)?;
        guard()?;
        // Both matching views are retained. Compare fresh borrowed observations
        // after the reentrant guard and temporary ownership work.
        let current = unsafe { find(controller.handle.0, &name) };
        let get_owner: unsafe extern "C" fn(*mut c_void) -> *mut c_void =
            unsafe { method(self.parameter.as_borrowed_raw().as_ptr(), 14) };
        let current_owner = unsafe { get_owner(self.parameter.as_borrowed_raw().as_ptr()) };
        let expected_owner = match self.owner.as_ref() {
            Some(owner) => unsafe { owner.as_borrowed_unknown() }
                .map_err(|error| error.to_string())?
                .as_ptr(),
            None => ptr::null_mut(),
        };
        if current != self.parameter.as_borrowed_raw().as_ptr() || current_owner != expected_owner {
            return Err("registered button parameter binding changed".into());
        }
        if let Some(expected_tag) = self.expected_tag {
            let get_tag: unsafe extern "C" fn(*mut c_void) -> i32 = unsafe { method(current, 13) };
            let tag = unsafe { get_tag(current) };
            if tag != expected_tag {
                return Err("registered button tag changed".into());
            }
            let get_name: unsafe extern "C" fn(*mut c_void) -> *const PlainCString =
                unsafe { method(current, 4) };
            let actual = unsafe { get_name(current) };
            if actual.is_null() || unsafe { (*actual).text }.is_null() {
                return Err("registered button name changed".into());
            }
            for (index, expected) in self.name.to_bytes_with_nul().iter().enumerate() {
                if unsafe { *(*actual).text.add(index) } as u8 != *expected {
                    return Err("registered button name changed".into());
                }
            }
        }
        Ok(())
    }

    fn check_owner<G: FnMut() -> Result<(), String>>(&self, guard: &mut G) -> Result<(), String> {
        match self.owner.as_ref() {
            Some(owner) => owner.check(guard).map_err(|error| error.to_string())?,
            None => match self.parameter.controller_owner(guard) {
                Ok(None) => {}
                Ok(Some(owner)) => {
                    owner.release(guard).map_err(|error| error.to_string())?;
                    return Err("button parameter controller association changed".into());
                }
                Err(error) => return Err(error.to_string()),
            },
        }
        Ok(())
    }
}

pub(super) fn execute(
    ctx: &Context<'_>,
    provider: &Provider,
    action: Action,
    chorder_guard: Option<&ClearGuard>,
) -> (Value, Option<ActionLease>) {
    if let Err(reason) = validate_clear_intent(action, chorder_guard) {
        return (refused(action, "invalid_action", &reason), None);
    }
    let spec = match registered(&ctx.instance.class_uid, action) {
        Ok(spec) => spec,
        Err(reason) => return (refused(action, "unregistered_action", &reason), None),
    };
    let native_class = if action == Action::ClearChorderSelected {
        "MusicEffect"
    } else {
        "AudioEffect"
    };
    if spec.expected_tag.is_some() && ctx.instance.native_class != native_class {
        return (
            refused(
                action,
                "unregistered_action",
                "the button requires the registered native class",
            ),
            None,
        );
    }
    let controller = match provider {
        Provider::DspValue(controller) => controller,
        _ => {
            return (
                refused(
                    action,
                    "unsupported_source",
                    "button actions require direct DSP access",
                ),
                None,
            );
        }
    };
    let mut guard = || {
        controller.check()?;
        ctx.check()
    };
    let Some(clear) = chorder_guard else {
        return run(controller, ctx.api, action, spec, &mut guard);
    };
    let selection =
        match ClearSelection::capture(controller, ctx, clear.expected_input_pitch, &mut guard) {
            Ok(selection) => selection,
            Err(reason) => return (refused(action, "chorder_selection_changed", &reason), None),
        };
    run_checked(
        controller,
        ctx.api,
        action,
        spec,
        &mut || {
            guard()?;
            selection.check()
        },
        |button| {
            if button.value != button.min {
                return Err("Chorder Clear is already active".into());
            }
            let (observed, alias) =
                chorder_chords::get(ctx, u32::from(clear.expected_input_pitch), 1)
                    .map_err(|failure| failure.message().to_owned())?;
            drop(alias);
            ctx.check()?;
            selection.check()?;
            if observed["model_sha256"] != clear.expected_model_sha256.as_str() {
                return Err("Chorder model SHA-256 changed".into());
            }
            Ok(())
        },
    )
}

struct ClearSelection {
    mode: parameters::ScalarReadLease,
    input: parameters::ScalarReadLease,
}
impl ClearSelection {
    fn capture<G: FnMut() -> Result<(), String>>(
        controller: &Controller,
        ctx: &Context<'_>,
        expected_pitch: u8,
        guard: &mut G,
    ) -> Result<Self, String> {
        const MODE: &str = "chorder.learnMode";
        const INPUT: parameters::IntegerEditProfile = parameters::IntegerEditProfile {
            name: "chorder.learnInputPitch",
            tag: 7,
            min: 0,
            max: 128,
            default: 0,
        };

        guard()?;
        let mode = controller
            .find(MODE, ctx.api)?
            .pin_raw_toggle_state(MODE, true, guard)
            .map_err(|error| error.to_string())?;
        if mode.description().tag != 6 {
            return Err("Chorder Learn Mode tag changed".into());
        }
        guard()?;
        // The DSP selector uses zero for no selection and MIDI pitch + 1 for a key.
        let input = controller
            .find(INPUT.name, ctx.api)?
            .pin_raw_integer_float_state(INPUT, i64::from(expected_pitch) + 1, guard)
            .map_err(|error| error.to_string())?;
        let selection = Self { mode, input };
        selection.check()?;
        guard()?;
        Ok(selection)
    }

    fn check(&self) -> Result<(), String> {
        self.mode
            .check_borrowed()
            .map_err(|error| error.to_string())?;
        self.input
            .check_borrowed()
            .map_err(|error| error.to_string())
    }
}
fn run<G: FnMut() -> Result<(), String>>(
    controller: &Controller,
    api: &Api,
    action: Action,
    spec: Spec,
    guard: &mut G,
) -> (Value, Option<ActionLease>) {
    run_checked(controller, api, action, spec, guard, |_| Ok(()))
}

fn run_checked<G, C>(
    controller: &Controller,
    api: &Api,
    action: Action,
    spec: Spec,
    guard: &mut G,
    check_intent: C,
) -> (Value, Option<ActionLease>)
where
    G: FnMut() -> Result<(), String>,
    C: FnOnce(&parameters::PulseState) -> Result<(), String>,
{
    let parameter = match guard()
        .and_then(|_| controller.find(spec.parameter.to_str().expect("fixed ASCII parameter"), api))
    {
        Ok(parameter) => parameter,
        Err(reason) => {
            return (
                refused(action, "action_parameter_unavailable", &reason),
                None,
            );
        }
    };
    let lease = match ActionLease::capture(parameter, spec, guard) {
        Ok(lease) => lease,
        Err(reason) => {
            return (refused(action, "action_owner_unavailable", &reason), None);
        }
    };
    let result = lease
        .parameter
        .pulse_range_checked(&mut || lease.check(controller, guard), check_intent);
    let mut value = match result {
        Ok(receipt) => json!({
            "ok":true,"outcome":"applied_unverified","verified":false,"effects_verified":false,
            "before":state(&receipt.before),"after":state(&receipt.after),
            "upper_dispatched":receipt.upper_dispatched,"reset_dispatched":receipt.reset_dispatched,
            "reset_verified":receipt.reset_verified,
        }),
        Err(receipt) => {
            let outcome = match receipt.outcome {
                parameters::Outcome::NotApplied => "not_applied",
                parameters::Outcome::Unknown => "unknown",
            };
            let mut value = error("native_action_error", &receipt.error.to_string(), outcome);
            value["upper_dispatched"] = json!(receipt.upper_dispatched);
            value["reset_dispatched"] = json!(receipt.reset_dispatched);
            value["reset_verified"] = json!(false);
            value["effects_verified"] = json!(false);
            value["verified"] = json!(false);
            if let Some(before) = receipt.before {
                value["before"] = state(&before);
            }
            if let Some(after) = receipt.after {
                value["after"] = state(&after);
            }
            value
        }
    };
    value["action"] = json!(action.name());
    value["param"] = json!(spec.parameter.to_str().expect("fixed ASCII parameter"));
    value["effect_parameters"] = json!(spec.effects);
    if let Err(reason) = lease.check(controller, guard) {
        value["ok"] = json!(false);
        value["outcome"] = json!(if value["upper_dispatched"] == true {
            "unknown"
        } else {
            "not_applied"
        });
        value["error"] = json!({"code":"action_context_changed","message":reason});
    }
    (value, Some(lease))
}
fn refused(action: Action, code: &str, reason: &str) -> Value {
    let mut value = error(code, reason, "not_applied");
    value["action"] = json!(action.name());
    value["verified"] = json!(false);
    value["effects_verified"] = json!(false);
    value["upper_dispatched"] = json!(false);
    value["reset_dispatched"] = json!(false);
    value["reset_verified"] = json!(false);
    value
}
fn state(state: &parameters::PulseState) -> Value {
    json!({
        "name":state.name,"tag":state.tag,"native_kind":format!("{:?}",state.kind),
        "enabled":state.enabled,"readOnly":state.read_only,
        "native_value":typed(Some(&state.value)),"native_min":typed(Some(&state.min)),
        "native_max":typed(Some(&state.max)),
    })
}

#[cfg(test)]
#[path = "action_tests.rs"]
mod tests;

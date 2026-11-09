use super::*;
use std::ffi::CStr;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum Action {
    SwapFilterFrequencies,
}
impl Action {
    fn name(self) -> &'static str {
        match self {
            Self::SwapFilterFrequencies => "swap_filter_frequencies",
        }
    }
}
#[derive(Clone, Copy)]
struct Spec {
    parameter: &'static CStr,
    effects: [&'static str; 2],
}
fn registered(class_uid: &str, action: Action) -> Result<Spec, String> {
    match (class_uid, action) {
        (
            "831A0046-BD71-45BF-A419-6FC90D6E9F2F" | "132C0E1F-06FD-4C96-BD65-678C04FE1C92",
            Action::SwapFilterFrequencies,
        ) => Ok(Spec {
            parameter: c"swapfreqs",
            effects: ["filterlow", "filterhigh"],
        }),
        ("36F3F4D1-CBB4-4BF7-A7E3-EBCCED53718B", Action::SwapFilterFrequencies) => Ok(Spec {
            parameter: c"studiocomp.swapfreqs",
            effects: [
                "studiocomp.sidechainfreqlow",
                "studiocomp.sidechainfreqhigh",
            ],
        }),
        _ => Err("button action is not registered for this exact native class".into()),
    }
}

pub(super) struct ActionLease {
    parameter: RawParameter,
    owner: Option<parameters::ControllerOwner>,
    name: &'static CStr,
}
impl ActionLease {
    fn capture<G: FnMut() -> Result<(), String>>(
        parameter: RawParameter,
        name: &'static CStr,
        guard: &mut G,
    ) -> Result<Self, String> {
        match parameter.controller_owner(guard) {
            Ok(owner) => Ok(Self {
                parameter,
                owner,
                name,
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
) -> (Value, Option<ActionLease>) {
    let spec = match registered(&ctx.instance.class_uid, action) {
        Ok(spec) => spec,
        Err(reason) => return (refused(action, "unregistered_action", &reason), None),
    };
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
    run(controller, ctx.api, action, spec, &mut || {
        controller.check()?;
        ctx.check()
    })
}
fn run<G: FnMut() -> Result<(), String>>(
    controller: &Controller,
    api: &Api,
    action: Action,
    spec: Spec,
    guard: &mut G,
) -> (Value, Option<ActionLease>) {
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
    let lease = match ActionLease::capture(parameter, spec.parameter, guard) {
        Ok(lease) => lease,
        Err(reason) => {
            return (refused(action, "action_owner_unavailable", &reason), None);
        }
    };
    let result = lease
        .parameter
        .pulse_range(&mut || lease.check(controller, guard));
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

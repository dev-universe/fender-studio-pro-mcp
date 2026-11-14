use super::*;
use std::cell::Cell;

const CHORDER: &str = "D2D9B002-12FC-4F1D-AFDA-70336D1DC108";
const PARAMETER: &std::ffi::CStr = c"chorder.autofill";

pub(super) fn validate(param: &str, value: f64) -> Result<(), String> {
    if param == "chorder.autofill" && value.is_finite() && (value == 0.0 || value == 1.0) {
        Ok(())
    } else {
        Err("numeric edit pilot requires chorder.autofill raw0 or1".into())
    }
}

struct Alias {
    identity: Owned,
    plugin: Owned,
    interface: Owned,
}
struct QueryObservation {
    status: i32,
    owner: Option<Owned>,
}
impl QueryObservation {
    fn capture(object: *mut c_void, iid: &Uid) -> Self {
        let mut output = ptr::null_mut();
        let status = unsafe { (unknown_vtable(object).query)(object, iid, &mut output) };
        Self {
            status,
            owner: (!output.is_null()).then(|| Owned(output)),
        }
    }
    fn matches(&self, expected: Option<*mut c_void>) -> bool {
        match (self.status, self.owner.as_ref(), expected) {
            (OK, Some(owner), Some(expected)) => owner.0 == expected,
            (NO_INTERFACE, None, None) => true,
            _ => false,
        }
    }
}
struct RouteWitness {
    _owners: Vec<Owned>,
    final_controller: Option<QueryObservation>,
    final_alias: Option<QueryObservation>,
}
struct EditRoute {
    witness: Cell<Option<RouteWitness>>,
    controller_identity: Owned,
    controller: Controller,
    alias: Option<Alias>,
    parent_identity: Owned,
    parent: Owned,
    node_identity: Owned,
    node: Owned,
    target_identity: Owned,
    target: Owned,
}

fn release<T, G: FnMut() -> Result<(), String>>(value: T, guard: &mut G) -> Result<(), String> {
    let before = guard();
    drop(value);
    let after = guard();
    before.and(after)
}
fn interface<G: FnMut() -> Result<(), String>>(
    object: *mut c_void,
    iid: &Uid,
    guard: &mut G,
) -> Result<Option<Owned>, String> {
    guard()?;
    let mut output = ptr::null_mut();
    let status = unsafe { (unknown_vtable(object).query)(object, iid, &mut output) };
    let owner = (!output.is_null()).then(|| Owned(output));
    if let Err(reason) = guard() {
        release(owner, guard)?;
        return Err(reason);
    }
    match (status, owner) {
        (OK, Some(owner)) => Ok(Some(owner)),
        (NO_INTERFACE, None) => Ok(None),
        (_, owner) => {
            release(owner, guard)?;
            Err("numeric edit route interface query failed".into())
        }
    }
}
fn required<G: FnMut() -> Result<(), String>>(
    object: *mut c_void,
    iid: &Uid,
    guard: &mut G,
) -> Result<Owned, String> {
    interface(object, iid, guard)?.ok_or("numeric edit route interface unavailable".into())
}
fn pin<G: FnMut() -> Result<(), String>>(
    pointer: *mut c_void,
    guard: &mut G,
) -> Result<Owned, String> {
    let owner = hold(pointer).ok_or("numeric edit route object unavailable".to_owned());
    if let Err(reason) = guard() {
        release(owner, guard)?;
        return Err(reason);
    }
    owner
}
fn borrowed(object: *mut c_void, slot: usize) -> *mut c_void {
    let get: Borrowed = unsafe { method(object, slot) };
    unsafe { get(object) }
}

impl EditRoute {
    fn capture<G: FnMut() -> Result<(), String>>(
        target: *mut c_void,
        node: *mut c_void,
        guard: &mut G,
    ) -> Result<Self, String> {
        guard()?;
        let captured = (|| {
            let target = pin(target, guard)?;
            let target_identity = required(target.0, &UNKNOWN, guard)?;
            let node = pin(node, guard)?;
            let node_identity = required(node.0, &UNKNOWN, guard)?;
            let parent = pin(borrowed(node.0, 6), guard)?;
            let parent_identity = required(parent.0, &UNKNOWN, guard)?;
            let alias = match interface(target.0, &ALIAS, guard)? {
                None => None,
                Some(interface) => {
                    let plugin = pin(borrowed(interface.0, 3), guard)?;
                    let identity = required(plugin.0, &UNKNOWN, guard)?;
                    Some(Alias {
                        identity,
                        plugin,
                        interface,
                    })
                }
            };
            let handle = required(target.0, &CONTROLLER, guard)?;
            let controller_identity = required(handle.0, &UNKNOWN, guard)?;
            let route = Self {
                witness: Cell::new(None),
                controller_identity,
                controller: Controller {
                    handle,
                    lease: None,
                },
                alias,
                parent_identity,
                parent,
                node_identity,
                node,
                target_identity,
                target,
            };
            route.witness.set(Some(route.observe(guard)?));
            Ok(route)
        })();
        if let Err(reason) = guard() {
            release(captured, guard)?;
            return Err(reason);
        }
        captured
    }

    fn find(&self) -> *mut c_void {
        let name = PlainCString {
            text: PARAMETER.as_ptr(),
            owner: ptr::null_mut(),
        };
        let find: unsafe extern "C" fn(*mut c_void, *const PlainCString) -> *mut c_void =
            unsafe { method(self.controller.handle.0, 5) };
        unsafe { find(self.controller.handle.0, &name) }
    }

    fn observe<G: FnMut() -> Result<(), String>>(
        &self,
        guard: &mut G,
    ) -> Result<RouteWitness, String> {
        let observed = (|| {
            let target = required(self.target.0, &UNKNOWN, guard)?;
            let node = required(self.node.0, &UNKNOWN, guard)?;
            let parent = required(self.parent.0, &UNKNOWN, guard)?;
            let controller = required(self.target.0, &CONTROLLER, guard)?;
            let controller_identity = required(controller.0, &UNKNOWN, guard)?;
            let alias = interface(self.target.0, &ALIAS, guard)?;
            let alias_identity = match self.alias.as_ref() {
                Some(alias) => Some(required(alias.plugin.0, &UNKNOWN, guard)?),
                None => None,
            };
            let matches = target.0 == self.target_identity.0
                && node.0 == self.node_identity.0
                && parent.0 == self.parent_identity.0
                && controller.0 == self.controller.handle.0
                && controller_identity.0 == self.controller_identity.0
                && match (&self.alias, &alias, &alias_identity) {
                    (None, None, None) => true,
                    (Some(expected), Some(alias), Some(identity)) => {
                        expected.interface.0 == alias.0 && expected.identity.0 == identity.0
                    }
                    _ => false,
                };
            Ok::<_, String>((
                matches,
                (
                    target,
                    node,
                    parent,
                    controller,
                    controller_identity,
                    alias,
                    alias_identity,
                ),
            ))
        })();
        match observed {
            Ok((
                true,
                (target, node, parent, controller, controller_identity, alias, alias_identity),
            )) => {
                let mut owners = vec![target, node, parent, controller, controller_identity];
                owners.extend(alias);
                owners.extend(alias_identity);
                Ok(RouteWitness {
                    _owners: owners,
                    final_controller: None,
                    final_alias: None,
                })
            }
            other => {
                let reason = other
                    .as_ref()
                    .err()
                    .cloned()
                    .unwrap_or("numeric edit route identity changed".into());
                release(other, guard)?;
                Err(reason)
            }
        }
    }

    fn check<G: FnMut() -> Result<(), String>, D: FnMut() -> Result<(), String>>(
        &self,
        parameter: *mut c_void,
        guard: &mut G,
        document: &mut D,
    ) -> Result<(), String> {
        // Release the previous observation before observing its replacement.
        // The final QI owners stay pinned through the reply and its receipt.
        release(self.witness.take(), guard)?;
        let mut witness = self.observe(guard)?;
        guard()?;
        witness.final_controller = Some(QueryObservation::capture(self.target.0, &CONTROLLER));
        witness.final_alias = Some(QueryObservation::capture(self.target.0, &ALIAS));
        let matches = witness
            .final_controller
            .as_ref()
            .expect("captured controller")
            .matches(Some(self.controller.handle.0))
            && witness
                .final_alias
                .as_ref()
                .expect("captured alias")
                .matches(self.alias.as_ref().map(|alias| alias.interface.0));
        let reentered = self.witness.replace(Some(witness));
        if reentered.is_some() {
            release(reentered, guard)?;
            return Err("numeric edit route check reentered".into());
        }
        if !matches {
            return Err("numeric edit route interface attachment changed".into());
        }
        if borrowed(self.node.0, 6) != self.parent.0
            || self
                .alias
                .as_ref()
                .is_some_and(|alias| borrowed(alias.interface.0, 3) != alias.plugin.0)
            || self.find() != parameter
        {
            return Err("numeric edit route attachment changed".into());
        }
        document()
    }
}

pub(super) struct EditLease {
    parameter: parameters::NumericEditLease,
    route: EditRoute,
    pointer: *mut c_void,
}
impl EditLease {
    pub(super) fn phase(&self) -> &'static str {
        self.parameter.phase().as_str()
    }
    pub(super) fn began(&self) -> bool {
        self.parameter.phase().effect_may_have_started()
    }
    pub(super) fn check(&self, ctx: &Context<'_>) -> Result<(), String> {
        self.parameter
            .check(&mut || {
                self.route
                    .check(self.pointer, &mut || ctx.check(), &mut || {
                        ctx.check_document()
                    })
            })
            .map_err(|error| error.to_string())?;
        ctx.check_document()
    }
}

pub(super) fn execute(ctx: &Context<'_>, param: &str, value: f64) -> (Value, Option<EditLease>) {
    if ctx.instance.class_uid != CHORDER {
        return (
            error(
                "unregistered_numeric_edit",
                "numeric edit pilot requires the exact Chorder class",
                "not_applied",
            ),
            None,
        );
    }
    if let Err(reason) = validate(param, value) {
        return (error("invalid_numeric_edit", &reason, "not_applied"), None);
    }
    let route = match EditRoute::capture(ctx.target.0, ctx.node.0, &mut || ctx.check()) {
        Ok(route) => route,
        Err(reason) => {
            return (
                error("numeric_edit_route_unavailable", &reason, "not_applied"),
                None,
            );
        }
    };
    let resolved = ctx
        .check()
        .and_then(|_| route.controller.find(param, ctx.api));
    let parameter = match resolved {
        Ok(parameter) => parameter,
        Err(reason) => {
            let released = release(route, &mut || ctx.check());
            return (
                error(
                    "numeric_edit_parameter_unavailable",
                    &released.err().unwrap_or(reason),
                    "not_applied",
                ),
                None,
            );
        }
    };
    let pointer = parameter.as_borrowed_raw().as_ptr();
    let attempt = parameter.edit_raw_number(parameters::Number::Float(value), &mut || {
        route.check(pointer, &mut || ctx.check(), &mut || ctx.check_document())
    });
    let lease = EditLease {
        parameter: attempt.lease,
        route,
        pointer,
    };
    let mut value = match attempt.result {
        Ok(readback) => {
            json!({"ok":true,"before":description(&readback.before),"after":description(&readback.after),"verified":readback.verified,"outcome":"observed"})
        }
        Err(failure) => {
            let outcome = if lease.began() || failure.outcome == parameters::Outcome::Unknown {
                "unknown"
            } else {
                "not_applied"
            };
            let mut value = error(
                "native_numeric_edit_error",
                &failure.error.to_string(),
                outcome,
            );
            if let Some(before) = failure.before {
                value["before"] = description(&before);
            }
            if let Some(after) = failure.after {
                value["after"] = description(&after);
            }
            value
        }
    };
    value["edit_phase"] = json!(lease.phase());
    (value, Some(lease))
}

#[cfg(test)]
#[path = "numeric_edit_tests.rs"]
mod tests;

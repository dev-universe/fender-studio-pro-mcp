use super::*;

const API: &str = "exact-level-controller-parameters-v1";
const COUNTS: [i32; 2] = [8, 25];

pub(super) struct Parameter {
    pub(super) parameter: RawParameter,
    pub(super) owner: Option<parameters::ControllerOwner>,
    pub(super) owner_view: *mut c_void,
    pub(super) node: usize,
    pub(super) index: i32,
}

pub(super) struct Descriptions {
    parameters: Vec<Parameter>,
    value: Value,
}

fn count(controller: *mut c_void) -> i32 {
    let get: Count = unsafe { method(controller, 3) };
    unsafe { get(controller) }
}
fn at(controller: *mut c_void, index: i32) -> *mut c_void {
    let get: unsafe extern "C" fn(*mut c_void, i32) -> *mut c_void =
        unsafe { method(controller, 4) };
    unsafe { get(controller, index) }
}

impl Parameter {
    pub(super) fn check<G: FnMut() -> Result<(), String>>(
        &self,
        pair: &Pair,
        guard: &mut G,
    ) -> Result<(), String> {
        guard()?;
        let controller = pair.capabilities[self.node]
            .interface
            .as_ref()
            .ok_or("Level controller lost")?;
        if count(controller.0) != COUNTS[self.node] {
            return Err("Level parameter count changed".into());
        }
        guard()?;
        let matches = at(controller.0, self.index) == self.parameter.as_borrowed_raw().as_ptr();
        guard()?;
        if !matches {
            return Err("Level indexed parameter association changed".into());
        }
        if let Some(owner) = &self.owner {
            owner.check(guard).map_err(|e| e.to_string())?;
        } else {
            let owner = self
                .parameter
                .controller_owner(guard)
                .map_err(|e| e.to_string())?;
            let absent = owner.is_none();
            release(owner, guard)?;
            if !absent {
                return Err("Level parameter owner changed".into());
            }
        }
        guard()?;
        if at(controller.0, self.index) != self.parameter.as_borrowed_raw().as_ptr() {
            return Err("Level indexed parameter association changed".into());
        }
        guard()?;
        if count(controller.0) != COUNTS[self.node] {
            return Err("Level parameter count changed".into());
        }
        guard()
    }
}

impl Descriptions {
    pub(super) fn attachments(&self, pair: &Pair) -> Result<(), String> {
        // Slot14 is the declared IParameter.getController used by ControllerOwner.
        // Only compare the borrowed view retained by that typed owner.
        for parameter in &self.parameters {
            let controller = pair.capabilities[parameter.node]
                .interface
                .as_ref()
                .ok_or("Level controller lost")?;
            if count(controller.0) != COUNTS[parameter.node]
                || at(controller.0, parameter.index)
                    != parameter.parameter.as_borrowed_raw().as_ptr()
                || borrowed(parameter.parameter.as_borrowed_raw().as_ptr(), 14)
                    != parameter.owner_view
            {
                return Err("Level indexed parameter attachment changed".into());
            }
        }
        Ok(())
    }
    pub(super) fn check<G: FnMut() -> Result<(), String>>(
        &self,
        pair: &Pair,
        guard: &mut G,
    ) -> Result<(), String> {
        for parameter in &self.parameters {
            parameter.check(pair, guard)?;
        }
        guard()
    }
}

impl Observation {
    pub(in crate::native_script::request::alias_controller) fn describe<
        G: FnMut() -> Result<(), String>,
    >(
        &mut self,
        text: parameters::TextApi,
        guard: &mut G,
    ) -> Result<(), String> {
        self.check(guard)?;
        let mut descriptions = Descriptions {
            parameters: Vec::new(),
            value: Value::Null,
        };
        let mut nodes = Vec::new();
        let mut complete = self.pair.is_some();
        if let Some(pair) = &self.pair {
            for node in 0..2 {
                let capability = &pair.capabilities[node];
                let expected = COUNTS[node];
                let observed = capability.value["count"].as_i64();
                let Some(controller) = capability
                    .interface
                    .as_ref()
                    .filter(|_| observed == Some(i64::from(expected)))
                else {
                    complete = false;
                    nodes.push(json!({"node_index":node,"expected_count":expected,"observed_count":observed,
                        "status":"unresolved_count_or_interface","parameters":null}));
                    continue;
                };
                let mut records = Vec::new();
                for index in 0..expected {
                    self.check(guard)?;
                    guard()?;
                    if count(controller.0) != expected {
                        return Err("Level parameter count changed".into());
                    }
                    guard()?;
                    let owned = pin(at(controller.0, index), guard)?;
                    let pointer = NonNull::new(owned.0).expect("retained parameter");
                    std::mem::forget(owned);
                    let parameter = unsafe { RawParameter::from_owned(pointer, text) };
                    let owner = parameter
                        .controller_owner(guard)
                        .map_err(|e| e.to_string())?;
                    let owner_view = owner
                        .as_ref()
                        .map(|owner| {
                            unsafe { owner.as_borrowed_unknown() }
                                .map(|p| p.as_ptr())
                                .map_err(|e| e.to_string())
                        })
                        .transpose()?
                        .unwrap_or(ptr::null_mut());
                    let owner_matches = if let Some(owner) = &owner {
                        let pointer =
                            unsafe { owner.as_borrowed_unknown() }.map_err(|e| e.to_string())?;
                        let canonical = interface(pointer.as_ptr(), &UNKNOWN, guard)?;
                        let expected_identity = if node == 0 {
                            pair.root_identity.0
                        } else {
                            pair.child_identity.0
                        };
                        let matches = canonical.0 == expected_identity;
                        release(canonical, guard)?;
                        Some(matches)
                    } else {
                        None
                    };
                    let parameter = Parameter {
                        parameter,
                        owner,
                        owner_view,
                        node,
                        index,
                    };
                    parameter.check(pair, guard)?;
                    self.check(guard)?;
                    let read = parameter.parameter.describe();
                    guard()?;
                    parameter.check(pair, guard)?;
                    self.check(guard)?;
                    let description = read.map_err(|error| {
                        format!("Level node {node} parameter {index} description failed: {error}")
                    })?;
                    let current_available = matches!(
                        description.raw.as_ref().map(|v| &v.value),
                        Some(
                            parameters::Value::Integer(_)
                                | parameters::Value::Float(_)
                                | parameters::Value::Text(_)
                        )
                    );
                    complete &= current_available;
                    let value = super::super::description(&description);
                    records.push(json!({"index":index,"status":if current_available {"observed"} else {"current_unavailable"},
                        "controller_owner_present":parameter.owner.is_some(),"controller_owner_canonical_matches_receiver":owner_matches,
                        "description":value,"can_set_via_this_api":false,"mutation_capability":false}));
                    descriptions.parameters.push(parameter);
                }
                nodes.push(json!({"node_index":node,"expected_count":expected,"observed_count":observed,
                    "status":if records.iter().all(|v| v["status"] == "observed") {"observed"} else {"incomplete"},"parameters":records}));
            }
        }
        descriptions.value = json!({"api":API,"status":if self.pair.is_none() {"unsupported_shape"} else if complete {"observed"} else {"incomplete"},
            "complete":complete,"max_total_parameters":33,"nodes":nodes,"mutation_capability":false,"atomic_snapshot":false});
        self.descriptions = Some(descriptions);
        self.check(guard)
    }

    pub(in crate::native_script::request::alias_controller) fn descriptions_value(
        &self,
    ) -> Option<Value> {
        self.descriptions.as_ref().map(|d| d.value.clone())
    }
}

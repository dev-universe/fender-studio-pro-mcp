use super::*;
use fender_native_node_api::{Inspection, Limits, NodeInspector, TextApi};

enum Observation {
    Absent,
    Present {
        inspection: Inspection,
        owner: parameters::ControllerOwner,
    },
}

/// Owns the same resolved parameter and its read-only controller observation.
/// These pins remain attached to the reply through native output publication.
pub(super) struct OwnerInspection {
    observation: Observation,
    parameter: RawParameter,
    parameter_native_class: String,
}

pub(super) fn text_api() -> Result<TextApi, String> {
    let get = constant_string().ok_or("native string constants unavailable")?;
    Ok(TextApi {
        get_constant_string: unsafe {
            std::mem::transmute::<
                ConstantString,
                unsafe extern "C" fn(*const c_char) -> *const c_void,
            >(get)
        },
    })
}

pub(super) fn invalidate(value: &mut Value) {
    if let Some(value) = value.as_object_mut() {
        for key in [
            "owner_scope",
            "controller_report",
            "controller",
            "parameter_native_class",
        ] {
            value.remove(key);
        }
    }
}

impl OwnerInspection {
    pub(super) fn capture<G: FnMut() -> Result<(), String>>(
        parameter: RawParameter,
        text: TextApi,
        guard: &mut G,
    ) -> Result<Self, String> {
        let observation = match parameter.controller_owner(guard) {
            Ok(None) => Ok(Observation::Absent),
            Err(error) => Err(error.to_string()),
            Ok(Some(owner)) => match reflect(&owner, text, guard) {
                Ok(inspection) => Ok(Observation::Present { inspection, owner }),
                Err(reason) => match owner.release(guard) {
                    Ok(()) => Err(reason),
                    Err(error) => Err(error.to_string()),
                },
            },
        };
        match observation {
            Ok(observation) => {
                let mut lease = Self {
                    observation,
                    parameter,
                    parameter_native_class: String::new(),
                };
                let checked = (|| {
                    guard()?;
                    lease.parameter_native_class =
                        class_name(lease.parameter.as_borrowed_raw().as_ptr());
                    guard()?;
                    lease.check(guard)
                })();
                if let Err(reason) = checked {
                    lease.release(guard)?;
                    return Err(reason);
                }
                Ok(lease)
            }
            Err(reason) => {
                release(parameter, guard)?;
                Err(reason)
            }
        }
    }

    pub(super) fn check<G: FnMut() -> Result<(), String>>(
        &self,
        guard: &mut G,
    ) -> Result<(), String> {
        match &self.observation {
            Observation::Absent => match self.parameter.controller_owner(guard) {
                Ok(None) => guard(),
                Ok(Some(owner)) => {
                    owner.release(guard).map_err(|error| error.to_string())?;
                    Err("parameter controller association changed".into())
                }
                Err(error) => Err(error.to_string()),
            },
            Observation::Present { inspection, owner } => {
                owner.check(guard).map_err(|error| error.to_string())?;
                inspection
                    .check(&mut || owner.check(guard).map_err(|error| error.to_string()))
                    .map_err(|error| error.to_string())?;
                owner.check(guard).map_err(|error| error.to_string())?;
                guard()
            }
        }
    }

    pub(super) fn value(&self) -> Value {
        let mut value = json!({
            "owner_scope":"parameter_controller_observation",
            "mutation_capability":false,
            "parameter_native_class":self.parameter_native_class,
        });
        match &self.observation {
            Observation::Absent => value["controller"] = Value::Null,
            Observation::Present { inspection, .. } => {
                let mut report =
                    node_inspect::report(inspection.report(), node_inspect::Scope::Target);
                report["coverage"] = json!("bounded_parameter_controller_metadata");
                report["scope"] = json!("parameter_controller_observation");
                report["mutation_capability"] = json!(false);
                report["limits"] = json!({"nodes":1,"depth":0,"ancestors":0,"children_per_node":1});
                value["controller_report"] = report;
            }
        }
        value
    }

    pub(super) fn release<G: FnMut() -> Result<(), String>>(
        self,
        guard: &mut G,
    ) -> Result<(), String> {
        let Self {
            observation,
            parameter,
            ..
        } = self;
        let observation = match observation {
            Observation::Absent => Ok(()),
            Observation::Present { inspection, owner } => {
                let inspection = release(inspection, guard);
                let owner = owner.release(guard).map_err(|error| error.to_string());
                inspection.and(owner)
            }
        };
        let parameter = release(parameter, guard);
        observation.and(parameter)
    }
}

fn reflect<G: FnMut() -> Result<(), String>>(
    owner: &parameters::ControllerOwner,
    text: TextApi,
    guard: &mut G,
) -> Result<Inspection, String> {
    guard()?;
    let borrowed = unsafe { owner.as_borrowed_unknown() }.map_err(|error| error.to_string())?;
    let inspector = hold(borrowed.as_ptr())
        .ok_or("parameter controller unavailable".to_owned())
        .map(|owned| {
            let pointer = NonNull::new(owned.0).expect("held controller");
            std::mem::forget(owned);
            unsafe { NodeInspector::from_owned(pointer, text) }
        });
    let result = (|| {
        guard()?;
        let inspector = inspector.as_ref().map_err(Clone::clone)?;
        inspector
            .inspect(
                Limits {
                    nodes: 1,
                    depth: 0,
                    ancestors: 0,
                    children_per_node: 1,
                },
                &mut || owner.check(guard).map_err(|error| error.to_string()),
            )
            .map_err(|error| error.to_string())
    })();
    let before_release = guard();
    drop(inspector);
    let after_release = guard();
    if let Err(reason) = before_release.and(after_release) {
        release(result, guard)?;
        return Err(reason);
    }
    result
}

fn release<T, G: FnMut() -> Result<(), String>>(value: T, guard: &mut G) -> Result<(), String> {
    let before = guard();
    drop(value);
    let after = guard();
    before.and(after)
}

#[cfg(test)]
#[path = "owner_inspect_tests.rs"]
mod tests;

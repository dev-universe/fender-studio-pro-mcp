//! Exact Level capability observations. No parameter object is requested.
use super::*;
use fender_native_node_api::{Relation, Report};

const API: &str = "exact-level-controller-capability-v1";
const MAX_PARAMETERS: i32 = 128;

struct Capability {
    interface: Option<Owned>,
    value: Value,
}

fn capability_inner<G: FnMut() -> Result<(), String>>(
    owner: *mut c_void,
    canonical: *mut c_void,
    guard: &mut G,
) -> Result<Capability, String> {
    guard()?;
    let mut out = ptr::null_mut();
    let result = unsafe { (unknown_vtable(owner).query)(owner, &CONTROLLER, &mut out) };
    let handle = if out.is_null() {
        None
    } else {
        Some(Owned(out))
    };
    if let Err(reason) = guard() {
        release(handle, guard)?;
        return Err(reason);
    }
    let status = match (result, handle.is_some()) {
        (NO_INTERFACE, false) => "absent",
        (OK, false) => "malformed_success",
        (NO_INTERFACE, true) => "malformed_absence",
        (OK, true) => {
            let identity = interface(out, &UNKNOWN, guard);
            let matches = identity
                .as_ref()
                .is_ok_and(|identity| identity.0 == canonical);
            let error = identity.as_ref().err().cloned();
            release(identity, guard)?;
            if let Some(error) = error {
                release(handle, guard)?;
                return Err(error);
            }
            if !matches {
                "identity_mismatch"
            } else {
                guard()?;
                let count: Count = unsafe { method(out, 3) };
                let count = unsafe { count(out) };
                if let Err(reason) = guard() {
                    release(handle, guard)?;
                    return Err(reason);
                }
                let status = if count < 0 {
                    "negative_count"
                } else if count > MAX_PARAMETERS {
                    "over_limit_count"
                } else {
                    "present"
                };
                return Ok(Capability {
                    interface: handle,
                    value: json!({"status":status,"count":if status == "present" {Some(count)} else {None},"canonical_identity_matches":true}),
                });
            }
        }
        _ => "qi_failed",
    };
    release(handle, guard)?;
    Ok(Capability {
        interface: None,
        value: json!({"status":status,"count":null,"canonical_identity_matches":if status == "identity_mismatch" {Some(false)} else {None}}),
    })
}

fn capability<G: FnMut() -> Result<(), String>>(
    owner: *mut c_void,
    canonical: *mut c_void,
    guard: &mut G,
) -> Result<Capability, String> {
    let result = capability_inner(owner, canonical, guard);
    if let Err(reason) = guard() {
        release(result, guard)?;
        return Err(reason);
    }
    result
}

fn exact_shape(report: &Report) -> bool {
    report.complete
        && report.nodes.len() == 2
        && report.edges.len() == 1
        && report.edges[0].from == 0
        && report.edges[0].to == 1
        && report.edges[0].relation == Relation::Child(0)
        && [
            ("AudioEffect", "IsLevelMeterComponent", 1),
            ("OutLevels", "LevelMeterComponent", 0),
        ]
        .iter()
        .zip(&report.nodes)
        .all(|((id, family, count), node)| {
            node.identity
                .as_ref()
                .is_some_and(|identity| identity.id == *id)
                && node.child_count == Some(*count)
                && node.reflection.available
                && node
                    .reflection
                    .types
                    .first()
                    .is_some_and(|ty| ty.name == *family)
        })
}

struct Pair {
    root: Owned,
    root_identity: Owned,
    child: Owned,
    child_identity: Owned,
    capabilities: [Capability; 2],
}

pub(in crate::native_script::request) struct Observation {
    pair: Option<Pair>,
}

fn count_children(node: *mut c_void) -> i32 {
    let get: Count = unsafe { method(node, 8) };
    unsafe { get(node) }
}
fn child(node: *mut c_void) -> *mut c_void {
    let get: unsafe extern "C" fn(*mut c_void, i32) -> *mut c_void = unsafe { method(node, 9) };
    unsafe { get(node, 0) }
}

impl Observation {
    pub(super) fn capture<G: FnMut() -> Result<(), String>>(
        binding: &ControllerBinding,
        report: &Report,
        guard: &mut G,
    ) -> Result<Self, String> {
        if !exact_shape(report) {
            return Ok(Self { pair: None });
        }
        let result = (|| {
            let root = interface(binding.controller.0, &NODE, guard)?;
            let root_identity = interface(root.0, &UNKNOWN, guard)?;
            if root_identity.0 != binding.identity.0 {
                return Err("Level root node canonical identity mismatch".into());
            }
            guard()?;
            if count_children(root.0) != 1 {
                return Err("Level root child count changed".into());
            }
            guard()?;
            let child = pin(child(root.0), guard)?;
            let child_node = interface(child.0, &NODE, guard)?;
            release(child, guard)?;
            let child_identity = interface(child_node.0, &UNKNOWN, guard)?;
            let mut observation = Self {
                pair: Some(Pair {
                    root,
                    root_identity,
                    child: child_node,
                    child_identity,
                    capabilities: std::array::from_fn(|_| Capability {
                        interface: None,
                        value: Value::Null,
                    }),
                }),
            };
            observation.check(guard)?;
            let pair = observation.pair.as_mut().expect("exact pair");
            pair.capabilities[0] = capability(pair.root.0, pair.root_identity.0, guard)?;
            pair.capabilities[1] = capability(pair.child.0, pair.child_identity.0, guard)?;
            observation.check(guard)?;
            Ok(observation)
        })();
        if let Err(reason) = guard() {
            release(result, guard)?;
            return Err(reason);
        }
        result
    }

    pub(super) fn check<G: FnMut() -> Result<(), String>>(
        &self,
        guard: &mut G,
    ) -> Result<(), String> {
        let Some(pair) = &self.pair else {
            return guard();
        };
        let result = (|| {
            for (node, canonical, capability) in [
                (&pair.root, &pair.root_identity, &pair.capabilities[0]),
                (&pair.child, &pair.child_identity, &pair.capabilities[1]),
            ] {
                let current = interface(node.0, &UNKNOWN, guard)?;
                let matches = current.0 == canonical.0;
                release(current, guard)?;
                if !matches {
                    return Err("Level node canonical identity changed".into());
                }
                if let Some(controller) = &capability.interface {
                    let current = interface(controller.0, &UNKNOWN, guard)?;
                    let matches = current.0 == canonical.0;
                    release(current, guard)?;
                    if !matches {
                        return Err("Level IController canonical identity changed".into());
                    }
                }
            }
            let parent = pin(borrowed(pair.child.0, 6), guard)?;
            let parent_identity = interface(parent.0, &UNKNOWN, guard)?;
            let matches = parent_identity.0 == pair.root_identity.0;
            release((parent_identity, parent), guard)?;
            if !matches {
                return Err("Level child parent changed".into());
            }
            self.attachments()
        })();
        guard()?;
        result
    }

    pub(super) fn attachments(&self) -> Result<(), String> {
        let Some(pair) = &self.pair else {
            return Ok(());
        };
        let matches = count_children(pair.root.0) == 1
            && count_children(pair.child.0) == 0
            && child(pair.root.0) == pair.child.0
            && borrowed(pair.child.0, 6) == pair.root.0;
        if matches {
            Ok(())
        } else {
            Err("Level fixed root/child attachment changed".into())
        }
    }

    pub(super) fn value(&self) -> Value {
        let nodes = self.pair.as_ref().map(|pair| {
            [
                json!({"node_index":0,"id":"AudioEffect","controller":pair.capabilities[0].value}),
                json!({"node_index":1,"id":"OutLevels","controller":pair.capabilities[1].value}),
            ]
        });
        json!({"api":API,"status":if nodes.is_some() {"observed"} else {"unsupported_shape"},
            "max_count":MAX_PARAMETERS,"nodes":nodes,"mutation_capability":false,"atomic_snapshot":false})
    }
}

#[cfg(test)]
#[path = "level_tests.rs"]
mod tests;

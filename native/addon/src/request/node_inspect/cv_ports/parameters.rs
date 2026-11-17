use super::*;
use fender_native_parameter_api as parameter_api;

const MAX_PARAMETERS_PER_NODE: u32 = 32;
const MAX_PARAMETERS: u32 = 64;
const MAX_RETAINED_TEXT_BYTES: usize = 48 * 1024;

fn strict_query(pointer: *mut c_void, iid: &Uid) -> Result<Option<Owned>, String> {
    let get: unsafe extern "C" fn(*mut c_void, *const Uid, *mut *mut c_void) -> i32 =
        unsafe { method(pointer, 0) };
    let mut out = ptr::null_mut();
    let status = unsafe { get(pointer, iid, &mut out) };
    match (status, out.is_null()) {
        (NO_INTERFACE, true) => Ok(None),
        (OK, false) => Ok(Some(Owned(out))),
        _ => Err("CV port indeterminate interface query".into()),
    }
}
fn canonical(pointer: *mut c_void) -> Result<Owned, String> {
    strict_query(pointer, &UNKNOWN)?.ok_or("CV parameter canonical identity unavailable".into())
}
fn parameter_count(pointer: *mut c_void) -> Result<u32, String> {
    let get: Count = unsafe { method(pointer, 3) };
    let count = unsafe { get(pointer) };
    if count < 0 {
        Err("CV port negative indexed parameter count".into())
    } else {
        Ok(count as u32)
    }
}
fn parameter_at(pointer: *mut c_void, index: u32) -> *mut c_void {
    let get: unsafe extern "C" fn(*mut c_void, i32) -> *mut c_void = unsafe { method(pointer, 4) };
    unsafe { get(pointer, index as i32) }
}

struct ParameterWitness {
    parameter: RawParameter,
    canonical: Owned,
    index: u32,
    observation: Value,
}
impl ParameterWitness {
    fn capture(
        controller: *mut c_void,
        index: u32,
        text: parameter_api::TextApi,
    ) -> Result<Self, String> {
        let pointer = parameter_at(controller, index);
        let held = hold(pointer).ok_or("CV indexed parameter unavailable")?;
        let canonical = canonical(held.0)?;
        let raw = NonNull::new(held.0).expect("retained indexed parameter");
        std::mem::forget(held);
        Ok(Self {
            parameter: unsafe { RawParameter::from_owned(raw, text) },
            canonical,
            index,
            observation: Value::Null,
        })
    }
    fn pointer(&self) -> *mut c_void {
        self.parameter.as_borrowed_raw().as_ptr()
    }
}
struct ControllerWitness {
    parameters: Vec<ParameterWitness>,
    interface: Owned,
    canonical: Owned,
    count: u32,
}
enum DirectController {
    Absent,
    Present(ControllerWitness),
}
struct NodeWitness {
    child_index: Option<usize>,
    snapshot: Snapshot,
    header: Header,
    controller: DirectController,
}
impl NodeWitness {
    fn pin<'a>(&self, route: &'a Route) -> &'a Pin {
        let root = route.root.as_ref().expect("retained PortSetup");
        match self.child_index {
            None => &root.pin,
            Some(index) => &root.children[index].pin,
        }
    }
    fn controller_check<G: FnMut() -> Result<(), String>>(
        &self,
        route: &Route,
        guard: &mut G,
    ) -> Result<(), String> {
        let current = strict_query(self.pin(route).node.0, &CONTROLLER)?;
        guard()?;
        let checked = match (&self.controller, current.as_ref()) {
            (DirectController::Absent, None) => Ok(()),
            (DirectController::Present(expected), Some(current)) => {
                let identity = canonical(current.0)?;
                let same = current.0 == expected.interface.0
                    && identity.0 == expected.canonical.0
                    && identity.0 == self.pin(route).canonical.0;
                drop(identity);
                guard()?;
                if !same || parameter_count(current.0)? != expected.count {
                    Err("CV direct controller identity or count changed".into())
                } else {
                    for parameter in &expected.parameters {
                        let raw = hold(parameter_at(current.0, parameter.index))
                            .ok_or("CV indexed parameter unavailable")?;
                        let identity = canonical(raw.0)?;
                        guard()?;
                        let same =
                            identity.0 == parameter.canonical.0 && raw.0 == parameter.pointer();
                        drop(identity);
                        drop(raw);
                        guard()?;
                        if !same {
                            return Err("CV indexed parameter identity changed".into());
                        }
                    }
                    Ok(())
                }
            }
            _ => Err("CV direct controller presence changed".into()),
        };
        drop(current);
        guard()?;
        checked
    }
    fn borrowed_parameters(&self) -> Result<(), String> {
        if let DirectController::Present(controller) = &self.controller {
            if parameter_count(controller.interface.0)? != controller.count {
                return Err("CV indexed controller count changed after ID copies".into());
            }
            for parameter in &controller.parameters {
                if parameter_at(controller.interface.0, parameter.index) != parameter.pointer() {
                    return Err("CV indexed parameter replaced after ID copies".into());
                }
            }
        }
        Ok(())
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Rows {
    Complete,
    IndexLimit,
    TextLimit,
}
pub(in crate::native_script::request) struct CvPortParametersInventory {
    nodes: Vec<NodeWitness>,
    rows: Rows,
    retained_text_bytes: usize,
    route: Route,
}
impl CvPortParametersInventory {
    pub(in crate::native_script::request) fn capture(
        ctx: &Context<'static>,
    ) -> Result<Self, String> {
        admit(&ctx.instance)?;
        let get = constant_string().ok_or("native string constants unavailable")?;
        let name = unsafe { get(c"PortSetup".as_ptr()) };
        if name.is_null() {
            return Err("fixed PortSetup identifier unavailable".into());
        }
        let route = Route::capture(ctx.target.0, ctx.node.0, name, &mut || ctx.check())?;
        Self::capture_route(route, raw_text(ctx.api), &mut || ctx.check())
    }
    pub(super) fn capture_route<G: FnMut() -> Result<(), String>>(
        route: Route,
        text: parameter_api::TextApi,
        guard: &mut G,
    ) -> Result<Self, String> {
        let mut nodes = Vec::new();
        if let Some(root) = &route.root {
            if root.children.len() != 7 {
                return Err("CV parameter read requires seven direct PortSetup children".into());
            }
            let mut ids = std::collections::BTreeSet::new();
            for child_index in std::iter::once(None).chain((0..7).map(Some)) {
                let pin = child_index.map_or(&root.pin, |index| &root.children[index].pin);
                let snapshot = pin.snapshot()?;
                guard()?;
                if child_index.is_some() {
                    let supported = if snapshot.id == "Input00" {
                        snapshot.first_type == "AudioFXEndPointComponent"
                    } else {
                        [
                            "Output01", "Output02", "Output03", "Output04", "Output05", "Output06",
                        ]
                        .contains(&snapshot.id.as_str())
                            && snapshot.first_type == "AudioOutputNode"
                    };
                    if !supported || !ids.insert(snapshot.id.clone()) {
                        return Err(
                            "CV parameter child identifier or first type unsupported".into()
                        );
                    }
                } else if snapshot != root.snapshot {
                    return Err("CV parameter PortSetup identity changed".into());
                }
                let header = Header::capture(pin)?;
                guard()?;
                let controller = match strict_query(pin.node.0, &CONTROLLER)? {
                    None => DirectController::Absent,
                    Some(interface) => {
                        let identity = canonical(interface.0)?;
                        if identity.0 != pin.canonical.0 {
                            return Err("CV foreign direct controller".into());
                        }
                        let count = parameter_count(interface.0)?;
                        DirectController::Present(ControllerWitness {
                            interface,
                            canonical: identity,
                            count,
                            parameters: Vec::new(),
                        })
                    }
                };
                guard()?;
                nodes.push(NodeWitness {
                    child_index,
                    snapshot,
                    header,
                    controller,
                });
            }
        }
        let total: u64 = nodes
            .iter()
            .map(|node| match &node.controller {
                DirectController::Absent => 0,
                DirectController::Present(controller) => controller.count as u64,
            })
            .sum();
        let limited = total > MAX_PARAMETERS as u64 || nodes.iter().any(|node|
            matches!(&node.controller, DirectController::Present(c) if c.count > MAX_PARAMETERS_PER_NODE));
        let mut inventory = Self {
            nodes,
            rows: if limited {
                Rows::IndexLimit
            } else {
                Rows::Complete
            },
            retained_text_bytes: 0,
            route,
        };
        inventory.check(guard, &mut || Ok(()))?;
        if !limited {
            'nodes: for node_index in 0..inventory.nodes.len() {
                let count = match &inventory.nodes[node_index].controller {
                    DirectController::Absent => continue,
                    DirectController::Present(controller) => controller.count,
                };
                for index in 0..count {
                    guard()?;
                    let interface = match &inventory.nodes[node_index].controller {
                        DirectController::Present(controller) => controller.interface.0,
                        DirectController::Absent => unreachable!(),
                    };
                    let parameter = ParameterWitness::capture(interface, index, text)?;
                    if let DirectController::Present(controller) =
                        &mut inventory.nodes[node_index].controller
                    {
                        controller.parameters.push(parameter);
                    }
                    inventory.check(guard, &mut || Ok(()))?;
                    let result = match &inventory.nodes[node_index].controller {
                        DirectController::Present(controller) => {
                            controller.parameters[index as usize].parameter.describe()
                        }
                        DirectController::Absent => unreachable!(),
                    };
                    inventory.check(guard, &mut || Ok(()))?;
                    let description = result.map_err(|error| error.to_string())?;
                    let observation = observed_parameter(index, &description);
                    drop(description);
                    let bytes = text_bytes(&observation);
                    let text_limited =
                        inventory.retained_text_bytes + bytes > MAX_RETAINED_TEXT_BYTES;
                    if text_limited {
                        drop(observation);
                        inventory.rows = Rows::TextLimit;
                        inventory.retained_text_bytes = 0;
                        for node in &mut inventory.nodes {
                            if let DirectController::Present(controller) = &mut node.controller {
                                for parameter in &mut controller.parameters {
                                    parameter.observation = Value::Null;
                                }
                            }
                        }
                    } else if let DirectController::Present(controller) =
                        &mut inventory.nodes[node_index].controller
                    {
                        controller.parameters[index as usize].observation = observation;
                    }
                    inventory.check(guard, &mut || Ok(()))?;
                    if text_limited {
                        break 'nodes;
                    }
                    inventory.retained_text_bytes += bytes;
                }
            }
        }
        if inventory.rows == Rows::Complete {
            for node_index in 0..inventory.nodes.len() {
                let count = match &inventory.nodes[node_index].controller {
                    DirectController::Absent => continue,
                    DirectController::Present(controller) => controller.parameters.len(),
                };
                for index in 0..count {
                    let details = match &inventory.nodes[node_index].controller {
                        DirectController::Present(controller) => controller.parameters[index]
                            .parameter
                            .details(&mut || inventory.check(guard, &mut || Ok(())))
                            .map_err(|error| error.to_string())?,
                        DirectController::Absent => unreachable!(),
                    };
                    if let DirectController::Present(controller) =
                        &mut inventory.nodes[node_index].controller
                    {
                        controller.parameters[index].observation["details"] =
                            observed_details(&details);
                    }
                }
            }
        }
        inventory.check(guard, &mut || Ok(()))?;
        Ok(inventory)
    }
    pub(in crate::native_script::request) fn check<G, B>(
        &self,
        guard: &mut G,
        borrowed_guard: &mut B,
    ) -> Result<(), String>
    where
        G: FnMut() -> Result<(), String>,
        B: FnMut() -> Result<(), String>,
    {
        guard()?;
        for node in &self.nodes {
            node.controller_check(&self.route, guard)?;
        }
        self.route.metadata(guard)?;
        for node in &self.nodes {
            let current = node.pin(&self.route).snapshot()?;
            guard()?;
            if current != node.snapshot {
                return Err("CV parameter node identity or type changed".into());
            }
        }
        guard()?;
        self.route.borrowed_ids()?;
        for node in &self.nodes {
            if id_borrowed(node.pin(&self.route).node.0)? != node.snapshot.id {
                return Err("CV parameter node ID changed after metadata cleanup".into());
            }
        }
        for node in &self.nodes {
            node.borrowed_parameters()?;
        }
        self.route.borrowed_headers()?;
        for node in &self.nodes {
            node.header.check_borrowed(&node.snapshot.first_type)?;
        }
        self.route.borrowed_attachments()?;
        borrowed_guard()
    }
    pub(in crate::native_script::request) fn value(&self) -> Value {
        let nodes: Vec<Value> = self.nodes.iter().enumerate().map(|(index,node)| {
            let controller = match &node.controller {
                DirectController::Absent => json!({"status":"absent"}),
                DirectController::Present(controller) => json!({"status":"observed","count":controller.count,
                    "parameters":if self.rows == Rows::Complete { controller.parameters.iter().map(|p|p.observation.clone()).collect::<Vec<_>>() } else {vec![]}}),
            };
            json!({"index":index,"child_index":node.child_index,"identity":node.snapshot,"controller":controller})
        }).collect();
        json!({"scope":"cv_port_parameters","availability":if self.route.root.is_some(){"observed"}else{"lookup_absent"},
            "coverage":"bounded_direct_indexed_host_parameters","observation":"sequential_metadata_and_values",
            "status":match self.rows {Rows::Complete=>"complete",Rows::IndexLimit=>"index_limit",Rows::TextLimit=>"text_limit"},
            "indexed_inventory_complete":self.route.root.is_some() && self.rows==Rows::Complete,
            "nodes":nodes,"owner":{"device":self.route.device_snapshot,"slot":self.route.slot_snapshot},
            "indexed_membership":self.route.root.as_ref().map(|root|json!({"child_index":root.slot_index,"slot_child_count":root.slot_count,"unique_canonical_match":true})),
            "limits":{"nodes":8,"parameters_per_node":MAX_PARAMETERS_PER_NODE,"total_parameters":MAX_PARAMETERS,"retained_parameter_text_bytes":MAX_RETAINED_TEXT_BYTES},
            "retained_parameter_text_bytes":self.retained_text_bytes,
            "declared_parameter_controller":"unobserved","original_binding":"unobserved",
            "assignment_values_observed":false,"routing_verified":false,"atomic_snapshot":false,"mutation_capability":false})
    }
}
fn text_bytes(value: &Value) -> usize {
    match value {
        Value::String(s) => s.len(),
        Value::Array(values) => values.iter().map(text_bytes).sum(),
        Value::Object(values) => values.values().map(text_bytes).sum(),
        _ => 0,
    }
}
fn observed_details(d: &parameter_api::ParameterDetails) -> Value {
    json!({"precision":d.precision,"storable":d.storable,"mutable":d.mutable,
        "store_list_value":d.store_list_value,"read_only":d.read_only,
        "can_undo":d.can_undo,"list_interface":d.list_interface})
}
fn observed_parameter(index: u32, d: &parameter_api::Description) -> Value {
    json!({"index":index,"name":d.name,"tag":d.tag,"kind":format!("{:?}",d.kind),"enabled":d.enabled,"read_only":d.read_only,
        "value":typed(d.raw.as_ref()),"min":typed(d.min.as_ref()),"max":typed(d.max.as_ref()),"default":typed(d.default_value.as_ref()),
        "normalized":match d.normalized {parameter_api::Normalized::NotApplicable=>json!({"kind":"not_applicable"}),parameter_api::Normalized::Finite(v)=>json!({"kind":"float32","bits_hex":format!("{:08X}",v.to_bits())}),parameter_api::Normalized::NonFinite{bits}=>json!({"kind":"non_finite_float32","bits_hex":format!("{bits:08X}")})},
        "display":d.display})
}

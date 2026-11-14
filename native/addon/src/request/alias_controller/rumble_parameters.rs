use super::*;
use std::collections::BTreeSet;

const MAX_COUNT: i32 = 128;
const MAX_TOTAL: usize = 256;
const MAX_BYTES: usize = 262144;

struct Capability {
    controller: Option<Owned>,
    status: &'static str,
    count: Option<i32>,
    raw_count: Option<i32>,
}
struct Child {
    capability: Capability,
    identity: Owned,
    node: Owned,
    index: i32,
    id: String,
    slot: Option<usize>,
}
struct Parameter {
    parameter: RawParameter,
    owner: Option<parameters::ControllerOwner>,
    owner_view: *mut c_void,
    child: usize,
    index: i32,
}
pub(in crate::native_script::request) struct Observation {
    parameters: Vec<Parameter>,
    holes: Vec<(usize, i32)>,
    children: Vec<Child>,
    root_identity: Owned,
    root_node: Owned,
    binding: ControllerBinding,
    state_alias: state::AliasLease,
    models: Vec<String>,
    root_count: i32,
}
fn slot(index: usize) -> String {
    if index == 25 {
        "amp".into()
    } else {
        format!("effect{}", index + 1)
    }
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
fn child_count(node: *mut c_void) -> i32 {
    let get: Count = unsafe { method(node, 8) };
    unsafe { get(node) }
}
fn child_at(node: *mut c_void, index: i32) -> *mut c_void {
    let get: unsafe extern "C" fn(*mut c_void, i32) -> *mut c_void = unsafe { method(node, 9) };
    unsafe { get(node, index) }
}
fn node_id<G: FnMut() -> Result<(), String>>(
    node: *mut c_void,
    guard: &mut G,
) -> Result<String, String> {
    guard()?;
    let get: unsafe extern "C" fn(*mut c_void) -> *const PlainString = unsafe { method(node, 3) };
    let reference = unsafe { get(node) };
    if reference.is_null() {
        return Err("Rumble child ID unavailable".into());
    }
    let owner = pin(unsafe { (*reference).object }, guard)?;
    let result = (|| {
        guard()?;
        let length: Count = unsafe { method(owner.0, 4) };
        let n = unsafe { length(owner.0) };
        guard()?;
        if !(0..=128).contains(&n) {
            return Err("Rumble child ID exceeds bound".into());
        }
        let mut units = vec![0xffff; n as usize + 1];
        let copy: unsafe extern "C" fn(*mut c_void, *mut u16, i32) -> i32 =
            unsafe { method(owner.0, 8) };
        let status = unsafe { copy(owner.0, units.as_mut_ptr(), n + 1) };
        guard()?;
        let after = unsafe { length(owner.0) };
        guard()?;
        if status != OK || after != n || units[n as usize] != 0 {
            return Err("Rumble child ID changed".into());
        }
        let value =
            String::from_utf16(&units[..n as usize]).map_err(|_| "Rumble child ID invalid")?;
        if value.len() > 128 || value.contains('\0') {
            return Err("Rumble child ID exceeds bound".into());
        }
        Ok(value)
    })();
    release(owner, guard)?;
    result
}
fn capability<G: FnMut() -> Result<(), String>>(
    node: *mut c_void,
    canonical: *mut c_void,
    guard: &mut G,
) -> Result<Capability, String> {
    guard()?;
    let mut out = ptr::null_mut();
    let status = unsafe { (unknown_vtable(node).query)(node, &CONTROLLER, &mut out) };
    let handle = (!out.is_null()).then(|| Owned(out));
    if let Err(reason) = guard() {
        release(handle, guard)?;
        return Err(reason);
    }
    let unavailable = match (status, handle.is_some()) {
        (NO_INTERFACE, false) => Some("controller_absent"),
        (OK, false) => Some("malformed_success"),
        (NO_INTERFACE, true) => Some("malformed_absence"),
        (OK, true) => None,
        _ => Some("qi_failed"),
    };
    if let Some(status) = unavailable {
        release(handle, guard)?;
        return Ok(Capability {
            controller: None,
            status,
            count: None,
            raw_count: None,
        });
    }
    let mut lost = false;
    let identity = interface(out, &UNKNOWN, &mut || {
        let result = guard();
        lost |= result.is_err();
        result
    });
    let matches = identity.as_ref().is_ok_and(|i| i.0 == canonical);
    let failed = identity.is_err();
    release(identity, guard)?;
    if lost {
        release(handle, guard)?;
        return Err("Rumble context guard failed".into());
    }
    if failed || !matches {
        release(handle, guard)?;
        return Ok(Capability {
            controller: None,
            status: if failed {
                "qi_failed"
            } else {
                "identity_mismatch"
            },
            count: None,
            raw_count: None,
        });
    }
    guard()?;
    let n = count(out);
    guard()?;
    if !(0..=MAX_COUNT).contains(&n) {
        release(handle, guard)?;
        return Ok(Capability {
            controller: None,
            status: if n < 0 {
                "negative_count"
            } else {
                "count_limit"
            },
            count: None,
            raw_count: Some(n),
        });
    }
    Ok(Capability {
        controller: handle,
        status: "present",
        count: Some(n),
        raw_count: Some(n),
    })
}
impl Parameter {
    fn check<G: FnMut() -> Result<(), String>>(
        &self,
        observation: &Observation,
        guard: &mut G,
    ) -> Result<(), String> {
        let child = &observation.children[self.child];
        let controller = child
            .capability
            .controller
            .as_ref()
            .ok_or("Rumble controller lost")?;
        guard()?;
        if Some(count(controller.0)) != child.capability.count {
            return Err("Rumble parameter count changed".into());
        }
        guard()?;
        if at(controller.0, self.index) != self.parameter.as_borrowed_raw().as_ptr() {
            return Err("Rumble parameter association changed".into());
        }
        guard()?;
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
                return Err("Rumble parameter owner changed".into());
            }
        }
        guard()
    }
}
impl Observation {
    pub(in crate::native_script::request) fn check_ownership<G: FnMut() -> Result<(), String>>(
        &self,
        guard: &mut G,
    ) -> Result<(), String> {
        self.binding.check(guard)?;
        let root = interface(self.root_node.0, &UNKNOWN, guard)?;
        let matches = root.0 == self.root_identity.0 && root.0 == self.binding.identity.0;
        release(root, guard)?;
        if !matches {
            return Err("Rumble root identity changed".into());
        }
        for child in &self.children {
            let identity = interface(child.node.0, &UNKNOWN, guard)?;
            let matches = identity.0 == child.identity.0;
            release(identity, guard)?;
            if !matches || node_id(child.node.0, guard)? != child.id {
                return Err("Rumble child identity changed".into());
            }
            if child.slot.is_some() {
                let current = capability(child.node.0, child.identity.0, guard)?;
                let matches = current.status == child.capability.status
                    && current.count == child.capability.count
                    && current.raw_count == child.capability.raw_count;
                release(current, guard)?;
                if !matches {
                    return Err("Rumble child capability changed".into());
                }
            }
        }
        for parameter in &self.parameters {
            parameter.check(self, guard)?;
        }
        self.state_alias.check()?;
        guard()
    }
    fn attachments(&self) -> Result<(), String> {
        if borrowed(self.binding.alias.alias.0, 3) != self.binding.alias.plugin.0
            || borrowed(self.binding.alias.node.0, 6) != self.binding.alias.parent.0
            || borrowed(self.binding.alias.provider.0, 3) != self.binding.controller.0
            || child_count(self.root_node.0) != self.root_count
        {
            return Err("Rumble root attachment changed".into());
        }
        for child in &self.children {
            if child_at(self.root_node.0, child.index) != child.node.0 {
                return Err("Rumble direct membership changed".into());
            }
            let parent = borrowed(child.node.0, 6);
            if !parent.is_null() && parent != self.root_node.0 {
                return Err("Rumble child parent changed".into());
            }
            if let Some(controller) = &child.capability.controller {
                if Some(count(controller.0)) != child.capability.count {
                    return Err("Rumble parameter count changed".into());
                }
            }
        }
        for (child, index) in &self.holes {
            let controller = self.children[*child]
                .capability
                .controller
                .as_ref()
                .ok_or("Rumble controller lost")?;
            if !at(controller.0, *index).is_null() {
                return Err("Rumble missing parameter changed".into());
            }
        }
        for p in &self.parameters {
            let controller = self.children[p.child]
                .capability
                .controller
                .as_ref()
                .ok_or("Rumble controller lost")?;
            if at(controller.0, p.index) != p.parameter.as_borrowed_raw().as_ptr()
                || borrowed(p.parameter.as_borrowed_raw().as_ptr(), 14) != p.owner_view
            {
                return Err("Rumble parameter attachment changed".into());
            }
        }
        Ok(())
    }
    pub(in crate::native_script::request) fn check(&self, ctx: &Context<'_>) -> Result<(), String> {
        self.check_ownership(&mut || ctx.check())?;
        let models = state::rumble_models(ctx, &self.state_alias)?;
        if models.classes != self.models {
            return Err("Rumble selected models changed".into());
        }
        ctx.check_document()?;
        ctx.check()?;
        self.attachments()
    }
}
fn kind(kind: parameters::Kind) -> i32 {
    use parameters::Kind::*;
    match kind {
        Toggle => 0,
        Integer => 1,
        Float => 2,
        String => 3,
        List => 4,
        Command => 5,
        Color => 6,
        Image => 7,
        Segments => 8,
        Scroll => 9,
        TextModel => 10,
        Unknown(n) => n,
    }
}
fn numeric(value: Option<&parameters::NativeValue>) -> Value {
    let Some(value) = value else {
        return json!({"status":"unavailable","variant_tag":null,"reason":"missing"});
    };
    match value.value {
        parameters::Value::Integer(n) => {
            json!({"status":"observed","variant_tag":value.variant_tag,"kind":"integer","decimal":n.to_string()})
        }
        parameters::Value::Float(n) if n.is_finite() => {
            json!({"status":"observed","variant_tag":value.variant_tag,"kind":"float64","bits_hex":format!("{:016x}",n.to_bits())})
        }
        parameters::Value::Float(_) | parameters::Value::NonFiniteFloat { .. } => {
            json!({"status":"unavailable","variant_tag":value.variant_tag,"reason":"non_finite"})
        }
        _ => json!({"status":"unavailable","variant_tag":value.variant_tag,"reason":"not_numeric"}),
    }
}
fn project(d: &parameters::Description) -> Option<Value> {
    if d.name.is_empty() || d.name.len() > 128 || d.name.contains('\0') {
        return None;
    }
    let current = numeric(d.raw.as_ref());
    let normalized = match d.normalized {
        parameters::Normalized::Finite(n) if n.is_finite() => {
            json!({"status":"observed","bits_hex":format!("{:08x}",n.to_bits())})
        }
        parameters::Normalized::Finite(_) | parameters::Normalized::NonFinite { .. } => {
            json!({"status":"unavailable","reason":"non_finite"})
        }
        parameters::Normalized::NotApplicable => {
            json!({"status":"unavailable","reason":"not_applicable"})
        }
    };
    let supported = matches!(kind(d.kind), 0 | 1 | 2 | 4 | 9)
        && current["status"] == "observed"
        && d.enabled
        && !d.read_only;
    Some(
        json!({"name":d.name,"tag":d.tag,"kind":kind(d.kind),"enabled":d.enabled,"read_only":d.read_only,"current":current,
        "min":numeric(d.min.as_ref()),"max":numeric(d.max.as_ref()),"default":numeric(d.default_value.as_ref()),"normalized":normalized,
        "support":{"raw_number":supported&&d.support.raw_number,"normalized":supported&&d.support.normalized&&normalized["status"]=="observed"}}),
    )
}
pub(in crate::native_script::request) fn failure() -> Value {
    json!({"ok":false,"outcome":"not_applied","native_outcome":"not_applied","mutation_capability":false,"can_set_via_this_action":false,"selection_authority":false,"atomic_snapshot":false,"audio_verified":false,"persistence_verified":false,"error":{"code":"rumble_controller_parameters_unavailable","message":"The bounded Rumble controller-parameter observation was not established"}})
}
pub(in crate::native_script::request) fn observe(
    ctx: &Context<'_>,
) -> Result<(Value, Observation), String> {
    let (before, state_alias) = state::capture_rumble_models(ctx)?;
    let mut guard = || ctx.check();
    let alias = AliasBinding::capture(ctx.target.0, ctx.node.0, &mut guard)?;
    let binding = ControllerBinding::capture(alias, &mut guard)?;
    if !state_alias.component_identity_matches(binding.alias.plugin_identity.0) {
        return Err("Rumble alias identity mismatch".into());
    }
    let root_node = interface(binding.controller.0, &NODE, &mut guard)?;
    let root_identity = interface(root_node.0, &UNKNOWN, &mut guard)?;
    if root_identity.0 != binding.identity.0 {
        return Err("Rumble root canonical identity mismatch".into());
    }
    guard()?;
    let root_count = child_count(root_node.0);
    guard()?;
    if !(0..=128).contains(&root_count) {
        return Err("Rumble root child count exceeds bound".into());
    }
    let mut observation = Observation {
        parameters: Vec::new(),
        holes: Vec::new(),
        children: Vec::new(),
        root_identity,
        root_node,
        binding,
        state_alias,
        models: before.classes.clone(),
        root_count,
    };
    let mut seen = BTreeSet::new();
    for index in 0..root_count {
        guard()?;
        let node = pin(child_at(observation.root_node.0, index), &mut guard)?;
        let identity = interface(node.0, &UNKNOWN, &mut guard)?;
        let id = node_id(node.0, &mut guard)?;
        let slot = (0..26).find(|i| slot(*i) == id);
        if slot.is_some() && !seen.insert(id.clone()) {
            return Err("Rumble duplicate fixed child ID".into());
        }
        let capability = if slot.is_some() {
            capability(node.0, identity.0, &mut guard)?
        } else {
            Capability {
                controller: None,
                status: "node_absent",
                count: None,
                raw_count: None,
            }
        };
        observation.children.push(Child {
            capability,
            identity,
            node,
            index,
            id,
            slot,
        });
    }
    observation.check_ownership(&mut guard)?;
    let reported: usize = observation
        .children
        .iter()
        .filter_map(|c| c.capability.count)
        .map(|n| n as usize)
        .sum();
    let limit = reported > MAX_TOTAL;
    let mut rows = Vec::with_capacity(26);
    let mut list_complete = !limit;
    let mut current_complete = !limit;
    let mut name_complete = !limit;
    let mut enumerated = 0usize;
    let mut census = true;
    for i in 0..26 {
        let child = observation.children.iter().position(|c| c.slot == Some(i));
        let Some(child_index) = child else {
            census = false;
            list_complete = false;
            current_complete = false;
            name_complete = false;
            rows.push(json!({"slot":slot(i),"serialized_class_uid":before.classes[i],"status":"node_absent","parameter_count":null,"enumeration_status":"unavailable","parameters":null,"serialized_name_count":before.names[i].len(),"matched_serialized_name_count":0}));
            continue;
        };
        let child = &observation.children[child_index];
        let Some(n) = child.capability.count else {
            census = false;
            list_complete = false;
            current_complete = false;
            name_complete = false;
            rows.push(json!({"slot":slot(i),"serialized_class_uid":before.classes[i],"status":child.capability.status,"parameter_count":null,"enumeration_status":"unavailable","parameters":null,"serialized_name_count":before.names[i].len(),"matched_serialized_name_count":0}));
            continue;
        };
        if limit {
            rows.push(json!({"slot":slot(i),"serialized_class_uid":before.classes[i],"status":"present","parameter_count":n,"enumeration_status":"descriptor_total_limit","parameters":null,"serialized_name_count":before.names[i].len(),"matched_serialized_name_count":0}));
            continue;
        }
        let mut records = Vec::new();
        let mut names = BTreeSet::new();
        for index in 0..n {
            observation.binding.check(&mut guard)?;
            let controller = observation.children[child_index]
                .capability
                .controller
                .as_ref()
                .ok_or("Rumble controller lost")?
                .0;
            let pointer = at(controller, index);
            guard()?;
            if pointer.is_null() {
                observation.holes.push((child_index, index));
                records.push(json!({"index":index,"status":"parameter_absent","description":null}));
                list_complete = false;
                current_complete = false;
                continue;
            }
            let owned = pin(pointer, &mut guard)?;
            let pointer = NonNull::new(owned.0).expect("retained parameter");
            std::mem::forget(owned);
            let parameter = unsafe { RawParameter::from_owned(pointer, raw_text(ctx.api)) };
            let owner = parameter
                .controller_owner(&mut guard)
                .map_err(|e| e.to_string())?;
            let owner_view = owner
                .as_ref()
                .map(|o| {
                    unsafe { o.as_borrowed_unknown() }
                        .map(|p| p.as_ptr())
                        .map_err(|e| e.to_string())
                })
                .transpose()?
                .unwrap_or(ptr::null_mut());
            if let Some(owner) = &owner {
                let view = unsafe { owner.as_borrowed_unknown() }.map_err(|e| e.to_string())?;
                let identity = interface(view.as_ptr(), &UNKNOWN, &mut guard)?;
                let matches = identity.0 == observation.children[child_index].identity.0;
                release(identity, &mut guard)?;
                if !matches {
                    return Err("Rumble parameter owner mismatch".into());
                }
            }
            let parameter = Parameter {
                parameter,
                owner,
                owner_view,
                child: child_index,
                index,
            };
            parameter.check(&observation, &mut guard)?;
            let description = parameter.parameter.describe();
            guard()?;
            parameter.check(&observation, &mut guard)?;
            let failed_description = description.is_err();
            let projected = description.ok().and_then(|d| project(&d));
            match projected {
                Some(description) => {
                    names.insert(
                        description["name"]
                            .as_str()
                            .expect("projected name")
                            .to_owned(),
                    );
                    current_complete &= description["current"]["status"] == "observed";
                    records
                        .push(json!({"index":index,"status":"observed","description":description}));
                    enumerated += 1;
                }
                None => {
                    records.push(json!({"index":index,"status":if failed_description{"description_unavailable"}else{"unsupported_metadata"},"description":null}));
                    list_complete = false;
                    current_complete = false;
                }
            }
            observation.parameters.push(parameter);
        }
        let matched = before.names[i].intersection(&names).count();
        name_complete &= matched == before.names[i].len();
        let complete = records.iter().all(|r| r["status"] == "observed");
        rows.push(json!({"slot":slot(i),"serialized_class_uid":before.classes[i],"status":"present","parameter_count":n,"enumeration_status":if complete{"complete"}else{"incomplete"},"parameters":records,"serialized_name_count":before.names[i].len(),"matched_serialized_name_count":matched}));
    }
    list_complete &= census;
    current_complete &= list_complete;
    name_complete &= list_complete;
    let after = state::rumble_models(ctx, &observation.state_alias)?;
    if after.classes != before.classes {
        return Err("Rumble selected models changed".into());
    }
    let value = json!({"ok":true,"outcome":"observed","operation":"rumble_parameters_read","schema":"rumble_controller_parameters_v1","instance":ctx.instance,"source":"component_alias_fixed_controllers","scope":"fixed_immediate_controller_parameter_lists","controllers":rows,"models_sha256":before.sha256,"before_after_model_ids_equal":true,"census_complete":census,"parameter_lists_complete":list_complete,"access_metadata_complete":list_complete,"current_values_complete":current_complete,"serialized_name_coverage_complete":name_complete,"reported_parameter_count":reported,"enumerated_parameter_count":enumerated,"descriptor_budget_exceeded":limit,"value_semantics":"sequential_snapshot","mutation_capability":false,"can_set_via_this_action":false,"selection_authority":false,"atomic_snapshot":false,"audio_verified":false,"persistence_verified":false});
    if !valid_receipt(&json!(ctx.instance), &value) {
        return Err("Rumble parameter receipt invalid".into());
    }
    observation.check(ctx)?;
    Ok((value, observation))
}
fn exact(value: &Value, keys: &[&str]) -> bool {
    value
        .as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)))
}
fn bounded_integer(value: &Value, min: i64, max: i64) -> bool {
    value.as_i64().is_some_and(|n| (min..=max).contains(&n))
}
fn hex(value: &Value, length: usize) -> Option<u64> {
    let text = value.as_str()?;
    if text.len() != length
        || !text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return None;
    }
    u64::from_str_radix(text, 16).ok()
}
fn valid_numeric(value: &Value) -> bool {
    if value["status"] == "unavailable" {
        return exact(value, &["status", "variant_tag", "reason"])
            && match value["reason"].as_str() {
                Some("missing") => value["variant_tag"].is_null(),
                Some("not_numeric" | "non_finite") => {
                    bounded_integer(&value["variant_tag"], i16::MIN.into(), i16::MAX.into())
                }
                _ => false,
            };
    }
    if value["status"] != "observed"
        || !bounded_integer(&value["variant_tag"], i16::MIN.into(), i16::MAX.into())
    {
        return false;
    }
    match value["kind"].as_str() {
        Some("integer") => {
            exact(value, &["status", "variant_tag", "kind", "decimal"])
                && value["decimal"]
                    .as_str()
                    .is_some_and(|s| s.parse::<i64>().is_ok_and(|n| n.to_string() == s))
        }
        Some("float64") => {
            exact(value, &["status", "variant_tag", "kind", "bits_hex"])
                && hex(&value["bits_hex"], 16).is_some_and(|n| f64::from_bits(n).is_finite())
        }
        _ => false,
    }
}
fn valid_normalized(value: &Value) -> bool {
    if value["status"] == "observed" {
        exact(value, &["status", "bits_hex"])
            && hex(&value["bits_hex"], 8).is_some_and(|n| f32::from_bits(n as u32).is_finite())
    } else {
        exact(value, &["status", "reason"])
            && value["status"] == "unavailable"
            && matches!(
                value["reason"].as_str(),
                Some("not_applicable" | "non_finite")
            )
    }
}
fn valid_description(value: &Value) -> bool {
    if !exact(
        value,
        &[
            "name",
            "tag",
            "kind",
            "enabled",
            "read_only",
            "current",
            "min",
            "max",
            "default",
            "normalized",
            "support",
        ],
    ) || !value["name"]
        .as_str()
        .is_some_and(|s| !s.is_empty() && s.len() <= 128 && !s.contains('\0'))
        || !bounded_integer(&value["tag"], i32::MIN.into(), i32::MAX.into())
        || !bounded_integer(&value["kind"], i32::MIN.into(), i32::MAX.into())
        || !value["enabled"].is_boolean()
        || !value["read_only"].is_boolean()
        || !["current", "min", "max", "default"]
            .iter()
            .all(|k| valid_numeric(&value[*k]))
        || !valid_normalized(&value["normalized"])
        || !exact(&value["support"], &["raw_number", "normalized"])
        || !value["support"]["raw_number"].is_boolean()
        || !value["support"]["normalized"].is_boolean()
    {
        return false;
    }
    let numeric = matches!(value["kind"].as_i64(), Some(0 | 1 | 2 | 4 | 9))
        && value["current"]["status"] == "observed"
        && value["enabled"] == true
        && value["read_only"] == false;
    (value["support"]["raw_number"] != true || numeric)
        && (value["support"]["normalized"] != true
            || numeric && value["normalized"]["status"] == "observed")
}
pub(in crate::native_script::request) fn valid_receipt(instance: &Value, value: &Value) -> bool {
    if !exact(
        value,
        &[
            "ok",
            "outcome",
            "operation",
            "schema",
            "instance",
            "source",
            "scope",
            "controllers",
            "models_sha256",
            "before_after_model_ids_equal",
            "census_complete",
            "parameter_lists_complete",
            "access_metadata_complete",
            "current_values_complete",
            "serialized_name_coverage_complete",
            "reported_parameter_count",
            "enumerated_parameter_count",
            "descriptor_budget_exceeded",
            "value_semantics",
            "mutation_capability",
            "can_set_via_this_action",
            "selection_authority",
            "atomic_snapshot",
            "audio_verified",
            "persistence_verified",
        ],
    ) || value["ok"] != true
        || value["outcome"] != "observed"
        || value["operation"] != "rumble_parameters_read"
        || value["schema"] != "rumble_controller_parameters_v1"
        || value["instance"] != *instance
        || value["source"] != "component_alias_fixed_controllers"
        || value["scope"] != "fixed_immediate_controller_parameter_lists"
        || value["before_after_model_ids_equal"] != true
        || value["value_semantics"] != "sequential_snapshot"
        || [
            "mutation_capability",
            "can_set_via_this_action",
            "selection_authority",
            "atomic_snapshot",
            "audio_verified",
            "persistence_verified",
        ]
        .iter()
        .any(|k| value[*k] != false)
    {
        return false;
    }
    let mut adapted = value.clone();
    adapted["plugin"] = json!("Rumble Native");
    adapted["native_outcome"] = json!("observed");
    if adapted.to_string().len() > MAX_BYTES {
        return false;
    }
    let Some(rows) = value["controllers"].as_array().filter(|r| r.len() == 26) else {
        return false;
    };
    let mut classes = Vec::new();
    let mut reported = 0usize;
    let mut enumerated = 0usize;
    let mut census = true;
    let mut lists = true;
    let mut current = true;
    let mut names_complete = true;
    let limit = value["descriptor_budget_exceeded"] == true;
    if !value["descriptor_budget_exceeded"].is_boolean() {
        return false;
    }
    for (i, row) in rows.iter().enumerate() {
        if !exact(
            row,
            &[
                "slot",
                "serialized_class_uid",
                "status",
                "parameter_count",
                "enumeration_status",
                "parameters",
                "serialized_name_count",
                "matched_serialized_name_count",
            ],
        ) || row["slot"] != slot(i)
            || !bounded_integer(&row["serialized_name_count"], 0, 8192)
            || !bounded_integer(
                &row["matched_serialized_name_count"],
                0,
                row["serialized_name_count"].as_i64().unwrap_or(-1),
            )
        {
            return false;
        }
        let Some(class) = row["serialized_class_uid"].as_str() else {
            return false;
        };
        classes.push(class.to_owned());
        if row["status"] != "present" {
            if !matches!(
                row["status"].as_str(),
                Some(
                    "node_absent"
                        | "controller_absent"
                        | "qi_failed"
                        | "malformed_success"
                        | "malformed_absence"
                        | "identity_mismatch"
                        | "negative_count"
                        | "count_limit"
                )
            ) || !row["parameter_count"].is_null()
                || !row["parameters"].is_null()
                || row["enumeration_status"] != "unavailable"
                || row["matched_serialized_name_count"] != 0
            {
                return false;
            }
            census = false;
            lists = false;
            current = false;
            names_complete = false;
            continue;
        }
        if !bounded_integer(&row["parameter_count"], 0, 128) {
            return false;
        }
        let n = row["parameter_count"].as_u64().expect("bounded count") as usize;
        reported += n;
        if limit {
            if !row["parameters"].is_null()
                || row["enumeration_status"] != "descriptor_total_limit"
                || row["matched_serialized_name_count"] != 0
            {
                return false;
            }
            lists = false;
            current = false;
            names_complete = false;
            continue;
        }
        let Some(parameters) = row["parameters"].as_array().filter(|p| p.len() == n) else {
            return false;
        };
        let mut names = BTreeSet::new();
        let mut complete = true;
        for (index, p) in parameters.iter().enumerate() {
            if !exact(p, &["index", "status", "description"])
                || p["index"].as_u64() != Some(index as u64)
            {
                return false;
            }
            if p["status"] == "observed" {
                if !valid_description(&p["description"]) {
                    return false;
                }
                names.insert(p["description"]["name"].as_str().expect("bounded name"));
                enumerated += 1;
                current &= p["description"]["current"]["status"] == "observed";
            } else {
                if !matches!(
                    p["status"].as_str(),
                    Some("parameter_absent" | "description_unavailable" | "unsupported_metadata")
                ) || !p["description"].is_null()
                {
                    return false;
                }
                complete = false;
                current = false;
            }
        }
        if row["enumeration_status"] != if complete { "complete" } else { "incomplete" }
            || row["matched_serialized_name_count"]
                .as_u64()
                .is_none_or(|n| n > names.len() as u64)
        {
            return false;
        }
        lists &= complete;
        names_complete &= row["serialized_name_count"] == row["matched_serialized_name_count"];
    }
    lists &= census && !limit;
    current &= lists;
    names_complete &= lists;
    let Some(hash) = state::rumble_model_digest(&classes) else {
        return false;
    };
    value["models_sha256"] == hash
        && value["census_complete"] == census
        && value["parameter_lists_complete"] == lists
        && value["access_metadata_complete"] == lists
        && value["current_values_complete"] == current
        && value["serialized_name_coverage_complete"] == names_complete
        && value["reported_parameter_count"].as_u64() == Some(reported as u64)
        && value["enumerated_parameter_count"].as_u64() == Some(enumerated as u64)
        && limit == (reported > MAX_TOTAL)
        && enumerated <= MAX_TOTAL
}

#[cfg(test)]
#[path = "rumble_parameter_tests.rs"]
mod tests;

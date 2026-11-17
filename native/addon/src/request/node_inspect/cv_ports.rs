use super::*;

#[path = "cv_ports/parameters.rs"]
pub(super) mod parameters;

const CV_CLASS: &str = "EDCD22A5-A01E-4EA6-9FE4-2BB1AB243E85";
const MAX_SLOT_CHILDREN: u32 = 32;
const MAX_ROOT_CHILDREN: u32 = 8;

pub(super) fn admit(instance: &Instance) -> Result<(), String> {
    if instance.class_uid == CV_CLASS
        && instance.native_class == "AudioSynth"
        && instance.object_id == "CV Instrument"
        && valid_uid(&instance.object_uid)
    {
        Ok(())
    } else {
        Err("cv_ports requires the exact native CV Instrument AudioSynth identity".into())
    }
}

struct Pin {
    node: Owned,
    canonical: Owned,
}
impl Pin {
    fn borrowed(pointer: *mut c_void) -> Result<Self, String> {
        let node = hold(pointer).ok_or("CV port node unavailable")?;
        let canonical = query(node.0, &UNKNOWN).ok_or("CV port canonical identity unavailable")?;
        let interface = query(node.0, &NODE).ok_or("CV port node interface unavailable")?;
        let identity = query(interface.0, &UNKNOWN).ok_or("CV port node identity unavailable")?;
        if identity.0 != canonical.0 {
            return Err("CV port foreign node interface".into());
        }
        drop(identity);
        drop(interface);
        Ok(Self { node, canonical })
    }
    fn identity(&self) -> Result<(), String> {
        let identity = query(self.node.0, &UNKNOWN).ok_or("CV port identity unavailable")?;
        let same = identity.0 == self.canonical.0;
        drop(identity);
        if same {
            Ok(())
        } else {
            Err("CV port canonical identity changed".into())
        }
    }
    fn snapshot(&self) -> Result<Snapshot, String> {
        self.identity()?;
        let object = query(self.node.0, &OBJECT).ok_or("CV port native type unavailable")?;
        let canonical = query(object.0, &UNKNOWN).ok_or("CV port type identity unavailable")?;
        if canonical.0 != self.canonical.0 {
            return Err("CV port foreign reflection interface".into());
        }
        drop(canonical);
        let info: Borrowed = unsafe { method(object.0, 3) };
        let info =
            hold(unsafe { info(object.0) }).ok_or("CV port first native type unavailable")?;
        let name: unsafe extern "C" fn(*mut c_void) -> *const c_char = unsafe { method(info.0, 5) };
        let first_type = ascii(unsafe { name(info.0) });
        drop(info);
        drop(object);
        let get: unsafe extern "C" fn(*mut c_void) -> *const PlainString =
            unsafe { method(self.node.0, 3) };
        let text = unsafe { get(self.node.0) };
        if text.is_null() {
            return Err("CV port identifier unavailable".into());
        }
        let owner = hold(unsafe { (*text).object }).ok_or("CV port identifier unavailable")?;
        let id = copy_string(owner.0, 512)?;
        drop(owner);
        let (object_uid, class_uid) = uids(self.node.0)?;
        if !valid_uid(&object_uid)
            || !class_uid.is_empty() && !valid_uid(&class_uid)
            || first_type.is_empty()
        {
            return Err("CV port stable identity or native type unavailable".into());
        }
        Ok(Snapshot {
            id,
            object_uid,
            class_uid: (!class_uid.is_empty()).then_some(class_uid),
            first_type,
        })
    }
}

struct Header {
    object: Owned,
}
impl Header {
    fn capture(pin: &Pin) -> Result<Self, String> {
        let object = query(pin.node.0, &OBJECT).ok_or("CV port native type unavailable")?;
        let canonical = query(object.0, &UNKNOWN).ok_or("CV port type identity unavailable")?;
        let same = canonical.0 == pin.canonical.0;
        drop(canonical);
        if !same {
            return Err("CV port foreign reflection interface".into());
        }
        Ok(Self { object })
    }
    fn check_borrowed(&self, expected: &str) -> Result<(), String> {
        let get: Borrowed = unsafe { method(self.object.0, 3) };
        let info = unsafe { get(self.object.0) };
        if info.is_null() {
            return Err("CV port first native type unavailable".into());
        }
        let name: unsafe extern "C" fn(*mut c_void) -> *const c_char = unsafe { method(info, 5) };
        if ascii(unsafe { name(info) }) != expected {
            return Err("CV port first native type changed after metadata cleanup".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct Snapshot {
    id: String,
    object_uid: String,
    class_uid: Option<String>,
    first_type: String,
}
fn uids(node: *mut c_void) -> Result<(String, String), String> {
    let object: unsafe extern "C" fn(*mut c_void) -> *const Uid = unsafe { method(node, 4) };
    let class: unsafe extern "C" fn(*mut c_void) -> *const Uid = unsafe { method(node, 5) };
    let object = unsafe { object(node) };
    let class = unsafe { class(node) };
    if object.is_null() || class.is_null() {
        return Err("CV port null UID reference is malformed".into());
    }
    let class = uid(class);
    Ok((
        uid(object),
        if class == ZERO_UID {
            String::new()
        } else {
            class
        },
    ))
}
fn id_borrowed(node: *mut c_void) -> Result<String, String> {
    let get: unsafe extern "C" fn(*mut c_void) -> *const PlainString = unsafe { method(node, 3) };
    let text = unsafe { get(node) };
    if text.is_null() {
        return Err("CV port identifier unavailable".into());
    }
    copy_string(unsafe { (*text).object }, 512)
}

fn count(node: *mut c_void, max: u32) -> Result<u32, String> {
    let get: unsafe extern "C" fn(*mut c_void) -> i32 = unsafe { method(node, 8) };
    let count = unsafe { get(node) };
    if count < 0 || count as u32 > max {
        Err("CV port child count unavailable or exceeds fixed bound".into())
    } else {
        Ok(count as u32)
    }
}
fn parent(node: *mut c_void) -> *mut c_void {
    let get: Borrowed = unsafe { method(node, 6) };
    unsafe { get(node) }
}
fn child(node: *mut c_void, index: u32) -> *mut c_void {
    let get: unsafe extern "C" fn(*mut c_void, i32) -> *mut c_void = unsafe { method(node, 9) };
    unsafe { get(node, index as i32) }
}
fn find(node: *mut c_void, name: *const PlainString) -> *mut c_void {
    let get: unsafe extern "C" fn(*mut c_void, *const PlainString) -> *mut c_void =
        unsafe { method(node, 10) };
    unsafe { get(node, name) }
}
struct Child {
    pin: Pin,
    object_uid: String,
    class_uid: String,
}
struct Root {
    header: Header,
    pin: Pin,
    snapshot: Snapshot,
    slot_index: u32,
    slot_count: u32,
    indexed_pointer: *mut c_void,
    slot_pointers: Vec<*mut c_void>,
    children: Vec<Child>,
}
struct Route {
    device_header: Header,
    slot_header: Header,
    root: Option<Root>,
    slot: Pin,
    device: Pin,
    slot_snapshot: Snapshot,
    device_snapshot: Snapshot,
    name: *const PlainString,
}
impl Route {
    fn capture<G: FnMut() -> Result<(), String>>(
        target: *mut c_void,
        node: *mut c_void,
        name: *const PlainString,
        guard: &mut G,
    ) -> Result<Self, String> {
        guard()?;
        let device = Pin::borrowed(node)?;
        let target_identity = query(target, &UNKNOWN).ok_or("CV target identity unavailable")?;
        let matches = target_identity.0 == device.canonical.0;
        drop(target_identity);
        guard()?;
        if !matches {
            return Err("CV Device belongs to a different target".into());
        }
        let slot = Pin::borrowed(parent(device.node.0))?;
        guard()?;
        let device_snapshot = device.snapshot()?;
        guard()?;
        admit(&Instance {
            object_uid: device_snapshot.object_uid.clone(),
            class_uid: device_snapshot.class_uid.clone().unwrap_or_default(),
            native_class: device_snapshot.first_type.clone(),
            object_id: device_snapshot.id.clone(),
        })?;
        let slot_snapshot = slot.snapshot()?;
        guard()?;
        if slot_snapshot.first_type != "AudioSynthSlot" {
            return Err("CV port owner is not AudioSynthSlot".into());
        }
        let device_header = Header::capture(&device)?;
        guard()?;
        let slot_header = Header::capture(&slot)?;
        guard()?;
        let pointer = find(slot.node.0, name);
        let root = if pointer.is_null() {
            guard()?;
            None
        } else {
            let pin = Pin::borrowed(pointer)?;
            guard()?;
            let snapshot = pin.snapshot()?;
            guard()?;
            if snapshot.id != "PortSetup" || snapshot.first_type != "AudioFXPortSetup" {
                return Err("CV PortSetup identifier or first native type unsupported".into());
            }
            let slot_count = count(slot.node.0, MAX_SLOT_CHILDREN)?;
            guard()?;
            let mut matched = None;
            let mut slot_pointers = Vec::new();
            for index in 0..slot_count {
                let pointer = child(slot.node.0, index);
                slot_pointers.push(pointer);
                let candidate = Pin::borrowed(pointer)?;
                guard()?;
                let same = candidate.canonical.0 == pin.canonical.0;
                drop(candidate);
                guard()?;
                if same {
                    if matched.is_some() {
                        return Err("CV PortSetup indexed membership is ambiguous".into());
                    }
                    matched = Some((index, pointer));
                }
            }
            let (slot_index, indexed_pointer) =
                matched.ok_or("CV PortSetup not a current indexed slot child")?;
            if count(slot.node.0, MAX_SLOT_CHILDREN)? != slot_count
                || child(slot.node.0, slot_index) != indexed_pointer
                || find(slot.node.0, name) != pin.node.0
            {
                return Err("CV PortSetup indexed membership changed".into());
            }
            guard()?;
            let child_count = count(pin.node.0, MAX_ROOT_CHILDREN)?;
            guard()?;
            let mut children: Vec<Child> = Vec::new();
            for index in 0..child_count {
                let child = Pin::borrowed(child(pin.node.0, index))?;
                guard()?;
                let (object_uid, class_uid) = uids(child.node.0)?;
                if children
                    .iter()
                    .any(|other| other.pin.canonical.0 == child.canonical.0)
                    || child.canonical.0 == pin.canonical.0
                    || child.canonical.0 == slot.canonical.0
                    || child.canonical.0 == device.canonical.0
                {
                    return Err("CV PortSetup child identities are ambiguous".into());
                }
                children.push(Child {
                    pin: child,
                    object_uid,
                    class_uid,
                });
                guard()?;
            }
            let header = Header::capture(&pin)?;
            guard()?;
            Some(Root {
                header,
                pin,
                snapshot,
                slot_index,
                slot_count,
                indexed_pointer,
                slot_pointers,
                children,
            })
        };
        let route = Self {
            device_header,
            slot_header,
            root,
            slot,
            device,
            slot_snapshot,
            device_snapshot,
            name,
        };
        route.metadata(guard)?;
        route.borrowed()?;
        guard()?;
        Ok(route)
    }
    fn metadata<G: FnMut() -> Result<(), String>>(&self, guard: &mut G) -> Result<(), String> {
        for (pin, snapshot) in [
            (&self.device, &self.device_snapshot),
            (&self.slot, &self.slot_snapshot),
        ] {
            let current = pin.snapshot()?;
            guard()?;
            if &current != snapshot {
                return Err("CV port owner identity or native type changed".into());
            }
        }
        if let Some(root) = &self.root {
            let current = root.pin.snapshot()?;
            guard()?;
            if current != root.snapshot {
                return Err("CV PortSetup identity or native type changed".into());
            }
            for child in &root.children {
                child.pin.identity()?;
                guard()?;
            }
        }
        Ok(())
    }
    fn borrowed_ids(&self) -> Result<(), String> {
        for (pin, snapshot) in [
            (&self.device, &self.device_snapshot),
            (&self.slot, &self.slot_snapshot),
        ] {
            if id_borrowed(pin.node.0)? != snapshot.id {
                return Err("CV port owner UID changed after metadata cleanup".into());
            }
        }
        if let Some(root) = &self.root {
            if id_borrowed(root.pin.node.0)? != root.snapshot.id {
                return Err("CV PortSetup attachment changed after metadata cleanup".into());
            }
        }
        Ok(())
    }
    fn borrowed_headers(&self) -> Result<(), String> {
        self.device_header
            .check_borrowed(&self.device_snapshot.first_type)?;
        self.slot_header
            .check_borrowed(&self.slot_snapshot.first_type)?;
        if let Some(root) = &self.root {
            root.header.check_borrowed(&root.snapshot.first_type)?;
        }
        Ok(())
    }
    fn borrowed_attachments(&self) -> Result<(), String> {
        for (pin, snapshot) in [
            (&self.device, &self.device_snapshot),
            (&self.slot, &self.slot_snapshot),
        ] {
            if uids(pin.node.0)?
                != (
                    snapshot.object_uid.clone(),
                    snapshot.class_uid.clone().unwrap_or_default(),
                )
            {
                return Err("CV port owner UID changed after metadata cleanup".into());
            }
        }
        if parent(self.device.node.0) != self.slot.node.0 {
            return Err("CV Device parent changed".into());
        }
        let expected = self
            .root
            .as_ref()
            .map_or(ptr::null_mut(), |root| root.pin.node.0);
        if find(self.slot.node.0, self.name) != expected {
            return Err("CV fixed PortSetup lookup changed".into());
        }
        if let Some(root) = &self.root {
            if count(self.slot.node.0, MAX_SLOT_CHILDREN)? != root.slot_count
                || child(self.slot.node.0, root.slot_index) != root.indexed_pointer
                || uids(root.pin.node.0)?
                    != (
                        root.snapshot.object_uid.clone(),
                        root.snapshot.class_uid.clone().unwrap_or_default(),
                    )
                || count(root.pin.node.0, MAX_ROOT_CHILDREN)? != root.children.len() as u32
            {
                return Err("CV PortSetup attachment changed after metadata cleanup".into());
            }
            for (index, expected) in root.slot_pointers.iter().enumerate() {
                if child(self.slot.node.0, index as u32) != *expected {
                    return Err("CV slot indexed membership changed after metadata cleanup".into());
                }
            }
            for (index, expected) in root.children.iter().enumerate() {
                if child(root.pin.node.0, index as u32) != expected.pin.node.0
                    || uids(expected.pin.node.0)?
                        != (expected.object_uid.clone(), expected.class_uid.clone())
                {
                    return Err("CV PortSetup direct child changed after metadata cleanup".into());
                }
            }
        }
        Ok(())
    }
    fn borrowed(&self) -> Result<(), String> {
        self.borrowed_ids()?;
        self.borrowed_headers()?;
        self.borrowed_attachments()
    }
}

pub(in crate::native_script::request) struct CvPortsInventory {
    inspection: Option<Inspection>,
    route: Route,
}
impl CvPortsInventory {
    pub(super) fn capture(ctx: &Context<'static>, text: TextApi) -> Result<Self, String> {
        admit(&ctx.instance)?;
        let get = constant_string().ok_or("native string constants unavailable")?;
        let name = unsafe { get(c"PortSetup".as_ptr()) };
        if name.is_null() {
            return Err("fixed PortSetup identifier unavailable".into());
        }
        let route = Route::capture(ctx.target.0, ctx.node.0, name, &mut || ctx.check())?;
        let inspection = if let Some(root) = &route.root {
            let held = hold(root.pin.node.0).ok_or("CV PortSetup unavailable")?;
            let pointer = NonNull::new(held.0).expect("held PortSetup");
            std::mem::forget(held);
            let inspector = unsafe { NodeInspector::from_owned(pointer, text) };
            let result = inspector.inspect(limits(Scope::CvPorts), &mut || ctx.check());
            drop(inspector);
            ctx.check()?;
            let inspection = result.map_err(|error| error.to_string())?;
            validate_root(inspection.report(), &root.snapshot)?;
            Some(inspection)
        } else {
            None
        };
        let inventory = Self { inspection, route };
        inventory.check(&mut || ctx.check(), &mut || ctx.check())?;
        Ok(inventory)
    }
    pub(super) fn check<G, B>(&self, guard: &mut G, borrowed_guard: &mut B) -> Result<(), String>
    where
        G: FnMut() -> Result<(), String>,
        B: FnMut() -> Result<(), String>,
    {
        guard()?;
        if let Some(inspection) = &self.inspection {
            inspection.check(guard).map_err(|error| error.to_string())?;
            validate_root(
                inspection.report(),
                &self.route.root.as_ref().expect("present root").snapshot,
            )?;
        }
        self.route.metadata(guard)?;
        guard()?;
        self.route.borrowed()?;
        borrowed_guard()
    }
    #[cfg(test)]
    pub(super) fn report(&self) -> Option<&Report> {
        self.inspection.as_ref().map(Inspection::report)
    }
    pub(super) fn value(&self) -> Value {
        let mut value = self.inspection.as_ref().map_or_else(
            || json!({"scope":"cv_ports","availability":"lookup_absent"}),
            |inspection| report(inspection.report(), Scope::CvPorts),
        );
        if let Some(root) = &self.route.root {
            value["availability"] = json!("observed");
            value["root_type_observed"] = json!(true);
            value["indexed_membership"] = json!({"child_index":root.slot_index,"slot_child_count":root.slot_count,"unique_canonical_match":true});
            value["child_metadata_complete"] = json!(
                self.inspection
                    .as_ref()
                    .is_some_and(|inspection| inspection.report().complete)
            );
        }
        value["owner"] =
            json!({"device":self.route.device_snapshot,"slot":self.route.slot_snapshot});
        value["assignment_values_observed"] = json!(false);
        value["routing_verified"] = json!(false);
        value["atomic_snapshot"] = json!(false);
        value["mutation_capability"] = json!(false);
        value["controller_support"] = json!("unknown");
        value["metadata_scope"] =
            json!("bounded_existing_hierarchy_reflection_with_parent_and_fixed_delegate_queries");
        value
    }
}
fn validate_root(report: &Report, expected: &Snapshot) -> Result<(), String> {
    let root = report
        .nodes
        .first()
        .ok_or("CV PortSetup root metadata unavailable")?;
    let identity = root
        .identity
        .as_ref()
        .ok_or("CV PortSetup root identity unavailable")?;
    if root.index != 0
        || identity.id != expected.id
        || identity.object_uid.as_deref() != Some(&expected.object_uid)
        || identity.class_uid != expected.class_uid
        || !root.reflection.available
        || root
            .reflection
            .types
            .first()
            .is_none_or(|info| info.name != "AudioFXPortSetup")
    {
        return Err("CV PortSetup first type or identity unsupported".into());
    }
    Ok(())
}

#[cfg(test)]
#[path = "cv_ports/tests.rs"]
mod tests;

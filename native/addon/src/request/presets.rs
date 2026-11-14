//! Finite, service-local preset capabilities. Paths and imported bytes never
//! cross this boundary; only entries observed through the fixed slot are used.
use super::*;
use fender_native_preset_api::{Availability, Catalog, Inputs, Page};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

const MAX_CATALOGS: usize = 4;
pub(super) const MAX_PAGE: u32 = 64;
static LIVE_CATALOGS: AtomicU32 = AtomicU32::new(0);
pub(super) fn default_limit() -> u32 {
    32
}
pub(crate) fn live_count() -> u32 {
    LIVE_CATALOGS.load(Ordering::Acquire)
}
pub(crate) fn limits() -> Value {
    json!({"max_count":MAX_CATALOGS,"max_entries_per_catalog":fender_native_preset_api::MAX_PRESETS,"max_page_size":MAX_PAGE,"default_page_size":default_limit()})
}
const MEDIATOR: Uid = Uid {
    a: 0x25e340af,
    b: 0xd6e6,
    c: 0x49cf,
    d: [0xb2, 0x58, 0x87, 0x47, 0x6e, 0x4a, 0x27, 0x3a],
};
const DATA_TARGET: Uid = Uid {
    a: 0x9967d17c,
    b: 0xa519,
    c: 0x4c00,
    d: [0xa7, 0xcd, 0x92, 0x1e, 0x4d, 0x7c, 0x5e, 0xcb],
};
const MANAGER: Uid = Uid {
    a: 0x861a7175,
    b: 0xac7b,
    c: 0x457c,
    d: [0xac, 0xcd, 0x8b, 0x35, 0xdf, 0xcc, 0xea, 0x9d],
};
const URL: Uid = Uid {
    a: 0xbfca729d,
    b: 0x5097,
    c: 0x4b38,
    d: [0x9f, 0x15, 0x52, 0x13, 0xf9, 0x79, 0, 0xc4],
};
const FILE_SYSTEM: Uid = Uid {
    a: 0x3e510860,
    b: 0x30ad,
    c: 0x4f70,
    d: [0xa9, 0x33, 0x6b, 0x0c, 0x1d, 0xc4, 0x13, 0x28],
};

#[derive(Clone, PartialEq, Eq)]
struct Key {
    document: *mut c_void,
    model: *mut c_void,
    target: *mut c_void,
    object_uid: String,
    class_uid: String,
}
impl Key {
    fn same_document(&self, ctx: &Context<'_>) -> bool {
        self.document == ctx.document._identity.0 && self.model == ctx.document._model_identity.0
    }
    fn same_target(&self, ctx: &Context<'_>, target: *mut c_void) -> bool {
        self.same_document(ctx)
            && self.target == target
            && self.object_uid == ctx.instance.object_uid
            && self.class_uid == ctx.instance.class_uid
    }
}

struct Route {
    // Interfaces and children release before their owning document pins.
    presets: Owned,
    _presets_identity: Owned,
    slot: Owned,
    _slot_identity: Owned,
    device: Owned,
    _target: Owned,
    _model: Owned,
    _document: Owned,
    key: Key,
    name: *const PlainString,
    slot_identity: (String, String, String),
    presets_identity: (String, String, String),
}
fn node_uids(node: *mut c_void) -> (String, String) {
    let object: unsafe extern "C" fn(*mut c_void) -> *const Uid = unsafe { method(node, 4) };
    let class: unsafe extern "C" fn(*mut c_void) -> *const Uid = unsafe { method(node, 5) };
    (uid(unsafe { object(node) }), uid(unsafe { class(node) }))
}
fn node_identity(node: *mut c_void) -> (String, String, String) {
    let (object_uid, class_uid) = node_uids(node);
    // A slot may have no class UID. Retain its exact observed class name as
    // part of the binding, rather than letting a stable pointer/UID mask drift.
    (object_uid, class_uid, class_name(node))
}
fn supported_slot_family(name: &str) -> bool {
    matches!(
        name,
        "AudioInsertSlot" | "AudioSynthSlot" | "MusicFXSlot" | "AudioFXSlot"
    )
}
impl Route {
    fn capture(ctx: &Context<'_>) -> Result<Self, String> {
        ctx.check()?;
        let get = constant_string().ok_or("native string constants unavailable")?;
        let name = unsafe { get(c"Presets".as_ptr()) };
        if name.is_null() {
            return Err("Presets identifier unavailable".into());
        }
        let device = hold(ctx.node.0).ok_or("Device node unavailable")?;
        let parent: Borrowed = unsafe { method(device.0, 6) };
        let slot = hold(unsafe { parent(device.0) }).ok_or("Device slot unavailable")?;
        let slot_identity = node_identity(slot.0);
        if !supported_slot_family(&slot_identity.2) {
            return Err("preset catalog requires the observed AudioInsertSlot, AudioSynthSlot, MusicFXSlot or AudioFXSlot family".into());
        }
        let find: unsafe extern "C" fn(*mut c_void, *const PlainString) -> *mut c_void =
            unsafe { method(slot.0, 10) };
        let presets =
            hold(unsafe { find(slot.0, name) }).ok_or("slot Presets child unavailable")?;
        let presets_identity = node_identity(presets.0);
        if presets_identity.2 != "PresetComponent" {
            return Err("slot Presets child has an unsupported native class".into());
        }
        if !valid_uid(&slot_identity.0)
            || !valid_uid(&presets_identity.0)
            || !valid_uid(&presets_identity.1)
        {
            return Err("slot or Presets child has no stable native identity".into());
        }
        let target =
            query(ctx.target.0, &UNKNOWN).ok_or("Device canonical identity unavailable")?;
        let key = Key {
            document: ctx.document._identity.0,
            model: ctx.document._model_identity.0,
            target: target.0,
            object_uid: ctx.instance.object_uid.clone(),
            class_uid: ctx.instance.class_uid.clone(),
        };
        let route = Self {
            _presets_identity: query(presets.0, &UNKNOWN).ok_or("Presets identity unavailable")?,
            _slot_identity: query(slot.0, &UNKNOWN).ok_or("slot identity unavailable")?,
            _model: hold(ctx.document._model_identity.0).ok_or("model identity unavailable")?,
            _document: hold(ctx.document._identity.0).ok_or("document identity unavailable")?,
            presets,
            slot,
            device,
            _target: target,
            key,
            name,
            slot_identity,
            presets_identity,
        };
        route.guard(ctx)?;
        Ok(route)
    }
    fn attachment(&self) -> Result<(), String> {
        // Class reflection queries and releases temporary native IObject
        // owners. Finish every metadata observation and its cleanup before the
        // fresh borrowed parent/child reads; release may detach this route.
        let slot_identity = node_identity(self.slot.0);
        let presets_identity = node_identity(self.presets.0);
        if slot_identity != self.slot_identity || presets_identity != self.presets_identity {
            return Err("Device slot or Presets attachment changed".into());
        }
        // Reflection cleanup may also change UIDs in place. Refresh these
        // borrowed identity values after BOTH reflection owners have released;
        // no new native owner or reflected metadata follows this phase.
        let slot_uids = node_uids(self.slot.0);
        let presets_uids = node_uids(self.presets.0);
        if slot_uids.0 != self.slot_identity.0
            || slot_uids.1 != self.slot_identity.1
            || presets_uids.0 != self.presets_identity.0
            || presets_uids.1 != self.presets_identity.1
        {
            return Err("Device slot or Presets attachment changed".into());
        }
        let parent: Borrowed = unsafe { method(self.device.0, 6) };
        let find: unsafe extern "C" fn(*mut c_void, *const PlainString) -> *mut c_void =
            unsafe { method(self.slot.0, 10) };
        if unsafe { parent(self.device.0) } != self.slot.0
            || unsafe { find(self.slot.0, self.name) } != self.presets.0
        {
            Err("Device slot or Presets attachment changed".into())
        } else {
            Ok(())
        }
    }
    fn guard(&self, ctx: &Context<'_>) -> Result<(), String> {
        self.attachment()?;
        // Preserve the existing final current-document/target/service check.
        // These sequential observations do not establish an atomic graph.
        ctx.check()
    }
    fn matches(&self, ctx: &Context<'_>) -> Result<(), String> {
        let target = query(ctx.target.0, &UNKNOWN).ok_or("Device identity unavailable")?;
        let matches = self.key.same_target(ctx, target.0);
        drop(target);
        ctx.check()?;
        if matches {
            Ok(())
        } else {
            Err("catalog belongs to a different service/document/model/Device".into())
        }
    }
}

pub(crate) struct Lease {
    catalog: Catalog,
    route: Route,
    issued: RefCell<Vec<bool>>,
}
impl Drop for Lease {
    fn drop(&mut self) {
        LIVE_CATALOGS.fetch_sub(1, Ordering::Relaxed);
    }
}
impl Lease {
    fn check(&self, ctx: &Context<'_>) -> Result<(), String> {
        self.route.matches(ctx)?;
        self.catalog
            .check(&mut || self.route.guard(ctx))
            .map_err(|e| e.to_string())?;
        self.route.guard(ctx)
    }
}
/// Registry ownership and publication ownership are separate. Taking a record
/// never destroys its host references while the registry is mutably borrowed.
struct Record<T> {
    token: String,
    lease: Rc<T>,
}
pub(crate) struct Registry<T = Lease> {
    entries: Vec<Record<T>>,
}
impl<T> Registry<T> {
    pub(crate) fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
    fn capacity(&self) -> Result<(), String> {
        if self.entries.len() < MAX_CATALOGS {
            Ok(())
        } else {
            Err("native preset catalog count limit reached".into())
        }
    }
    fn get(&self, token: &str) -> Result<Rc<T>, String> {
        self.entries
            .iter()
            .find(|e| e.token == token)
            .map(|e| Rc::clone(&e.lease))
            .ok_or("catalog token is expired, released or belongs to another service".into())
    }
    fn take(&mut self, token: &str) -> Result<Record<T>, String> {
        self.entries
            .iter()
            .position(|e| e.token == token)
            .map(|i| self.entries.swap_remove(i))
            .ok_or("catalog token is expired, released or belongs to another service".into())
    }
}
fn with_registry<T>(service: *mut Service, f: impl FnOnce(&mut Registry) -> T) -> T {
    let service = unsafe { &*service };
    // No foreign calls, native owners dropped, or Rust callbacks in this borrow.
    let registry = unsafe { &mut *service.presets.get() };
    let result = f(registry);
    service
        .presets_pending
        .store(!registry.entries.is_empty(), Ordering::Release);
    result
}
pub(super) fn expire(ctx: &Context<'_>) -> Result<(), String> {
    let old = with_registry(ctx.service, |r| {
        let mut old = Vec::new();
        let mut i = 0;
        while i < r.entries.len() {
            if r.entries[i].lease.route.key.same_document(ctx) {
                i += 1;
            } else {
                old.push(r.entries.swap_remove(i));
            }
        }
        old
    });
    drop(old);
    ctx.check()
}
pub(crate) fn drain(service: *mut Service) {
    let old = with_registry(service, |r| std::mem::take(&mut r.entries));
    drop(old);
}
#[link(name = "System")]
unsafe extern "C" {
    fn arc4random_buf(buffer: *mut c_void, bytes: usize);
}
fn token() -> String {
    let mut bytes = [0u8; 32];
    unsafe { arc4random_buf(bytes.as_mut_ptr().cast(), bytes.len()) };
    bytes.iter().map(|b| format!("{b:02X}")).collect()
}
fn transfer(owner: Owned) -> NonNull<c_void> {
    let p = NonNull::new(owner.0).expect("owned interface");
    std::mem::forget(owner);
    p
}
fn discover(ctx: &Context<'_>, auxiliary: Owned) -> Result<Rc<Lease>, String> {
    with_registry(ctx.service, |r| r.capacity())?;
    let route = Route::capture(ctx)?;
    let mediator = query(route.slot.0, &MEDIATOR).ok_or("slot IPresetMediator unavailable")?;
    let data_target =
        query(route.presets.0, &DATA_TARGET).ok_or("Presets IDataTarget unavailable")?;
    let empty_url = query(auxiliary.0, &URL).ok_or("auxiliary has no IUrl interface")?;
    drop(auxiliary);
    let table = unsafe { (ctx.api.objects)() };
    if table.is_null() {
        return Err("object table unavailable".into());
    }
    let by_id: unsafe extern "C" fn(*mut c_void, *const Uid) -> *mut c_void =
        unsafe { method(table, 8) };
    let manager = hold(unsafe { by_id(table, &MANAGER) }).ok_or("preset manager unavailable")?;
    let exact_manager = query(manager.0, &MANAGER).ok_or("IPresetManager unavailable")?;
    drop(manager);
    let plugin_manager =
        hold(unsafe { (ctx.api.plugin_manager)() }).ok_or("plug-in manager unavailable")?;
    let allocator =
        query(plugin_manager.0, &ALLOCATOR).ok_or("plug-in IClassAllocator unavailable")?;
    drop(plugin_manager);
    let native_file_system =
        hold(unsafe { (ctx.api.file_system)() }).ok_or("native file system unavailable")?;
    let file_system = query(native_file_system.0, &FILE_SYSTEM).ok_or("IFileSystem unavailable")?;
    drop(native_file_system);
    route.guard(ctx)?;
    let inputs = Inputs {
        manager: transfer(exact_manager),
        mediator: transfer(mediator),
        data_target: transfer(data_target),
        allocator: transfer(allocator),
        empty_url: transfer(empty_url),
        file_system: transfer(file_system),
    };
    let catalog = unsafe { Catalog::discover_owned(inputs, &mut || route.guard(ctx)) }
        .map_err(|e| e.to_string())?;
    route.guard(ctx)?;
    LIVE_CATALOGS.fetch_add(1, Ordering::Relaxed);
    Ok(Rc::new(Lease {
        catalog,
        route,
        issued: RefCell::new(vec![false; fender_native_preset_api::MAX_PRESETS]),
    }))
}

pub(super) struct Receipt {
    lease: Rc<Lease>,
    active: bool,
}
impl Receipt {
    pub(super) fn check(&self, ctx: &Context<'_>) -> Result<(), String> {
        if self.active {
            self.lease.check(ctx)
        } else {
            self.lease.route.matches(ctx)
        }
    }
}
fn availability(value: &Availability) -> (&'static str, bool) {
    match value {
        Availability::LocalReadOnly => ("local_read_only", true),
        Availability::LocalWritable => ("local_writable", true),
        Availability::MissingLocalFile => ("missing_local_file", false),
        Availability::NoLocation => ("no_location", false),
        Availability::UnsupportedProtocol(_) => ("unsupported_protocol", false),
        Availability::NonLocalHost => ("non_local_host", false),
        Availability::InvalidLocalLocation => ("invalid_local_location", false),
        Availability::ClassMismatch => ("class_mismatch", false),
    }
}
fn page_value(token: &str, page: &Page) -> Value {
    let mut excluded = BTreeMap::<&str, u32>::new();
    let entries = page
        .entries
        .iter()
        .map(|entry| {
            let (availability, recallable) = availability(&entry.availability);
            if !recallable {
                *excluded.entry(availability).or_default() += 1;
            }
            json!({"index":entry.index,"name":entry.name,"class_key":entry.class_key,
            "read_only":entry.read_only,"availability":availability,"recallable":recallable})
        })
        .collect::<Vec<_>>();
    json!({"ok":true,"outcome":"observed","catalog_token":token,"entries":entries,"observed_count":page.observed_count,
        "offset":page.offset,"next":page.next,"catalog_complete":page.catalog_complete,"excluded_on_page":excluded,
        "factory_origin":"not_inferred","recall_policy":"matching_class_available_local_file"})
}
/// Only a public QI's documented absence is an optional capability miss.
/// Failed/malformed outputs are not owned references and are never inspected.
fn name_interface(object: *mut c_void, iid: &Uid) -> Result<Option<Owned>, String> {
    let mut out = ptr::null_mut();
    let result = unsafe { (unknown_vtable(object).query)(object, iid, &mut out) };
    match (result, out.is_null()) {
        (OK, false) => Ok(Some(Owned(out))),
        (NO_INTERFACE, true) => Ok(None),
        _ => Err("preset name interface query returned invalid evidence".into()),
    }
}
fn unavailable_name(reason: &'static str) -> Value {
    json!({"status":"unavailable","reason":reason,"scope":"preset_component_display","atomic_snapshot":false})
}
fn current_name<G: FnMut() -> Result<(), String>>(
    presets: &Owned,
    identity: &Owned,
    api: &Api,
    guard: &mut G,
) -> Result<Value, String> {
    guard()?;
    let mut controller = None;
    let mut canonical = None;
    let mut parameter = None;
    let result = (|| {
        let queried = name_interface(presets.0, &CONTROLLER);
        controller = queried?.map(|handle| Controller {
            handle,
            lease: None,
        });
        guard()?;
        let Some(controller) = controller.as_ref() else {
            return Ok(unavailable_name("controller_unavailable"));
        };
        canonical = name_interface(controller.handle.0, &UNKNOWN)?;
        guard()?;
        if canonical.as_ref().is_none_or(|owner| owner.0 != identity.0) {
            return Err("preset name controller identity changed".into());
        }
        parameter = controller.find("presetName", api).ok();
        guard()?;
        let Some(parameter) = parameter.as_ref() else {
            return Ok(unavailable_name("preset_name_unavailable"));
        };
        match parameter.string_display("presetName", guard) {
            Ok(display_text) => Ok(json!({"status":"observed","display_text":display_text,
                "scope":"preset_component_display","atomic_snapshot":false})),
            Err(parameters::Error::UnsupportedKind) => {
                Ok(unavailable_name("preset_name_not_string"))
            }
            Err(error) => Err(error.to_string()),
        }
    })();
    // The original Route pins Presets through ALL temporary owner cleanup.
    // Its fresh attachment/context guard must be the last foreign observation.
    let before_release = guard();
    drop(parameter);
    drop(canonical);
    drop(controller);
    let after_release = guard();
    before_release.and(after_release)?;
    result
}
pub(super) fn invalidate(value: &mut Value) {
    if let Some(value) = value.as_object_mut() {
        value.remove("current_preset_observation");
    }
}
fn page(
    ctx: &Context<'_>,
    catalog_token: Option<&str>,
    offset: u32,
    limit: u32,
    auxiliary: Option<Owned>,
) -> Result<(Value, Receipt), String> {
    let (token, lease, new) = if let Some(token) = catalog_token {
        (
            token.to_owned(),
            with_registry(ctx.service, |r| r.get(token))?,
            false,
        )
    } else {
        (
            token(),
            discover(
                ctx,
                auxiliary.ok_or("initial catalog requires empty IUrl auxiliary")?,
            )?,
            true,
        )
    };
    lease.check(ctx)?;
    let page = lease
        .catalog
        .page(offset, limit, &mut || lease.route.guard(ctx))
        .map_err(|e| e.to_string())?;
    let mut value = page_value(&token, &page);
    value["current_preset_observation"] = current_name(
        &lease.route.presets,
        &lease.route._presets_identity,
        ctx.api,
        &mut || lease.route.guard(ctx),
    )?;
    for entry in &page.entries {
        lease.issued.borrow_mut()[entry.index as usize] = true;
    }
    if new {
        with_registry(ctx.service, |r| r.capacity())?;
        let record = Record {
            token,
            lease: Rc::clone(&lease),
        };
        with_registry(ctx.service, |r| r.entries.push(record));
    }
    lease.check(ctx)?;
    Ok((
        value,
        Receipt {
            lease,
            active: true,
        },
    ))
}
fn apply(ctx: &Context<'_>, token: &str, index: u32) -> (Value, Option<Receipt>) {
    let lease = match with_registry(ctx.service, |r| r.get(token)) {
        Ok(lease) => lease,
        Err(reason) => return (error("invalid_catalog_token", &reason, "not_applied"), None),
    };
    let result = (|| {
        lease.check(ctx).map_err(|e| (e, false))?;
        if !issued_index(&lease.issued.borrow(), index) {
            return Err(("preset index was not issued by this catalog".into(), false));
        }
        lease
            .catalog
            .recall(index, &mut || lease.route.guard(ctx))
            .map_err(|e| (e.error.to_string(), e.dispatched))
    })();
    (
        apply_value(token, index, result),
        Some(Receipt {
            lease,
            active: true,
        }),
    )
}
fn issued_index(issued: &[bool], index: u32) -> bool {
    issued.get(index as usize).copied().unwrap_or(false)
}
fn apply_value(
    token: &str,
    index: u32,
    result: Result<fender_native_preset_api::Recall, (String, bool)>,
) -> Value {
    match result {
        Ok(recall) => {
            json!({"ok":true,"catalog_token":token,"index":index,"accepted":recall.accepted,"verified":false,
            "outcome":if recall.accepted { "applied_unverified" } else { "not_applied" }})
        }
        Err((reason, dispatched)) => error(
            "preset_apply_failed",
            &reason,
            if dispatched { "unknown" } else { "not_applied" },
        ),
    }
}
fn release(ctx: &Context<'_>, token: &str) -> Result<(Value, Receipt), String> {
    let lease = with_registry(ctx.service, |r| r.get(token))?;
    lease.route.matches(ctx)?;
    let record = with_registry(ctx.service, |r| r.take(token))?;
    drop(record);
    ctx.check()?;
    Ok((
        json!({"ok":true,"catalog_token":token,"released":true,"verified":true,"outcome":"applied_verified"}),
        Receipt {
            lease,
            active: false,
        },
    ))
}
pub(super) fn execute(
    ctx: &Context<'_>,
    request: &Request,
    auxiliary: Option<Owned>,
) -> (Value, Option<Receipt>) {
    let result = match request {
        Request::Presets {
            catalog_token,
            offset,
            limit,
            ..
        } => page(ctx, catalog_token.as_deref(), *offset, *limit, auxiliary),
        Request::PresetApply {
            catalog_token,
            index,
            ..
        } => return apply(ctx, catalog_token, *index),
        Request::PresetRelease { catalog_token, .. } => release(ctx, catalog_token),
        _ => unreachable!("finite preset dispatch"),
    };
    match result {
        Ok((value, receipt)) => (value, Some(receipt)),
        Err(reason) => (error("preset_request_failed", &reason, "not_applied"), None),
    }
}

#[cfg(test)]
#[path = "presets_name_tests.rs"]
mod name_tests;
#[cfg(test)]
#[path = "presets_tests.rs"]
mod tests;

#[cfg(test)]
mod family_tests {
    use super::*;
    use std::{cell::Cell, ffi::CStr};

    #[repr(C)]
    struct View {
        table: *const *const c_void,
        owner: *mut Node,
    }
    #[repr(C)]
    struct Node {
        table: *const *const c_void,
        refs: Cell<u32>,
        parent: Cell<*mut c_void>,
        child: Cell<*mut c_void>,
        detach_on_release: Cell<*mut c_void>,
        uid_on_release: Cell<Option<Uid>>,
        class_uid_on_release: Cell<Option<Uid>>,
        object_uid: Cell<Uid>,
        class_uid: Cell<Uid>,
        class_uid_available: Cell<bool>,
        object_available: Cell<bool>,
        class_name: Cell<&'static CStr>,
        object: View,
        info: View,
    }
    unsafe fn node<'a>(pointer: *mut c_void) -> &'a Node {
        unsafe { &*pointer.cast::<Node>() }
    }
    unsafe extern "C" fn retain(pointer: *mut c_void) -> u32 {
        let node = unsafe { node(pointer) };
        node.refs.set(node.refs.get() + 1);
        node.refs.get()
    }
    unsafe extern "C" fn release(pointer: *mut c_void) -> u32 {
        let node = unsafe { node(pointer) };
        node.refs
            .set(node.refs.get().checked_sub(1).expect("double release"));
        node.refs.get()
    }
    unsafe extern "C" fn query_node(
        pointer: *mut c_void,
        iid: *const Uid,
        out: *mut *mut c_void,
    ) -> i32 {
        let node = unsafe { node(pointer) };
        let iid = unsafe { *iid };
        let found = if iid == UNKNOWN {
            pointer
        } else if iid == OBJECT && node.object_available.get() {
            ptr::from_ref(&node.object).cast_mut().cast()
        } else {
            return NO_INTERFACE;
        };
        unsafe {
            retain(pointer);
            *out = found;
        }
        OK
    }
    unsafe extern "C" fn release_object(pointer: *mut c_void) -> u32 {
        let owner = unsafe { &*(*pointer.cast::<View>()).owner };
        let replacement = owner.detach_on_release.replace(ptr::null_mut());
        if !replacement.is_null() {
            owner.child.set(replacement);
        }
        if let Some(uid) = owner.uid_on_release.take() {
            owner.object_uid.set(uid);
        }
        if let Some(uid) = owner.class_uid_on_release.take() {
            owner.class_uid.set(uid);
        }
        unsafe { release((*pointer.cast::<View>()).owner.cast()) }
    }
    unsafe extern "C" fn info(pointer: *mut c_void) -> *mut c_void {
        let owner = unsafe { &*(*pointer.cast::<View>()).owner };
        ptr::from_ref(&owner.info).cast_mut().cast()
    }
    unsafe extern "C" fn name(pointer: *mut c_void) -> *const c_char {
        unsafe { &*(*pointer.cast::<View>()).owner }
            .class_name
            .get()
            .as_ptr()
    }
    unsafe extern "C" fn object_uid(pointer: *mut c_void) -> *const Uid {
        unsafe { node(pointer) }.object_uid.as_ptr()
    }
    unsafe extern "C" fn class_uid(pointer: *mut c_void) -> *const Uid {
        let node = unsafe { node(pointer) };
        if node.class_uid_available.get() {
            node.class_uid.as_ptr()
        } else {
            ptr::null()
        }
    }
    unsafe extern "C" fn parent(pointer: *mut c_void) -> *mut c_void {
        unsafe { node(pointer) }.parent.get()
    }
    unsafe extern "C" fn child(pointer: *mut c_void, _: *const PlainString) -> *mut c_void {
        unsafe { node(pointer) }.child.get()
    }

    struct Nodes {
        nodes: Box<[Node; 3]>,
        _node_table: Box<[*const c_void; 11]>,
        _object_table: Box<[*const c_void; 4]>,
        _info_table: Box<[*const c_void; 6]>,
    }
    impl Nodes {
        fn new(family: &'static CStr) -> Self {
            let mut node_table = Box::new([ptr::null(); 11]);
            node_table[0] = query_node as *const c_void;
            node_table[1] = retain as *const c_void;
            node_table[2] = release as *const c_void;
            node_table[4] = object_uid as *const c_void;
            node_table[5] = class_uid as *const c_void;
            node_table[6] = parent as *const c_void;
            node_table[10] = child as *const c_void;
            let mut object_table = Box::new([ptr::null(); 4]);
            object_table[2] = release_object as *const c_void;
            object_table[3] = info as *const c_void;
            let mut info_table = Box::new([ptr::null(); 6]);
            info_table[5] = name as *const c_void;
            let mut nodes = Box::new(std::array::from_fn(|index| Node {
                table: node_table.as_ptr(),
                refs: Cell::new(1),
                parent: Cell::new(ptr::null_mut()),
                child: Cell::new(ptr::null_mut()),
                detach_on_release: Cell::new(ptr::null_mut()),
                uid_on_release: Cell::new(None),
                class_uid_on_release: Cell::new(None),
                object_uid: Cell::new(SERVICE_CID),
                class_uid: Cell::new(API_CLASS),
                class_uid_available: Cell::new(index != 1),
                object_available: Cell::new(true),
                class_name: Cell::new(if index == 1 {
                    family
                } else if index == 2 {
                    c"PresetComponent"
                } else {
                    c"Device"
                }),
                object: View {
                    table: object_table.as_ptr(),
                    owner: ptr::null_mut(),
                },
                info: View {
                    table: info_table.as_ptr(),
                    owner: ptr::null_mut(),
                },
            }));
            for node in &mut *nodes {
                let pointer = ptr::from_mut(node);
                node.object.owner = pointer;
                node.info.owner = pointer;
            }
            let result = Self {
                nodes,
                _node_table: node_table,
                _object_table: object_table,
                _info_table: info_table,
            };
            result.nodes[0].parent.set(result.pointer(1));
            result.nodes[1].child.set(result.pointer(2));
            result
        }
        fn pointer(&self, index: usize) -> *mut c_void {
            ptr::from_ref(&self.nodes[index]).cast_mut().cast()
        }
        fn route(&self, ctx: &Context<'_>) -> Route {
            let target = query(ctx.target.0, &UNKNOWN).unwrap();
            Route {
                presets: hold(self.pointer(2)).unwrap(),
                _presets_identity: query(self.pointer(2), &UNKNOWN).unwrap(),
                slot: hold(self.pointer(1)).unwrap(),
                _slot_identity: query(self.pointer(1), &UNKNOWN).unwrap(),
                device: hold(self.pointer(0)).unwrap(),
                key: Key {
                    document: ctx.document._identity.0,
                    model: ctx.document._model_identity.0,
                    target: target.0,
                    object_uid: ctx.instance.object_uid.clone(),
                    class_uid: ctx.instance.class_uid.clone(),
                },
                _target: target,
                _model: hold(ctx.document._model_identity.0).unwrap(),
                _document: hold(ctx.document._identity.0).unwrap(),
                name: ptr::null(),
                slot_identity: node_identity(self.pointer(1)),
                presets_identity: node_identity(self.pointer(2)),
            }
        }
    }
    impl Drop for Nodes {
        fn drop(&mut self) {
            assert!(
                self.nodes.iter().all(|node| node.refs.get() == 1),
                "route/reflection reference leak"
            );
        }
    }

    #[test]
    fn preset_slot_policy_accepts_only_the_four_observed_exact_families() {
        for family in [
            "AudioInsertSlot",
            "AudioSynthSlot",
            "MusicFXSlot",
            "AudioFXSlot",
        ] {
            assert!(supported_slot_family(family));
        }
        for family in [
            "",
            "DeviceSlot",
            "AudioFXSlotExtra",
            "CustomAudioFXSlot",
            "AudioFXSlot/Presets",
            "audiofxslot",
            "AudioSynthFolder",
            "MusicFXFolder",
            "MixFXSlot",
            "PannerSlot",
            "audiosynthslot",
            "AudioSynthSlot/Presets",
            "CustomAudioInsertSlot",
        ] {
            assert!(!supported_slot_family(family));
        }
    }

    #[test]
    fn all_observed_families_keep_slot_and_preset_pins_with_null_slot_class_uid() {
        let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
        for family in [
            c"AudioInsertSlot",
            c"AudioSynthSlot",
            c"MusicFXSlot",
            c"AudioFXSlot",
        ] {
            let mut host = super::super::guard_tests::Fixture::new();
            host.canonical_queries();
            let nodes = Nodes::new(family);
            let mut ctx = host.context();
            ctx.node = hold(nodes.pointer(0)).unwrap();
            let route = nodes.route(&ctx);
            assert_eq!(route.slot_identity.1, "");
            assert_eq!(route.slot_identity.2, family.to_str().unwrap());
            assert_eq!(route.presets_identity.2, "PresetComponent");
            route.guard(&ctx).unwrap();
            route.matches(&ctx).unwrap();
            assert!(nodes.nodes.iter().all(|node| node.refs.get() > 1));
            drop(route);
            drop(ctx);
            drop(nodes);
        }
    }

    #[test]
    fn same_identity_class_drift_and_unavailable_class_metadata_are_refused() {
        let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
        let mut host = super::super::guard_tests::Fixture::new();
        host.canonical_queries();
        let nodes = Nodes::new(c"AudioSynthSlot");
        let mut ctx = host.context();
        ctx.node = hold(nodes.pointer(0)).unwrap();
        let route = nodes.route(&ctx);
        for changed in [c"MusicFXSlot", c"DeviceSlot", c""] {
            nodes.nodes[1].class_name.set(changed);
            assert!(
                route.guard(&ctx).is_err(),
                "same canonical slot/class UID accepted changed family"
            );
        }
        nodes.nodes[1].class_name.set(c"AudioSynthSlot");
        route.guard(&ctx).unwrap();
        nodes.nodes[2].class_name.set(c"OtherPresetComponent");
        assert!(route.guard(&ctx).is_err());
        nodes.nodes[2].class_name.set(c"PresetComponent");
        nodes.nodes[1].object_available.set(false);
        assert!(route.guard(&ctx).is_err());
        nodes.nodes[1].object_available.set(true);
        route.guard(&ctx).unwrap();
        nodes.nodes[0].parent.set(nodes.pointer(2));
        assert!(route.guard(&ctx).is_err());
        nodes.nodes[0].parent.set(nodes.pointer(1));
        nodes.nodes[1].child.set(nodes.pointer(0));
        assert!(route.guard(&ctx).is_err());
        nodes.nodes[1].child.set(nodes.pointer(2));
        route.guard(&ctx).unwrap();
        host.switch_document();
        assert!(route.guard(&ctx).is_err());
        assert!(route.matches(&ctx).is_err());
        drop(route);
        drop(ctx);
        drop(nodes);
    }

    #[test]
    fn audiofx_family_still_requires_the_exact_bound_class_and_object_uid() {
        let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
        let mut host = super::super::guard_tests::Fixture::new();
        host.canonical_queries();
        let nodes = Nodes::new(c"AudioFXSlot");
        let mut ctx = host.context();
        ctx.node = hold(nodes.pointer(0)).unwrap();
        let route = nodes.route(&ctx);
        assert_eq!(route.slot_identity.1, "");
        route.guard(&ctx).unwrap();
        nodes.nodes[1].class_name.set(c"AudioFXSlotExtra");
        assert!(route.guard(&ctx).is_err());
        nodes.nodes[1].class_name.set(c"AudioFXSlot");
        nodes.nodes[1].object_uid.set(Uid {
            a: 0,
            b: 0,
            c: 0,
            d: [0; 8],
        });
        assert!(
            route.guard(&ctx).is_err(),
            "AudioFXSlot must not lose its object UID binding"
        );
        nodes.nodes[1].object_uid.set(SERVICE_CID);
        route.guard(&ctx).unwrap();
        drop(route);
        drop(ctx);
        drop(nodes);
    }

    #[test]
    fn reflection_owner_release_cannot_hide_a_detached_presets_child() {
        let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
        let mut host = super::super::guard_tests::Fixture::new();
        host.canonical_queries();
        let nodes = Nodes::new(c"AudioSynthSlot");
        let mut ctx = host.context();
        ctx.node = hold(nodes.pointer(0)).unwrap();
        let route = nodes.route(&ctx);
        route.guard(&ctx).unwrap();
        nodes.nodes[1].detach_on_release.set(nodes.pointer(0));
        let result = route.guard(&ctx);
        assert_eq!(nodes.nodes[1].child.get(), nodes.pointer(0));
        assert!(
            result.is_err(),
            "reflection owner release detached Presets after the cached child read"
        );
        drop(route);
        drop(ctx);
        drop(nodes);
    }

    #[test]
    fn reflection_owner_release_cannot_hide_same_pointer_uid_drift() {
        let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
        for changed_node in [1, 2] {
            for class_uid in [false, true] {
                let mut host = super::super::guard_tests::Fixture::new();
                host.canonical_queries();
                let nodes = Nodes::new(c"AudioSynthSlot");
                nodes.nodes[changed_node].class_uid_available.set(true);
                let mut ctx = host.context();
                ctx.node = hold(nodes.pointer(0)).unwrap();
                let route = nodes.route(&ctx);
                route.guard(&ctx).unwrap();
                if class_uid {
                    nodes.nodes[changed_node]
                        .class_uid_on_release
                        .set(Some(SERVICE_CID));
                } else {
                    nodes.nodes[changed_node]
                        .uid_on_release
                        .set(Some(API_CLASS));
                }
                assert!(
                    route.guard(&ctx).is_err(),
                    "reflection release changed slot/preset UID after cached identity read"
                );
                drop(route);
                drop(ctx);
                drop(nodes);
            }
        }
    }
}

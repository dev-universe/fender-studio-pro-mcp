//! Exact primary Inserts-folder ownership, separate from Device Context.
use super::*;
use fender_native_node_api::{Inspection, NodeInspector, TextApi};
use std::{
    marker::PhantomData,
    rc::Rc,
    thread::{self, ThreadId},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(super) struct FolderIdentity {
    pub(super) id: String,
    pub(super) native_class: String,
    pub(super) object_uid: String,
    pub(super) class_uid: Option<String>,
}

// These addresses are comparison keys only. An Rc<FolderContext> must retain
// the corresponding native owners for every registry lifetime using this key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FolderBindingKey {
    service: usize,
    document: usize,
    model: usize,
    environment: [usize; 3], // manager, ActiveDocument controller, Environment
    folder: usize,
    channel: usize,
    folder_identity: FolderIdentity,
    channel_identity: FolderIdentity,
}
impl FolderBindingKey {
    pub(super) fn current_document(&self, ctx: &Context<'_>) -> bool {
        self.service == ctx.service as usize
            && self.document == ctx.document._identity.0 as usize
            && self.model == ctx.document._model_identity.0 as usize
    }
    pub(super) fn same_document(&self, other: &Self) -> bool {
        self.service == other.service
            && self.document == other.document
            && self.model == other.model
            && self.environment == other.environment
    }
}

struct Pin {
    node: Owned,
    canonical: Owned,
    returned: *mut c_void,
}
fn finish<T, G: FnMut() -> Result<(), String>>(
    result: Result<T, String>,
    guard: &mut G,
) -> Result<T, String> {
    if let Err(error) = guard() {
        drop(result);
        guard()?;
        return Err(error);
    }
    result
}
fn query_guarded<G: FnMut() -> Result<(), String>>(
    source: &Owned,
    iid: &Uid,
    guard: &mut G,
) -> Result<Owned, String> {
    guard()?;
    let result =
        query(source.0, iid).ok_or_else(|| "folder binding interface unavailable".to_string());
    finish(result, guard)
}
impl Pin {
    fn owned<G: FnMut() -> Result<(), String>>(
        source: Owned,
        guard: &mut G,
    ) -> Result<Self, String> {
        let result = (|| {
            let original = query_guarded(&source, &UNKNOWN, guard)?;
            let node = query_guarded(&source, &NODE, guard)?;
            let canonical = query_guarded(&node, &UNKNOWN, guard)?;
            if original.0 != canonical.0 {
                return Err("folder node canonical identity mismatch".into());
            }
            let returned = source.0;
            drop(original);
            drop(source);
            guard()?;
            Ok(Self {
                node,
                canonical,
                returned,
            })
        })();
        finish(result, guard)
    }
    fn parent<G: FnMut() -> Result<(), String>>(
        &self,
        guard: &mut G,
    ) -> Result<Option<Self>, String> {
        guard()?;
        let get: Borrowed = unsafe { method(self.node.0, 6) };
        let raw = unsafe { get(self.node.0) };
        let owner = hold(raw); // Pin a borrowed result before any callback guard.
        let result = (|| {
            guard()?;
            owner.map(|owner| Self::owned(owner, guard)).transpose()
        })();
        finish(result, guard)
    }
    fn metadata<G: FnMut() -> Result<(), String>>(
        &self,
        guard: &mut G,
    ) -> Result<FolderIdentity, String> {
        let result = (|| {
            guard()?;
            let get_uid: unsafe extern "C" fn(*mut c_void) -> *const Uid =
                unsafe { method(self.node.0, 4) };
            let object_uid = uid(unsafe { get_uid(self.node.0) });
            guard()?;
            let get_class: unsafe extern "C" fn(*mut c_void) -> *const Uid =
                unsafe { method(self.node.0, 5) };
            let class_uid = uid(unsafe { get_class(self.node.0) });
            guard()?;
            let class_uid = if class_uid.is_empty() || class_uid == ZERO_UID {
                None
            } else {
                if !valid_uid(&class_uid) {
                    return Err("folder binding class UID unavailable".into());
                }
                Some(class_uid)
            };
            let get_id: unsafe extern "C" fn(*mut c_void) -> *const PlainString =
                unsafe { method(self.node.0, 3) };
            let id = unsafe { get_id(self.node.0) };
            let owner = if id.is_null() {
                None
            } else {
                hold(unsafe { (*id).object })
            };
            guard()?;
            let owner = owner.ok_or("folder binding identifier unavailable")?;
            let id = copy_string(owner.0, 512)?;
            drop(owner);
            guard()?;
            if id.contains('\0') {
                return Err("folder binding identifier contains NUL".into());
            }
            let native_class = class_name(self.node.0);
            guard()?;
            Ok(FolderIdentity {
                id,
                native_class,
                object_uid,
                class_uid,
            })
        })();
        finish(result, guard)
    }
}
fn validate_folder(identity: &FolderIdentity) -> Result<(), String> {
    if identity.id == "Inserts"
        && identity.native_class == "AudioInsertFolder"
        && valid_uid(&identity.object_uid)
    {
        Ok(())
    } else {
        Err(
            "primary folder requires exact Inserts/AudioInsertFolder with a stable object UID"
                .into(),
        )
    }
}
fn validate_channel(identity: &FolderIdentity) -> Result<(), String> {
    if identity.native_class == "AudioTrackChannel" && valid_uid(&identity.object_uid) {
        Ok(())
    } else {
        Err("Inserts parent requires AudioTrackChannel with a stable object UID".into())
    }
}

/// Request-scoped deadline, retained native binding. A capsule may keep this in
/// Rc solely as its native-pin owner; later operations must guard with a fresh
/// request Context, compare the pure binding keys, and never check an old clock.
pub(super) struct FolderContext<'a> {
    chain: Vec<Pin>,
    ancestry: Inspection,
    environment: node_inspect::DocumentEnvironmentRoute,
    inserts_name: PlainString,
    _inserts_text: Owned,
    document: Document,
    service: *mut Service,
    api: &'a Api,
    key: FolderBindingKey,
    started: Instant,
    thread: ThreadId,
    _local: PhantomData<Rc<()>>,
}
impl<'a> FolderContext<'a> {
    pub(super) fn capture(
        service: *mut Service,
        target: Owned,
        expected: Option<&str>,
        api: &'a Api,
    ) -> Result<Self, String> {
        let started = Instant::now();
        check_service_active(service)?;
        if !on_main(api) {
            return Err("folder request left the host GUI thread".into());
        }
        let document = active_document(api)?;
        let get = constant_string().ok_or("folder text API unavailable")?;
        Self::capture_document(service, target, expected, api, document, started, get)
    }
    fn capture_document(
        service: *mut Service,
        target: Owned,
        expected: Option<&str>,
        api: &'a Api,
        document: Document,
        started: Instant,
        get: ConstantString,
    ) -> Result<Self, String> {
        let mut guard = || check_document_only(service, &document, started, api);
        let result = (|| {
            guard()?;
            let folder = Pin::owned(target, &mut guard)?;
            let folder_identity = folder.metadata(&mut guard)?;
            validate_folder(&folder_identity)?;
            if expected.is_some_and(|uid| uid != folder_identity.object_uid) {
                return Err("expected_folder does not match native Inserts".into());
            }
            let mut chain = vec![folder];
            loop {
                let parent = chain.last().expect("root").parent(&mut guard)?;
                let Some(parent) = parent else { break };
                if chain.len() == 9 {
                    return Err("folder parent path exceeds eight ancestors".into());
                }
                if chain
                    .iter()
                    .any(|pin| pin.canonical.0 == parent.canonical.0)
                {
                    return Err("folder parent path contains a cycle".into());
                }
                chain.push(parent);
            }
            let channel = chain.get(1).ok_or("Inserts has no parent channel")?;
            let channel_identity = channel.metadata(&mut guard)?;
            validate_channel(&channel_identity)?;
            let name = unsafe { get(c"Inserts".as_ptr()) };
            let (inserts_name, inserts_text) = if name.is_null() {
                return Err("fixed Inserts identifier unavailable".into());
            } else {
                let name = unsafe { *name };
                let owner = hold(name.object);
                guard()?;
                (name, owner.ok_or("fixed Inserts text unavailable")?)
            };
            let owner = hold(chain[0].node.0).expect("retained folder");
            guard()?;
            let pointer = NonNull::new(owner.0).expect("owned folder");
            std::mem::forget(owner);
            let inspector = unsafe {
                NodeInspector::from_owned(
                    pointer,
                    TextApi {
                        get_constant_string: std::mem::transmute::<
                            ConstantString,
                            unsafe extern "C" fn(*const c_char) -> *const c_void,
                        >(get),
                    },
                )
            };
            let ancestry = inspector.inspect_ancestors(&mut guard);
            drop(inspector);
            guard()?;
            let ancestry = ancestry.map_err(|error| error.to_string())?;
            if !ancestry.report().complete || !ancestry.report().diagnostics.is_empty() {
                return Err("folder ancestry is incomplete".into());
            }
            let environment = node_inspect::DocumentEnvironmentRoute::capture_with_text(
                &document.manager,
                get,
                &mut guard,
            )?;
            if !environment.matches(&ancestry)? {
                return Err("Inserts does not belong to the active document Environment".into());
            }
            let key = FolderBindingKey {
                service: service as usize,
                document: document._identity.0 as usize,
                model: document._model_identity.0 as usize,
                environment: environment.canonical_keys(),
                folder: chain[0].canonical.0 as usize,
                channel: chain[1].canonical.0 as usize,
                folder_identity,
                channel_identity,
            };
            Ok((
                chain,
                ancestry,
                environment,
                inserts_name,
                inserts_text,
                key,
            ))
        })();
        let result = finish(result, &mut guard)?;
        let ctx = Self {
            chain: result.0,
            ancestry: result.1,
            environment: result.2,
            inserts_name: result.3,
            _inserts_text: result.4,
            key: result.5,
            document,
            service,
            api,
            started,
            thread: thread::current().id(),
            _local: PhantomData,
        };
        if let Err(error) = ctx.check() {
            drop(ctx);
            return Err(error);
        }
        Ok(ctx)
    }
    pub(super) fn identity(&self) -> &FolderIdentity {
        &self.key.folder_identity
    }
    pub(super) fn binding_key(&self) -> FolderBindingKey {
        self.key.clone()
    }
    pub(super) fn check_document(&self) -> Result<(), String> {
        if thread::current().id() != self.thread {
            return Err("folder request left its creator thread".into());
        }
        check_document_only(self.service, &self.document, self.started, self.api)
    }
    pub(super) fn owned_folder(&self) -> Result<Owned, String> {
        self.check()?;
        let owner =
            hold(self.chain[0].node.0).ok_or_else(|| "retained Inserts unavailable".to_string());
        if let Err(error) = self.check() {
            drop(owner);
            self.check_document()?;
            return Err(error);
        }
        owner
    }
    pub(super) fn check(&self) -> Result<(), String> {
        self.check_document()?;
        self.ancestry
            .check(&mut || self.check_document())
            .map_err(|error| error.to_string())?;
        if self.chain[0].metadata(&mut || self.check_document())? != self.key.folder_identity
            || self.chain[1].metadata(&mut || self.check_document())? != self.key.channel_identity
        {
            return Err("retained Inserts or channel identity changed".into());
        }
        self.environment
            .check_family(&mut || self.check_document())?;
        self.check_borrowed_attachments()
    }
    pub(super) fn check_borrowed_attachments(&self) -> Result<(), String> {
        self.check_document()?;
        self.environment.check()?;
        for (index, pin) in self.chain.iter().enumerate() {
            let get: Borrowed = unsafe { method(pin.node.0, 6) };
            let expected = self
                .chain
                .get(index + 1)
                .map_or(ptr::null_mut(), |next| next.returned);
            if unsafe { get(pin.node.0) } != expected {
                return Err("retained Inserts parent path changed".into());
            }
        }
        let find: unsafe extern "C" fn(*mut c_void, *const PlainString) -> *mut c_void =
            unsafe { method(self.chain[1].node.0, 10) };
        if unsafe { find(self.chain[1].node.0, &self.inserts_name) } != self.chain[0].node.0 {
            return Err("channel reverse Inserts attachment changed".into());
        }
        self.check_document()
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    type Log = Rc<RefCell<Vec<(&'static str, &'static str)>>>;
    #[derive(Clone, Copy)]
    enum Hook {
        Parent(*mut Mock),
        Child(*mut Mock),
        Terminate(*mut Service),
        Attach(*mut Mock, *mut c_void),
        DetachProvider(*const Cell<bool>),
    }
    #[repr(C)]
    struct Mock {
        table: *const *const c_void,
        refs: Cell<i32>,
        canonical: Cell<*mut c_void>,
        interfaces: RefCell<Vec<(Uid, *mut c_void)>>,
        fault: Cell<Option<(i32, *mut c_void)>>,
        parent: Cell<*mut c_void>,
        child: Cell<*mut c_void>,
        active: Cell<*mut c_void>,
        children: RefCell<Vec<(*mut c_void, *mut c_void)>>,
        object_uid: Cell<Uid>,
        class_uid: Cell<Uid>,
        id: PlainString,
        text: Vec<u16>,
        info: Cell<*mut c_void>,
        family: CString,
        hook: Cell<Option<(&'static str, Hook)>>,
        label: &'static str,
        log: Log,
    }
    fn event(m: &Mock, op: &'static str) {
        m.log.borrow_mut().push((m.label, op));
        if let Some((selected, hook)) = m.hook.get() {
            if selected == op {
                m.hook.set(None);
                unsafe {
                    match hook {
                        Hook::Parent(p) => (*p).parent.set(ptr::null_mut()),
                        Hook::Child(p) => (*p).child.set(ptr::null_mut()),
                        Hook::Terminate(p) => (*p).terminated.store(true, Ordering::Release),
                        Hook::Attach(p, slot) => (*p).child.set(slot),
                        Hook::DetachProvider(valid) => (*valid).set(false),
                    }
                }
            }
        }
    }
    unsafe fn mock<'a>(p: *mut c_void) -> &'a Mock {
        unsafe { &*p.cast::<Mock>() }
    }
    unsafe extern "C" fn retain(p: *mut c_void) -> u32 {
        let m = unsafe { mock(p) };
        event(m, "retain");
        assert!(m.refs.get() > 0);
        m.refs.set(m.refs.get() + 1);
        m.refs.get() as u32
    }
    unsafe extern "C" fn release(p: *mut c_void) -> u32 {
        let m = unsafe { mock(p) };
        event(m, "release");
        assert!(m.refs.get() > 0);
        m.refs.set(m.refs.get() - 1);
        m.refs.get() as u32
    }
    unsafe extern "C" fn qi(p: *mut c_void, iid: *const Uid, out: *mut *mut c_void) -> i32 {
        let m = unsafe { mock(p) };
        event(m, "query");
        unsafe { *out = ptr::null_mut() };
        if let Some((status, value)) = m.fault.get() {
            unsafe { *out = value };
            return status;
        }
        let found = if unsafe { *iid } == UNKNOWN {
            Some(m.canonical.get())
        } else {
            m.interfaces
                .borrow()
                .iter()
                .find(|(key, _)| *key == unsafe { *iid })
                .map(|(_, p)| *p)
        };
        if let Some(value) = found {
            unsafe {
                retain(value);
                *out = value
            };
            OK
        } else {
            NO_INTERFACE
        }
    }
    unsafe extern "C" fn id(p: *mut c_void) -> *const PlainString {
        let m = unsafe { mock(p) };
        event(m, "id");
        &m.id
    }
    unsafe extern "C" fn object_uid(p: *mut c_void) -> *const Uid {
        let m = unsafe { mock(p) };
        event(m, "uid");
        m.object_uid.as_ptr()
    }
    unsafe extern "C" fn class_uid(p: *mut c_void) -> *const Uid {
        let m = unsafe { mock(p) };
        event(m, "class");
        m.class_uid.as_ptr()
    }
    unsafe extern "C" fn parent(p: *mut c_void) -> *mut c_void {
        let m = unsafe { mock(p) };
        event(m, "parent");
        m.parent.get()
    }
    unsafe extern "C" fn active(p: *mut c_void) -> *mut c_void {
        let m = unsafe { mock(p) };
        event(m, "active");
        m.active.get()
    }
    unsafe extern "C" fn child(p: *mut c_void, key: *const PlainString) -> *mut c_void {
        let m = unsafe { mock(p) };
        event(m, "child");
        if m.label == "graph-junction" {
            SLOT_READS.with(|reads| reads.set(reads.get() + 1));
        }
        let key = if key.is_null() {
            ptr::null_mut()
        } else {
            unsafe { (*key).object }
        };
        if let Some((_, value)) = m.children.borrow().iter().find(|(name, _)| *name == key) {
            *value
        } else {
            m.child.get()
        }
    }
    unsafe extern "C" fn zero(_: *mut c_void) -> i32 {
        0
    }
    unsafe extern "C" fn null(_: *mut c_void) -> *mut c_void {
        ptr::null_mut()
    }
    unsafe extern "C" fn typename(p: *mut c_void) -> *const c_char {
        let m = unsafe { mock(p) };
        event(m, "typename");
        m.family.as_ptr()
    }
    unsafe extern "C" fn typeinfo(p: *mut c_void) -> *mut c_void {
        let m = unsafe { mock(p) };
        event(m, "typeinfo");
        m.info.get()
    }
    unsafe extern "C" fn length(p: *mut c_void) -> i32 {
        let m = unsafe { mock(p) };
        event(m, "length");
        m.text.len() as i32
    }
    unsafe extern "C" fn copy(p: *mut c_void, out: *mut u16, n: i32) -> i32 {
        let m = unsafe { mock(p) };
        event(m, "copy");
        assert!(n as usize > m.text.len());
        unsafe {
            ptr::copy_nonoverlapping(m.text.as_ptr(), out, m.text.len());
            *out.add(m.text.len()) = 0
        };
        OK
    }
    static LOOKUP: AtomicPtr<Names> = AtomicPtr::new(ptr::null_mut());
    struct Names {
        inserts: PlainString,
        active: PlainString,
        environment: PlainString,
    }
    unsafe extern "C" fn text(key: *const c_char) -> *const PlainString {
        let names = LOOKUP.load(Ordering::Acquire);
        assert!(!names.is_null());
        let names = unsafe { &*names };
        match unsafe { std::ffi::CStr::from_ptr(key) }.to_bytes() {
            b"Inserts" | b"Slot" => &names.inserts,
            b"ActiveDocument" => &names.active,
            b"Environment" => &names.environment,
            _ => ptr::null(),
        }
    }
    thread_local! {
        static SLOT_READS: Cell<u32> = const { Cell::new(0) };
        static PROVIDER_CLEANUP: Cell<Option<(*mut c_void, u32)>> = const { Cell::new(None) };
    }
    pub(in super::super) fn provider_cleanup_hook() {
        PROVIDER_CLEANUP.with(|hook| {
            if let Some((owner, threshold)) = hook.get() {
                if SLOT_READS.with(Cell::get) >= threshold {
                    hook.set(None);
                    drop(query(owner, &UNKNOWN).unwrap());
                }
            }
        });
    }
    unsafe extern "C" fn graph_text(key: *const c_char) -> *const c_void {
        unsafe { text(key) }.cast()
    }
    unsafe extern "C" fn one(_: *mut c_void) -> i32 {
        1
    }
    unsafe extern "C" fn graph_child(p: *mut c_void, index: i32) -> *mut c_void {
        assert_eq!(index, 0);
        unsafe { mock(p) }.child.get()
    }
    pub(in super::super) struct Fixture {
        _service: guard_tests::Fixture,
        service: *mut Service,
        objects: Vec<std::pin::Pin<Box<Mock>>>,
        tables: Vec<std::pin::Pin<Box<[*const c_void; 11]>>>,
        names: Box<Names>,
        log: Log,
        folder: *mut c_void,
        channel: *mut c_void,
        environment: *mut c_void,
        controller: *mut c_void,
        manager: *mut c_void,
        document: *mut c_void,
        model: *mut c_void,
        old_device: *mut c_void,
        other: *mut c_void,
    }
    impl Fixture {
        pub(in super::super) fn new() -> Self {
            let mut service = guard_tests::Fixture::new();
            let ctx = service.context();
            let pointer = ctx.service;
            drop(ctx);
            let mut f = Self {
                _service: service,
                service: pointer,
                objects: vec![],
                tables: vec![],
                names: Box::new(Names {
                    inserts: PlainString {
                        object: ptr::null_mut(),
                        opaque_debug: ptr::null_mut(),
                    },
                    active: PlainString {
                        object: ptr::null_mut(),
                        opaque_debug: ptr::null_mut(),
                    },
                    environment: PlainString {
                        object: ptr::null_mut(),
                        opaque_debug: ptr::null_mut(),
                    },
                }),
                log: Rc::new(RefCell::new(vec![])),
                folder: ptr::null_mut(),
                channel: ptr::null_mut(),
                environment: ptr::null_mut(),
                controller: ptr::null_mut(),
                manager: ptr::null_mut(),
                document: ptr::null_mut(),
                model: ptr::null_mut(),
                old_device: ptr::null_mut(),
                other: ptr::null_mut(),
            };
            f.names.inserts.object = f.string("Inserts");
            f.names.active.object = f.string("ActiveDocument");
            f.names.environment.object = f.string("Environment");
            LOOKUP.store(ptr::addr_of_mut!(*f.names), Ordering::Release);
            f.folder = f.node("folder", "Inserts", "AudioInsertFolder");
            f.channel = f.node("channel", "track", "AudioTrackChannel");
            f.environment = f.node(
                "environment",
                "SongEnvironment",
                "UltimateStudioEnvironment",
            );
            f.controller = f.node("controller", "controller", "SongController");
            f.manager = f.node("manager", "manager", "DocumentManager");
            f.document = f.node("document", "document", "Document");
            f.model = f.node("model", "model", "Song");
            f.old_device = f.node("old-device", "Device", "AudioEffect");
            f.other = f.node("other", "SongEnvironment", "UltimateStudioEnvironment");
            unsafe {
                mock(f.folder).parent.set(f.channel);
                mock(f.channel).parent.set(f.environment);
                mock(f.channel).child.set(f.folder);
                mock(f.manager)
                    .children
                    .borrow_mut()
                    .push((f.names.active.object, f.controller));
                mock(f.controller)
                    .children
                    .borrow_mut()
                    .push((f.names.environment.object, f.environment));
                mock(f.manager).active.set(f.document);
                mock(f.document).parent.set(f.model);
            }
            f
        }
        fn object(
            &mut self,
            label: &'static str,
            methods: &[(usize, *const c_void)],
            value: &str,
        ) -> *mut c_void {
            let mut table = Box::pin([ptr::null(); 11]);
            table[0] = qi as *const c_void;
            table[1] = retain as *const c_void;
            table[2] = release as *const c_void;
            for &(slot, function) in methods {
                table[slot] = function;
            }
            let mut m = Box::pin(Mock {
                table: table.as_ptr(),
                refs: Cell::new(1),
                canonical: Cell::new(ptr::null_mut()),
                interfaces: RefCell::new(vec![]),
                fault: Cell::new(None),
                parent: Cell::new(ptr::null_mut()),
                child: Cell::new(ptr::null_mut()),
                active: Cell::new(ptr::null_mut()),
                children: RefCell::new(vec![]),
                object_uid: Cell::new(SERVICE_CID),
                class_uid: Cell::new(Uid {
                    a: 0,
                    b: 0,
                    c: 0,
                    d: [0; 8],
                }),
                id: PlainString {
                    object: ptr::null_mut(),
                    opaque_debug: ptr::null_mut(),
                },
                text: value.encode_utf16().collect(),
                info: Cell::new(ptr::null_mut()),
                family: CString::new(value).unwrap(),
                hook: Cell::new(None),
                label,
                log: self.log.clone(),
            });
            let p = ptr::addr_of_mut!(*m).cast();
            m.canonical.set(p);
            self.objects.push(m);
            self.tables.push(table);
            p
        }
        fn string(&mut self, value: &str) -> *mut c_void {
            self.object(
                "text",
                &[(4, length as *const c_void), (8, copy as *const c_void)],
                value,
            )
        }
        fn node(&mut self, label: &'static str, value: &str, family: &str) -> *mut c_void {
            let string = self.string(value);
            let p = self.object(
                label,
                &[
                    (3, id as *const c_void),
                    (4, object_uid as *const c_void),
                    (5, class_uid as *const c_void),
                    (6, parent as *const c_void),
                    (8, zero as *const c_void),
                    (9, active as *const c_void),
                    (10, child as *const c_void),
                ],
                "",
            );
            let reflection = self.object("object", &[(3, typeinfo as *const c_void)], "");
            let info = self.object(
                "type",
                &[
                    (3, parent as *const c_void),
                    (4, zero as *const c_void),
                    (5, typename as *const c_void),
                    (6, null as *const c_void),
                    (7, class_uid as *const c_void),
                    (8, null as *const c_void),
                    (9, null as *const c_void),
                ],
                family,
            );
            unsafe { mock(reflection) }.info.set(info);
            let m = unsafe { &mut *p.cast::<Mock>() };
            m.id.object = string;
            m.interfaces
                .borrow_mut()
                .extend([(NODE, p), (OBJECT, reflection)]);
            let mut uid = SERVICE_CID;
            uid.d[7] = self.objects.len() as u8;
            m.object_uid.set(uid);
            p
        }
        fn document(&self) -> Document {
            Document {
                manager: hold(self.manager).unwrap(),
                document: hold(self.document).unwrap(),
                _identity: hold(self.document).unwrap(),
                model: hold(self.model).unwrap(),
                _model_identity: hold(self.model).unwrap(),
            }
        }
        pub(in super::super) fn capture(&self) -> Result<FolderContext<'static>, String> {
            FolderContext::capture_document(
                self.service(),
                hold(self.folder).unwrap(),
                None,
                self.api(),
                self.document(),
                Instant::now(),
                text,
            )
        }
        pub(in super::super) fn service(&self) -> *mut Service {
            self.service
        }
        pub(in super::super) fn api(&self) -> &'static Api {
            api().unwrap()
        }
        pub(in super::super) fn graph_context(&self) -> Box<Context<'static>> {
            let target = unsafe { mock(self.old_device) };
            Box::new(Context {
                service: self.service,
                target: hold(self.old_device).unwrap(),
                node: hold(self.old_device).unwrap(),
                instance: Instance {
                    object_uid: uid(target.object_uid.as_ptr()),
                    class_uid: uid(target.class_uid.as_ptr()),
                    native_class: "AudioEffect".into(),
                    object_id: "Device".into(),
                },
                document: self.document(),
                started: Instant::now(),
                api: self.api(),
            })
        }
        pub(in super::super) fn focused_graph(
            &mut self,
        ) -> (fender_native_node_api::Inspection, *mut c_void) {
            let root = self.node("graph", "Graph", "AudioFXGraph");
            let junction = self.node("graph-junction", "splitter", "AudioFXGraph::SplitterNode");
            unsafe { mock(root) }.child.set(junction);
            let table = self
                .objects
                .iter()
                .position(|m| ptr::eq(&**m as *const Mock as *const c_void, root))
                .unwrap();
            self.tables[table][8] = one as *const c_void;
            self.tables[table][9] = graph_child as *const c_void;
            let owner = hold(root).unwrap();
            let pointer = NonNull::new(owner.0).unwrap();
            std::mem::forget(owner);
            let inspector = unsafe {
                fender_native_node_api::NodeInspector::from_owned(
                    pointer,
                    fender_native_node_api::TextApi {
                        get_constant_string: graph_text,
                    },
                )
            };
            let focused = inspector
                .inspect_children_with_slots(&mut || Ok(()))
                .unwrap();
            drop(inspector);
            (focused, junction)
        }
        pub(in super::super) fn reset_slot_reads(&self) {
            SLOT_READS.with(|reads| reads.set(0));
        }
        pub(in super::super) fn slot_reads(&self) -> u32 {
            SLOT_READS.with(Cell::get)
        }
        pub(in super::super) fn cleanup_adds_slot_after(&self, junction: *mut c_void, reads: u32) {
            unsafe { mock(self.other) }.hook.set(Some((
                "release",
                Hook::Attach(junction.cast(), self.old_device),
            )));
            PROVIDER_CLEANUP.with(|hook| hook.set(Some((self.other, reads))));
        }
        pub(in super::super) fn slot_absent(&self, junction: *mut c_void) -> bool {
            unsafe { mock(junction) }.child.get().is_null()
        }
        pub(in super::super) fn channel_cleanup_detaches_provider(&self, valid: &Cell<bool>) {
            let channel = unsafe { mock(self.channel) };
            let object = channel
                .interfaces
                .borrow()
                .iter()
                .find(|(iid, _)| *iid == OBJECT)
                .unwrap()
                .1;
            unsafe { mock(object) }
                .hook
                .set(Some(("release", Hook::DetachProvider(valid))));
        }
        pub(in super::super) fn switch_document(&self) {
            unsafe { mock(self.manager) }.active.set(ptr::null_mut())
        }
        pub(in super::super) fn switch_model(&self) {
            unsafe { mock(self.document) }.parent.set(ptr::null_mut())
        }
        pub(in super::super) fn replace_folder(&self) {
            unsafe { mock(self.channel) }.child.set(self.other)
        }
        fn balanced(&self) {
            for m in &self.objects {
                assert_eq!(m.refs.get(), 1, "{} native owners", m.label)
            }
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            LOOKUP.store(ptr::null_mut(), Ordering::Release);
            PROVIDER_CLEANUP.with(|hook| hook.set(None));
            self.balanced();
        }
    }

    #[test]
    fn primary_folder_accepts_missing_class_without_a_device_and_returns_owned_exact_node() {
        let _lock = super::super::super::tests::TEST_LOCK.lock().unwrap();
        let f = Fixture::new();
        let ctx = f.capture().unwrap();
        assert!(ctx.identity().class_uid.is_none());
        assert_eq!(ctx.identity().id, "Inserts");
        let owner = ctx.owned_folder().unwrap();
        assert_eq!(owner.0, ctx.chain[0].node.0);
        drop(owner);
        ctx.check().unwrap();
        drop(ctx);
        f.balanced();
    }
    #[test]
    fn present_folder_class_is_copied_and_must_not_change() {
        let _lock = super::super::super::tests::TEST_LOCK.lock().unwrap();
        let f = Fixture::new();
        unsafe { mock(f.folder) }.class_uid.set(API_CLASS);
        let ctx = f.capture().unwrap();
        assert_eq!(
            ctx.identity().class_uid.as_deref(),
            Some(uid(&API_CLASS).as_str())
        );
        unsafe { mock(f.folder) }.class_uid.set(SERVICE_CID);
        assert!(ctx.check().is_err());
        drop(ctx);
        f.balanced();
    }
    #[test]
    fn fixed_folder_and_channel_family_and_nonzero_uids_are_required() {
        let _lock = super::super::super::tests::TEST_LOCK.lock().unwrap();
        for which in 0..4 {
            let f = Fixture::new();
            let node = if which < 2 { f.folder } else { f.channel };
            if which % 2 == 0 {
                unsafe { mock(node) }.object_uid.set(Uid {
                    a: 0,
                    b: 0,
                    c: 0,
                    d: [0; 8],
                })
            } else {
                let reflection = unsafe { mock(node) }
                    .interfaces
                    .borrow()
                    .iter()
                    .find(|(iid, _)| *iid == OBJECT)
                    .unwrap()
                    .1;
                let info = unsafe { mock(reflection) }.info.get();
                unsafe { &mut *info.cast::<Mock>() }.family = CString::new("WrongFamily").unwrap();
            }
            assert!(f.capture().is_err(), "case {which}");
            f.balanced();
        }
    }
    #[test]
    fn foreign_same_named_environment_and_cycles_never_bind() {
        let _lock = super::super::super::tests::TEST_LOCK.lock().unwrap();
        let f = Fixture::new();
        unsafe { mock(f.controller) }.children.borrow_mut()[0].1 = f.other;
        assert!(f.capture().is_err());
        f.balanced();
        drop(f);
        let f = Fixture::new();
        unsafe { mock(f.environment) }.parent.set(f.channel);
        assert!(f.capture().is_err());
        f.balanced();
    }
    #[test]
    fn changed_parent_reverse_controller_environment_document_or_model_is_rejected() {
        let _lock = super::super::super::tests::TEST_LOCK.lock().unwrap();
        for which in 0..6 {
            let f = Fixture::new();
            let ctx = f.capture().unwrap();
            match which {
                0 => unsafe { mock(f.folder) }.parent.set(f.other),
                1 => f.replace_folder(),
                2 => unsafe { mock(f.manager) }.children.borrow_mut()[0].1 = f.other,
                3 => unsafe { mock(f.controller) }.children.borrow_mut()[0].1 = f.other,
                4 => f.switch_document(),
                _ => f.switch_model(),
            }
            assert!(ctx.check().is_err(), "case {which}");
            drop(ctx);
            f.balanced();
        }
    }
    #[test]
    fn old_device_replacement_is_legal_and_never_read_by_folder_guard() {
        let _lock = super::super::super::tests::TEST_LOCK.lock().unwrap();
        let f = Fixture::new();
        let ctx = f.capture().unwrap();
        f.log.borrow_mut().clear();
        unsafe { mock(f.old_device) }.object_uid.set(UNKNOWN);
        unsafe { mock(f.old_device) }.class_uid.set(UNKNOWN);
        unsafe { mock(f.old_device) }.parent.set(f.other);
        ctx.check().unwrap();
        assert!(
            !f.log
                .borrow()
                .iter()
                .any(|(label, _)| *label == "old-device")
        );
        drop(ctx);
        f.balanced();
    }
    #[test]
    fn binding_key_is_pure_but_pinned_and_compares_separate_model_controller_environment() {
        let _lock = super::super::super::tests::TEST_LOCK.lock().unwrap();
        let f = Fixture::new();
        let ctx = f.capture().unwrap();
        let next = f.capture().unwrap();
        f.log.borrow_mut().clear();
        let key = ctx.binding_key();
        assert_eq!(key, next.binding_key());
        assert!(key.same_document(&next.binding_key()));
        assert!(f.log.borrow().is_empty());
        assert_ne!(key.model, key.environment[1]);
        drop(next);
        drop(ctx);
        f.balanced();
    }
    #[test]
    fn metadata_and_temporary_release_reentry_precedes_final_edges() {
        let _lock = super::super::super::tests::TEST_LOCK.lock().unwrap();
        for release_hook in [false, true] {
            let f = Fixture::new();
            let ctx = f.capture().unwrap();
            if release_hook {
                let text = unsafe { mock(f.folder) }.id.object;
                unsafe { mock(text) }
                    .hook
                    .set(Some(("release", Hook::Parent(f.folder.cast()))));
            } else {
                unsafe { mock(f.folder) }
                    .hook
                    .set(Some(("class", Hook::Child(f.channel.cast()))));
            }
            assert!(ctx.check().is_err());
            drop(ctx);
            f.balanced();
        }
    }
    #[test]
    fn service_loss_wrong_thread_and_old_budget_do_no_follow_on_foreign_work() {
        let _lock = super::super::super::tests::TEST_LOCK.lock().unwrap();
        let f = Fixture::new();
        let mut ctx = f.capture().unwrap();
        let current = ctx.thread;
        ctx.thread = std::thread::spawn(|| thread::current().id())
            .join()
            .unwrap();
        f.log.borrow_mut().clear();
        assert!(ctx.check().is_err());
        assert!(f.log.borrow().is_empty());
        ctx.thread = current;
        ctx.started = Instant::now() - Duration::from_secs(3);
        assert!(ctx.check().is_err());
        assert!(f.log.borrow().is_empty());
        ctx.started = Instant::now();
        unsafe { mock(f.folder) }
            .hook
            .set(Some(("query", Hook::Terminate(f.service))));
        assert!(ctx.check().is_err());
        unsafe { &*f.service }
            .terminated
            .store(false, Ordering::Release);
        drop(ctx);
        f.balanced();
    }
    #[test]
    fn failed_nonnull_query_is_not_owned_or_dereferenced() {
        let _lock = super::super::super::tests::TEST_LOCK.lock().unwrap();
        let f = Fixture::new();
        unsafe { mock(f.folder) }
            .fault
            .set(Some((1, ptr::dangling_mut::<c_void>())));
        assert!(f.capture().is_err());
        f.balanced();
    }
    #[test]
    fn final_document_witness_is_after_all_metadata_and_borrowed_edges() {
        let _lock = super::super::super::tests::TEST_LOCK.lock().unwrap();
        let f = Fixture::new();
        let ctx = f.capture().unwrap();
        f.log.borrow_mut().clear();
        ctx.check().unwrap();
        let log = f.log.borrow();
        assert_eq!(
            &log[log.len() - 2..],
            &[("manager", "active"), ("document", "parent")]
        );
        let final_find = log
            .iter()
            .rposition(|row| *row == ("channel", "child"))
            .unwrap();
        assert!(
            log[final_find + 1..]
                .iter()
                .all(|(_, op)| matches!(*op, "active" | "parent"))
        );
        drop(log);
        drop(ctx);
        f.balanced();
    }
    #[test]
    fn overlong_ancestor_path_and_incomplete_reflection_are_closed_failures() {
        let _lock = super::super::super::tests::TEST_LOCK.lock().unwrap();
        let mut f = Fixture::new();
        let mut from = f.environment;
        for _ in 0..8 {
            let next = f.node("ancestor", "ancestor", "Ancestor");
            unsafe { mock(from) }.parent.set(next);
            from = next;
        }
        assert!(f.capture().err().unwrap().contains("eight ancestors"));
        f.balanced();
        drop(f);
        let f = Fixture::new();
        let reflection = unsafe { mock(f.folder) }
            .interfaces
            .borrow()
            .iter()
            .find(|(iid, _)| *iid == OBJECT)
            .unwrap()
            .1;
        let info = unsafe { mock(reflection) }.info.get();
        unsafe { mock(info) }.parent.set(info);
        assert!(f.capture().err().unwrap().contains("incomplete"));
        f.balanced();
    }
    #[test]
    fn primary_identifier_expected_uid_and_both_metadata_uids_are_not_optional() {
        let _lock = super::super::super::tests::TEST_LOCK.lock().unwrap();
        let f = Fixture::new();
        unsafe { &mut *f.folder.cast::<Mock>() }.id.object = f.names.active.object;
        assert!(f.capture().is_err());
        f.balanced();
        drop(f);
        let f = Fixture::new();
        let result = FolderContext::capture_document(
            f.service,
            hold(f.folder).unwrap(),
            Some(ZERO_UID),
            api().unwrap(),
            f.document(),
            Instant::now(),
            text,
        );
        assert!(result.is_err());
        drop(result);
        f.balanced();
        drop(f);
        for node in 0..2 {
            let f = Fixture::new();
            let ctx = f.capture().unwrap();
            unsafe { mock(if node == 0 { f.folder } else { f.channel }) }
                .object_uid
                .set(UNKNOWN);
            assert!(ctx.check().is_err());
            drop(ctx);
            f.balanced();
        }
    }
    #[test]
    fn legacy_expiry_key_reads_only_owned_document_and_model_keys() {
        let _lock = super::super::super::tests::TEST_LOCK.lock().unwrap();
        let f = Fixture::new();
        let folder = f.capture().unwrap();
        let mut legacy = Context {
            service: f.service,
            target: hold(f.old_device).unwrap(),
            node: hold(f.old_device).unwrap(),
            instance: Instance {
                object_uid: uid(&SERVICE_CID),
                class_uid: uid(&API_CLASS),
                native_class: "AudioEffect".into(),
                object_id: "Device".into(),
            },
            document: f.document(),
            started: Instant::now(),
            api: api().unwrap(),
        };
        f.log.borrow_mut().clear();
        assert!(folder.binding_key().current_document(&legacy));
        assert!(f.log.borrow().is_empty());
        let previous =
            std::mem::replace(&mut legacy.document._model_identity, hold(f.other).unwrap());
        assert!(!folder.binding_key().current_document(&legacy));
        drop(previous);
        drop(legacy);
        drop(folder);
        f.balanced();
    }
    #[test]
    fn native_pins_and_lookup_text_remain_owned_for_the_pure_key_lifetime() {
        let _lock = super::super::super::tests::TEST_LOCK.lock().unwrap();
        let f = Fixture::new();
        let saved = Rc::new(f.capture().unwrap());
        let key = saved.binding_key();
        let saved_again = saved.clone();
        drop(saved);
        for p in [
            f.folder,
            f.channel,
            f.environment,
            f.controller,
            f.manager,
            f.document,
            f.model,
        ] {
            assert!(unsafe { mock(p) }.refs.get() > 1);
        }
        let fresh = f.capture().unwrap();
        assert_eq!(key, fresh.binding_key());
        fresh.check().unwrap();
        drop(fresh);
        drop(saved_again);
        f.balanced();
    }
}

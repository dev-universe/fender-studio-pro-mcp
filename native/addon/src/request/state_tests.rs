use super::*;

#[repr(C)]
struct Mock {
    table: *const *const c_void,
    refs: AtomicU32,
    alias: *mut c_void,
    parent: *mut c_void,
    active: *mut c_void,
    uid: Uid,
    class: Uid,
    storable: bool,
    bytes: Mutex<Vec<u8>>,
    saves: AtomicU32,
    loads: AtomicU32,
    load_result: bool,
    stop_on_load: *mut Service,
}
unsafe extern "C" fn retain(p: *mut c_void) -> u32 {
    unsafe { &*p.cast::<Mock>() }
        .refs
        .fetch_add(1, Ordering::Relaxed)
        + 1
}
unsafe extern "C" fn release_mock(p: *mut c_void) -> u32 {
    unsafe { &*p.cast::<Mock>() }
        .refs
        .fetch_sub(1, Ordering::Relaxed)
        - 1
}
unsafe extern "C" fn query_mock(p: *mut c_void, iid: *const Uid, out: *mut *mut c_void) -> i32 {
    let mock = unsafe { &*p.cast::<Mock>() };
    unsafe { *out = ptr::null_mut() };
    if unsafe { *iid } == UNKNOWN
        || unsafe { *iid } == ALIAS && !mock.alias.is_null()
        || unsafe { *iid } == STORABLE && mock.storable
    {
        unsafe {
            retain(p);
            *out = p;
        }
        OK
    } else {
        NO_INTERFACE
    }
}
unsafe extern "C" fn alias(p: *mut c_void) -> *mut c_void {
    unsafe { (*p.cast::<Mock>()).alias }
}
unsafe extern "C" fn parent(p: *mut c_void) -> *mut c_void {
    unsafe { (*p.cast::<Mock>()).parent }
}
unsafe extern "C" fn active(p: *mut c_void) -> *mut c_void {
    unsafe { (*p.cast::<Mock>()).active }
}
unsafe extern "C" fn object_uid(p: *mut c_void) -> *const Uid {
    unsafe { ptr::addr_of!((*p.cast::<Mock>()).uid) }
}
unsafe extern "C" fn class_uid(p: *mut c_void) -> *const Uid {
    unsafe { ptr::addr_of!((*p.cast::<Mock>()).class) }
}
unsafe extern "C" fn save(p: *mut c_void, stream: *mut c_void) -> u8 {
    let mock = unsafe { &*p.cast::<Mock>() };
    mock.saves.fetch_add(1, Ordering::Relaxed);
    let bytes = mock.bytes.lock().unwrap().clone();
    let write: unsafe extern "C" fn(*mut c_void, *const c_void, i32) -> i32 =
        unsafe { method(stream, 4) };
    u8::from(
        unsafe { write(stream, bytes.as_ptr().cast(), bytes.len() as i32) } == bytes.len() as i32,
    )
}
unsafe extern "C" fn load(p: *mut c_void, stream: *mut c_void) -> u8 {
    let mock = unsafe { &*p.cast::<Mock>() };
    mock.loads.fetch_add(1, Ordering::Relaxed);
    let read: unsafe extern "C" fn(*mut c_void, *mut c_void, i32) -> i32 =
        unsafe { method(stream, 3) };
    let mut bytes = Vec::new();
    loop {
        let mut chunk = [0; 3];
        let count = unsafe { read(stream, chunk.as_mut_ptr().cast(), chunk.len() as i32) };
        if count < 0 {
            return 0;
        }
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..count as usize]);
    }
    *mock.bytes.lock().unwrap() = bytes;
    if !mock.stop_on_load.is_null() {
        unsafe {
            (*mock.stop_on_load)
                .terminated
                .store(true, Ordering::Release);
        }
    }
    u8::from(mock.load_result)
}
unsafe extern "C" fn empty() -> *mut c_void {
    ptr::null_mut()
}
unsafe extern "C" fn thread() -> i64 {
    0
}
unsafe extern "C" fn text(_: *const c_char) -> *const PlainCString {
    ptr::null()
}
struct Fixture {
    _table: Box<[*const c_void; 10]>,
    _plugin_table: Box<[*const c_void; 10]>,
    manager: Box<Mock>,
    document: Box<Mock>,
    model: Box<Mock>,
    device: Box<Mock>,
    parent: Box<Mock>,
    plugin: Box<Mock>,
    service: Box<Service>,
    api: Box<Api>,
}
fn service() -> Box<Service> {
    Box::new(Service {
        vtable: &SERVICE_VTABLE,
        refs: AtomicU32::new(1),
        initialized: AtomicBool::new(true),
        script: ScriptFacet::new(),
        registered: AtomicBool::new(true),
        terminated: AtomicBool::new(false),
        initializing: AtomicBool::new(false),
        stopping: AtomicBool::new(false),
        module_attached: AtomicBool::new(false),
        calls: AtomicU32::new(0),
        busy: AtomicBool::new(true),
        receipt: std::cell::UnsafeCell::new(None),
        receipt_pending: AtomicBool::new(false),
        document_context: std::cell::UnsafeCell::new(
            super::super::document_context::Registry::new(1),
        ),
        document_context_pending: AtomicBool::new(false),
        states: std::cell::UnsafeCell::new(Registry::new()),
        states_pending: AtomicBool::new(false),
        presets: std::cell::UnsafeCell::new(super::super::presets::Registry::new()),
        presets_pending: AtomicBool::new(false),
        insert_chains: std::cell::UnsafeCell::new(super::super::insert_chain::Registry::new()),
        insert_chains_pending: AtomicBool::new(false),
    })
}
impl Fixture {
    fn new() -> Self {
        let mut table = Box::new([ptr::null::<c_void>(); 10]);
        table[0] = query_mock as *const c_void;
        table[1] = retain as *const c_void;
        table[2] = release_mock as *const c_void;
        table[3] = alias as *const c_void;
        table[4] = object_uid as *const c_void;
        table[5] = class_uid as *const c_void;
        table[6] = parent as *const c_void;
        table[9] = active as *const c_void;
        let mut plugin_table = table.clone();
        plugin_table[4] = save as *const c_void;
        plugin_table[5] = load as *const c_void;
        let make = || {
            Box::new(Mock {
                table: table.as_ptr(),
                refs: AtomicU32::new(1),
                alias: ptr::null_mut(),
                parent: ptr::null_mut(),
                active: ptr::null_mut(),
                uid: SERVICE_CID,
                class: API_CLASS,
                storable: false,
                bytes: Mutex::new(b"state".to_vec()),
                saves: AtomicU32::new(0),
                loads: AtomicU32::new(0),
                load_result: true,
                stop_on_load: ptr::null_mut(),
            })
        };
        let (mut manager, mut document, mut model, mut device, mut parent, mut plugin) =
            (make(), make(), make(), make(), make(), make());
        manager.active = ptr::addr_of_mut!(*document).cast();
        document.parent = ptr::addr_of_mut!(*model).cast();
        device.alias = ptr::addr_of_mut!(*plugin).cast();
        device.parent = ptr::addr_of_mut!(*parent).cast();
        plugin.table = plugin_table.as_ptr();
        plugin.storable = true;
        let service = service();
        let api = Box::new(Api {
            gui: empty,
            objects: empty,
            main_thread: empty,
            thread_self: thread,
            cstring: text,
            empty,
            desktop: empty,
            scripting: empty,
            plugin_manager: empty,
            file_system: empty,
        });
        READY.store(true, Ordering::Release);
        Self {
            _table: table,
            _plugin_table: plugin_table,
            manager,
            document,
            model,
            device,
            parent,
            plugin,
            service,
            api,
        }
    }
    fn context(&mut self) -> Context<'_> {
        Context {
            service: ptr::addr_of_mut!(*self.service),
            target: hold(ptr::addr_of_mut!(*self.device).cast()).unwrap(),
            node: hold(ptr::addr_of_mut!(*self.device).cast()).unwrap(),
            instance: Instance {
                object_uid: uid(&self.device.uid),
                class_uid: uid(&self.device.class),
                native_class: "mock".into(),
                object_id: "mock".into(),
            },
            document: Document {
                manager: hold(ptr::addr_of_mut!(*self.manager).cast()).unwrap(),
                document: hold(ptr::addr_of_mut!(*self.document).cast()).unwrap(),
                _identity: hold(ptr::addr_of_mut!(*self.document).cast()).unwrap(),
                model: hold(ptr::addr_of_mut!(*self.model).cast()).unwrap(),
                _model_identity: hold(ptr::addr_of_mut!(*self.model).cast()).unwrap(),
            },
            started: Instant::now(),
            api: &self.api,
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        drain(ptr::addr_of_mut!(*self.service));
        for object in [
            &self.manager,
            &self.document,
            &self.model,
            &self.device,
            &self.parent,
            &self.plugin,
        ] {
            assert_eq!(object.refs.load(Ordering::Acquire), 1);
        }
        assert!(!self.service.states_pending.load(Ordering::Acquire));
        assert_eq!(live_count(), 0);
        READY.store(false, Ordering::Release);
    }
}
#[test]
fn saved_bytes_are_frozen_reusable_and_never_claim_semantic_verification() {
    let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut fixture = Fixture::new();
    let plugin = ptr::addr_of_mut!(*fixture.plugin);
    let ctx = fixture.context();
    let (saved, lease) = capture(&ctx).unwrap();
    assert_eq!(saved["byte_count"], 5);
    assert_eq!(
        saved["sha256"],
        "4ba69735ca53765ed6a709edb56c6ea236b7193a3b29a6b390c346f0f4340e4e"
    );
    assert_eq!(saved["verified"], false);
    let token = saved["token"].as_str().unwrap();
    assert!(valid_token(token));
    drop(lease);
    for _ in 0..2 {
        *unsafe { &*plugin }.bytes.lock().unwrap() = b"changed".to_vec();
        let (restored, lease) = restore(&ctx, token);
        assert_eq!(restored["load_accepted"], true);
        assert_eq!(restored["verified"], false);
        assert_eq!(restored["outcome"], "applied_unverified");
        assert_eq!(&*unsafe { &*plugin }.bytes.lock().unwrap(), b"state");
        drop(lease);
    }
    assert_eq!(release(&ctx, token).unwrap()["released"], true);
    assert_eq!(restore(&ctx, token).0["outcome"], "not_applied");
    drop(ctx);
}
#[test]
fn state_tokens_are_bound_to_device_and_expire_with_document_identity() {
    let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut fixture = Fixture::new();
    let device = ptr::addr_of_mut!(*fixture.device);
    let plugin = ptr::addr_of_mut!(*fixture.plugin);
    let mut ctx = fixture.context();
    let (saved, lease) = capture(&ctx).unwrap();
    drop(lease);
    let token = saved["token"].as_str().unwrap();
    unsafe {
        (*device).uid = API_CLASS;
    }
    ctx.instance.object_uid = uid(&API_CLASS);
    assert_eq!(restore(&ctx, token).0["outcome"], "not_applied");
    assert_eq!(unsafe { &*plugin }.loads.load(Ordering::Relaxed), 0);
    assert!(release(&ctx, token).is_err());
    ctx.document._identity = hold(device.cast()).unwrap();
    expire(&ctx).unwrap();
    assert_eq!(live_count(), 0);
    assert_eq!(restore(&ctx, token).0["outcome"], "not_applied");
    drop(ctx);
}
#[test]
fn load_failure_and_service_reentry_are_unknown_after_dispatch() {
    let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
    for stop in [false, true] {
        let mut fixture = Fixture::new();
        let plugin = ptr::addr_of_mut!(*fixture.plugin);
        let service = ptr::addr_of_mut!(*fixture.service);
        let ctx = fixture.context();
        let (saved, lease) = capture(&ctx).unwrap();
        drop(lease);
        let token = saved["token"].as_str().unwrap();
        if stop {
            unsafe {
                (*plugin).stop_on_load = service;
            }
        } else {
            unsafe {
                (*plugin).load_result = false;
            }
        }
        let (result, lease) = restore(&ctx, token);
        assert_eq!(result["ok"], false);
        assert_eq!(result["outcome"], "unknown");
        assert_eq!(unsafe { &*plugin }.loads.load(Ordering::Relaxed), 1);
        drop(lease);
        drop(ctx);
    }
}
#[test]
fn changed_alias_or_parent_blocks_restoration_before_native_load() {
    let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
    for replace_alias in [false, true] {
        let mut fixture = Fixture::new();
        let device = ptr::addr_of_mut!(*fixture.device);
        let plugin = ptr::addr_of_mut!(*fixture.plugin);
        let parent = ptr::addr_of_mut!(*fixture.parent);
        let replacement_parent = ptr::addr_of_mut!(*fixture.model);
        let plugin_table = fixture._plugin_table.as_ptr();
        let ctx = fixture.context();
        let (saved, lease) = capture(&ctx).unwrap();
        if replace_alias {
            unsafe {
                (*parent).table = plugin_table;
                (*parent).storable = true;
                (*device).alias = parent.cast();
            }
        } else {
            unsafe {
                (*device).parent = replacement_parent.cast();
            }
        }
        assert!(lease.guard(&ctx).is_err());
        drop(lease);
        assert_eq!(
            restore(&ctx, saved["token"].as_str().unwrap()).0["outcome"],
            "not_applied"
        );
        assert_eq!(unsafe { &*plugin }.loads.load(Ordering::Relaxed), 0);
        assert_eq!(unsafe { &*parent }.loads.load(Ordering::Relaxed), 0);
        drop(ctx);
    }
}
#[test]
fn capsule_limits_reject_before_an_additional_native_save() {
    let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut fixture = Fixture::new();
    let plugin = ptr::addr_of_mut!(*fixture.plugin);
    let ctx = fixture.context();
    let mut tokens = Vec::new();
    for _ in 0..8 {
        let (saved, lease) = capture(&ctx).unwrap();
        tokens.push(saved["token"].as_str().unwrap().to_owned());
        drop(lease);
    }
    assert!(capture(&ctx).is_err());
    assert_eq!(unsafe { &*plugin }.saves.load(Ordering::Relaxed), 8);
    release(&ctx, &tokens[0]).unwrap();
    let (_, lease) = capture(&ctx).unwrap();
    drop(lease);
    assert_eq!(unsafe { &*plugin }.saves.load(Ordering::Relaxed), 9);
    assert!(with_registry(ctx.service, |r| r.capacity(16 * 1024 * 1024 + 1)).is_err());
    drop(ctx);
}

#[test]
fn tokens_cannot_be_replayed_in_a_new_service() {
    let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut fixture = Fixture::new();
    let plugin = ptr::addr_of_mut!(*fixture.plugin);
    let mut ctx = fixture.context();
    let (saved, lease) = capture(&ctx).unwrap();
    drop(lease);
    let original_service = ctx.service;
    let mut other = service();
    ctx.service = ptr::addr_of_mut!(*other);
    let (result, lease) = restore(&ctx, saved["token"].as_str().unwrap());
    assert_eq!(result["outcome"], "not_applied");
    assert_eq!(unsafe { &*plugin }.loads.load(Ordering::Relaxed), 0);
    drop(lease);
    ctx.service = original_service;
    release(&ctx, saved["token"].as_str().unwrap()).unwrap();
    drop(ctx);
}

#[test]
fn total_capsule_bytes_are_bounded_even_before_count_limit() {
    let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut fixture = Fixture::new();
    *fixture.plugin.bytes.lock().unwrap() = vec![7; 16 * 1024 * 1024];
    let mut ctx = fixture.context();
    for _ in 0..4 {
        ctx.started = Instant::now();
        let (saved, lease) = capture(&ctx).unwrap();
        assert_eq!(saved["byte_count"], 16 * 1024 * 1024);
        drop(lease);
    }
    ctx.started = Instant::now();
    assert!(capture(&ctx).is_err());
    assert_eq!(live_count(), 4);
    assert_eq!(
        with_registry(ctx.service, |registry| registry.bytes()),
        64 * 1024 * 1024
    );
    drop(ctx);
}

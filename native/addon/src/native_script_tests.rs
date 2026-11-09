use super::*;

pub(super) static TEST_LOCK: Mutex<()> = Mutex::new(());

static OPEN_CALLS: AtomicU32 = AtomicU32::new(0);
static OPEN_PATH: AtomicPtr<c_char> = AtomicPtr::new(ptr::null_mut());
unsafe extern "C" fn failed_open(path: *const c_char, flags: i32) -> *mut c_void {
    assert_eq!(flags, SELF_PIN_FLAGS);
    OPEN_CALLS.fetch_add(1, Ordering::Relaxed);
    OPEN_PATH.store(path.cast_mut(), Ordering::Relaxed);
    ptr::null_mut()
}
#[test]
fn own_bundle_and_code_pin_fail_closed_without_paths_or_fallbacks() {
    let _serial = TEST_LOCK.lock().unwrap();
    let a = ptr::without_provenance_mut::<c_void>(0x1000);
    let b = ptr::without_provenance_mut::<c_void>(0x2000);
    assert!(!bundle_matches(a, ptr::null_mut(), false));
    assert!(!bundle_matches(a, b, true));
    assert!(!bundle_matches(a, a, false));
    assert!(bundle_matches(a, a, true));
    OPEN_CALLS.store(0, Ordering::Relaxed);
    assert!(pin_with(ptr::null(), failed_open).is_null());
    assert!(pin_with(c"relative".as_ptr(), failed_open).is_null());
    assert_eq!(OPEN_CALLS.load(Ordering::Relaxed), 0);
    let exact = c"/owned/already-loaded/probe".as_ptr();
    assert!(pin_with(exact, failed_open).is_null());
    assert_eq!(OPEN_CALLS.load(Ordering::Relaxed), 1);
    assert_eq!(OPEN_PATH.load(Ordering::Relaxed), exact.cast_mut());
}

static REGISTERED: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static TABLE_OWNS: AtomicBool = AtomicBool::new(false);
static REGISTER_CALLS: AtomicU32 = AtomicU32::new(0);
static FAIL_REGISTER: AtomicBool = AtomicBool::new(false);
static FAIL_UNREGISTER: AtomicBool = AtomicBool::new(false);
static REENTER_UNREGISTER: AtomicBool = AtomicBool::new(false);
static UNREGISTER_IN_FLIGHT: AtomicBool = AtomicBool::new(false);
static REENTER_REGISTER: AtomicBool = AtomicBool::new(false);
static STOP_BEFORE_REGISTER_PUBLISH: AtomicBool = AtomicBool::new(false);
static MODULE_STARTUPS: AtomicU32 = AtomicU32::new(0);
static MODULE_SHUTDOWNS: AtomicU32 = AtomicU32::new(0);
static MODULE_ACTIVE: AtomicBool = AtomicBool::new(false);
static STOP_DURING_MODULE_STARTUP: AtomicBool = AtomicBool::new(false);
static STOP_DURING_MODULE_SHUTDOWN: AtomicBool = AtomicBool::new(false);
static REINIT_DURING_MODULE_STARTUP: AtomicBool = AtomicBool::new(false);
const MOCK_MODULE: *mut c_void = ptr::without_provenance_mut::<c_void>(0x4000);
#[repr(C)]
struct MockScriptingTable {
    unused: [usize; 8],
    startup: unsafe extern "C" fn(*mut c_void, *const PlainCString, *mut c_void, *const c_void, u8),
    shutdown: unsafe extern "C" fn(*mut c_void, *mut c_void, u8),
}
#[repr(C)]
struct MockScripting {
    vtable: *const MockScriptingTable,
}
unsafe impl Sync for MockScripting {}
static SCRIPTING_TABLE: MockScriptingTable = MockScriptingTable {
    unused: [0; 8],
    startup: mock_module_startup,
    shutdown: mock_module_shutdown,
};
static SCRIPTING: MockScripting = MockScripting {
    vtable: &SCRIPTING_TABLE,
};
unsafe extern "C" fn mock_scripting() -> *mut c_void {
    ptr::addr_of!(SCRIPTING).cast_mut().cast()
}
unsafe extern "C" fn mock_module_startup(
    _: *mut c_void,
    id: *const PlainCString,
    module: *mut c_void,
    args: *const c_void,
    load: u8,
) {
    assert_eq!(ascii(unsafe { (*id).text }), MODULE_ID.to_str().unwrap());
    assert_eq!(module, MOCK_MODULE);
    assert!(args.is_null());
    assert_eq!(load, 0);
    assert!(!MODULE_ACTIVE.swap(true, Ordering::AcqRel));
    MODULE_STARTUPS.fetch_add(1, Ordering::Relaxed);
    let owner = MODULE_OWNER.load(Ordering::Acquire);
    assert!(!owner.is_null());
    if REINIT_DURING_MODULE_STARTUP.swap(false, Ordering::AcqRel) {
        assert_eq!(unsafe { start(owner) }, FAILED);
    }
    if STOP_DURING_MODULE_STARTUP.swap(false, Ordering::AcqRel) {
        let shutdowns = MODULE_SHUTDOWNS.load(Ordering::Acquire);
        assert_eq!(unsafe { stop(owner) }, OK);
        // Detach is deferred until the native startup call actually returns.
        assert!(MODULE_ACTIVE.load(Ordering::Acquire));
        assert_eq!(MODULE_SHUTDOWNS.load(Ordering::Acquire), shutdowns);
    }
}
unsafe extern "C" fn mock_module_shutdown(_: *mut c_void, module: *mut c_void, unload: u8) {
    assert_eq!(module, MOCK_MODULE);
    assert_eq!(unload, 0);
    assert!(REGISTERED.load(Ordering::Acquire).is_null());
    assert!(!UNREGISTER_IN_FLIGHT.load(Ordering::Acquire));
    assert!(MODULE_ACTIVE.load(Ordering::Acquire));
    MODULE_SHUTDOWNS.fetch_add(1, Ordering::Relaxed);
    if STOP_DURING_MODULE_SHUTDOWN.swap(false, Ordering::AcqRel) {
        let owner = MODULE_OWNER.load(Ordering::Acquire);
        assert!(!owner.is_null());
        assert_eq!(unsafe { stop(owner) }, OK);
    }
    assert!(MODULE_ACTIVE.swap(false, Ordering::AcqRel));
}
#[repr(C)]
struct MockTableVtable {
    unused: [usize; 3],
    register:
        unsafe extern "C" fn(*mut c_void, *mut c_void, *const Uid, *const PlainCString, i32) -> i32,
    unregister: unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32,
}
#[repr(C)]
struct MockTable {
    vtable: *const MockTableVtable,
}
unsafe impl Sync for MockTable {}
static MOCK_TABLE_VTABLE: MockTableVtable = MockTableVtable {
    unused: [0; 3],
    register: mock_register,
    unregister: mock_unregister,
};
static MOCK_TABLE: MockTable = MockTable {
    vtable: &MOCK_TABLE_VTABLE,
};
unsafe extern "C" fn mock_register(
    _: *mut c_void,
    object: *mut c_void,
    id: *const Uid,
    name: *const PlainCString,
    flags: i32,
) -> i32 {
    REGISTER_CALLS.fetch_add(1, Ordering::Relaxed);
    assert!(unsafe { *id == SERVICE_CID });
    assert_eq!(flags, 0);
    assert_eq!(ascii(unsafe { (*name).text }), "FenderMcpNative");
    if FAIL_REGISTER.load(Ordering::Acquire) {
        return FAILED;
    }
    if STOP_BEFORE_REGISTER_PUBLISH.load(Ordering::Acquire) {
        stop_during_register(object);
    }
    assert!(REGISTERED.swap(object, Ordering::AcqRel).is_null());
    if TABLE_OWNS.load(Ordering::Acquire) {
        unsafe { (unknown_vtable(object).retain)(object) };
    }
    if !STOP_BEFORE_REGISTER_PUBLISH.load(Ordering::Acquire) {
        stop_during_register(object);
    }
    OK
}
fn stop_during_register(object: *mut c_void) {
    if !REENTER_REGISTER.swap(false, Ordering::AcqRel) {
        return;
    }
    let owner = unsafe { (*object.cast::<ScriptFacet>()).owner };
    assert_eq!(unsafe { stop(owner) }, OK);
    assert!(unsafe { (*owner).terminated.load(Ordering::Acquire) });
    assert!(!unsafe { (*owner).initialized.load(Ordering::Acquire) });
    let calls = unsafe { (*owner).calls.load(Ordering::Acquire) };
    let mut out = VariantOut {
        kind: 1,
        padding: [0; 6],
        payload: 777,
    };
    let mut msg = message(c"capabilities", 0);
    assert_eq!(
        unsafe { script_invoke(object, &mut out, ptr::addr_of_mut!(msg).cast()) },
        0
    );
    assert_eq!((out.kind, out.payload), (1, 777));
    assert_eq!(unsafe { (*owner).calls.load(Ordering::Acquire) }, calls);
}
unsafe extern "C" fn mock_unregister(_: *mut c_void, object: *mut c_void) -> i32 {
    if FAIL_UNREGISTER.load(Ordering::Acquire) {
        return FAILED;
    }
    assert!(!UNREGISTER_IN_FLIGHT.swap(true, Ordering::AcqRel));
    assert_eq!(REGISTERED.swap(ptr::null_mut(), Ordering::AcqRel), object);
    if REENTER_UNREGISTER.swap(false, Ordering::AcqRel) {
        let owner = unsafe { (*object.cast::<ScriptFacet>()).owner };
        assert_eq!(unsafe { stop(owner) }, OK);
    }
    if TABLE_OWNS.load(Ordering::Acquire) {
        unsafe { (unknown_vtable(object).release)(object) };
    }
    UNREGISTER_IN_FLIGHT.store(false, Ordering::Release);
    OK
}
unsafe extern "C" fn mock_table() -> *mut c_void {
    ptr::addr_of!(MOCK_TABLE).cast_mut().cast()
}
unsafe extern "C" fn no_singleton() -> *mut c_void {
    ptr::null_mut()
}
unsafe extern "C" fn no_thread() -> i64 {
    0
}
struct StaticCString(PlainCString);
unsafe impl Sync for StaticCString {}
static GLOBAL_CSTRING: StaticCString = StaticCString(PlainCString {
    text: c"FenderMcpNative".as_ptr(),
    owner: ptr::null_mut(),
});
static MODULE_CSTRING: StaticCString = StaticCString(PlainCString {
    text: MODULE_ID.as_ptr(),
    owner: ptr::null_mut(),
});
unsafe extern "C" fn mock_cstring(name: *const c_char) -> *const PlainCString {
    if ascii(name) == MODULE_ID.to_str().unwrap() {
        &MODULE_CSTRING.0
    } else {
        &GLOBAL_CSTRING.0
    }
}

static LIVE_STRINGS: AtomicU32 = AtomicU32::new(0);
static FAIL_ASSIGN: AtomicBool = AtomicBool::new(false);
pub(super) static ASSIGN_REENTER_STOP: AtomicPtr<Service> = AtomicPtr::new(ptr::null_mut());
#[repr(C)]
struct MockStringVtable {
    base: UnknownVtable,
    unused3: usize,
    length: unsafe extern "C" fn(*mut c_void) -> i32,
    unused5_7: [usize; 3],
    copy: unsafe extern "C" fn(*mut c_void, *mut u16, i32) -> i32,
    unused9_17: [usize; 9],
    clone: Borrowed,
    unused19_23: [usize; 5],
    assign: unsafe extern "C" fn(*mut c_void, *const u16, i32) -> i32,
}
#[repr(C)]
struct MockString {
    vtable: *const MockStringVtable,
    refs: AtomicU32,
    units: Mutex<Vec<u16>>,
    immortal: bool,
}
unsafe impl Sync for MockString {}
static STRING_TABLE: MockStringVtable = MockStringVtable {
    base: UnknownVtable {
        query: no_query,
        retain: string_retain,
        release: string_release,
    },
    unused3: 0,
    length: string_length,
    unused5_7: [0; 3],
    copy: string_copy,
    unused9_17: [0; 9],
    clone: string_clone,
    unused19_23: [0; 5],
    assign: string_assign,
};
static EMPTY: MockString = MockString {
    vtable: &STRING_TABLE,
    refs: AtomicU32::new(1),
    units: Mutex::new(Vec::new()),
    immortal: true,
};
unsafe extern "C" fn no_query(_: *mut c_void, _: *const Uid, out: *mut *mut c_void) -> i32 {
    unsafe { *out = ptr::null_mut() };
    NO_INTERFACE
}
unsafe extern "C" fn string_retain(p: *mut c_void) -> u32 {
    unsafe {
        (*p.cast::<MockString>())
            .refs
            .fetch_add(1, Ordering::Relaxed)
            + 1
    }
}
unsafe extern "C" fn string_release(p: *mut c_void) -> u32 {
    let s = unsafe { &*p.cast::<MockString>() };
    let remaining = s.refs.fetch_sub(1, Ordering::AcqRel) - 1;
    if remaining == 0 {
        assert!(!s.immortal);
        unsafe { drop(Box::from_raw(p.cast::<MockString>())) };
        LIVE_STRINGS.fetch_sub(1, Ordering::Relaxed);
    }
    remaining
}
unsafe extern "C" fn string_clone(_: *mut c_void) -> *mut c_void {
    LIVE_STRINGS.fetch_add(1, Ordering::Relaxed);
    Box::into_raw(Box::new(MockString {
        vtable: &STRING_TABLE,
        refs: AtomicU32::new(1),
        units: Mutex::new(Vec::new()),
        immortal: false,
    }))
    .cast()
}
unsafe extern "C" fn string_assign(p: *mut c_void, text: *const u16, count: i32) -> i32 {
    if FAIL_ASSIGN.load(Ordering::Acquire) {
        return FAILED;
    }
    let s = unsafe { &*p.cast::<MockString>() };
    *s.units.lock().unwrap() = unsafe { std::slice::from_raw_parts(text, count as usize) }.to_vec();
    let service = ASSIGN_REENTER_STOP.swap(ptr::null_mut(), Ordering::AcqRel);
    if !service.is_null() {
        assert_eq!(unsafe { stop(service) }, OK);
    }
    OK
}
unsafe extern "C" fn string_length(p: *mut c_void) -> i32 {
    unsafe { &*p.cast::<MockString>() }
        .units
        .lock()
        .unwrap()
        .len() as i32
}
unsafe extern "C" fn string_copy(p: *mut c_void, out: *mut u16, cap: i32) -> i32 {
    let units = unsafe { &*p.cast::<MockString>() }.units.lock().unwrap();
    if cap <= units.len() as i32 {
        return FAILED;
    }
    unsafe {
        ptr::copy_nonoverlapping(units.as_ptr(), out, units.len());
        *out.add(units.len()) = 0;
    }
    OK
}
unsafe extern "C" fn mock_empty() -> *mut c_void {
    ptr::addr_of!(EMPTY).cast_mut().cast()
}
pub(super) fn output_text(out: &VariantOut) -> String {
    assert_eq!(out.kind, 3 | 16);
    let string = unsafe { &*(out.payload as *const MockString) };
    String::from_utf16(&string.units.lock().unwrap()).unwrap()
}
pub(super) fn clear_output(out: &mut VariantOut) {
    if out.kind & 16 != 0 && (out.kind & 15 == 3 || out.kind & 15 == 4) && out.payload != 0 {
        let p = out.payload as *mut c_void;
        unsafe { (unknown_vtable(p).release)(p) };
    }
    out.kind = 0;
    out.payload = 0;
}

#[repr(C)]
struct MockMessageVtable {
    unused: [usize; 3],
    id: unsafe extern "C" fn(*mut c_void) -> *const PlainCString,
    count: Count,
    arg: unsafe extern "C" fn(*mut c_void, i32) -> *const VariantOut,
}
#[repr(C)]
struct MockMessage {
    vtable: *const MockMessageVtable,
    id: PlainCString,
    count: i32,
    arg: VariantOut,
    arg2: VariantOut,
    arg3: VariantOut,
}
static MESSAGE_TABLE: MockMessageVtable = MockMessageVtable {
    unused: [0; 3],
    id: message_id,
    count: message_count,
    arg: message_arg,
};
unsafe extern "C" fn message_id(p: *mut c_void) -> *const PlainCString {
    unsafe { &(*p.cast::<MockMessage>()).id }
}
unsafe extern "C" fn message_count(p: *mut c_void) -> i32 {
    unsafe { (*p.cast::<MockMessage>()).count }
}
unsafe extern "C" fn message_arg(p: *mut c_void, index: i32) -> *const VariantOut {
    match index {
        0 => unsafe { &(*p.cast::<MockMessage>()).arg },
        1 => unsafe { &(*p.cast::<MockMessage>()).arg2 },
        2 => unsafe { &(*p.cast::<MockMessage>()).arg3 },
        _ => ptr::null(),
    }
}
fn message(name: &std::ffi::CStr, count: i32) -> MockMessage {
    MockMessage {
        vtable: &MESSAGE_TABLE,
        id: PlainCString {
            text: name.as_ptr(),
            owner: ptr::null_mut(),
        },
        count,
        arg: VariantOut {
            kind: 0,
            padding: [0; 6],
            payload: 0,
        },
        arg2: VariantOut {
            kind: 0,
            padding: [0; 6],
            payload: 0,
        },
        arg3: VariantOut {
            kind: 0,
            padding: [0; 6],
            payload: 0,
        },
    }
}
#[repr(C)]
struct MockTarget {
    vtable: *const UnknownVtable,
    refs: AtomicU32,
}
static TARGET_TABLE: UnknownVtable = UnknownVtable {
    query: no_query,
    retain: target_retain,
    release: target_release,
};
unsafe extern "C" fn target_retain(p: *mut c_void) -> u32 {
    unsafe {
        (*p.cast::<MockTarget>())
            .refs
            .fetch_add(1, Ordering::Relaxed)
            + 1
    }
}
unsafe extern "C" fn target_release(p: *mut c_void) -> u32 {
    unsafe {
        (*p.cast::<MockTarget>())
            .refs
            .fetch_sub(1, Ordering::Relaxed)
            - 1
    }
}

#[test]
fn metadata_is_borrowed_stable_and_module_associated() {
    let _serial = TEST_LOCK.lock().unwrap();
    assert_eq!(unsafe { type_flags(ptr::null_mut()) }, 2);
    assert_eq!(
        ascii(unsafe { type_name(ptr::null_mut()) }),
        "FenderMcpNative"
    );
    assert_eq!(METHODS[0].name, c"capabilities".as_ptr());
    assert!(METHODS[2].name.is_null());
    assert_eq!(
        ascii(METHODS[1].args),
        "target: Object, request: string, auxiliary: Object = null"
    );
    let module = ptr::without_provenance_mut::<c_void>(0x4000);
    MODULE_REF.store(module, Ordering::Release);
    assert_eq!(unsafe { type_module(ptr::null_mut()) }, module);
    let mut queried = ptr::null_mut();
    assert_eq!(
        unsafe {
            type_query(
                ptr::addr_of!(TYPE).cast_mut().cast(),
                &TYPE_INFO,
                &mut queried,
            )
        },
        OK
    );
    assert_eq!(TYPE.refs.load(Ordering::Acquire), 2);
    assert_eq!(unsafe { type_release(queried) }, 1);
    MODULE_REF.store(ptr::null_mut(), Ordering::Release);
}

#[test]
fn registration_proxy_lifetime_unboxing_and_dynamic_string_ownership() {
    let _serial = TEST_LOCK.lock().unwrap();
    install_mock_api();
    REGISTER_CALLS.store(0, Ordering::Release);
    assert!(!runtime_supported());
    READY.store(false, Ordering::Release);
    let mut raw = ptr::null_mut();
    assert_eq!(
        unsafe { factory_create(ptr::null_mut(), &SERVICE_CID, &COMPONENT, &mut raw) },
        FAILED
    );
    assert!(raw.is_null());
    assert_eq!(REGISTER_CALLS.load(Ordering::Acquire), 0);
    READY.store(true, Ordering::Release);
    for table_owns in [false, true] {
        TABLE_OWNS.store(table_owns, Ordering::Release);
        assert_eq!(
            unsafe { factory_create(ptr::null_mut(), &SERVICE_CID, &COMPONENT, &mut raw) },
            OK
        );
        let service = raw.cast::<Service>();
        READY.store(false, Ordering::Release);
        assert_eq!(unsafe { service_initialize(raw, ptr::null_mut()) }, FAILED);
        assert!(!unsafe { (*service).registered.load(Ordering::Acquire) });
        READY.store(true, Ordering::Release);
        FAIL_REGISTER.store(true, Ordering::Release);
        assert_eq!(unsafe { service_initialize(raw, ptr::null_mut()) }, FAILED);
        assert_eq!(unsafe { (*service).refs.load(Ordering::Acquire) }, 1);
        FAIL_REGISTER.store(false, Ordering::Release);
        assert_eq!(unsafe { service_initialize(raw, ptr::null_mut()) }, OK);
        let mut proxy = ptr::null_mut();
        assert_eq!(unsafe { service_query(raw, &OBJECT, &mut proxy) }, OK);
        let expected = if table_owns { 4 } else { 3 };
        assert_eq!(unsafe { (*service).refs.load(Ordering::Acquire) }, expected);
        let mut out = VariantOut {
            kind: 1,
            padding: [0; 6],
            payload: 7,
        };
        let mut msg = message(c"capabilities", 0);
        assert_eq!(
            unsafe { script_invoke(proxy, &mut out, ptr::addr_of_mut!(msg).cast()) },
            1
        );
        assert!(output_text(&out).contains("fender-mcp-native-v1"));
        assert_eq!(LIVE_STRINGS.load(Ordering::Acquire), 1);
        FAIL_ASSIGN.store(true, Ordering::Release);
        let old = out.payload;
        assert!(!return_json(&mut out, "new", api().unwrap()));
        assert_eq!(out.payload, old);
        assert_eq!(LIVE_STRINGS.load(Ordering::Acquire), 1);
        FAIL_ASSIGN.store(false, Ordering::Release);
        assert!(return_json(&mut out, "한글 🎵", api().unwrap()));
        assert_eq!(output_text(&out), "한글 🎵");
        assert_eq!(LIVE_STRINGS.load(Ordering::Acquire), 1);
        let mut target = MockTarget {
            vtable: &TARGET_TABLE,
            refs: AtomicU32::new(1),
        };
        msg = message(c"request", 2);
        msg.arg.kind = 4 | 16;
        msg.arg.payload = ptr::addr_of_mut!(target) as u64;
        let input = unsafe { string_clone(ptr::null_mut()) };
        let units: Vec<_> = "{\"operation\":\"invalid\"}".encode_utf16().collect();
        assert_eq!(
            unsafe { string_assign(input, units.as_ptr(), units.len() as i32) },
            OK
        );
        msg.arg2.kind = 3;
        msg.arg2.payload = input as u64;
        assert_eq!(
            unsafe { script_invoke(proxy, &mut out, ptr::addr_of_mut!(msg).cast()) },
            1
        );
        assert!(output_text(&out).contains("invalid_request"));
        assert_eq!(target.refs.load(Ordering::Acquire), 1);
        msg.count = 3;
        msg.arg3.kind = 4;
        msg.arg3.payload = ptr::addr_of_mut!(target) as u64;
        assert_eq!(
            unsafe { script_invoke(proxy, &mut out, ptr::addr_of_mut!(msg).cast()) },
            1
        );
        assert!(output_text(&out).contains("invalid_request"));
        assert_eq!(target.refs.load(Ordering::Acquire), 1);
        let units: Vec<_> = "{\"operation\":\"inspect\"}".encode_utf16().collect();
        assert_eq!(
            unsafe { string_assign(input, units.as_ptr(), units.len() as i32) },
            OK
        );
        assert_eq!(
            unsafe { script_invoke(proxy, &mut out, ptr::addr_of_mut!(msg).cast()) },
            1
        );
        assert!(output_text(&out).contains("invalid_arguments"));
        assert_eq!(target.refs.load(Ordering::Acquire), 1);
        msg.arg3.kind = 3;
        assert_eq!(
            unsafe { script_invoke(proxy, &mut out, ptr::addr_of_mut!(msg).cast()) },
            1
        );
        assert!(output_text(&out).contains("invalid_arguments"));
        msg.count = 2;
        unsafe { string_release(input) };
        msg.arg2.payload = 0;
        assert_eq!(
            unsafe { script_invoke(proxy, &mut out, ptr::addr_of_mut!(msg).cast()) },
            1
        );
        assert!(output_text(&out).contains("invalid_arguments"));
        // A nested call fails without invoking native APIs or changing output.
        unsafe { (*service).busy.store(true, Ordering::Release) };
        assert_eq!(
            unsafe { script_invoke(proxy, &mut out, ptr::addr_of_mut!(msg).cast()) },
            0
        );
        unsafe { (*service).busy.store(false, Ordering::Release) };
        // Production has per-call bounds, never a process-total 64-call cap.
        msg = message(c"capabilities", 0);
        for _ in 0..70 {
            assert_eq!(
                unsafe { script_invoke(proxy, &mut out, ptr::addr_of_mut!(msg).cast()) },
                1
            );
        }
        assert_eq!(LIVE_STRINGS.load(Ordering::Acquire), 1);
        clear_output(&mut out);
        assert_eq!(LIVE_STRINGS.load(Ordering::Acquire), 0);
        FAIL_UNREGISTER.store(true, Ordering::Release);
        assert_eq!(unsafe { service_terminate(raw) }, FAILED);
        assert!(unsafe { (*service).registered.load(Ordering::Acquire) });
        assert!(!unsafe { (*service).initialized.load(Ordering::Acquire) });
        FAIL_UNREGISTER.store(false, Ordering::Release);
        REENTER_UNREGISTER.store(true, Ordering::Release);
        assert_eq!(unsafe { service_terminate(raw) }, OK);
        assert!(REGISTERED.load(Ordering::Acquire).is_null());
        assert_eq!(unsafe { service_initialize(raw, ptr::null_mut()) }, FAILED);
        assert_eq!(unsafe { service_release(raw) }, 1); // JS proxy remains alive.
        assert!(!module_exit_state());
        assert_eq!(
            unsafe { script_invoke(proxy, &mut out, ptr::addr_of_mut!(msg).cast()) },
            0
        );
        // A different service may be active without reactivating the old proxy.
        let mut second = ptr::null_mut();
        assert_eq!(
            unsafe { factory_create(ptr::null_mut(), &SERVICE_CID, &COMPONENT, &mut second) },
            OK
        );
        assert_eq!(unsafe { service_initialize(second, ptr::null_mut()) }, OK);
        assert_eq!(
            unsafe { script_invoke(proxy, &mut out, ptr::addr_of_mut!(msg).cast()) },
            0
        );
        assert_eq!(unsafe { service_terminate(second) }, OK);
        assert_eq!(unsafe { service_release(second) }, 0);
        assert_eq!(unsafe { script_release(proxy) }, 0);
        assert_eq!(LIVE_SERVICES.load(Ordering::Acquire), 0);
        assert!(module_exit_state());
    }
    assert_eq!(EMPTY.refs.load(Ordering::Acquire), 1);
    READY.store(false, Ordering::Release);
}

pub(super) fn install_mock_api() {
    MODULE_REF.store(MOCK_MODULE, Ordering::Release);
    API.get_or_init(|| {
        Some(Api {
            gui: no_singleton,
            objects: mock_table,
            main_thread: no_singleton,
            thread_self: no_thread,
            cstring: mock_cstring,
            empty: mock_empty,
            desktop: no_singleton,
            scripting: mock_scripting,
            plugin_manager: mock_scripting,
            file_system: mock_scripting,
        })
    });
}

#[test]
fn module_attachment_pairs_through_reentry_and_rejects_duplicate_owners() {
    let _serial = TEST_LOCK.lock().unwrap();
    install_mock_api();
    assert!(MODULE_OWNER.load(Ordering::Acquire).is_null());
    assert!(!MODULE_ACTIVE.load(Ordering::Acquire));
    READY.store(true, Ordering::Release);
    for table_owns in [false, true] {
        TABLE_OWNS.store(table_owns, Ordering::Release);
        let startups = MODULE_STARTUPS.load(Ordering::Acquire);
        let shutdowns = MODULE_SHUTDOWNS.load(Ordering::Acquire);
        let registrations = REGISTER_CALLS.load(Ordering::Acquire);
        let mut raw = ptr::null_mut();
        assert_eq!(
            unsafe { factory_create(ptr::null_mut(), &SERVICE_CID, &COMPONENT, &mut raw) },
            OK
        );
        let mut proxy = ptr::null_mut();
        assert_eq!(unsafe { service_query(raw, &OBJECT, &mut proxy) }, OK);
        STOP_DURING_MODULE_STARTUP.store(true, Ordering::Release);
        REINIT_DURING_MODULE_STARTUP.store(true, Ordering::Release);
        assert_eq!(unsafe { service_initialize(raw, ptr::null_mut()) }, FAILED);
        let service = raw.cast::<Service>();
        assert!(unsafe { (*service).terminated.load(Ordering::Acquire) });
        assert!(!unsafe { (*service).module_attached.load(Ordering::Acquire) });
        assert!(!unsafe { (*service).initializing.load(Ordering::Acquire) });
        assert!(MODULE_OWNER.load(Ordering::Acquire).is_null());
        assert!(!MODULE_ACTIVE.load(Ordering::Acquire));
        assert_eq!(MODULE_STARTUPS.load(Ordering::Acquire), startups + 1);
        assert_eq!(MODULE_SHUTDOWNS.load(Ordering::Acquire), shutdowns + 1);
        assert_eq!(REGISTER_CALLS.load(Ordering::Acquire), registrations);
        assert_eq!(unsafe { (*service).refs.load(Ordering::Acquire) }, 2);
        assert_eq!(unsafe { service_initialize(raw, ptr::null_mut()) }, FAILED);
        assert_eq!(unsafe { service_release(raw) }, 1);
        assert_eq!(unsafe { script_release(proxy) }, 0);
        // Only one module owner can publish the fixed global API. An unrelated
        // second instance fails before a duplicate startup or registration.
        assert_eq!(
            unsafe { factory_create(ptr::null_mut(), &SERVICE_CID, &COMPONENT, &mut raw) },
            OK
        );
        assert_eq!(unsafe { service_initialize(raw, ptr::null_mut()) }, OK);
        assert_eq!(MODULE_OWNER.load(Ordering::Acquire), raw.cast::<Service>());
        let mut other = ptr::null_mut();
        assert_eq!(
            unsafe { factory_create(ptr::null_mut(), &SERVICE_CID, &COMPONENT, &mut other) },
            OK
        );
        assert_eq!(
            unsafe { service_initialize(other, ptr::null_mut()) },
            FAILED
        );
        assert_eq!(unsafe { service_release(other) }, 0);
        assert_eq!(MODULE_STARTUPS.load(Ordering::Acquire), startups + 2);
        assert_eq!(REGISTER_CALLS.load(Ordering::Acquire), registrations + 1);
        STOP_DURING_MODULE_SHUTDOWN.store(true, Ordering::Release);
        assert_eq!(unsafe { service_terminate(raw) }, OK);
        assert_eq!(MODULE_SHUTDOWNS.load(Ordering::Acquire), shutdowns + 2);
        assert!(MODULE_OWNER.load(Ordering::Acquire).is_null());
        assert!(!MODULE_ACTIVE.load(Ordering::Acquire));
        assert!(REGISTERED.load(Ordering::Acquire).is_null());
        assert_eq!(unsafe { service_release(raw) }, 0);
        assert!(module_exit_state());
    }
    assert_eq!(
        MODULE_STARTUPS.load(Ordering::Acquire),
        MODULE_SHUTDOWNS.load(Ordering::Acquire)
    );
    READY.store(false, Ordering::Release);
}

#[test]
fn stop_during_registration_cleans_late_entry_without_reactivation() {
    let _serial = TEST_LOCK.lock().unwrap();
    install_mock_api();
    assert!(REGISTERED.load(Ordering::Acquire).is_null());
    assert_eq!(LIVE_SERVICES.load(Ordering::Acquire), 0);
    assert_eq!(LIVE_STRINGS.load(Ordering::Acquire), 0);
    READY.store(true, Ordering::Release);
    // Native table completion may publish before or after its callback stops
    // the service. Both table reference policies must balance identically.
    for table_owns in [false, true] {
        for stop_before_publish in [false, true] {
            for cleanup_fails_once in [false, true] {
                TABLE_OWNS.store(table_owns, Ordering::Release);
                STOP_BEFORE_REGISTER_PUBLISH.store(stop_before_publish, Ordering::Release);
                REENTER_REGISTER.store(true, Ordering::Release);
                FAIL_UNREGISTER.store(cleanup_fails_once, Ordering::Release);
                let mut raw = ptr::null_mut();
                assert_eq!(
                    unsafe { factory_create(ptr::null_mut(), &SERVICE_CID, &COMPONENT, &mut raw) },
                    OK
                );
                let service = raw.cast::<Service>();
                let mut proxy = ptr::null_mut();
                assert_eq!(unsafe { service_query(raw, &OBJECT, &mut proxy) }, OK);
                let calls = unsafe { (*service).calls.load(Ordering::Acquire) };
                assert_eq!(unsafe { service_initialize(raw, ptr::null_mut()) }, FAILED);
                assert!(unsafe { (*service).terminated.load(Ordering::Acquire) });
                assert!(!unsafe { (*service).initialized.load(Ordering::Acquire) });
                assert_eq!(unsafe { (*service).calls.load(Ordering::Acquire) }, 0);
                assert_eq!(unsafe { (*service).calls.load(Ordering::Acquire) }, calls);
                if cleanup_fails_once {
                    // A failed unregister must remain visible and strongly
                    // owned, then permit explicit retry while still inactive.
                    assert_eq!(REGISTERED.load(Ordering::Acquire), proxy);
                    assert!(unsafe { (*service).registered.load(Ordering::Acquire) });
                    assert_eq!(
                        unsafe { (*service).refs.load(Ordering::Acquire) },
                        if table_owns { 4 } else { 3 }
                    );
                    FAIL_UNREGISTER.store(false, Ordering::Release);
                    assert_eq!(unsafe { service_terminate(raw) }, OK);
                }
                assert!(REGISTERED.load(Ordering::Acquire).is_null());
                assert!(!unsafe { (*service).registered.load(Ordering::Acquire) });
                assert_eq!(unsafe { (*service).refs.load(Ordering::Acquire) }, 2);
                let registrations = REGISTER_CALLS.load(Ordering::Acquire);
                assert_eq!(unsafe { service_initialize(raw, ptr::null_mut()) }, FAILED);
                assert_eq!(REGISTER_CALLS.load(Ordering::Acquire), registrations);
                let mut out = VariantOut {
                    kind: 1,
                    padding: [0; 6],
                    payload: 777,
                };
                let mut msg = message(c"capabilities", 0);
                assert_eq!(
                    unsafe { script_invoke(proxy, &mut out, ptr::addr_of_mut!(msg).cast()) },
                    0
                );
                assert_eq!((out.kind, out.payload), (1, 777));
                assert_eq!(LIVE_STRINGS.load(Ordering::Acquire), 0);
                assert_eq!(unsafe { service_release(raw) }, 1);
                // Global READY and another live service never reactivate the
                // stopped proxy that remains independently retained by JS.
                let mut second = ptr::null_mut();
                assert_eq!(
                    unsafe {
                        factory_create(ptr::null_mut(), &SERVICE_CID, &COMPONENT, &mut second)
                    },
                    OK
                );
                assert_eq!(unsafe { service_initialize(second, ptr::null_mut()) }, OK);
                assert_eq!(
                    unsafe { script_invoke(proxy, &mut out, ptr::addr_of_mut!(msg).cast()) },
                    0
                );
                assert_eq!(unsafe { (*service).calls.load(Ordering::Acquire) }, calls);
                assert_eq!(unsafe { service_terminate(second) }, OK);
                assert_eq!(unsafe { service_release(second) }, 0);
                assert_eq!(unsafe { script_release(proxy) }, 0);
                assert!(REGISTERED.load(Ordering::Acquire).is_null());
                assert_eq!(LIVE_SERVICES.load(Ordering::Acquire), 0);
                assert!(module_exit_state());
            }
        }
    }
    READY.store(false, Ordering::Release);
}

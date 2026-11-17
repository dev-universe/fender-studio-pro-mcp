//! Native Fender Studio Pro 8.1.1 add-on with a finite synchronous script API.
//! ABI facts: public ccl-framework commit ad5876b1a7b64aa247f4378c86a7862e7c284ba3.
//! This file does not include or translate CCL implementation code.
#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_char, c_void};
use std::fs::{self, DirBuilder, OpenOptions};
use std::io::Write;
use std::mem::{align_of, offset_of, size_of};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::PathBuf;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
mod native_script;

const OK: i32 = 0;
const NO_INTERFACE: i32 = 0x8000_4002_u32 as i32;
const INVALID_POINTER: i32 = 0x8000_4003_u32 as i32;
const CLASS_NOT_FOUND: i32 = 0x8004_0154_u32 as i32;
const FAILED: i32 = 0x8000_4005_u32 as i32;

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
struct Uid {
    a: u32,
    b: u16,
    c: u16,
    d: [u8; 8],
}

const UNKNOWN: Uid = Uid {
    a: 0,
    b: 0,
    c: 0,
    d: [0xc0, 0, 0, 0, 0, 0, 0, 0x46],
};
const ALLOCATOR: Uid = Uid {
    a: 0xc489642f,
    b: 0xed26,
    c: 0x490f,
    d: [0xb6, 0x0a, 0x67, 0x88, 0x64, 0x58, 0xb0, 0xc1],
};
const FACTORY_IID: Uid = Uid {
    a: 0xbeac2a68,
    b: 0x8c44,
    c: 0x4ce3,
    d: [0xb4, 0x46, 0x58, 0x9b, 0xd9, 0x21, 0x5c, 0xee],
};
const COMPONENT: Uid = Uid {
    a: 0x11688bdb,
    b: 0xf961,
    c: 0x4535,
    d: [0xac, 0xda, 0x1d, 0x29, 0x75, 0x8e, 0xc6, 0xc8],
};
// Independently generated service UUID.
const SERVICE_CID: Uid = Uid {
    a: 0x9bc66956,
    b: 0x3cb1,
    c: 0x43f5,
    d: [0xab, 0x90, 0x7d, 0xf5, 0xc6, 0x08, 0xc9, 0x04],
};

#[repr(C)]
struct UnknownVtable {
    query: unsafe extern "C" fn(*mut c_void, *const Uid, *mut *mut c_void) -> i32,
    retain: unsafe extern "C" fn(*mut c_void) -> u32,
    release: unsafe extern "C" fn(*mut c_void) -> u32,
}

// The second pointer is opaque CCL debugger data. Only copy it as part of the
// documented PlainString representation; never inspect or dereference it.
#[repr(C)]
#[derive(Clone, Copy)]
struct PlainString {
    object: *mut c_void,
    opaque_debug: *mut c_void,
}

#[repr(C)]
struct VersionDesc {
    name: PlainString,
    version: PlainString,
    vendor: PlainString,
    copyright: PlainString,
    url: PlainString,
}

#[repr(C)]
struct ClassDesc {
    flags: i32,
    cid: Uid,
    category: PlainString,
    name: PlainString,
    subcategory: PlainString,
    description: PlainString,
}

type ConstantString = unsafe extern "C" fn(*const c_char) -> *const PlainString;

#[link(name = "System")]
unsafe extern "C" {
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

static GET_STRING: OnceLock<Option<ConstantString>> = OnceLock::new();
static READY: AtomicBool = AtomicBool::new(false);
static MODULE_REF: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static LIVE_SERVICES: AtomicU32 = AtomicU32::new(0);
static MARKER_LOCK: Mutex<u32> = Mutex::new(0);

fn constant_string() -> Option<ConstantString> {
    *GET_STRING.get_or_init(|| {
        // macOS dlfcn.h defines RTLD_DEFAULT as (void*)-2. This only looks up
        // already-loaded public APIs; it does not open any vendor library.
        let symbol = unsafe { dlsym((-2_isize) as *mut c_void, c"CCLGetConstantString".as_ptr()) };
        if symbol.is_null() {
            None
        } else {
            Some(unsafe { std::mem::transmute::<*mut c_void, ConstantString>(symbol) })
        }
    })
}

// Runtime output is restricted to one owned directory, static event names and
// process/time/counter numbers. No song names, UI values or pointers are logged.
fn marker(event: &'static str) {
    log_detail(event, "{}");
}

fn private_folder() -> Option<PathBuf> {
    use std::os::unix::fs::MetadataExt;
    let home = std::env::var_os("HOME")?;
    let folder =
        PathBuf::from(home).join("Library/Application Support/fender-studio-pro-mcp/native");
    if fs::symlink_metadata(&folder).is_ok_and(|m| m.file_type().is_symlink()) {
        return None;
    }
    DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&folder)
        .ok()?;
    let meta = fs::symlink_metadata(&folder).ok()?;
    if !meta.is_dir() || meta.mode() & 0o777 != 0o700 {
        return None;
    }
    Some(folder)
}

fn log_detail(event: &'static str, detail_json: &str) {
    if cfg!(test) {
        return;
    }
    let Ok(mut count) = MARKER_LOCK.lock() else {
        return;
    };
    if *count >= 4096 {
        return;
    }
    let Some(folder) = private_folder() else {
        return;
    };
    let path = folder.join(format!("native-{}.jsonl", std::process::id()));
    use std::os::unix::fs::MetadataExt;
    let existing_bytes = match fs::symlink_metadata(&path) {
        Ok(m) if m.is_file() && !m.file_type().is_symlink() && m.mode() & 0o777 == 0o600 => m.len(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => 0,
        _ => return,
    };
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    let record = format!(
        "{{\"probe\":\"native-v1-ccl512\",\"event\":\"{event}\",\"pid\":{},\"unix_ms\":{millis},\"factory_refs\":{},\"services_live\":{},\"detail\":{detail_json}}}\n",
        std::process::id(),
        FACTORY.refs.load(Ordering::Relaxed),
        LIVE_SERVICES.load(Ordering::Relaxed)
    );
    if existing_bytes.saturating_add(record.len() as u64) > 4 * 1024 * 1024 {
        return;
    }
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(path)
    {
        if file.write_all(record.as_bytes()).is_ok() {
            *count += 1;
        }
    }
}

unsafe fn unknown_vtable(object: *mut c_void) -> &'static UnknownVtable {
    unsafe { &**object.cast::<*const UnknownVtable>() }
}

unsafe fn assign_string(destination: *mut PlainString, incoming: PlainString) {
    // These are live, host-constructed CCL strings. Hold the new reference
    // before replacing and releasing the destination's old reference.
    unsafe { (unknown_vtable(incoming.object).retain)(incoming.object) };
    let previous = unsafe { ptr::replace(destination, incoming) };
    if !previous.object.is_null() {
        unsafe { (unknown_vtable(previous.object).release)(previous.object) };
    }
}

fn strings<const N: usize>(values: [&'static std::ffi::CStr; N]) -> Option<[PlainString; N]> {
    let get = constant_string()?;
    let mut result = [PlainString {
        object: ptr::null_mut(),
        opaque_debug: ptr::null_mut(),
    }; N];
    for (slot, value) in result.iter_mut().zip(values) {
        let pointer = unsafe { get(value.as_ptr()) };
        if pointer.is_null() {
            return None;
        }
        *slot = unsafe { *pointer };
        if slot.object.is_null() {
            return None;
        }
    }
    Some(result)
}

#[repr(C)]
struct FactoryVtable {
    base: UnknownVtable,
    create: unsafe extern "C" fn(*mut c_void, *const Uid, *const Uid, *mut *mut c_void) -> i32,
    version: unsafe extern "C" fn(*mut c_void, *mut VersionDesc),
    count: unsafe extern "C" fn(*mut c_void) -> i32,
    description: unsafe extern "C" fn(*mut c_void, *mut ClassDesc, i32) -> u8,
    attributes: unsafe extern "C" fn(*mut c_void, *mut c_void, *const Uid, *const c_void) -> u8,
}

#[repr(C)]
struct Factory {
    vtable: *const FactoryVtable,
    refs: AtomicU32,
}
// The vtable is immutable for module lifetime; the only changing field is atomic.
unsafe impl Sync for Factory {}
static FACTORY_VTABLE: FactoryVtable = FactoryVtable {
    base: UnknownVtable {
        query: factory_query,
        retain: factory_retain,
        release: factory_release,
    },
    create: factory_create,
    version: factory_version,
    count: factory_count,
    description: factory_description,
    attributes: factory_attributes,
};
static FACTORY: Factory = Factory {
    vtable: &FACTORY_VTABLE,
    refs: AtomicU32::new(0),
};

unsafe extern "C" fn factory_query(
    this: *mut c_void,
    iid: *const Uid,
    out: *mut *mut c_void,
) -> i32 {
    if out.is_null() {
        return INVALID_POINTER;
    }
    unsafe { *out = ptr::null_mut() };
    if iid.is_null() {
        return INVALID_POINTER;
    }
    if unsafe { *iid == UNKNOWN || *iid == ALLOCATOR || *iid == FACTORY_IID } {
        unsafe {
            factory_retain(this);
            *out = this;
        }
        OK
    } else {
        NO_INTERFACE
    }
}

unsafe extern "C" fn factory_retain(_: *mut c_void) -> u32 {
    FACTORY.refs.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "C" fn factory_release(_: *mut c_void) -> u32 {
    let mut previous = FACTORY.refs.load(Ordering::Relaxed);
    loop {
        if previous == 0 {
            marker("factory_release_without_reference");
            return 0;
        }
        match FACTORY.refs.compare_exchange_weak(
            previous,
            previous - 1,
            Ordering::AcqRel,
            Ordering::Relaxed,
        ) {
            Ok(_) => {
                if previous == 1 {
                    marker("factory_released");
                }
                return previous - 1;
            }
            Err(actual) => previous = actual,
        }
    }
}

unsafe extern "C" fn factory_version(_: *mut c_void, out: *mut VersionDesc) {
    if !READY.load(Ordering::Acquire) || out.is_null() {
        return;
    }
    let Some(values) = strings([
        c"Fender MCP Native",
        c"0.1.0",
        c"fender-studio-pro-mcp",
        c"Independently authored native interoperability add-on",
        c"https://github.com/dev-universe/fender-studio-pro-mcp",
    ]) else {
        marker("factory_version_string_failed");
        return;
    };
    unsafe {
        assign_string(ptr::addr_of_mut!((*out).name), values[0]);
        assign_string(ptr::addr_of_mut!((*out).version), values[1]);
        assign_string(ptr::addr_of_mut!((*out).vendor), values[2]);
        assign_string(ptr::addr_of_mut!((*out).copyright), values[3]);
        assign_string(ptr::addr_of_mut!((*out).url), values[4]);
    }
    marker("factory_version");
}

unsafe extern "C" fn factory_count(_: *mut c_void) -> i32 {
    marker("factory_count");
    1
}

unsafe extern "C" fn factory_description(_: *mut c_void, out: *mut ClassDesc, index: i32) -> u8 {
    if !READY.load(Ordering::Acquire) || out.is_null() || index != 0 {
        return 0;
    }
    let Some(values) = strings([
        c"ProgramService",
        c"Fender MCP Native",
        c"",
        c"Bounded DSP metadata and guarded native parameter control",
    ]) else {
        marker("factory_description_string_failed");
        return 0;
    };
    unsafe {
        (*out).flags = 0;
        (*out).cid = SERVICE_CID;
        assign_string(ptr::addr_of_mut!((*out).category), values[0]);
        assign_string(ptr::addr_of_mut!((*out).name), values[1]);
        assign_string(ptr::addr_of_mut!((*out).subcategory), values[2]);
        assign_string(ptr::addr_of_mut!((*out).description), values[3]);
    }
    marker("factory_description");
    1
}

unsafe extern "C" fn factory_attributes(
    _: *mut c_void,
    _: *mut c_void,
    _: *const Uid,
    _: *const c_void,
) -> u8 {
    0
}

#[repr(C)]
struct ServiceVtable {
    base: UnknownVtable,
    initialize: unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32,
    terminate: unsafe extern "C" fn(*mut c_void) -> i32,
    can_terminate: unsafe extern "C" fn(*mut c_void) -> u8,
}

#[repr(C)]
struct Service {
    vtable: *const ServiceVtable,
    refs: AtomicU32,
    initialized: AtomicBool,
    script: native_script::ScriptFacet,
    registered: AtomicBool,
    terminated: AtomicBool,
    initializing: AtomicBool,
    stopping: AtomicBool,
    module_attached: AtomicBool,
    calls: AtomicU32,
    busy: AtomicBool,
    receipt: std::cell::UnsafeCell<Option<native_script::ReceiptLease>>,
    receipt_pending: AtomicBool,
    document_context: std::cell::UnsafeCell<native_script::DocumentContextRegistry>,
    document_context_pending: AtomicBool,
    states: std::cell::UnsafeCell<native_script::StateRegistry>,
    states_pending: AtomicBool,
    presets: std::cell::UnsafeCell<native_script::PresetRegistry>,
    presets_pending: AtomicBool,
    insert_chains: std::cell::UnsafeCell<native_script::InsertChainRegistry>,
    insert_chains_pending: AtomicBool,
}
static SERVICE_VTABLE: ServiceVtable = ServiceVtable {
    base: UnknownVtable {
        query: service_query,
        retain: service_retain,
        release: service_release,
    },
    initialize: service_initialize,
    terminate: service_terminate,
    can_terminate: service_can_terminate,
};

unsafe extern "C" fn factory_create(
    _: *mut c_void,
    cid: *const Uid,
    iid: *const Uid,
    out: *mut *mut c_void,
) -> i32 {
    if out.is_null() {
        return INVALID_POINTER;
    }
    unsafe { *out = ptr::null_mut() };
    if cid.is_null() || iid.is_null() {
        return INVALID_POINTER;
    }
    if unsafe { *cid != SERVICE_CID } {
        return CLASS_NOT_FOUND;
    }
    if unsafe { *iid != UNKNOWN && *iid != COMPONENT } {
        return NO_INTERFACE;
    }
    if !READY.load(Ordering::Acquire) {
        return FAILED;
    }
    let Some(document_context_serial) = native_script::next_document_context_serial() else {
        return FAILED;
    };
    let service = Box::new(Service {
        vtable: &SERVICE_VTABLE,
        refs: AtomicU32::new(1),
        initialized: AtomicBool::new(false),
        script: native_script::ScriptFacet::new(),
        registered: AtomicBool::new(false),
        terminated: AtomicBool::new(false),
        initializing: AtomicBool::new(false),
        stopping: AtomicBool::new(false),
        module_attached: AtomicBool::new(false),
        calls: AtomicU32::new(0),
        busy: AtomicBool::new(false),
        receipt: std::cell::UnsafeCell::new(None),
        receipt_pending: AtomicBool::new(false),
        document_context: std::cell::UnsafeCell::new(native_script::DocumentContextRegistry::new(
            document_context_serial,
        )),
        document_context_pending: AtomicBool::new(false),
        states: std::cell::UnsafeCell::new(native_script::StateRegistry::new()),
        states_pending: AtomicBool::new(false),
        presets: std::cell::UnsafeCell::new(native_script::PresetRegistry::new()),
        presets_pending: AtomicBool::new(false),
        insert_chains: std::cell::UnsafeCell::new(native_script::InsertChainRegistry::new()),
        insert_chains_pending: AtomicBool::new(false),
    });
    let raw = Box::into_raw(service);
    unsafe {
        (*raw).script.owner = raw;
        *out = raw.cast();
    };
    LIVE_SERVICES.fetch_add(1, Ordering::Relaxed);
    marker("service_create");
    OK
}

unsafe extern "C" fn service_query(
    this: *mut c_void,
    iid: *const Uid,
    out: *mut *mut c_void,
) -> i32 {
    if out.is_null() {
        return INVALID_POINTER;
    }
    unsafe { *out = ptr::null_mut() };
    if iid.is_null() {
        return INVALID_POINTER;
    }
    if unsafe { *iid == UNKNOWN || *iid == COMPONENT } {
        unsafe {
            service_retain(this);
            *out = this;
        }
        OK
    } else if unsafe { *iid == native_script::OBJECT } {
        unsafe {
            service_retain(this);
            *out = ptr::addr_of_mut!((*this.cast::<Service>()).script).cast();
        }
        OK
    } else {
        NO_INTERFACE
    }
}

unsafe extern "C" fn service_retain(this: *mut c_void) -> u32 {
    unsafe {
        (*this.cast::<Service>())
            .refs
            .fetch_add(1, Ordering::Relaxed)
            + 1
    }
}

unsafe extern "C" fn service_release(this: *mut c_void) -> u32 {
    let service = unsafe { &*this.cast::<Service>() };
    let mut previous = service.refs.load(Ordering::Acquire);
    loop {
        if previous == 0 {
            marker("service_release_without_reference");
            return 0;
        }
        if previous == 1
            && (service.registered.load(Ordering::Acquire)
                || service.module_attached.load(Ordering::Acquire)
                || service.initializing.load(Ordering::Acquire)
                || service.receipt_pending.load(Ordering::Acquire)
                || service.document_context_pending.load(Ordering::Acquire)
                || service.states_pending.load(Ordering::Acquire)
                || service.presets_pending.load(Ordering::Acquire)
                || service.insert_chains_pending.load(Ordering::Acquire))
        {
            // Registered objects own a separate reference. Reaching this branch
            // violates that invariant; preserve the allocation, never remove
            // a global script object at reference count zero.
            marker("service_destroy_refused_registered_object");
            return 1;
        }
        match service.refs.compare_exchange_weak(
            previous,
            previous - 1,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => break,
            Err(actual) => previous = actual,
        }
    }
    if previous != 1 {
        return previous - 1;
    }
    unsafe { drop(Box::from_raw(this.cast::<Service>())) };
    LIVE_SERVICES.fetch_sub(1, Ordering::Relaxed);
    marker("service_destroy");
    0
}

unsafe extern "C" fn service_initialize(this: *mut c_void, _context: *mut c_void) -> i32 {
    if !READY.load(Ordering::Acquire) {
        return FAILED;
    }
    unsafe { native_script::start(this.cast::<Service>()) }
}

unsafe extern "C" fn service_terminate(this: *mut c_void) -> i32 {
    unsafe { native_script::stop(this.cast::<Service>()) }
}

unsafe extern "C" fn service_can_terminate(_: *mut c_void) -> u8 {
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn CCLModuleMain(module: *mut c_void, reason: i32) -> u8 {
    match reason {
        1 => {
            marker("module_init");
            if module.is_null()
                || constant_string().is_none()
                || !native_script::runtime_supported()
                || !native_script::pin_own_image(module)
            {
                marker("module_native_abi_unavailable");
                return 0;
            }
            MODULE_REF.store(module, Ordering::Release);
            READY.store(true, Ordering::Release);
            marker("module_ready");
            1
        }
        2 => {
            READY.store(false, Ordering::Release);
            let clean = native_script::module_exit_state();
            marker("module_exit");
            u8::from(clean)
        }
        _ => {
            marker("module_unknown_reason_rejected");
            0
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn CCLGetClassFactory() -> *mut c_void {
    marker("factory_requested");
    if !READY.load(Ordering::Acquire) {
        marker("factory_without_ready_module");
        return ptr::null_mut();
    }
    let result = ptr::addr_of!(FACTORY).cast_mut().cast();
    unsafe { factory_retain(result) };
    result
}

// These constants fail the Rust build if the independently declared layouts
// differ from the public-header clang assertions in layout_check.cpp.
const _: () = {
    assert!(size_of::<Uid>() == 16 && align_of::<Uid>() == 4);
    assert!(offset_of!(Uid, d) == 8);
    assert!(size_of::<PlainString>() == 16 && align_of::<PlainString>() == 8);
    assert!(offset_of!(PlainString, opaque_debug) == 8);
    assert!(size_of::<VersionDesc>() == 80 && align_of::<VersionDesc>() == 8);
    assert!(offset_of!(VersionDesc, name) == 0 && offset_of!(VersionDesc, version) == 16);
    assert!(
        offset_of!(VersionDesc, vendor) == 32
            && offset_of!(VersionDesc, copyright) == 48
            && offset_of!(VersionDesc, url) == 64
    );
    assert!(size_of::<ClassDesc>() == 88 && align_of::<ClassDesc>() == 8);
    assert!(offset_of!(ClassDesc, cid) == 4 && offset_of!(ClassDesc, category) == 24);
    assert!(
        offset_of!(ClassDesc, name) == 40
            && offset_of!(ClassDesc, subcategory) == 56
            && offset_of!(ClassDesc, description) == 72
    );
    assert!(size_of::<UnknownVtable>() == 24 && size_of::<FactoryVtable>() == 64);
    assert!(size_of::<ServiceVtable>() == 48);
    assert!(offset_of!(Factory, vtable) == 0 && offset_of!(Service, vtable) == 0);
};

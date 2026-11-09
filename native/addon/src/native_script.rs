//! Independently authored synchronous CCL5.1.2 script object and lifecycle.
use super::*;

const WRONG_THREAD: i32 = 0x8001_010e_u32 as i32;

const NODE: Uid = Uid {
    a: 0xdd45c0c2,
    b: 0x0903,
    c: 0x4648,
    d: [0x9a, 0xa0, 0xcb, 0x89, 0xe5, 0xa6, 0xcd, 2],
};
const CONTROLLER: Uid = Uid {
    a: 0x46ee5c3a,
    b: 0x6a49,
    c: 0x4ebc,
    d: [0xa0, 0x74, 0x4f, 0xc4, 0xc5, 0x92, 0xee, 0xd9],
};
pub(super) const OBJECT: Uid = Uid {
    a: 0x2e726012,
    b: 0x522c,
    c: 0x4108,
    d: [0x98, 0x62, 0x0c, 0xd5, 0x71, 0x79, 0x52, 0x49],
};
const ALIAS: Uid = Uid {
    a: 0x3c288d77,
    b: 0x4cd1,
    c: 0x4481,
    d: [0x86, 0x64, 0x7f, 0x42, 0xf6, 0x9b, 0x57, 0xd4],
};
#[repr(C)]
#[derive(Clone, Copy)]
struct PlainCString {
    text: *const c_char,
    owner: *mut c_void,
}

type Singleton = unsafe extern "C" fn() -> *mut c_void;
type CStringLookup = unsafe extern "C" fn(*const c_char) -> *const PlainCString;
type ThreadSelf = unsafe extern "C" fn() -> i64;
type Count = unsafe extern "C" fn(*mut c_void) -> i32;
type Borrowed = unsafe extern "C" fn(*mut c_void) -> *mut c_void;

#[link(name = "System")]
unsafe extern "C" {
    fn pthread_main_np() -> i32;
    fn dladdr(address: *const c_void, info: *mut DlInfo) -> i32;
    fn _dyld_get_image_header(index: u32) -> *const c_void;
}

#[repr(C)]
struct DlInfo {
    file: *const c_char,
    base: *mut c_void,
    symbol: *const c_char,
    address: *mut c_void,
}
const EXPECTED_GUI_UUID: [u8; 16] = [
    0x1a, 0x8c, 0x66, 0xae, 0x25, 0x0d, 0x3c, 0x7f, 0x80, 0x1c, 0x69, 0x04, 0xd5, 0x08, 0x2f, 0x57,
];
const EXPECTED_APP_UUID: [u8; 16] = [
    0x6b, 0x54, 0x73, 0x78, 0xb3, 0xbb, 0x32, 0x2a, 0x87, 0xc9, 0x90, 0x14, 0x93, 0xfd, 0x2b, 0xed,
];
fn supported_ids(gui: Option<[u8; 16]>, app: Option<[u8; 16]>) -> bool {
    gui == Some(EXPECTED_GUI_UUID) && app == Some(EXPECTED_APP_UUID)
}

// Read loader-owned image metadata, never an arbitrary caller address.
fn image_uuid(base: *const c_void) -> Option<[u8; 16]> {
    if base.is_null() {
        return None;
    }
    let base = base.cast::<u8>();
    let at = |offset: usize| unsafe { ptr::read_unaligned(base.add(offset).cast::<u32>()) };
    if at(0) != 0xfeed_facf {
        return None;
    }
    let commands = at(16) as usize;
    let bytes = at(20) as usize;
    if commands > 256 || bytes > 65536 {
        return None;
    }
    let mut offset = 32;
    for _ in 0..commands {
        if offset + 8 > 32 + bytes {
            return None;
        }
        let command = at(offset);
        let size = at(offset + 4) as usize;
        if size < 8 || offset + size > 32 + bytes {
            return None;
        }
        if command == 0x1b {
            if size != 24 {
                return None;
            }
            let mut actual = [0; 16];
            unsafe { ptr::copy_nonoverlapping(base.add(offset + 8), actual.as_mut_ptr(), 16) };
            return Some(actual);
        }
        offset += size;
    }
    None
}

// Read loader-owned Mach-O metadata only; no version-sensitive interface calls.
pub(super) fn runtime_supported() -> bool {
    let Some(a) = api() else {
        return false;
    };
    let mut info = DlInfo {
        file: ptr::null(),
        base: ptr::null_mut(),
        symbol: ptr::null(),
        address: ptr::null_mut(),
    };
    if unsafe { dladdr(a.gui as *const c_void, &mut info) } == 0 {
        return false;
    }
    let gui = image_uuid(info.base);
    let app = image_uuid(unsafe { _dyld_get_image_header(0) });
    let matched = supported_ids(gui, app);
    log_detail(
        "runtime_abi_guard",
        &format!(
            "{{\"matched\":{matched},\"cclgui_matched\":{},\"app_matched\":{},\"header_commit\":\"ad5876b1a7b64aa247f4378c86a7862e7c284ba3\",\"mode\":{}}}",
            gui == Some(EXPECTED_GUI_UUID),
            app == Some(EXPECTED_APP_UUID),
            quoted("native_script_api")
        ),
    );
    matched
}

struct Api {
    gui: Singleton,
    objects: Singleton,
    main_thread: Singleton,
    thread_self: ThreadSelf,
    cstring: CStringLookup,
    empty: Singleton,
    desktop: Singleton,
    scripting: Singleton,
    plugin_manager: Singleton,
    file_system: Singleton,
}
static API: OnceLock<Option<Api>> = OnceLock::new();
fn api() -> Option<&'static Api> {
    API.get_or_init(|| {
        macro_rules! sym {
            ($name:literal,$ty:ty) => {{
                let p = unsafe { dlsym((-2_isize) as *mut c_void, $name.as_ptr()) };
                if p.is_null() {
                    return None;
                }
                unsafe { std::mem::transmute::<*mut c_void, $ty>(p) }
            }};
        }
        Some(Api {
            gui: sym!(c"CCLGetGUI", Singleton),
            objects: sym!(c"CCLGetObjectTable", Singleton),
            main_thread: sym!(c"CCLGetMainThread", Singleton),
            thread_self: sym!(c"CCLGetThreadSelfID", ThreadSelf),
            cstring: sym!(c"CCLGetConstantCString", CStringLookup),
            empty: sym!(c"CCLGetEmptyString", Singleton),
            desktop: sym!(c"CCLGetDesktop", Singleton),
            scripting: sym!(c"CCLGetScriptingManager", Singleton),
            plugin_manager: sym!(c"CCLGetPlugInManager", Singleton),
            file_system: sym!(c"CCLGetFileSystem", Singleton),
        })
    })
    .as_ref()
}
unsafe fn method<T: Copy>(object: *mut c_void, slot: usize) -> T {
    let table = unsafe { *object.cast::<*const *const c_void>() };
    let code = unsafe { *table.add(slot) };
    unsafe { std::mem::transmute_copy::<*const c_void, T>(&code) }
}
fn on_main(a: &Api) -> bool {
    if cfg!(test) {
        return true;
    }
    if unsafe { pthread_main_np() } == 0 {
        return false;
    }
    let main = unsafe { (a.main_thread)() };
    if main.is_null() {
        return false;
    }
    let id: unsafe extern "C" fn(*mut c_void) -> i64 = unsafe { method(main, 3) };
    unsafe { id(main) == (a.thread_self)() }
}

struct Owned(*mut c_void);
impl Drop for Owned {
    fn drop(&mut self) {
        unsafe { (unknown_vtable(self.0).release)(self.0) };
    }
}
fn hold(object: *mut c_void) -> Option<Owned> {
    if object.is_null() {
        return None;
    }
    unsafe { (unknown_vtable(object).retain)(object) };
    Some(Owned(object))
}
fn query(object: *mut c_void, iid: &Uid) -> Option<Owned> {
    if object.is_null() {
        return None;
    }
    let mut out = ptr::null_mut();
    let result = unsafe { (unknown_vtable(object).query)(object, iid, &mut out) };
    if result == OK && !out.is_null() {
        Some(Owned(out))
    } else {
        None
    }
}

fn quoted(value: &str) -> String {
    let mut out = String::from("\"");
    for c in value.chars().take(512) {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < ' ' => {
                use std::fmt::Write;
                let _ = write!(&mut out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
fn ascii(pointer: *const c_char) -> String {
    if pointer.is_null() {
        return String::new();
    }
    let mut bytes = Vec::new();
    for index in 0..512 {
        let byte = unsafe { *pointer.add(index) } as u8;
        if byte == 0 {
            break;
        }
        bytes.push(byte);
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

fn uid(pointer: *const Uid) -> String {
    if pointer.is_null() {
        return String::new();
    }
    let u = unsafe { *pointer };
    format!(
        "{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
        u.a, u.b, u.c, u.d[0], u.d[1], u.d[2], u.d[3], u.d[4], u.d[5], u.d[6], u.d[7]
    )
}
fn class_name(object: *mut c_void) -> String {
    let Some(o) = query(object, &OBJECT) else {
        return String::new();
    };
    let info: Borrowed = unsafe { method(o.0, 3) };
    let info = unsafe { info(o.0) };
    if info.is_null() {
        return String::new();
    }
    let name: unsafe extern "C" fn(*mut c_void) -> *const c_char = unsafe { method(info, 5) };
    ascii(unsafe { name(info) })
}

const TYPE_INFO: Uid = Uid {
    a: 0xaa0ad2d0,
    b: 0x65da,
    c: 0x4d7e,
    d: [0xb0, 0x63, 0x6d, 0x29, 0x8f, 0xb3, 0xda, 0x4b],
};
const API_CLASS: Uid = Uid {
    a: 0xfa2bc22d,
    b: 0x07c6,
    c: 0x40a1,
    d: [0x8c, 0xa0, 0x77, 0x1f, 0xda, 0x61, 0x3d, 0x08],
};
const GLOBAL_NAME: &std::ffi::CStr = c"FenderMcpNative";
const SELF_PIN_FLAGS: i32 = 0x10 | 0x80; // Darwin dlfcn.h: NOLOAD|NODELETE, compiler asserted.
static SELF_PIN: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static BUNDLE_PIN: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
// The fixed global object name supports one active/pending service. Reserving
// ownership rejects duplicate startup for the same module, including reentry.
static MODULE_OWNER: AtomicPtr<Service> = AtomicPtr::new(ptr::null_mut());
static LIVE_RECEIPTS: AtomicU32 = AtomicU32::new(0);
pub(super) type ReceiptLease = request::ReceiptLease;
pub(super) type StateRegistry = request::state::Registry;
pub(super) type PresetRegistry = request::presets::Registry;
fn release_receipt(service: *mut Service) {
    let s = unsafe { &*service };
    // GUI-only, and take the Rust slot before any native destructor can reenter.
    let old = unsafe { (&mut *s.receipt.get()).take() };
    s.receipt_pending.store(false, Ordering::Release);
    if let Some(old) = old {
        LIVE_RECEIPTS.fetch_sub(1, Ordering::Relaxed);
        drop(old);
    }
}
fn retain_receipt(service: *mut Service, receipt: ReceiptLease) {
    let s = unsafe { &*service };
    debug_assert!(!s.receipt_pending.load(Ordering::Acquire));
    unsafe {
        *s.receipt.get() = Some(receipt);
    }
    s.receipt_pending.store(true, Ordering::Release);
    LIVE_RECEIPTS.fetch_add(1, Ordering::Relaxed);
}
const MODULE_ID: &std::ffi::CStr = c"fender-studio-pro-mcp.native";
#[link(name = "System")]
unsafe extern "C" {
    fn dlopen(path: *const c_char, mode: i32) -> *mut c_void;
}
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFStringCreateWithCharacters(
        allocator: *const c_void,
        chars: *const u16,
        count: isize,
    ) -> *mut c_void;
    fn CFBundleGetBundleWithIdentifier(identifier: *const c_void) -> *mut c_void;
    fn CFGetTypeID(object: *const c_void) -> usize;
    fn CFBundleGetTypeID() -> usize;
    fn CFRetain(object: *const c_void) -> *const c_void;
    fn CFRelease(object: *const c_void);
}

fn pin_with(
    path: *const c_char,
    open: unsafe extern "C" fn(*const c_char, i32) -> *mut c_void,
) -> *mut c_void {
    if path.is_null() || unsafe { *path } != b'/' as c_char {
        return ptr::null_mut();
    }
    // Exact loader-owned path only, never a caller-selected path or fallback.
    unsafe { open(path, SELF_PIN_FLAGS) }
}
fn bundle_matches(module: *mut c_void, bundle: *mut c_void, type_matches: bool) -> bool {
    !module.is_null() && !bundle.is_null() && module == bundle && type_matches
}
pub(super) fn pin_own_image(module: *mut c_void) -> bool {
    if !SELF_PIN.load(Ordering::Acquire).is_null() {
        return !module.is_null() && module == BUNDLE_PIN.load(Ordering::Acquire);
    }
    if module.is_null() {
        return false;
    }
    // Obtain a known-valid OWN bundle before inspecting a CF type. An opaque
    // incoming ModuleRef is never passed directly to CFGetTypeID.
    let units: Vec<u16> = "dev.universe.fender-mcp.native".encode_utf16().collect();
    let identifier =
        unsafe { CFStringCreateWithCharacters(ptr::null(), units.as_ptr(), units.len() as isize) };
    if identifier.is_null() {
        return false;
    }
    let bundle = unsafe { CFBundleGetBundleWithIdentifier(identifier) };
    unsafe { CFRelease(identifier) };
    let correct_type = !bundle.is_null() && unsafe { CFGetTypeID(bundle) == CFBundleGetTypeID() };
    log_detail(
        "own_bundle_guard",
        &format!(
            "{{\"found\":{},\"module_matches\":{},\"type_matches\":{correct_type}}}",
            !bundle.is_null(),
            bundle == module
        ),
    );
    if !bundle_matches(module, bundle, correct_type) {
        return false;
    }
    let mut info = DlInfo {
        file: ptr::null(),
        base: ptr::null_mut(),
        symbol: ptr::null(),
        address: ptr::null_mut(),
    };
    let mut local = DlInfo {
        file: ptr::null(),
        base: ptr::null_mut(),
        symbol: ptr::null(),
        address: ptr::null_mut(),
    };
    if unsafe { dladdr(CCLGetClassFactory as *const c_void, &mut info) } == 0
        || unsafe { dladdr(script_invoke as *const c_void, &mut local) } == 0
        || info.base.is_null()
        || info.base != local.base
    {
        marker("own_image_identity_failed");
        return false;
    }
    let pin = pin_with(info.file, dlopen);
    if pin.is_null() {
        marker("own_image_pin_failed");
        return false;
    }
    unsafe { CFRetain(bundle) };
    BUNDLE_PIN.store(bundle, Ordering::Release);
    SELF_PIN.store(pin, Ordering::Release);
    marker("own_code_and_bundle_pinned_until_process_exit");
    true
}

#[repr(C)]
struct VariantOut {
    kind: i16,
    padding: [u8; 6],
    payload: u64,
}
#[repr(C)]
struct MethodDefinition {
    name: *const c_char,
    args: *const c_char,
    result: *const c_char,
}
// Immutable pointers into static strings; no cross-thread native calls.
unsafe impl Sync for MethodDefinition {}
static METHODS: [MethodDefinition; 3] = [
    MethodDefinition {
        name: c"capabilities".as_ptr(),
        args: c"".as_ptr(),
        result: c"string".as_ptr(),
    },
    MethodDefinition {
        name: c"request".as_ptr(),
        args: c"target: Object, request: string, auxiliary: Object = null".as_ptr(),
        result: c"string".as_ptr(),
    },
    MethodDefinition {
        name: ptr::null(),
        args: ptr::null(),
        result: ptr::null(),
    },
];
#[repr(C)]
struct TypeVtable {
    base: UnknownVtable,
    parent: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
    flags: unsafe extern "C" fn(*mut c_void) -> i32,
    name: unsafe extern "C" fn(*mut c_void) -> *const c_char,
    namespace: unsafe extern "C" fn(*mut c_void) -> *const c_char,
    class: unsafe extern "C" fn(*mut c_void) -> *const Uid,
    methods: unsafe extern "C" fn(*mut c_void) -> *const MethodDefinition,
    properties: unsafe extern "C" fn(*mut c_void) -> *const c_void,
    module: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
    create: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
}
#[repr(C)]
struct TypeObject {
    vtable: *const TypeVtable,
    refs: AtomicU32,
}
unsafe impl Sync for TypeObject {}
static TYPE_TABLE: TypeVtable = TypeVtable {
    base: UnknownVtable {
        query: type_query,
        retain: type_retain,
        release: type_release,
    },
    parent: type_null,
    flags: type_flags,
    name: type_name,
    namespace: type_namespace,
    class: type_class,
    methods: type_methods,
    properties: type_properties,
    module: type_module,
    create: type_null,
};
static TYPE: TypeObject = TypeObject {
    vtable: &TYPE_TABLE,
    refs: AtomicU32::new(1),
};
unsafe extern "C" fn type_query(this: *mut c_void, iid: *const Uid, out: *mut *mut c_void) -> i32 {
    if out.is_null() {
        return INVALID_POINTER;
    }
    unsafe { *out = ptr::null_mut() };
    if iid.is_null() {
        return INVALID_POINTER;
    }
    if unsafe { *iid == UNKNOWN || *iid == TYPE_INFO } {
        unsafe {
            type_retain(this);
            *out = this;
        }
        OK
    } else {
        NO_INTERFACE
    }
}
unsafe extern "C" fn type_retain(_: *mut c_void) -> u32 {
    TYPE.refs.fetch_add(1, Ordering::Relaxed) + 1
}
unsafe extern "C" fn type_release(_: *mut c_void) -> u32 {
    let mut old = TYPE.refs.load(Ordering::Acquire);
    loop {
        // One immortal static baseline; callers own only additional references.
        if old <= 1 {
            return 1;
        }
        match TYPE
            .refs
            .compare_exchange_weak(old, old - 1, Ordering::AcqRel, Ordering::Acquire)
        {
            Ok(_) => return old - 1,
            Err(actual) => old = actual,
        }
    }
}
unsafe extern "C" fn type_null(_: *mut c_void) -> *mut c_void {
    ptr::null_mut()
}
unsafe extern "C" fn type_flags(_: *mut c_void) -> i32 {
    2
}
unsafe extern "C" fn type_name(_: *mut c_void) -> *const c_char {
    c"FenderMcpNative".as_ptr()
}
unsafe extern "C" fn type_namespace(_: *mut c_void) -> *const c_char {
    c"FenderMcp".as_ptr()
}
unsafe extern "C" fn type_class(_: *mut c_void) -> *const Uid {
    &API_CLASS
}
unsafe extern "C" fn type_methods(_: *mut c_void) -> *const MethodDefinition {
    METHODS.as_ptr()
}
unsafe extern "C" fn type_properties(_: *mut c_void) -> *const c_void {
    ptr::null()
}
unsafe extern "C" fn type_module(_: *mut c_void) -> *mut c_void {
    MODULE_REF.load(Ordering::Acquire)
}

#[repr(C)]
struct ObjectVtable {
    base: UnknownVtable,
    info: unsafe extern "C" fn(*mut c_void) -> *const TypeObject,
    get: unsafe extern "C" fn(*mut c_void, *mut VariantOut, *const PlainCString) -> u8,
    set: unsafe extern "C" fn(*mut c_void, *const PlainCString, *const VariantOut) -> u8,
    names: unsafe extern "C" fn(*mut c_void, *mut c_void) -> u8,
    invoke: unsafe extern "C" fn(*mut c_void, *mut VariantOut, *mut c_void) -> u8,
}
#[repr(C)]
pub(super) struct ScriptFacet {
    vtable: *const ObjectVtable,
    pub(super) owner: *mut Service,
}
impl ScriptFacet {
    pub(super) fn new() -> Self {
        Self {
            vtable: &OBJECT_TABLE,
            owner: ptr::null_mut(),
        }
    }
}
static OBJECT_TABLE: ObjectVtable = ObjectVtable {
    base: UnknownVtable {
        query: script_query,
        retain: script_retain,
        release: script_release,
    },
    info: script_info,
    get: script_get,
    set: script_set,
    names: script_names,
    invoke: script_invoke,
};
unsafe extern "C" fn script_query(
    this: *mut c_void,
    iid: *const Uid,
    out: *mut *mut c_void,
) -> i32 {
    let owner = unsafe { (*this.cast::<ScriptFacet>()).owner };
    unsafe { service_query(owner.cast(), iid, out) }
}
unsafe extern "C" fn script_retain(this: *mut c_void) -> u32 {
    unsafe { service_retain((*this.cast::<ScriptFacet>()).owner.cast()) }
}
unsafe extern "C" fn script_release(this: *mut c_void) -> u32 {
    unsafe { service_release((*this.cast::<ScriptFacet>()).owner.cast()) }
}
unsafe extern "C" fn script_info(_: *mut c_void) -> *const TypeObject {
    &TYPE
}
unsafe extern "C" fn script_get(_: *mut c_void, _: *mut VariantOut, _: *const PlainCString) -> u8 {
    0
}
unsafe extern "C" fn script_set(
    _: *mut c_void,
    _: *const PlainCString,
    _: *const VariantOut,
) -> u8 {
    0
}
unsafe extern "C" fn script_names(_: *mut c_void, _: *mut c_void) -> u8 {
    0
}

unsafe fn clear_output(out: *mut VariantOut) {
    let previous = unsafe {
        ptr::replace(
            out,
            VariantOut {
                kind: 0,
                padding: [0; 6],
                payload: 0,
            },
        )
    };
    if previous.kind & 16 != 0 && matches!(previous.kind & 15, 3 | 4) && previous.payload != 0 {
        let owner = previous.payload as *mut c_void;
        unsafe { (unknown_vtable(owner).release)(owner) };
    }
}
unsafe fn install_string(out: *mut VariantOut, string: Owned) {
    // Plain stores only. All native output preparation/old-owner cleanup has
    // already completed before the final request guard.
    unsafe {
        *out = VariantOut {
            kind: 3 | 16,
            padding: [0; 6],
            payload: string.0 as u64,
        };
    }
    std::mem::forget(string);
}
fn prepare_json(text: &str, a: &Api) -> Option<Owned> {
    if text.len() > request::MAX_RESPONSE_BYTES {
        return None;
    }
    let empty = unsafe { (a.empty)() };
    if empty.is_null() {
        return None;
    }
    let clone: Borrowed = unsafe { method(empty, 18) };
    let raw = unsafe { clone(empty) };
    if raw.is_null() {
        return None;
    }
    let string = Owned(raw);
    let chars: Vec<u16> = text.encode_utf16().collect();
    let assign: unsafe extern "C" fn(*mut c_void, *const u16, i32) -> i32 =
        unsafe { method(string.0, 24) };
    if unsafe { assign(string.0, chars.as_ptr(), chars.len() as i32) } != OK {
        return None;
    }
    Some(string)
}
#[cfg(test)]
fn return_json(out: *mut VariantOut, text: &str, a: &Api) -> bool {
    if out.is_null() {
        return false;
    }
    let Some(string) = prepare_json(text, a) else {
        return false;
    };
    unsafe {
        clear_output(out);
        install_string(out, string);
    }
    true
}
fn publish_reply(
    out: *mut VariantOut,
    mut reply: request::Reply,
    a: &Api,
    service: *mut Service,
) -> bool {
    let Some(string) = prepare_json(&reply.encode(), a) else {
        return false;
    };
    // Release an existing shared output before the last guard, because release
    // can invoke host callbacks. Failure before this point preserves old output.
    unsafe {
        clear_output(out);
    }
    if reply.final_check(service) {
        if let Some(receipt) = reply.take_receipt() {
            retain_receipt(service, receipt);
        }
        unsafe {
            install_string(out, string);
        }
        true
    } else {
        // The reply is already downgraded to a non-success outcome. Release
        // invalidated pins before preparing that error; no verified claim can
        // survive either destructor or another output allocation callback.
        drop(string);
        reply.release_context();
        let Some(error) = prepare_json(&reply.encode(), a) else {
            return false;
        };
        unsafe {
            install_string(out, error);
        }
        true
    }
}

unsafe extern "C" fn script_invoke(
    this: *mut c_void,
    out: *mut VariantOut,
    msg: *mut c_void,
) -> u8 {
    if this.is_null() || out.is_null() || msg.is_null() {
        return 0;
    }
    let owner = unsafe { (*this.cast::<ScriptFacet>()).owner };
    let _pin = hold(owner.cast());
    let service = unsafe { &*owner };
    // Per-service gate remains false for old retained JS proxies after stop,
    // even if a new service later sets the module's global READY again.
    if !READY.load(Ordering::Acquire)
        || service.terminated.load(Ordering::Acquire)
        || !service.initialized.load(Ordering::Acquire)
        || !service.registered.load(Ordering::Acquire)
    {
        return 0;
    }
    let Some(a) = api() else {
        return 0;
    };
    if !on_main(a) {
        return 0;
    }
    let get_id: unsafe extern "C" fn(*mut c_void) -> *const PlainCString =
        unsafe { method(msg, 3) };
    let id = unsafe { get_id(msg) };
    if id.is_null() {
        return 0;
    }
    let name = ascii(unsafe { (*id).text });
    if name != "capabilities" && name != "request" {
        return 0;
    }
    if service.busy.swap(true, Ordering::AcqRel) {
        return 0;
    }
    let _busy = CallFlag(&service.busy);
    service.calls.fetch_add(1, Ordering::Relaxed);
    let count: Count = unsafe { method(msg, 4) };
    let count = unsafe { count(msg) };
    let reply = if name == "capabilities" && count == 0 {
        request::Reply::unbound(
            serde_json::json!({"ok":true,"api":"fender-mcp-native-v1","max_page_size":128,"max_request_bytes":16384,"max_response_bytes":262144,"abi":"ccl-5.1.2-fsp-8.1.1-arm64","exact_float_bits":true,"state_api":"component_alias_storable_v1","state_limits":request::state::limits(),"preset_api":"guarded_slot_catalog_v1","preset_limits":request::presets::limits(),"auxiliary_type_observation":true}),
        )
    } else if name == "request" && matches!(count, 2 | 3) {
        let arg: unsafe extern "C" fn(*mut c_void, i32) -> *const VariantOut =
            unsafe { method(msg, 5) };
        let target = unsafe { arg(msg, 0) };
        let json = unsafe { arg(msg, 1) };
        let auxiliary = if count == 3 {
            unsafe { arg(msg, 2) }
        } else {
            ptr::null()
        };
        if target.is_null()
            || json.is_null()
            || unsafe {
                (*target).kind & 15 != 4
                    || (*target).payload == 0
                    || (*json).kind & 15 != 3
                    || (*json).payload == 0
            }
            || (count == 3
                && (auxiliary.is_null()
                    || unsafe { (*auxiliary).kind & 15 != 4 || (*auxiliary).payload == 0 }))
        {
            request::Reply::error(
                "invalid_arguments",
                "request requires one native object and one JSON string",
                "not_applied",
            )
        } else {
            let target =
                hold(unsafe { (*target).payload as *mut c_void }).expect("nonnull checked");
            let auxiliary = if count == 3 {
                hold(unsafe { (*auxiliary).payload as *mut c_void })
            } else {
                None
            };
            match request::copy_string(
                unsafe { (*json).payload as *mut c_void },
                request::MAX_REQUEST_BYTES,
            ) {
                Ok(json) if json.len() <= request::MAX_REQUEST_BYTES => {
                    request::dispatch(owner, target, &json, auxiliary, a)
                }
                _ => request::Reply::error(
                    "invalid_request",
                    "request is not valid bounded UTF-16/UTF-8 JSON",
                    "not_applied",
                ),
            }
        }
    } else {
        request::Reply::error(
            "invalid_arguments",
            "unsupported method argument count",
            "not_applied",
        )
    };
    let returned = publish_reply(out, reply, a, owner);
    log_detail(
        "script_call",
        &format!(
            "{{\"method\":{},\"argc\":{count},\"returned\":{returned},\"calls\":{}}}",
            quoted(&name),
            service.calls.load(Ordering::Acquire)
        ),
    );
    u8::from(returned)
}

struct CallFlag<'a>(&'a AtomicBool);
impl Drop for CallFlag<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
unsafe fn attach_module(service: *mut Service, a: &Api) -> i32 {
    let s = unsafe { &*service };
    if s.module_attached.load(Ordering::Acquire) {
        return FAILED;
    }
    let module = MODULE_REF.load(Ordering::Acquire);
    let manager = unsafe { (a.scripting)() };
    let name = unsafe { (a.cstring)(MODULE_ID.as_ptr()) };
    if module.is_null() || manager.is_null() || name.is_null() {
        return FAILED;
    }
    if MODULE_OWNER
        .compare_exchange(
            ptr::null_mut(),
            service,
            Ordering::AcqRel,
            Ordering::Acquire,
        )
        .is_err()
    {
        marker("script_module_already_owned");
        return FAILED;
    }
    let startup: unsafe extern "C" fn(
        *mut c_void,
        *const PlainCString,
        *mut c_void,
        *const c_void,
        u8,
    ) = unsafe { method(manager, 8) };
    // The owner reservation and start's pin survive a reentrant stop. Do not
    // claim completion or detach while the native startup call is in flight.
    unsafe { startup(manager, name, module, ptr::null(), 0) };
    s.module_attached.store(true, Ordering::Release);
    marker("script_module_attached_load_false");
    OK
}
unsafe fn detach_module(service: *mut Service, a: &Api) -> i32 {
    let s = unsafe { &*service };
    if !s.module_attached.load(Ordering::Acquire) {
        return OK;
    }
    if MODULE_OWNER.load(Ordering::Acquire) != service {
        marker("script_module_owner_mismatch");
        return FAILED;
    }
    let manager = unsafe { (a.scripting)() };
    if manager.is_null() {
        return FAILED;
    }
    let shutdown: unsafe extern "C" fn(*mut c_void, *mut c_void, u8) =
        unsafe { method(manager, 9) };
    // Keep the global reservation until shutdown returns; nested stop sees no
    // attached lease, and another start cannot duplicate or interrupt detach.
    s.module_attached.store(false, Ordering::Release);
    unsafe { shutdown(manager, MODULE_REF.load(Ordering::Acquire), 0) };
    let cleared = MODULE_OWNER
        .compare_exchange(
            service,
            ptr::null_mut(),
            Ordering::AcqRel,
            Ordering::Acquire,
        )
        .is_ok();
    if !cleared {
        marker("script_module_owner_mismatch");
        return FAILED;
    }
    marker("script_module_detached_unload_false");
    OK
}
pub(super) unsafe fn start(service: *mut Service) -> i32 {
    let _pin = hold(service.cast());
    if !READY.load(Ordering::Acquire) {
        return FAILED;
    }
    let Some(a) = api() else {
        return FAILED;
    };
    if !on_main(a) {
        return WRONG_THREAD;
    }
    let s = unsafe { &*service };
    if s.terminated.load(Ordering::Acquire) {
        return FAILED;
    }
    if s.registered.load(Ordering::Acquire) {
        return OK;
    }
    if s.initializing.swap(true, Ordering::AcqRel) {
        return FAILED;
    }
    let _initializing = CallFlag(&s.initializing);
    s.initialized.store(false, Ordering::Release);
    let attached = unsafe { attach_module(service, a) };
    if attached != OK {
        return attached;
    }
    if s.terminated.load(Ordering::Acquire) || !READY.load(Ordering::Acquire) {
        s.terminated.store(true, Ordering::Release);
        let cleanup = unsafe { detach_module(service, a) };
        return if cleanup == OK { FAILED } else { cleanup };
    }
    let table = unsafe { (a.objects)() };
    let name = unsafe { (a.cstring)(GLOBAL_NAME.as_ptr()) };
    if table.is_null() || name.is_null() {
        let cleanup = unsafe { detach_module(service, a) };
        return if cleanup == OK { FAILED } else { cleanup };
    }
    let register: unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        *const Uid,
        *const PlainCString,
        i32,
    ) -> i32 = unsafe { method(table, 3) };
    unsafe { service_retain(service.cast()) };
    let status = unsafe {
        register(
            table,
            ptr::addr_of_mut!((*service).script).cast(),
            &SERVICE_CID,
            name,
            0,
        )
    };
    if status != OK {
        unsafe { service_release(service.cast()) };
        let cleanup = unsafe { detach_module(service, a) };
        return if cleanup == OK { status } else { cleanup };
    }
    s.registered.store(true, Ordering::Release);
    if s.terminated.load(Ordering::Acquire) || !READY.load(Ordering::Acquire) {
        marker("registration_completed_after_stop");
        let cleanup = unsafe { stop(service) };
        return if cleanup == OK { FAILED } else { cleanup };
    }
    s.initialized.store(true, Ordering::Release);
    marker("script_object_registered");
    OK
}
pub(super) unsafe fn stop(service: *mut Service) -> i32 {
    let _pin = hold(service.cast());
    let s = unsafe { &*service };
    s.initialized.store(false, Ordering::Release);
    s.terminated.store(true, Ordering::Release);
    // Nested stop cannot detach while the outer native unregister or shutdown
    // is still in flight. The outer pinned frame completes paired cleanup.
    if s.stopping.swap(true, Ordering::AcqRel) {
        return OK;
    }
    let _stopping = CallFlag(&s.stopping);
    // Startup/registration may complete after this callback. Their initiating
    // frame owns the reservation/pin and performs rollback once native returns.
    if s.initializing.load(Ordering::Acquire) && !s.registered.load(Ordering::Acquire) {
        marker("service_stop_deferred_initialization");
        return OK;
    }
    if !s.registered.load(Ordering::Acquire) && !s.module_attached.load(Ordering::Acquire) {
        marker("service_terminate");
        return OK;
    }
    let Some(a) = api() else {
        return FAILED;
    };
    if !on_main(a) {
        return WRONG_THREAD;
    }
    release_receipt(service);
    request::state::drain(service);
    request::presets::drain(service);
    if s.registered.load(Ordering::Acquire) {
        let table = unsafe { (a.objects)() };
        if table.is_null() {
            return FAILED;
        }
        let unregister: unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32 =
            unsafe { method(table, 4) };
        s.registered.store(false, Ordering::Release);
        let status = unsafe { unregister(table, ptr::addr_of_mut!((*service).script).cast()) };
        if status != OK {
            s.registered.store(true, Ordering::Release);
            marker("script_unregister_failed");
            return status;
        }
        marker("script_object_unregistered");
        unsafe { service_release(service.cast()) };
    }
    let cleanup = unsafe { detach_module(service, a) };
    if cleanup != OK {
        return cleanup;
    }
    marker("service_terminate");
    OK
}
pub(super) fn module_exit_state() -> bool {
    let clean = LIVE_SERVICES.load(Ordering::Acquire) == 0
        && FACTORY.refs.load(Ordering::Acquire) == 0
        && TYPE.refs.load(Ordering::Acquire) == 1
        && MODULE_OWNER.load(Ordering::Acquire).is_null()
        && LIVE_RECEIPTS.load(Ordering::Acquire) == 0
        && request::state::live_count() == 0
        && request::presets::live_count() == 0;
    log_detail(
        "module_exit_state",
        &format!(
            "{{\"clean\":{clean},\"services\":{},\"factory_refs\":{},\"metadata_owned_refs\":{},\"metadata_static_baseline\":1,\"module_owned\":{},\"image_pin_kept_until_process_exit\":{},\"own_bundle_pin_kept_until_process_exit\":{},\"receipt_leases\":{},\"state_capsules\":{},\"preset_catalogs\":{}}}",
            LIVE_SERVICES.load(Ordering::Acquire),
            FACTORY.refs.load(Ordering::Acquire),
            TYPE.refs.load(Ordering::Acquire).saturating_sub(1),
            !MODULE_OWNER.load(Ordering::Acquire).is_null(),
            !SELF_PIN.load(Ordering::Acquire).is_null(),
            !BUNDLE_PIN.load(Ordering::Acquire).is_null(),
            LIVE_RECEIPTS.load(Ordering::Acquire),
            request::state::live_count(),
            request::presets::live_count()
        ),
    );
    clean
}
const _: () = {
    assert!(
        size_of::<VariantOut>() == 16
            && align_of::<VariantOut>() == 8
            && offset_of!(VariantOut, payload) == 8
    );
    assert!(
        size_of::<MethodDefinition>() == 24
            && offset_of!(MethodDefinition, args) == 8
            && offset_of!(MethodDefinition, result) == 16
    );
    assert!(size_of::<ObjectVtable>() == 64 && offset_of!(ObjectVtable, invoke) == 56);
    assert!(size_of::<TypeVtable>() == 96 && offset_of!(TypeVtable, module) == 80);
};

#[cfg(test)]
#[path = "native_script_tests.rs"]
mod tests;

#[path = "request.rs"]
mod request;

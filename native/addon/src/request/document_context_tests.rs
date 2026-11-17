use super::*;
use std::sync::{Mutex, atomic::AtomicUsize};

#[repr(C)]
struct Mock {
    table: *const *const c_void,
    refs: AtomicU32,
    active: AtomicPtr<c_void>,
    path: AtomicPtr<c_void>,
    title: PlainString,
    units: Mutex<Vec<u16>>,
    last_release: AtomicUsize,
    service: AtomicPtr<Service>,
    expected: AtomicPtr<c_void>,
    allocated: bool,
}

unsafe extern "C" fn query_mock(p: *mut c_void, iid: *const Uid, out: *mut *mut c_void) -> i32 {
    unsafe { *out = ptr::null_mut() };
    if unsafe { *iid } == UNKNOWN {
        unsafe {
            retain(p);
            *out = p;
        }
        OK
    } else {
        NO_INTERFACE
    }
}
unsafe extern "C" fn retain(p: *mut c_void) -> u32 {
    unsafe { &*p.cast::<Mock>() }
        .refs
        .fetch_add(1, Ordering::Relaxed)
        + 1
}
unsafe extern "C" fn release(p: *mut c_void) -> u32 {
    let mock = unsafe { &*p.cast::<Mock>() };
    let left = mock.refs.fetch_sub(1, Ordering::AcqRel) - 1;
    if left == 1 {
        let hook = mock.last_release.swap(0, Ordering::AcqRel);
        if hook != 0 {
            let callback: unsafe extern "C" fn(*mut c_void) = unsafe { std::mem::transmute(hook) };
            unsafe { callback(p) };
        }
    }
    if left == 0 && mock.allocated {
        unsafe { drop(Box::from_raw(p.cast::<Mock>())) };
    }
    left
}
unsafe extern "C" fn active(p: *mut c_void) -> *mut c_void {
    unsafe { &*p.cast::<Mock>() }.active.load(Ordering::Acquire)
}
unsafe extern "C" fn title(p: *mut c_void) -> *const PlainString {
    unsafe { &(*p.cast::<Mock>()).title }
}
unsafe extern "C" fn path(p: *mut c_void) -> *mut c_void {
    unsafe { &*p.cast::<Mock>() }.path.load(Ordering::Acquire)
}
unsafe extern "C" fn get_url(p: *mut c_void, out: *mut PlainString, params: u8) {
    assert_eq!(params, 0);
    let owner = unsafe { &*p.cast::<Mock>() }.path.load(Ordering::Acquire);
    unsafe {
        retain(owner);
        release((*out).object);
        (*out).object = owner;
        (*out).opaque_debug = ptr::null_mut();
    }
}
unsafe extern "C" fn length(p: *mut c_void) -> i32 {
    unsafe { &*p.cast::<Mock>() }.units.lock().unwrap().len() as i32
}
unsafe extern "C" fn copy(p: *mut c_void, out: *mut u16, cap: i32) -> i32 {
    let units = unsafe { &*p.cast::<Mock>() }.units.lock().unwrap();
    if cap <= units.len() as i32 {
        return FAILED;
    }
    unsafe {
        ptr::copy_nonoverlapping(units.as_ptr(), out, units.len());
        *out.add(units.len()) = 0;
    }
    OK
}
unsafe extern "C" fn private(_: *mut c_void) -> *mut c_void {
    ptr::null_mut()
}
static EMPTY: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ASSIGN_MANAGER: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ASSIGN_DOCUMENT: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
unsafe extern "C" fn empty() -> *mut c_void {
    EMPTY.load(Ordering::Acquire)
}
unsafe extern "C" fn null() -> *mut c_void {
    ptr::null_mut()
}
unsafe extern "C" fn thread_id() -> i64 {
    1
}
unsafe extern "C" fn cstring(_: *const c_char) -> *const PlainCString {
    ptr::null()
}
unsafe extern "C" fn clone_string(p: *mut c_void) -> *mut c_void {
    let table = unsafe { &*p.cast::<Mock>() }.table;
    let mut mock = Mock::new(table);
    mock.allocated = true;
    Box::into_raw(Box::new(mock)).cast()
}
unsafe extern "C" fn assign(p: *mut c_void, units: *const u16, n: i32) -> i32 {
    *unsafe { &*p.cast::<Mock>() }.units.lock().unwrap() =
        unsafe { std::slice::from_raw_parts(units, n as usize) }.to_vec();
    let manager = ASSIGN_MANAGER.swap(ptr::null_mut(), Ordering::AcqRel);
    if !manager.is_null() {
        unsafe { &*manager.cast::<Mock>() }
            .active
            .store(ASSIGN_DOCUMENT.load(Ordering::Acquire), Ordering::Release);
    }
    OK
}

impl Mock {
    fn new(table: *const *const c_void) -> Self {
        Self {
            table,
            refs: AtomicU32::new(1),
            active: AtomicPtr::new(ptr::null_mut()),
            path: AtomicPtr::new(ptr::null_mut()),
            title: PlainString {
                object: ptr::null_mut(),
                opaque_debug: ptr::null_mut(),
            },
            units: Mutex::new(Vec::new()),
            last_release: AtomicUsize::new(0),
            service: AtomicPtr::new(ptr::null_mut()),
            expected: AtomicPtr::new(ptr::null_mut()),
            allocated: false,
        }
    }
}

struct Fixture {
    #[allow(
        clippy::vec_box,
        reason = "Native mocks keep stable addresses while the fixture grows"
    )]
    objects: Vec<Box<Mock>>,
    _table: Box<[*const c_void; 25]>,
    _document_table: Box<[*const c_void; 25]>,
    _url_table: Box<[*const c_void; 25]>,
    service: Box<Service>,
    api: &'static Api,
}

impl Fixture {
    fn new() -> Self {
        super::super::super::tests::install_mock_api();
        let mut table = Box::new([ptr::null(); 25]);
        for (i, p) in [
            (0, query_mock as *const c_void),
            (1, retain as *const c_void),
            (2, release as *const c_void),
            (4, length as *const c_void),
            (8, copy as *const c_void),
            (9, active as *const c_void),
            (18, clone_string as *const c_void),
            (21, private as *const c_void),
            (24, assign as *const c_void),
        ] {
            table[i] = p;
        }
        let api = Box::leak(Box::new(Api {
            gui: null,
            objects: null,
            main_thread: null,
            thread_self: thread_id,
            cstring,
            empty,
            desktop: null,
            scripting: null,
            plugin_manager: null,
            file_system: null,
        }));
        let service = Box::new(Service {
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
            document_context: std::cell::UnsafeCell::new(Registry::new(
                next_service_serial().unwrap(),
            )),
            document_context_pending: AtomicBool::new(false),
            states: std::cell::UnsafeCell::new(state::Registry::new()),
            states_pending: AtomicBool::new(false),
            presets: std::cell::UnsafeCell::new(presets::Registry::new()),
            presets_pending: AtomicBool::new(false),
            insert_chains: std::cell::UnsafeCell::new(insert_chain::Registry::new()),
            insert_chains_pending: AtomicBool::new(false),
        });
        let mut document_table = table.clone();
        document_table[3] = title as *const c_void;
        document_table[4] = path as *const c_void;
        let mut url_table = table.clone();
        url_table[8] = get_url as *const c_void;
        let mut fixture = Self {
            objects: Vec::new(),
            _table: table,
            _document_table: document_table,
            _url_table: url_table,
            service,
            api,
        };
        let empty = fixture.object();
        EMPTY.store(empty, Ordering::Release);
        READY.store(true, Ordering::Release);
        fixture
    }
    fn object(&mut self) -> *mut c_void {
        let mut object = Box::new(Mock::new(self._table.as_ptr()));
        let raw = ptr::addr_of_mut!(*object).cast();
        self.objects.push(object);
        raw
    }
    fn document(&mut self, name: &str, url: &str) -> *mut c_void {
        let title_text = self.object();
        let url_text = self.object();
        let url_object = self.object();
        let document = self.object();
        unsafe {
            *(*title_text.cast::<Mock>()).units.lock().unwrap() = name.encode_utf16().collect();
            *(*url_text.cast::<Mock>()).units.lock().unwrap() = url.encode_utf16().collect();
            (*url_object.cast::<Mock>())
                .path
                .store(url_text, Ordering::Release);
            (*url_object.cast::<Mock>()).table = self._url_table.as_ptr();
            (*document.cast::<Mock>())
                .path
                .store(url_object, Ordering::Release);
            (*document.cast::<Mock>()).title.object = title_text;
            (*document.cast::<Mock>()).table = self._document_table.as_ptr();
        }
        document
    }
    fn snapshot(&mut self, doc: *mut c_void) -> (*mut c_void, Snapshot) {
        let manager = self.object();
        unsafe {
            (*manager.cast::<Mock>())
                .active
                .store(doc, Ordering::Release);
        }
        let snapshot = Snapshot::capture(
            ptr::addr_of_mut!(*self.service),
            hold(manager).unwrap(),
            self.api,
        )
        .unwrap();
        (manager, snapshot)
    }
    fn observe(&mut self, manager: *mut c_void) -> Reply {
        observe(
            ptr::addr_of_mut!(*self.service),
            hold(manager).unwrap(),
            self.api,
        )
        .unwrap()
    }
    fn publish(&mut self, reply: Reply) -> Value {
        let mut out = VariantOut {
            kind: 0,
            payload: 0,
        };
        assert!(publish_reply(
            &mut out,
            NativeReply::DocumentContext(reply),
            self.api,
            ptr::addr_of_mut!(*self.service)
        ));
        let result = serde_json::from_str(
            &copy_string(out.payload as *mut c_void, MAX_RESPONSE_BYTES).unwrap(),
        )
        .unwrap();
        unsafe { clear_output(&mut out) };
        result
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let service = ptr::addr_of_mut!(*self.service);
        release_receipt(service);
        drain(service);
        ASSIGN_MANAGER.store(ptr::null_mut(), Ordering::Release);
        for object in &self.objects {
            assert_eq!(object.refs.load(Ordering::Acquire), 1);
        }
        READY.store(false, Ordering::Release);
    }
}

#[test]
fn service_serial_and_generation_never_wrap() {
    let serial = AtomicU64::new(u64::MAX - 1);
    assert_eq!(reserve_serial(&serial), Some(u64::MAX - 1));
    assert_eq!(reserve_serial(&serial), None);
    assert_eq!(reserve_serial(&AtomicU64::new(0)), None);
}

#[test]
fn closed_context_has_stable_token_and_no_document_metadata() {
    let _lock = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut f = Fixture::new();
    let (manager, snapshot) = f.snapshot(ptr::null_mut());
    drop(snapshot);
    let reply = f.observe(manager);
    let first = f.publish(reply);
    assert_eq!(first["open"], false);
    assert_eq!(first["title"], Value::Null);
    assert_eq!(first["fileUrl"], Value::Null);
    let reply = f.observe(manager);
    let second = f.publish(reply);
    assert_eq!(first["token"], second["token"]);
    assert!(first["token"].as_str().unwrap().ends_with(":1"));
    assert_eq!(first.as_object().unwrap().len(), 7);
}

#[test]
fn open_document_token_survives_metadata_changes_and_advances_on_identity_or_close() {
    let _lock = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut f = Fixture::new();
    let a = f.document("한글 🎵", "file:///scratch/a.song");
    let b = f.document("second", "");
    let (manager, snapshot) = f.snapshot(a);
    drop(snapshot);
    let reply = f.observe(manager);
    let first = f.publish(reply);
    assert_eq!(first["title"], "한글 🎵");
    assert_eq!(first["fileUrl"], "file:///scratch/a.song");
    let title = unsafe { (*a.cast::<Mock>()).title.object };
    *unsafe { &*title.cast::<Mock>() }.units.lock().unwrap() = "renamed".encode_utf16().collect();
    let reply = f.observe(manager);
    let second = f.publish(reply);
    assert_eq!(first["token"], second["token"]);
    assert_eq!(second["title"], "renamed");
    unsafe {
        (*manager.cast::<Mock>()).active.store(b, Ordering::Release);
    }
    let reply = f.observe(manager);
    let changed = f.publish(reply);
    assert_ne!(first["token"], changed["token"]);
    assert_eq!(changed["fileUrl"], Value::Null);
    unsafe {
        (*manager.cast::<Mock>())
            .active
            .store(ptr::null_mut(), Ordering::Release);
    }
    let reply = f.observe(manager);
    let closed = f.publish(reply);
    assert_ne!(closed["token"], changed["token"]);
    assert_eq!(closed["open"], false);
    unsafe {
        (*manager.cast::<Mock>()).active.store(a, Ordering::Release);
    }
    let reply = f.observe(manager);
    let reopened = f.publish(reply);
    assert_ne!(first["token"], reopened["token"]);
    assert!(reopened["token"].as_str().unwrap().ends_with(":4"));
}

unsafe extern "C" fn retire_reentry(p: *mut c_void) {
    let mock = unsafe { &*p.cast::<Mock>() };
    let service = mock.service.load(Ordering::Acquire);
    assert!(unsafe { (*service).busy.load(Ordering::Acquire) });
    {
        let registry = unsafe { (*service).document_context.get().as_mut().unwrap() };
        assert_eq!(
            registry.identity.as_ref().unwrap().0,
            mock.expected.load(Ordering::Acquire)
        );
        assert!(registry.observed);
    }
    assert!(unsafe { (*service).document_context_pending.load(Ordering::Acquire) });
    let manager = mock.active.load(Ordering::Acquire);
    unsafe {
        (*manager.cast::<Mock>()).active.store(p, Ordering::Release);
    }
}

#[test]
fn retired_owner_reentry_observes_committed_state_and_invalidates_reply() {
    let _lock = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut f = Fixture::new();
    let a = f.document("a", "");
    let b = f.document("b", "");
    let (manager, snapshot) = f.snapshot(a);
    drop(snapshot);
    let reply = f.observe(manager);
    f.publish(reply);
    unsafe {
        (*a.cast::<Mock>())
            .service
            .store(ptr::addr_of_mut!(*f.service), Ordering::Release);
        (*a.cast::<Mock>()).expected.store(b, Ordering::Release);
        (*a.cast::<Mock>()).active.store(manager, Ordering::Release);
        (*a.cast::<Mock>())
            .last_release
            .store(retire_reentry as *const () as usize, Ordering::Release);
        (*manager.cast::<Mock>()).active.store(b, Ordering::Release);
    }
    let error = observe(ptr::addr_of_mut!(*f.service), hold(manager).unwrap(), f.api)
        .err()
        .unwrap();
    assert!(error.contains("active document changed"));
    let registry = unsafe { &*f.service.document_context.get() };
    assert_eq!(registry.generation, 2);
    assert_eq!(registry.identity.as_ref().unwrap().0, b);
}

unsafe extern "C" fn retire_terminate(p: *mut c_void) {
    let mock = unsafe { &*p.cast::<Mock>() };
    let service = mock.service.load(Ordering::Acquire);
    assert!(unsafe { (*service).busy.load(Ordering::Acquire) });
    unsafe {
        (*service).registered.store(false, Ordering::Release);
    }
    assert_eq!(unsafe { stop(service) }, OK);
}

#[test]
fn termination_from_retired_owner_drains_state_without_publishing_success() {
    let _lock = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut f = Fixture::new();
    let a = f.document("a", "");
    let b = f.document("b", "");
    let (manager, snapshot) = f.snapshot(a);
    drop(snapshot);
    let reply = f.observe(manager);
    f.publish(reply);
    unsafe {
        (*a.cast::<Mock>())
            .service
            .store(ptr::addr_of_mut!(*f.service), Ordering::Release);
        (*a.cast::<Mock>())
            .last_release
            .store(retire_terminate as *const () as usize, Ordering::Release);
        (*manager.cast::<Mock>()).active.store(b, Ordering::Release);
    }
    let error = observe(ptr::addr_of_mut!(*f.service), hold(manager).unwrap(), f.api)
        .err()
        .unwrap();
    assert!(error.contains("no longer active"));
    assert!(!f.service.document_context_pending.load(Ordering::Acquire));
    assert!(
        unsafe { &*f.service.document_context.get() }
            .identity
            .is_none()
    );
}

#[test]
fn json_preparation_reentry_cannot_publish_a_stale_context() {
    let _lock = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut f = Fixture::new();
    let a = f.document("a", "");
    let b = f.document("b", "");
    let (manager, snapshot) = f.snapshot(a);
    drop(snapshot);
    let reply = f.observe(manager);
    ASSIGN_DOCUMENT.store(b, Ordering::Release);
    ASSIGN_MANAGER.store(manager, Ordering::Release);
    let response = f.publish(reply);
    assert_eq!(response["ok"], false);
    assert!(response.get("token").is_none());
    assert_eq!(response["outcome"], "not_applied");
    assert!(!f.service.receipt_pending.load(Ordering::Acquire));
}

unsafe extern "C" fn output_switch(p: *mut c_void) {
    let mock = unsafe { &*p.cast::<Mock>() };
    let manager = mock.active.load(Ordering::Acquire);
    unsafe {
        (*manager.cast::<Mock>())
            .active
            .store(mock.expected.load(Ordering::Acquire), Ordering::Release);
    }
}

#[test]
fn old_output_release_reentry_cannot_publish_a_stale_context() {
    let _lock = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut f = Fixture::new();
    let a = f.document("a", "");
    let b = f.document("b", "");
    let (manager, snapshot) = f.snapshot(a);
    drop(snapshot);
    let reply = f.observe(manager);
    let old = f.object();
    unsafe {
        (*old.cast::<Mock>())
            .active
            .store(manager, Ordering::Release);
        (*old.cast::<Mock>()).expected.store(b, Ordering::Release);
        (*old.cast::<Mock>())
            .last_release
            .store(output_switch as *const () as usize, Ordering::Release);
    }
    std::mem::forget(hold(old).unwrap());
    let mut out = VariantOut {
        kind: 4 | 16,
        payload: old as u64,
    };
    assert!(publish_reply(
        &mut out,
        NativeReply::DocumentContext(reply),
        f.api,
        ptr::addr_of_mut!(*f.service)
    ));
    let response: Value =
        serde_json::from_str(&copy_string(out.payload as *mut c_void, MAX_RESPONSE_BYTES).unwrap())
            .unwrap();
    assert_eq!(response["ok"], false);
    assert!(response.get("token").is_none());
    unsafe { clear_output(&mut out) };
}

#[test]
fn metadata_changes_before_final_publication_are_refused() {
    let _lock = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut f = Fixture::new();
    let doc = f.document("before", "file:///before.song");
    let (manager, snapshot) = f.snapshot(doc);
    drop(snapshot);
    let mut reply = f.observe(manager);
    let text = unsafe { (*doc.cast::<Mock>()).title.object };
    *unsafe { &*text.cast::<Mock>() }.units.lock().unwrap() = "after".encode_utf16().collect();
    assert!(!reply.final_check(ptr::addr_of_mut!(*f.service)));
    assert!(reply.value.get("token").is_none());
}

#[test]
fn string_output_replacement_and_receipt_ownership_balance() {
    let _lock = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut f = Fixture::new();
    let doc = f.document("", "file:///x.song");
    let (manager, snapshot) = f.snapshot(doc);
    drop(snapshot);
    let empty = EMPTY.load(Ordering::Acquire);
    let reply = f.observe(manager);
    let response = f.publish(reply);
    assert_eq!(response["title"], "");
    assert_eq!(
        unsafe { &*empty.cast::<Mock>() }
            .refs
            .load(Ordering::Acquire),
        1
    );
    assert_eq!(
        unsafe { &*doc.cast::<Mock>() }.refs.load(Ordering::Acquire),
        4
    );
    assert!(f.service.receipt_pending.load(Ordering::Acquire));
    release_receipt(ptr::addr_of_mut!(*f.service));
    assert_eq!(
        unsafe { &*doc.cast::<Mock>() }.refs.load(Ordering::Acquire),
        2
    );
    f.service.registered.store(false, Ordering::Release);
    assert_eq!(
        unsafe { service_release(ptr::addr_of_mut!(*f.service).cast()) },
        1
    );
    drain(ptr::addr_of_mut!(*f.service));
    assert_eq!(
        unsafe { &*doc.cast::<Mock>() }.refs.load(Ordering::Acquire),
        1
    );
}

#[test]
fn unavailable_manager_is_an_error_and_generation_exhaustion_keeps_existing_owner() {
    let _lock = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut f = Fixture::new();
    let mut error = dispatch(ptr::addr_of_mut!(*f.service), f.api);
    let error: Value = serde_json::from_str(&error.encode()).unwrap();
    assert_eq!(error["ok"], false);
    assert!(error.get("open").is_none());
    let a = f.document("a", "");
    let b = f.document("b", "");
    let (manager, snapshot) = f.snapshot(a);
    drop(snapshot);
    let reply = f.observe(manager);
    f.publish(reply);
    unsafe {
        (*f.service.document_context.get()).generation = u64::MAX;
        (*manager.cast::<Mock>()).active.store(b, Ordering::Release);
    }
    assert!(
        observe(ptr::addr_of_mut!(*f.service), hold(manager).unwrap(), f.api)
            .err()
            .unwrap()
            .contains("exhausted")
    );
    assert_eq!(
        unsafe { &*f.service.document_context.get() }
            .identity
            .as_ref()
            .unwrap()
            .0,
        a
    );
}

#[test]
fn document_text_rejects_bounds_nul_and_invalid_utf16() {
    let _lock = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut f = Fixture::new();
    let string = f.object();
    for invalid in [
        vec![0],
        vec![0xd800],
        vec![b'a' as u16; MAX_TITLE_UNITS + 1],
    ] {
        *unsafe { &*string.cast::<Mock>() }.units.lock().unwrap() = invalid;
        assert!(text(string, MAX_TITLE_UNITS).is_err());
    }
    *unsafe { &*string.cast::<Mock>() }.units.lock().unwrap() = vec![b'a' as u16; MAX_TITLE_UNITS];
    assert_eq!(
        text(string, MAX_TITLE_UNITS).unwrap().len(),
        MAX_TITLE_UNITS
    );
}

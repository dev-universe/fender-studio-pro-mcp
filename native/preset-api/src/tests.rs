use super::*;
use crate::ffi::{CStringRepr, Uid, Variant};
use std::{
    cell::{Cell, RefCell},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering},
    },
};

struct Table<const N: usize>([*const c_void; N]);
unsafe impl<const N: usize> Sync for Table<N> {}
#[repr(C)]
struct Mock {
    table: *const *const c_void,
    refs: AtomicU32,
    dynamic: bool,
    dynamic_live: Arc<AtomicUsize>,
    text: Vec<u16>,
    repr: StringRepr,
    key: Cell<*mut c_void>,
    target: Cell<*mut c_void>,
    attributes: Cell<*mut c_void>,
    entries: RefCell<Vec<*mut c_void>>,
    position: Cell<usize>,
    preset: bool,
    name: Cell<*mut c_void>,
    protocol: Cell<*mut c_void>,
    host: Cell<*mut c_void>,
    url_id: Cell<u32>,
    has_url: Cell<bool>,
    read_only: Cell<bool>,
    existing: Cell<bool>,
    empty: Cell<bool>,
    absolute: Cell<bool>,
    native: Cell<bool>,
    file_kind: Cell<i32>,
    available: Cell<bool>,
    allowed: Cell<bool>,
    accepted: Cell<bool>,
    inserts: Cell<u32>,
    received: Cell<*mut c_void>,
    alive: Arc<AtomicBool>,
    after_insert_fail: Cell<bool>,
    can_change: Cell<*mut c_void>,
    new_protocol: Cell<*mut c_void>,
    keep_list: Cell<bool>,
    retained_list: Cell<*mut c_void>,
    release_fails_guard: Cell<bool>,
    release_switch: Cell<*mut c_void>,
    switch_target: Cell<*mut c_void>,
    release_switch_after: Cell<usize>,
}
unsafe fn m<'a>(p: *mut c_void) -> &'a Mock {
    unsafe { &*p.cast::<Mock>() }
}
unsafe extern "C" fn retain(p: *mut c_void) -> u32 {
    unsafe { m(p) }.refs.fetch_add(1, Ordering::Relaxed) + 1
}
unsafe extern "C" fn release(p: *mut c_void) -> u32 {
    let o = unsafe { m(p) };
    let left = o.refs.fetch_sub(1, Ordering::AcqRel) - 1;
    if o.release_fails_guard.get() {
        o.alive.store(false, Ordering::Release);
    }
    if !o.release_switch.get().is_null() {
        let remaining = o.release_switch_after.get();
        if remaining > 1 {
            o.release_switch_after.set(remaining - 1);
        } else {
            unsafe { m(o.release_switch.get()) }
                .target
                .set(o.switch_target.get());
        }
    }
    if left == 0 {
        assert!(o.dynamic);
        o.dynamic_live.fetch_sub(1, Ordering::Relaxed);
        drop(unsafe { Box::from_raw(p.cast::<Mock>()) });
    }
    left
}
unsafe extern "C" fn query(p: *mut c_void, iid: *const Uid, out: *mut *mut c_void) -> i32 {
    unsafe { *out = ptr::null_mut() };
    if unsafe { *iid } == ffi::UNKNOWN || (unsafe { *iid } == ffi::PRESET && unsafe { m(p) }.preset)
    {
        unsafe {
            retain(p);
            *out = p
        };
        0
    } else {
        0x80004002u32 as i32
    }
}
fn table() -> [*const c_void; 32] {
    let mut t = [ptr::null(); 32];
    t[0] = query as *const c_void;
    t[1] = retain as *const c_void;
    t[2] = release as *const c_void;
    t
}
fn string_table() -> Table<32> {
    let mut t = table();
    t[4] = length as *const c_void;
    t[8] = copy_text as *const c_void;
    Table(t)
}
fn generic_table() -> Table<32> {
    Table(table())
}
fn allocator_table() -> Table<32> {
    let mut t = table();
    t[3] = allocate as *const c_void;
    Table(t)
}
fn attrs_table() -> Table<32> {
    let mut t = table();
    t[10] = attribute as *const c_void;
    Table(t)
}
fn mediator_table() -> Table<32> {
    let mut t = table();
    t[3] = metadata as *const c_void;
    t[4] = target as *const c_void;
    Table(t)
}
fn manager_table() -> Table<32> {
    let mut t = table();
    t[7] = presets as *const c_void;
    Table(t)
}
fn list_table() -> Table<32> {
    let mut t = table();
    t[3] = iterator as *const c_void;
    Table(t)
}
fn iter_table() -> Table<32> {
    let mut t = table();
    t[3] = done as *const c_void;
    t[4] = next as *const c_void;
    Table(t)
}
fn preset_table() -> Table<32> {
    let mut t = table();
    t[3] = read_only as *const c_void;
    t[5] = preset_name as *const c_void;
    t[6] = preset_meta as *const c_void;
    t[7] = preset_url as *const c_void;
    Table(t)
}
fn url_table() -> Table<32> {
    let mut t = table();
    t[3] = url_clone as *const c_void;
    t[5] = url_equal as *const c_void;
    t[6] = url_empty as *const c_void;
    t[7] = url_kind as *const c_void;
    t[10] = url_protocol as *const c_void;
    t[12] = url_host as *const c_void;
    t[24] = url_native as *const c_void;
    t[31] = url_absolute as *const c_void;
    Table(t)
}
fn fs_table() -> Table<32> {
    let mut t = table();
    t[4] = file_exists as *const c_void;
    Table(t)
}
fn data_table() -> Table<32> {
    let mut t = table();
    t[3] = can_insert as *const c_void;
    t[4] = insert as *const c_void;
    Table(t)
}
unsafe extern "C" fn length(p: *mut c_void) -> i32 {
    unsafe { m(p) }.text.len() as i32
}
unsafe extern "C" fn copy_text(p: *mut c_void, out: *mut u16, size: i32) -> i32 {
    let o = unsafe { m(p) };
    if size < o.text.len() as i32 + 1 {
        return -1;
    }
    unsafe {
        ptr::copy_nonoverlapping(o.text.as_ptr(), out, o.text.len());
        *out.add(o.text.len()) = 0;
    }
    0
}
unsafe extern "C" fn allocate(
    p: *mut c_void,
    cid: *const Uid,
    iid: *const Uid,
    out: *mut *mut c_void,
) -> i32 {
    assert!(unsafe { *cid } == ffi::ATTRIBUTES_CLASS && unsafe { *iid } == ffi::ATTRIBUTES);
    let attrs = unsafe { m(p) }.attributes.get();
    unsafe {
        retain(attrs);
        *out = attrs
    };
    0
}
unsafe extern "C" fn attribute(p: *mut c_void, out: *mut Variant, name: *const CStringRepr) -> u8 {
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr((*name).text) },
        c"Class:ID"
    );
    let key = unsafe { m(p) }.key.get();
    unsafe {
        retain(key);
        (*out).tag = 0x13;
        (*out).payload = key as usize as u64;
    }
    1
}
unsafe extern "C" fn metadata(p: *mut c_void, attrs: *mut c_void) -> u8 {
    unsafe { m(attrs) }.key.set(unsafe { m(p) }.key.get());
    1
}
unsafe extern "C" fn target(p: *mut c_void) -> *mut c_void {
    unsafe { m(p) }.target.get()
}
unsafe extern "C" fn presets(p: *mut c_void, _: *mut c_void, _: *mut c_void) -> *mut c_void {
    let o = unsafe { m(p) };
    if !o.available.get() {
        return ptr::null_mut();
    }
    let list = o.target.get();
    unsafe { retain(list) };
    list
}
unsafe extern "C" fn iterator(p: *mut c_void) -> *mut c_void {
    let o = unsafe { m(p) };
    let mut iter = new(
        unsafe { &*o.attributes.get().cast::<[*const c_void; 32]>() },
        o.alive.clone(),
        o.dynamic_live.clone(),
    );
    iter.dynamic = true;
    *iter.entries.borrow_mut() = o.entries.borrow().clone();
    o.dynamic_live.fetch_add(1, Ordering::Relaxed);
    Box::into_raw(Box::new(iter)).cast()
}
unsafe extern "C" fn done(p: *mut c_void) -> u8 {
    let o = unsafe { m(p) };
    (o.position.get() >= o.entries.borrow().len()) as u8
}
unsafe extern "C" fn next(p: *mut c_void) -> *mut c_void {
    let o = unsafe { m(p) };
    let index = o.position.get();
    o.position.set(index + 1);
    o.entries
        .borrow()
        .get(index)
        .copied()
        .unwrap_or(ptr::null_mut())
}
unsafe extern "C" fn read_only(p: *mut c_void) -> u8 {
    unsafe { m(p) }.read_only.get() as u8
}
unsafe extern "C" fn preset_name(p: *mut c_void) -> *const StringRepr {
    let name = unsafe { m(p) }.name.get();
    unsafe { &m(name).repr }
}
unsafe extern "C" fn preset_meta(p: *mut c_void) -> *mut c_void {
    unsafe { m(p) }.attributes.get()
}
unsafe extern "C" fn preset_url(p: *mut c_void, url: *mut c_void) -> u8 {
    let o = unsafe { m(p) };
    if !o.has_url.get() {
        return 0;
    }
    let u = unsafe { m(url) };
    u.protocol.set(o.protocol.get());
    u.host.set(o.host.get());
    u.url_id.set(o.url_id.get());
    u.empty.set(false);
    u.absolute.set(o.absolute.get());
    u.native.set(o.native.get());
    u.file_kind.set(o.file_kind.get());
    1
}
unsafe extern "C" fn url_clone(p: *mut c_void, out: *mut *mut c_void) {
    let o = unsafe { m(p) };
    let mut clone = new(
        unsafe { &*o.table.cast::<[*const c_void; 32]>() },
        o.alive.clone(),
        o.dynamic_live.clone(),
    );
    clone.dynamic = true;
    clone.empty.set(o.empty.get());
    o.dynamic_live.fetch_add(1, Ordering::Relaxed);
    unsafe { *out = Box::into_raw(Box::new(clone)).cast() };
}
unsafe extern "C" fn url_equal(a: *mut c_void, b: *mut c_void, _: u8) -> u8 {
    let (a, b) = (unsafe { m(a) }, unsafe { m(b) });
    (a.url_id.get() == b.url_id.get()
        && a.protocol.get() == b.protocol.get()
        && a.host.get() == b.host.get()) as u8
}
unsafe extern "C" fn url_empty(p: *mut c_void) -> u8 {
    unsafe { m(p) }.empty.get() as u8
}
unsafe extern "C" fn url_kind(p: *mut c_void) -> i32 {
    unsafe { m(p) }.file_kind.get()
}
unsafe extern "C" fn url_protocol(p: *mut c_void) -> *const StringRepr {
    unsafe { &m(m(p).protocol.get()).repr }
}
unsafe extern "C" fn url_host(p: *mut c_void) -> *const StringRepr {
    unsafe { &m(m(p).host.get()).repr }
}
unsafe extern "C" fn url_native(p: *mut c_void) -> u8 {
    unsafe { m(p) }.native.get() as u8
}
unsafe extern "C" fn url_absolute(p: *mut c_void) -> u8 {
    unsafe { m(p) }.absolute.get() as u8
}
unsafe extern "C" fn file_exists(p: *mut c_void, _: *mut c_void) -> u8 {
    unsafe { m(p) }.existing.get() as u8
}
unsafe extern "C" fn can_insert(
    p: *mut c_void,
    list: *mut c_void,
    _: *mut c_void,
    _: *mut c_void,
    index: i32,
) -> u8 {
    assert_eq!(index, -1);
    let o = unsafe { m(p) };
    let create: unsafe extern "C" fn(*mut c_void) -> *mut c_void = unsafe { ffi::method(list, 3) };
    let iterator = unsafe { Owned::take(NonNull::new(create(list)).unwrap()) };
    let done: unsafe extern "C" fn(*mut c_void) -> u8 = unsafe { ffi::method(iterator.raw(), 3) };
    let next: unsafe extern "C" fn(*mut c_void) -> *mut c_void =
        unsafe { ffi::method(iterator.raw(), 4) };
    assert_eq!(unsafe { done(iterator.raw()) }, 0);
    assert!(!unsafe { next(iterator.raw()) }.is_null());
    assert_eq!(unsafe { done(iterator.raw()) }, 1);
    if o.keep_list.get() {
        let retain: unsafe extern "C" fn(*mut c_void) -> u32 = unsafe { ffi::method(list, 1) };
        unsafe { retain(list) };
        o.retained_list.set(list);
    }
    if !o.can_change.get().is_null() {
        unsafe { m(o.can_change.get()) }
            .protocol
            .set(o.new_protocol.get());
    }
    o.allowed.get() as u8
}
unsafe extern "C" fn insert(p: *mut c_void, list: *mut c_void, _: *mut c_void, index: i32) -> u8 {
    assert_eq!(index, -1);
    let o = unsafe { m(p) };
    let first: unsafe extern "C" fn(*mut c_void) -> *mut c_void = unsafe { ffi::method(list, 5) };
    o.received.set(unsafe { first(list) });
    o.inserts.set(o.inserts.get() + 1);
    if o.after_insert_fail.get() {
        o.alive.store(false, Ordering::Release);
    }
    o.accepted.get() as u8
}
fn new(
    table: &[*const c_void; 32],
    alive: Arc<AtomicBool>,
    dynamic_live: Arc<AtomicUsize>,
) -> Mock {
    Mock {
        table: table.as_ptr(),
        refs: AtomicU32::new(1),
        dynamic: false,
        dynamic_live,
        text: Vec::new(),
        repr: StringRepr {
            object: ptr::null_mut(),
            private: ptr::null_mut(),
        },
        key: Cell::new(ptr::null_mut()),
        target: Cell::new(ptr::null_mut()),
        attributes: Cell::new(ptr::null_mut()),
        entries: RefCell::new(Vec::new()),
        position: Cell::new(0),
        preset: false,
        name: Cell::new(ptr::null_mut()),
        protocol: Cell::new(ptr::null_mut()),
        host: Cell::new(ptr::null_mut()),
        url_id: Cell::new(1),
        has_url: Cell::new(true),
        read_only: Cell::new(true),
        existing: Cell::new(true),
        empty: Cell::new(true),
        absolute: Cell::new(true),
        native: Cell::new(true),
        file_kind: Cell::new(1),
        available: Cell::new(true),
        allowed: Cell::new(true),
        accepted: Cell::new(true),
        inserts: Cell::new(0),
        received: Cell::new(ptr::null_mut()),
        alive,
        after_insert_fail: Cell::new(false),
        can_change: Cell::new(ptr::null_mut()),
        new_protocol: Cell::new(ptr::null_mut()),
        keep_list: Cell::new(false),
        retained_list: Cell::new(ptr::null_mut()),
        release_fails_guard: Cell::new(false),
        release_switch: Cell::new(ptr::null_mut()),
        switch_target: Cell::new(ptr::null_mut()),
        release_switch_after: Cell::new(0),
    }
}
struct Fixture {
    tables: Vec<std::pin::Pin<Box<Table<32>>>>,
    objects: Vec<std::pin::Pin<Box<Mock>>>,
    alive: Arc<AtomicBool>,
    dynamic_live: Arc<AtomicUsize>,
    manager: *mut c_void,
    mediator: *mut c_void,
    data: *mut c_void,
    allocator: *mut c_void,
    url: *mut c_void,
    fs: *mut c_void,
    attrs: *mut c_void,
    presets: Vec<*mut c_void>,
    file: *mut c_void,
    https: *mut c_void,
    other_key: *mut c_void,
    target2: *mut c_void,
}
impl Fixture {
    fn object(&mut self, table: Table<32>) -> *mut c_void {
        self.tables.push(Box::pin(table));
        let o = Box::pin(new(
            &self.tables.last().unwrap().0,
            self.alive.clone(),
            self.dynamic_live.clone(),
        ));
        let p = ptr::from_ref(&*o).cast_mut().cast();
        self.objects.push(o);
        p
    }
    fn string(&mut self, text: &str) -> *mut c_void {
        let p = self.object(string_table());
        let o = self.objects.last_mut().unwrap();
        o.text = text.encode_utf16().collect();
        o.repr.object = p;
        p
    }
    fn new() -> Self {
        let mut f = Self {
            tables: Vec::new(),
            objects: Vec::new(),
            alive: Arc::new(AtomicBool::new(true)),
            dynamic_live: Arc::new(AtomicUsize::new(0)),
            manager: ptr::null_mut(),
            mediator: ptr::null_mut(),
            data: ptr::null_mut(),
            allocator: ptr::null_mut(),
            url: ptr::null_mut(),
            fs: ptr::null_mut(),
            attrs: ptr::null_mut(),
            presets: Vec::new(),
            file: ptr::null_mut(),
            https: ptr::null_mut(),
            other_key: ptr::null_mut(),
            target2: ptr::null_mut(),
        };
        let key = f.string("D9AE9ACD-69B4-4B43-B8D5-983E39C559A5");
        f.other_key = f.string("OTHER-CLASS");
        f.file = f.string("file");
        f.https = f.string("https");
        let host = f.string("");
        let target = f.object(generic_table());
        f.target2 = f.object(generic_table());
        f.attrs = f.object(attrs_table());
        unsafe { m(f.attrs) }.key.set(key);
        f.allocator = f.object(allocator_table());
        unsafe { m(f.allocator) }.attributes.set(f.attrs);
        f.mediator = f.object(mediator_table());
        unsafe { m(f.mediator) }.key.set(key);
        unsafe { m(f.mediator) }.target.set(target);
        f.data = f.object(data_table());
        f.url = f.object(url_table());
        f.fs = f.object(fs_table());
        for (index, name) in ["Factory One", "Factory Two"].iter().enumerate() {
            let name = f.string(name);
            let meta = f.object(attrs_table());
            unsafe { m(meta) }.key.set(key);
            let p = f.object(preset_table());
            let preset = f.objects.last_mut().unwrap();
            preset.preset = true;
            preset.name.set(name);
            preset.attributes.set(meta);
            preset.protocol.set(f.file);
            preset.host.set(host);
            preset.url_id.set(index as u32 + 1);
            f.presets.push(p);
        }
        let list = f.object(list_table());
        *unsafe { m(list) }.entries.borrow_mut() = f.presets.clone();
        let iter = f.object(iter_table());
        unsafe { m(list) }
            .attributes
            .set(unsafe { m(iter) }.table.cast_mut().cast());
        f.manager = f.object(manager_table());
        unsafe { m(f.manager) }.target.set(list);
        f
    }
    fn guard(&self) -> impl FnMut() -> Result<(), String> + use<> {
        let alive = self.alive.clone();
        move || {
            if alive.load(Ordering::Acquire) {
                Ok(())
            } else {
                Err("context changed".into())
            }
        }
    }
    fn catalog(&self) -> Result<Catalog, Error> {
        let own = |p| {
            let o = unsafe { Owned::hold(p) }.unwrap();
            let raw = NonNull::new(o.raw()).unwrap();
            std::mem::forget(o);
            raw
        };
        unsafe {
            Catalog::discover_owned(
                Inputs {
                    manager: own(self.manager),
                    mediator: own(self.mediator),
                    data_target: own(self.data),
                    allocator: own(self.allocator),
                    empty_url: own(self.url),
                    file_system: own(self.fs),
                },
                &mut self.guard(),
            )
        }
    }
    fn balanced(&self) {
        assert_eq!(self.dynamic_live.load(Ordering::Acquire), 0);
        for o in &self.objects {
            assert_eq!(o.refs.load(Ordering::Acquire), 1, "owned native ref leaked");
        }
    }
}

#[test]
fn matching_local_catalog_pages_and_recalls_only_selected_entry() {
    let f = Fixture::new();
    let c = f.catalog().unwrap();
    let p = c.page(0, 1, &mut f.guard()).unwrap();
    assert_eq!(p.entries[0].name, "Factory One");
    assert_eq!(p.next, Some(1));
    assert!(p.catalog_complete);
    assert_eq!(p.entries[0].availability, Availability::LocalReadOnly);
    let r = c.recall(1, &mut f.guard()).unwrap();
    assert_eq!(
        r,
        Recall {
            accepted: true,
            verified: false
        }
    );
    assert_eq!(unsafe { m(f.data) }.received.get(), f.presets[1]);
    drop(c);
    f.balanced();
}
#[test]
fn remote_entries_are_visible_but_never_dispatched() {
    let f = Fixture::new();
    unsafe { m(f.presets[0]) }.protocol.set(f.https);
    let c = f.catalog().unwrap();
    assert_eq!(
        c.page(0, 2, &mut f.guard()).unwrap().entries[0].availability,
        Availability::UnsupportedProtocol("https".into())
    );
    let e = c.recall(0, &mut f.guard()).unwrap_err();
    assert!(!e.dispatched);
    assert_eq!(unsafe { m(f.data) }.inserts.get(), 0);
    drop(c);
    f.balanced();
}
#[test]
fn changed_class_metadata_is_refused_before_dispatch() {
    let f = Fixture::new();
    let c = f.catalog().unwrap();
    unsafe { m(f.mediator) }.key.set(f.other_key);
    assert!(matches!(
        c.recall(0, &mut f.guard()),
        Err(RecallError {
            error: Error::Changed,
            dispatched: false
        })
    ));
    drop(c);
    f.balanced();
}
#[test]
fn same_class_target_replacement_is_refused() {
    let f = Fixture::new();
    let c = f.catalog().unwrap();
    unsafe { m(f.mediator) }.target.set(f.target2);
    assert!(matches!(c.page(0, 2, &mut f.guard()), Err(Error::Changed)));
    assert!(!c.recall(0, &mut f.guard()).unwrap_err().dispatched);
    drop(c);
    f.balanced();
}
#[test]
fn data_target_refusal_does_not_invoke_insert() {
    let f = Fixture::new();
    unsafe { m(f.data) }.allowed.set(false);
    let c = f.catalog().unwrap();
    assert_eq!(
        c.recall(0, &mut f.guard()).unwrap(),
        Recall {
            accepted: false,
            verified: false
        }
    );
    assert_eq!(unsafe { m(f.data) }.inserts.get(), 0);
    drop(c);
    f.balanced();
}
#[test]
fn accepted_native_insert_never_claims_verified_state() {
    let f = Fixture::new();
    let c = f.catalog().unwrap();
    let r = c.recall(0, &mut f.guard()).unwrap();
    assert!(r.accepted);
    assert!(!r.verified);
    drop(c);
    f.balanced();
}
#[test]
fn rejected_native_insert_remains_unverified() {
    let f = Fixture::new();
    unsafe { m(f.data) }.accepted.set(false);
    let c = f.catalog().unwrap();
    assert_eq!(
        c.recall(0, &mut f.guard()).unwrap(),
        Recall {
            accepted: false,
            verified: false
        }
    );
    assert_eq!(unsafe { m(f.data) }.inserts.get(), 1);
    drop(c);
    f.balanced();
}
#[test]
fn context_loss_after_insert_is_unknown_dispatched() {
    let f = Fixture::new();
    let c = f.catalog().unwrap();
    unsafe { m(f.data) }.after_insert_fail.set(true);
    let e = c.recall(0, &mut f.guard()).unwrap_err();
    assert!(e.dispatched);
    assert!(matches!(e.error, Error::Guard(_)));
    drop(c);
    f.balanced();
}
#[test]
fn local_url_changed_during_can_insert_is_refused() {
    let f = Fixture::new();
    let c = f.catalog().unwrap();
    unsafe { m(f.data) }.can_change.set(f.presets[0]);
    unsafe { m(f.data) }.new_protocol.set(f.https);
    let e = c.recall(0, &mut f.guard()).unwrap_err();
    assert!(!e.dispatched);
    assert_eq!(e.error, Error::Changed);
    assert_eq!(unsafe { m(f.data) }.inserts.get(), 0);
    drop(c);
    f.balanced();
}
#[test]
fn no_available_store_is_not_an_empty_complete_catalog() {
    let f = Fixture::new();
    unsafe { m(f.manager) }.available.set(false);
    assert!(matches!(f.catalog(), Err(Error::Unavailable(_))));
    f.balanced();
}
#[test]
fn missing_file_and_mismatched_entry_class_are_not_recallable() {
    let f = Fixture::new();
    unsafe { m(f.fs) }.existing.set(false);
    let c = f.catalog().unwrap();
    assert_eq!(
        c.page(0, 1, &mut f.guard()).unwrap().entries[0].availability,
        Availability::MissingLocalFile
    );
    assert!(!c.recall(0, &mut f.guard()).unwrap_err().dispatched);
    drop(c);
    f.balanced();
}
#[test]
fn bounded_pagination_rejects_invalid_arguments() {
    let f = Fixture::new();
    let c = f.catalog().unwrap();
    for (offset, limit) in [(0, 0), (0, 129), (3, 1)] {
        assert_eq!(
            c.page(offset, limit, &mut f.guard()).unwrap_err(),
            Error::OutOfRange
        );
    }
    assert!(!c.recall(3, &mut f.guard()).unwrap_err().dispatched);
    drop(c);
    f.balanced();
}
#[test]
fn host_retained_single_item_list_owns_its_preset() {
    let f = Fixture::new();
    let c = f.catalog().unwrap();
    unsafe { m(f.data) }.keep_list.set(true);
    c.recall(0, &mut f.guard()).unwrap();
    let pointer = unsafe { m(f.data) }.retained_list.get();
    drop(c);
    assert_eq!(unsafe { m(f.presets[0]) }.refs.load(Ordering::Acquire), 2);
    let list = unsafe { Owned::take(NonNull::new(pointer).unwrap()) };
    drop(list);
    f.balanced();
}
#[test]
fn temporary_release_context_change_prevents_success() {
    let f = Fixture::new();
    let c = f.catalog().unwrap();
    unsafe { m(f.attrs) }.release_fails_guard.set(true);
    assert!(matches!(c.page(0, 1, &mut f.guard()), Err(Error::Guard(_))));
    unsafe { m(f.attrs) }.release_fails_guard.set(false);
    drop(c);
    f.balanced();
}

#[test]
fn foreign_class_entry_is_accounted_and_never_recalled() {
    let f = Fixture::new();
    let attributes = unsafe { m(f.presets[0]) }.attributes.get();
    unsafe { m(attributes) }.key.set(f.other_key);
    let catalog = f.catalog().unwrap();
    assert_eq!(
        catalog.page(0, 1, &mut f.guard()).unwrap().entries[0].availability,
        Availability::ClassMismatch
    );
    assert!(!catalog.recall(0, &mut f.guard()).unwrap_err().dispatched);
    assert_eq!(unsafe { m(f.data) }.inserts.get(), 0);
    drop(catalog);
    f.balanced();
}

#[test]
fn oversized_catalog_stays_explicitly_incomplete() {
    let f = Fixture::new();
    let list = unsafe { m(f.manager) }.target.get();
    *unsafe { m(list) }.entries.borrow_mut() = vec![f.presets[0]; MAX_PRESETS + 1];
    let catalog = f.catalog().unwrap();
    let page = catalog
        .page(MAX_PRESETS as u32 - 1, 1, &mut f.guard())
        .unwrap();
    assert_eq!(page.observed_count, MAX_PRESETS as u32);
    assert!(!page.catalog_complete);
    assert_eq!(page.next, None);
    drop(catalog);
    f.balanced();
}

#[test]
fn target_switch_during_final_entry_cleanup_cannot_dispatch() {
    let f = Fixture::new();
    let catalog = f.catalog().unwrap();
    let text = unsafe { m(f.file) };
    text.release_switch.set(f.mediator);
    text.switch_target.set(f.target2);
    text.release_switch_after.set(2);
    let error = catalog.recall(0, &mut f.guard()).unwrap_err();
    assert_eq!(error.error, Error::Changed);
    assert!(!error.dispatched);
    assert_eq!(unsafe { m(f.data) }.inserts.get(), 0);
    drop(catalog);
    f.balanced();
}

#[test]
fn failed_query_never_claims_or_releases_its_output() {
    #[repr(C)]
    struct Probe {
        table: *const *const c_void,
        releases: Cell<u32>,
        output: Cell<*mut c_void>,
        status: Cell<i32>,
    }
    unsafe extern "C" fn query(p: *mut c_void, _: *const Uid, out: *mut *mut c_void) -> i32 {
        let probe = unsafe { &*p.cast::<Probe>() };
        unsafe { *out = probe.output.get() };
        probe.status.get()
    }
    unsafe extern "C" fn release(p: *mut c_void) -> u32 {
        let probe = unsafe { &*p.cast::<Probe>() };
        probe.releases.set(probe.releases.get() + 1);
        1
    }
    let table = [
        query as *const c_void,
        ptr::null(),
        release as *const c_void,
    ];
    let sentinel = Probe {
        table: table.as_ptr(),
        releases: Cell::new(0),
        output: Cell::new(ptr::null_mut()),
        status: Cell::new(-1),
    };
    let root = Probe {
        table: table.as_ptr(),
        releases: Cell::new(0),
        output: Cell::new((&sentinel as *const Probe).cast_mut().cast()),
        status: Cell::new(-1),
    };
    let owner =
        unsafe { Owned::take(NonNull::new((&root as *const Probe).cast_mut().cast()).unwrap()) };
    assert!(owner.query(&ffi::UNKNOWN).is_err());
    assert_eq!(
        sentinel.releases.get(),
        0,
        "failed QI output is not an owned reference"
    );
    drop(owner);
    assert_eq!(root.releases.get(), 1);
}

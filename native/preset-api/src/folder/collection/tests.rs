use super::*;
use crate::folder::FolderInputs;
use ffi::{CStringRepr, Variant};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    pin::Pin,
};

const PATH: &str = "/private/tmp/server-private/opaque_7.multipreset";
#[derive(Clone)]
struct Events {
    calls: Rc<RefCell<Vec<&'static str>>>,
    alive: Rc<Cell<bool>>,
    clones: Rc<RefCell<VecDeque<*mut c_void>>>,
    opened: Rc<RefCell<VecDeque<*mut c_void>>>,
}
#[repr(C)]
struct Mock {
    table: *const *const c_void,
    refs: Cell<i32>,
    identity: Cell<*mut c_void>,
    missing: Cell<Option<Uid>>,
    query_failure: Cell<Option<(i32, *mut c_void)>>,
    foreign_collection: Cell<*mut c_void>,
    handler: Cell<*mut c_void>,
    attrs: Cell<*mut c_void>,
    created: Cell<*mut c_void>,
    target: Cell<*mut c_void>,
    text_pointer: Cell<*mut c_void>,
    protocol: RefCell<StringRepr>,
    host: RefCell<StringRepr>,
    text: Vec<u16>,
    path: RefCell<Vec<u8>>,
    count: Cell<i32>,
    boolean: Cell<u8>,
    action: Cell<u8>,
    file_kind: Cell<i32>,
    native: Cell<u8>,
    absolute: Cell<u8>,
    parameters: Cell<u8>,
    fail_assign: Cell<bool>,
    fail_path: Cell<bool>,
    flags: Cell<i32>,
    invalidate: Cell<Option<&'static str>>,
    release_invalidate: Cell<bool>,
    release_handler: Cell<Option<(*mut c_void, *mut c_void)>>,
    events: Events,
}
unsafe fn object<'a>(p: *mut c_void) -> &'a Mock {
    unsafe { &*p.cast::<Mock>() }
}
fn call(m: &Mock, name: &'static str) {
    m.events.calls.borrow_mut().push(name);
    if m.invalidate.get() == Some(name) {
        m.events.alive.set(false);
    }
}
unsafe extern "C" fn retain(p: *mut c_void) -> u32 {
    let m = unsafe { object(p) };
    call(m, "retain");
    assert!(m.refs.get() > 0);
    m.refs.set(m.refs.get() + 1);
    m.refs.get() as u32
}
unsafe extern "C" fn release(p: *mut c_void) -> u32 {
    let m = unsafe { object(p) };
    call(m, "release");
    assert!(m.refs.get() > 0);
    m.refs.set(m.refs.get() - 1);
    if m.release_invalidate.get() {
        m.events.alive.set(false);
    }
    if let Some((registry, handler)) = m.release_handler.get() {
        unsafe { object(registry) }.handler.set(handler);
    }
    m.refs.get() as u32
}
unsafe extern "C" fn query(p: *mut c_void, iid: *const Uid, out: *mut *mut c_void) -> i32 {
    let m = unsafe { object(p) };
    call(m, "query");
    if let Some((status, output)) = m.query_failure.get() {
        unsafe { *out = output };
        return status;
    }
    if m.missing.get() == Some(unsafe { *iid }) {
        unsafe { *out = ptr::null_mut() };
        return 0x80004002_u32 as i32;
    }
    let output = if unsafe { *iid } == ffi::UNKNOWN {
        m.identity.get()
    } else if unsafe { *iid } == COLLECTION && !m.foreign_collection.get().is_null() {
        m.foreign_collection.get()
    } else {
        p
    };
    if !output.is_null() {
        unsafe { retain(output) };
    }
    unsafe { *out = output };
    0
}
unsafe extern "C" fn handler(p: *mut c_void, target: *mut c_void) -> *mut c_void {
    let m = unsafe { object(p) };
    call(m, "handler");
    assert!(!target.is_null());
    m.handler.get()
}
unsafe extern "C" fn flags(p: *mut c_void, target: *mut c_void) -> i32 {
    let m = unsafe { object(p) };
    call(m, "flags");
    assert!(!target.is_null());
    m.flags.get()
}
unsafe extern "C" fn attributes(
    p: *mut c_void,
    cid: *const Uid,
    iid: *const Uid,
    out: *mut *mut c_void,
) -> i32 {
    let m = unsafe { object(p) };
    call(m, "attributes");
    assert!(unsafe { *cid } == ffi::ATTRIBUTES_CLASS && unsafe { *iid } == ffi::ATTRIBUTES);
    unsafe {
        retain(m.attrs.get());
        *out = m.attrs.get()
    };
    0
}
unsafe extern "C" fn metadata(p: *mut c_void, attrs: *mut c_void) -> u8 {
    let m = unsafe { object(p) };
    call(m, "metadata");
    assert!(!attrs.is_null());
    m.boolean.get()
}
unsafe extern "C" fn count(p: *mut c_void) -> i32 {
    let m = unsafe { object(p) };
    call(m, "count");
    m.count.get()
}
unsafe extern "C" fn attribute(p: *mut c_void, value: *mut Variant, id: *const CStringRepr) -> u8 {
    let m = unsafe { object(p) };
    call(m, "attribute");
    let key = unsafe { CStr::from_ptr((*id).text) };
    assert!([c"Class:Name", c"Class:Vendor", c"Class:Category"].contains(&key));
    if m.boolean.get() == 0 {
        return 0;
    }
    unsafe {
        (*value).tag = 3;
        (*value).payload = m.text_pointer.get() as usize as u64
    };
    1
}
unsafe extern "C" fn length(p: *mut c_void) -> i32 {
    let m = unsafe { object(p) };
    call(m, "length");
    assert!(m.refs.get() > 0);
    m.text.len() as i32
}
unsafe extern "C" fn text_copy(p: *mut c_void, out: *mut u16, n: i32) -> i32 {
    let m = unsafe { object(p) };
    call(m, "text_copy");
    assert!(m.refs.get() > 0 && n as usize > m.text.len());
    unsafe {
        ptr::copy_nonoverlapping(m.text.as_ptr(), out, m.text.len());
        *out.add(m.text.len()) = 0
    };
    0
}
unsafe extern "C" fn clone_url(p: *mut c_void, out: *mut *mut c_void) {
    let m = unsafe { object(p) };
    call(m, "clone");
    let cloned = m
        .events
        .clones
        .borrow_mut()
        .pop_front()
        .unwrap_or(ptr::null_mut());
    if cloned.is_null() {
        unsafe { *out = ptr::null_mut() };
        return;
    }
    let c = unsafe { object(cloned) };
    c.path.replace(m.path.borrow().clone());
    c.file_kind.set(m.file_kind.get());
    c.native.set(m.native.get());
    c.absolute.set(m.absolute.get());
    c.parameters.set(m.parameters.get());
    c.fail_assign.set(m.fail_assign.get());
    c.fail_path.set(m.fail_path.get());
    c.protocol.borrow_mut().object = m.protocol.borrow().object;
    c.host.borrow_mut().object = m.host.borrow().object;
    unsafe {
        retain(cloned);
        *out = cloned
    };
}
unsafe extern "C" fn empty(p: *mut c_void) -> u8 {
    let m = unsafe { object(p) };
    call(m, "empty");
    u8::from(m.path.borrow().is_empty())
}
unsafe extern "C" fn kind(p: *mut c_void) -> i32 {
    let m = unsafe { object(p) };
    call(m, "kind");
    m.file_kind.get()
}
unsafe extern "C" fn native(p: *mut c_void) -> u8 {
    let m = unsafe { object(p) };
    call(m, "native");
    m.native.get()
}
unsafe extern "C" fn absolute(p: *mut c_void) -> u8 {
    let m = unsafe { object(p) };
    call(m, "absolute");
    m.absolute.get()
}
unsafe extern "C" fn parameters(p: *mut c_void) -> u8 {
    let m = unsafe { object(p) };
    call(m, "parameters");
    m.parameters.get()
}
unsafe extern "C" fn protocol(p: *mut c_void) -> *const StringRepr {
    let m = unsafe { object(p) };
    call(m, "protocol");
    m.protocol.as_ptr()
}
unsafe extern "C" fn host(p: *mut c_void) -> *const StringRepr {
    let m = unsafe { object(p) };
    call(m, "host");
    m.host.as_ptr()
}
unsafe extern "C" fn from_posix(p: *mut c_void, path: *const c_char, kind: i32) -> u8 {
    let m = unsafe { object(p) };
    call(m, "from_posix");
    assert_eq!(kind, 1);
    if m.fail_assign.get() {
        return 0;
    }
    m.path
        .replace(unsafe { CStr::from_ptr(path) }.to_bytes().to_vec());
    1
}
unsafe extern "C" fn to_posix(p: *mut c_void, out: *mut c_char, n: i32) -> u8 {
    let m = unsafe { object(p) };
    call(m, "to_posix");
    if m.fail_path.get() {
        return 0;
    }
    let path = m.path.borrow();
    assert!(n as usize > path.len());
    unsafe {
        ptr::copy_nonoverlapping(path.as_ptr(), out.cast(), path.len());
        *out.add(path.len()) = 0
    };
    1
}
unsafe extern "C" fn create(p: *mut c_void, url: *mut c_void, attrs: *mut c_void) -> *mut c_void {
    let m = unsafe { object(p) };
    call(m, "create");
    assert_eq!(attrs, m.attrs.get());
    let output = m.created.get();
    if !output.is_null() {
        unsafe { object(output) }
            .path
            .replace(unsafe { object(url) }.path.borrow().clone());
        unsafe { retain(output) };
    }
    output
}
unsafe extern "C" fn open(
    p: *mut c_void,
    url: *mut c_void,
    descriptor: *mut c_void,
) -> *mut c_void {
    let m = unsafe { object(p) };
    call(m, "open");
    assert!(descriptor.is_null());
    let output = m
        .events
        .opened
        .borrow_mut()
        .pop_front()
        .unwrap_or(ptr::null_mut());
    if !output.is_null() {
        unsafe { object(output) }
            .path
            .replace(unsafe { object(url) }.path.borrow().clone());
        unsafe { retain(output) };
    }
    output
}
unsafe extern "C" fn preset_url(p: *mut c_void, url: *mut c_void) -> u8 {
    let m = unsafe { object(p) };
    call(m, "preset_url");
    unsafe { object(url) }.path.replace(m.path.borrow().clone());
    m.boolean.get()
}
unsafe extern "C" fn store(p: *mut c_void, target: *mut c_void) -> u8 {
    let m = unsafe { object(p) };
    call(m, "store");
    assert_eq!(target, m.target.get());
    m.action.get()
}
unsafe extern "C" fn restore(p: *mut c_void, target: *mut c_void) -> u8 {
    let m = unsafe { object(p) };
    call(m, "restore");
    assert_eq!(target, m.target.get());
    m.action.get()
}
struct Fixture {
    objects: Vec<Pin<Box<Mock>>>,
    tables: Vec<Pin<Box<[*const c_void; 42]>>>,
    events: Events,
    folder: *mut c_void,
    registry: *mut c_void,
    allocator: *mut c_void,
    handler: *mut c_void,
    attrs: *mut c_void,
    prototype: *mut c_void,
    created: *mut c_void,
    first_open: *mut c_void,
    second_open: *mut c_void,
    text: *mut c_void,
    protocol: *mut c_void,
    host: *mut c_void,
    other: *mut c_void,
}
impl Fixture {
    fn new() -> Self {
        let events = Events {
            calls: Rc::new(RefCell::new(vec![])),
            alive: Rc::new(Cell::new(true)),
            clones: Rc::new(RefCell::new(VecDeque::new())),
            opened: Rc::new(RefCell::new(VecDeque::new())),
        };
        let mut f = Self {
            objects: vec![],
            tables: vec![],
            events,
            folder: ptr::null_mut(),
            registry: ptr::null_mut(),
            allocator: ptr::null_mut(),
            handler: ptr::null_mut(),
            attrs: ptr::null_mut(),
            prototype: ptr::null_mut(),
            created: ptr::null_mut(),
            first_open: ptr::null_mut(),
            second_open: ptr::null_mut(),
            text: ptr::null_mut(),
            protocol: ptr::null_mut(),
            host: ptr::null_mut(),
            other: ptr::null_mut(),
        };
        f.text = f.object(
            &[
                (4, length as *const c_void),
                (8, text_copy as *const c_void),
            ],
            "FX Chains",
        );
        f.protocol = f.object(
            &[
                (4, length as *const c_void),
                (8, text_copy as *const c_void),
            ],
            "file",
        );
        f.host = f.object(
            &[
                (4, length as *const c_void),
                (8, text_copy as *const c_void),
            ],
            "",
        );
        f.folder = f.object(&[(3, metadata as *const c_void)], "");
        f.registry = f.object(&[(6, handler as *const c_void)], "");
        f.allocator = f.object(&[(3, attributes as *const c_void)], "");
        f.handler = f.object(
            &[
                (9, open as *const c_void),
                (10, create as *const c_void),
                (11, flags as *const c_void),
            ],
            "",
        );
        f.attrs = f.object(
            &[
                (5, count as *const c_void),
                (10, attribute as *const c_void),
            ],
            "",
        );
        f.created = f.preset();
        f.first_open = f.preset();
        f.second_open = f.preset();
        f.events
            .opened
            .borrow_mut()
            .extend([f.first_open, f.second_open]);
        f.prototype = f.url();
        for _ in 0..24 {
            let u = f.url();
            f.events.clones.borrow_mut().push_back(u);
        }
        f.other = f.object(&[], "");
        unsafe { object(f.registry) }.handler.set(f.handler);
        unsafe { object(f.allocator) }.attrs.set(f.attrs);
        unsafe { object(f.handler) }.attrs.set(f.attrs);
        unsafe { object(f.handler) }.created.set(f.created);
        unsafe { object(f.attrs) }.text_pointer.set(f.text);
        f
    }
    fn object(&mut self, methods: &[(usize, *const c_void)], text: &str) -> *mut c_void {
        let mut table = Box::pin([ptr::null(); 42]);
        table[0] = query as *const c_void;
        table[1] = retain as *const c_void;
        table[2] = release as *const c_void;
        for &(slot, method) in methods {
            table[slot] = method;
        }
        let mut m = Box::pin(Mock {
            table: table.as_ptr(),
            refs: Cell::new(1),
            identity: Cell::new(ptr::null_mut()),
            missing: Cell::new(None),
            query_failure: Cell::new(None),
            foreign_collection: Cell::new(ptr::null_mut()),
            handler: Cell::new(ptr::null_mut()),
            attrs: Cell::new(ptr::null_mut()),
            created: Cell::new(ptr::null_mut()),
            target: Cell::new(ptr::null_mut()),
            text_pointer: Cell::new(ptr::null_mut()),
            protocol: RefCell::new(StringRepr {
                object: self.protocol,
                private: ptr::null_mut(),
            }),
            host: RefCell::new(StringRepr {
                object: self.host,
                private: ptr::null_mut(),
            }),
            text: text.encode_utf16().collect(),
            path: RefCell::new(vec![]),
            count: Cell::new(3),
            boolean: Cell::new(1),
            action: Cell::new(1),
            file_kind: Cell::new(1),
            native: Cell::new(1),
            absolute: Cell::new(1),
            parameters: Cell::new(0),
            fail_assign: Cell::new(false),
            fail_path: Cell::new(false),
            flags: Cell::new(3),
            invalidate: Cell::new(None),
            release_invalidate: Cell::new(false),
            release_handler: Cell::new(None),
            events: self.events.clone(),
        });
        let p = (&mut *m as *mut Mock).cast();
        m.identity.set(p);
        self.objects.push(m);
        self.tables.push(table);
        p
    }
    fn url(&mut self) -> *mut c_void {
        self.object(
            &[
                (3, clone_url as *const c_void),
                (6, empty as *const c_void),
                (7, kind as *const c_void),
                (10, protocol as *const c_void),
                (12, host as *const c_void),
                (24, native as *const c_void),
                (27, to_posix as *const c_void),
                (28, from_posix as *const c_void),
                (31, absolute as *const c_void),
                (41, parameters as *const c_void),
            ],
            "",
        )
    }
    fn preset(&mut self) -> *mut c_void {
        let p = self.object(
            &[
                (3, count as *const c_void),
                (7, preset_url as *const c_void),
                (9, store as *const c_void),
                (10, restore as *const c_void),
            ],
            "",
        );
        unsafe { object(p) }.target.set(self.folder);
        p
    }
    fn guard(&self) -> impl FnMut() -> Result<(), String> + use<> {
        let events = self.events.clone();
        move || {
            events.calls.borrow_mut().push("guard");
            if events.alive.get() {
                Ok(())
            } else {
                Err("binding changed".into())
            }
        }
    }
    fn route(&self) -> Rc<FolderPresetRoute> {
        for p in [self.folder, self.registry, self.allocator] {
            unsafe { retain(p) };
        }
        Rc::new(
            unsafe {
                FolderPresetRoute::discover_owned(
                    FolderInputs {
                        folder: NonNull::new(self.folder).unwrap(),
                        registry: NonNull::new(self.registry).unwrap(),
                        allocator: NonNull::new(self.allocator).unwrap(),
                    },
                    &mut self.guard(),
                )
            }
            .unwrap(),
        )
    }
    fn destination(&self) -> Result<PrivateDestination, Error> {
        unsafe {
            retain(self.prototype);
            PrivateDestination::from_owned_empty_url(
                NonNull::new(self.prototype).unwrap(),
                PATH,
                &mut self.guard(),
            )
        }
    }
    fn capture(&self) -> Result<CollectionFile, CollectionCallError> {
        CollectionFile::capture(self.route(), self.destination().unwrap(), &mut self.guard())
    }
    fn calls(&self, name: &str) -> usize {
        self.events
            .calls
            .borrow()
            .iter()
            .filter(|c| **c == name)
            .count()
    }
    fn balanced(&self) {
        for m in &self.objects {
            assert_eq!(m.refs.get(), 1, "unbalanced native owner");
        }
    }
}

#[test]
fn exact_handler_capture_reopens_and_restores_exact_folder_once() {
    let f = Fixture::new();
    let file = f.capture().unwrap();
    assert_eq!(file.part_count(), 3);
    assert_eq!(f.calls("create"), 1);
    assert_eq!(f.calls("store"), 1);
    assert_eq!(f.calls("open"), 1);
    let restored = file.restore(&mut f.guard()).unwrap();
    assert_eq!(
        restored,
        CollectionRestore {
            accepted: true,
            verified: false,
            part_count: 3
        }
    );
    assert_eq!(f.calls("open"), 2);
    assert_eq!(f.calls("restore"), 1);
    drop(file);
    f.balanced();
}
#[test]
fn normalized_server_filename_is_required_before_any_url_method() {
    for path in [
        "relative.multipreset",
        "/tmp/./x.multipreset",
        "/tmp/../x.multipreset",
        "/tmp//x.multipreset",
        "/tmp/x.preset",
        "/tmp/.multipreset",
        "/tmp/a b.multipreset",
        "/tmp/x.multipreset\0",
    ] {
        let f = Fixture::new();
        unsafe { retain(f.prototype) };
        assert!(
            unsafe {
                PrivateDestination::from_owned_empty_url(
                    NonNull::new(f.prototype).unwrap(),
                    path,
                    &mut f.guard(),
                )
            }
            .is_err()
        );
        assert_eq!(f.calls("clone"), 0);
        f.balanced();
    }
}
#[test]
fn url_must_remain_plain_absolute_native_file_with_exact_path() {
    for invalid in 0..6 {
        let f = Fixture::new();
        let p = unsafe { object(f.prototype) };
        match invalid {
            0 => p.file_kind.set(2),
            1 => p.native.set(0),
            2 => p.absolute.set(0),
            3 => p.parameters.set(1),
            4 => p.fail_assign.set(true),
            _ => p.fail_path.set(true),
        }
        assert!(f.destination().is_err());
        f.balanced();
    }
}
#[test]
fn changed_destination_path_blocks_store() {
    let f = Fixture::new();
    let d = f.destination().unwrap();
    unsafe { object(d.url.raw()) }
        .path
        .replace(b"/other/file.multipreset".to_vec());
    let error = CollectionFile::capture(f.route(), d, &mut f.guard())
        .err()
        .unwrap();
    assert!(!error.dispatched);
    assert_eq!(f.calls("store"), 0);
    f.balanced();
}
#[test]
fn clone_alias_is_rejected_without_mutating_prototype() {
    let f = Fixture::new();
    let first = *f.events.clones.borrow().front().unwrap();
    unsafe { object(first) }.identity.set(f.prototype);
    assert!(f.destination().is_err());
    assert_eq!(f.calls("from_posix"), 0);
    f.balanced();
}
#[test]
fn collection_interface_is_required_and_must_share_canonical_identity() {
    for foreign in [false, true] {
        let f = Fixture::new();
        let p = unsafe { object(f.created) };
        if foreign {
            p.foreign_collection.set(f.other)
        } else {
            p.missing.set(Some(COLLECTION))
        }
        let error = f.capture().err().unwrap();
        assert!(!error.dispatched);
        assert_eq!(f.calls("store"), 0);
        f.balanced();
    }
}
#[test]
fn failed_qi_nonnull_sentinel_is_never_owned_or_dereferenced() {
    for status in [1, -1, 0x80004002_u32 as i32] {
        let f = Fixture::new();
        unsafe { object(f.created) }
            .query_failure
            .set(Some((status, ptr::dangling_mut::<c_void>())));
        let error = f.capture().err().unwrap();
        assert!(!error.dispatched);
        assert_eq!(f.calls("store"), 0);
        f.balanced();
    }
}
#[test]
fn collection_counts_are_bounded_before_store_and_on_reopen() {
    for reopen in [false, true] {
        for count in [-1, MAX_COLLECTION_PRESETS as i32 + 1] {
            let f = Fixture::new();
            unsafe { object(if reopen { f.first_open } else { f.created }) }
                .count
                .set(count);
            let error = f.capture().err().unwrap();
            assert_eq!(error.dispatched, reopen);
            assert_eq!(f.calls("store"), usize::from(reopen));
            f.balanced();
        }
    }
}
#[test]
fn missing_metadata_and_refused_metadata_never_store() {
    for absent in [false, true] {
        let f = Fixture::new();
        let route = f.route();
        let d = f.destination().unwrap();
        unsafe { object(if absent { f.attrs } else { f.folder }) }
            .boolean
            .set(0);
        let error = CollectionFile::capture(route, d, &mut f.guard())
            .err()
            .unwrap();
        assert!(!error.dispatched);
        assert_eq!(f.calls("create"), 0);
        assert_eq!(f.calls("store"), 0);
        f.balanced();
    }
}
#[test]
fn changed_handler_and_lost_capability_never_store() {
    for changed in [false, true] {
        let f = Fixture::new();
        let route = f.route();
        let d = f.destination().unwrap();
        if changed {
            unsafe { object(f.registry) }.handler.set(f.other)
        } else {
            unsafe { object(f.handler) }.flags.set(1)
        }
        let error = CollectionFile::capture(route, d, &mut f.guard())
            .err()
            .unwrap();
        assert!(!error.dispatched);
        assert_eq!(f.calls("create"), 0);
        assert_eq!(f.calls("store"), 0);
        f.balanced();
    }
}
#[test]
fn false_store_and_post_store_guard_loss_are_uncertain_and_never_open() {
    for guard_loss in [false, true] {
        let f = Fixture::new();
        let p = unsafe { object(f.created) };
        if guard_loss {
            p.invalidate.set(Some("store"))
        } else {
            p.action.set(0)
        }
        let error = f.capture().err().unwrap();
        assert!(error.dispatched);
        assert_eq!(f.calls("store"), 1);
        assert_eq!(f.calls("open"), 0);
        f.balanced();
    }
}
#[test]
fn failed_open_after_store_is_dispatched_and_never_restores() {
    let f = Fixture::new();
    f.events.opened.borrow_mut().clear();
    let error = f.capture().err().unwrap();
    assert!(error.dispatched);
    assert_eq!(f.calls("open"), 1);
    assert_eq!(f.calls("restore"), 0);
    f.balanced();
}
#[test]
fn false_restore_and_post_dispatch_guard_loss_are_uncertain_without_retry() {
    for guard_loss in [false, true] {
        let f = Fixture::new();
        let file = f.capture().unwrap();
        let p = unsafe { object(f.second_open) };
        if guard_loss {
            p.invalidate.set(Some("restore"))
        } else {
            p.action.set(0)
        }
        let error = file.restore(&mut f.guard()).err().unwrap();
        assert!(error.dispatched);
        assert_eq!(f.calls("restore"), 1);
        drop(file);
        f.balanced();
    }
}
#[test]
fn changed_reopened_count_blocks_restore() {
    let f = Fixture::new();
    let file = f.capture().unwrap();
    unsafe { object(f.second_open) }.count.set(4);
    let error = file.restore(&mut f.guard()).err().unwrap();
    assert!(!error.dispatched);
    assert_eq!(f.calls("restore"), 0);
    drop(file);
    f.balanced();
}
#[test]
fn callback_context_loss_during_create_releases_owned_factory_result() {
    let f = Fixture::new();
    unsafe { object(f.handler) }.invalidate.set(Some("create"));
    let error = f.capture().err().unwrap();
    assert!(!error.dispatched);
    assert_eq!(f.calls("store"), 0);
    f.balanced();
}
#[test]
fn prototype_release_reentry_is_followed_by_fresh_guard() {
    let f = Fixture::new();
    unsafe { object(f.prototype) }.release_invalidate.set(true);
    assert!(f.destination().is_err());
    assert_eq!(f.events.calls.borrow().last(), Some(&"guard"));
    f.balanced();
}
#[test]
fn borrowed_url_text_is_retained_immediately_before_guard() {
    let f = Fixture::new();
    let d = f.destination().unwrap();
    let calls = f.events.calls.borrow();
    for (i, c) in calls.iter().enumerate() {
        if matches!(*c, "protocol" | "host") {
            assert_eq!(&calls[i + 1..i + 3], &["retain", "guard"]);
        }
    }
    drop(calls);
    drop(d);
    f.balanced();
}
#[test]
fn wrong_thread_check_has_no_native_call() {
    let f = Fixture::new();
    let mut d = f.destination().unwrap();
    let correct = d.thread;
    d.thread = std::thread::spawn(|| thread::current().id())
        .join()
        .unwrap();
    let before = f.events.calls.borrow().len();
    assert!(matches!(d.check(&mut f.guard()), Err(Error::WrongThread)));
    assert_eq!(f.events.calls.borrow().len(), before);
    d.thread = correct;
    drop(d);
    f.balanced();
}

#[test]
fn empty_prototype_and_real_clone_are_required() {
    let f = Fixture::new();
    unsafe { object(f.prototype) }
        .path
        .replace(b"/already.multipreset".to_vec());
    assert!(f.destination().is_err());
    assert_eq!(f.calls("clone"), 0);
    f.balanced();
    let f = Fixture::new();
    f.events.clones.borrow_mut().clear();
    assert!(f.destination().is_err());
    assert_eq!(f.calls("from_posix"), 0);
    f.balanced();
}
#[test]
fn failed_url_qi_output_never_becomes_an_owned_pointer() {
    let f = Fixture::new();
    unsafe { object(f.prototype) }
        .query_failure
        .set(Some((1, ptr::dangling_mut::<c_void>())));
    assert!(f.destination().is_err());
    assert_eq!(f.calls("clone"), 0);
    f.balanced();
}
#[test]
fn metadata_cleanup_handler_switch_prevents_store() {
    let f = Fixture::new();
    let route = f.route();
    let d = f.destination().unwrap();
    unsafe { object(f.attrs) }
        .release_handler
        .set(Some((f.registry, f.other)));
    let error = CollectionFile::capture(route, d, &mut f.guard())
        .err()
        .unwrap();
    assert!(!error.dispatched);
    assert_eq!(f.calls("create"), 1);
    assert_eq!(f.calls("store"), 0);
    assert_eq!(f.events.calls.borrow().last(), Some(&"guard"));
    f.balanced();
}
#[test]
fn reopened_collection_interface_is_required_after_a_store() {
    let f = Fixture::new();
    unsafe { object(f.first_open) }
        .missing
        .set(Some(COLLECTION));
    let error = f.capture().err().unwrap();
    assert!(error.dispatched);
    assert_eq!(f.calls("store"), 1);
    assert_eq!(f.calls("open"), 1);
    f.balanced();
}
#[test]
fn every_new_foreign_boundary_stops_when_context_is_lost() {
    for name in [
        "empty",
        "clone",
        "from_posix",
        "kind",
        "native",
        "absolute",
        "parameters",
        "protocol",
        "host",
        "to_posix",
    ] {
        let f = Fixture::new();
        for p in std::iter::once(f.prototype).chain(f.events.clones.borrow().iter().copied()) {
            unsafe { object(p) }.invalidate.set(Some(name));
        }
        assert!(f.destination().is_err(), "{name}");
        f.balanced();
    }
    for name in ["create", "preset_url", "count", "store", "open"] {
        let f = Fixture::new();
        let route = f.route();
        let d = f.destination().unwrap();
        for p in [f.handler, f.created, f.first_open] {
            unsafe { object(p) }.invalidate.set(Some(name));
        }
        let error = CollectionFile::capture(route, d, &mut f.guard())
            .err()
            .unwrap();
        assert_eq!(error.dispatched, matches!(name, "store" | "open"), "{name}");
        assert_eq!(f.calls("restore"), 0);
        f.balanced();
    }
}
#[test]
fn malformed_action_booleans_never_become_success() {
    let f = Fixture::new();
    unsafe { object(f.created) }.action.set(2);
    let error = f.capture().err().unwrap();
    assert!(error.dispatched);
    assert_eq!(f.calls("open"), 0);
    f.balanced();
    let f = Fixture::new();
    let file = f.capture().unwrap();
    unsafe { object(f.second_open) }.action.set(2);
    let error = file.restore(&mut f.guard()).err().unwrap();
    assert!(error.dispatched);
    assert_eq!(f.calls("restore"), 1);
    drop(file);
    f.balanced();
}

#[test]
fn nonlocal_or_missing_url_text_never_becomes_a_destination() {
    for bad in 0..3 {
        let f = Fixture::new();
        let p = unsafe { object(f.prototype) };
        match bad {
            0 => p.protocol.borrow_mut().object = f.text,
            1 => p.host.borrow_mut().object = f.text,
            _ => p.protocol.borrow_mut().object = ptr::null_mut(),
        }
        assert!(f.destination().is_err());
        f.balanced();
    }
}
#[test]
fn a_preset_for_a_foreign_location_is_not_accepted() {
    let f = Fixture::new();
    let d = f.destination().unwrap();
    unsafe { object(f.created) }
        .path
        .replace(b"/private/other.multipreset".to_vec());
    let preset = unsafe { Owned::hold(f.created) }.unwrap();
    assert!(inspect_preset(preset, &d, &mut f.guard()).is_err());
    assert_eq!(f.calls("store"), 0);
    drop(d);
    f.balanced();
}
#[test]
fn restore_refuses_a_changed_destination_or_handler_before_open() {
    for handler_changed in [false, true] {
        let f = Fixture::new();
        let file = f.capture().unwrap();
        if handler_changed {
            unsafe { object(f.registry) }.handler.set(f.other);
        } else {
            unsafe { object(file.destination.url.raw()) }
                .path
                .replace(b"/private/other.multipreset".to_vec());
        }
        let error = file.restore(&mut f.guard()).err().unwrap();
        assert!(!error.dispatched);
        assert_eq!(f.calls("open"), 1);
        assert_eq!(f.calls("restore"), 0);
        drop(file);
        f.balanced();
    }
}

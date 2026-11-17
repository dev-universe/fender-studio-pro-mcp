use super::*;
use std::{
    cell::{Cell, RefCell},
    ffi::CStr,
    rc::Rc,
};

#[repr(C)]
struct Mock {
    vtable: *const *const c_void,
    refs: Cell<i32>,
    dead_retain: Cell<bool>,
    dead_read: Cell<bool>,
    dead_release: Cell<bool>,
    identity: Cell<*mut c_void>,
    handler: Cell<*mut c_void>,
    attrs: Cell<*mut c_void>,
    text: Vec<u16>,
    text_pointer: Cell<*mut c_void>,
    query_failure: Cell<Option<(i32, *mut c_void)>>,
    query_replacement: Cell<Option<(Uid, *mut c_void)>>,
    count_delta: Cell<i32>,
    missing: Cell<Option<Uid>>,
    flags: Cell<i32>,
    count: Cell<i32>,
    meta_ok: Cell<u8>,
    factory_status: Cell<i32>,
    factory_output: Cell<Option<*mut c_void>>,
    field_kind: Cell<i16>,
    field_found: Cell<u8>,
    invalidate_on: RefCell<Option<&'static str>>,
    alive: Rc<Cell<bool>>,
    calls: Rc<RefCell<Vec<&'static str>>>,
    release_switch: Cell<*mut c_void>,
    release_new_handler: Cell<*mut c_void>,
}
unsafe fn mock<'a>(p: *mut c_void) -> &'a Mock {
    unsafe { &*p.cast::<Mock>() }
}
fn call(o: &Mock, name: &'static str) {
    o.calls.borrow_mut().push(name);
    if o.invalidate_on.borrow().as_ref() == Some(&name) {
        o.alive.set(false);
    }
}
unsafe extern "C" fn retain(p: *mut c_void) -> u32 {
    let o = unsafe { mock(p) };
    call(o, "retain");
    if o.refs.get() <= 0 {
        o.dead_retain.set(true);
        return 0;
    }
    o.refs.set(o.refs.get() + 1);
    o.refs.get() as u32
}
unsafe extern "C" fn release(p: *mut c_void) -> u32 {
    let o = unsafe { mock(p) };
    call(o, "release");
    if o.refs.get() <= 0 {
        o.dead_release.set(true);
        return 0;
    }
    o.refs.set(o.refs.get() - 1);
    if !o.release_switch.get().is_null() {
        unsafe { mock(o.release_switch.get()) }
            .handler
            .set(o.release_new_handler.get());
    }
    o.refs.get() as u32
}
unsafe extern "C" fn query(p: *mut c_void, iid: *const Uid, out: *mut *mut c_void) -> i32 {
    let o = unsafe { mock(p) };
    call(o, "query");
    if let Some((status, output)) = o.query_failure.get() {
        unsafe { *out = output };
        return status;
    }
    if o.missing.get() == Some(unsafe { *iid }) {
        unsafe { *out = ptr::null_mut() };
        return 0x80004002u32 as i32;
    }
    if let Some((selected, output)) = o.query_replacement.get() {
        if selected == unsafe { *iid } {
            unsafe {
                retain(output);
                *out = output;
            }
            return 0;
        }
    }
    let output = if unsafe { *iid } == ffi::UNKNOWN {
        o.identity.get()
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
    let o = unsafe { mock(p) };
    call(o, "handler");
    assert!(!target.is_null());
    o.handler.get()
}
unsafe extern "C" fn flags(p: *mut c_void, target: *mut c_void) -> i32 {
    let o = unsafe { mock(p) };
    call(o, "flags");
    assert!(!target.is_null());
    o.flags.get()
}
unsafe extern "C" fn create(
    p: *mut c_void,
    cid: *const Uid,
    iid: *const Uid,
    out: *mut *mut c_void,
) -> i32 {
    let o = unsafe { mock(p) };
    call(o, "create");
    assert!(unsafe { *cid } == ffi::ATTRIBUTES_CLASS && unsafe { *iid } == ffi::ATTRIBUTES);
    let output = o.factory_output.get().unwrap_or(o.attrs.get());
    if o.factory_status.get() == 0 && !output.is_null() {
        unsafe { retain(output) };
    }
    unsafe { *out = output };
    o.factory_status.get()
}
unsafe extern "C" fn metadata(p: *mut c_void, attrs: *mut c_void) -> u8 {
    let o = unsafe { mock(p) };
    call(o, "metadata");
    assert!(!attrs.is_null());
    o.meta_ok.get()
}
unsafe extern "C" fn count(p: *mut c_void) -> i32 {
    let o = unsafe { mock(p) };
    call(o, "count");
    let count = o.count.get();
    o.count.set(count + o.count_delta.get());
    count
}
unsafe extern "C" fn attribute(p: *mut c_void, value: *mut Variant, id: *const CStringRepr) -> u8 {
    let o = unsafe { mock(p) };
    call(o, "attribute");
    let name = unsafe { CStr::from_ptr((*id).text) }.to_str().unwrap();
    assert!(["Class:Name", "Class:Vendor", "Class:Category"].contains(&name));
    unsafe {
        (*value).tag = o.field_kind.get();
        (*value).payload = o.text_pointer.get() as usize as u64;
    }
    if o.field_found.get() == 1 && o.field_kind.get() == 19 {
        unsafe { retain(o.text_pointer.get()) };
    }
    o.field_found.get()
}
unsafe extern "C" fn length(p: *mut c_void) -> i32 {
    let o = unsafe { mock(p) };
    call(o, "length");
    if o.refs.get() <= 0 {
        o.dead_read.set(true);
        return -1;
    }
    o.text.len() as i32
}
unsafe extern "C" fn copy(p: *mut c_void, out: *mut u16, size: i32) -> i32 {
    let o = unsafe { mock(p) };
    call(o, "copy");
    if o.refs.get() <= 0 {
        o.dead_read.set(true);
        return -1;
    }
    assert!(size as usize > o.text.len());
    unsafe {
        ptr::copy_nonoverlapping(o.text.as_ptr(), out, o.text.len());
        *out.add(o.text.len()) = if o.meta_ok.get() == 1 { 0 } else { 0xffff };
    }
    0
}
struct Fixture {
    objects: Vec<std::pin::Pin<Box<Mock>>>,
    tables: Vec<std::pin::Pin<Box<[*const c_void; 16]>>>,
    alive: Rc<Cell<bool>>,
    calls: Rc<RefCell<Vec<&'static str>>>,
    folder: *mut c_void,
    registry: *mut c_void,
    allocator: *mut c_void,
    handler: *mut c_void,
    attrs: *mut c_void,
    text: *mut c_void,
    other: *mut c_void,
}
impl Fixture {
    fn new() -> Self {
        let mut f = Self {
            objects: vec![],
            tables: vec![],
            alive: Rc::new(Cell::new(true)),
            calls: Rc::new(RefCell::new(vec![])),
            folder: ptr::null_mut(),
            registry: ptr::null_mut(),
            allocator: ptr::null_mut(),
            handler: ptr::null_mut(),
            attrs: ptr::null_mut(),
            text: ptr::null_mut(),
            other: ptr::null_mut(),
        };
        f.folder = f.object(&[(3, metadata as *const c_void)], "");
        f.registry = f.object(&[(6, handler as *const c_void)], "");
        f.allocator = f.object(&[(3, create as *const c_void)], "");
        f.handler = f.object(&[(11, flags as *const c_void)], "");
        f.attrs = f.object(
            &[
                (5, count as *const c_void),
                (10, attribute as *const c_void),
            ],
            "",
        );
        f.text = f.object(
            &[(4, length as *const c_void), (8, copy as *const c_void)],
            "FX Chains",
        );
        f.other = f.object(&[], "");
        unsafe { mock(f.registry) }.handler.set(f.handler);
        unsafe { mock(f.allocator) }.attrs.set(f.attrs);
        unsafe { mock(f.attrs) }.text_pointer.set(f.text);
        f
    }
    fn object(&mut self, methods: &[(usize, *const c_void)], text: &str) -> *mut c_void {
        let mut table = Box::pin([ptr::null(); 16]);
        table[0] = query as *const c_void;
        table[1] = retain as *const c_void;
        table[2] = release as *const c_void;
        for &(slot, function) in methods {
            table[slot] = function;
        }
        let mut o = Box::pin(Mock {
            vtable: table.as_ptr(),
            refs: Cell::new(1),
            dead_retain: Cell::new(false),
            dead_read: Cell::new(false),
            dead_release: Cell::new(false),
            identity: Cell::new(ptr::null_mut()),
            handler: Cell::new(ptr::null_mut()),
            attrs: Cell::new(ptr::null_mut()),
            text: text.encode_utf16().collect(),
            text_pointer: Cell::new(ptr::null_mut()),
            query_failure: Cell::new(None),
            query_replacement: Cell::new(None),
            count_delta: Cell::new(0),
            missing: Cell::new(None),
            flags: Cell::new(3),
            count: Cell::new(3),
            meta_ok: Cell::new(1),
            factory_status: Cell::new(0),
            factory_output: Cell::new(None),
            field_kind: Cell::new(3),
            field_found: Cell::new(1),
            invalidate_on: RefCell::new(None),
            alive: self.alive.clone(),
            calls: self.calls.clone(),
            release_switch: Cell::new(ptr::null_mut()),
            release_new_handler: Cell::new(ptr::null_mut()),
        });
        let pointer = (&mut *o as *mut Mock).cast();
        o.identity.set(pointer);
        self.objects.push(o);
        self.tables.push(table);
        pointer
    }
    fn guard(&self) -> impl FnMut() -> Result<(), String> + use<> {
        let alive = self.alive.clone();
        move || {
            if alive.get() {
                Ok(())
            } else {
                Err("fixture changed".into())
            }
        }
    }
    fn route(&self) -> Result<FolderPresetRoute, Error> {
        for p in [self.folder, self.registry, self.allocator] {
            unsafe { retain(p) };
        }
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
    }
    fn balanced(&self) {
        for o in &self.objects {
            assert_eq!(o.refs.get(), 1, "native owners were not balanced");
        }
    }
}

#[test]
fn exact_folder_preflight_reports_only_bounded_metadata() {
    let f = Fixture::new();
    let route = f.route().unwrap();
    let observed = route.observe(&mut f.guard()).unwrap();
    assert_eq!(observed.target_flags, 3);
    assert_eq!(observed.metadata.attribute_count, 3);
    assert_eq!(observed.metadata.class_name.as_deref(), Some("FX Chains"));
    assert!(
        !f.calls
            .borrow()
            .iter()
            .any(|x| matches!(*x, "store" | "restore" | "url"))
    );
    drop(route);
    f.balanced();
}
#[test]
fn required_interfaces_are_not_silently_optional() {
    for iid in [MEDIATOR, COLLECTOR, REGISTRY, HANDLER] {
        let f = Fixture::new();
        let object = if iid == REGISTRY {
            f.registry
        } else if iid == HANDLER {
            f.handler
        } else {
            f.folder
        };
        unsafe { mock(object) }.missing.set(Some(iid));
        assert!(f.route().is_err());
        f.balanced();
    }
}
#[test]
fn absent_handler_never_falls_back() {
    let f = Fixture::new();
    unsafe { mock(f.registry) }.handler.set(ptr::null_mut());
    assert!(f.route().is_err());
    assert!(!f.calls.borrow().contains(&"flags"));
    f.balanced();
}
#[test]
fn import_and_export_are_both_required() {
    for flags in [-1, 0, 1, 2] {
        let f = Fixture::new();
        unsafe { mock(f.handler) }.flags.set(flags);
        assert!(f.route().is_err());
        f.balanced();
    }
}
#[test]
fn additional_known_handler_flags_are_retained() {
    let f = Fixture::new();
    unsafe { mock(f.handler) }.flags.set(19);
    let r = f.route().unwrap();
    assert_eq!(r.observe(&mut f.guard()).unwrap().target_flags, 19);
    drop(r);
    f.balanced();
}
#[test]
fn changed_current_handler_invalidates_retained_route() {
    let f = Fixture::new();
    let r = f.route().unwrap();
    unsafe { mock(f.registry) }.handler.set(f.other);
    assert_eq!(r.observe(&mut f.guard()), Err(Error::Changed));
    drop(r);
    f.balanced();
}
#[test]
fn changed_folder_canonical_identity_is_rejected() {
    let f = Fixture::new();
    let r = f.route().unwrap();
    unsafe { mock(f.folder) }.identity.set(f.other);
    assert_eq!(r.observe(&mut f.guard()), Err(Error::Changed));
    drop(r);
    f.balanced();
}
#[test]
fn metadata_refusal_and_bounds_do_not_produce_observations() {
    for mode in 0..4 {
        let f = Fixture::new();
        match mode {
            0 => unsafe { mock(f.folder) }.meta_ok.set(0),
            1 => unsafe { mock(f.attrs) }.count.set(-1),
            2 => unsafe { mock(f.attrs) }.count.set(65),
            _ => unsafe { mock(f.attrs) }.field_kind.set(1),
        };
        assert!(f.route().is_err());
        f.balanced();
    }
}
#[test]
fn optional_metadata_keys_may_be_missing() {
    let f = Fixture::new();
    unsafe { mock(f.attrs) }.field_found.set(0);
    unsafe { mock(f.attrs) }.field_kind.set(0);
    let r = f.route().unwrap();
    assert_eq!(r.observe(&mut f.guard()).unwrap().metadata.class_name, None);
    drop(r);
    f.balanced();
}
#[test]
fn failed_nonnull_factory_output_is_never_released() {
    let f = Fixture::new();
    unsafe { mock(f.allocator) }.factory_status.set(-1);
    unsafe { mock(f.allocator) }
        .factory_output
        .set(Some(f.other));
    assert!(f.route().is_err());
    f.balanced();
}
#[test]
fn success_null_factory_is_rejected() {
    let f = Fixture::new();
    unsafe { mock(f.allocator) }
        .factory_output
        .set(Some(ptr::null_mut()));
    assert!(f.route().is_err());
    f.balanced();
}
#[test]
fn failed_nonnull_query_output_is_never_released() {
    let f = Fixture::new();
    unsafe { mock(f.folder) }
        .query_failure
        .set(Some((-1, f.other)));
    assert!(f.route().is_err());
    f.balanced();
}
#[test]
fn query_success_null_is_unavailable() {
    let f = Fixture::new();
    unsafe { mock(f.folder) }
        .query_failure
        .set(Some((0, ptr::null_mut())));
    assert!(f.route().is_err());
    f.balanced();
}
#[test]
fn context_loss_at_each_foreign_operation_stops_preflight() {
    for event in [
        "query",
        "retain",
        "handler",
        "flags",
        "create",
        "metadata",
        "count",
        "attribute",
        "length",
        "copy",
        "release",
    ] {
        let f = Fixture::new();
        for o in &f.objects {
            *o.invalidate_on.borrow_mut() = Some(event);
        }
        assert!(matches!(f.route(), Err(Error::Guard(_))), "{event}");
        f.balanced();
    }
}
#[test]
fn temporary_release_can_replace_handler_without_a_success() {
    let f = Fixture::new();
    let r = f.route().unwrap();
    unsafe { mock(f.text) }.release_switch.set(f.registry);
    unsafe { mock(f.text) }.release_new_handler.set(f.other);
    assert_eq!(r.observe(&mut f.guard()), Err(Error::Changed));
    drop(r);
    f.balanced();
}
#[test]
fn wrong_thread_refuses_before_native_work() {
    let f = Fixture::new();
    let mut r = f.route().unwrap();
    r.thread = std::thread::spawn(|| thread::current().id())
        .join()
        .unwrap();
    f.calls.borrow_mut().clear();
    assert_eq!(r.observe(&mut f.guard()), Err(Error::WrongThread));
    assert!(f.calls.borrow().is_empty());
    drop(r);
    f.balanced();
}

#[test]
fn queried_interfaces_must_share_the_sources_canonical_identity() {
    for iid in [MEDIATOR, COLLECTOR, REGISTRY, HANDLER] {
        let f = Fixture::new();
        let source = if iid == REGISTRY {
            f.registry
        } else if iid == HANDLER {
            f.handler
        } else {
            f.folder
        };
        unsafe { mock(source) }
            .query_replacement
            .set(Some((iid, f.other)));
        assert!(matches!(f.route(), Err(Error::Changed)));
        f.balanced();
    }
}
#[test]
fn changing_metadata_count_is_not_a_stable_observation() {
    let f = Fixture::new();
    unsafe { mock(f.attrs) }.count_delta.set(1);
    assert!(matches!(f.route(), Err(Error::Changed)));
    f.balanced();
}
#[test]
fn metadata_text_must_be_bounded_valid_terminated_utf16() {
    for mode in 0..4 {
        let f = Fixture::new();
        let text = unsafe { &mut *f.text.cast::<Mock>() };
        match mode {
            0 => text.text = vec![65; MAX_METADATA_TEXT + 1],
            1 => text.text = vec![0xd800],
            2 => text.text = vec![0],
            _ => text.meta_ok.set(0),
        }
        assert!(matches!(f.route(), Err(Error::InvalidData(_))));
        f.balanced();
    }
}
#[test]
fn malformed_boolean_getters_are_not_truthy_success() {
    for mode in 0..2 {
        let f = Fixture::new();
        if mode == 0 {
            unsafe { mock(f.folder) }.meta_ok.set(2);
        } else {
            unsafe { mock(f.attrs) }.field_found.set(2);
        }
        assert!(f.route().is_err());
        f.balanced();
    }
}

fn revoke_attribute_text_at_first_guard(
    f: &Fixture,
    kind: i16,
    lose_context: bool,
) -> Result<Option<String>, Error> {
    unsafe { mock(f.attrs) }.field_kind.set(kind);
    let attrs = unsafe { Owned::hold(f.attrs) }.unwrap();
    let revoked = Cell::new(false);
    let mut check = || {
        let returned = f.calls.borrow().contains(&"attribute");
        if returned && !revoked.replace(returned) {
            unsafe { release(f.text) };
        }
        if returned && lose_context {
            Err("fixture changed after metadata".into())
        } else {
            Ok(())
        }
    };
    let result = text_attribute(&attrs, c"Class:Name".as_ptr(), &mut check);
    drop(attrs);
    assert!(revoked.get());
    let text = unsafe { mock(f.text) };
    assert!(
        !text.dead_retain.get(),
        "borrowed String must be pinned before its original owner is revoked"
    );
    assert!(
        !text.dead_read.get(),
        "logically dead canary payload must not be read"
    );
    assert!(
        !text.dead_release.get(),
        "logically dead canary must not be released"
    );
    assert_eq!(
        text.refs.get(),
        0,
        "the original reference and every temporary pin must be released"
    );
    for object in &f.objects {
        if std::ptr::from_ref(&**object).cast::<c_void>().cast_mut() != f.text {
            assert_eq!(object.refs.get(), 1);
        }
    }
    result
}
#[test]
fn borrowed_metadata_string_is_pinned_before_reentrant_guard() {
    let f = Fixture::new();
    let result = revoke_attribute_text_at_first_guard(&f, 3, false);
    assert_eq!(result, Ok(Some("FX Chains".into())));
}
#[test]
fn owned_metadata_string_survives_the_same_reentrant_guard() {
    let f = Fixture::new();
    let result = revoke_attribute_text_at_first_guard(&f, 19, false);
    assert_eq!(result, Ok(Some("FX Chains".into())));
}
#[test]
fn post_attribute_context_loss_releases_pins_before_returning_failure() {
    for kind in [3, 19] {
        let f = Fixture::new();
        assert!(matches!(
            revoke_attribute_text_at_first_guard(&f, kind, true),
            Err(Error::Guard(_))
        ));
    }
}
#[test]
fn failed_or_malformed_text_variants_never_claim_their_payload() {
    for (found, kind) in [(0, 3), (0, 19), (2, 3), (2, 19), (1, 4), (1, 20)] {
        let f = Fixture::new();
        unsafe { mock(f.attrs) }.field_found.set(found);
        unsafe { mock(f.attrs) }.field_kind.set(kind);
        let attrs = unsafe { Owned::hold(f.attrs) }.unwrap();
        assert!(text_attribute(&attrs, c"Class:Name".as_ptr(), &mut f.guard()).is_err());
        drop(attrs);
        f.balanced();
        assert!(
            !f.calls
                .borrow()
                .iter()
                .any(|event| matches!(*event, "length" | "copy"))
        );
    }
}

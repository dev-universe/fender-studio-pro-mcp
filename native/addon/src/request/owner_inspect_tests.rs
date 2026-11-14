use super::*;
use std::cell::Cell;

const OBJECT: Uid = Uid {
    a: 0x2e726012,
    b: 0x522c,
    c: 0x4108,
    d: [0x98, 0x62, 0x0c, 0xd5, 0x71, 0x79, 0x52, 0x49],
};
#[repr(C)]
struct Fake {
    table: *const *const c_void,
    refs: Cell<u32>,
    controller: Cell<*mut c_void>,
    object: Cell<bool>,
    native_class: Cell<*const c_char>,
    class_reads: Cell<u32>,
    stop_after_class: Cell<*mut Service>,
    stop_on_release: Cell<*mut Service>,
    gets: Cell<u32>,
    updates: Cell<u32>,
}
unsafe fn fake<'a>(pointer: *mut c_void) -> &'a Fake {
    unsafe { &*pointer.cast::<Fake>() }
}
unsafe extern "C" fn retain(pointer: *mut c_void) -> u32 {
    let value = unsafe { fake(pointer) };
    value.refs.set(value.refs.get() + 1);
    value.refs.get()
}
unsafe extern "C" fn release_mock(pointer: *mut c_void) -> u32 {
    let value = unsafe { fake(pointer) };
    value.refs.set(value.refs.get() - 1);
    let stop = value.stop_on_release.replace(ptr::null_mut());
    if !stop.is_null() {
        unsafe {
            service_terminate(stop.cast());
        }
    }
    value.refs.get()
}
unsafe extern "C" fn query_mock(
    pointer: *mut c_void,
    iid: *const Uid,
    out: *mut *mut c_void,
) -> i32 {
    let value = unsafe { fake(pointer) };
    let iid = unsafe { *iid };
    unsafe {
        *out = ptr::null_mut();
    }
    if iid == UNKNOWN || (iid == OBJECT && value.object.get()) {
        unsafe {
            retain(pointer);
            *out = pointer;
        }
        OK
    } else {
        NO_INTERFACE
    }
}
unsafe extern "C" fn controller(pointer: *mut c_void) -> *mut c_void {
    let value = unsafe { fake(pointer) };
    value.gets.set(value.gets.get() + 1);
    value.controller.get()
}
unsafe extern "C" fn get_type(pointer: *mut c_void) -> *mut c_void {
    if unsafe { fake(pointer) }.native_class.get().is_null() {
        ptr::null_mut()
    } else {
        pointer
    }
}
unsafe extern "C" fn native_class(pointer: *mut c_void) -> *const c_char {
    let value = unsafe { fake(pointer) };
    value.class_reads.set(value.class_reads.get() + 1);
    value
        .stop_on_release
        .set(value.stop_after_class.replace(ptr::null_mut()));
    value.native_class.get()
}
unsafe extern "C" fn update(pointer: *mut c_void) {
    let value = unsafe { fake(pointer) };
    value.updates.set(value.updates.get() + 1);
}
unsafe extern "C" fn constant(_: *const c_char) -> *const c_void {
    ptr::null()
}
fn text() -> TextApi {
    TextApi {
        get_constant_string: constant,
    }
}

struct Fixture {
    native: Box<Fake>,
    _table: Box<[*const c_void; 16]>,
}
impl Fixture {
    fn new() -> Self {
        let mut table = Box::new([ptr::null(); 16]);
        table[0] = query_mock as *const c_void;
        table[1] = retain as *const c_void;
        table[2] = release_mock as *const c_void;
        table[3] = get_type as *const c_void;
        table[5] = native_class as *const c_void;
        table[14] = controller as *const c_void;
        table[15] = update as *const c_void;
        Self {
            native: Box::new(Fake {
                table: table.as_ptr(),
                refs: Cell::new(1),
                controller: Cell::new(ptr::null_mut()),
                object: Cell::new(false),
                native_class: Cell::new(ptr::null()),
                class_reads: Cell::new(0),
                stop_after_class: Cell::new(ptr::null_mut()),
                stop_on_release: Cell::new(ptr::null_mut()),
                gets: Cell::new(0),
                updates: Cell::new(0),
            }),
            _table: table,
        }
    }
    fn pointer(&self) -> *mut c_void {
        ptr::from_ref(&*self.native).cast_mut().cast()
    }
    fn parameter(&self, api: &Api) -> RawParameter {
        let retained = hold(self.pointer()).unwrap();
        let pointer = NonNull::new(retained.0).unwrap();
        std::mem::forget(retained);
        unsafe { RawParameter::from_owned(pointer, raw_text(api)) }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        assert_eq!(
            self.native.refs.get(),
            1,
            "all observation references released"
        );
        assert_eq!(unsafe { release_mock(self.pointer()) }, 0);
    }
}

fn reply(context: Box<Context<'static>>, owner: OwnerInspection) -> Reply {
    let mut value = owner.value();
    value["ok"] = json!(true);
    value["source"] = json!("dsp");
    value["instance"] = json!(context.instance);
    Reply {
        value,
        menu: None,
        catalog: None,
        edit: None,
        action: None,
        presets: None,
        owner: Some(owner),
        gui: None,
        auxiliary: None,
        folder_preflight: None,
        nodes: None,
        provider: None,
        context: Some(context),
        mutation: false,
    }
}

#[test]
fn parameter_native_class_is_copied_from_the_retained_parameter_without_value_ffi() {
    let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut host = guard_tests::Fixture::new();
    let context = host.context();
    let parameter = Fixture::new();
    let owner = Fixture::new();
    parameter.native.controller.set(owner.pointer());
    parameter.native.object.set(true);
    parameter
        .native
        .native_class
        .set(c"DeviceParamFolder".as_ptr());
    let lease = OwnerInspection::capture(parameter.parameter(context.api), text(), &mut || {
        context.check()
    })
    .unwrap();
    assert_eq!(parameter.native.class_reads.get(), 1);
    parameter.native.native_class.set(c"ChangedClass".as_ptr());
    assert_eq!(lease.value()["parameter_native_class"], "DeviceParamFolder");
    assert_eq!(lease.value()["parameter_native_class"], "DeviceParamFolder");
    assert_eq!(parameter.native.class_reads.get(), 1);
    assert_eq!(owner.native.class_reads.get(), 0);
    lease.release(&mut || context.check()).unwrap();
    assert_eq!(parameter.native.refs.get(), 1);
    assert_eq!(owner.native.refs.get(), 1);
    assert_eq!(parameter.native.updates.get(), 0);
    assert_eq!(owner.native.updates.get(), 0);
}

#[test]
fn parameter_native_class_cleanup_reentry_discards_capture_and_releases_pins() {
    let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut host = guard_tests::Fixture::new();
    let context = host.context();
    let parameter = Fixture::new();
    let owner = Fixture::new();
    parameter.native.controller.set(owner.pointer());
    parameter.native.object.set(true);
    parameter
        .native
        .native_class
        .set(c"DeviceParamFolder".as_ptr());
    parameter.native.stop_after_class.set(context.service);
    assert!(
        OwnerInspection::capture(parameter.parameter(context.api), text(), &mut || context
            .check())
        .is_err()
    );
    assert_eq!(parameter.native.class_reads.get(), 1);
    assert_eq!(parameter.native.refs.get(), 1);
    assert_eq!(owner.native.refs.get(), 1);
    assert_eq!(parameter.native.updates.get(), 0);
    assert_eq!(owner.native.updates.get(), 0);
}

#[test]
fn nonnode_controller_metadata_is_bounded_readonly_and_retained_through_publication() {
    let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut host = guard_tests::Fixture::new();
    let context = host.context();
    let parameter = Fixture::new();
    let owner = Fixture::new();
    parameter.native.controller.set(owner.pointer());
    let lease = OwnerInspection::capture(parameter.parameter(context.api), text(), &mut || {
        context.check()
    })
    .unwrap();
    let value = lease.value();
    assert_eq!(value["owner_scope"], "parameter_controller_observation");
    assert_eq!(value["mutation_capability"], false);
    assert_eq!(value["parameter_native_class"], "");
    assert_eq!(
        value["controller_report"]["limits"],
        json!({"nodes":1,"depth":0,"ancestors":0,"children_per_node":1})
    );
    assert_eq!(
        value["controller_report"]["nodes"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(value["controller_report"]["nodes"][0]["identity"].is_null());
    assert_eq!(
        value["controller_report"]["nodes"][0]["reflection"]["available"],
        false
    );
    assert_eq!(
        value["controller_report"]["nodes"][0]["reflection"]["issues"],
        json!(["no_object_interface"])
    );
    assert_eq!(value["controller_report"]["complete"], false);
    let service = context.service;
    let reply = reply(context, lease);
    let mut out = VariantOut {
        kind: 0,
        payload: 0,
    };
    assert!(publish_reply(&mut out, reply, api().unwrap(), service));
    assert_eq!(LIVE_RECEIPTS.load(Ordering::Acquire), 1);
    assert!(owner.native.refs.get() > 1);
    assert!(parameter.native.refs.get() > 1);
    release_receipt(service);
    assert_eq!(LIVE_RECEIPTS.load(Ordering::Acquire), 0);
    assert_eq!(owner.native.refs.get(), 1);
    assert_eq!(parameter.native.refs.get(), 1);
    assert_eq!(parameter.native.updates.get(), 0);
    assert_eq!(owner.native.updates.get(), 0);
    super::super::super::tests::clear_output(&mut out);
}

#[test]
fn stable_null_owner_is_retained_and_a_new_owner_before_publication_invalidates_it() {
    let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut host = guard_tests::Fixture::new();
    let context = host.context();
    let parameter = Fixture::new();
    let owner = Fixture::new();
    let lease = OwnerInspection::capture(parameter.parameter(context.api), text(), &mut || {
        context.check()
    })
    .unwrap();
    assert!(lease.value()["controller"].is_null());
    assert!(lease.value().get("controller_report").is_none());
    assert!(parameter.native.refs.get() > 1);
    let service = context.service;
    let mut reply = reply(context, lease);
    parameter.native.controller.set(owner.pointer());
    assert!(!reply.final_check(service));
    assert_eq!(reply.value["ok"], false);
    assert!(reply.value.get("controller").is_none());
    assert!(reply.value.get("controller_report").is_none());
    assert!(reply.value.get("owner_scope").is_none());
    assert!(reply.value.get("parameter_native_class").is_none());
    drop(reply);
    assert_eq!(parameter.native.refs.get(), 1);
    assert_eq!(owner.native.refs.get(), 1);
}

#[test]
fn changed_owner_before_publication_discards_metadata_and_releases_both_owners() {
    let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut host = guard_tests::Fixture::new();
    let context = host.context();
    let parameter = Fixture::new();
    let original = Fixture::new();
    let replacement = Fixture::new();
    parameter.native.controller.set(original.pointer());
    let lease = OwnerInspection::capture(parameter.parameter(context.api), text(), &mut || {
        context.check()
    })
    .unwrap();
    let service = context.service;
    let mut reply = reply(context, lease);
    parameter.native.controller.set(replacement.pointer());
    assert!(!reply.final_check(service));
    assert!(reply.value.get("controller_report").is_none());
    assert!(reply.value.get("parameter_native_class").is_none());
    assert_eq!(reply.value["outcome"], "not_applied");
    drop(reply);
    assert_eq!(original.native.refs.get(), 1);
    assert_eq!(replacement.native.refs.get(), 1);
    assert_eq!(parameter.native.refs.get(), 1);
}

#[test]
fn parameter_release_reentry_after_receipt_is_checked_by_the_next_context() {
    let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut host = guard_tests::Fixture::new();
    let context = host.context();
    let parameter = Fixture::new();
    let owner = Fixture::new();
    parameter.native.controller.set(owner.pointer());
    let lease = OwnerInspection::capture(parameter.parameter(context.api), text(), &mut || {
        context.check()
    })
    .unwrap();
    let service = context.service;
    let mut out = VariantOut {
        kind: 0,
        payload: 0,
    };
    assert!(publish_reply(
        &mut out,
        reply(context, lease),
        api().unwrap(),
        service
    ));
    let next = host.context();
    parameter.native.stop_on_release.set(service);
    assert!(release_previous(&next).is_err());
    assert_eq!(LIVE_RECEIPTS.load(Ordering::Acquire), 0);
    assert_eq!(parameter.native.refs.get(), 1);
    assert_eq!(owner.native.refs.get(), 1);
    drop(next);
    super::super::super::tests::clear_output(&mut out);
}

#[test]
fn output_reentry_and_unavailable_type_info_never_publish_controller_success() {
    let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut host = guard_tests::Fixture::new();
    let context = host.context();
    let parameter = Fixture::new();
    let owner = Fixture::new();
    parameter.native.controller.set(owner.pointer());
    owner.native.object.set(true);
    assert!(
        OwnerInspection::capture(parameter.parameter(context.api), text(), &mut || context
            .check())
        .is_err()
    );
    assert_eq!(parameter.native.refs.get(), 1);
    assert_eq!(owner.native.refs.get(), 1);
    owner.native.object.set(false);
    let lease = OwnerInspection::capture(parameter.parameter(context.api), text(), &mut || {
        context.check()
    })
    .unwrap();
    let service = context.service;
    super::super::super::tests::ASSIGN_REENTER_STOP.store(service, Ordering::Release);
    let mut out = VariantOut {
        kind: 0,
        payload: 0,
    };
    assert!(publish_reply(
        &mut out,
        reply(context, lease),
        api().unwrap(),
        service
    ));
    let value: Value =
        serde_json::from_str(&super::super::super::tests::output_text(&out)).unwrap();
    assert_eq!(value["ok"], false);
    assert!(value.get("controller_report").is_none());
    assert_eq!(LIVE_RECEIPTS.load(Ordering::Acquire), 0);
    assert_eq!(owner.native.refs.get(), 1);
    assert_eq!(parameter.native.refs.get(), 1);
    super::super::super::tests::clear_output(&mut out);
}

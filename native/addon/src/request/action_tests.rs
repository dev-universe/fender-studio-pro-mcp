use super::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

const OBJECT: Uid = Uid {
    a: 0x2e726012,
    b: 0x522c,
    c: 0x4108,
    d: [0x98, 0x62, 0x0c, 0xd5, 0x71, 0x79, 0x52, 0x49],
};
const NAME: PlainCString = PlainCString {
    text: c"swapfreqs".as_ptr(),
    owner: ptr::null_mut(),
};
const VALUE: PlainCString = PlainCString {
    text: c"value".as_ptr(),
    owner: ptr::null_mut(),
};
const MIN: PlainCString = PlainCString {
    text: c"min".as_ptr(),
    owner: ptr::null_mut(),
};
const MAX: PlainCString = PlainCString {
    text: c"max".as_ptr(),
    owner: ptr::null_mut(),
};

#[repr(C)]
struct Object {
    table: *const *const c_void,
    parameter: *mut Parameter,
}
#[repr(C)]
struct Parameter {
    table: *const *const c_void,
    refs: Cell<u32>,
    object: Object,
    owner: Cell<*mut c_void>,
    next_owner: Cell<*mut c_void>,
    current: Cell<i64>,
    enabled: Cell<bool>,
    readonly: Cell<bool>,
    writes: RefCell<Vec<(i64, bool)>>,
    context: Rc<Cell<bool>>,
    change_after_upper: Cell<u8>,
    ignore_reset: Cell<bool>,
    auto_reset: Cell<bool>,
    lookup: *mut Lookup,
}
#[repr(C)]
struct Lookup {
    table: *const *const c_void,
    refs: Cell<u32>,
    parameter: Cell<*mut c_void>,
    find_calls: Cell<u32>,
    owner_on_find: Cell<Option<*mut c_void>>,
}
unsafe fn parameter<'a>(p: *mut c_void) -> &'a Parameter {
    unsafe { &*p.cast::<Parameter>() }
}
unsafe extern "C" fn retain(p: *mut c_void) -> u32 {
    let p = unsafe { parameter(p) };
    p.refs.set(p.refs.get() + 1);
    p.refs.get()
}
unsafe extern "C" fn release_mock(p: *mut c_void) -> u32 {
    let p = unsafe { parameter(p) };
    p.refs.set(p.refs.get() - 1);
    p.refs.get()
}
unsafe extern "C" fn object_retain(p: *mut c_void) -> u32 {
    unsafe { retain((*p.cast::<Object>()).parameter.cast()) }
}
unsafe extern "C" fn object_release(p: *mut c_void) -> u32 {
    unsafe { release_mock((*p.cast::<Object>()).parameter.cast()) }
}
unsafe extern "C" fn query_mock(p: *mut c_void, iid: *const Uid, out: *mut *mut c_void) -> i32 {
    let f = unsafe { parameter(p) };
    unsafe { *out = ptr::null_mut() };
    if unsafe { *iid } == UNKNOWN {
        unsafe {
            retain(p);
            *out = p
        };
        OK
    } else if unsafe { *iid } == OBJECT {
        unsafe {
            retain(p);
            *out = ptr::addr_of!(f.object).cast_mut().cast()
        };
        OK
    } else {
        NO_INTERFACE
    }
}
unsafe extern "C" fn kind(_: *mut c_void) -> i32 {
    0
}
unsafe extern "C" fn name(_: *mut c_void) -> *const PlainCString {
    &NAME
}
unsafe extern "C" fn tag(_: *mut c_void) -> i32 {
    17
}
unsafe extern "C" fn enabled(p: *mut c_void) -> u8 {
    u8::from(unsafe { parameter(p) }.enabled.get())
}
unsafe extern "C" fn readonly(p: *mut c_void, mask: i32) -> u8 {
    assert_eq!(mask, 0x1000);
    u8::from(unsafe { parameter(p) }.readonly.get())
}
unsafe extern "C" fn owner(p: *mut c_void) -> *mut c_void {
    unsafe { parameter(p) }.owner.get()
}
unsafe extern "C" fn property(p: *mut c_void, out: *mut VariantOut, id: *const PlainCString) -> u8 {
    let f = unsafe { &*(*p.cast::<Object>()).parameter };
    let value = match unsafe { CStr::from_ptr((*id).text) } {
        id if id == c"value" => f.current.get(),
        id if id == c"min" => 0,
        id if id == c"max" => 1,
        _ => panic!("button action must not call other properties"),
    };
    unsafe {
        (*out).kind = 1;
        (*out).payload = value as u64;
    }
    1
}
unsafe extern "C" fn set(p: *mut c_void, value: *const VariantOut, notify: u8) {
    let f = unsafe { parameter(p) };
    assert_eq!(unsafe { (*value).kind }, 1);
    let value = unsafe { (*value).payload } as i64;
    let upper = f.writes.borrow().is_empty();
    f.writes.borrow_mut().push((value, notify != 0));
    if upper || !f.ignore_reset.get() {
        f.current.set(value);
    }
    if upper {
        if f.auto_reset.get() {
            f.current.set(0);
        }
        match f.change_after_upper.get() {
            1 => f.context.set(false),
            2 => f.owner.set(f.next_owner.get()),
            3 => unsafe { (*f.lookup).parameter.set(ptr::null_mut()) },
            _ => (),
        }
    }
}
unsafe extern "C" fn lookup_retain(p: *mut c_void) -> u32 {
    let p = unsafe { &*p.cast::<Lookup>() };
    p.refs.set(p.refs.get() + 1);
    p.refs.get()
}
unsafe extern "C" fn lookup_release(p: *mut c_void) -> u32 {
    let p = unsafe { &*p.cast::<Lookup>() };
    p.refs.set(p.refs.get() - 1);
    p.refs.get()
}
unsafe extern "C" fn find(p: *mut c_void, name: *const PlainCString) -> *mut c_void {
    let f = unsafe { &*p.cast::<Lookup>() };
    f.find_calls.set(f.find_calls.get() + 1);
    assert!(
        matches!(unsafe{CStr::from_ptr((*name).text)},n if n==c"swapfreqs" || n==c"studiocomp.swapfreqs")
    );
    let result = f.parameter.get();
    if let Some(owner) = f.owner_on_find.take() {
        unsafe { parameter(result) }.owner.set(owner);
    }
    result
}
unsafe extern "C" fn text(name: *const c_char) -> *const PlainCString {
    match unsafe { CStr::from_ptr(name) } {
        n if n == c"value" => &VALUE,
        n if n == c"min" => &MIN,
        n if n == c"max" => &MAX,
        _ => panic!("unexpected button text service"),
    }
}
unsafe extern "C" fn empty() -> *mut c_void {
    panic!("button action must not create strings")
}
unsafe extern "C" fn thread() -> i64 {
    0
}
fn api_mock() -> Api {
    Api {
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
    }
}
struct Fixture {
    parameter: Box<Parameter>,
    lookup: Box<Lookup>,
    _parameter_table: Box<[*const c_void; 22]>,
    _object_table: Box<[*const c_void; 5]>,
    _lookup_table: Box<[*const c_void; 6]>,
}
impl Fixture {
    fn new() -> Self {
        let mut pt = Box::new([ptr::null(); 22]);
        pt[0] = query_mock as *const c_void;
        pt[1] = retain as *const c_void;
        pt[2] = release_mock as *const c_void;
        pt[3] = kind as *const c_void;
        pt[4] = name as *const c_void;
        pt[6] = enabled as *const c_void;
        pt[8] = readonly as *const c_void;
        pt[13] = tag as *const c_void;
        pt[14] = owner as *const c_void;
        pt[21] = set as *const c_void;
        let mut ot = Box::new([ptr::null(); 5]);
        ot[1] = object_retain as *const c_void;
        ot[2] = object_release as *const c_void;
        ot[4] = property as *const c_void;
        let mut lt = Box::new([ptr::null(); 6]);
        lt[1] = lookup_retain as *const c_void;
        lt[2] = lookup_release as *const c_void;
        lt[5] = find as *const c_void;
        let mut lookup = Box::new(Lookup {
            table: lt.as_ptr(),
            refs: Cell::new(1),
            parameter: Cell::new(ptr::null_mut()),
            find_calls: Cell::new(0),
            owner_on_find: Cell::new(None),
        });
        let mut parameter = Box::new(Parameter {
            table: pt.as_ptr(),
            refs: Cell::new(1),
            object: Object {
                table: ot.as_ptr(),
                parameter: ptr::null_mut(),
            },
            owner: Cell::new(ptr::null_mut()),
            next_owner: Cell::new(ptr::null_mut()),
            current: Cell::new(0),
            enabled: Cell::new(true),
            readonly: Cell::new(false),
            writes: RefCell::new(vec![]),
            context: Rc::new(Cell::new(true)),
            change_after_upper: Cell::new(0),
            ignore_reset: Cell::new(false),
            auto_reset: Cell::new(false),
            lookup: ptr::from_mut(&mut *lookup),
        });
        parameter.object.parameter = ptr::from_mut(&mut *parameter);
        lookup.parameter.set(ptr::from_mut(&mut *parameter).cast());
        Self {
            parameter,
            lookup,
            _parameter_table: pt,
            _object_table: ot,
            _lookup_table: lt,
        }
    }
    fn pointer(&self) -> *mut c_void {
        ptr::from_ref(&*self.parameter).cast_mut().cast()
    }
    fn provider(&self) -> Provider {
        Provider::DspValue(Controller {
            handle: hold(ptr::from_ref(&*self.lookup).cast_mut().cast()).unwrap(),
            lease: None,
        })
    }
    fn run(&self, provider: &Provider) -> (Value, Option<ActionLease>) {
        run(
            provider.scalar().unwrap(),
            &api_mock(),
            Action::SwapFilterFrequencies,
            registered(
                "831A0046-BD71-45BF-A419-6FC90D6E9F2F",
                Action::SwapFilterFrequencies,
            )
            .unwrap(),
            &mut || {
                self.parameter
                    .context
                    .get()
                    .then_some(())
                    .ok_or_else(|| "synthetic context changed".into())
            },
        )
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        assert_eq!(self.parameter.refs.get(), 1);
        assert_eq!(self.lookup.refs.get(), 1);
    }
}

#[test]
fn request_accepts_only_the_registered_action_shape_and_required_instance() {
    let base = json!({"operation":"parameter_action","action":"swap_filter_frequencies","expected_instance":"12345678-1234-1234-1234-123456789ABC"});
    let request: Request = serde_json::from_value(base.clone()).unwrap();
    request.validate().unwrap();
    assert_eq!(request.source(), Source::Dsp);
    assert!(!request.needs_auxiliary());
    for key in ["source", "param", "value", "method", "path", "url"] {
        let mut value = base.clone();
        value[key] = json!("unexpected");
        assert!(serde_json::from_value::<Request>(value).is_err());
    }
    for value in [
        Value::Null,
        json!(""),
        json!(ZERO_UID),
        json!("12345678-1234-1234-1234-123456789abc"),
    ] {
        let mut request = base.clone();
        request["expected_instance"] = value;
        assert!(match serde_json::from_value::<Request>(request) {
            Ok(request) => request.validate().is_err(),
            Err(_) => true,
        });
    }
    let mut missing = base.clone();
    missing.as_object_mut().unwrap().remove("expected_instance");
    assert!(serde_json::from_value::<Request>(missing).is_err());
    let mut other = base;
    other["action"] = json!("reset_minimum");
    assert!(serde_json::from_value::<Request>(other).is_err());
}
#[test]
fn class_registry_is_exact_and_uses_verified_canonical_fields() {
    let action = Action::SwapFilterFrequencies;
    for class in [
        "831A0046-BD71-45BF-A419-6FC90D6E9F2F",
        "132C0E1F-06FD-4C96-BD65-678C04FE1C92",
    ] {
        let spec = registered(class, action).unwrap();
        assert_eq!(spec.parameter, c"swapfreqs");
        assert_eq!(spec.effects, ["filterlow", "filterhigh"]);
    }
    let compressor = registered("36F3F4D1-CBB4-4BF7-A7E3-EBCCED53718B", action).unwrap();
    assert_eq!(compressor.parameter, c"studiocomp.swapfreqs");
    assert_eq!(
        compressor.effects,
        [
            "studiocomp.sidechainfreqlow",
            "studiocomp.sidechainfreqhigh"
        ]
    );
    for class in [
        ZERO_UID,
        "831a0046-bd71-45bf-a419-6fc90d6e9f2f",
        "Expander",
        "",
        "831A0046-BD71-45BF-A419-6FC90D6E9F20",
    ] {
        assert!(registered(class, action).is_err());
    }
}
#[test]
fn button_sequence_is_unverified_even_when_reset_reads_exactly() {
    for auto_reset in [false, true] {
        let fixture = Fixture::new();
        fixture.parameter.auto_reset.set(auto_reset);
        let provider = fixture.provider();
        let (value, lease) = fixture.run(&provider);
        assert_eq!(value["ok"], true);
        assert_eq!(value["outcome"], "applied_unverified");
        assert_eq!(value["verified"], false);
        assert_eq!(value["effects_verified"], false);
        assert_eq!(value["reset_verified"], true);
        assert_eq!(
            &*fixture.parameter.writes.borrow(),
            &[(1, true), (0, false)]
        );
        assert_eq!(
            value["effect_parameters"],
            json!(["filterlow", "filterhigh"])
        );
        assert_eq!(value["after"]["native_value"]["data"]["value"], 0);
        drop(lease);
        drop(provider);
    }
}
#[test]
fn unsupported_access_stops_before_upper_dispatch() {
    let fixture = Fixture::new();
    fixture.parameter.readonly.set(true);
    let provider = fixture.provider();
    let (value, lease) = fixture.run(&provider);
    assert_eq!(value["outcome"], "not_applied");
    assert_eq!(value["upper_dispatched"], false);
    assert!(fixture.parameter.writes.borrow().is_empty());
    drop(lease);
    drop(provider);
}
#[test]
fn context_owner_and_binding_changes_after_upper_are_unknown_without_reset() {
    for change in [1, 2, 3] {
        let fixture = Fixture::new();
        let other = Fixture::new();
        fixture.parameter.next_owner.set(other.pointer());
        fixture.parameter.change_after_upper.set(change);
        let provider = fixture.provider();
        let (value, lease) = fixture.run(&provider);
        assert_eq!(value["outcome"], "unknown");
        assert_eq!(value["upper_dispatched"], true);
        assert_eq!(value["reset_dispatched"], false);
        assert_eq!(&*fixture.parameter.writes.borrow(), &[(1, true)]);
        assert_eq!(value["effects_verified"], false);
        drop(lease);
        drop(provider);
    }
}
#[test]
fn a_failed_reset_is_unknown_and_does_not_retry() {
    let fixture = Fixture::new();
    fixture.parameter.ignore_reset.set(true);
    let provider = fixture.provider();
    let (value, lease) = fixture.run(&provider);
    assert_eq!(value["outcome"], "unknown");
    assert_eq!(value["reset_dispatched"], true);
    assert_eq!(value["reset_verified"], false);
    assert_eq!(
        &*fixture.parameter.writes.borrow(),
        &[(1, true), (0, false)]
    );
    drop(lease);
    drop(provider);
}
#[test]
fn a_present_owner_is_retained_and_checked_through_the_action() {
    let fixture = Fixture::new();
    let owner = Fixture::new();
    fixture.parameter.owner.set(owner.pointer());
    let provider = fixture.provider();
    let (value, lease) = fixture.run(&provider);
    assert_eq!(value["ok"], true);
    assert!(owner.parameter.refs.get() > 1);
    drop(lease);
    assert_eq!(owner.parameter.refs.get(), 1);
    drop(provider);
}
fn reply(
    context: Box<Context<'static>>,
    provider: Provider,
    value: Value,
    lease: ActionLease,
) -> Reply {
    Reply {
        value,
        catalog: None,
        edit: None,
        action: Some(lease),
        presets: None,
        owner: None,
        gui: None,
        auxiliary: None,
        folder_preflight: None,
        nodes: None,
        provider: Some(provider),
        context: Some(context),
        mutation: true,
    }
}
#[test]
fn action_parameter_and_owner_survive_output_until_receipt_release() {
    let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut host = guard_tests::Fixture::new();
    let context = host.context();
    let fixture = Fixture::new();
    let owner = Fixture::new();
    fixture.parameter.owner.set(owner.pointer());
    let provider = fixture.provider();
    let (value, lease) = fixture.run(&provider);
    assert_eq!(value["ok"], true);
    let service = context.service;
    let reply = reply(context, provider, value, lease.unwrap());
    let mut out = VariantOut {
        kind: 0,
        padding: [0; 6],
        payload: 0,
    };
    assert!(publish_reply(&mut out, reply, api().unwrap(), service));
    let value: Value =
        serde_json::from_str(&super::super::super::tests::output_text(&out)).unwrap();
    assert_eq!(value["outcome"], "applied_unverified");
    assert!(owner.parameter.refs.get() > 1);
    assert!(fixture.parameter.refs.get() > 1);
    release_receipt(service);
    assert_eq!(owner.parameter.refs.get(), 1);
    assert_eq!(fixture.parameter.refs.get(), 1);
    super::super::super::tests::clear_output(&mut out);
}
#[test]
fn owner_replacement_before_publication_cannot_publish_success() {
    let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut host = guard_tests::Fixture::new();
    let context = host.context();
    let fixture = Fixture::new();
    let owner = Fixture::new();
    let provider = fixture.provider();
    let (value, lease) = fixture.run(&provider);
    let service = context.service;
    let mut reply = reply(context, provider, value, lease.unwrap());
    fixture.parameter.owner.set(owner.pointer());
    assert!(!reply.final_check(service));
    assert_eq!(reply.value["outcome"], "unknown");
    assert_eq!(reply.value["ok"], false);
    assert_eq!(reply.value["effects_verified"], false);
    drop(reply);
}
#[test]
fn output_reentry_turns_an_action_into_unknown_without_another_pulse() {
    let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut host = guard_tests::Fixture::new();
    let context = host.context();
    let service = context.service;
    let fixture = Fixture::new();
    let provider = fixture.provider();
    let (value, lease) = fixture.run(&provider);
    let reply = reply(context, provider, value, lease.unwrap());
    super::super::super::tests::ASSIGN_REENTER_STOP.store(service, Ordering::Release);
    let mut out = VariantOut {
        kind: 0,
        padding: [0; 6],
        payload: 0,
    };
    assert!(publish_reply(&mut out, reply, api().unwrap(), service));
    let value: Value =
        serde_json::from_str(&super::super::super::tests::output_text(&out)).unwrap();
    assert_eq!(value["outcome"], "unknown");
    assert_eq!(value["effects_verified"], false);
    assert_eq!(
        &*fixture.parameter.writes.borrow(),
        &[(1, true), (0, false)]
    );
    super::super::super::tests::clear_output(&mut out);
}

#[test]
fn unregistered_classes_and_non_dsp_providers_never_find_a_button() {
    let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut host = guard_tests::Fixture::new();
    let mut context = host.context();
    let fixture = Fixture::new();
    let provider = fixture.provider();
    let (value, lease) = execute(&context, &provider, Action::SwapFilterFrequencies);
    assert_eq!(value["error"]["code"], "unregistered_action");
    assert_eq!(value["outcome"], "not_applied");
    assert_eq!(value["upper_dispatched"], false);
    assert!(lease.is_none());
    assert_eq!(fixture.lookup.find_calls.get(), 0);
    let Provider::DspValue(controller) = provider else {
        unreachable!()
    };
    let provider = Provider::Controller(controller);
    context.instance.class_uid = "831A0046-BD71-45BF-A419-6FC90D6E9F2F".into();
    let (value, lease) = execute(&context, &provider, Action::SwapFilterFrequencies);
    assert_eq!(value["error"]["code"], "unsupported_source");
    assert_eq!(value["outcome"], "not_applied");
    assert_eq!(value["upper_dispatched"], false);
    assert!(lease.is_none());
    assert_eq!(fixture.lookup.find_calls.get(), 0);
    assert!(fixture.parameter.writes.borrow().is_empty());
    drop(provider);
}

#[test]
fn binding_change_during_post_lookup_guard_is_refused() {
    let fixture = Fixture::new();
    let provider = fixture.provider();
    let (value, lease) = fixture.run(&provider);
    assert_eq!(value["ok"], true);
    let lease = lease.unwrap();
    let before = fixture.lookup.find_calls.get();
    let result = lease.check(provider.scalar().unwrap(), &mut || {
        if fixture.lookup.find_calls.get() > before {
            fixture.lookup.parameter.set(ptr::null_mut());
        }
        Ok(())
    });
    assert!(fixture.lookup.parameter.get().is_null());
    assert!(
        result.is_err(),
        "binding changed during the final guard but lease validation accepted it"
    );
    drop(lease);
    drop(provider);
}

#[test]
fn owner_change_during_first_binding_lookup_is_refused() {
    let fixture = Fixture::new();
    let owner = Fixture::new();
    let provider = fixture.provider();
    let (value, lease) = fixture.run(&provider);
    assert_eq!(value["ok"], true);
    let lease = lease.unwrap();
    fixture.lookup.owner_on_find.set(Some(owner.pointer()));
    assert!(
        lease
            .check(provider.scalar().unwrap(), &mut || Ok(()))
            .is_err()
    );
    assert!(fixture.lookup.owner_on_find.get().is_none());
    drop(lease);
    drop(provider);
}

#[test]
fn owner_change_during_last_guard_is_refused() {
    let fixture = Fixture::new();
    let owner = Fixture::new();
    let provider = fixture.provider();
    let (value, lease) = fixture.run(&provider);
    assert_eq!(value["ok"], true);
    let lease = lease.unwrap();
    let mut guards = 0;
    lease
        .check(provider.scalar().unwrap(), &mut || {
            guards += 1;
            Ok(())
        })
        .unwrap();
    let last_guard = guards;
    let mut current_guard = 0;
    let result = lease.check(provider.scalar().unwrap(), &mut || {
        current_guard += 1;
        if current_guard == last_guard {
            fixture.parameter.owner.set(owner.pointer());
        }
        Ok(())
    });
    assert_eq!(fixture.parameter.owner.get(), owner.pointer());
    assert!(
        result.is_err(),
        "owner changed during final guard but action lease accepted it"
    );
    drop(lease);
    drop(provider);
}

thread_local! {
    static PUBLICATION_ACTIVE_ORIGINAL: Cell<*const c_void> = const { Cell::new(ptr::null()) };
    static PUBLICATION_ACTIVE_CALLS: Cell<usize> = const { Cell::new(0) };
    static PUBLICATION_ACTIVE_TRIGGER: Cell<usize> = const { Cell::new(0) };
    static PUBLICATION_ACTIVE_PARAMETER: Cell<*mut Parameter> = const { Cell::new(ptr::null_mut()) };
    static PUBLICATION_ACTIVE_OWNER: Cell<*mut c_void> = const { Cell::new(ptr::null_mut()) };
}
unsafe extern "C" fn active_with_owner_reentry(p: *mut c_void) -> *mut c_void {
    let observed = PUBLICATION_ACTIVE_CALLS.with(|calls| {
        let count = calls.get() + 1;
        calls.set(count);
        count
    });
    if observed == PUBLICATION_ACTIVE_TRIGGER.with(Cell::get) {
        let parameter = PUBLICATION_ACTIVE_PARAMETER.with(Cell::get);
        unsafe {
            (*parameter)
                .owner
                .set(PUBLICATION_ACTIVE_OWNER.with(Cell::get));
        }
    }
    let original: unsafe extern "C" fn(*mut c_void) -> *mut c_void =
        unsafe { std::mem::transmute(PUBLICATION_ACTIVE_ORIGINAL.with(Cell::get)) };
    unsafe { original(p) }
}
#[test]
fn common_context_reentry_cannot_publish_a_stale_action_owner() {
    let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut host = guard_tests::Fixture::new();
    let context = host.context();
    let service = context.service;
    let manager = context.document.manager.0;
    let original_table = unsafe { *manager.cast::<*const *const c_void>() };
    let mut table = unsafe { std::slice::from_raw_parts(original_table, 10) }.to_vec();
    PUBLICATION_ACTIVE_ORIGINAL.with(|value| value.set(table[9]));
    table[9] = active_with_owner_reentry as *const c_void;
    unsafe {
        *manager.cast::<*const *const c_void>() = table.as_ptr();
    }
    let fixture = Fixture::new();
    let owner = Fixture::new();
    let provider = fixture.provider();
    let (value, lease) = fixture.run(&provider);
    let mut reply = reply(context, provider, value, lease.unwrap());
    PUBLICATION_ACTIVE_CALLS.with(|calls| calls.set(0));
    PUBLICATION_ACTIVE_TRIGGER.with(|trigger| trigger.set(0));
    assert!(reply.final_check(service));
    let last_active = PUBLICATION_ACTIVE_CALLS.with(Cell::get);
    PUBLICATION_ACTIVE_CALLS.with(|calls| calls.set(0));
    PUBLICATION_ACTIVE_TRIGGER.with(|trigger| trigger.set(last_active));
    PUBLICATION_ACTIVE_PARAMETER.with(|parameter| parameter.set(fixture.pointer().cast()));
    PUBLICATION_ACTIVE_OWNER.with(|target| target.set(owner.pointer()));
    let valid = reply.final_check(service);
    let owner_changed = fixture.parameter.owner.get() == owner.pointer();
    unsafe {
        *manager.cast::<*const *const c_void>() = original_table;
    }
    PUBLICATION_ACTIVE_TRIGGER.with(|value| value.set(0));
    PUBLICATION_ACTIVE_PARAMETER.with(|value| value.set(ptr::null_mut()));
    PUBLICATION_ACTIVE_OWNER.with(|value| value.set(ptr::null_mut()));
    PUBLICATION_ACTIVE_ORIGINAL.with(|value| value.set(ptr::null()));
    drop(reply);
    assert!(owner_changed);
    assert!(
        !valid,
        "last common context callback replaced owner after the action check"
    );
}

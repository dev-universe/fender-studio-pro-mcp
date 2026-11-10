use super::*;
use std::cell::{Cell, RefCell};

#[test]
fn comparison_opt_in_is_strict_and_finite() {
    let base = json!({"operation":"parameter_details","param":PARAMETER,"source":"dsp","expected_instance":"61D64D66-2A46-4BEE-A623-EC5CEB898313"});
    for flag in [false, true] {
        let mut value = base.clone();
        value["compare_gui"] = json!(flag);
        assert!(
            serde_json::from_value::<Request>(value)
                .unwrap()
                .validate()
                .is_ok()
        );
    }
    assert!(matches!(
        serde_json::from_value::<Request>(base.clone()).unwrap(),
        Request::ParameterDetails {
            compare_gui: false,
            ..
        }
    ));
    for flag in [Value::Null, json!(1), json!("true"), json!([])] {
        let mut value = base.clone();
        value["compare_gui"] = flag;
        assert!(serde_json::from_value::<Request>(value).is_err());
    }
    for (field, bad) in [
        ("param", json!("autofill")),
        ("source", json!("controller")),
        ("source", json!("auto")),
        ("expected_instance", json!(ZERO_UID)),
    ] {
        let mut value = base.clone();
        value["compare_gui"] = json!(true);
        value[field] = bad;
        assert!(
            serde_json::from_value::<Request>(value)
                .unwrap()
                .validate()
                .is_err()
        );
    }
}
#[test]
fn final_invalidation_keeps_coverage_and_removes_all_identity_claims() {
    let mut value = json!({"ok":true,"gui_comparison":{"status":"compared","topParameterSame":true,"declaredObserverSame":true,"originalSame":null,"coverage":{"complete":true,"candidate_count":1,"visited_views":7,"max_views":128,"max_depth":16}}});
    invalidate(&mut value);
    assert_eq!(value["gui_comparison"]["reason"], "stale_association");
    assert_eq!(value["gui_comparison"]["coverage"]["visited_views"], 7);
    for field in [
        "topParameterSame",
        "declaredObserverSame",
        "declaredObserverPresence",
        "originalSame",
        "editorMatch",
    ] {
        assert!(value["gui_comparison"].get(field).is_none());
    }
    assert_eq!(value["gui_comparison"]["mutation_capability"], false);
}

#[repr(C)]
struct Fake {
    table: *const *const c_void,
    refs: Cell<u32>,
    canonical: Cell<*mut c_void>,
    control: Cell<*mut c_void>,
    object: Cell<*mut c_void>,
    parameter: Cell<*mut c_void>,
    observer: Cell<*mut c_void>,
    children: RefCell<Vec<*mut c_void>>,
    cursor: Cell<usize>,
    name: PlainCString,
    qi_failure: Cell<bool>,
    qi_status: Cell<i32>,
    detach: Cell<bool>,
    stop_on_release: Cell<*const Cell<bool>>,
    mutations: Cell<u32>,
}
unsafe fn f<'a>(p: *mut c_void) -> &'a Fake {
    unsafe { &*p.cast::<Fake>() }
}
unsafe extern "C" fn retain_mock(p: *mut c_void) -> u32 {
    let x = unsafe { f(p) };
    x.refs.set(x.refs.get() + 1);
    x.refs.get()
}
unsafe extern "C" fn release_mock(p: *mut c_void) -> u32 {
    let x = unsafe { f(p) };
    x.refs.set(x.refs.get() - 1);
    let stop = x.stop_on_release.replace(ptr::null());
    if !stop.is_null() {
        unsafe { &*stop }.set(true);
    }
    x.refs.get()
}
unsafe extern "C" fn query_mock(p: *mut c_void, iid: *const Uid, out: *mut *mut c_void) -> i32 {
    let x = unsafe { f(p) };
    unsafe { *out = ptr::null_mut() };
    let iid = unsafe { *iid };
    if x.qi_failure.get() && iid == CONTROL {
        return x.qi_status.get();
    }
    let raw = if iid == UNKNOWN {
        if x.canonical.get().is_null() {
            p
        } else {
            x.canonical.get()
        }
    } else if iid == VIEW {
        p
    } else if iid == OBJECT {
        x.object.get()
    } else if iid == CONTROL {
        x.control.get()
    } else {
        ptr::null_mut()
    };
    if raw.is_null() {
        NO_INTERFACE
    } else {
        unsafe {
            retain_mock(raw);
            *out = raw
        };
        OK
    }
}
unsafe extern "C" fn parameter_mock(p: *mut c_void) -> *mut c_void {
    unsafe { f(p) }.parameter.get()
}
unsafe extern "C" fn observer_mock(p: *mut c_void) -> *mut c_void {
    unsafe { f(p) }.observer.get()
}
unsafe extern "C" fn children_mock(p: *mut c_void) -> *mut c_void {
    if unsafe { f(p) }.children.borrow().is_empty() {
        ptr::null_mut()
    } else {
        p
    }
}
unsafe extern "C" fn iterator_mock(p: *mut c_void) -> *mut c_void {
    unsafe { f(p) }.cursor.set(0);
    unsafe { retain_mock(p) };
    p
}
unsafe extern "C" fn done_mock(p: *mut c_void) -> u8 {
    (unsafe { f(p) }.cursor.get() >= unsafe { f(p) }.children.borrow().len()) as u8
}
unsafe extern "C" fn next_mock(p: *mut c_void) -> *mut c_void {
    let x = unsafe { f(p) };
    let n = x.cursor.get();
    x.cursor.set(n + 1);
    x.children.borrow()[n]
}
unsafe extern "C" fn edge_mock(p: *mut c_void, c: *mut c_void, _: u8) -> u8 {
    (!unsafe { f(p) }.detach.get() && unsafe { f(p) }.children.borrow().contains(&c)) as u8
}
unsafe extern "C" fn name_mock(p: *mut c_void) -> *const PlainCString {
    &unsafe { f(p) }.name
}
unsafe extern "C" fn kind_mock(_: *mut c_void) -> i32 {
    99
}
unsafe extern "C" fn zero_i(_: *mut c_void) -> i32 {
    0
}
unsafe extern "C" fn yes(_: *mut c_void) -> u8 {
    1
}
unsafe extern "C" fn state(_: *mut c_void, _: i32) -> u8 {
    0
}
unsafe extern "C" fn no_property(_: *mut c_void, _: *mut c_void, _: *const c_void) -> u8 {
    0
}
unsafe extern "C" fn display(_: *mut c_void, _: *mut c_void) {}
unsafe extern "C" fn private_mock(_: *mut c_void) -> *mut c_void {
    ptr::null_mut()
}
unsafe extern "C" fn copy_mock(_: *mut c_void, out: *mut u16, _: i32) -> i32 {
    unsafe { *out = 0 };
    OK
}
unsafe extern "C" fn mutate(p: *mut c_void) {
    let x = unsafe { f(p) };
    x.mutations.set(x.mutations.get() + 1);
}
thread_local! {static EMPTY:Cell<*mut c_void>=const {Cell::new(ptr::null_mut())};}
unsafe extern "C" fn empty() -> *mut c_void {
    EMPTY.with(Cell::get)
}
unsafe extern "C" fn cstring(p: *const c_char) -> *const PlainCString {
    p.cast()
}
unsafe extern "C" fn thread() -> i64 {
    0
}
fn api() -> Api {
    Api {
        gui: empty,
        objects: empty,
        main_thread: empty,
        thread_self: thread,
        cstring,
        empty,
        desktop: empty,
        scripting: empty,
        plugin_manager: empty,
        file_system: empty,
    }
}
struct Fixture {
    value: Box<Fake>,
    _table: Box<[*const c_void; 46]>,
}
impl Fixture {
    fn new(kind: &str) -> Self {
        let mut table = Box::new([ptr::null(); 46]);
        table[0] = query_mock as *const c_void;
        table[1] = retain_mock as *const c_void;
        table[2] = release_mock as *const c_void;
        match kind {
            "view" => {
                table[20] = children_mock as *const c_void;
                table[3] = done_mock as *const c_void;
                table[4] = next_mock as *const c_void;
                table[11] = iterator_mock as *const c_void;
                table[12] = edge_mock as *const c_void;
            }
            "control" => {
                table[3] = parameter_mock as *const c_void;
            }
            "parameter" => {
                table[3] = kind_mock as *const c_void;
                table[4] = name_mock as *const c_void;
                table[6] = yes as *const c_void;
                table[8] = state as *const c_void;
                table[12] = name_mock as *const c_void;
                table[13] = zero_i as *const c_void;
                table[14] = observer_mock as *const c_void;
                table[15] = mutate as *const c_void;
                table[44] = display as *const c_void;
            }
            "object" => {
                table[4] = no_property as *const c_void;
            }
            "string" => {
                table[4] = zero_i as *const c_void;
                table[8] = copy_mock as *const c_void;
                table[21] = private_mock as *const c_void;
            }
            _ => {}
        }
        Self {
            value: Box::new(Fake {
                table: table.as_ptr(),
                refs: Cell::new(1),
                canonical: Cell::new(ptr::null_mut()),
                control: Cell::new(ptr::null_mut()),
                object: Cell::new(ptr::null_mut()),
                parameter: Cell::new(ptr::null_mut()),
                observer: Cell::new(ptr::null_mut()),
                children: RefCell::new(Vec::new()),
                cursor: Cell::new(0),
                name: PlainCString {
                    text: c"chorder.autofill".as_ptr(),
                    owner: ptr::null_mut(),
                },
                qi_failure: Cell::new(false),
                qi_status: Cell::new(FAILED),
                detach: Cell::new(false),
                stop_on_release: Cell::new(ptr::null()),
                mutations: Cell::new(0),
            }),
            _table: table,
        }
    }
    fn raw(&self) -> *mut c_void {
        ptr::from_ref(&*self.value).cast_mut().cast()
    }
    fn parameter(&self) -> RawParameter {
        retained_parameter(self.raw(), &api()).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        assert_eq!(self.value.refs.get(), 1, "balanced references");
        assert_eq!(self.value.mutations.get(), 0, "no setter/update dispatch");
    }
}

#[test]
fn declared_observer_and_parameter_canonical_identity_are_pinned_across_interface_views() {
    let a = Fixture::new("parameter");
    let b = Fixture::new("parameter");
    let canonical = Fixture::new("identity");
    let observer_a = Fixture::new("identity");
    let observer_b = Fixture::new("identity");
    let observer_identity = Fixture::new("identity");
    a.value.canonical.set(canonical.raw());
    b.value.canonical.set(canonical.raw());
    a.value.observer.set(observer_a.raw());
    b.value.observer.set(observer_b.raw());
    observer_a.value.canonical.set(observer_identity.raw());
    observer_b.value.canonical.set(observer_identity.raw());
    let a_pin = ParameterPin::capture(a.parameter(), &mut || Ok(())).unwrap();
    let b_pin = ParameterPin::capture(b.parameter(), &mut || Ok(())).unwrap();
    assert_ne!(a.raw(), b.raw());
    assert_eq!(a_pin.canonical.0, b_pin.canonical.0);
    assert_eq!(
        a_pin.observer_canonical.as_ref().unwrap().0,
        b_pin.observer_canonical.as_ref().unwrap().0
    );
    let mut witnesses = Vec::new();
    a_pin.check(&mut || Ok(()), &mut witnesses).unwrap();
    b_pin.check(&mut || Ok(()), &mut witnesses).unwrap();
    drop(witnesses);
    drop((a_pin, b_pin));
}
#[test]
fn absent_observer_appearance_and_present_replacement_are_refused() {
    let parameter = Fixture::new("parameter");
    let owner = Fixture::new("identity");
    let other = Fixture::new("identity");
    let pin = ParameterPin::capture(parameter.parameter(), &mut || Ok(())).unwrap();
    assert!(pin.observer.is_none());
    parameter.value.observer.set(owner.raw());
    assert!(pin.check(&mut || Ok(()), &mut Vec::new()).is_err());
    drop(pin);
    let pin = ParameterPin::capture(parameter.parameter(), &mut || Ok(())).unwrap();
    parameter.value.observer.set(other.raw());
    assert!(pin.check(&mut || Ok(()), &mut Vec::new()).is_err());
    assert!(pin.check_borrowed().is_err());
    drop(pin);
}
#[test]
fn canonical_change_and_failed_query_release_are_not_positive_identity_evidence() {
    let parameter = Fixture::new("parameter");
    let replacement = Fixture::new("identity");
    let pin = ParameterPin::capture(parameter.parameter(), &mut || Ok(())).unwrap();
    parameter.value.canonical.set(replacement.raw());
    assert!(pin.check(&mut || Ok(()), &mut Vec::new()).is_err());
    drop(pin);
    let view = Fixture::new("view");
    view.value.qi_failure.set(true);
    assert!(optional(view.raw(), &CONTROL, &mut || Ok(())).is_err());
}

fn run_scan(root: &Fixture) -> Result<Scan, WalkFailure> {
    let mut coverage = Coverage::default();
    scan(root.raw(), &api(), &mut coverage, &mut || Ok(()))
}
#[test]
fn walk_distinguishes_unique_missing_and_duplicate_controls_even_with_shared_parameter() {
    let empty_string = Fixture::new("string");
    EMPTY.with(|x| x.set(empty_string.raw()));
    let root = Fixture::new("view");
    let a = Fixture::new("view");
    let b = Fixture::new("view");
    let control = Fixture::new("control");
    let parameter = Fixture::new("parameter");
    let object = Fixture::new("object");
    parameter.value.object.set(object.raw());
    control.value.parameter.set(parameter.raw());
    a.value.control.set(control.raw());
    root.value.children.borrow_mut().push(a.raw());
    let scan = run_scan(&root).unwrap();
    assert_eq!(scan.coverage.candidates, 1);
    assert_eq!(scan.coverage.views, 2);
    assert!(scan.coverage.complete);
    drop(scan);
    b.value.control.set(control.raw());
    root.value.children.borrow_mut().push(b.raw());
    let scan = run_scan(&root).unwrap();
    assert_eq!(scan.coverage.candidates, 2);
    drop(scan);
    control.value.parameter.set(ptr::null_mut());
    let scan = run_scan(&root).unwrap();
    assert_eq!(scan.coverage.candidates, 0);
    assert!(scan.coverage.complete);
    drop(scan);
    EMPTY.with(|x| x.set(ptr::null_mut()));
}
#[test]
fn walk_refuses_cycles_detached_edges_qi_failures_and_bounds() {
    let root = Fixture::new("view");
    let child = Fixture::new("view");
    root.value.children.borrow_mut().push(root.raw());
    assert!(run_scan(&root).is_err());
    root.value.children.replace(vec![child.raw()]);
    root.value.detach.set(true);
    assert!(run_scan(&root).is_err());
    root.value.detach.set(false);
    child.value.qi_failure.set(true);
    assert!(run_scan(&root).is_err());
    child.value.qi_failure.set(false);
    let children: Vec<_> = (0..128).map(|_| Fixture::new("view")).collect();
    root.value
        .children
        .replace(children.iter().map(Fixture::raw).collect());
    assert!(run_scan(&root).is_err());
    let chain: Vec<_> = (0..18).map(|_| Fixture::new("view")).collect();
    for i in 0..17 {
        chain[i]
            .value
            .children
            .borrow_mut()
            .push(chain[i + 1].raw());
    }
    assert!(run_scan(&chain[0]).is_err());
}
#[test]
fn incomplete_walk_distinguishes_topology_bounds_and_native_query_status() {
    fn failure(root: &Fixture) -> Value {
        json!(run_scan(root).err().expect("incomplete walk"))
    }
    let root = Fixture::new("view");
    let child = Fixture::new("view");
    root.value.children.replace(vec![root.raw()]);
    assert_eq!(
        failure(&root),
        json!({"kind":"repeated_view","stage":"view_identity","depth":1,"query_status":null})
    );
    root.value.children.replace(vec![child.raw()]);
    root.value.detach.set(true);
    assert_eq!(
        failure(&root),
        json!({"kind":"detached_view","stage":"edge","depth":1,"query_status":null})
    );
    root.value.detach.set(false);
    child.value.qi_failure.set(true);
    assert_eq!(
        failure(&root),
        json!({"kind":"interface_query_failed","stage":"control_query","depth":1,"query_status":FAILED})
    );
    child.value.qi_failure.set(false);
    let children: Vec<_> = (0..128).map(|_| Fixture::new("view")).collect();
    root.value
        .children
        .replace(children.iter().map(Fixture::raw).collect());
    assert_eq!(
        failure(&root),
        json!({"kind":"view_limit","stage":"children","depth":0,"query_status":null})
    );
    let chain: Vec<_> = (0..18).map(|_| Fixture::new("view")).collect();
    for i in 0..17 {
        chain[i]
            .value
            .children
            .borrow_mut()
            .push(chain[i + 1].raw());
    }
    assert_eq!(
        failure(&chain[0]),
        json!({"kind":"depth_limit","stage":"children","depth":16,"query_status":null})
    );
}
#[test]
fn walk_query_status_and_guard_loss_remain_distinct_without_native_messages() {
    let root = Fixture::new("view");
    root.value.qi_failure.set(true);
    root.value.qi_status.set(17);
    let failure = run_scan(&root).err().expect("query failed");
    assert_eq!(failure.query_status, Some(17));
    assert_eq!(failure.kind, WalkKind::InterfaceQueryFailed);
    root.value.qi_status.set(NO_INTERFACE);
    let observed = run_scan(&root).expect("exact no-interface is a noncontrol");
    assert!(observed.coverage.complete);
    drop(observed);
    root.value.qi_failure.set(false);
    let mut coverage = Coverage::default();
    let failure = scan(root.raw(), &api(), &mut coverage, &mut || {
        Err("private native/context detail".into())
    })
    .err()
    .expect("guard failed");
    assert_eq!(
        json!(failure),
        json!({"kind":"context_guard_failed","stage":"root","depth":0,"query_status":null})
    );
    assert!(!coverage.complete);
}
#[test]
fn walk_checks_release_reentry_and_balances_every_retained_view() {
    let root = Fixture::new("view");
    let child = Fixture::new("view");
    let stop = Cell::new(false);
    child.value.stop_on_release.set(&stop);
    root.value.children.borrow_mut().push(child.raw());
    let mut coverage = Coverage::default();
    assert!(
        scan(root.raw(), &api(), &mut coverage, &mut || if stop.get() {
            Err("stale".into())
        } else {
            Ok(())
        })
        .is_err()
    );
    assert!(stop.get());
}

#[test]
fn exact_name_matching_has_no_local_name_or_caption_fallback() {
    let empty_string = Fixture::new("string");
    EMPTY.with(|x| x.set(empty_string.raw()));
    let root = Fixture::new("view");
    let control = Fixture::new("control");
    let mut parameter = Fixture::new("parameter");
    let object = Fixture::new("object");
    parameter.value.object.set(object.raw());
    parameter.value.name.text = c"autofill".as_ptr();
    control.value.parameter.set(parameter.raw());
    root.value.control.set(control.raw());
    let scan = run_scan(&root).unwrap();
    assert_eq!(scan.coverage.candidates, 0);
    assert!(scan.coverage.complete);
    drop(scan);
    EMPTY.with(|x| x.set(ptr::null_mut()));
}
#[test]
fn exact_view_and_depth_limits_accept_complete_walks_at_the_boundary() {
    let root = Fixture::new("view");
    let children: Vec<_> = (0..127).map(|_| Fixture::new("view")).collect();
    root.value
        .children
        .replace(children.iter().map(Fixture::raw).collect());
    let scan = run_scan(&root).unwrap();
    assert_eq!(scan.coverage.views, 128);
    assert!(scan.coverage.complete);
    drop(scan);
    let chain: Vec<_> = (0..17).map(|_| Fixture::new("view")).collect();
    for i in 0..16 {
        chain[i]
            .value
            .children
            .borrow_mut()
            .push(chain[i + 1].raw());
    }
    let scan = run_scan(&chain[0]).unwrap();
    assert!(scan.coverage.complete);
    assert_eq!(scan.coverage.views, 17);
    drop(scan);
}
#[test]
fn read_receipt_transfer_preserves_diagnostics_and_stale_publication_never_claims_mutation() {
    let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut host = guard_tests::Fixture::new();
    let ctx = host.context();
    let service = ctx.service;
    let controller = Fixture::new("identity");
    let gui = ComparisonRead {
        value: unresolved("missing_editor", Coverage::default()),
        pins: None,
    };
    let mut reply = Reply::unbound(
        json!({"ok":true,"outcome":"observed","source":"dsp","gui_comparison":gui.value()}),
    );
    reply.gui = Some(gui);
    reply.provider = Some(Provider::DspValue(Controller {
        handle: hold(controller.raw()).unwrap(),
        lease: None,
    }));
    reply.context = Some(ctx);
    assert!(reply.final_check(service));
    let receipt = reply.take_receipt().unwrap();
    assert!(receipt._gui.is_some());
    assert!(reply.gui.is_none());
    drop(receipt);
    drop(reply);
    let ctx = host.context();
    let gui = ComparisonRead {
        value: unresolved("missing_editor", Coverage::default()),
        pins: None,
    };
    let mut reply = Reply::unbound(
        json!({"ok":true,"outcome":"observed","source":"dsp","gui_comparison":gui.value()}),
    );
    reply.gui = Some(gui);
    reply.provider = Some(Provider::DspValue(Controller {
        handle: hold(controller.raw()).unwrap(),
        lease: None,
    }));
    reply.context = Some(ctx);
    host.switch_document();
    assert!(!reply.final_check(service));
    assert_eq!(reply.value["outcome"], "not_applied");
    assert_eq!(reply.value["gui_comparison"]["reason"], "stale_association");
    assert!(
        reply.value["gui_comparison"]
            .get("topParameterSame")
            .is_none()
    );
    reply.release_context();
    drop(reply);
}

#[test]
fn different_native_class_is_refused_before_editor_discovery() {
    let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut host = guard_tests::Fixture::new();
    let ctx = host.context();
    let controller = Fixture::new("identity");
    let parameter = Fixture::new("parameter");
    let direct = Controller {
        handle: hold(controller.raw()).unwrap(),
        lease: None,
    };
    assert!(ComparisonRead::capture(&ctx, &direct, parameter.parameter()).is_err());
    assert_eq!(parameter.value.refs.get(), 1);
    drop(direct);
    drop(ctx);
}

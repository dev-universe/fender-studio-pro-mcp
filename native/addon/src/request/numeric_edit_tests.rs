use super::*;
use std::cell::Cell;

#[repr(C)]
struct Mock {
    table: *const *const c_void,
    refs: AtomicU32,
    controller: *mut c_void,
    alias: *mut c_void,
    parent: *mut c_void,
    parameter: *mut c_void,
    canonical: *mut c_void,
    has_alias: bool,
    release_target: *mut Mock,
    release_replacement: *mut c_void,
    redirect_on_release: Cell<bool>,
    change_controller: bool,
    change_alias_presence: bool,
    query_count: Cell<usize>,
    query_invalidate_at: Cell<Option<usize>>,
    query_context: *const Cell<bool>,
}
unsafe extern "C" fn retain(p: *mut c_void) -> u32 {
    unsafe { &*p.cast::<Mock>() }
        .refs
        .fetch_add(1, Ordering::Relaxed)
        + 1
}
unsafe extern "C" fn release_mock(p: *mut c_void) -> u32 {
    let mock = unsafe { &*p.cast::<Mock>() };
    if mock.redirect_on_release.replace(false) {
        unsafe {
            if mock.change_controller {
                (*mock.release_target).controller = mock.release_replacement;
            } else if mock.change_alias_presence {
                (*mock.release_target).has_alias = true;
            } else {
                (*mock.release_target).alias = mock.release_replacement;
            }
        };
    }
    mock.refs.fetch_sub(1, Ordering::Relaxed) - 1
}
unsafe extern "C" fn query_mock(p: *mut c_void, iid: *const Uid, output: *mut *mut c_void) -> i32 {
    let mock = unsafe { &*p.cast::<Mock>() };
    mock.query_count.set(mock.query_count.get() + 1);
    if mock.query_invalidate_at.get() == Some(mock.query_count.get())
        && !mock.query_context.is_null()
    {
        unsafe {
            (*mock.query_context).set(false);
        }
    }
    let pointer = if unsafe { *iid } == UNKNOWN {
        if mock.canonical.is_null() {
            p
        } else {
            mock.canonical
        }
    } else if unsafe { *iid } == CONTROLLER {
        mock.controller
    } else if unsafe { *iid } == ALIAS && mock.has_alias {
        p
    } else {
        ptr::null_mut()
    };
    unsafe { *output = pointer };
    if pointer.is_null() {
        NO_INTERFACE
    } else {
        unsafe { retain(pointer) };
        OK
    }
}
unsafe extern "C" fn alias(p: *mut c_void) -> *mut c_void {
    unsafe { (*p.cast::<Mock>()).alias }
}
unsafe extern "C" fn parent(p: *mut c_void) -> *mut c_void {
    unsafe { (*p.cast::<Mock>()).parent }
}
unsafe extern "C" fn find(p: *mut c_void, name: *const PlainCString) -> *mut c_void {
    assert_eq!(unsafe { std::ffi::CStr::from_ptr((*name).text) }, PARAMETER);
    unsafe { (*p.cast::<Mock>()).parameter }
}
struct Fixture {
    _table: Box<[*const c_void; 7]>,
    target: Box<Mock>,
    controller: Box<Mock>,
    plugin: Box<Mock>,
    parent: Box<Mock>,
    parameter: Box<Mock>,
    replacement: Box<Mock>,
}
impl Fixture {
    fn new(with_alias: bool) -> Self {
        let mut table = Box::new([ptr::null::<c_void>(); 7]);
        table[0] = query_mock as *const c_void;
        table[1] = retain as *const c_void;
        table[2] = release_mock as *const c_void;
        table[3] = alias as *const c_void;
        table[5] = find as *const c_void;
        table[6] = parent as *const c_void;
        let make = || {
            Box::new(Mock {
                table: table.as_ptr(),
                refs: AtomicU32::new(1),
                controller: ptr::null_mut(),
                alias: ptr::null_mut(),
                parent: ptr::null_mut(),
                parameter: ptr::null_mut(),
                canonical: ptr::null_mut(),
                has_alias: false,
                release_target: ptr::null_mut(),
                release_replacement: ptr::null_mut(),
                redirect_on_release: Cell::new(false),
                change_controller: false,
                change_alias_presence: false,
                query_count: Cell::new(0),
                query_invalidate_at: Cell::new(None),
                query_context: ptr::null(),
            })
        };
        let (mut target, mut controller, mut plugin, mut parent, mut parameter, replacement) =
            (make(), make(), make(), make(), make(), make());
        target.controller = ptr::addr_of_mut!(*controller).cast();
        target.has_alias = with_alias;
        target.alias = ptr::addr_of_mut!(*plugin).cast();
        target.parent = ptr::addr_of_mut!(*parent).cast();
        controller.parameter = ptr::addr_of_mut!(*parameter).cast();
        plugin.release_target = ptr::addr_of_mut!(*target);
        plugin.release_replacement = ptr::addr_of!(*replacement).cast_mut().cast();
        Self {
            _table: table,
            target,
            controller,
            plugin,
            parent,
            parameter,
            replacement,
        }
    }
    fn capture(&mut self) -> EditRoute {
        let target = ptr::addr_of_mut!(*self.target).cast();
        EditRoute::capture(target, target, &mut || Ok(())).unwrap()
    }
    fn pointer(&mut self) -> *mut c_void {
        ptr::addr_of_mut!(*self.parameter).cast()
    }
    fn assert_released(&self) {
        for mock in [
            &self.target,
            &self.controller,
            &self.plugin,
            &self.parent,
            &self.parameter,
            &self.replacement,
        ] {
            assert_eq!(mock.refs.load(Ordering::Relaxed), 1);
        }
    }
}
#[test]
fn pilot_wire_grammar_is_closed_and_requires_fresh_identity() {
    let base = json!({"operation":"edit_numeric_parameter","param":"chorder.autofill","value":0,"expected_instance":uid(&SERVICE_CID)});
    for value in [0, 1] {
        let mut request = base.clone();
        request["value"] = json!(value);
        let request: Request = serde_json::from_value(request).unwrap();
        assert!(request.validate().is_ok());
        assert_eq!(request.source(), Source::Dsp);
        assert!(!request.needs_auxiliary());
    }
    for (field, value) in [
        ("source", json!("dsp")),
        ("mode", json!("raw")),
        ("text", json!("Off")),
        ("normalized", json!(0)),
        ("begin", json!(true)),
    ] {
        let mut request = base.clone();
        request[field] = value;
        assert!(serde_json::from_value::<Request>(request).is_err());
    }
    for value in [-1.0, 0.5, 2.0] {
        let mut request = base.clone();
        request["value"] = json!(value);
        assert!(
            serde_json::from_value::<Request>(request)
                .unwrap()
                .validate()
                .is_err()
        );
    }
    for value in ["other", "", "chorder.learnMode"] {
        let mut request = base.clone();
        request["param"] = json!(value);
        assert!(
            serde_json::from_value::<Request>(request)
                .unwrap()
                .validate()
                .is_err()
        );
    }
    let mut missing = base.clone();
    missing.as_object_mut().unwrap().remove("expected_instance");
    assert!(serde_json::from_value::<Request>(missing).is_err());
    let mut wrong = base;
    wrong["expected_instance"] = json!(ZERO_UID);
    assert!(
        serde_json::from_value::<Request>(wrong)
            .unwrap()
            .validate()
            .is_err()
    );
}
#[test]
fn route_retains_stable_present_and_absent_aliases_without_mutation() {
    for with_alias in [false, true] {
        let mut fixture = Fixture::new(with_alias);
        let route = fixture.capture();
        assert!(
            route
                .check(fixture.pointer(), &mut || Ok(()), &mut || Ok(()))
                .is_ok()
        );
        drop(route);
        fixture.assert_released();
    }
}
#[test]
fn route_refuses_each_rebound_attachment_and_canonical_identity() {
    for change in 0..7 {
        let mut fixture = Fixture::new(true);
        let route = fixture.capture();
        let other = ptr::addr_of_mut!(*fixture.replacement).cast();
        match change {
            0 => fixture.target.parent = other,
            1 => fixture.target.alias = other,
            2 => fixture.target.controller = other,
            3 => fixture.controller.parameter = other,
            4 => fixture.plugin.canonical = other,
            5 => fixture.parent.canonical = other,
            6 => fixture.controller.canonical = other,
            _ => unreachable!(),
        }
        assert!(
            route
                .check(fixture.pointer(), &mut || Ok(()), &mut || Ok(()))
                .is_err(),
            "change {change}"
        );
        drop(route);
        fixture.assert_released();
    }
}
#[test]
fn route_refuses_new_alias_and_release_reentry_before_publication() {
    let mut absent = Fixture::new(false);
    let route = absent.capture();
    absent.target.has_alias = true;
    assert!(
        route
            .check(absent.pointer(), &mut || Ok(()), &mut || Ok(()))
            .is_err()
    );
    drop(route);
    absent.assert_released();
    let mut fixture = Fixture::new(true);
    let route = fixture.capture();
    fixture.plugin.redirect_on_release.set(true);
    assert!(
        route
            .check(fixture.pointer(), &mut || Ok(()), &mut || Ok(()))
            .is_err()
    );
    drop(route);
    fixture.assert_released();
}
#[test]
fn route_capture_and_check_stop_on_context_loss() {
    let mut fixture = Fixture::new(true);
    let target = ptr::addr_of_mut!(*fixture.target).cast();
    let calls = Cell::new(0);
    assert!(
        EditRoute::capture(target, target, &mut || {
            calls.set(calls.get() + 1);
            if calls.get() >= 4 {
                Err("context lost".into())
            } else {
                Ok(())
            }
        })
        .is_err()
    );
    fixture.assert_released();
    let route = fixture.capture();
    assert!(
        route
            .check(
                fixture.pointer(),
                &mut || Err("context lost".into()),
                &mut || Ok(())
            )
            .is_err()
    );
    drop(route);
    fixture.assert_released();
}
#[test]
fn observed_edit_phase_survives_invalidated_and_oversize_responses() {
    for phase in [
        "begin_dispatched",
        "value_dispatched",
        "end_dispatched",
        "closed_checked",
    ] {
        let mut reply = Reply::unbound(
            json!({"ok":true,"outcome":"observed","edit_phase":phase,"verified":true}),
        );
        reply.mutation = true;
        reply.invalidate("context lost".into());
        assert_eq!(reply.value["outcome"], "unknown");
        assert_eq!(reply.value["edit_phase"], phase);
        assert!(reply.value.get("verified").is_none());
        reply.value["padding"] = json!("x".repeat(MAX_RESPONSE_BYTES));
        let encoded: Value = serde_json::from_str(&reply.encode()).unwrap();
        assert_eq!(encoded["outcome"], "unknown");
        assert_eq!(encoded["edit_phase"], phase);
    }
}

#[test]
fn last_context_guard_precedes_final_borrowed_attachment_reads() {
    for change in 0..3 {
        let mut fixture = Fixture::new(true);
        let route = fixture.capture();
        let count = Cell::new(0);
        assert!(
            route
                .check(
                    fixture.pointer(),
                    &mut || {
                        count.set(count.get() + 1);
                        Ok(())
                    },
                    &mut || Ok(())
                )
                .is_ok()
        );
        let last = count.get();
        count.set(0);
        let target = ptr::addr_of_mut!(*fixture.target);
        let controller = ptr::addr_of_mut!(*fixture.controller);
        let other = ptr::addr_of_mut!(*fixture.replacement).cast();
        let result = route.check(
            fixture.pointer(),
            &mut || {
                count.set(count.get() + 1);
                if count.get() == last {
                    unsafe {
                        match change {
                            0 => (*target).alias = other,
                            1 => (*target).parent = other,
                            2 => (*controller).parameter = other,
                            _ => unreachable!(),
                        }
                    }
                }
                Ok(())
            },
            &mut || Ok(()),
        );
        assert!(result.is_err(), "last guard redirect {change}");
        drop(route);
        fixture.assert_released();
    }
}

#[test]
fn reviewer_route_rejects_controller_replacement_during_observation_release() {
    let mut fixture = Fixture::new(true);
    let route = fixture.capture();
    fixture.controller.release_target = ptr::addr_of_mut!(*fixture.target);
    fixture.controller.release_replacement = ptr::addr_of_mut!(*fixture.replacement).cast();
    fixture.controller.change_controller = true;
    fixture.controller.redirect_on_release.set(true);
    let result = route.check(fixture.pointer(), &mut || Ok(()), &mut || Ok(()));
    assert!(
        result.is_err(),
        "controller replacement was accepted: {result:?}"
    );
}
#[test]
fn reviewer_route_rejects_alias_appearance_during_observation_release() {
    let mut fixture = Fixture::new(false);
    let route = fixture.capture();
    fixture.controller.release_target = ptr::addr_of_mut!(*fixture.target);
    fixture.controller.change_alias_presence = true;
    fixture.controller.redirect_on_release.set(true);
    let result = route.check(fixture.pointer(), &mut || Ok(()), &mut || Ok(()));
    assert!(result.is_err(), "alias appearance was accepted: {result:?}");
}
#[test]
fn reviewer_route_rejects_alias_disappearance_at_last_context_guard() {
    let mut fixture = Fixture::new(true);
    let route = fixture.capture();
    let count = Cell::new(0);
    route
        .check(
            fixture.pointer(),
            &mut || {
                count.set(count.get() + 1);
                Ok(())
            },
            &mut || Ok(()),
        )
        .unwrap();
    let last = count.get();
    count.set(0);
    let target = ptr::addr_of_mut!(*fixture.target);
    let result = route.check(
        fixture.pointer(),
        &mut || {
            count.set(count.get() + 1);
            if count.get() == last {
                unsafe {
                    (*target).has_alias = false;
                }
            }
            Ok(())
        },
        &mut || Ok(()),
    );
    assert!(
        result.is_err(),
        "alias disappearance was accepted: {result:?}"
    );
}

#[test]
fn reviewer_route_final_query_context_loss_is_rejected() {
    let mut fixture = Fixture::new(true);
    let route = fixture.capture();
    fixture.target.query_count.set(0);
    route
        .check(fixture.pointer(), &mut || Ok(()), &mut || Ok(()))
        .unwrap();
    let final_query = fixture.target.query_count.get();
    let context = Cell::new(true);
    fixture.target.query_context = &context;
    fixture.target.query_count.set(0);
    fixture.target.query_invalidate_at.set(Some(final_query));
    let result = route.check(
        fixture.pointer(),
        &mut || {
            if context.get() {
                Ok(())
            } else {
                Err("context lost".into())
            }
        },
        &mut || {
            if context.get() {
                Ok(())
            } else {
                Err("document lost".into())
            }
        },
    );
    assert!(!context.get(), "fault was not injected");
    assert!(
        result.is_err(),
        "final query context loss was accepted: {result:?}"
    );
}

use super::*;
use std::{
    cell::{Cell, RefCell},
    ffi::CStr,
};

const ZERO: Uid = Uid {
    a: 0,
    b: 0,
    c: 0,
    d: [0; 8],
};
const CV: Uid = Uid {
    a: 0xedcd22a5,
    b: 0xa01e,
    c: 0x4ea6,
    d: [0x9f, 0xe4, 0x2b, 0xb1, 0xab, 0x24, 0x3e, 0x85],
};
#[repr(C)]
struct View {
    table: *const *const c_void,
    owner: *mut Fake,
    kind: usize,
}
struct Fake {
    views: [View; 5],
    refs: Cell<u32>,
    canonical: Cell<*mut c_void>,
    node_interface: Cell<*mut c_void>,
    reflection_identity: Cell<*mut c_void>,
    parent: Cell<*mut c_void>,
    fixed: Cell<*mut c_void>,
    children: RefCell<Vec<*mut c_void>>,
    count_override: Cell<Option<i32>>,
    object_uid: Cell<Uid>,
    class_uid: Cell<Uid>,
    family: Cell<&'static CStr>,
    type_parent: Cell<*mut c_void>,
    text: RefCell<Vec<u16>>,
    plain: PlainString,
    release_detach: Cell<*mut Fake>,
    release_uid: Cell<*mut Fake>,
    release_family: Cell<*mut Fake>,
    release_id: Cell<*mut Fake>,
    queries: RefCell<Vec<Uid>>,
    available: Cell<bool>,
    malformed: Cell<Option<Uid>>,
    null_class: Cell<bool>,
    null_type: Cell<bool>,
    null_name: Cell<bool>,
    copy_family: Cell<*mut Fake>,
    controller: Cell<*mut c_void>,
    controller_status: Cell<Option<i32>>,
    copy_action: RefCell<Option<Box<dyn FnOnce()>>>,
    copy_calls: Cell<u32>,
    copy_action_at: Cell<Option<u32>>,
}
unsafe fn fake<'a>(p: *mut c_void) -> &'a Fake {
    unsafe { &*(*p.cast::<View>()).owner }
}
unsafe extern "C" fn retain(p: *mut c_void) -> u32 {
    let f = unsafe { fake(p) };
    f.refs.set(f.refs.get() + 1);
    f.refs.get()
}
unsafe extern "C" fn release(p: *mut c_void) -> u32 {
    let f = unsafe { fake(p) };
    f.refs
        .set(f.refs.get().checked_sub(1).expect("double release"));
    if unsafe { (*p.cast::<View>()).kind } >= 2 {
        let target = f.release_detach.replace(ptr::null_mut());
        if !target.is_null() {
            unsafe { &*target }.fixed.set(ptr::null_mut());
        }
        let target = f.release_uid.replace(ptr::null_mut());
        if !target.is_null() {
            unsafe { &*target }.object_uid.set(API_CLASS);
        }
    }
    if unsafe { (*p.cast::<View>()).kind } >= 2 {
        let target = f.release_family.replace(ptr::null_mut());
        if !target.is_null() {
            unsafe { &*target }.family.set(c"ChangedAfterCleanup");
        }
    }
    if unsafe { (*p.cast::<View>()).kind } == 4 {
        let target = f.release_id.replace(ptr::null_mut());
        if !target.is_null() {
            *unsafe { &*target }.text.borrow_mut() = "Changed ID".encode_utf16().collect();
        }
    }
    f.refs.get()
}
unsafe extern "C" fn query_fake(p: *mut c_void, iid: *const Uid, out: *mut *mut c_void) -> i32 {
    let f = unsafe { fake(p) };
    let iid = unsafe { *iid };
    f.queries.borrow_mut().push(iid);
    if iid == CONTROLLER {
        if let Some(status) = f.controller_status.get() {
            let pointer = f.controller.get();
            unsafe { *out = pointer };
            if status == OK && !pointer.is_null() {
                unsafe {
                    let retain: unsafe extern "C" fn(*mut c_void) -> u32 = method(pointer, 1);
                    retain(pointer);
                };
            }
            return status;
        }
        let pointer = f.controller.get();
        unsafe { *out = pointer };
        if pointer.is_null() {
            return NO_INTERFACE;
        }
        unsafe {
            let retain: unsafe extern "C" fn(*mut c_void) -> u32 = method(pointer, 1);
            retain(pointer);
        };
        return OK;
    }
    if f.malformed.get() == Some(iid) {
        unsafe { *out = ptr::dangling_mut::<c_void>() };
        return NO_INTERFACE;
    }

    unsafe { *out = ptr::null_mut() };
    let found = if iid == UNKNOWN {
        if unsafe { (*p.cast::<View>()).kind } == 2 && !f.reflection_identity.get().is_null() {
            f.reflection_identity.get()
        } else {
            f.canonical.get()
        }
    } else if iid == NODE {
        f.node_interface.get()
    } else if iid == OBJECT && f.available.get() {
        ptr::from_ref(&f.views[2]).cast_mut().cast()
    } else {
        ptr::null_mut()
    };
    if found.is_null() {
        NO_INTERFACE
    } else {
        unsafe {
            retain(found);
            *out = found
        };
        OK
    }
}
unsafe extern "C" fn id(p: *mut c_void) -> *const PlainString {
    &unsafe { fake(p) }.plain
}
unsafe extern "C" fn object_uid(p: *mut c_void) -> *const Uid {
    unsafe { fake(p) }.object_uid.as_ptr()
}
unsafe extern "C" fn class_uid(p: *mut c_void) -> *const Uid {
    let f = unsafe { fake(p) };
    if f.null_class.get() {
        ptr::null()
    } else {
        f.class_uid.as_ptr()
    }
}
unsafe extern "C" fn parent_of(p: *mut c_void) -> *mut c_void {
    unsafe { fake(p) }.parent.get()
}
unsafe extern "C" fn count_of(p: *mut c_void) -> i32 {
    let f = unsafe { fake(p) };
    f.count_override
        .get()
        .unwrap_or(f.children.borrow().len() as i32)
}
unsafe extern "C" fn child_of(p: *mut c_void, i: i32) -> *mut c_void {
    unsafe { fake(p) }
        .children
        .borrow()
        .get(i as usize)
        .copied()
        .unwrap_or(ptr::null_mut())
}
unsafe extern "C" fn find_of(p: *mut c_void, name: *const PlainString) -> *mut c_void {
    if name.cast::<c_char>() == c"PortSetup".as_ptr() {
        unsafe { fake(p) }.fixed.get()
    } else {
        ptr::null_mut()
    }
}
unsafe extern "C" fn type_info(p: *mut c_void) -> *mut c_void {
    let f = unsafe { fake(p) };
    if f.null_type.get() {
        ptr::null_mut()
    } else {
        ptr::from_ref(&f.views[3]).cast_mut().cast()
    }
}
unsafe extern "C" fn type_parent(p: *mut c_void) -> *mut c_void {
    unsafe { fake(p) }.type_parent.get()
}
unsafe extern "C" fn type_name(p: *mut c_void) -> *const c_char {
    let f = unsafe { fake(p) };
    if f.null_name.get() {
        ptr::null()
    } else {
        f.family.get().as_ptr()
    }
}
unsafe extern "C" fn zero(_: *mut c_void) -> i32 {
    0
}
unsafe extern "C" fn null(_: *mut c_void) -> *mut c_void {
    ptr::null_mut()
}
unsafe extern "C" fn length(p: *mut c_void) -> i32 {
    unsafe { fake(p) }.text.borrow().len() as i32
}
unsafe extern "C" fn copy(p: *mut c_void, buffer: *mut u16, cap: i32) -> i32 {
    let f = unsafe { fake(p) };
    let target = f.copy_family.replace(ptr::null_mut());
    if !target.is_null() {
        unsafe { &*target }
            .family
            .set(c"ChangedDuringFinalStringCopy");
    }
    f.copy_calls.set(f.copy_calls.get() + 1);
    if f.copy_action_at
        .get()
        .is_none_or(|at| at == f.copy_calls.get())
    {
        let action = f.copy_action.borrow_mut().take();
        if let Some(action) = action {
            action();
        }
    }
    let text = f.text.borrow();
    assert!(cap as usize > text.len());
    unsafe {
        ptr::copy_nonoverlapping(text.as_ptr(), buffer, text.len());
        *buffer.add(text.len()) = 0
    };
    OK
}
unsafe extern "C" fn text(name: *const c_char) -> *const c_void {
    name.cast()
}
struct Fixture {
    nodes: Vec<Box<Fake>>,
    _node: Box<[*const c_void; 11]>,
    _object: Box<[*const c_void; 4]>,
    _info: Box<[*const c_void; 10]>,
    _text: Box<[*const c_void; 9]>,
}
impl Fixture {
    fn new() -> Self {
        let mut n = Box::new([ptr::null(); 11]);
        let mut o = Box::new([ptr::null(); 4]);
        let mut i = Box::new([ptr::null(); 10]);
        let mut t = Box::new([ptr::null(); 9]);
        for table in [&mut n[..], &mut o[..], &mut i[..], &mut t[..]] {
            table[0] = query_fake as *const c_void;
            table[1] = retain as *const c_void;
            table[2] = release as *const c_void;
        }
        n[3] = id as *const c_void;
        n[4] = object_uid as *const c_void;
        n[5] = class_uid as *const c_void;
        n[6] = parent_of as *const c_void;
        n[8] = count_of as *const c_void;
        n[9] = child_of as *const c_void;
        n[10] = find_of as *const c_void;
        o[3] = type_info as *const c_void;
        i[3] = type_parent as *const c_void;
        i[4] = zero as *const c_void;
        i[5] = type_name as *const c_void;
        i[6] = null as *const c_void;
        i[7] = class_uid as *const c_void;
        i[8] = null as *const c_void;
        i[9] = null as *const c_void;
        t[4] = length as *const c_void;
        t[8] = copy as *const c_void;
        let mut nodes = Vec::new();
        for index in 0..40 {
            let (id, family) = match index {
                0 => ("CV Instrument", c"AudioSynth"),
                1 => ("CV Slot", c"AudioSynthSlot"),
                2 => ("PortSetup", c"AudioFXPortSetup"),
                _ => ("AudioIn", c"AudioFXPort"),
            };
            let mut f = Box::new(Fake {
                views: std::array::from_fn(|kind| View {
                    table: match kind {
                        0 | 1 => n.as_ptr(),
                        2 => o.as_ptr(),
                        3 => i.as_ptr(),
                        _ => t.as_ptr(),
                    },
                    owner: ptr::null_mut(),
                    kind,
                }),
                refs: Cell::new(1),
                canonical: Cell::new(ptr::null_mut()),
                node_interface: Cell::new(ptr::null_mut()),
                reflection_identity: Cell::new(ptr::null_mut()),
                parent: Cell::new(ptr::null_mut()),
                fixed: Cell::new(ptr::null_mut()),
                children: RefCell::new(vec![]),
                count_override: Cell::new(None),
                object_uid: Cell::new(Uid {
                    a: 0x12340000 + index,
                    b: 0x1234,
                    c: 0x1234,
                    d: [0x12; 8],
                }),
                class_uid: Cell::new(if index == 0 { CV } else { ZERO }),
                family: Cell::new(family),
                type_parent: Cell::new(ptr::null_mut()),
                text: RefCell::new(id.encode_utf16().collect()),
                plain: PlainString {
                    object: ptr::null_mut(),
                    opaque_debug: ptr::null_mut(),
                },
                release_detach: Cell::new(ptr::null_mut()),
                release_uid: Cell::new(ptr::null_mut()),
                release_family: Cell::new(ptr::null_mut()),
                release_id: Cell::new(ptr::null_mut()),
                queries: RefCell::new(vec![]),
                available: Cell::new(true),
                malformed: Cell::new(None),
                null_class: Cell::new(false),
                null_type: Cell::new(false),
                null_name: Cell::new(false),
                copy_family: Cell::new(ptr::null_mut()),
                controller: Cell::new(ptr::null_mut()),
                controller_status: Cell::new(None),
                copy_action: RefCell::new(None),
                copy_calls: Cell::new(0),
                copy_action_at: Cell::new(None),
            });
            let owner = ptr::from_mut(&mut *f);
            for view in &mut f.views {
                view.owner = owner;
            }
            f.canonical.set(ptr::from_mut(&mut f.views[0]).cast());
            f.node_interface.set(ptr::from_mut(&mut f.views[1]).cast());
            f.plain.object = ptr::from_mut(&mut f.views[4]).cast();
            nodes.push(f);
        }
        let result = Self {
            nodes,
            _node: n,
            _object: o,
            _info: i,
            _text: t,
        };
        result.nodes[0].parent.set(result.p(1));
        result.nodes[1].fixed.set(result.p(2));
        result.nodes[1]
            .children
            .borrow_mut()
            .extend([result.p(0), result.p(2)]);
        result.nodes[2]
            .children
            .borrow_mut()
            .extend([result.p(3), result.p(4)]);
        result
    }
    fn p(&self, index: usize) -> *mut c_void {
        ptr::from_ref(&self.nodes[index].views[1]).cast_mut().cast()
    }
    fn route(&self) -> Result<Route, String> {
        Route::capture(
            self.p(0),
            self.p(0),
            c"PortSetup".as_ptr().cast(),
            &mut || Ok(()),
        )
    }
    fn inventory(&self) -> CvPortsInventory {
        let route = self.route().unwrap();
        let held = hold(self.p(2)).unwrap();
        let pointer = NonNull::new(held.0).unwrap();
        std::mem::forget(held);
        let inspector = unsafe {
            NodeInspector::from_owned(
                pointer,
                TextApi {
                    get_constant_string: text,
                },
            )
        };
        let inspection = inspector
            .inspect(limits(Scope::CvPorts), &mut || Ok(()))
            .unwrap();
        drop(inspector);
        validate_root(inspection.report(), &route.root.as_ref().unwrap().snapshot).unwrap();
        CvPortsInventory {
            inspection: Some(inspection),
            route,
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        for n in &self.nodes {
            assert_eq!(n.refs.get(), 1, "leaked fake owner");
        }
    }
}

#[test]
fn exact_cv_route_uses_current_unique_index_and_absent_class_ids() {
    let f = Fixture::new();
    f.nodes[1].children.borrow_mut().reverse();
    let route = f.route().unwrap();
    let root = route.root.as_ref().unwrap();
    assert_eq!(root.slot_index, 0);
    assert_eq!(root.slot_count, 2);
    assert_eq!(root.snapshot.first_type, "AudioFXPortSetup");
    assert_eq!(root.snapshot.class_uid, None);
    route.borrowed().unwrap();
    drop(route);
    let inventory = f.inventory();
    let value = inventory.value();
    assert_eq!(value["availability"], "observed");
    assert_eq!(
        value["nodes"][0]["declared_interfaces"],
        json!({"preset_collector":false,"preset_mediator":false,"storable":false})
    );
    assert!(!f.nodes[2].queries.borrow().contains(&CONTROLLER));
    assert_eq!(value["nodes"][0]["identity"]["class_uid"], Value::Null);
    assert_eq!(
        value["nodes"][0]["reflection"]["types"][0]["name"],
        "AudioFXPortSetup"
    );
    for field in [
        "assignment_values_observed",
        "routing_verified",
        "atomic_snapshot",
        "mutation_capability",
    ] {
        assert_eq!(value[field], false);
    }
    inventory.check(&mut || Ok(()), &mut || Ok(())).unwrap();
}
#[test]
fn finite_membership_scan_accepts_32_and_rejects_oversize_missing_duplicates_and_missing_children()
{
    let f = Fixture::new();
    *f.nodes[1].children.borrow_mut() = (2..34).map(|i| f.p(i)).collect();
    let route = f.route().unwrap();
    assert_eq!(route.root.as_ref().unwrap().slot_count, 32);
    drop(route);
    f.nodes[1].count_override.set(Some(33));
    assert!(f.route().is_err());
    f.nodes[1].count_override.set(None);
    f.nodes[1].children.borrow_mut()[0] = f.p(34);
    assert!(f.route().is_err());
    f.nodes[1].children.borrow_mut()[0] = f.p(2);
    f.nodes[1].children.borrow_mut()[1] = f.p(2);
    assert!(f.route().is_err());
    f.nodes[1].children.borrow_mut()[1] = ptr::null_mut();
    assert!(f.route().is_err());
}
#[test]
fn stable_null_lookup_is_separate_and_null_to_present_fails_without_indexed_fallback() {
    let f = Fixture::new();
    f.nodes[1].fixed.set(ptr::null_mut());
    let inventory = CvPortsInventory {
        inspection: None,
        route: f.route().unwrap(),
    };
    assert_eq!(inventory.value()["availability"], "lookup_absent");
    assert!(inventory.value().get("nodes").is_none());
    inventory.check(&mut || Ok(()), &mut || Ok(())).unwrap();
    f.nodes[1].fixed.set(f.p(2));
    assert!(inventory.check(&mut || Ok(()), &mut || Ok(())).is_err());
}
#[test]
fn exact_families_and_canonical_interfaces_are_required() {
    for index in 0..3 {
        let f = Fixture::new();
        f.nodes[index].family.set(c"Foreign");
        assert!(f.route().is_err());
    }
    let f = Fixture::new();
    f.nodes[0].class_uid.set(API_CLASS);
    assert!(f.route().is_err());
    let f = Fixture::new();
    f.nodes[2].reflection_identity.set(f.p(35));
    assert!(f.route().is_err());
    let f = Fixture::new();
    f.nodes[2].node_interface.set(f.p(35));
    assert!(f.route().is_err());
    let f = Fixture::new();
    f.nodes[2].available.set(false);
    assert!(f.route().is_err());
    let f = Fixture::new();
    let route = Route::capture(
        f.p(35),
        f.p(0),
        c"PortSetup".as_ptr().cast(),
        &mut || Ok(()),
    );
    assert!(route.is_err());
}
#[test]
fn distinct_interface_pointers_with_same_canonical_identity_match_membership() {
    let f = Fixture::new();
    let alternative = ptr::from_ref(&f.nodes[2].views[0]).cast_mut().cast();
    f.nodes[1].children.borrow_mut()[1] = alternative;
    let route = f.route().unwrap();
    assert_eq!(route.root.as_ref().unwrap().indexed_pointer, alternative);
    route.borrowed().unwrap();
}
#[test]
fn final_borrowed_checks_reject_slot_root_and_child_drift() {
    for change in 0..9 {
        let f = Fixture::new();
        let route = f.route().unwrap();
        match change {
            0 => f.nodes[0].parent.set(f.p(35)),
            1 => f.nodes[1].fixed.set(f.p(35)),
            2 => f.nodes[1].children.borrow_mut().swap(0, 1),
            3 => f.nodes[1].count_override.set(Some(3)),
            4 => f.nodes[2].count_override.set(Some(3)),
            5 => f.nodes[2].object_uid.set(API_CLASS),
            6 => f.nodes[3].object_uid.set(API_CLASS),
            7 => f.nodes[2].children.borrow_mut()[0] = f.p(35),
            8 => f.nodes[1].object_uid.set(API_CLASS),
            _ => unreachable!(),
        };
        assert!(route.borrowed().is_err(), "case {change}");
    }
}
#[test]
fn metadata_cleanup_cannot_detach_or_change_uid_after_the_last_sweep() {
    for index in 0..3 {
        let f = Fixture::new();
        let inventory = f.inventory();
        f.nodes[index]
            .release_detach
            .set(ptr::from_ref(&*f.nodes[1]).cast_mut());
        assert!(
            inventory.check(&mut || Ok(()), &mut || Ok(())).is_err(),
            "release owner {index}"
        );
    }
    let f = Fixture::new();
    let inventory = f.inventory();
    f.nodes[2]
        .release_uid
        .set(ptr::from_ref(&*f.nodes[3]).cast_mut());
    assert!(inventory.check(&mut || Ok(()), &mut || Ok(())).is_err());
    let f = Fixture::new();
    let inventory = f.inventory();
    assert!(
        inventory
            .check(&mut || Ok(()), &mut || Err(
                "last document publication witness".into()
            ))
            .is_err()
    );
}
#[test]
fn root_first_type_survives_inherited_metadata_budget_pressure() {
    let f = Fixture::new();
    for index in 2..39 {
        f.nodes[index].type_parent.set(
            ptr::from_ref(&f.nodes[index + 1].views[3])
                .cast_mut()
                .cast(),
        );
    }
    let inventory = f.inventory();
    let value = inventory.value();
    assert_eq!(value["root_type_observed"], true);
    assert_eq!(
        value["nodes"][0]["reflection"]["types"][0]["name"],
        "AudioFXPortSetup"
    );
    assert_eq!(value["complete"], false);
    assert_eq!(value["child_metadata_complete"], false);
}

#[test]
fn cv_report_is_invalidated_on_final_document_and_output_publication_failures() {
    let _serial = crate::native_script::tests::TEST_LOCK.lock().unwrap();
    for output_failure in [false, true] {
        let mut host = super::super::super::guard_tests::Fixture::new();
        let context = host.context();
        let service = context.service;
        let f = Fixture::new();
        let inventory = f.inventory();
        let mut reply =
            Reply::unbound(json!({"ok":true,"source":"dsp","node_report":inventory.value()}));
        reply.nodes = Some(Inventory::CvPorts(inventory));
        reply.context = Some(context);
        if output_failure {
            crate::native_script::tests::ASSIGN_REENTER_STOP.store(service, Ordering::Release);
            let mut out = VariantOut {
                kind: 0,
                payload: 0,
            };
            assert!(publish_reply(&mut out, reply, api().unwrap(), service));
            let value: Value =
                serde_json::from_str(&crate::native_script::tests::output_text(&out)).unwrap();
            assert_eq!(value["ok"], false);
            assert!(value.get("node_report").is_none());
            assert_eq!(LIVE_RECEIPTS.load(Ordering::Acquire), 0);
            crate::native_script::tests::clear_output(&mut out);
        } else {
            host.switch_document();
            assert!(!reply.final_check(service));
            assert_eq!(reply.value["ok"], false);
            assert!(reply.value.get("node_report").is_none());
            drop(reply);
        }
        drop(f);
        drop(host);
    }
}

#[test]
fn cv_inventory_pins_survive_publication_until_receipt_release() {
    let _serial = crate::native_script::tests::TEST_LOCK.lock().unwrap();
    let mut host = super::super::super::guard_tests::Fixture::new();
    let context = host.context();
    let service = context.service;
    let f = Fixture::new();
    let inventory = f.inventory();
    let mut reply =
        Reply::unbound(json!({"ok":true,"source":"dsp","node_report":inventory.value()}));
    reply.nodes = Some(Inventory::CvPorts(inventory));
    reply.context = Some(context);
    let mut out = VariantOut {
        kind: 0,
        payload: 0,
    };
    assert!(publish_reply(&mut out, reply, api().unwrap(), service));
    assert_eq!(LIVE_RECEIPTS.load(Ordering::Acquire), 1);
    assert!(f.nodes[2].refs.get() > 1);
    release_receipt(service);
    assert_eq!(LIVE_RECEIPTS.load(Ordering::Acquire), 0);
    assert_eq!(f.nodes[2].refs.get(), 1);
    crate::native_script::tests::clear_output(&mut out);
}

#[test]
fn retained_same_canonical_headers_detect_class_only_changes_from_temporary_cleanup() {
    for index in 0..3 {
        let f = Fixture::new();
        let route = f.route().unwrap();
        f.nodes[index]
            .release_family
            .set(ptr::from_ref(&*f.nodes[index]).cast_mut());
        route.metadata(&mut || Ok(())).unwrap();
        assert!(
            route.borrowed().is_err(),
            "last copied type changed during cleanup {index}"
        );
    }
    for owner in 0..3 {
        for changed in 0..3 {
            let f = Fixture::new();
            let inventory = f.inventory();
            f.nodes[owner]
                .release_family
                .set(ptr::from_ref(&*f.nodes[changed]).cast_mut());
            assert!(
                inventory.check(&mut || Ok(()), &mut || Ok(())).is_err(),
                "owner {owner} changed {changed}"
            );
        }
    }
}
#[test]
fn final_membership_sweep_rejects_a_new_duplicate_elsewhere_in_the_slot() {
    let f = Fixture::new();
    let inventory = f.inventory();
    f.nodes[1].children.borrow_mut()[0] = f.p(2);
    assert!(inventory.check(&mut || Ok(()), &mut || Ok(())).is_err());
}

#[test]
fn malformed_route_qi_outputs_fail_without_adopting_failure_sentinels() {
    for iid in [UNKNOWN, NODE, OBJECT] {
        let f = Fixture::new();
        f.nodes[2].malformed.set(Some(iid));
        assert!(f.route().is_err());
    }
}
#[test]
fn publication_detects_canonical_and_class_drift_for_every_retained_route_node() {
    for index in 0..5 {
        let f = Fixture::new();
        let inventory = f.inventory();
        f.nodes[index].canonical.set(f.p(35));
        assert!(
            inventory.check(&mut || Ok(()), &mut || Ok(())).is_err(),
            "canonical {index}"
        );
    }
    for index in 0..3 {
        let f = Fixture::new();
        let inventory = f.inventory();
        f.nodes[index].family.set(c"Changed");
        assert!(
            inventory.check(&mut || Ok(()), &mut || Ok(())).is_err(),
            "family {index}"
        );
    }
}
#[test]
fn every_guard_failure_releases_all_new_route_owners() {
    for fail_at in 1..100 {
        let f = Fixture::new();
        let mut calls = 0;
        let result = Route::capture(f.p(0), f.p(0), c"PortSetup".as_ptr().cast(), &mut || {
            calls += 1;
            if calls == fail_at {
                Err("document changed".into())
            } else {
                Ok(())
            }
        });
        if calls >= fail_at {
            assert!(result.is_err());
        }
        drop(result);
    }
}

#[test]
fn empty_class_uid_is_absent_but_null_class_uid_reference_is_malformed() {
    let f = Fixture::new();
    let route = f.route().unwrap();
    assert!(route.slot_snapshot.class_uid.is_none());
    assert!(route.root.as_ref().unwrap().snapshot.class_uid.is_none());
    drop(route);
    for index in 1..4 {
        let f = Fixture::new();
        f.nodes[index].null_class.set(true);
        assert!(
            matches!(f.route(), Err(reason) if reason.contains("null UID reference")),
            "node {index}"
        );
    }
    let f = Fixture::new();
    let inventory = f.inventory();
    f.nodes[2].null_class.set(true);
    assert!(inventory.check(&mut || Ok(()), &mut || Ok(())).is_err());
}
#[test]
fn root_child_bound_is_eight_and_wrong_identifiers_are_not_relabelled() {
    let f = Fixture::new();
    *f.nodes[2].children.borrow_mut() = (3..11).map(|index| f.p(index)).collect();
    let inventory = f.inventory();
    assert_eq!(inventory.value()["nodes"].as_array().unwrap().len(), 9);
    drop(inventory);
    f.nodes[2].children.borrow_mut().push(f.p(11));
    assert!(f.route().is_err());
    let f = Fixture::new();
    *f.nodes[2].text.borrow_mut() = "Other".encode_utf16().collect();
    assert!(f.route().is_err());
}

#[test]
fn last_type_header_read_rejects_null_and_over_bound_names_without_temporary_cleanup() {
    for kind in 0..3 {
        let f = Fixture::new();
        let route = f.route().unwrap();
        match kind {
            0=>f.nodes[2].null_type.set(true),
            1=>f.nodes[2].null_name.set(true),
            2=>f.nodes[2].family.set(c"XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"),
            _=>unreachable!(),
        }
        assert!(route.borrowed().is_err());
    }
    let f = Fixture::new();
    let route = f.route().unwrap();
    let target = ptr::from_ref(&*f.nodes[1]).cast_mut();
    f.nodes[2].release_detach.set(target);
    route.borrowed().unwrap();
    assert_eq!(f.nodes[2].release_detach.get(), target);
}

#[test]
fn copied_identifier_is_rechecked_after_its_temporary_string_owner_cleanup() {
    for index in 0..3 {
        let f = Fixture::new();
        let route = f.route().unwrap();
        f.nodes[index]
            .release_id
            .set(ptr::from_ref(&*f.nodes[index]).cast_mut());
        route.metadata(&mut || Ok(())).unwrap();
        assert!(route.borrowed().is_err(), "identifier {index}");
    }
}

#[test]
fn guard_reentry_after_last_class_copy_is_checked_by_the_borrowed_header() {
    let f = Fixture::new();
    let route = f.route().unwrap();
    let mut calls = 0;
    route
        .metadata(&mut || {
            calls += 1;
            if calls == 3 {
                f.nodes[2].family.set(c"ChangedDuringGuard");
            }
            Ok(())
        })
        .unwrap();
    assert!(route.borrowed().is_err());
}

#[test]
fn final_string_copy_class_reentry_must_fail_closed() {
    let f = Fixture::new();
    let inventory = f.inventory();
    let mut baseline_calls = 0;
    inventory
        .check(
            &mut || {
                baseline_calls += 1;
                Ok(())
            },
            &mut || Ok(()),
        )
        .unwrap();
    let mut calls = 0;
    let result = inventory.check(
        &mut || {
            calls += 1;
            if calls == baseline_calls {
                f.nodes[0]
                    .copy_family
                    .set(ptr::from_ref(&*f.nodes[2]).cast_mut());
            }
            Ok(())
        },
        &mut || Ok(()),
    );
    assert_eq!(calls, baseline_calls);
    assert_eq!(f.nodes[2].family.get(), c"ChangedDuringFinalStringCopy");
    assert!(
        result.is_err(),
        "full Inventory.check accepted root class drift during final borrowed Device ID copy"
    );
}

#[path = "parameters/tests.rs"]
mod parameter_tests;

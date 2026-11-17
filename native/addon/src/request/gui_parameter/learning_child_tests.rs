use super::super::super::tests::{learning_child_parameter_refs, with_learning_child_parameters};
use super::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[repr(C)]
struct Base {
    table: *const *const c_void,
    refs: Cell<u32>,
    on_release: RefCell<Option<Rc<dyn Fn()>>>,
}
unsafe fn base<'a>(p: *mut c_void) -> &'a Base {
    unsafe { &*p.cast::<Base>() }
}
unsafe extern "C" fn retain(p: *mut c_void) -> u32 {
    let b = unsafe { base(p) };
    b.refs.set(b.refs.get() + 1);
    b.refs.get()
}
unsafe extern "C" fn drop_ref(p: *mut c_void) -> u32 {
    let b = unsafe { base(p) };
    b.refs.set(b.refs.get() - 1);
    let hook = b.on_release.borrow_mut().take();
    if let Some(hook) = hook {
        hook();
    }
    b.refs.get()
}
#[repr(C)]
struct Text {
    base: Base,
    units: Vec<u16>,
    table: Box<[*const c_void; 11]>,
}
impl Text {
    fn new(text: &str) -> Box<Self> {
        let mut table = Box::new([ptr::null(); 11]);
        table[1] = retain as *const c_void;
        table[2] = drop_ref as *const c_void;
        table[4] = length as *const c_void;
        table[8] = copy_text as *const c_void;
        Box::new(Self {
            base: Base {
                table: table.as_ptr(),
                refs: Cell::new(1),
                on_release: RefCell::new(None),
            },
            units: text.encode_utf16().collect(),
            table,
        })
    }
    fn raw(&self) -> *mut c_void {
        ptr::from_ref(self).cast_mut().cast()
    }
}
impl Drop for Text {
    fn drop(&mut self) {
        assert_eq!(self.base.refs.get(), 1);
    }
}
unsafe extern "C" fn length(p: *mut c_void) -> i32 {
    unsafe { &*p.cast::<Text>() }.units.len() as i32
}
unsafe extern "C" fn copy_text(p: *mut c_void, out: *mut u16, size: i32) -> i32 {
    let text = unsafe { &*p.cast::<Text>() };
    if size as usize <= text.units.len() {
        return FAILED;
    }
    unsafe {
        ptr::copy_nonoverlapping(text.units.as_ptr(), out, text.units.len());
        *out.add(text.units.len()) = 0;
    }
    OK
}
#[repr(C)]
struct Node {
    base: Base,
    id: PlainString,
    text: Box<Text>,
    count: Cell<i32>,
    child: Cell<*mut c_void>,
    parent: Cell<*mut c_void>,
    parameters: [Cell<*mut c_void>; 2],
    support_node: Cell<bool>,
    support_controller: Cell<bool>,
    node_interface: Cell<*mut c_void>,
    controller_interface: Cell<*mut c_void>,
    query_status: Cell<i32>,
    garbage_output: Cell<bool>,
    null_success: Cell<bool>,
    finds: Cell<usize>,
    parameter_gets: Cell<usize>,
    table: Box<[*const c_void; 11]>,
}
unsafe fn node<'a>(p: *mut c_void) -> &'a Node {
    unsafe { &*p.cast::<Node>() }
}
unsafe extern "C" fn query(p: *mut c_void, iid: *const Uid, out: *mut *mut c_void) -> i32 {
    let n = unsafe { node(p) };
    let iid = unsafe { *iid };
    unsafe {
        *out = ptr::null_mut();
    }
    if iid != UNKNOWN && n.query_status.get() != OK {
        if n.garbage_output.get() {
            unsafe {
                *out = ptr::dangling_mut();
            }
        }
        return n.query_status.get();
    }
    if iid != UNKNOWN && n.null_success.get() {
        return OK;
    }
    let raw = if iid == UNKNOWN {
        p
    } else if iid == NODE && n.support_node.get() {
        if n.node_interface.get().is_null() {
            p
        } else {
            n.node_interface.get()
        }
    } else if iid == CONTROLLER && n.support_controller.get() {
        if n.controller_interface.get().is_null() {
            p
        } else {
            n.controller_interface.get()
        }
    } else {
        return NO_INTERFACE;
    };
    unsafe {
        retain(raw);
        *out = raw;
    }
    OK
}
unsafe extern "C" fn get_id(p: *mut c_void) -> *const PlainString {
    &unsafe { node(p) }.id
}
unsafe extern "C" fn count(p: *mut c_void) -> i32 {
    unsafe { node(p) }.count.get()
}
unsafe extern "C" fn parent(p: *mut c_void) -> *mut c_void {
    unsafe { node(p) }.parent.get()
}
unsafe extern "C" fn constant(name: *const c_char) -> *const c_void {
    assert_eq!(unsafe { CStr::from_ptr(name) }, c"pitch61");
    c"pitch61".as_ptr().cast()
}
unsafe extern "C" fn find_child(p: *mut c_void, name: *const c_void) -> *mut c_void {
    assert_eq!(name, c"pitch61".as_ptr().cast());
    let n = unsafe { node(p) };
    n.finds.set(n.finds.get() + 1);
    n.child.get()
}
unsafe extern "C" fn find_parameter(p: *mut c_void, name: *const PlainCString) -> *mut c_void {
    let n = unsafe { node(p) };
    n.parameter_gets.set(n.parameter_gets.get() + 1);
    match unsafe { CStr::from_ptr((*name).text) }.to_bytes() {
        b"learnChord" => n.parameters[0].get(),
        b"chordAssigned" => n.parameters[1].get(),
        _ => panic!("unexpected local parameter lookup"),
    }
}
impl Node {
    fn new(id: &str) -> Box<Self> {
        let mut table = Box::new([ptr::null(); 11]);
        table[0] = query as *const c_void;
        table[1] = retain as *const c_void;
        table[2] = drop_ref as *const c_void;
        table[3] = get_id as *const c_void;
        table[5] = find_parameter as *const c_void;
        table[6] = parent as *const c_void;
        table[8] = count as *const c_void;
        table[10] = find_child as *const c_void;
        let text = Text::new(id);
        Box::new(Self {
            base: Base {
                table: table.as_ptr(),
                refs: Cell::new(1),
                on_release: RefCell::new(None),
            },
            id: PlainString {
                object: text.raw(),
                opaque_debug: ptr::null_mut(),
            },
            text,
            count: Cell::new(128),
            child: Cell::new(ptr::null_mut()),
            parent: Cell::new(ptr::null_mut()),
            parameters: [Cell::new(ptr::null_mut()), Cell::new(ptr::null_mut())],
            support_node: Cell::new(true),
            support_controller: Cell::new(true),
            node_interface: Cell::new(ptr::null_mut()),
            controller_interface: Cell::new(ptr::null_mut()),
            query_status: Cell::new(OK),
            garbage_output: Cell::new(false),
            null_success: Cell::new(false),
            finds: Cell::new(0),
            parameter_gets: Cell::new(0),
            table,
        })
    }
    fn raw(&self) -> *mut c_void {
        ptr::from_ref(self).cast_mut().cast()
    }
    fn pin(&self) -> InterfacePin {
        InterfacePin::capture(hold(self.raw()).unwrap(), &mut || Ok(())).unwrap()
    }
}
impl Drop for Node {
    fn drop(&mut self) {
        assert_eq!(self.base.refs.get(), 1, "balanced node refs");
    }
}
fn text_api() -> TextApi {
    TextApi {
        get_constant_string: constant,
    }
}
fn capture(root: &InterfacePin, api: &Api) -> Result<FixedLearningChild, String> {
    FixedLearningChild::capture(root, text_api(), api, &mut || Ok(()))
}
fn pair<F: FnOnce(&Api, &Node, &Node)>(run: F) {
    with_learning_child_parameters(|api, learn, assigned| {
        let root = Node::new("ChorderComponent");
        let child = Node::new("pitch61");
        root.child.set(child.raw());
        child.parent.set(root.raw());
        child.parameters[0].set(learn);
        child.parameters[1].set(assigned);
        run(api, &root, &child);
    });
}
#[test]
fn fixed_child_reads_one_named_child_and_two_local_descriptors() {
    pair(|api, root, child| {
        let pin = root.pin();
        let probe = capture(&pin, api).unwrap();
        let value = probe.value();
        assert_eq!(value["status"], "observed");
        assert_eq!(value["mutation_capability"], false);
        assert_eq!(value["parameters"]["learnChord"]["name"], "learnChord");
        assert_eq!(
            value["parameters"]["chordAssigned"]["name"],
            "chordAssigned"
        );
        assert_eq!(value["output_pitch_mapping"], "not_established");
        assert_eq!(root.finds.get(), 1);
        let mut witnesses = Vec::new();
        probe.check(api, &mut || Ok(()), &mut witnesses).unwrap();
        drop(witnesses);
        probe.retain_publication_interfaces().unwrap();
        probe.check_borrowed().unwrap();
        assert!(child.parameter_gets.get() >= 6);
        if let Ok(folder) = std::env::var("FSP_NATIVE33_CHILD_FIXTURES") {
            std::fs::write(
                std::path::Path::new(&folder).join("observed.json"),
                serde_json::to_vec_pretty(&value).unwrap(),
            )
            .unwrap();
        }
    });
}
#[test]
fn unsupported_and_failed_root_queries_never_find_a_child() {
    for scenario in 0..4 {
        pair(|api, root, _| {
            let pin = root.pin();
            match scenario {
                0 => root.support_node.set(false),
                1 => {
                    root.query_status.set(FAILED);
                    root.garbage_output.set(true);
                }
                2 => root.null_success.set(true),
                3 => root.count.set(129),
                _ => unreachable!(),
            }
            let value = capture(&pin, api).unwrap().value();
            assert_eq!(value["status"], "unresolved");
            assert_eq!(root.finds.get(), 0);
            assert!(value.get("binding").is_none());
        });
    }
}
#[test]
fn absent_wrong_id_or_detached_children_cannot_report_binding() {
    for scenario in 0..3 {
        pair(|api, root, child| {
            let other = Node::new("wrong");
            let pin = root.pin();
            match scenario {
                0 => root.child.set(ptr::null_mut()),
                1 => {
                    root.child.set(other.raw());
                    other.parent.set(root.raw());
                }
                2 => child.parent.set(other.raw()),
                _ => unreachable!(),
            }
            let probe = capture(&pin, api).unwrap();
            assert_eq!(probe.value()["status"], "unresolved");
            assert!(probe.value().get("binding").is_none());
            assert_eq!(child.parameter_gets.get(), 0);
        });
    }
}
#[test]
fn child_controller_and_missing_parameter_failures_are_bounded() {
    for scenario in 0..3 {
        pair(|api, root, child| {
            let other = Node::new("other");
            let pin = root.pin();
            match scenario {
                0 => child.support_controller.set(false),
                1 => child.controller_interface.set(other.raw()),
                2 => child.parameters[1].set(ptr::null_mut()),
                _ => unreachable!(),
            }
            let probe = capture(&pin, api).unwrap();
            assert_eq!(probe.value()["status"], "unresolved");
            let mut witnesses = Vec::new();
            probe.check(api, &mut || Ok(()), &mut witnesses).unwrap();
            drop(witnesses);
            probe.retain_publication_interfaces().unwrap();
            probe.check_borrowed().unwrap();
        });
    }
}
#[test]
fn every_capture_guard_failure_releases_successful_owners_without_a_setter() {
    let mut total = 0;
    pair(|api, root, _| {
        let pin = root.pin();
        let _probe = FixedLearningChild::capture(&pin, text_api(), api, &mut || {
            total += 1;
            Ok(())
        })
        .unwrap();
    });
    for fail in 0..total {
        pair(|api, root, _| {
            let pin = root.pin();
            let mut step = 0;
            assert!(
                FixedLearningChild::capture(&pin, text_api(), api, &mut || {
                    let reject = step == fail;
                    step += 1;
                    if reject {
                        Err("context changed".into())
                    } else {
                        Ok(())
                    }
                })
                .is_err(),
                "guard {fail}"
            );
        });
    }
}
#[test]
fn child_parent_controller_and_parameter_replacements_are_detected() {
    for scenario in 0..4 {
        pair(|api, root, child| {
            let other = Node::new("pitch61");
            let pin = root.pin();
            let probe = capture(&pin, api).unwrap();
            match scenario {
                0 => root.child.set(other.raw()),
                1 => child.parent.set(other.raw()),
                2 => child.controller_interface.set(other.raw()),
                3 => child.parameters[0].set(child.parameters[1].get()),
                _ => unreachable!(),
            }
            let mut witnesses = Vec::new();
            assert!(probe.check(api, &mut || Ok(()), &mut witnesses).is_err());
        });
    }
}
#[test]
fn witness_release_reentry_cannot_publish_a_detached_child_or_replaced_parameter() {
    for parameter in [false, true] {
        pair(|api, root, child| {
            let other = Node::new("other");
            let pin = root.pin();
            let probe = capture(&pin, api).unwrap();
            let mut witnesses = Vec::new();
            probe.check(api, &mut || Ok(()), &mut witnesses).unwrap();
            let c = child.raw();
            let p = other.raw();
            *child.base.on_release.borrow_mut() = Some(Rc::new(move || {
                let child = unsafe { node(c) };
                if parameter {
                    child.parameters[0].set(child.parameters[1].get());
                } else {
                    child.parent.set(p);
                }
            }));
            drop(witnesses);
            probe.retain_publication_interfaces().unwrap();
            assert!(probe.check_borrowed().is_err());
        });
    }
}
#[test]
fn local_parameter_pins_are_released_before_root_owner_cleanup() {
    pair(|api, root, child| {
        let pin = root.pin();
        let probe = capture(&pin, api).unwrap();
        let learn = child.parameters[0].get();
        let assigned = child.parameters[1].get();
        *root.base.on_release.borrow_mut() = Some(Rc::new(move || {
            assert_eq!(learning_child_parameter_refs(learn), 1);
            assert_eq!(learning_child_parameter_refs(assigned), 1);
        }));
        drop(probe);
    });
}
#[test]
fn unresolved_non_direct_probe_has_no_interfaces_or_authority() {
    let value = FixedLearningChild::not_direct().value();
    assert_eq!(value["reason"], "root_not_direct");
    assert_eq!(value["interfaces"]["root_node"]["status"], "not_attempted");
    assert_eq!(value["root_child_count"], Value::Null);
    assert!(value.get("binding").is_none());
}
#[test]
fn late_release_controller_and_root_interface_reassociation_refuse_publication() {
    for controller in [true, false] {
        pair(|api, root, child| {
            let other = Node::new("other");
            let pin = root.pin();
            let probe = capture(&pin, api).unwrap();
            let mut witnesses = Vec::new();
            probe.check(api, &mut || Ok(()), &mut witnesses).unwrap();
            let target = if controller { child.raw() } else { root.raw() };
            let replacement = other.raw();
            *unsafe { base(target) }.on_release.borrow_mut() = Some(Rc::new(move || {
                let target = unsafe { node(target) };
                if controller {
                    target.controller_interface.set(replacement);
                } else {
                    target.node_interface.set(replacement);
                }
            }));
            drop(witnesses);
            assert!(probe.retain_publication_interfaces().is_err());
        });
    }
}
#[test]
fn publication_witnesses_survive_both_terminal_passes_and_are_bounded() {
    pair(|api, root, child| {
        let pin = root.pin();
        let probe = capture(&pin, api).unwrap();
        let released = Rc::new(Cell::new(false));
        let did_release = Rc::clone(&released);
        let c = child.raw();
        let learn = child.parameters[0].get();
        *child.base.on_release.borrow_mut() = Some(Rc::new(move || {
            assert!(unsafe { base(c) }.refs.get() > 1);
            assert!(learning_child_parameter_refs(learn) > 1);
            did_release.set(true);
        }));
        for count in [7, 14] {
            probe.retain_publication_interfaces().unwrap();
            probe.check_borrowed().unwrap();
            assert_eq!(probe.publication_witnesses.borrow().len(), count);
            assert!(!released.get());
        }
        assert!(probe.retain_publication_interfaces().is_err());
        assert!(!released.get());
        assert_eq!(probe.publication_witnesses.borrow().len(), 14);
        drop(probe);
        assert!(released.get());
    });
}
#[test]
fn retired_publication_witnesses_reenter_before_fresh_normal_checks() {
    pair(|api, root, child| {
        let other = Node::new("other");
        let pin = root.pin();
        let probe = capture(&pin, api).unwrap();
        probe.retain_publication_interfaces().unwrap();
        probe.check_borrowed().unwrap();
        let c = child.raw();
        let replacement = other.raw();
        *child.base.on_release.borrow_mut() = Some(Rc::new(move || {
            unsafe { node(c) }.controller_interface.set(replacement);
        }));
        probe.retire_publication_witnesses(&mut || Ok(())).unwrap();
        assert!(probe.publication_witnesses.borrow().is_empty());
        let mut witnesses = Vec::new();
        assert!(probe.check(api, &mut || Ok(()), &mut witnesses).is_err());
    });
}

#[path = "chord_note_tests.rs"]
mod note_tests;

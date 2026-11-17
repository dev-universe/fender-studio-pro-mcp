use super::*;
use std::cell::{Cell, RefCell};

thread_local! { static EMPTY_NAME_STRING: Cell<*mut c_void> = const { Cell::new(ptr::null_mut()) }; }
#[repr(C)]
struct Fake {
    table: *const *const c_void,
    refs: Cell<u32>,
    role: u8,
    context: Rc<Cell<bool>>,
    controller: Cell<*mut c_void>,
    canonical: Cell<*mut c_void>,
    parameter: Cell<*mut c_void>,
    text: Cell<*mut c_void>,
    query_mode: Cell<u8>,
    kind: Cell<i32>,
    name: PlainCString,
    lookups: RefCell<Vec<String>>,
    units: RefCell<Vec<u16>>,
    stop_release_at: Cell<Option<u32>>,
    queries: Cell<u32>,
    displays: Cell<u32>,
}
unsafe fn fake<'a>(pointer: *mut c_void) -> &'a Fake {
    unsafe { &*pointer.cast::<Fake>() }
}
unsafe extern "C" fn retain(pointer: *mut c_void) -> u32 {
    let object = unsafe { fake(pointer) };
    object.refs.set(object.refs.get() + 1);
    object.refs.get()
}
unsafe extern "C" fn release(pointer: *mut c_void) -> u32 {
    let object = unsafe { fake(pointer) };
    if object.stop_release_at.get() == Some(object.refs.get()) {
        object.context.set(false);
    }
    object
        .refs
        .set(object.refs.get().checked_sub(1).expect("double release"));
    object.refs.get()
}
unsafe extern "C" fn qi(pointer: *mut c_void, iid: *const Uid, out: *mut *mut c_void) -> i32 {
    let object = unsafe { fake(pointer) };
    let iid = unsafe { *iid };
    object.queries.set(object.queries.get() + 1);
    unsafe { *out = ptr::null_mut() };
    if iid == CONTROLLER && object.role == 0 {
        match object.query_mode.get() {
            1 => return NO_INTERFACE,
            2 => return OK,
            3 => return FAILED,
            4 => {
                unsafe { *out = object.controller.get() };
                return FAILED;
            }
            _ => {}
        }
        let view = object.controller.get();
        unsafe {
            retain(view);
            *out = view;
        }
        return OK;
    }
    if iid == UNKNOWN {
        let view = if object.canonical.get().is_null() {
            pointer
        } else {
            object.canonical.get()
        };
        unsafe {
            retain(view);
            *out = view;
        }
        return OK;
    }
    NO_INTERFACE
}
unsafe extern "C" fn find(pointer: *mut c_void, name: *const PlainCString) -> *mut c_void {
    let object = unsafe { fake(pointer) };
    let name = unsafe { std::ffi::CStr::from_ptr((*name).text) }
        .to_str()
        .unwrap()
        .to_owned();
    object.lookups.borrow_mut().push(name);
    object.parameter.get()
}
unsafe extern "C" fn kind(pointer: *mut c_void) -> i32 {
    unsafe { fake(pointer) }.kind.get()
}
unsafe extern "C" fn name(pointer: *mut c_void) -> *const PlainCString {
    &unsafe { fake(pointer) }.name
}
unsafe extern "C" fn tag(_: *mut c_void) -> i32 {
    7
}
unsafe extern "C" fn display(pointer: *mut c_void, output: *mut PlainString) {
    let object = unsafe { fake(pointer) };
    object.displays.set(object.displays.get() + 1);
    let output = unsafe { &mut *output };
    unsafe {
        release(output.object);
        retain(object.text.get());
    }
    output.object = object.text.get();
    output.opaque_debug = ptr::null_mut();
}
unsafe extern "C" fn length(pointer: *mut c_void) -> i32 {
    unsafe { fake(pointer) }.units.borrow().len() as i32
}
unsafe extern "C" fn copy(pointer: *mut c_void, out: *mut u16, size: i32) -> i32 {
    let object = unsafe { fake(pointer) };
    let units = object.units.borrow();
    if size != units.len() as i32 + 1 {
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
unsafe extern "C" fn empty() -> *mut c_void {
    EMPTY_NAME_STRING.with(Cell::get)
}
unsafe extern "C" fn unused() -> *mut c_void {
    ptr::null_mut()
}
unsafe extern "C" fn thread() -> i64 {
    0
}
unsafe extern "C" fn constant(_: *const c_char) -> *const PlainCString {
    ptr::null()
}

struct Fixture {
    nodes: Box<[Fake; 6]>,
    _tables: [Box<[*const c_void; 46]>; 4],
    context: Rc<Cell<bool>>,
}
impl Fixture {
    fn new() -> Self {
        let context = Rc::new(Cell::new(true));
        let mut tables: [Box<[*const c_void; 46]>; 4] =
            std::array::from_fn(|_| Box::new([ptr::null(); 46]));
        for table in &mut tables {
            table[0] = qi as *const c_void;
            table[1] = retain as *const c_void;
            table[2] = release as *const c_void;
        }
        tables[1][5] = find as *const c_void;
        tables[2][3] = kind as *const c_void;
        tables[2][4] = name as *const c_void;
        tables[2][13] = tag as *const c_void;
        tables[2][44] = display as *const c_void;
        tables[3][4] = length as *const c_void;
        tables[3][8] = copy as *const c_void;
        tables[3][21] = private as *const c_void;
        let nodes = Box::new(std::array::from_fn(|index| Fake {
            table: tables[if index >= 3 {
                if index == 5 { 0 } else { 3 }
            } else {
                index
            }]
            .as_ptr(),
            refs: Cell::new(1),
            role: index as u8,
            context: context.clone(),
            controller: Cell::new(ptr::null_mut()),
            canonical: Cell::new(ptr::null_mut()),
            parameter: Cell::new(ptr::null_mut()),
            text: Cell::new(ptr::null_mut()),
            query_mode: Cell::new(0),
            kind: Cell::new(3),
            name: PlainCString {
                text: c"presetName".as_ptr(),
                owner: ptr::null_mut(),
            },
            lookups: RefCell::new(Vec::new()),
            units: RefCell::new(if index == 4 {
                "Initial* 한글 🎸".encode_utf16().collect()
            } else {
                Vec::new()
            }),
            stop_release_at: Cell::new(None),
            queries: Cell::new(0),
            displays: Cell::new(0),
        }));
        let fixture = Self {
            nodes,
            _tables: tables,
            context,
        };
        fixture.nodes[0].controller.set(fixture.pointer(1));
        fixture.nodes[1].canonical.set(fixture.pointer(0));
        fixture.nodes[1].parameter.set(fixture.pointer(2));
        fixture.nodes[2].text.set(fixture.pointer(4));
        EMPTY_NAME_STRING.with(|empty| {
            assert!(empty.get().is_null());
            empty.set(fixture.pointer(3));
        });
        fixture
    }
    fn pointer(&self, index: usize) -> *mut c_void {
        ptr::from_ref(&self.nodes[index]).cast_mut().cast()
    }
    fn observe(&self) -> Result<Value, String> {
        self.observe_guard(&mut || {
            if self.context.get() {
                Ok(())
            } else {
                Err("context changed".into())
            }
        })
    }
    fn observe_guard<G: FnMut() -> Result<(), String>>(
        &self,
        guard: &mut G,
    ) -> Result<Value, String> {
        let api = Api {
            gui: unused,
            objects: unused,
            main_thread: unused,
            thread_self: thread,
            cstring: constant,
            empty,
            desktop: unused,
            scripting: unused,
            plugin_manager: unused,
            file_system: unused,
        };
        let presets = hold(self.pointer(0)).unwrap();
        let identity = query(self.pointer(0), &UNKNOWN).unwrap();
        current_name(&presets, &identity, &api, guard)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        EMPTY_NAME_STRING.with(|empty| empty.set(ptr::null_mut()));
        assert!(
            self.nodes.iter().all(|node| node.refs.get() == 1),
            "temporary reference leak"
        );
    }
}

#[test]
fn fixed_name_observation_uses_exact_public_controller_and_only_one_parameter() {
    let fixture = Fixture::new();
    let value = fixture.observe().unwrap();
    assert_eq!(
        value,
        json!({"status":"observed","display_text":"Initial* 한글 🎸","scope":"preset_component_display","atomic_snapshot":false})
    );
    assert_eq!(*fixture.nodes[1].lookups.borrow(), ["presetName"]);
    assert_eq!(
        fixture.nodes[2].queries.get(),
        0,
        "no IObject/property reflection"
    );
    assert_eq!(fixture.nodes[2].displays.get(), 1);
}
#[test]
fn known_missing_capabilities_keep_catalog_compatible_without_fabricating_text() {
    for scenario in 0..3 {
        let fixture = Fixture::new();
        match scenario {
            0 => fixture.nodes[0].query_mode.set(1),
            1 => fixture.nodes[1].parameter.set(ptr::null_mut()),
            _ => fixture.nodes[2].kind.set(2),
        }
        let value = fixture.observe().unwrap();
        assert_eq!(value["status"], "unavailable");
        assert!(value.get("display_text").is_none());
        assert_eq!(fixture.nodes[2].displays.get(), 0);
    }
}
#[test]
fn malformed_queries_foreign_identity_wrong_parameter_and_invalid_text_withhold_observation() {
    for scenario in 0..6 {
        let mut fixture = Fixture::new();
        match scenario {
            0..=2 => fixture.nodes[0].query_mode.set(scenario + 2),
            3 => fixture.nodes[1].canonical.set(fixture.pointer(5)),
            4 => fixture.nodes[2].name.text = c"presetMenu".as_ptr(),
            _ => *fixture.nodes[4].units.borrow_mut() = vec![0xd800],
        }
        assert!(fixture.observe().is_err(), "scenario{scenario}");
    }
}
#[test]
fn controller_parameter_and_both_string_release_reentries_invalidate_the_name() {
    for (index, refs) in [(0, 4), (1, 2), (2, 2), (4, 3), (4, 2)] {
        let fixture = Fixture::new();
        fixture.nodes[index].stop_release_at.set(Some(refs));
        assert!(fixture.observe().is_err(), "owner{index} refs{refs}");
    }
}
#[test]
fn publication_invalidation_removes_current_name_without_changing_apply_semantics() {
    let mut value = json!({"ok":true,"outcome":"observed","current_preset_observation":{"status":"observed","display_text":"stale"}});
    invalidate(&mut value);
    assert!(value.get("current_preset_observation").is_none());
    let accepted = apply_value(
        "TOKEN",
        0,
        Ok(fender_native_preset_api::Recall {
            accepted: true,
            verified: false,
        }),
    );
    assert_eq!(accepted["verified"], false);
    assert_eq!(accepted["outcome"], "applied_unverified");
    assert!(accepted.get("current_preset_observation").is_none());
}

#[test]
fn every_name_route_guard_is_fail_closed_and_the_last_guard_follows_temporary_release() {
    let fixture = Fixture::new();
    let mut calls = 0;
    fixture
        .observe_guard(&mut || {
            calls += 1;
            Ok(())
        })
        .unwrap();
    drop(fixture);
    for stop in 1..=calls {
        let fixture = Fixture::new();
        let mut observed = 0;
        assert!(
            fixture
                .observe_guard(&mut || {
                    observed += 1;
                    if observed >= stop {
                        Err("document/attachment changed".into())
                    } else {
                        Ok(())
                    }
                })
                .is_err(),
            "guard{stop}"
        );
    }
    let fixture = Fixture::new();
    let mut observed = 0;
    fixture
        .observe_guard(&mut || {
            observed += 1;
            if observed == calls {
                assert_eq!(
                    fixture.nodes[1].refs.get(),
                    1,
                    "controller already released"
                );
                assert_eq!(fixture.nodes[2].refs.get(), 1, "parameter already released");
                assert_eq!(
                    fixture.nodes[4].refs.get(),
                    1,
                    "OUT String already released"
                );
            }
            Ok(())
        })
        .unwrap();
}

use super::super::super::super::chord_note::{NotePins, Selection};
use super::*;
use std::ffi::CStr;

type Hook = Rc<dyn Fn()>;
thread_local! { static QUERY_CALLS: Cell<usize> = const { Cell::new(0) }; }
unsafe extern "C" fn note_query(
    pointer: *mut c_void,
    iid: *const Uid,
    out: *mut *mut c_void,
) -> i32 {
    QUERY_CALLS.with(|calls| calls.set(calls.get() + 1));
    unsafe { query(pointer, iid, out) }
}
#[repr(C)]
struct Parameter {
    table: *const *const c_void,
    refs: Cell<u32>,
    name: PlainCString,
    tag: Cell<i32>,
    kind: Cell<i32>,
    enabled: Cell<bool>,
    readonly: Cell<bool>,
    current: Cell<i64>,
    min: Cell<i64>,
    max: Cell<i64>,
    default: Cell<i64>,
    canonical: Cell<*mut c_void>,
    controller: Cell<*mut c_void>,
    original: Cell<*mut c_void>,
    object: *mut c_void,
    calls: RefCell<Vec<&'static str>>,
    read_count: Cell<usize>,
    on_read: RefCell<Option<Hook>>,
    on_begin: RefCell<Option<Hook>>,
    on_value: RefCell<Option<Hook>>,
    on_end: RefCell<Option<Hook>>,
    on_release: RefCell<Option<Hook>>,
}
#[repr(C)]
struct Object {
    table: *const *const c_void,
    refs: Cell<u32>,
    parameter: *const Parameter,
}
unsafe fn parameter<'a>(pointer: *mut c_void) -> &'a Parameter {
    unsafe { &*pointer.cast::<Parameter>() }
}
unsafe extern "C" fn param_retain(pointer: *mut c_void) -> u32 {
    let p = unsafe { parameter(pointer) };
    p.refs.set(p.refs.get() + 1);
    p.refs.get()
}
unsafe extern "C" fn param_release(pointer: *mut c_void) -> u32 {
    let p = unsafe { parameter(pointer) };
    p.refs.set(p.refs.get() - 1);
    let hook = p.on_release.borrow_mut().take();
    if let Some(hook) = hook {
        hook();
    }
    p.refs.get()
}
unsafe extern "C" fn object_retain(pointer: *mut c_void) -> u32 {
    let o = unsafe { &*pointer.cast::<Object>() };
    o.refs.set(o.refs.get() + 1);
    o.refs.get()
}
unsafe extern "C" fn object_release(pointer: *mut c_void) -> u32 {
    let o = unsafe { &*pointer.cast::<Object>() };
    o.refs.set(o.refs.get() - 1);
    o.refs.get()
}
unsafe extern "C" fn param_query(
    pointer: *mut c_void,
    iid: *const Uid,
    out: *mut *mut c_void,
) -> i32 {
    QUERY_CALLS.with(|calls| calls.set(calls.get() + 1));
    let p = unsafe { parameter(pointer) };
    let iid = unsafe { *iid };
    let target = if iid == UNKNOWN {
        if p.canonical.get().is_null() {
            pointer
        } else {
            p.canonical.get()
        }
    } else if iid == OBJECT {
        p.object
    } else {
        ptr::null_mut()
    };
    unsafe {
        *out = target;
    }
    if target.is_null() {
        NO_INTERFACE
    } else {
        let retain: unsafe extern "C" fn(*mut c_void) -> u32 = unsafe { method(target, 1) };
        unsafe {
            retain(target);
        }
        OK
    }
}
unsafe extern "C" fn param_name(pointer: *mut c_void) -> *const PlainCString {
    &unsafe { parameter(pointer) }.name
}
unsafe extern "C" fn param_kind(pointer: *mut c_void) -> i32 {
    unsafe { parameter(pointer) }.kind.get()
}
unsafe extern "C" fn param_tag(pointer: *mut c_void) -> i32 {
    unsafe { parameter(pointer) }.tag.get()
}
unsafe extern "C" fn param_enabled(pointer: *mut c_void) -> u8 {
    unsafe { parameter(pointer) }.enabled.get().into()
}
unsafe extern "C" fn param_state(pointer: *mut c_void, state: i32) -> u8 {
    if state == 0x1000 {
        unsafe { parameter(pointer) }.readonly.get().into()
    } else {
        0
    }
}
unsafe extern "C" fn param_controller(pointer: *mut c_void) -> *mut c_void {
    unsafe { parameter(pointer) }.controller.get()
}
unsafe extern "C" fn param_original(pointer: *mut c_void) -> *mut c_void {
    unsafe { parameter(pointer) }.original.get()
}
fn hook(value: &RefCell<Option<Hook>>) {
    let callback = value.borrow().clone();
    if let Some(callback) = callback {
        callback();
    }
}
unsafe extern "C" fn param_begin(pointer: *mut c_void) {
    let p = unsafe { parameter(pointer) };
    p.calls.borrow_mut().push("begin");
    hook(&p.on_begin);
}
unsafe extern "C" fn param_end(pointer: *mut c_void) {
    let p = unsafe { parameter(pointer) };
    p.calls.borrow_mut().push("end");
    hook(&p.on_end);
}
unsafe extern "C" fn param_set(pointer: *mut c_void, value: *const VariantOut, update: u8) {
    let p = unsafe { parameter(pointer) };
    assert_eq!(update, 1);
    assert_eq!(unsafe { (*value).kind }, 1);
    p.calls.borrow_mut().push("value");
    p.current.set(unsafe { (*value).payload } as i64);
    hook(&p.on_value);
}
unsafe extern "C" fn normalized(pointer: *mut c_void) -> f32 {
    let p = unsafe { parameter(pointer) };
    (p.current.get() - p.min.get()) as f32 / (p.max.get() - p.min.get()) as f32
}
unsafe extern "C" fn display(_: *mut c_void, _: *mut c_void) {}
unsafe extern "C" fn formatter(_: *mut c_void) -> *mut c_void {
    ptr::null_mut()
}
unsafe extern "C" fn property(pointer: *mut c_void, out: *mut VariantOut, id: *const c_void) -> u8 {
    let o = unsafe { &*pointer.cast::<Object>() };
    let p = unsafe { &*o.parameter };
    let name = unsafe { CStr::from_ptr(id.cast()) };
    p.read_count.set(p.read_count.get() + 1);
    let value = match name.to_bytes() {
        b"value" => p.current.get(),
        b"min" => p.min.get(),
        b"max" => p.max.get(),
        b"default" => p.default.get(),
        _ => return 0,
    };
    unsafe {
        *out = VariantOut {
            kind: 1,
            payload: value as u64,
        };
    }
    hook(&p.on_read);
    1
}
struct NativeParameter {
    value: Box<Parameter>,
    object: Box<Object>,
    _parameter_table: Box<[*const c_void; 46]>,
    _object_table: Box<[*const c_void; 5]>,
}
impl NativeParameter {
    fn new(name: &'static CStr, tag: i32, kind: i32, values: [i64; 4], readonly: bool) -> Self {
        let [current, min, max, default] = values;
        let mut parameter_table = Box::new([ptr::null(); 46]);
        parameter_table[0] = param_query as *const c_void;
        parameter_table[1] = param_retain as *const c_void;
        parameter_table[2] = param_release as *const c_void;
        parameter_table[3] = param_kind as *const c_void;
        parameter_table[4] = param_name as *const c_void;
        parameter_table[6] = param_enabled as *const c_void;
        parameter_table[8] = param_state as *const c_void;
        parameter_table[12] = param_name as *const c_void;
        parameter_table[13] = param_tag as *const c_void;
        parameter_table[14] = param_controller as *const c_void;
        parameter_table[16] = param_begin as *const c_void;
        parameter_table[17] = param_end as *const c_void;
        parameter_table[18] = param_original as *const c_void;
        parameter_table[21] = param_set as *const c_void;
        parameter_table[35] = normalized as *const c_void;
        parameter_table[41] = formatter as *const c_void;
        parameter_table[44] = display as *const c_void;
        let mut object_table = Box::new([ptr::null(); 5]);
        object_table[1] = object_retain as *const c_void;
        object_table[2] = object_release as *const c_void;
        object_table[4] = property as *const c_void;
        let mut value = Box::new(Parameter {
            table: parameter_table.as_ptr(),
            refs: Cell::new(1),
            name: PlainCString {
                text: name.as_ptr(),
                owner: ptr::null_mut(),
            },
            tag: Cell::new(tag),
            kind: Cell::new(kind),
            enabled: Cell::new(true),
            readonly: Cell::new(readonly),
            current: Cell::new(current),
            min: Cell::new(min),
            max: Cell::new(max),
            default: Cell::new(default),
            canonical: Cell::new(ptr::null_mut()),
            controller: Cell::new(ptr::null_mut()),
            original: Cell::new(ptr::null_mut()),
            object: ptr::null_mut(),
            calls: RefCell::new(Vec::new()),
            read_count: Cell::new(0),
            on_read: RefCell::new(None),
            on_begin: RefCell::new(None),
            on_value: RefCell::new(None),
            on_end: RefCell::new(None),
            on_release: RefCell::new(None),
        });
        let mut object = Box::new(Object {
            table: object_table.as_ptr(),
            refs: Cell::new(1),
            parameter: ptr::from_ref(&*value),
        });
        value.object = ptr::from_mut(&mut *object).cast();
        value.original.set(ptr::from_ref(&*value).cast_mut().cast());
        Self {
            value,
            object,
            _parameter_table: parameter_table,
            _object_table: object_table,
        }
    }
    fn raw(&self) -> *mut c_void {
        ptr::from_ref(&*self.value).cast_mut().cast()
    }
    fn parameter(&self, api: &Api) -> RawParameter {
        retained_parameter(self.raw(), api).unwrap()
    }
}
impl Drop for NativeParameter {
    fn drop(&mut self) {
        assert_eq!(self.value.refs.get(), 1, "balanced parameter refs");
        assert_eq!(self.object.refs.get(), 1, "balanced numeric object refs");
    }
}
unsafe extern "C" fn note_constant(name: *const c_char) -> *const c_void {
    assert!(
        CHILD_NAMES
            .iter()
            .any(|child| child.to_bytes() == unsafe { CStr::from_ptr(name) }.to_bytes())
    );
    name.cast()
}
unsafe extern "C" fn note_find_child(pointer: *mut c_void, name: *const c_void) -> *mut c_void {
    let n = unsafe { node(pointer) };
    n.finds.set(n.finds.get() + 1);
    let child = n.child.get();
    if child.is_null() {
        return child;
    }
    let wanted = unsafe { CStr::from_ptr(name.cast()) }.to_string_lossy();
    let actual = String::from_utf16(&unsafe { node(child) }.text.units).unwrap();
    if wanted == actual {
        child
    } else {
        ptr::null_mut()
    }
}
struct Notes {
    mode: NativeParameter,
    pitch: NativeParameter,
    member: NativeParameter,
    assigned: NativeParameter,
    root: Box<Node>,
    child: Box<Node>,
}
impl Notes {
    fn new(input: u8, output: u8) -> Self {
        let mode = NativeParameter::new(c"learnMode", 5, 0, [1, 0, 1, 0], false);
        let pitch = NativeParameter::new(
            c"learnInputPitch",
            6,
            1,
            [i64::from(input), -1, 127, -1],
            false,
        );
        let member = NativeParameter::new(c"learnChord", 1, 0, [0, 0, 1, 0], false);
        let assigned = NativeParameter::new(c"chordAssigned", 6, 1, [0, 0, 2, 0], true);
        let mut root = Node::new("ChorderComponent");
        let mut child = Node::new(&format!("pitch{}", u16::from(output) + 1));
        root.table[0] = note_query as *const c_void;
        child.table[0] = note_query as *const c_void;
        root.table[10] = note_find_child as *const c_void;
        root.child.set(child.raw());
        child.parent.set(root.raw());
        child.parameters[0].set(member.raw());
        child.parameters[1].set(assigned.raw());
        member.value.controller.set(child.raw());
        assigned.value.controller.set(child.raw());
        Self {
            mode,
            pitch,
            member,
            assigned,
            root,
            child,
        }
    }
    fn pins(&self, api: &Api, input: u8, output: u8) -> Result<NotePins, String> {
        let selected = Selection::capture(
            self.mode.parameter(api),
            self.pitch.parameter(api),
            input,
            &mut || Ok(()),
        )?;
        let root = self.root.pin();
        let child = FixedLearningChild::capture_note(
            &root,
            output,
            TextApi {
                get_constant_string: note_constant,
            },
            api,
            &mut || selected.check_borrowed(),
        )?;
        let child = super::super::super::LearningNoteChild {
            child,
            witnesses: RefCell::new(Vec::new()),
        };
        let member = child
            .parameter(api)?
            .pin_raw_toggle_binding("learnChord", &mut || selected.check_borrowed())
            .map_err(|error| error.to_string())?;
        Ok(NotePins {
            selected,
            child,
            member,
        })
    }
}
fn with_notes<F: FnOnce(&Api, &Notes)>(input: u8, output: u8, run: F) {
    with_learning_child_parameters(|api, _, _| {
        let notes = Notes::new(input, output);
        run(api, &notes);
    });
}
#[test]
fn note_membership_uses_only_declared_bounded_child_slots_and_one_lifecycle() {
    for output in [0, 60, 127] {
        with_notes(60, output, |api, notes| {
            let pins = notes.pins(api, 60, output).unwrap();
            pins.check(api, &|| Ok(()), &|| Ok(())).unwrap();
            let attempt = pins.edit(
                pins.child.parameter(api).unwrap(),
                true,
                api,
                &|| Ok(()),
                &|| Ok(()),
            );
            assert!(attempt.result.as_ref().unwrap().verified);
            assert_eq!(
                &*notes.member.value.calls.borrow(),
                &["begin", "value", "end"]
            );
            assert_eq!(notes.member.value.current.get(), 1);
            assert!(notes.mode.value.calls.borrow().is_empty());
            assert!(notes.pitch.value.calls.borrow().is_empty());
            attempt
                .lease
                .check(&mut || pins.check(api, &|| Ok(()), &|| Ok(())))
                .unwrap();
            pins.child.retain_publication().unwrap();
            pins.check_borrowed(&|| Ok(())).unwrap();
        });
    }
}
#[test]
fn missing_or_nonexact_selection_refuses_without_dispatching_any_control() {
    for scenario in 0..7 {
        with_notes(60, 0, |api, notes| {
            match scenario {
                0 => notes.mode.value.current.set(0),
                1 => notes.pitch.value.current.set(-1),
                2 => notes.pitch.value.current.set(61),
                3 => notes.mode.value.tag.set(7),
                4 => notes.mode.value.default.set(1),
                5 => notes.pitch.value.max.set(128),
                6 => notes.pitch.value.readonly.set(true),
                _ => unreachable!(),
            }
            assert!(notes.pins(api, 60, 0).is_err());
            assert!(notes.member.value.calls.borrow().is_empty());
            assert!(notes.mode.value.calls.borrow().is_empty());
            assert!(notes.pitch.value.calls.borrow().is_empty());
        });
    }
}
#[test]
fn malformed_membership_or_indicator_profiles_never_reach_begin() {
    for scenario in 0..16 {
        with_notes(60, 0, |api, notes| {
            match scenario {
                0 => notes.member.value.tag.set(2),
                1 => notes.member.value.kind.set(1),
                2 => notes.member.value.readonly.set(true),
                3 => notes.member.value.enabled.set(false),
                4 => notes.member.value.min.set(-1),
                5 => notes.member.value.max.set(2),
                6 => notes.member.value.default.set(1),
                7 => notes.member.value.current.set(2),
                8 => notes.assigned.value.readonly.set(false),
                9 => notes.assigned.value.kind.set(0),
                10 => notes.assigned.value.max.set(3),
                11 => notes.assigned.value.tag.set(7),
                12 => notes.root.child.set(ptr::null_mut()),
                13 => notes.child.parent.set(ptr::null_mut()),
                14 => notes.child.parameters[0].set(ptr::null_mut()),
                15 => notes.child.parameters[0].set(notes.assigned.raw()),
                _ => unreachable!(),
            }
            assert!(notes.pins(api, 60, 0).is_err());
            assert!(notes.member.value.calls.borrow().is_empty());
        });
    }
}
#[test]
fn selection_switches_inside_begin_value_or_end_invalidate_and_stop_next_dispatch() {
    for stage in 0..3 {
        for mode_change in [false, true] {
            with_notes(60, 0, |api, notes| {
                let pins = notes.pins(api, 60, 0).unwrap();
                let mode = notes.mode.raw() as usize;
                let pitch = notes.pitch.raw() as usize;
                let callback = Rc::new(move || unsafe {
                    if mode_change {
                        parameter(mode as *mut c_void).current.set(0)
                    } else {
                        parameter(pitch as *mut c_void).current.set(61)
                    }
                });
                *[
                    &notes.member.value.on_begin,
                    &notes.member.value.on_value,
                    &notes.member.value.on_end,
                ][stage]
                    .borrow_mut() = Some(callback);
                let attempt = pins.edit(
                    pins.child.parameter(api).unwrap(),
                    true,
                    api,
                    &|| Ok(()),
                    &|| Ok(()),
                );
                assert_eq!(
                    attempt.result.as_ref().unwrap_err().outcome,
                    parameters::Outcome::Unknown
                );
                assert_eq!(
                    attempt.lease.phase(),
                    [
                        parameters::NumericEditPhase::BeginDispatched,
                        parameters::NumericEditPhase::ValueDispatched,
                        parameters::NumericEditPhase::EndDispatched
                    ][stage]
                );
                assert_eq!(
                    &*notes.member.value.calls.borrow(),
                    [
                        &["begin"][..],
                        &["begin", "value"][..],
                        &["begin", "value", "end"][..]
                    ][stage]
                );
            });
        }
    }
}
#[test]
fn child_detach_or_replacement_at_lifecycle_boundaries_is_unknown_without_rebinding() {
    for stage in 0..3 {
        for replacement in [false, true] {
            with_notes(60, 0, |api, notes| {
                let pins = notes.pins(api, 60, 0).unwrap();
                let child = notes.child.raw() as usize;
                let root = notes.root.raw() as usize;
                let callback = Rc::new(move || unsafe {
                    if replacement {
                        node(root as *mut c_void).child.set(ptr::null_mut())
                    } else {
                        node(child as *mut c_void).parent.set(ptr::null_mut())
                    }
                });
                *[
                    &notes.member.value.on_begin,
                    &notes.member.value.on_value,
                    &notes.member.value.on_end,
                ][stage]
                    .borrow_mut() = Some(callback);
                let attempt = pins.edit(
                    pins.child.parameter(api).unwrap(),
                    true,
                    api,
                    &|| Ok(()),
                    &|| Ok(()),
                );
                assert_eq!(
                    attempt.result.as_ref().unwrap_err().outcome,
                    parameters::Outcome::Unknown
                );
                assert_eq!(
                    &*notes.member.value.calls.borrow(),
                    [
                        &["begin"][..],
                        &["begin", "value"][..],
                        &["begin", "value", "end"][..]
                    ][stage]
                );
            });
        }
    }
}

#[test]
fn selection_switch_in_last_member_scalar_read_blocks_each_next_dispatch() {
    let mut boundaries = [0; 3];
    with_notes(60, 0, |api, notes| {
        let pins = notes.pins(api, 60, 0).unwrap();
        let counters: Rc<[Cell<usize>; 3]> = Rc::new(std::array::from_fn(|_| Cell::new(0)));
        let member = notes.member.raw() as usize;
        for (index, field) in [
            &notes.member.value.on_begin,
            &notes.member.value.on_value,
            &notes.member.value.on_end,
        ]
        .into_iter()
        .enumerate()
        {
            let counters = counters.clone();
            *field.borrow_mut() = Some(Rc::new(move || {
                counters[index].set(unsafe { parameter(member as *mut c_void) }.read_count.get())
            }));
        }
        notes.member.value.read_count.set(0);
        let attempt = pins.edit(
            pins.child.parameter(api).unwrap(),
            true,
            api,
            &|| Ok(()),
            &|| Ok(()),
        );
        assert!(attempt.result.as_ref().unwrap().verified);
        boundaries = std::array::from_fn(|index| counters[index].get());
    });
    assert!(boundaries[0] > 0 && boundaries[0] < boundaries[1] && boundaries[1] < boundaries[2]);
    for (stage, read) in boundaries.into_iter().enumerate() {
        for change_mode in [false, true] {
            with_notes(60, 0, |api, notes| {
                let pins = notes.pins(api, 60, 0).unwrap();
                let member = notes.member.raw() as usize;
                let mode = notes.mode.raw() as usize;
                let pitch = notes.pitch.raw() as usize;
                notes.member.value.read_count.set(0);
                *notes.member.value.on_read.borrow_mut() = Some(Rc::new(move || unsafe {
                    if parameter(member as *mut c_void).read_count.get() == read {
                        if change_mode {
                            parameter(mode as *mut c_void).current.set(0)
                        } else {
                            parameter(pitch as *mut c_void).current.set(61)
                        }
                    }
                }));
                let attempt = pins.edit(
                    pins.child.parameter(api).unwrap(),
                    true,
                    api,
                    &|| Ok(()),
                    &|| Ok(()),
                );
                assert_eq!(
                    attempt.result.as_ref().unwrap_err().outcome,
                    if stage == 0 {
                        parameters::Outcome::NotApplied
                    } else {
                        parameters::Outcome::Unknown
                    }
                );
                assert_eq!(
                    attempt.lease.phase(),
                    [
                        parameters::NumericEditPhase::Prepared,
                        parameters::NumericEditPhase::BeginDispatched,
                        parameters::NumericEditPhase::ValueDispatched
                    ][stage]
                );
                assert_eq!(
                    &*notes.member.value.calls.borrow(),
                    [&[][..], &["begin"][..], &["begin", "value"][..]][stage]
                );
            });
        }
    }
}

#[test]
fn final_borrowed_note_check_detects_selection_and_child_drift_without_qi_or_writes() {
    for scenario in 0..6 {
        with_notes(60, 0, |api, notes| {
            let pins = notes.pins(api, 60, 0).unwrap();
            let attempt = pins.edit(
                pins.child.parameter(api).unwrap(),
                true,
                api,
                &|| Ok(()),
                &|| Ok(()),
            );
            assert!(attempt.result.as_ref().unwrap().verified);
            pins.child.retain_publication().unwrap();
            let refs = notes.member.value.refs.get();
            let root_refs = notes.root.base.refs.get();
            match scenario {
                0 => notes.mode.value.current.set(0),
                1 => notes.pitch.value.current.set(61),
                2 => notes.mode.value.default.set(1),
                3 => notes.root.child.set(ptr::null_mut()),
                4 => notes.child.parent.set(ptr::null_mut()),
                5 => notes.child.parameters[0].set(notes.assigned.raw()),
                _ => unreachable!(),
            }
            QUERY_CALLS.with(|calls| calls.set(0));
            assert!(pins.check_borrowed(&|| Ok(())).is_err());
            QUERY_CALLS.with(|calls| assert_eq!(calls.get(), 0));
            assert_eq!(notes.member.value.refs.get(), refs);
            assert_eq!(notes.root.base.refs.get(), root_refs);
            assert_eq!(
                &*notes.member.value.calls.borrow(),
                &["begin", "value", "end"]
            );
        });
    }
}

#[test]
fn root_controller_parameter_and_native_errors_at_each_boundary_stop_dispatch() {
    for stage in 0..4 {
        for scenario in 0..12 {
            with_notes(60, 0, |api, notes| {
                let pins = notes.pins(api, 60, 0).unwrap();
                let root = notes.root.raw() as usize;
                let child = notes.child.raw() as usize;
                let member = notes.member.raw() as usize;
                let assigned = notes.assigned.raw() as usize;
                let callback: Hook = Rc::new(move || unsafe {
                    let root = root as *mut c_void;
                    let child = child as *mut c_void;
                    let member = parameter(member as *mut c_void);
                    let assigned = assigned as *mut c_void;
                    match scenario {
                        0 => node(root).count.set(127),
                        1 => node(root).node_interface.set(child),
                        2 => node(child).support_controller.set(false),
                        3 => node(child).controller_interface.set(root),
                        4 => node(child).parameters[0].set(assigned),
                        5 => member.controller.set(ptr::null_mut()),
                        6 => member.original.set(assigned),
                        7 => member.canonical.set(assigned),
                        8 => member.readonly.set(true),
                        9 => node(root).query_status.set(FAILED),
                        10 => node(child).query_status.set(FAILED),
                        11 => member.default.set(1),
                        _ => unreachable!(),
                    }
                });
                if stage == 0 {
                    callback();
                } else {
                    *[
                        &notes.member.value.on_begin,
                        &notes.member.value.on_value,
                        &notes.member.value.on_end,
                    ][stage - 1]
                        .borrow_mut() = Some(callback);
                }
                let attempt =
                    pins.edit(notes.member.parameter(api), true, api, &|| Ok(()), &|| {
                        Ok(())
                    });
                assert_eq!(
                    attempt.result.as_ref().unwrap_err().outcome,
                    if stage == 0 {
                        parameters::Outcome::NotApplied
                    } else {
                        parameters::Outcome::Unknown
                    },
                    "stage={stage} scenario={scenario}",
                );
                assert_eq!(
                    attempt.lease.phase(),
                    [
                        parameters::NumericEditPhase::Prepared,
                        parameters::NumericEditPhase::BeginDispatched,
                        parameters::NumericEditPhase::ValueDispatched,
                        parameters::NumericEditPhase::EndDispatched,
                    ][stage],
                );
                assert_eq!(
                    &*notes.member.value.calls.borrow(),
                    [
                        &[][..],
                        &["begin"][..],
                        &["begin", "value"][..],
                        &["begin", "value", "end"][..]
                    ][stage],
                );
            });
        }
    }
}

#[test]
fn retained_outer_observer_guard_failure_at_each_boundary_invalidates_the_attempt() {
    for stage in 0..4 {
        for borrowed_failure in [false, true] {
            with_notes(60, 0, |api, notes| {
                let pins = notes.pins(api, 60, 0).unwrap();
                let observer_matches = Rc::new(Cell::new(true));
                let state = observer_matches.clone();
                let callback: Hook = Rc::new(move || state.set(false));
                if stage == 0 {
                    callback();
                } else {
                    *[
                        &notes.member.value.on_begin,
                        &notes.member.value.on_value,
                        &notes.member.value.on_end,
                    ][stage - 1]
                        .borrow_mut() = Some(callback);
                }
                let check = || {
                    if observer_matches.get() {
                        Ok(())
                    } else {
                        Err("retained outer observer does not match".into())
                    }
                };
                let attempt = pins.edit(
                    notes.member.parameter(api),
                    true,
                    api,
                    &|| if borrowed_failure { Ok(()) } else { check() },
                    &|| if borrowed_failure { check() } else { Ok(()) },
                );
                assert_eq!(
                    attempt.result.as_ref().unwrap_err().outcome,
                    if stage == 0 {
                        parameters::Outcome::NotApplied
                    } else {
                        parameters::Outcome::Unknown
                    },
                );
                assert_eq!(
                    &*notes.member.value.calls.borrow(),
                    [
                        &[][..],
                        &["begin"][..],
                        &["begin", "value"][..],
                        &["begin", "value", "end"][..]
                    ][stage],
                );
            });
        }
    }
}

#[test]
fn unchanged_membership_and_fresh_own_original_restoration_keep_global_controls_unedited() {
    with_notes(60, 0, |api, notes| {
        {
            let pins = notes.pins(api, 60, 0).unwrap();
            let unchanged = pins.edit(
                pins.child.parameter(api).unwrap(),
                false,
                api,
                &|| Ok(()),
                &|| Ok(()),
            );
            let read = unchanged.result.as_ref().unwrap();
            assert!(read.verified);
            assert_eq!(read.before.raw, read.after.raw);
        }
        {
            let pins = notes.pins(api, 60, 0).unwrap();
            let assigned = notes.assigned.raw() as usize;
            *notes.member.value.on_value.borrow_mut() = Some(Rc::new(move || unsafe {
                parameter(assigned as *mut c_void).current.set(2);
            }));
            let changed = pins.edit(
                pins.child.parameter(api).unwrap(),
                true,
                api,
                &|| Ok(()),
                &|| Ok(()),
            );
            assert!(changed.result.as_ref().unwrap().verified);
            assert_eq!(notes.member.value.current.get(), 1);
            assert_eq!(notes.assigned.value.current.get(), 2);
            assert!(notes.assigned.value.calls.borrow().is_empty());
        }
        {
            let pins = notes.pins(api, 60, 0).unwrap();
            let restored = pins.edit(
                pins.child.parameter(api).unwrap(),
                false,
                api,
                &|| Ok(()),
                &|| Ok(()),
            );
            assert!(restored.result.as_ref().unwrap().verified);
            assert_eq!(notes.member.value.current.get(), 0);
        }
        assert_eq!(notes.mode.value.current.get(), 1);
        assert_eq!(notes.pitch.value.current.get(), 60);
        assert!(notes.mode.value.calls.borrow().is_empty());
        assert!(notes.pitch.value.calls.borrow().is_empty());
        assert_eq!(
            &*notes.member.value.calls.borrow(),
            &[
                "begin", "value", "end", "begin", "value", "end", "begin", "value", "end"
            ]
        );
    });
}

#[test]
fn independent_last_scalar_qi_reassociation_must_block_begin() {
    let mut boundary = 0;
    with_notes(60, 0, |api, notes| {
        let pins = notes.pins(api, 60, 0).unwrap();
        let reads = Rc::new(Cell::new(0));
        let seen = reads.clone();
        let member = notes.member.raw() as usize;
        *notes.member.value.on_begin.borrow_mut() = Some(Rc::new(move || {
            seen.set(unsafe { parameter(member as *mut c_void) }.read_count.get());
        }));
        notes.member.value.read_count.set(0);
        let attempt = pins.edit(notes.member.parameter(api), true, api, &|| Ok(()), &|| {
            Ok(())
        });
        assert!(attempt.result.as_ref().unwrap().verified);
        boundary = reads.get();
    });
    assert!(boundary > 0);
    for scenario in 0..2 {
        with_notes(60, 0, |api, notes| {
            let pins = notes.pins(api, 60, 0).unwrap();
            let member = notes.member.raw() as usize;
            let child = notes.child.raw() as usize;
            let root = notes.root.raw() as usize;
            notes.member.value.read_count.set(0);
            *notes.member.value.on_read.borrow_mut() = Some(Rc::new(move || unsafe {
                if parameter(member as *mut c_void).read_count.get() == boundary {
                    if scenario == 0 {
                        node(child as *mut c_void)
                            .controller_interface
                            .set(root as *mut c_void);
                    } else {
                        node(root as *mut c_void)
                            .node_interface
                            .set(child as *mut c_void);
                    }
                }
            }));
            let attempt = pins.edit(notes.member.parameter(api), true, api, &|| Ok(()), &|| {
                Ok(())
            });
            eprintln!(
                "independent scenario={scenario} outcome={:?} phase={:?} calls={:?}",
                attempt.result.as_ref().map(|_| ()).map_err(|e| e.outcome),
                attempt.lease.phase(),
                notes.member.value.calls.borrow()
            );
            assert!(
                notes.member.value.calls.borrow().is_empty(),
                "changed QI association was present before begin dispatch"
            );
            assert_eq!(
                attempt.result.as_ref().unwrap_err().outcome,
                parameters::Outcome::NotApplied
            );
        });
    }
}

#[test]
fn last_scalar_persistent_qi_change_blocks_each_following_lifecycle() {
    let mut boundaries = [0; 3];
    with_notes(60, 0, |api, notes| {
        let pins = notes.pins(api, 60, 0).unwrap();
        let seen: Rc<[Cell<usize>; 3]> = Rc::new(std::array::from_fn(|_| Cell::new(0)));
        let member = notes.member.raw() as usize;
        for (index, field) in [
            &notes.member.value.on_begin,
            &notes.member.value.on_value,
            &notes.member.value.on_end,
        ]
        .into_iter()
        .enumerate()
        {
            let seen = seen.clone();
            *field.borrow_mut() = Some(Rc::new(move || {
                seen[index].set(unsafe { parameter(member as *mut c_void) }.read_count.get());
            }));
        }
        notes.member.value.read_count.set(0);
        let attempt = pins.edit(notes.member.parameter(api), true, api, &|| Ok(()), &|| {
            Ok(())
        });
        assert!(attempt.result.as_ref().unwrap().verified);
        boundaries = std::array::from_fn(|index| seen[index].get());
    });
    for (stage, boundary) in boundaries.into_iter().enumerate() {
        for reassociate_child in [false, true] {
            with_notes(60, 0, |api, notes| {
                let pins = notes.pins(api, 60, 0).unwrap();
                let member = notes.member.raw() as usize;
                let child = notes.child.raw() as usize;
                let root = notes.root.raw() as usize;
                notes.member.value.read_count.set(0);
                *notes.member.value.on_read.borrow_mut() = Some(Rc::new(move || unsafe {
                    if parameter(member as *mut c_void).read_count.get() == boundary {
                        if reassociate_child {
                            node(child as *mut c_void)
                                .controller_interface
                                .set(root as *mut c_void);
                        } else {
                            node(root as *mut c_void)
                                .node_interface
                                .set(child as *mut c_void);
                        }
                    }
                }));
                let attempt =
                    pins.edit(notes.member.parameter(api), true, api, &|| Ok(()), &|| {
                        Ok(())
                    });
                assert_eq!(
                    attempt.result.as_ref().unwrap_err().outcome,
                    if stage == 0 {
                        parameters::Outcome::NotApplied
                    } else {
                        parameters::Outcome::Unknown
                    }
                );
                assert_eq!(
                    attempt.lease.phase(),
                    [
                        parameters::NumericEditPhase::Prepared,
                        parameters::NumericEditPhase::BeginDispatched,
                        parameters::NumericEditPhase::ValueDispatched
                    ][stage]
                );
                assert_eq!(
                    &*notes.member.value.calls.borrow(),
                    [&[][..], &["begin"][..], &["begin", "value"][..]][stage]
                );
            });
        }
    }
}

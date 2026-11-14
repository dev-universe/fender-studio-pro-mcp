use super::*;
use std::ffi::{c_char, c_void};

#[repr(C)]
struct Param {
    table: *const *const c_void,
    refs: Cell<u32>,
    owner: Cell<*mut c_void>,
    object: Cell<*mut c_void>,
}
unsafe extern "C" fn p_retain(p: *mut c_void) -> u32 {
    let p = unsafe { &*p.cast::<Param>() };
    p.refs.set(p.refs.get() + 1);
    p.refs.get()
}
unsafe extern "C" fn p_release(p: *mut c_void) -> u32 {
    let p = unsafe { &*p.cast::<Param>() };
    p.refs.set(p.refs.get() - 1);
    p.refs.get()
}
unsafe extern "C" fn p_owner(p: *mut c_void) -> *mut c_void {
    unsafe { &*p.cast::<Param>() }.owner.get()
}
unsafe extern "C" fn unavailable(p: *mut c_void, iid: *const Uid, out: *mut *mut c_void) -> i32 {
    let object = unsafe { &*p.cast::<Param>() }.object.get();
    if unsafe { *iid } == UNKNOWN {
        unsafe {
            *out = p;
            p_retain(p);
        }
        OK
    } else if unsafe { *iid } == OBJECT && !object.is_null() {
        unsafe {
            *out = object;
            object_retain(object);
        }
        OK
    } else {
        unsafe { *out = ptr::null_mut() };
        NO_INTERFACE
    }
}
unsafe extern "C" fn original(p: *mut c_void) -> *mut c_void {
    p
}
unsafe extern "C" fn kind(_: *mut c_void) -> i32 {
    11
}
#[repr(C)]
struct Name {
    text: *const c_char,
    owner: *mut c_void,
}
unsafe extern "C" fn name(_: *mut c_void) -> *const Name {
    static NAME: Name = Name {
        text: c"nativeName".as_ptr(),
        owner: ptr::null_mut(),
    };
    &NAME
}
unsafe impl Sync for Name {}
unsafe extern "C" fn zero(_: *mut c_void) -> i32 {
    0
}
unsafe extern "C" fn flag(_: *mut c_void) -> u8 {
    1
}
thread_local! { static STRING: Cell<*mut c_void> = const { Cell::new(ptr::null_mut()) }; }
unsafe extern "C" fn empty() -> *mut c_void {
    STRING.with(Cell::get)
}
unsafe extern "C" fn constant(_: *const c_char) -> *const c_void {
    static ID: Name = Name {
        text: c"value".as_ptr(),
        owner: ptr::null_mut(),
    };
    &ID as *const Name as *const c_void
}
fn text() -> parameters::TextApi {
    parameters::TextApi {
        get_empty_string: empty,
        get_constant_cstring: constant,
    }
}
struct ParameterFixture {
    p: Box<Param>,
    _table: Box<[*const c_void; 46]>,
}
impl ParameterFixture {
    fn new(owner: *mut c_void) -> Self {
        let mut table = Box::new([ptr::null(); 46]);
        table[0] = unavailable as *const c_void;
        table[1] = p_retain as *const c_void;
        table[2] = p_release as *const c_void;
        table[3] = kind as *const c_void;
        table[4] = name as *const c_void;
        table[6] = flag as *const c_void;
        table[8] = zero as *const c_void;
        table[13] = zero as *const c_void;
        table[14] = p_owner as *const c_void;
        table[18] = original as *const c_void;
        Self {
            p: Box::new(Param {
                table: table.as_ptr(),
                refs: Cell::new(1),
                owner: Cell::new(owner),
                object: Cell::new(ptr::null_mut()),
            }),
            _table: table,
        }
    }
    fn pointer(&self) -> *mut c_void {
        ptr::from_ref(&*self.p).cast_mut().cast()
    }
}
impl Drop for ParameterFixture {
    fn drop(&mut self) {
        assert_eq!(self.p.refs.get(), 1);
    }
}
fn bound(f: &Fixture) -> Observation {
    let mut observation = f.pair();
    let pair = observation.pair.as_mut().unwrap();
    for i in 0..2 {
        f.objects[i].count.set([8, 25][i]);
        pair.capabilities[i] = capability(f.pointer(i), f.pointer(i), &mut || f.guard()).unwrap();
    }
    observation
}
#[test]
fn count_drift_is_explicit_incomplete_and_zero_stays_present() {
    for count in [0, 7, 9, 26] {
        let f = Fixture::new();
        let mut observation = bound(&f);
        f.objects[0].count.set(count);
        f.objects[1].count.set(0);
        let pair = observation.pair.as_mut().unwrap();
        for i in 0..2 {
            pair.capabilities[i] =
                capability(f.pointer(i), f.pointer(i), &mut || f.guard()).unwrap();
        }
        observation.describe(text(), &mut || f.guard()).unwrap();
        let value = observation.descriptions_value().unwrap();
        assert_eq!(value["complete"], false);
        assert_eq!(value["nodes"][0]["observed_count"], count);
        assert!(value["nodes"][0]["parameters"].is_null());
        assert_eq!(value["mutation_capability"], false);
        assert_eq!(
            observation.value()["nodes"][1]["controller"]["status"],
            "present"
        );
    }
}
#[test]
fn null_parameter_and_failed_typed_description_fail_with_balanced_owners() {
    for has_parameter in [false, true] {
        let f = Fixture::new();
        let p = ParameterFixture::new(f.pointer(0));
        let mut observation = bound(&f);
        if has_parameter {
            f.objects[0].parameter.set(p.pointer());
        }
        assert!(observation.describe(text(), &mut || f.guard()).is_err());
        drop(observation);
        assert_eq!(p.p.refs.get(), 1);
    }
}
#[test]
fn parameter_association_count_owner_and_context_changes_fail_closed() {
    for change in 0..4 {
        let f = Fixture::new();
        let p = ParameterFixture::new(f.pointer(0));
        let observation = bound(&f);
        let pair = observation.pair.as_ref().unwrap();
        f.objects[0].parameter.set(p.pointer());
        let held = pin(p.pointer(), &mut || f.guard()).unwrap();
        let raw = unsafe { RawParameter::from_owned(NonNull::new(held.0).unwrap(), text()) };
        std::mem::forget(held);
        let owner = raw.controller_owner(&mut || f.guard()).unwrap();
        let parameter = super::super::descriptors::Parameter {
            parameter: raw,
            owner,
            owner_view: f.pointer(0),
            node: 0,
            index: 0,
        };
        parameter.check(pair, &mut || f.guard()).unwrap();
        match change {
            0 => f.objects[0].parameter.set(ptr::null_mut()),
            1 => f.objects[0].count.set(7),
            2 => p.p.owner.set(f.pointer(1)),
            _ => f.objects[0].lost.set(true),
        }
        assert!(parameter.check(pair, &mut || f.guard()).is_err());
        drop(parameter);
        drop(observation);
    }
}

#[test]
fn mode_setting_retains_named_parameter_and_owner_before_after_and_at_publication() {
    use super::super::settings::{ModeProfile, check_counts, check_named, dispatch_mode_change};
    for phase in 0..3 {
        for owner_changed in [false, true] {
            let f = Fixture::new();
            let p = ParameterFixture::new(f.pointer(1));
            let observation = settings_observation(&f, 25);
            f.objects[1].parameter.set(p.pointer());
            let held = pin(p.pointer(), &mut || f.guard()).unwrap();
            let raw = unsafe { RawParameter::from_owned(NonNull::new(held.0).unwrap(), text()) };
            std::mem::forget(held);
            let binding = raw.pin_binding_read(&mut || f.guard()).unwrap();
            let owner = binding
                .parameter()
                .controller_owner(&mut || f.guard())
                .unwrap()
                .unwrap();
            let profile = Cell::new(Some(ModeProfile {
                mode: 0,
                child_count: 25,
            }));
            let writes = Cell::new(0);
            let drift = || {
                if owner_changed {
                    p.p.owner.set(f.pointer(0));
                } else {
                    f.objects[1].parameter.set(ptr::null_mut());
                }
            };
            let mut check = || {
                observation.check(&mut || f.guard())?;
                check_counts(
                    observation.pair.as_ref().unwrap(),
                    profile.get().unwrap(),
                    &mut || f.guard(),
                )?;
                binding
                    .check(&mut || f.guard())
                    .map_err(|e| e.to_string())?;
                owner.check(&mut || f.guard()).map_err(|e| e.to_string())?;
                check_named(f.pointer(1), p.pointer(), f.pointer(1), "mode")
            };
            if phase == 0 {
                drift();
            }
            let result = dispatch_mode_change(
                &profile,
                ModeProfile {
                    mode: 4,
                    child_count: 23,
                },
                &mut check,
                || {
                    writes.set(writes.get() + 1);
                    f.objects[1].count.set(23);
                    if phase == 1 {
                        drift();
                    }
                },
            );
            if phase == 2 {
                result.unwrap();
                drift();
                assert!(check().is_err());
            } else {
                assert!(result.is_err());
            }
            assert_eq!(writes.get(), usize::from(phase != 0));
            drop(owner);
            drop(binding);
            drop(observation);
        }
    }
}

#[repr(C)]
struct ReadObject {
    table: *const *const c_void,
    refs: Cell<u32>,
}
unsafe extern "C" fn object_retain(p: *mut c_void) -> u32 {
    let o = unsafe { &*p.cast::<ReadObject>() };
    o.refs.set(o.refs.get() + 1);
    o.refs.get()
}
unsafe extern "C" fn object_release(p: *mut c_void) -> u32 {
    let o = unsafe { &*p.cast::<ReadObject>() };
    o.refs.set(o.refs.get() - 1);
    o.refs.get()
}
unsafe extern "C" fn no_property(_: *mut c_void, _: *mut c_void, _: *const c_void) -> u8 {
    0
}
unsafe extern "C" fn length(_: *mut c_void) -> i32 {
    0
}
unsafe extern "C" fn copy(_: *mut c_void, out: *mut u16, _: i32) -> i32 {
    unsafe { *out = 0 };
    OK
}
unsafe extern "C" fn private(_: *mut c_void) -> *mut c_void {
    ptr::null_mut()
}
unsafe extern "C" fn display(_: *mut c_void, _: *mut c_void) {}
unsafe extern "C" fn readonly(_: *mut c_void, _: i32) -> u8 {
    1
}
#[test]
fn read_only_unavailable_current_data_is_retained_and_incomplete_without_setters() {
    let f = Fixture::new();
    let mut p = ParameterFixture::new(f.pointer(0));
    let mut object_table = Box::new([ptr::null(); 5]);
    object_table[1] = object_retain as *const c_void;
    object_table[2] = object_release as *const c_void;
    object_table[4] = no_property as *const c_void;
    let object = Box::new(ReadObject {
        table: object_table.as_ptr(),
        refs: Cell::new(1),
    });
    let mut string_table = Box::new([ptr::null(); 22]);
    string_table[1] = object_retain as *const c_void;
    string_table[2] = object_release as *const c_void;
    string_table[4] = length as *const c_void;
    string_table[8] = copy as *const c_void;
    string_table[21] = private as *const c_void;
    let string = Box::new(ReadObject {
        table: string_table.as_ptr(),
        refs: Cell::new(1),
    });
    STRING.with(|s| s.set(ptr::from_ref(&*string).cast_mut().cast()));
    p.p.object.set(ptr::from_ref(&*object).cast_mut().cast());
    p._table[8] = readonly as *const c_void;
    p._table[44] = display as *const c_void;
    let mut observation = bound(&f);
    f.objects[0].parameter.set(p.pointer());
    f.objects[1].parameter.set(p.pointer());
    observation.describe(text(), &mut || f.guard()).unwrap();
    let value = observation.descriptions_value().unwrap();
    assert_eq!(value["status"], "incomplete");
    assert_eq!(value["nodes"][0]["parameters"].as_array().unwrap().len(), 8);
    assert_eq!(
        value["nodes"][1]["parameters"].as_array().unwrap().len(),
        25
    );
    for node in value["nodes"].as_array().unwrap() {
        for record in node["parameters"].as_array().unwrap() {
            assert_eq!(record["status"], "current_unavailable");
            assert_eq!(record["description"]["native_kind"], "Unknown(11)");
            assert_eq!(record["description"]["name"], "nativeName");
            assert_eq!(record["description"]["readOnly"], true);
            assert_eq!(record["description"]["writable"], false);
            assert_eq!(record["can_set_via_this_api"], false);
            assert_eq!(record["mutation_capability"], false);
        }
    }
    assert_eq!(
        value["nodes"][0]["parameters"][0]["controller_owner_canonical_matches_receiver"],
        true
    );
    assert_eq!(
        value["nodes"][1]["parameters"][0]["controller_owner_canonical_matches_receiver"],
        false
    );
    let seen = Cell::new(0);
    observation
        .check(&mut || {
            seen.set(seen.get() + 1);
            f.guard()
        })
        .unwrap();
    let final_guard = seen.get();
    seen.set(0);
    assert!(
        observation
            .check(&mut || {
                seen.set(seen.get() + 1);
                if seen.get() == final_guard {
                    f.objects[0].parameter.set(ptr::null_mut());
                }
                f.guard()
            })
            .is_err()
    );
    drop(observation);
    assert_eq!(p.p.refs.get(), 1);
    assert_eq!(object.refs.get(), 1);
    assert_eq!(string.refs.get(), 1);
    object_table[4] = numeric_property as *const c_void;
    p._table[3] = integer_kind as *const c_void;
    p._table[8] = readwrite as *const c_void;
    p._table[35] = normalized as *const c_void;
    p._table[41] = private as *const c_void;
    f.objects[0].parameter.set(p.pointer());
    let mut observation = bound(&f);
    observation.describe(text(), &mut || f.guard()).unwrap();
    let value = observation.descriptions_value().unwrap();
    assert_eq!(value["complete"], true);
    let held = pin(p.pointer(), &mut || f.guard()).unwrap();
    let raw = unsafe { RawParameter::from_owned(NonNull::new(held.0).unwrap(), text()) };
    std::mem::forget(held);
    let expected = super::super::super::description(&raw.describe().unwrap());
    assert_eq!(expected["readOnly"], false);
    assert_eq!(expected["writable"], true);
    for node in value["nodes"].as_array().unwrap() {
        for record in node["parameters"].as_array().unwrap() {
            assert_eq!(record["description"], expected);
            assert_eq!(record["can_set_via_this_api"], false);
            assert_eq!(record["mutation_capability"], false);
        }
    }
    drop(raw);
    drop(observation);
    assert_eq!(p.p.refs.get(), 1);
    assert_eq!(object.refs.get(), 1);
    assert_eq!(string.refs.get(), 1);
    STRING.with(|s| s.set(ptr::null_mut()));
}

#[repr(C)]
struct NumericVariant {
    tag: i16,
    padding: [u8; 6],
    payload: u64,
}
unsafe extern "C" fn numeric_property(
    _: *mut c_void,
    out: *mut NumericVariant,
    _: *const c_void,
) -> u8 {
    unsafe {
        *out = NumericVariant {
            tag: 1,
            padding: [0; 6],
            payload: 0,
        };
    }
    1
}
unsafe extern "C" fn integer_kind(_: *mut c_void) -> i32 {
    1
}
unsafe extern "C" fn readwrite(_: *mut c_void, _: i32) -> u8 {
    0
}
unsafe extern "C" fn normalized(_: *mut c_void) -> f32 {
    0.0
}

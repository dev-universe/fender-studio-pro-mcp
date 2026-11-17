use super::*;
use std::{
    cell::{Cell, RefCell},
    ffi::CStr,
    rc::Rc,
    sync::OnceLock,
};

#[derive(Clone, Copy)]
struct Numeric {
    tag: i16,
    payload: u64,
}
impl Numeric {
    fn integer(value: i64) -> Self {
        Self {
            tag: 1,
            payload: value as u64,
        }
    }
    fn float(value: f64) -> Self {
        Self {
            tag: 2,
            payload: value.to_bits(),
        }
    }
    fn number(self) -> Number {
        if self.tag == 1 {
            Number::Integer(self.payload as i64)
        } else {
            Number::Float(f64::from_bits(self.payload))
        }
    }
}
#[repr(C)]
struct Object {
    table: *const usize,
    parameter: *mut Fake,
}
#[repr(C)]
struct Owner {
    table: *const usize,
    refs: Cell<u32>,
    context: Rc<Cell<bool>>,
    writes: Rc<RefCell<Vec<(Numeric, bool)>>>,
    stop_on_release: Cell<bool>,
}
#[repr(C)]
struct Fake {
    table: *const usize,
    refs: Cell<u32>,
    object: Object,
    kind: Cell<i32>,
    tag: Cell<i32>,
    enabled: Cell<bool>,
    read_only: Cell<bool>,
    name: Cell<*const ffi::CStringRepr>,
    value: Cell<Numeric>,
    min: Cell<Numeric>,
    max: Cell<Numeric>,
    controller: Cell<*mut c_void>,
    context: Rc<Cell<bool>>,
    writes: Rc<RefCell<Vec<(Numeric, bool)>>>,
    auto_reset: Cell<bool>,
    change_after_upper: Cell<u8>,
    fail_value_after_reset: Cell<bool>,
    ignore_reset: Cell<bool>,
    calls: Cell<usize>,
    stop_on_parameter_release: Cell<bool>,
    stop_on_release_after_reset: Cell<bool>,
}
const NAME: ffi::CStringRepr = ffi::CStringRepr {
    text: c"swapfreqs".as_ptr(),
    owner: std::ptr::null_mut(),
};
const OTHER: ffi::CStringRepr = ffi::CStringRepr {
    text: c"changed".as_ptr(),
    owner: std::ptr::null_mut(),
};
const VALUE: ffi::CStringRepr = ffi::CStringRepr {
    text: c"value".as_ptr(),
    owner: std::ptr::null_mut(),
};
const MIN: ffi::CStringRepr = ffi::CStringRepr {
    text: c"min".as_ptr(),
    owner: std::ptr::null_mut(),
};
const MAX: ffi::CStringRepr = ffi::CStringRepr {
    text: c"max".as_ptr(),
    owner: std::ptr::null_mut(),
};
unsafe fn fake<'a>(p: *mut c_void) -> &'a Fake {
    unsafe { &*p.cast::<Fake>() }
}
unsafe fn object<'a>(p: *mut c_void) -> &'a Fake {
    unsafe { &*(*p.cast::<Object>()).parameter }
}
unsafe extern "C" fn retain(p: *mut c_void) -> u32 {
    let p = unsafe { fake(p) };
    p.refs.set(p.refs.get() + 1);
    p.refs.get()
}
unsafe extern "C" fn release_mock(p: *mut c_void) -> u32 {
    let p = unsafe { fake(p) };
    if p.stop_on_parameter_release.get() && !p.writes.borrow().is_empty()
        || p.stop_on_release_after_reset.get() && p.writes.borrow().len() == 2
    {
        p.context.set(false);
    }
    p.refs.set(p.refs.get() - 1);
    p.refs.get()
}
unsafe extern "C" fn object_retain(p: *mut c_void) -> u32 {
    unsafe { retain((*p.cast::<Object>()).parameter.cast()) }
}
unsafe extern "C" fn object_release(p: *mut c_void) -> u32 {
    unsafe { release_mock((*p.cast::<Object>()).parameter.cast()) }
}
unsafe extern "C" fn owner_retain(p: *mut c_void) -> u32 {
    let p = unsafe { &*p.cast::<Owner>() };
    p.refs.set(p.refs.get() + 1);
    p.refs.get()
}
unsafe extern "C" fn owner_release(p: *mut c_void) -> u32 {
    let p = unsafe { &*p.cast::<Owner>() };
    if p.stop_on_release.get() && !p.writes.borrow().is_empty() {
        p.context.set(false);
    }
    p.refs.set(p.refs.get() - 1);
    p.refs.get()
}
unsafe extern "C" fn owner_query(
    p: *mut c_void,
    iid: *const ffi::Uid,
    out: *mut *mut c_void,
) -> i32 {
    unsafe {
        *out = std::ptr::null_mut();
    }
    if unsafe { *iid } != ffi::UNKNOWN {
        return ffi::NO_INTERFACE;
    }
    unsafe {
        owner_retain(p);
        *out = p;
    }
    ffi::OK
}
unsafe extern "C" fn query(p: *mut c_void, iid: *const ffi::Uid, out: *mut *mut c_void) -> i32 {
    let f = unsafe { fake(p) };
    f.calls.set(f.calls.get() + 1);
    unsafe {
        *out = std::ptr::null_mut();
    }
    if unsafe { *iid } != ffi::OBJECT {
        return ffi::NO_INTERFACE;
    }
    unsafe {
        retain(p);
        *out = std::ptr::addr_of!(f.object).cast_mut().cast();
    }
    ffi::OK
}
unsafe extern "C" fn kind(p: *mut c_void) -> i32 {
    let p = unsafe { fake(p) };
    p.calls.set(p.calls.get() + 1);
    p.kind.get()
}
unsafe extern "C" fn name(p: *mut c_void) -> *const ffi::CStringRepr {
    unsafe { fake(p) }.name.get()
}
unsafe extern "C" fn tag(p: *mut c_void) -> i32 {
    unsafe { fake(p) }.tag.get()
}
unsafe extern "C" fn enabled(p: *mut c_void) -> u8 {
    u8::from(unsafe { fake(p) }.enabled.get())
}
unsafe extern "C" fn state(p: *mut c_void, mask: i32) -> u8 {
    assert_eq!(mask, 0x1000);
    u8::from(unsafe { fake(p) }.read_only.get())
}
unsafe extern "C" fn controller(p: *mut c_void) -> *mut c_void {
    unsafe { fake(p) }.controller.get()
}
unsafe extern "C" fn property(p: *mut c_void, out: *mut Variant, id: *const c_void) -> u8 {
    let f = unsafe { object(p) };
    let id = unsafe { CStr::from_ptr((*id.cast::<ffi::CStringRepr>()).text) };
    let value = if id == c"value" {
        if f.fail_value_after_reset.get() && f.writes.borrow().len() == 2 {
            return 0;
        }
        f.value.get()
    } else if id == c"min" {
        f.min.get()
    } else if id == c"max" {
        f.max.get()
    } else {
        panic!("unexpected pulse property")
    };
    if value.tag == 0x14 && value.payload != 0 {
        unsafe {
            owner_retain(value.payload as usize as *mut c_void);
        }
    }
    unsafe {
        (*out).tag = value.tag;
        (*out).payload = value.payload;
    }
    1
}
unsafe extern "C" fn set(p: *mut c_void, value: *const Variant, notify: u8) {
    let f = unsafe { fake(p) };
    let value = Numeric {
        tag: unsafe { (*value).tag },
        payload: unsafe { (*value).payload },
    };
    let upper = f.writes.borrow().is_empty();
    f.writes.borrow_mut().push((value, notify != 0));
    if !f.ignore_reset.get() || upper {
        f.value.set(value);
    }
    if upper {
        if f.auto_reset.get() {
            f.value.set(f.min.get());
        }
        match f.change_after_upper.get() {
            1 => f.context.set(false),
            2 => f.tag.set(99),
            3 => f.min.set(Numeric::integer(-1)),
            4 => f.max.set(Numeric::integer(2)),
            5 => f.name.set(&OTHER),
            6 => f.enabled.set(false),
            7 => f.read_only.set(true),
            8 => f.controller.set(std::ptr::null_mut()),
            9 => f.value.set(Numeric {
                tag: 0x14,
                payload: f.controller.get() as usize as u64,
            }),
            _ => (),
        }
    }
}
unsafe extern "C" fn empty() -> *mut c_void {
    panic!("pulse must not create formatter strings")
}
unsafe extern "C" fn text(name: *const std::ffi::c_char) -> *const c_void {
    match unsafe { CStr::from_ptr(name) } {
        name if name == c"value" => (&VALUE as *const ffi::CStringRepr).cast(),
        name if name == c"min" => (&MIN as *const ffi::CStringRepr).cast(),
        name if name == c"max" => (&MAX as *const ffi::CStringRepr).cast(),
        _ => panic!("unexpected pulse property lookup"),
    }
}
struct Fixture {
    parameter: Box<Fake>,
    owner: Box<Owner>,
}
impl Fixture {
    fn new(with_owner: bool) -> Self {
        static TABLE: OnceLock<[usize; 45]> = OnceLock::new();
        static OBJECT: OnceLock<[usize; 8]> = OnceLock::new();
        static OWNER: OnceLock<[usize; 3]> = OnceLock::new();
        let table = TABLE.get_or_init(|| {
            let mut t = [0; 45];
            t[0] = query as *const () as usize;
            t[1] = retain as *const () as usize;
            t[2] = release_mock as *const () as usize;
            t[3] = kind as *const () as usize;
            t[4] = name as *const () as usize;
            t[6] = enabled as *const () as usize;
            t[8] = state as *const () as usize;
            t[13] = tag as *const () as usize;
            t[14] = controller as *const () as usize;
            t[21] = set as *const () as usize;
            t
        });
        let object = OBJECT.get_or_init(|| {
            let mut t = [0; 8];
            t[1] = object_retain as *const () as usize;
            t[2] = object_release as *const () as usize;
            t[4] = property as *const () as usize;
            t
        });
        let owner_table = OWNER.get_or_init(|| {
            [
                owner_query as *const () as usize,
                owner_retain as *const () as usize,
                owner_release as *const () as usize,
            ]
        });
        let context = Rc::new(Cell::new(true));
        let writes = Rc::new(RefCell::new(Vec::new()));
        let mut owner = Box::new(Owner {
            table: owner_table.as_ptr(),
            refs: Cell::new(1),
            context: context.clone(),
            writes: writes.clone(),
            stop_on_release: Cell::new(false),
        });
        let mut parameter = Box::new(Fake {
            table: table.as_ptr(),
            refs: Cell::new(1),
            object: Object {
                table: object.as_ptr(),
                parameter: std::ptr::null_mut(),
            },
            kind: Cell::new(0),
            tag: Cell::new(12),
            enabled: Cell::new(true),
            read_only: Cell::new(false),
            name: Cell::new(&NAME),
            value: Cell::new(Numeric::integer(0)),
            min: Cell::new(Numeric::integer(0)),
            max: Cell::new(Numeric::integer(1)),
            controller: Cell::new(if with_owner {
                std::ptr::addr_of_mut!(*owner).cast()
            } else {
                std::ptr::null_mut()
            }),
            context,
            writes,
            auto_reset: Cell::new(false),
            change_after_upper: Cell::new(0),
            fail_value_after_reset: Cell::new(false),
            ignore_reset: Cell::new(false),
            calls: Cell::new(0),
            stop_on_parameter_release: Cell::new(false),
            stop_on_release_after_reset: Cell::new(false),
        });
        parameter.object.parameter = std::ptr::addr_of_mut!(*parameter);
        Self { parameter, owner }
    }
    fn raw(&self) -> *mut c_void {
        std::ptr::addr_of!(*self.parameter).cast_mut().cast()
    }
    fn parameter(&self) -> RawParameter {
        unsafe {
            retain(self.raw());
            RawParameter::from_owned(
                NonNull::new(self.raw()).unwrap(),
                TextApi {
                    get_empty_string: empty,
                    get_constant_cstring: text,
                },
            )
        }
    }
    fn guard(&self) -> impl FnMut() -> Result<(), String> {
        let context = self.parameter.context.clone();
        move || {
            if context.get() {
                Ok(())
            } else {
                Err("document/provider changed".into())
            }
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        assert_eq!(self.parameter.refs.get(), 1);
        assert_eq!(self.owner.refs.get(), 1);
    }
}

#[test]
fn pulse_notifies_only_upper_and_observes_reset_without_verifying_effects() {
    for with_owner in [false, true] {
        for float in [false, true] {
            let f = Fixture::new(with_owner);
            if float {
                f.parameter.value.set(Numeric::float(0.));
                f.parameter.min.set(Numeric::float(0.));
                f.parameter.max.set(Numeric::float(1.));
            }
            let p = f.parameter();
            let receipt = p.pulse_range(&mut f.guard()).unwrap();
            let writes = f.parameter.writes.borrow();
            assert_eq!(writes.len(), 2);
            assert!(value::compare(writes[0].0.number(), Number::Integer(1)).is_eq());
            assert!(value::compare(writes[1].0.number(), Number::Integer(0)).is_eq());
            assert_eq!([writes[0].1, writes[1].1], [true, false]);
            assert!(receipt.upper_dispatched && receipt.reset_dispatched && receipt.reset_verified);
            assert!(!receipt.effects_verified);
            assert_eq!(receipt.outcome, PulseOutcome::AppliedUnverified);
        }
    }
}
#[test]
fn synchronous_upper_auto_reset_still_sends_sdk_unnotified_lower() {
    let f = Fixture::new(true);
    f.parameter.auto_reset.set(true);
    let p = f.parameter();
    assert!(p.pulse_range(&mut f.guard()).unwrap().reset_verified);
    assert_eq!(
        f.parameter
            .writes
            .borrow()
            .iter()
            .map(|w| w.1)
            .collect::<Vec<_>>(),
        [true, false]
    );
}
#[test]
fn preconditions_reject_without_any_setter_dispatch() {
    for case in 0..10 {
        let f = Fixture::new(true);
        let p = f.parameter();
        match case {
            0 => f.parameter.context.set(false),
            1 => f.parameter.kind.set(5),
            2 => f.parameter.kind.set(3),
            3 => f.parameter.kind.set(7),
            4 => f.parameter.enabled.set(false),
            5 => f.parameter.read_only.set(true),
            6 => f.parameter.min.set(Numeric::integer(-1)),
            7 => f.parameter.max.set(Numeric::integer(2)),
            8 => f.parameter.value.set(Numeric::float(0.5)),
            _ => f.parameter.value.set(Numeric { tag: 4, payload: 0 }),
        }
        let error = p.pulse_range(&mut f.guard()).unwrap_err();
        assert_eq!(error.outcome, Outcome::NotApplied);
        assert!(!error.upper_dispatched && !error.reset_dispatched && !error.effects_verified);
        assert!(f.parameter.writes.borrow().is_empty());
    }
}
#[test]
fn upper_context_or_metadata_changes_stop_without_a_lower_write() {
    for change in 1..=8 {
        let f = Fixture::new(true);
        f.parameter.change_after_upper.set(change);
        let p = f.parameter();
        let error = p.pulse_range(&mut f.guard()).unwrap_err();
        assert_eq!(error.outcome, Outcome::Unknown);
        assert!(error.upper_dispatched && !error.reset_dispatched && !error.effects_verified);
        assert_eq!(f.parameter.writes.borrow().len(), 1);
    }
}
#[test]
fn a_new_controller_after_a_stable_null_owner_is_not_reset() {
    let f = Fixture::new(false);
    let p = f.parameter();
    let owner = std::ptr::addr_of!(*f.owner).cast_mut().cast();
    let mut guard = || {
        if !f.parameter.writes.borrow().is_empty() {
            f.parameter.controller.set(owner);
        }
        Ok(())
    };
    let error = p.pulse_range(&mut guard).unwrap_err();
    assert_eq!(error.outcome, Outcome::Unknown);
    assert!(!error.reset_dispatched);
    assert_eq!(f.parameter.writes.borrow().len(), 1);
}
#[test]
fn native_owner_release_reentry_after_upper_prevents_the_lower() {
    for with_owner in [false, true] {
        let f = Fixture::new(with_owner);
        f.owner.stop_on_release.set(true);
        f.parameter.stop_on_parameter_release.set(!with_owner);
        let p = f.parameter();
        let error = p.pulse_range(&mut f.guard()).unwrap_err();
        assert_eq!(error.outcome, Outcome::Unknown);
        assert!(!error.reset_dispatched);
        assert_eq!(f.parameter.writes.borrow().len(), 1);
    }
}
#[test]
fn reset_read_failure_or_ignored_reset_remains_unknown() {
    for ignored in [false, true] {
        let f = Fixture::new(true);
        f.parameter.ignore_reset.set(ignored);
        f.parameter.fail_value_after_reset.set(!ignored);
        let p = f.parameter();
        let error = p.pulse_range(&mut f.guard()).unwrap_err();
        assert_eq!(error.outcome, Outcome::Unknown);
        assert!(error.upper_dispatched && error.reset_dispatched && !error.effects_verified);
        assert_eq!(f.parameter.writes.borrow().len(), 2);
    }
}
#[test]
fn final_native_cleanup_cannot_publish_success_after_context_loss() {
    let f = Fixture::new(true);
    f.parameter.stop_on_release_after_reset.set(true);
    let p = f.parameter();
    let error = p.pulse_range(&mut f.guard()).unwrap_err();
    assert_eq!(error.outcome, Outcome::Unknown);
    assert!(error.upper_dispatched && error.reset_dispatched && !error.effects_verified);
    assert_eq!(f.parameter.writes.borrow().len(), 2);
}
#[test]
fn unexpected_owned_property_is_released_before_guarded_failure() {
    let f = Fixture::new(true);
    f.parameter.change_after_upper.set(9);
    f.owner.stop_on_release.set(true);
    let p = f.parameter();
    let error = p.pulse_range(&mut f.guard()).unwrap_err();
    assert_eq!(error.outcome, Outcome::Unknown);
    assert!(!error.reset_dispatched && !error.effects_verified);
    assert_eq!(f.parameter.writes.borrow().len(), 1);
}
#[test]
fn wrong_thread_is_refused_before_any_native_lookup() {
    let f = Fixture::new(true);
    let mut p = f.parameter();
    p.thread = std::thread::spawn(|| std::thread::current().id())
        .join()
        .unwrap();
    let error = p.pulse_range(&mut f.guard()).unwrap_err();
    assert_eq!(error.error, Error::WrongThread);
    assert_eq!(error.outcome, Outcome::NotApplied);
    assert_eq!(f.parameter.calls.get(), 0);
    assert!(f.parameter.writes.borrow().is_empty());
}

use super::*;
use std::{cell::Cell, rc::Rc, sync::OnceLock};

#[repr(C)]
struct Fake {
    table: *const usize,
    refs: Cell<u32>,
    controller: Cell<*mut c_void>,
    canonical: Cell<*mut c_void>,
    gets: Cell<usize>,
    updates: Cell<usize>,
    unknown_queries: Cell<usize>,
    other_queries: Cell<usize>,
    context: Rc<Cell<bool>>,
    stop_on_get: Cell<bool>,
    stop_on_query: Cell<bool>,
    stop_on_release_at: Cell<Option<u32>>,
    change_parameter_on_release: Cell<*mut c_void>,
    replace_with: Cell<*mut c_void>,
    query_error: Cell<Option<i32>>,
}
unsafe fn fake<'a>(pointer: *mut c_void) -> &'a Fake {
    unsafe { &*pointer.cast() }
}
unsafe extern "C" fn retain(pointer: *mut c_void) -> u32 {
    let f = unsafe { fake(pointer) };
    f.refs.set(f.refs.get() + 1);
    f.refs.get()
}
unsafe extern "C" fn release_mock(pointer: *mut c_void) -> u32 {
    let f = unsafe { fake(pointer) };
    if f.stop_on_release_at.get() == Some(f.refs.get()) {
        f.context.set(false);
    }
    let parameter = f.change_parameter_on_release.get();
    if !parameter.is_null() {
        unsafe { fake(parameter) }
            .controller
            .set(f.replace_with.get());
    }
    let count = f.refs.get() - 1;
    f.refs.set(count);
    if count == 0 {
        drop(unsafe { Box::from_raw(pointer.cast::<Fake>()) });
    }
    count
}
unsafe extern "C" fn query(
    pointer: *mut c_void,
    iid: *const ffi::Uid,
    out: *mut *mut c_void,
) -> i32 {
    let f = unsafe { fake(pointer) };
    if unsafe { *iid } != ffi::UNKNOWN {
        f.other_queries.set(f.other_queries.get() + 1);
        return ffi::NO_INTERFACE;
    }
    f.unknown_queries.set(f.unknown_queries.get() + 1);
    if f.stop_on_query.get() {
        f.context.set(false);
    }
    if let Some(error) = f.query_error.get() {
        return error;
    }
    let canonical = if f.canonical.get().is_null() {
        pointer
    } else {
        f.canonical.get()
    };
    unsafe {
        retain(canonical);
        *out = canonical;
    }
    ffi::OK
}
unsafe extern "C" fn get(pointer: *mut c_void) -> *mut c_void {
    let f = unsafe { fake(pointer) };
    f.gets.set(f.gets.get() + 1);
    if f.stop_on_get.get() {
        f.context.set(false);
    }
    f.controller.get()
}
unsafe extern "C" fn update(pointer: *mut c_void) {
    let f = unsafe { fake(pointer) };
    f.updates.set(f.updates.get() + 1);
}
struct Fixture {
    pointer: *mut c_void,
}
impl Fixture {
    fn new(context: Rc<Cell<bool>>) -> Self {
        static TABLE: OnceLock<[usize; 16]> = OnceLock::new();
        let table = TABLE.get_or_init(|| {
            let mut table = [0; 16];
            table[0] = query as *const () as usize;
            table[1] = retain as *const () as usize;
            table[2] = release_mock as *const () as usize;
            table[14] = get as *const () as usize;
            table[15] = update as *const () as usize;
            table
        });
        let pointer = Box::into_raw(Box::new(Fake {
            table: table.as_ptr(),
            refs: Cell::new(1),
            controller: Cell::new(std::ptr::null_mut()),
            canonical: Cell::new(std::ptr::null_mut()),
            gets: Cell::new(0),
            updates: Cell::new(0),
            unknown_queries: Cell::new(0),
            other_queries: Cell::new(0),
            context,
            stop_on_get: Cell::new(false),
            stop_on_query: Cell::new(false),
            stop_on_release_at: Cell::new(None),
            change_parameter_on_release: Cell::new(std::ptr::null_mut()),
            replace_with: Cell::new(std::ptr::null_mut()),
            query_error: Cell::new(None),
        }))
        .cast();
        Self { pointer }
    }
    fn f(&self) -> &Fake {
        unsafe { fake(self.pointer) }
    }
    fn parameter(&self) -> RawParameter {
        unsafe extern "C" fn empty() -> *mut c_void {
            std::ptr::null_mut()
        }
        unsafe extern "C" fn text(_: *const std::ffi::c_char) -> *const c_void {
            std::ptr::null()
        }
        unsafe {
            retain(self.pointer);
            RawParameter::from_owned(
                NonNull::new(self.pointer).unwrap(),
                TextApi {
                    get_empty_string: empty,
                    get_constant_cstring: text,
                },
            )
        }
    }
    fn guard(&self) -> impl FnMut() -> Result<(), String> {
        let context = self.f().context.clone();
        move || {
            if context.get() {
                Ok(())
            } else {
                Err("provider/document replaced".into())
            }
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        unsafe {
            release_mock(self.pointer);
        }
    }
}

#[test]
fn null_owner_is_observed_twice_without_reflection_or_reference_leaks() {
    let f = Fixture::new(Rc::new(Cell::new(true)));
    let p = f.parameter();
    assert!(p.controller_owner(&mut f.guard()).unwrap().is_none());
    assert!(f.f().gets.get() >= 2);
    assert_eq!(f.f().refs.get(), 2);
    assert_eq!(f.f().unknown_queries.get(), 0);
    assert_eq!(f.f().other_queries.get(), 0);
    assert_eq!(f.f().updates.get(), 0);
}

#[test]
fn nonnode_owner_pins_parameter_and_unknown_until_guarded_release() {
    let context = Rc::new(Cell::new(true));
    let parameter = Fixture::new(context.clone());
    let controller = Fixture::new(context);
    parameter.f().controller.set(controller.pointer);
    let p = parameter.parameter();
    let owner = p.controller_owner(&mut parameter.guard()).unwrap().unwrap();
    assert_eq!(
        unsafe { owner.as_borrowed_unknown() }.unwrap().as_ptr(),
        controller.pointer
    );
    assert_eq!(controller.f().refs.get(), 3);
    assert_eq!(parameter.f().refs.get(), 3);
    drop(p);
    assert_eq!(parameter.f().refs.get(), 2);
    owner.check(&mut parameter.guard()).unwrap();
    owner.release(&mut parameter.guard()).unwrap();
    assert_eq!(parameter.f().refs.get(), 1);
    assert_eq!(controller.f().refs.get(), 1);
    assert_eq!(controller.f().other_queries.get(), 0);
    assert_eq!(parameter.f().updates.get(), 0);
}

#[test]
fn borrowed_owner_is_retained_before_guard_reentry_and_failure_releases_every_pin() {
    let context = Rc::new(Cell::new(true));
    let parameter = Fixture::new(context.clone());
    let controller = Fixture::new(context);
    parameter.f().controller.set(controller.pointer);
    let p = parameter.parameter();
    let mut checked = false;
    let mut guard = || {
        if parameter.f().gets.get() > 0 {
            if !checked {
                assert_eq!(controller.f().refs.get(), 2);
                checked = true;
            }
            Err("guard reentered".into())
        } else {
            Ok(())
        }
    };
    assert!(matches!(
        p.controller_owner(&mut guard),
        Err(Error::Guard(_))
    ));
    assert!(checked);
    assert_eq!(controller.f().refs.get(), 1);
    assert_eq!(parameter.f().refs.get(), 2);
}

#[test]
fn stale_and_native_get_query_release_reentry_are_rejected_without_leaks() {
    for scenario in 0..4 {
        let context = Rc::new(Cell::new(true));
        let parameter = Fixture::new(context.clone());
        let controller = Fixture::new(context);
        parameter.f().controller.set(controller.pointer);
        let p = parameter.parameter();
        match scenario {
            0 => parameter.f().context.set(false),
            1 => parameter.f().stop_on_get.set(true),
            2 => controller.f().stop_on_query.set(true),
            _ => controller.f().stop_on_release_at.set(Some(5)),
        }
        assert!(matches!(
            p.controller_owner(&mut parameter.guard()),
            Err(Error::Guard(_))
        ));
        assert_eq!(controller.f().refs.get(), 1);
        assert_eq!(parameter.f().refs.get(), 2);
        if scenario == 0 {
            assert_eq!(parameter.f().gets.get(), 0);
        }
    }
}

#[test]
fn replaced_or_removed_association_and_canonical_changes_are_rejected() {
    let context = Rc::new(Cell::new(true));
    let parameter = Fixture::new(context.clone());
    let controller = Fixture::new(context.clone());
    let replacement = Fixture::new(context);
    parameter.f().controller.set(controller.pointer);
    let p = parameter.parameter();
    let owner = p.controller_owner(&mut parameter.guard()).unwrap().unwrap();
    parameter.f().controller.set(replacement.pointer);
    assert_eq!(
        owner.check(&mut parameter.guard()),
        Err(Error::MetadataChanged)
    );
    parameter.f().controller.set(std::ptr::null_mut());
    assert_eq!(
        owner.check(&mut parameter.guard()),
        Err(Error::MetadataChanged)
    );
    parameter.f().controller.set(controller.pointer);
    controller.f().canonical.set(replacement.pointer);
    assert_eq!(
        owner.check(&mut parameter.guard()),
        Err(Error::MetadataChanged)
    );
    assert_eq!(replacement.f().refs.get(), 1);
    controller.f().canonical.set(std::ptr::null_mut());
    owner.release(&mut parameter.guard()).unwrap();
    assert_eq!(controller.f().refs.get(), 1);
    assert_eq!(parameter.f().refs.get(), 2);
}

#[test]
fn association_change_during_temporary_release_is_caught_after_cleanup() {
    let context = Rc::new(Cell::new(true));
    let parameter = Fixture::new(context.clone());
    let controller = Fixture::new(context.clone());
    let replacement = Fixture::new(context);
    parameter.f().controller.set(controller.pointer);
    let p = parameter.parameter();
    let owner = p.controller_owner(&mut parameter.guard()).unwrap().unwrap();
    controller
        .f()
        .change_parameter_on_release
        .set(parameter.pointer);
    controller.f().replace_with.set(replacement.pointer);
    assert_eq!(
        owner.check(&mut parameter.guard()),
        Err(Error::MetadataChanged)
    );
    controller
        .f()
        .change_parameter_on_release
        .set(std::ptr::null_mut());
    owner.release(&mut parameter.guard()).unwrap();
    assert_eq!(controller.f().refs.get(), 1);
    assert_eq!(replacement.f().refs.get(), 1);
    assert_eq!(parameter.f().refs.get(), 2);
}

#[test]
fn owner_appearing_during_null_parameter_pin_release_is_not_reported_absent() {
    let context = Rc::new(Cell::new(true));
    let parameter = Fixture::new(context.clone());
    let owner = Fixture::new(context);
    let p = parameter.parameter();
    parameter
        .f()
        .change_parameter_on_release
        .set(parameter.pointer);
    parameter.f().replace_with.set(owner.pointer);
    assert!(matches!(
        p.controller_owner(&mut parameter.guard()),
        Err(Error::MetadataChanged)
    ));
    assert_eq!(owner.f().refs.get(), 1);
    assert_eq!(parameter.f().refs.get(), 2);
    parameter
        .f()
        .change_parameter_on_release
        .set(std::ptr::null_mut());
}

#[test]
fn newly_present_owner_and_failed_canonical_query_never_become_null_success() {
    let context = Rc::new(Cell::new(true));
    let parameter = Fixture::new(context.clone());
    let controller = Fixture::new(context);
    let p = parameter.parameter();
    let mut changed = false;
    let mut guard = || {
        if parameter.f().gets.get() == 1 && !changed {
            parameter.f().controller.set(controller.pointer);
            changed = true;
        }
        Ok(())
    };
    assert!(matches!(
        p.controller_owner(&mut guard),
        Err(Error::MetadataChanged)
    ));
    assert_eq!(controller.f().refs.get(), 1);
    assert_eq!(parameter.f().refs.get(), 2);
    controller.f().query_error.set(Some(-123));
    assert!(matches!(
        p.controller_owner(&mut parameter.guard()),
        Err(Error::NativeFailure("IUnknown", -123))
    ));
    assert_eq!(controller.f().refs.get(), 1);
    assert_eq!(parameter.f().refs.get(), 2);
}

#[test]
fn wrong_creator_thread_blocks_owner_get_check_and_borrow() {
    let context = Rc::new(Cell::new(true));
    let parameter = Fixture::new(context.clone());
    let controller = Fixture::new(context);
    parameter.f().controller.set(controller.pointer);
    let mut p = parameter.parameter();
    let original = thread::current().id();
    let other = std::thread::spawn(|| thread::current().id())
        .join()
        .unwrap();
    p.thread = other;
    assert!(matches!(
        p.controller_owner(&mut parameter.guard()),
        Err(Error::WrongThread)
    ));
    assert_eq!(parameter.f().gets.get(), 0);
    p.thread = original;
    let mut owner = p.controller_owner(&mut parameter.guard()).unwrap().unwrap();
    let calls = parameter.f().gets.get();
    owner.thread = other;
    assert_eq!(owner.check(&mut parameter.guard()), Err(Error::WrongThread));
    assert_eq!(
        unsafe { owner.as_borrowed_unknown() },
        Err(Error::WrongThread)
    );
    assert_eq!(parameter.f().gets.get(), calls);
    owner.thread = original;
    owner.release(&mut parameter.guard()).unwrap();
}

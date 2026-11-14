use crate::ffi::{self, Owned, Uid};
use std::{
    ffi::c_void,
    ptr,
    sync::atomic::{AtomicU32, Ordering},
};

#[repr(C)]
struct List {
    table: *const *const c_void,
    refs: AtomicU32,
    item: Owned,
}
#[repr(C)]
struct Iter {
    table: *const *const c_void,
    refs: AtomicU32,
    item: Owned,
    done: bool,
}
struct Table<const N: usize>([*const c_void; N]);
unsafe impl<const N: usize> Sync for Table<N> {}
static LIST_TABLE: Table<11> = Table([
    l_query as *const c_void,
    l_retain as *const c_void,
    l_release as *const c_void,
    l_iter as *const c_void,
    l_empty as *const c_void,
    l_first as *const c_void,
    l_first as *const c_void,
    l_contains as *const c_void,
    l_add as *const c_void,
    l_remove as *const c_void,
    l_clear as *const c_void,
]);
static ITER_TABLE: Table<5> = Table([
    i_query as *const c_void,
    i_retain as *const c_void,
    i_release as *const c_void,
    i_done as *const c_void,
    i_next as *const c_void,
]);
unsafe extern "C" fn l_query(p: *mut c_void, iid: *const Uid, out: *mut *mut c_void) -> i32 {
    if iid.is_null() || out.is_null() {
        return 0x80004003u32 as i32;
    }
    unsafe { *out = ptr::null_mut() };
    if [ffi::UNKNOWN, ffi::LIST, ffi::CONTAINER].contains(unsafe { &*iid }) {
        unsafe {
            l_retain(p);
            *out = p
        };
        0
    } else {
        0x80004002u32 as i32
    }
}
unsafe extern "C" fn l_retain(p: *mut c_void) -> u32 {
    unsafe { &*p.cast::<List>() }
        .refs
        .fetch_add(1, Ordering::Relaxed)
        + 1
}
unsafe extern "C" fn l_release(p: *mut c_void) -> u32 {
    let left = unsafe { &*p.cast::<List>() }
        .refs
        .fetch_sub(1, Ordering::AcqRel)
        - 1;
    if left == 0 {
        drop(unsafe { Box::from_raw(p.cast::<List>()) });
    }
    left
}
unsafe extern "C" fn l_iter(p: *mut c_void) -> *mut c_void {
    let item = unsafe { Owned::hold((*p.cast::<List>()).item.raw()) }.expect("owned preset");
    Box::into_raw(Box::new(Iter {
        table: ITER_TABLE.0.as_ptr(),
        refs: AtomicU32::new(1),
        item,
        done: false,
    }))
    .cast()
}
unsafe extern "C" fn l_empty(_: *mut c_void) -> u8 {
    0
}
unsafe extern "C" fn l_first(p: *mut c_void) -> *mut c_void {
    unsafe { (*p.cast::<List>()).item.raw() }
}
unsafe extern "C" fn l_contains(p: *mut c_void, item: *mut c_void) -> u8 {
    (unsafe { l_first(p) } == item) as u8
}
unsafe extern "C" fn l_add(_: *mut c_void, _: *mut c_void, _: u8) -> u8 {
    0
}
unsafe extern "C" fn l_remove(_: *mut c_void, _: *mut c_void) -> u8 {
    0
}
unsafe extern "C" fn l_clear(_: *mut c_void) {}
unsafe extern "C" fn i_query(p: *mut c_void, iid: *const Uid, out: *mut *mut c_void) -> i32 {
    if iid.is_null() || out.is_null() {
        return 0x80004003u32 as i32;
    }
    unsafe { *out = ptr::null_mut() };
    if [ffi::UNKNOWN, ffi::ITERATOR].contains(unsafe { &*iid }) {
        unsafe {
            i_retain(p);
            *out = p
        };
        0
    } else {
        0x80004002u32 as i32
    }
}
unsafe extern "C" fn i_retain(p: *mut c_void) -> u32 {
    unsafe { &*p.cast::<Iter>() }
        .refs
        .fetch_add(1, Ordering::Relaxed)
        + 1
}
unsafe extern "C" fn i_release(p: *mut c_void) -> u32 {
    let left = unsafe { &*p.cast::<Iter>() }
        .refs
        .fetch_sub(1, Ordering::AcqRel)
        - 1;
    if left == 0 {
        drop(unsafe { Box::from_raw(p.cast::<Iter>()) });
    }
    left
}
unsafe extern "C" fn i_done(p: *mut c_void) -> u8 {
    (unsafe { (*p.cast::<Iter>()).done }) as u8
}
unsafe extern "C" fn i_next(p: *mut c_void) -> *mut c_void {
    let iter = unsafe { &mut *p.cast::<Iter>() };
    if iter.done {
        ptr::null_mut()
    } else {
        iter.done = true;
        iter.item.raw()
    }
}
pub(crate) fn one(item: &Owned) -> Owned {
    let item = unsafe { Owned::hold(item.raw()) }.expect("owned preset");
    let pointer = Box::into_raw(Box::new(List {
        table: LIST_TABLE.0.as_ptr(),
        refs: AtomicU32::new(1),
        item,
    }))
    .cast();
    unsafe { Owned::take(std::ptr::NonNull::new(pointer).expect("Box is nonnull")) }
}

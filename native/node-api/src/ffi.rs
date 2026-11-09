use std::{
    ffi::{c_char, c_void},
    marker::PhantomData,
    ptr::NonNull,
    rc::Rc,
};

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Uid {
    pub a: u32,
    pub b: u16,
    pub c: u16,
    pub d: [u8; 8],
}
pub(crate) const UNKNOWN: Uid = Uid {
    a: 0,
    b: 0,
    c: 0,
    d: [0xc0, 0, 0, 0, 0, 0, 0, 0x46],
};
pub(crate) const NODE: Uid = Uid {
    a: 0xdd45c0c2,
    b: 0x0903,
    c: 0x4648,
    d: [0x9a, 0xa0, 0xcb, 0x89, 0xe5, 0xa6, 0xcd, 0x02],
};
pub(crate) const OBJECT: Uid = Uid {
    a: 0x2e726012,
    b: 0x522c,
    c: 0x4108,
    d: [0x98, 0x62, 0x0c, 0xd5, 0x71, 0x79, 0x52, 0x49],
};
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct PlainString {
    pub object: *mut c_void,
    pub private: *mut c_void,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct MethodDefinition {
    pub name: *const c_char,
    pub args: *const c_char,
    pub retval: *const c_char,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct PropertyDefinition {
    pub name: *const c_char,
    pub data_type: i32,
    pub type_name: *const c_char,
    pub type_info: *const c_void,
}

pub(crate) unsafe fn method<T: Copy>(object: *mut c_void, slot: usize) -> T {
    let table = unsafe { *object.cast::<*const *const c_void>() };
    unsafe { std::mem::transmute_copy::<*const c_void, T>(&*table.add(slot)) }
}
pub(crate) struct Owned {
    pointer: NonNull<c_void>,
    _local: PhantomData<Rc<()>>,
}
impl Owned {
    pub(crate) unsafe fn take(pointer: NonNull<c_void>) -> Self {
        Self {
            pointer,
            _local: PhantomData,
        }
    }
    pub(crate) unsafe fn hold(pointer: *mut c_void) -> Option<Self> {
        let pointer = NonNull::new(pointer)?;
        let retain: unsafe extern "C" fn(*mut c_void) -> u32 =
            unsafe { method(pointer.as_ptr(), 1) };
        unsafe { retain(pointer.as_ptr()) };
        Some(unsafe { Self::take(pointer) })
    }
    pub(crate) fn raw(&self) -> *mut c_void {
        self.pointer.as_ptr()
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        let release: unsafe extern "C" fn(*mut c_void) -> u32 = unsafe { method(self.raw(), 2) };
        unsafe { release(self.raw()) };
    }
}
pub(crate) unsafe fn query(object: *mut c_void, iid: &Uid) -> Option<Owned> {
    let get: unsafe extern "C" fn(*mut c_void, *const Uid, *mut *mut c_void) -> i32 =
        unsafe { method(object, 0) };
    let mut pointer = std::ptr::null_mut();
    let status = unsafe { get(object, iid, &mut pointer) };
    let owner = NonNull::new(pointer).map(|p| unsafe { Owned::take(p) });
    if status == 0 {
        owner
    } else {
        drop(owner);
        None
    }
}
pub(crate) unsafe fn uid(pointer: *const Uid) -> Option<String> {
    let value = unsafe { pointer.as_ref() }?;
    if value.a == 0 && value.b == 0 && value.c == 0 && value.d == [0; 8] {
        return None;
    }
    Some(format!(
        "{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
        value.a,
        value.b,
        value.c,
        value.d[0],
        value.d[1],
        value.d[2],
        value.d[3],
        value.d[4],
        value.d[5],
        value.d[6],
        value.d[7]
    ))
}
const _: () = {
    use std::mem::{align_of, offset_of, size_of};
    assert!(size_of::<Uid>() == 16 && align_of::<Uid>() == 4);
    assert!(size_of::<PlainString>() == 16 && align_of::<PlainString>() == 8);
    assert!(size_of::<MethodDefinition>() == 24 && align_of::<MethodDefinition>() == 8);
    assert!(offset_of!(MethodDefinition, args) == 8 && offset_of!(MethodDefinition, retval) == 16);
    assert!(size_of::<PropertyDefinition>() == 32 && align_of::<PropertyDefinition>() == 8);
    assert!(
        offset_of!(PropertyDefinition, data_type) == 8
            && offset_of!(PropertyDefinition, type_name) == 16
            && offset_of!(PropertyDefinition, type_info) == 24
    );
};

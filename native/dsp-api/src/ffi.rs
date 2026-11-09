use super::Error;
use std::{
    ffi::{c_char, c_void},
    marker::PhantomData,
    ptr::NonNull,
    rc::Rc,
};

#[repr(C)]
pub(crate) struct Uid {
    pub a: u32,
    pub b: u16,
    pub c: u16,
    pub d: [u8; 8],
}
const DSP_PROVIDER: Uid = Uid {
    a: 0xd889caea,
    b: 0x8356,
    c: 0x45bb,
    d: [0x8b, 0x45, 0xee, 0xcd, 0x11, 0x31, 0xa9, 0x4d],
};
/// Plain wire representation; ownership belongs to the enclosing InfoRecord.
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct PlainString {
    pub object: *mut c_void,
    pub private: *mut c_void,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct PlainCString {
    pub text: *const c_char,
    pub owner: *mut c_void,
}

pub(crate) unsafe fn method<T: Copy>(object: *mut c_void, slot: usize) -> T {
    let table = unsafe { *object.cast::<*const *const c_void>() };
    unsafe { std::mem::transmute_copy::<*const c_void, T>(&*table.add(slot)) }
}
pub(crate) unsafe fn retain(object: *mut c_void) {
    let retain: unsafe extern "C" fn(*mut c_void) -> u32 = unsafe { method(object, 1) };
    unsafe { retain(object) };
}
pub(crate) unsafe fn release(object: *mut c_void) {
    let release: unsafe extern "C" fn(*mut c_void) -> u32 = unsafe { method(object, 2) };
    unsafe { release(object) };
}
pub(crate) struct Owned {
    pointer: NonNull<c_void>,
    _not_send_sync: PhantomData<Rc<()>>,
}
impl Owned {
    pub(crate) unsafe fn take(pointer: NonNull<c_void>) -> Self {
        Self {
            pointer,
            _not_send_sync: PhantomData,
        }
    }
    pub(crate) fn raw(&self) -> *mut c_void {
        self.pointer.as_ptr()
    }
    pub(crate) fn query_provider(&self) -> Result<Self, Error> {
        let query: unsafe extern "C" fn(*mut c_void, *const Uid, *mut *mut c_void) -> i32 =
            unsafe { method(self.raw(), 0) };
        let mut out = std::ptr::null_mut();
        let result = unsafe { query(self.raw(), &DSP_PROVIDER, &mut out) };
        if result != 0 {
            return Err(Error::MissingProvider(result));
        }
        let out = NonNull::new(out).ok_or(Error::NullProvider)?;
        Ok(unsafe { Self::take(out) })
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        unsafe { release(self.raw()) };
    }
}
const _: () = {
    assert!(std::mem::size_of::<Uid>() == 16 && std::mem::align_of::<Uid>() == 4);
    assert!(std::mem::size_of::<PlainString>() == 16 && std::mem::align_of::<PlainString>() == 8);
    assert!(std::mem::size_of::<PlainCString>() == 16 && std::mem::align_of::<PlainCString>() == 8);
};

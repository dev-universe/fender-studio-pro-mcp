use super::{Error, TextApi};
use std::{
    ffi::{CStr, c_void},
    marker::PhantomData,
    ptr::NonNull,
    rc::Rc,
};

pub(crate) const OK: i32 = 0;
pub(crate) const NO_INTERFACE: i32 = 0x80004002u32 as i32;
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Uid {
    pub a: u32,
    pub b: u16,
    pub c: u16,
    pub d: [u8; 8],
}
pub(crate) const OBJECT: Uid = Uid {
    a: 0x2e726012,
    b: 0x522c,
    c: 0x4108,
    d: [0x98, 0x62, 0x0c, 0xd5, 0x71, 0x79, 0x52, 0x49],
};
pub(crate) const UNKNOWN: Uid = Uid {
    a: 0,
    b: 0,
    c: 0,
    d: [0xc0, 0, 0, 0, 0, 0, 0, 0x46],
};
pub(crate) const LIST_PARAMETER: Uid = Uid {
    a: 0x143641b5,
    b: 0x92a0,
    c: 0x4a2e,
    d: [0xb7, 0x0c, 0x84, 0xc5, 0x7e, 0x8f, 0xbd, 0xb3],
};
#[repr(C)]
pub(crate) struct CStringRepr {
    pub text: *const std::ffi::c_char,
    pub owner: *mut c_void,
}
#[repr(C)]
pub(crate) struct StringRepr {
    pub object: *mut c_void,
    pub private: *mut c_void,
}

// Every slot is an internal source-checked constant; none comes from a request.
pub(crate) unsafe fn method<T: Copy>(object: *mut c_void, slot: usize) -> T {
    let table = unsafe { *object.cast::<*const *const c_void>() };
    let code = unsafe { *table.add(slot) };
    unsafe { std::mem::transmute_copy::<*const c_void, T>(&code) }
}
pub(crate) struct Owned {
    pointer: NonNull<c_void>,
    _not_send_sync: PhantomData<Rc<()>>,
}
impl Owned {
    /// Takes one existing caller-owned COM-style reference.
    pub(crate) unsafe fn take(pointer: NonNull<c_void>) -> Self {
        Self {
            pointer,
            _not_send_sync: PhantomData,
        }
    }
    pub(crate) fn raw(&self) -> *mut c_void {
        self.pointer.as_ptr()
    }
    pub(crate) fn into_raw(self) -> *mut c_void {
        let pointer = self.raw();
        std::mem::forget(self);
        pointer
    }
    pub(crate) unsafe fn hold(pointer: *mut c_void) -> Result<Self, Error> {
        let pointer = NonNull::new(pointer).ok_or(Error::MissingInterface("IUnknown"))?;
        let retain: unsafe extern "C" fn(*mut c_void) -> u32 =
            unsafe { method(pointer.as_ptr(), 1) };
        unsafe { retain(pointer.as_ptr()) };
        Ok(unsafe { Self::take(pointer) })
    }
    pub(crate) fn query(&self, iid: &Uid) -> Result<Self, Error> {
        let get: unsafe extern "C" fn(*mut c_void, *const Uid, *mut *mut c_void) -> i32 =
            unsafe { method(self.raw(), 0) };
        let mut out = std::ptr::null_mut();
        let result = unsafe { get(self.raw(), iid, &mut out) };
        if result != OK {
            return Err(Error::MissingInterface("IObject"));
        }
        let out = NonNull::new(out).ok_or(Error::MissingInterface("IObject"))?;
        Ok(unsafe { Self::take(out) })
    }
    pub(crate) fn query_optional(
        &self,
        iid: &Uid,
        name: &'static str,
    ) -> Result<Option<Self>, Error> {
        let get: unsafe extern "C" fn(*mut c_void, *const Uid, *mut *mut c_void) -> i32 =
            unsafe { method(self.raw(), 0) };
        let mut out = std::ptr::null_mut();
        let result = unsafe { get(self.raw(), iid, &mut out) };
        if result == NO_INTERFACE && out.is_null() {
            return Ok(None);
        }
        // A successful QI transfers one reference. A failed QI has no valid
        // output reference to own, even if a provider violates the null rule.
        if result != OK {
            return Err(Error::NativeFailure(name, result));
        }
        let out = NonNull::new(out).ok_or(Error::MissingInterface(name))?;
        Ok(Some(unsafe { Self::take(out) }))
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        let release: unsafe extern "C" fn(*mut c_void) -> u32 = unsafe { method(self.raw(), 2) };
        unsafe { release(self.raw()) };
    }
}

pub(crate) fn property_id(api: TextApi, name: &'static CStr) -> Result<*const c_void, Error> {
    let pointer = unsafe { (api.get_constant_cstring)(name.as_ptr()) };
    if pointer.is_null() {
        Err(Error::MissingInterface("constant CString"))
    } else {
        Ok(pointer)
    }
}
pub(crate) fn name(parameter: *mut c_void) -> Result<String, Error> {
    let get: unsafe extern "C" fn(*mut c_void) -> *const CStringRepr =
        unsafe { method(parameter, 4) };
    let reference = unsafe { get(parameter) };
    if reference.is_null() {
        return Err(Error::InvalidData("null parameter name reference"));
    }
    let text = unsafe { (*reference).text };
    if text.is_null() {
        return Ok(String::new());
    }
    let mut bytes = Vec::new();
    for index in 0..=super::MAX_NAME_BYTES {
        let byte = unsafe { *text.add(index) } as u8;
        if byte == 0 {
            return String::from_utf8(bytes).map_err(|_| Error::InvalidText);
        }
        if index == super::MAX_NAME_BYTES {
            return Err(Error::TextTooLong);
        }
        bytes.push(byte);
    }
    Err(Error::TextTooLong)
}
const _: () = {
    assert!(std::mem::size_of::<Uid>() == 16);
    assert!(std::mem::size_of::<StringRepr>() == 16);
    assert!(std::mem::size_of::<CStringRepr>() == 16);
};

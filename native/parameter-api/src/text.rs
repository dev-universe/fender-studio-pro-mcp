use super::{
    Error, MAX_TEXT_UNITS, TextApi,
    ffi::{self, Owned, StringRepr},
};
use std::{ffi::c_void, ptr::NonNull};

pub(crate) fn copy_i_string(pointer: *mut c_void) -> Result<String, Error> {
    let owner = unsafe { Owned::hold(pointer) }?;
    let length: unsafe extern "C" fn(*mut c_void) -> i32 = unsafe { ffi::method(owner.raw(), 4) };
    let n = unsafe { length(owner.raw()) };
    if n < 0 || n as usize > MAX_TEXT_UNITS {
        return Err(Error::TextTooLong);
    }
    // A non-NUL sentinel also detects an incorrect successful copy that never
    // writes the required terminator.
    let mut buffer = vec![0xffff_u16; n as usize + 1];
    let copy: unsafe extern "C" fn(*mut c_void, *mut u16, i32) -> i32 =
        unsafe { ffi::method(owner.raw(), 8) };
    let result = unsafe { copy(owner.raw(), buffer.as_mut_ptr(), n + 1) };
    if result != ffi::OK {
        return Err(Error::NativeFailure("IString.copyTo", result));
    }
    if unsafe { length(owner.raw()) } != n || buffer[n as usize] != 0 {
        return Err(Error::InvalidData(
            "String changed length or lacks termination",
        ));
    }
    String::from_utf16(&buffer[..n as usize]).map_err(|_| Error::InvalidText)
}
pub(crate) struct NativeString {
    repr: StringRepr,
}
impl NativeString {
    pub(crate) fn empty(api: TextApi) -> Result<Self, Error> {
        let owner = unsafe { Owned::hold((api.get_empty_string)()) }?;
        let private: unsafe extern "C" fn(*mut c_void) -> *mut c_void =
            unsafe { ffi::method(owner.raw(), 21) };
        let data = unsafe { private(owner.raw()) };
        Ok(Self {
            repr: StringRepr {
                object: owner.into_raw(),
                private: data,
            },
        })
    }
    pub(crate) fn from_text(api: TextApi, value: &str) -> Result<Self, Error> {
        let mut units: Vec<u16> = value.encode_utf16().take(MAX_TEXT_UNITS + 1).collect();
        if units.len() > MAX_TEXT_UNITS {
            return Err(Error::TextTooLong);
        }
        if units.contains(&0) {
            return Err(Error::InvalidText);
        }
        let empty = unsafe { Owned::hold((api.get_empty_string)()) }?;
        let clone: unsafe extern "C" fn(*mut c_void) -> *mut c_void =
            unsafe { ffi::method(empty.raw(), 18) };
        let pointer = NonNull::new(unsafe { clone(empty.raw()) })
            .ok_or(Error::MissingInterface("IString.cloneString"))?;
        let owner = unsafe { Owned::take(pointer) };
        let assign: unsafe extern "C" fn(*mut c_void, *const u16, i32) -> i32 =
            unsafe { ffi::method(owner.raw(), 24) };
        let result = unsafe { assign(owner.raw(), units.as_mut_ptr(), units.len() as i32) };
        if result != ffi::OK {
            return Err(Error::NativeFailure("IString.assignChars", result));
        }
        let private: unsafe extern "C" fn(*mut c_void) -> *mut c_void =
            unsafe { ffi::method(owner.raw(), 21) };
        let data = unsafe { private(owner.raw()) };
        Ok(Self {
            repr: StringRepr {
                object: owner.into_raw(),
                private: data,
            },
        })
    }
    pub(crate) fn as_ptr(&self) -> *const StringRepr {
        &self.repr
    }
    pub(crate) fn as_mut_ptr(&mut self) -> *mut StringRepr {
        &mut self.repr
    }
    pub(crate) fn copy(&self) -> Result<String, Error> {
        copy_i_string(self.repr.object)
    }
}
impl Drop for NativeString {
    fn drop(&mut self) {
        if let Some(pointer) = NonNull::new(self.repr.object) {
            drop(unsafe { Owned::take(pointer) });
        }
    }
}

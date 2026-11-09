//! Independently authored exact-8.1.1 interoperability record. No vendor code.
use super::{
    Error, InfoError, MAX_NAME_BYTES, MAX_TITLE_UNITS, Parameter, TextApi,
    ffi::{self, PlainCString, PlainString},
};
use std::ffi::c_void;

#[repr(C)]
pub(crate) struct ParamInfo {
    tag: u32,
    folder: u32,
    name: PlainCString,
    cstring_24: PlainCString,
    title: PlainString,
    string_56: PlainString,
    string_72: PlainString,
    cstring_88: PlainCString,
    float_104: f32,
    integer_108: u32,
    flags: u32,
    integer_116: u32,
    integer_120: u64,
}
pub(crate) struct EmptyValues {
    string: PlainString,
    cstring: PlainCString,
}
impl EmptyValues {
    pub(crate) fn new(api: TextApi) -> Result<Self, Error> {
        let string = unsafe { (api.get_constant_string)(c"".as_ptr()) }.cast::<PlainString>();
        if string.is_null() {
            return Err(Error::MissingTextConstant("String"));
        }
        let cstring = unsafe { (api.get_constant_cstring)(c"".as_ptr()) }.cast::<PlainCString>();
        if cstring.is_null() {
            return Err(Error::MissingTextConstant("CString"));
        }
        let string = unsafe { *string };
        let cstring = unsafe { *cstring };
        if string.object.is_null() {
            return Err(Error::InvalidEmptyString);
        }
        Ok(Self { string, cstring })
    }
    pub(crate) fn record(&self) -> InfoRecord {
        // A zeroed native String is invalid. Each destination owns one retained
        // reference copied from a valid borrowed public constant representation.
        for _ in 0..3 {
            unsafe { ffi::retain(self.string.object) };
            if !self.cstring.owner.is_null() {
                unsafe { ffi::retain(self.cstring.owner) };
            }
        }
        InfoRecord(ParamInfo {
            tag: 0,
            folder: 0,
            name: self.cstring,
            cstring_24: self.cstring,
            title: self.string,
            string_56: self.string,
            string_72: self.string,
            cstring_88: self.cstring,
            float_104: 0.0,
            integer_108: 0,
            flags: 0,
            integer_116: 0,
            integer_120: 0,
        })
    }
}
pub(crate) struct InfoRecord(pub(crate) ParamInfo);
impl InfoRecord {
    pub(crate) fn decode(&self, index: u32) -> Result<Parameter, InfoError> {
        if self.0.title.object.is_null()
            || self.0.string_56.object.is_null()
            || self.0.string_72.object.is_null()
        {
            return Err(InfoError::InvalidStringOwners);
        }
        Ok(Parameter {
            index,
            tag: self.0.tag,
            name: copy_name(self.0.name.text)?,
            title: copy_title(self.0.title.object)?,
            folder: self.0.folder,
            raw_flags: self.0.flags,
        })
    }
}
impl Drop for InfoRecord {
    fn drop(&mut self) {
        for owner in [
            self.0.cstring_88.owner,
            self.0.string_72.object,
            self.0.string_56.object,
            self.0.title.object,
            self.0.cstring_24.owner,
            self.0.name.owner,
        ] {
            if !owner.is_null() {
                unsafe { ffi::release(owner) };
            }
        }
    }
}
fn copy_name(text: *const std::ffi::c_char) -> Result<String, InfoError> {
    if text.is_null() {
        return Ok(String::new());
    }
    let mut bytes = Vec::new();
    for index in 0..=MAX_NAME_BYTES {
        let byte = unsafe { *text.add(index) } as u8;
        if byte == 0 {
            return String::from_utf8(bytes).map_err(|_| InfoError::InvalidNameUtf8);
        }
        if index == MAX_NAME_BYTES {
            return Err(InfoError::NameTooLong);
        }
        bytes.push(byte);
    }
    Err(InfoError::NameTooLong)
}
fn copy_title(object: *mut c_void) -> Result<String, InfoError> {
    let length: unsafe extern "C" fn(*mut c_void) -> i32 = unsafe { ffi::method(object, 4) };
    let n = unsafe { length(object) };
    if !(0..=MAX_TITLE_UNITS as i32).contains(&n) {
        return Err(InfoError::TitleTooLong);
    }
    let mut buffer = vec![0xffff_u16; n as usize + 1];
    let copy: unsafe extern "C" fn(*mut c_void, *mut u16, i32) -> i32 =
        unsafe { ffi::method(object, 8) };
    let status = unsafe { copy(object, buffer.as_mut_ptr(), n + 1) };
    if status != 0 {
        return Err(InfoError::TitleCopyFailed(status));
    }
    if unsafe { length(object) } != n {
        return Err(InfoError::TitleChangedLength);
    }
    if buffer[n as usize] != 0 {
        return Err(InfoError::TitleNotTerminated);
    }
    String::from_utf16(&buffer[..n as usize]).map_err(|_| InfoError::InvalidTitleUtf16)
}
const _: () = {
    use std::mem::{align_of, offset_of, size_of};
    assert!(size_of::<ParamInfo>() == 128 && align_of::<ParamInfo>() == 8);
    assert!(offset_of!(ParamInfo, tag) == 0 && offset_of!(ParamInfo, folder) == 4);
    assert!(offset_of!(ParamInfo, name) == 8 && offset_of!(ParamInfo, cstring_24) == 24);
    assert!(
        offset_of!(ParamInfo, title) == 40
            && offset_of!(ParamInfo, string_56) == 56
            && offset_of!(ParamInfo, string_72) == 72
    );
    assert!(
        offset_of!(ParamInfo, cstring_88) == 88
            && offset_of!(ParamInfo, float_104) == 104
            && offset_of!(ParamInfo, integer_108) == 108
    );
    assert!(
        offset_of!(ParamInfo, flags) == 112
            && offset_of!(ParamInfo, integer_116) == 116
            && offset_of!(ParamInfo, integer_120) == 120
    );
};

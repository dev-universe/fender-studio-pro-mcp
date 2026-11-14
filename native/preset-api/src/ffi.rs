use crate::Error;
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
pub(crate) const PRESET: Uid = Uid {
    a: 0x340b7644,
    b: 0x8eee,
    c: 0x4f23,
    d: [0xb9, 0xd3, 0xfe, 0xf0, 0x8f, 0x20, 0xda, 0xdf],
};
pub(crate) const ATTRIBUTES_CLASS: Uid = Uid {
    a: 0x138ed4e7,
    b: 0x1786,
    c: 0x4a9d,
    d: [0xa0, 0xef, 0xd7, 0x6f, 0xba, 0x49, 0xb4, 0xda],
};
pub(crate) const ATTRIBUTES: Uid = Uid {
    a: 0x6bb2c332,
    b: 0x0137,
    c: 0x4820,
    d: [0xb1, 0x37, 0x47, 0x01, 0xb3, 0x90, 0xe5, 0x12],
};
pub(crate) const CONTAINER: Uid = Uid {
    a: 0x703469c0,
    b: 0x5c71,
    c: 0x4488,
    d: [0x9c, 0x9f, 0x93, 0xdb, 0x43, 0x5e, 0x72, 0xe8],
};
pub(crate) const LIST: Uid = Uid {
    a: 0x462f2bf1,
    b: 0x256f,
    c: 0x402f,
    d: [0xa5, 0x75, 0xf6, 0x2c, 0x55, 0x61, 0x5f, 0x71],
};
pub(crate) const ITERATOR: Uid = Uid {
    a: 0xcec32585,
    b: 0x7e3f,
    c: 0x44a2,
    d: [0xbd, 0x59, 0xd1, 0xd8, 0xcc, 0xb2, 0x58, 0x7d],
};
#[repr(C)]
pub(crate) struct CStringRepr {
    pub text: *const c_char,
    pub owner: *mut c_void,
}
#[repr(C)]
pub(crate) struct StringRepr {
    pub object: *mut c_void,
    pub private: *mut c_void,
}
#[repr(C)]
pub(crate) struct Variant {
    pub tag: i16,
    pub padding: [u8; 6],
    pub payload: u64,
}
impl Variant {
    pub fn empty() -> Self {
        Self {
            tag: 0,
            padding: [0; 6],
            payload: 0,
        }
    }
}
impl Drop for Variant {
    fn drop(&mut self) {
        if self.tag & 0x10 != 0 && matches!(self.tag & 15, 3 | 4) {
            if let Some(p) = NonNull::new(self.payload as usize as *mut c_void) {
                drop(unsafe { Owned::take(p) });
            }
        }
    }
}
pub(crate) unsafe fn method<T: Copy>(p: *mut c_void, slot: usize) -> T {
    let table = unsafe { *p.cast::<*const *const c_void>() };
    unsafe { std::mem::transmute_copy::<*const c_void, T>(&*table.add(slot)) }
}
pub(crate) struct Owned {
    pointer: NonNull<c_void>,
    _local: PhantomData<Rc<()>>,
}
impl Owned {
    pub unsafe fn take(pointer: NonNull<c_void>) -> Self {
        Self {
            pointer,
            _local: PhantomData,
        }
    }
    pub unsafe fn hold(p: *mut c_void) -> Result<Self, Error> {
        let p = NonNull::new(p).ok_or(Error::Unavailable("null native object"))?;
        let retain: unsafe extern "C" fn(*mut c_void) -> u32 = unsafe { method(p.as_ptr(), 1) };
        unsafe { retain(p.as_ptr()) };
        Ok(unsafe { Self::take(p) })
    }
    pub fn raw(&self) -> *mut c_void {
        self.pointer.as_ptr()
    }
    pub fn query(&self, iid: &Uid) -> Result<Self, Error> {
        let query: unsafe extern "C" fn(*mut c_void, *const Uid, *mut *mut c_void) -> i32 =
            unsafe { method(self.raw(), 0) };
        let mut out = std::ptr::null_mut();
        let status = unsafe { query(self.raw(), iid, &mut out) };
        unsafe { Self::from_success(status, out, "required interface is unavailable") }
    }
    pub unsafe fn from_success(
        status: i32,
        out: *mut c_void,
        error: &'static str,
    ) -> Result<Self, Error> {
        if status != 0 {
            return Err(Error::Unavailable(error));
        }
        NonNull::new(out)
            .map(|pointer| unsafe { Self::take(pointer) })
            .ok_or(Error::Unavailable("successful native result returned null"))
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        let release: unsafe extern "C" fn(*mut c_void) -> u32 = unsafe { method(self.raw(), 2) };
        unsafe { release(self.raw()) };
    }
}

pub(crate) fn copy_string(reference: *const StringRepr, limit: usize) -> Result<String, Error> {
    let reference =
        unsafe { reference.as_ref() }.ok_or(Error::InvalidData("missing String reference"))?;
    copy_i_string(reference.object, limit)
}
pub(crate) fn copy_i_string(pointer: *mut c_void, limit: usize) -> Result<String, Error> {
    let owner = unsafe { Owned::hold(pointer) }?;
    let length: unsafe extern "C" fn(*mut c_void) -> i32 = unsafe { method(owner.raw(), 4) };
    let len = unsafe { length(owner.raw()) };
    if len < 0 || len as usize > limit {
        return Err(Error::InvalidData("native text exceeds the bound"));
    }
    let mut text = vec![0xffff; len as usize + 1];
    let copy: unsafe extern "C" fn(*mut c_void, *mut u16, i32) -> i32 =
        unsafe { method(owner.raw(), 8) };
    if unsafe { copy(owner.raw(), text.as_mut_ptr(), text.len() as i32) } != 0
        || unsafe { length(owner.raw()) } != len
        || text[len as usize] != 0
    {
        return Err(Error::InvalidData(
            "native text changed or was not terminated",
        ));
    }
    String::from_utf16(&text[..len as usize])
        .map_err(|_| Error::InvalidData("invalid native UTF-16"))
}
pub(crate) fn class_key(attributes: &Owned) -> Result<String, Error> {
    let name = CStringRepr {
        text: c"Class:ID".as_ptr(),
        owner: std::ptr::null_mut(),
    };
    let mut value = Variant::empty();
    let get: unsafe extern "C" fn(*mut c_void, *mut Variant, *const CStringRepr) -> u8 =
        unsafe { method(attributes.raw(), 10) };
    if unsafe { get(attributes.raw(), &mut value, &name) } == 0 || value.tag & 15 != 3 {
        return Err(Error::InvalidData("preset class metadata is missing"));
    }
    let text = copy_i_string(value.payload as usize as *mut c_void, 128)?;
    if text.is_empty() || text.contains('\0') {
        return Err(Error::InvalidData("invalid preset class metadata"));
    }
    Ok(text)
}
const _: () = {
    assert!(std::mem::size_of::<Uid>() == 16);
    assert!(std::mem::size_of::<StringRepr>() == 16);
    assert!(std::mem::size_of::<CStringRepr>() == 16);
    assert!(std::mem::size_of::<Variant>() == 16);
    assert!(std::mem::align_of::<Variant>() == 8);
    assert!(std::mem::offset_of!(Variant, payload) == 8);
};

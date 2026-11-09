use super::{Error, GuiApi};
use std::{ffi::c_void, marker::PhantomData, ptr::NonNull, rc::Rc};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Uid {
    pub a: u32,
    pub b: u16,
    pub c: u16,
    pub d: [u8; 8],
}
impl Uid {
    pub fn is_empty(self) -> bool {
        self.a == 0 && self.b == 0 && self.c == 0 && self.d == [0; 8]
    }
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
    d: [0x9a, 0xa0, 0xcb, 0x89, 0xe5, 0xa6, 0xcd, 2],
};
pub(crate) const OBJECT: Uid = Uid {
    a: 0x2e726012,
    b: 0x522c,
    c: 0x4108,
    d: [0x98, 0x62, 0x0c, 0xd5, 0x71, 0x79, 0x52, 0x49],
};
pub(crate) const VIEW: Uid = Uid {
    a: 0xf5b09f71,
    b: 0x6f3f,
    c: 0x4f03,
    d: [0xa5, 0x4a, 0xb2, 0x49, 0xb8, 0x29, 0x62, 0xd6],
};
pub(crate) const CONTROLLER: Uid = Uid {
    a: 0x46ee5c3a,
    b: 0x6a49,
    c: 0x4ebc,
    d: [0xa0, 0x74, 0x4f, 0xc4, 0xc5, 0x92, 0xee, 0xd9],
};
pub const EXTERNAL_MONO: Uid = Uid {
    a: 0x873281d9,
    b: 0x58de,
    c: 0x4b2d,
    d: [0xb3, 0xbd, 0x52, 0x8f, 0x54, 0x9b, 0x1a, 0xd9],
};
pub const EXTERNAL_STEREO: Uid = Uid {
    a: 0x889dd9f8,
    b: 0x14f0,
    c: 0x44a5,
    d: [0xbc, 0xee, 0x0f, 0x2b, 0x15, 0x7f, 0xc5, 0xd4],
};

pub(crate) unsafe fn method<T: Copy>(object: *mut c_void, slot: usize) -> T {
    let table = unsafe { *object.cast::<*const *const c_void>() };
    unsafe { std::mem::transmute_copy::<*const c_void, T>(&*table.add(slot)) }
}
pub(crate) struct Owned {
    p: NonNull<c_void>,
    _thread: PhantomData<Rc<()>>,
}
impl Owned {
    pub(crate) unsafe fn take(p: NonNull<c_void>) -> Self {
        Self {
            p,
            _thread: PhantomData,
        }
    }
    pub(crate) fn raw(&self) -> *mut c_void {
        self.p.as_ptr()
    }
    pub(crate) fn query(&self, iid: &Uid) -> Result<Self, Error> {
        let q: unsafe extern "C" fn(*mut c_void, *const Uid, *mut *mut c_void) -> i32 =
            unsafe { method(self.raw(), 0) };
        let mut out = std::ptr::null_mut();
        if unsafe { q(self.raw(), iid, &mut out) } != 0 {
            return Err(Error::MissingInterface);
        }
        Ok(unsafe { Self::take(NonNull::new(out).ok_or(Error::MissingInterface)?) })
    }
    pub(crate) unsafe fn hold(p: *mut c_void) -> Result<Self, Error> {
        let p = NonNull::new(p).ok_or(Error::MissingObject)?;
        let retain: unsafe extern "C" fn(*mut c_void) -> u32 = unsafe { method(p.as_ptr(), 1) };
        unsafe { retain(p.as_ptr()) };
        Ok(unsafe { Self::take(p) })
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        let release: unsafe extern "C" fn(*mut c_void) -> u32 = unsafe { method(self.raw(), 2) };
        unsafe { release(self.raw()) };
    }
}
pub(crate) fn borrowed(p: *mut c_void, slot: usize) -> *mut c_void {
    let f: unsafe extern "C" fn(*mut c_void) -> *mut c_void = unsafe { method(p, slot) };
    unsafe { f(p) }
}
pub(crate) fn count(p: *mut c_void, slot: usize) -> i32 {
    let f: unsafe extern "C" fn(*mut c_void) -> i32 = unsafe { method(p, slot) };
    unsafe { f(p) }
}
pub(crate) fn class(object: &Owned) -> Result<String, Error> {
    let o = object.query(&OBJECT)?;
    class_from_object(&o)
}
pub(crate) fn class_from_object(o: &Owned) -> Result<String, Error> {
    let info = borrowed(o.raw(), 3);
    if info.is_null() {
        return Err(Error::InvalidMetadata);
    }
    let get: unsafe extern "C" fn(*mut c_void) -> *const std::ffi::c_char =
        unsafe { method(info, 5) };
    let text = unsafe { get(info) };
    if text.is_null() {
        return Err(Error::InvalidMetadata);
    }
    let mut bytes = Vec::new();
    for i in 0..=128 {
        let b = unsafe { *text.add(i) } as u8;
        if b == 0 {
            return String::from_utf8(bytes).map_err(|_| Error::InvalidMetadata);
        }
        if i == 128 {
            return Err(Error::InvalidMetadata);
        }
        bytes.push(b);
    }
    Err(Error::InvalidMetadata)
}
pub(crate) fn uid(node: &Owned, slot: usize) -> Result<Uid, Error> {
    let f: unsafe extern "C" fn(*mut c_void) -> *const Uid = unsafe { method(node.raw(), slot) };
    let p = unsafe { f(node.raw()) };
    if p.is_null() {
        Err(Error::InvalidIdentity)
    } else {
        Ok(unsafe { *p })
    }
}
pub(crate) fn find_raw(
    node: &Owned,
    api: GuiApi,
    key: &'static std::ffi::CStr,
) -> Result<*mut c_void, Error> {
    let name = unsafe { (api.get_constant_string)(key.as_ptr()) };
    if name.is_null() {
        return Err(Error::MissingConstant);
    }
    let f: unsafe extern "C" fn(*mut c_void, *const c_void) -> *mut c_void =
        unsafe { method(node.raw(), 10) };
    let found = unsafe { f(node.raw(), name) };
    if !found.is_null() {
        return Ok(found);
    }
    let f: unsafe extern "C" fn(*mut c_void, *const c_void) -> *mut c_void =
        unsafe { method(node.raw(), 13) };
    Ok(unsafe { f(node.raw(), name) })
}
const _: () = {
    assert!(std::mem::size_of::<Uid>() == 16 && std::mem::align_of::<Uid>() == 4);
};

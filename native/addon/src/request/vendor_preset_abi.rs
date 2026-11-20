use std::ffi::{c_char, c_void};
use std::mem::{MaybeUninit, size_of};

#[repr(C, align(8))]
struct CppString([MaybeUninit<u8>; 24]);

#[repr(C)]
struct VendorReturn {
    prefix: [MaybeUninit<u8>; 8],
    status: i32,
    padding: [MaybeUninit<u8>; 4],
    message: CppString,
}

#[repr(C, align(16))]
struct FileStorage([MaybeUninit<u8>; 160]);

const _: () = assert!(size_of::<VendorReturn>() == 40);

#[link(name = "System")]
unsafe extern "C" {
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

pub(super) struct Apply {
    pub receiver: *mut c_void,
    pub constructor: *const c_void,
    pub activate: *const c_void,
    pub destructor: *const c_void,
}

// Caller validates the exact official binary and rebinds the attached editor
// immediately before this call on the active document and main thread.
pub(super) unsafe fn apply(target: Apply, path: &[u8]) -> Result<i32, &'static str> {
    if target.receiver.is_null()
        || target.constructor.is_null()
        || target.activate.is_null()
        || target.destructor.is_null()
        || !(1..=4096).contains(&path.len())
        || path.contains(&0)
    {
        return Err("vendor file arguments are invalid");
    }
    let process = (-2_isize) as *mut c_void;
    let assign = unsafe {
        dlsym(
            process,
            c"_ZNSt3__112basic_stringIcNS_11char_traitsIcEENS_9allocatorIcEEE6assignEPKcm".as_ptr(),
        )
    };
    let destroy_string = unsafe {
        dlsym(
            process,
            c"_ZNSt3__112basic_stringIcNS_11char_traitsIcEENS_9allocatorIcEEED1Ev".as_ptr(),
        )
    };
    if assign.is_null() || destroy_string.is_null() {
        return Err("macOS libc++ string functions unavailable");
    }
    let assign: unsafe extern "C" fn(*mut CppString, *const u8, usize) -> *mut CppString =
        unsafe { std::mem::transmute(assign) };
    let destroy_string: unsafe extern "C" fn(*mut CppString) =
        unsafe { std::mem::transmute(destroy_string) };
    let construct_file: unsafe extern "C" fn(*mut FileStorage, *const CppString) =
        unsafe { std::mem::transmute(target.constructor) };
    let load: unsafe extern "C" fn(*mut c_void, *mut FileStorage) -> VendorReturn =
        unsafe { std::mem::transmute(target.activate) };
    let destroy_file: unsafe extern "C" fn(*mut FileStorage) =
        unsafe { std::mem::transmute(target.destructor) };

    let mut path_string = CppString([MaybeUninit::new(0); 24]);
    let mut file = FileStorage([MaybeUninit::uninit(); 160]);
    unsafe {
        if assign(&mut path_string, path.as_ptr(), path.len()) != &raw mut path_string {
            destroy_string(&mut path_string);
            return Err("macOS libc++ string assignment refused the path");
        }
        construct_file(&mut file, &path_string);
        let mut result = load(target.receiver, &mut file);
        destroy_file(&mut file);
        destroy_string(&mut path_string);
        destroy_string(&mut result.message);
        Ok(result.status)
    }
}

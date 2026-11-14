//! Fixed public-interface preflight for an already bound Inserts folder.
pub mod collection;
use crate::{Error, ffi, guard};
use ffi::{CStringRepr, Owned, Uid, Variant};
use std::{
    ffi::c_void,
    ptr,
    ptr::NonNull,
    thread::{self, ThreadId},
};

const MEDIATOR: Uid = Uid {
    a: 0x25e340af,
    b: 0xd6e6,
    c: 0x49cf,
    d: [0xb2, 0x58, 0x87, 0x47, 0x6e, 0x4a, 0x27, 0x3a],
};
const COLLECTOR: Uid = Uid {
    a: 0x30802b95,
    b: 0x7bce,
    c: 0x481d,
    d: [0xa1, 0x38, 0x4b, 0xd0, 0x65, 0x1c, 0x71, 0xbb],
};
const REGISTRY: Uid = Uid {
    a: 0x6c19f466,
    b: 0xc0c3,
    c: 0x4c91,
    d: [0x8b, 0x1e, 0x5e, 0x00, 0x29, 0xc6, 0x74, 0xa8],
};
const HANDLER: Uid = Uid {
    a: 0x4351b0e8,
    b: 0xa282,
    c: 0x45ae,
    d: [0xbf, 0x9c, 0x1a, 0x9d, 0x44, 0xa4, 0x5b, 0x58],
};
pub const MAX_METADATA_ATTRIBUTES: u32 = 64;
pub const MAX_METADATA_TEXT: usize = 256;
const REQUIRED_FLAGS: u32 = 3;

/// Every field transfers one owned reference to the exact named CCL interface.
pub struct FolderInputs {
    pub folder: NonNull<c_void>,
    pub registry: NonNull<c_void>,
    pub allocator: NonNull<c_void>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderMetadata {
    pub attribute_count: u32,
    pub class_name: Option<String>,
    pub class_vendor: Option<String>,
    pub class_category: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderObservation {
    pub target_flags: u32,
    pub metadata: FolderMetadata,
}

/// Retained preflight handles only. No preset, file, URL or mutation is exposed.
///
/// ```compile_fail
/// use fender_native_preset_api::folder::FolderPresetRoute;
/// fn send(route: FolderPresetRoute) { std::thread::spawn(move || drop(route)); }
/// ```
/// ```compile_fail
/// use fender_native_preset_api::folder::FolderPresetRoute;
/// fn sync<T: Sync>() {} fn check() { sync::<FolderPresetRoute>(); }
/// ```
pub struct FolderPresetRoute {
    folder: Owned,
    folder_identity: Owned,
    mediator: Owned,
    collector: Owned,
    registry: Owned,
    allocator: Owned,
    handler_source: Owned,
    handler: Owned,
    handler_identity: Owned,
    thread: ThreadId,
}

fn query<G: FnMut() -> Result<(), String>>(
    source: &Owned,
    iid: &Uid,
    g: &mut G,
) -> Result<Owned, Error> {
    guard(g)?;
    let result = source.query(iid);
    if let Err(error) = guard(g) {
        drop(result);
        guard(g)?;
        return Err(error);
    }
    result
}
fn same_interface<G: FnMut() -> Result<(), String>>(
    source: &Owned,
    iid: &Uid,
    g: &mut G,
) -> Result<Owned, Error> {
    let result = (|| {
        let original = query(source, &ffi::UNKNOWN, g)?;
        let interface = query(source, iid, g)?;
        let identity = query(&interface, &ffi::UNKNOWN, g)?;
        if identity.raw() != original.raw() {
            return Err(Error::Changed);
        }
        Ok(interface)
    })();
    if let Err(error) = guard(g) {
        drop(result);
        guard(g)?;
        return Err(error);
    }
    result
}
fn borrowed_handler(registry: &Owned, folder: &Owned) -> *mut c_void {
    let get: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
        unsafe { ffi::method(registry.raw(), 6) };
    unsafe { get(registry.raw(), folder.raw()) }
}

impl FolderPresetRoute {
    /// Consumes the three references on success and failure.
    ///
    /// # Safety
    /// Inputs must be live CCL 5.1.2 interfaces on the validated host GUI thread.
    /// The folder is already independently bound to the current service,
    /// document and unique channel. Registry is IPresetFileRegistry and allocator
    /// is IClassAllocator. The guard revalidates that binding after callbacks.
    /// Every call and final native release must stay on this creator thread.
    pub unsafe fn discover_owned<G: FnMut() -> Result<(), String>>(
        inputs: FolderInputs,
        g: &mut G,
    ) -> Result<Self, Error> {
        let folder = unsafe { Owned::take(inputs.folder) };
        let registry = unsafe { Owned::take(inputs.registry) };
        let allocator = unsafe { Owned::take(inputs.allocator) };
        let result = (|| {
            guard(g)?;
            let folder_identity = query(&folder, &ffi::UNKNOWN, g)?;
            let mediator = same_interface(&folder, &MEDIATOR, g)?;
            let collector = same_interface(&folder, &COLLECTOR, g)?;
            let typed_registry = same_interface(&registry, &REGISTRY, g)?;
            drop(registry);
            guard(g)?;
            let pointer = borrowed_handler(&typed_registry, &folder);
            let source = unsafe { Owned::hold(pointer) };
            guard(g)?;
            let handler_source = source?;
            let handler = same_interface(&handler_source, &HANDLER, g)?;
            let handler_identity = query(&handler_source, &ffi::UNKNOWN, g)?;
            let route = Self {
                folder,
                folder_identity,
                mediator,
                collector,
                registry: typed_registry,
                allocator,
                handler_source,
                handler,
                handler_identity,
                thread: thread::current().id(),
            };
            route.observe(g)?;
            Ok(route)
        })();
        // All failure-path owners have been released before this final witness.
        // The successful route retains its owners through caller publication.
        if let Err(error) = guard(g) {
            drop(result);
            guard(g)?;
            return Err(error);
        }
        result
    }
    fn thread(&self) -> Result<(), Error> {
        if thread::current().id() == self.thread {
            Ok(())
        } else {
            Err(Error::WrongThread)
        }
    }
    pub fn check<G: FnMut() -> Result<(), String>>(&self, g: &mut G) -> Result<(), Error> {
        self.thread()?;
        guard(g)?;
        let result = (|| {
            for object in [&self.folder, &self.mediator, &self.collector] {
                let identity = query(object, &ffi::UNKNOWN, g)?;
                if identity.raw() != self.folder_identity.raw() {
                    return Err(Error::Changed);
                }
            }
            let identity = query(&self.handler, &ffi::UNKNOWN, g)?;
            if identity.raw() != self.handler_identity.raw() {
                return Err(Error::Changed);
            }
            Ok(())
        })();
        guard(g)?;
        result?;
        let attached = borrowed_handler(&self.registry, &self.folder) == self.handler_source.raw();
        guard(g)?;
        if attached {
            Ok(())
        } else {
            Err(Error::Changed)
        }
    }
    fn flags<G: FnMut() -> Result<(), String>>(&self, g: &mut G) -> Result<u32, Error> {
        guard(g)?;
        let get: unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32 =
            unsafe { ffi::method(self.handler.raw(), 11) };
        let flags = unsafe { get(self.handler.raw(), self.folder.raw()) };
        guard(g)?;
        if flags < 0 || flags as u32 & REQUIRED_FLAGS != REQUIRED_FLAGS {
            return Err(Error::Unavailable(
                "folder handler cannot import and export",
            ));
        }
        Ok(flags as u32)
    }
    fn metadata_attributes<G: FnMut() -> Result<(), String>>(
        &self,
        g: &mut G,
    ) -> Result<(Owned, FolderMetadata), Error> {
        let result = (|| {
            guard(g)?;
            let create: unsafe extern "C" fn(
                *mut c_void,
                *const Uid,
                *const Uid,
                *mut *mut c_void,
            ) -> i32 = unsafe { ffi::method(self.allocator.raw(), 3) };
            let mut out = ptr::null_mut();
            let status = unsafe {
                create(
                    self.allocator.raw(),
                    &ffi::ATTRIBUTES_CLASS,
                    &ffi::ATTRIBUTES,
                    &mut out,
                )
            };
            let attributes =
                unsafe { Owned::from_success(status, out, "folder Attributes factory failed") };
            guard(g)?;
            let attributes = attributes?;
            let get: unsafe extern "C" fn(*mut c_void, *mut c_void) -> u8 =
                unsafe { ffi::method(self.mediator.raw(), 3) };
            let available = unsafe { get(self.mediator.raw(), attributes.raw()) };
            guard(g)?;
            if available != 1 {
                return Err(Error::Unavailable("folder preset metadata unavailable"));
            }
            let count: unsafe extern "C" fn(*mut c_void) -> i32 =
                unsafe { ffi::method(attributes.raw(), 5) };
            let before = unsafe { count(attributes.raw()) };
            guard(g)?;
            if before < 0 || before as u32 > MAX_METADATA_ATTRIBUTES {
                return Err(Error::InvalidData("folder metadata exceeds bound"));
            }
            let result = FolderMetadata {
                attribute_count: before as u32,
                class_name: text_attribute(&attributes, c"Class:Name".as_ptr(), g)?,
                class_vendor: text_attribute(&attributes, c"Class:Vendor".as_ptr(), g)?,
                class_category: text_attribute(&attributes, c"Class:Category".as_ptr(), g)?,
            };
            let after = unsafe { count(attributes.raw()) };
            guard(g)?;
            if after != before {
                return Err(Error::Changed);
            }
            Ok((attributes, result))
        })();
        if let Err(error) = guard(g) {
            drop(result);
            guard(g)?;
            return Err(error);
        }
        result
    }
    fn metadata<G: FnMut() -> Result<(), String>>(
        &self,
        g: &mut G,
    ) -> Result<FolderMetadata, Error> {
        let result = (|| {
            let (attributes, metadata) = self.metadata_attributes(g)?;
            drop(attributes);
            guard(g)?;
            Ok(metadata)
        })();
        guard(g)?;
        result
    }
    pub fn observe<G: FnMut() -> Result<(), String>>(
        &self,
        g: &mut G,
    ) -> Result<FolderObservation, Error> {
        self.check(g)?;
        let target_flags = self.flags(g)?;
        let metadata = self.metadata(g)?;
        if self.flags(g)? != target_flags {
            return Err(Error::Changed);
        }
        self.check(g)?;
        Ok(FolderObservation {
            target_flags,
            metadata,
        })
    }
}

fn text_attribute<G: FnMut() -> Result<(), String>>(
    attributes: &Owned,
    name: *const std::ffi::c_char,
    g: &mut G,
) -> Result<Option<String>, Error> {
    let result = (|| {
        guard(g)?;
        let mut value = Variant::empty();
        let id = CStringRepr {
            text: name,
            owner: ptr::null_mut(),
        };
        let get: unsafe extern "C" fn(*mut c_void, *mut Variant, *const CStringRepr) -> u8 =
            unsafe { ffi::method(attributes.raw(), 10) };
        let found = unsafe { get(attributes.raw(), &mut value, &id) };
        if found != 1 || !matches!(value.tag, 3 | 19) {
            let empty = value.tag == 0;
            value.tag = 0;
            guard(g)?;
            return if found == 0 && empty {
                Ok(None)
            } else {
                Err(Error::InvalidData("folder metadata text is unavailable"))
            };
        }
        let text = unsafe { Owned::hold(value.payload as usize as *mut c_void) };
        guard(g)?;
        let text = copy_text(text?, g)?;
        drop(value);
        guard(g)?;
        if text.contains('\0') {
            return Err(Error::InvalidData("folder metadata contains NUL"));
        }
        Ok(Some(text))
    })();
    guard(g)?;
    result
}
fn copy_text<G: FnMut() -> Result<(), String>>(text: Owned, g: &mut G) -> Result<String, Error> {
    let result = (|| {
        guard(g)?;
        let length: unsafe extern "C" fn(*mut c_void) -> i32 =
            unsafe { ffi::method(text.raw(), 4) };
        let before = unsafe { length(text.raw()) };
        guard(g)?;
        if before < 0 || before as usize > MAX_METADATA_TEXT {
            return Err(Error::InvalidData("folder metadata text exceeds bound"));
        }
        let mut chars = vec![0xffff; before as usize + 1];
        let copy: unsafe extern "C" fn(*mut c_void, *mut u16, i32) -> i32 =
            unsafe { ffi::method(text.raw(), 8) };
        let status = unsafe { copy(text.raw(), chars.as_mut_ptr(), chars.len() as i32) };
        guard(g)?;
        let after = unsafe { length(text.raw()) };
        guard(g)?;
        if status != 0 || before != after || chars[before as usize] != 0 {
            return Err(Error::InvalidData("folder metadata text changed"));
        }
        let result = String::from_utf16(&chars[..before as usize])
            .map_err(|_| Error::InvalidData("folder metadata is not UTF-16"));
        drop(text);
        guard(g)?;
        result
    })();
    guard(g)?;
    result
}

#[cfg(test)]
mod tests;

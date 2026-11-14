//! Host-authored collection presets at an integrating-owner private destination.
use super::{FolderPresetRoute, query, same_interface};
use crate::{Error, ffi, guard};
use ffi::{Owned, StringRepr, Uid};
use std::{
    ffi::{CStr, CString, c_char, c_void},
    path::Path,
    ptr,
    ptr::NonNull,
    rc::Rc,
    thread::{self, ThreadId},
};

const URL: Uid = Uid {
    a: 0xbfca729d,
    b: 0x5097,
    c: 0x4b38,
    d: [0x9f, 0x15, 0x52, 0x13, 0xf9, 0x79, 0x00, 0xc4],
};
const COLLECTION: Uid = Uid {
    a: 0xf860434e,
    b: 0xe8de,
    c: 0x4307,
    d: [0x8d, 0xbd, 0xc9, 0x1a, 0x3c, 0x5f, 0x5b, 0xc6],
};
pub const MAX_COLLECTION_PRESETS: u32 = 128;
const MAX_PATH: usize = 2048;

#[derive(Debug)]
pub struct CollectionCallError {
    pub error: Error,
    /// The store or restore call began. False is not a filesystem rollback
    /// guarantee: the public handler may already have performed I/O.
    pub dispatched: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CollectionRestore {
    pub accepted: bool,
    pub verified: bool,
    pub part_count: u32,
}

/// Native URL only. The integrating owner retains filesystem authority.
pub struct PrivateDestination {
    url: Owned,
    identity: Owned,
    path: CString,
    thread: ThreadId,
}
impl PrivateDestination {
    /// Consumes one owned IUrl reference on success and failure.
    ///
    /// # Safety
    /// `empty_url` is a live CCL 5.1.2 IUrl on the validated GUI thread. `path`
    /// names a new, exclusive file in a server-owned private directory. It is
    /// never caller-supplied MCP data. The owner enforces symlink, file ownership,
    /// byte/hash and lifetime rules. Every release stays on this creator thread.
    pub unsafe fn from_owned_empty_url<G: FnMut() -> Result<(), String>>(
        empty_url: NonNull<c_void>,
        path: &str,
        g: &mut G,
    ) -> Result<Self, Error> {
        let prototype = unsafe { Owned::take(empty_url) };
        let result = (|| {
            validate_private_path(path)?;
            guard(g)?;
            let prototype = same_interface(&prototype, &URL, g)?;
            let empty: unsafe extern "C" fn(*mut c_void) -> u8 =
                unsafe { ffi::method(prototype.raw(), 6) };
            if unsafe { empty(prototype.raw()) } != 1 {
                return Err(Error::InvalidData("URL prototype is not empty"));
            }
            guard(g)?;
            let url = clone_url(&prototype, g)?;
            let path =
                CString::new(path).map_err(|_| Error::InvalidData("private path contains NUL"))?;
            let assign: unsafe extern "C" fn(*mut c_void, *const c_char, i32) -> u8 =
                unsafe { ffi::method(url.raw(), 28) };
            let accepted = unsafe { assign(url.raw(), path.as_ptr(), 1) };
            guard(g)?;
            if accepted != 1 {
                return Err(Error::InvalidData("private POSIX path was refused"));
            }
            let identity = query(&url, &ffi::UNKNOWN, g)?;
            let destination = Self {
                url,
                identity,
                path,
                thread: thread::current().id(),
            };
            destination.check(g)?;
            Ok(destination)
        })();
        drop(prototype);
        let result = result.and_then(|destination| {
            destination.check(g)?;
            Ok(destination)
        });
        finish(result, g)
    }
    fn check<G: FnMut() -> Result<(), String>>(&self, g: &mut G) -> Result<(), Error> {
        if thread::current().id() != self.thread {
            return Err(Error::WrongThread);
        }
        let result = (|| {
            let identity = query(&self.url, &ffi::UNKNOWN, g)?;
            if identity.raw() != self.identity.raw() {
                return Err(Error::Changed);
            }
            check_url(&self.url, &self.path, g)
        })();
        finish(result, g)
    }
}

/// A successful host store reopened through its exact handler. No state bytes
/// or paths are exposed. The owner must still verify and hash the private file.
///
/// ```compile_fail
/// use fender_native_preset_api::folder::collection::CollectionFile;
/// fn send(v:CollectionFile){std::thread::spawn(move || drop(v));}
/// ```
/// ```compile_fail
/// use fender_native_preset_api::folder::collection::CollectionFile;
/// fn sync<T:Sync>(){} fn check(){sync::<CollectionFile>();}
/// ```
pub struct CollectionFile {
    route: Rc<FolderPresetRoute>,
    destination: PrivateDestination,
    preset: Owned,
    collection: Owned,
    identity: Owned,
    part_count: u32,
}
struct PresetOwner {
    preset: Owned,
    collection: Owned,
    identity: Owned,
    part_count: u32,
}
impl CollectionFile {
    /// Creates, stores exactly the bound folder once, then opens a fresh preset.
    /// No default handler, user-library notification or retry is performed.
    /// A false store can leave a partial private file. Success only witnesses
    /// host acceptance plus a bounded reopen, not graph/state completeness.
    pub fn capture<G: FnMut() -> Result<(), String>>(
        route: Rc<FolderPresetRoute>,
        destination: PrivateDestination,
        g: &mut G,
    ) -> Result<Self, CollectionCallError> {
        Self::capture_with_validation(route, destination, g, &mut || Ok(()))
    }
    /// Runs an integrating owner's private-file validation after an accepted
    /// store and before the first archive reopen. The callback must seal and
    /// verify the server-owned file's ownership, size and hash. It receives no
    /// native pointer, path or archive bytes from this API.
    ///
    /// Validation failure is dispatched and may leave partial private output.
    /// This post-store gate cannot cancel or impose a hard pre-write byte cap.
    pub fn capture_with_validation<
        G: FnMut() -> Result<(), String>,
        V: FnMut() -> Result<(), String>,
    >(
        route: Rc<FolderPresetRoute>,
        destination: PrivateDestination,
        g: &mut G,
        post_store: &mut V,
    ) -> Result<Self, CollectionCallError> {
        let mut dispatched = false;
        let result = (|| {
            route.check(g)?;
            destination.check(g)?;
            let (attributes, metadata) = route.metadata_attributes(g)?;
            if [
                &metadata.class_name,
                &metadata.class_vendor,
                &metadata.class_category,
            ]
            .iter()
            .any(|text| text.as_ref().is_none_or(|text| text.is_empty()))
            {
                return Err(Error::Unavailable("collection metadata is incomplete"));
            }
            route.flags(g)?;
            route.check(g)?;
            let create: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> *mut c_void =
                unsafe { ffi::method(route.handler.raw(), 10) };
            let created =
                unsafe { create(route.handler.raw(), destination.url.raw(), attributes.raw()) };
            let created = take_preset(created);
            guard(g)?;
            let created = created?;
            let created = inspect_preset(created, &destination, g)?;
            drop(attributes);
            guard(g)?;
            destination.check(g)?;
            route.flags(g)?;
            route.check(g)?;
            let store: unsafe extern "C" fn(*mut c_void, *mut c_void) -> u8 =
                unsafe { ffi::method(created.preset.raw(), 9) };
            dispatched = true;
            let accepted = unsafe { store(created.preset.raw(), route.folder.raw()) };
            guard(g)?;
            if accepted != 1 {
                return Err(Error::Unavailable("collection store did not succeed"));
            }
            drop(created);
            guard(g)?;
            destination.check(g)?;
            route.check(g)?;
            let validation = post_store();
            guard(g)?;
            validation.map_err(Error::Guard)?;
            destination.check(g)?;
            route.flags(g)?;
            route.check(g)?;
            let fresh = open_preset(&route, &destination, g)?;
            route.check(g)?;
            Ok(Self {
                route,
                destination,
                preset: fresh.preset,
                collection: fresh.collection,
                identity: fresh.identity,
                part_count: fresh.part_count,
            })
        })();
        finish(result, g).map_err(|error| CollectionCallError { error, dispatched })
    }
    pub fn part_count(&self) -> u32 {
        self.part_count
    }
    pub fn check<G: FnMut() -> Result<(), String>>(&self, g: &mut G) -> Result<(), Error> {
        self.route.check(g)?;
        self.destination.check(g)?;
        let result = (|| {
            for interface in [&self.preset, &self.collection] {
                let identity = query(interface, &ffi::UNKNOWN, g)?;
                if identity.raw() != self.identity.raw() {
                    return Err(Error::Changed);
                }
            }
            verify_preset_url(&self.preset, &self.destination, g)?;
            if count_presets(&self.collection, g)? != self.part_count {
                return Err(Error::Changed);
            }
            Ok(())
        })();
        finish(result, g)?;
        self.route.check(g)
    }
    /// Reads the file through a fresh preset before one exact-folder load.
    /// `dispatched=true` errors may follow partial host mutation. A true
    /// acceptance still requires independent graph/state verification.
    pub fn restore<G: FnMut() -> Result<(), String>>(
        &self,
        g: &mut G,
    ) -> Result<CollectionRestore, CollectionCallError> {
        let mut dispatched = false;
        let result = (|| {
            self.check(g)?;
            let fresh = open_preset(&self.route, &self.destination, g)?;
            if fresh.part_count != self.part_count {
                return Err(Error::Changed);
            }
            self.destination.check(g)?;
            self.route.flags(g)?;
            self.route.check(g)?;
            let restore: unsafe extern "C" fn(*mut c_void, *mut c_void) -> u8 =
                unsafe { ffi::method(fresh.preset.raw(), 10) };
            dispatched = true;
            let accepted = unsafe { restore(fresh.preset.raw(), self.route.folder.raw()) };
            guard(g)?;
            if accepted != 1 {
                return Err(Error::Unavailable("collection restore did not succeed"));
            }
            drop(fresh);
            guard(g)?;
            self.destination.check(g)?;
            self.route.check(g)?;
            Ok(CollectionRestore {
                accepted: true,
                verified: false,
                part_count: self.part_count,
            })
        })();
        finish(result, g).map_err(|error| CollectionCallError { error, dispatched })
    }
}
fn finish<T, G: FnMut() -> Result<(), String>>(
    result: Result<T, Error>,
    g: &mut G,
) -> Result<T, Error> {
    if let Err(error) = guard(g) {
        drop(result);
        guard(g)?;
        return Err(error);
    }
    result
}
fn validate_private_path(path: &str) -> Result<(), Error> {
    if path.len() >= MAX_PATH
        || path.contains('\0')
        || !path.starts_with('/')
        || path
            .split('/')
            .skip(1)
            .any(|c| c.is_empty() || c == "." || c == "..")
    {
        return Err(Error::InvalidData(
            "private path is not an absolute normalized POSIX file",
        ));
    }
    let name = Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or(Error::InvalidData("private filename unavailable"))?;
    let stem = name
        .strip_suffix(".multipreset")
        .ok_or(Error::InvalidData("private collection extension required"))?;
    if stem.is_empty()
        || stem.len() > 64
        || !stem
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(Error::InvalidData(
            "private filename must be an opaque bounded name",
        ));
    }
    Ok(())
}
fn clone_url<G: FnMut() -> Result<(), String>>(source: &Owned, g: &mut G) -> Result<Owned, Error> {
    let result = (|| {
        guard(g)?;
        let original = query(source, &ffi::UNKNOWN, g)?;
        let clone: unsafe extern "C" fn(*mut c_void, *mut *mut c_void) =
            unsafe { ffi::method(source.raw(), 3) };
        let mut out = ptr::null_mut();
        unsafe { clone(source.raw(), &mut out) };
        let cloned = NonNull::new(out)
            .map(|p| unsafe { Owned::take(p) })
            .ok_or(Error::Unavailable("URL clone returned null"));
        guard(g)?;
        let cloned = cloned?;
        let typed = same_interface(&cloned, &URL, g)?;
        let identity = query(&typed, &ffi::UNKNOWN, g)?;
        if original.raw() == identity.raw() {
            return Err(Error::InvalidData("URL clone aliases its prototype"));
        }
        drop(identity);
        drop(cloned);
        drop(original);
        guard(g)?;
        Ok(typed)
    })();
    finish(result, g)
}
fn check_url<G: FnMut() -> Result<(), String>>(
    url: &Owned,
    path: &CStr,
    g: &mut G,
) -> Result<(), Error> {
    guard(g)?;
    let scalar: unsafe extern "C" fn(*mut c_void) -> i32 = unsafe { ffi::method(url.raw(), 7) };
    if unsafe { scalar(url.raw()) } != 1 {
        return Err(Error::InvalidData("destination is not a file"));
    }
    guard(g)?;
    for (slot, wanted) in [(24, 1), (31, 1), (41, 0)] {
        let get: unsafe extern "C" fn(*mut c_void) -> u8 = unsafe { ffi::method(url.raw(), slot) };
        let observed = unsafe { get(url.raw()) };
        guard(g)?;
        if observed != wanted {
            return Err(Error::InvalidData(
                "destination URL is not a plain local absolute file",
            ));
        }
    }
    let protocol = url_text(url, 10, g)?;
    let host = url_text(url, 12, g)?;
    if protocol != "file" || (!host.is_empty() && host != "localhost") {
        return Err(Error::InvalidData("destination URL is not local"));
    }
    let copy: unsafe extern "C" fn(*mut c_void, *mut c_char, i32) -> u8 =
        unsafe { ffi::method(url.raw(), 27) };
    let mut buffer = vec![0x7f_u8; MAX_PATH];
    let copied = unsafe { copy(url.raw(), buffer.as_mut_ptr().cast(), MAX_PATH as i32) };
    guard(g)?;
    if copied != 1 {
        return Err(Error::InvalidData("destination POSIX path unavailable"));
    }
    let observed = CStr::from_bytes_until_nul(&buffer)
        .map_err(|_| Error::InvalidData("destination POSIX path is not terminated"))?;
    if observed != path {
        return Err(Error::Changed);
    }
    Ok(())
}
fn url_text<G: FnMut() -> Result<(), String>>(
    url: &Owned,
    slot: usize,
    g: &mut G,
) -> Result<String, Error> {
    guard(g)?;
    let get: unsafe extern "C" fn(*mut c_void) -> *const StringRepr =
        unsafe { ffi::method(url.raw(), slot) };
    let reference = unsafe { get(url.raw()) };
    let reference = unsafe { reference.as_ref() }.ok_or(Error::InvalidData("missing URL text"))?;
    let owner = unsafe { Owned::hold(reference.object) };
    guard(g)?;
    super::copy_text(owner?, g)
}
fn take_preset(pointer: *mut c_void) -> Result<Owned, Error> {
    NonNull::new(pointer)
        .map(|p| unsafe { Owned::take(p) })
        .ok_or(Error::Unavailable("collection preset unavailable"))
}
fn inspect_preset<G: FnMut() -> Result<(), String>>(
    preset: Owned,
    destination: &PrivateDestination,
    g: &mut G,
) -> Result<PresetOwner, Error> {
    let result = (|| {
        let original = preset;
        let preset = same_interface(&original, &ffi::PRESET, g)?;
        drop(original);
        guard(g)?;
        let collection = same_interface(&preset, &COLLECTION, g)?;
        let identity = query(&preset, &ffi::UNKNOWN, g)?;
        verify_preset_url(&preset, destination, g)?;
        let part_count = count_presets(&collection, g)?;
        Ok(PresetOwner {
            preset,
            collection,
            identity,
            part_count,
        })
    })();
    finish(result, g)
}
fn verify_preset_url<G: FnMut() -> Result<(), String>>(
    preset: &Owned,
    destination: &PrivateDestination,
    g: &mut G,
) -> Result<(), Error> {
    let result = (|| {
        let cloned = clone_url(&destination.url, g)?;
        let get: unsafe extern "C" fn(*mut c_void, *mut c_void) -> u8 =
            unsafe { ffi::method(preset.raw(), 7) };
        let accepted = unsafe { get(preset.raw(), cloned.raw()) };
        guard(g)?;
        if accepted != 1 {
            return Err(Error::InvalidData("collection preset URL unavailable"));
        }
        check_url(&cloned, &destination.path, g)
    })();
    finish(result, g)
}
fn count_presets<G: FnMut() -> Result<(), String>>(
    collection: &Owned,
    g: &mut G,
) -> Result<u32, Error> {
    guard(g)?;
    let get: unsafe extern "C" fn(*mut c_void) -> i32 = unsafe { ffi::method(collection.raw(), 3) };
    let count = unsafe { get(collection.raw()) };
    guard(g)?;
    if !(0..=MAX_COLLECTION_PRESETS as i32).contains(&count) {
        return Err(Error::InvalidData("collection part count exceeds bound"));
    }
    Ok(count as u32)
}
fn open_preset<G: FnMut() -> Result<(), String>>(
    route: &FolderPresetRoute,
    destination: &PrivateDestination,
    g: &mut G,
) -> Result<PresetOwner, Error> {
    let result = (|| {
        route.check(g)?;
        destination.check(g)?;
        let open: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> *mut c_void =
            unsafe { ffi::method(route.handler.raw(), 9) };
        let preset = unsafe { open(route.handler.raw(), destination.url.raw(), ptr::null_mut()) };
        let preset = take_preset(preset);
        guard(g)?;
        inspect_preset(preset?, destination, g)
    })();
    finish(result, g)
}

#[cfg(test)]
mod tests;

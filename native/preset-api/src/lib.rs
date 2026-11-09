//! Owned local preset catalogs and fixed folder preflight. No IPC or file writes.
#![deny(unsafe_op_in_unsafe_fn)]
mod ffi;
pub mod folder;
mod list;
use ffi::{Owned, StringRepr};
use std::{
    ffi::c_void,
    ptr,
    ptr::NonNull,
    thread::{self, ThreadId},
};

pub const MAX_PRESETS: usize = 4096;
pub const MAX_PAGE: u32 = 128;
pub const CCL_ABI_VERSION: &str = "5.1.2";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    WrongThread,
    Unavailable(&'static str),
    InvalidData(&'static str),
    Changed,
    Guard(String),
    OutOfRange,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Availability {
    LocalReadOnly,
    LocalWritable,
    MissingLocalFile,
    NoLocation,
    UnsupportedProtocol(String),
    NonLocalHost,
    InvalidLocalLocation,
    ClassMismatch,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetInfo {
    pub index: u32,
    pub name: String,
    pub class_key: String,
    pub read_only: bool,
    pub availability: Availability,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub entries: Vec<PresetInfo>,
    pub observed_count: u32,
    pub offset: u32,
    pub next: Option<u32>,
    pub catalog_complete: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Recall {
    pub accepted: bool,
    pub verified: bool,
}
#[derive(Debug)]
pub struct RecallError {
    pub error: Error,
    pub dispatched: bool,
}

/// Each pointer transfers one owned reference to the exact interface named.
pub struct Inputs {
    pub manager: NonNull<c_void>,
    pub mediator: NonNull<c_void>,
    pub data_target: NonNull<c_void>,
    pub allocator: NonNull<c_void>,
    pub empty_url: NonNull<c_void>,
    pub file_system: NonNull<c_void>,
}
struct Services {
    manager: Owned,
    mediator: Owned,
    data_target: Owned,
    allocator: Owned,
    empty_url: Owned,
    file_system: Owned,
}
struct Entry {
    info: PresetInfo,
    preset: Owned,
    url: Option<Owned>,
}
struct Target {
    handle: Owned,
    identity: Owned,
}
/// One fixed target's catalog and references, confined to the verified GUI thread.
///
/// ```compile_fail
/// use fender_native_preset_api::Catalog;
/// fn send(c:Catalog){ std::thread::spawn(move || drop(c)); }
/// ```
/// ```compile_fail
/// use fender_native_preset_api::Catalog;
/// fn sync<T:Sync>(){} fn check(){sync::<Catalog>();}
/// ```
pub struct Catalog {
    services: Services,
    target: Target,
    class_key: String,
    entries: Vec<Entry>,
    complete: bool,
    thread: ThreadId,
}
fn guard<G: FnMut() -> Result<(), String>>(g: &mut G) -> Result<(), Error> {
    g().map_err(Error::Guard)
}

impl Services {
    fn attributes(&self) -> Result<Owned, Error> {
        let create: unsafe extern "C" fn(
            *mut c_void,
            *const ffi::Uid,
            *const ffi::Uid,
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
        unsafe { Owned::from_success(status, out, "Attributes factory failed") }
    }
    fn meta(&self) -> Result<(Owned, String), Error> {
        let attributes = self.attributes()?;
        let get: unsafe extern "C" fn(*mut c_void, *mut c_void) -> u8 =
            unsafe { ffi::method(self.mediator.raw(), 3) };
        if unsafe { get(self.mediator.raw(), attributes.raw()) } == 0 {
            return Err(Error::Unavailable("target preset metadata unavailable"));
        }
        let key = ffi::class_key(&attributes)?;
        Ok((attributes, key))
    }
    fn target(&self) -> Result<Target, Error> {
        let get: unsafe extern "C" fn(*mut c_void) -> *mut c_void =
            unsafe { ffi::method(self.mediator.raw(), 4) };
        let object = unsafe { Owned::hold(get(self.mediator.raw())) }?;
        let identity = object.query(&ffi::UNKNOWN)?;
        Ok(Target {
            handle: object,
            identity,
        })
    }
    fn url(&self, preset: &Owned) -> Result<Option<Owned>, Error> {
        let clone: unsafe extern "C" fn(*mut c_void, *mut *mut c_void) =
            unsafe { ffi::method(self.empty_url.raw(), 3) };
        let mut out = ptr::null_mut();
        unsafe { clone(self.empty_url.raw(), &mut out) };
        let url = NonNull::new(out)
            .map(|p| unsafe { Owned::take(p) })
            .ok_or(Error::Unavailable("URL clone returned null"))?;
        let get: unsafe extern "C" fn(*mut c_void, *mut c_void) -> u8 =
            unsafe { ffi::method(preset.raw(), 7) };
        if unsafe { get(preset.raw(), url.raw()) } == 0 {
            Ok(None)
        } else {
            Ok(Some(url))
        }
    }
    fn location(&self, url: &Owned, read_only: bool) -> Result<Availability, Error> {
        let get: unsafe extern "C" fn(*mut c_void) -> *const StringRepr =
            unsafe { ffi::method(url.raw(), 10) };
        let protocol = ffi::copy_string(unsafe { get(url.raw()) }, 64)?;
        if protocol != "file" {
            return Ok(Availability::UnsupportedProtocol(protocol));
        }
        let host: unsafe extern "C" fn(*mut c_void) -> *const StringRepr =
            unsafe { ffi::method(url.raw(), 12) };
        let host = ffi::copy_string(unsafe { host(url.raw()) }, 256)?;
        if !host.is_empty() && host != "localhost" {
            return Ok(Availability::NonLocalHost);
        }
        let native: unsafe extern "C" fn(*mut c_void) -> u8 = unsafe { ffi::method(url.raw(), 24) };
        let absolute: unsafe extern "C" fn(*mut c_void) -> u8 =
            unsafe { ffi::method(url.raw(), 31) };
        let kind: unsafe extern "C" fn(*mut c_void) -> i32 = unsafe { ffi::method(url.raw(), 7) };
        if unsafe { native(url.raw()) } == 0
            || unsafe { absolute(url.raw()) } == 0
            || unsafe { kind(url.raw()) } != 1
        {
            return Ok(Availability::InvalidLocalLocation);
        }
        let exists: unsafe extern "C" fn(*mut c_void, *mut c_void) -> u8 =
            unsafe { ffi::method(self.file_system.raw(), 4) };
        if unsafe { exists(self.file_system.raw(), url.raw()) } == 0 {
            return Ok(Availability::MissingLocalFile);
        }
        Ok(if read_only {
            Availability::LocalReadOnly
        } else {
            Availability::LocalWritable
        })
    }
    fn entry(&self, index: u32, preset: Owned, class_key: &str) -> Result<Entry, Error> {
        let name: unsafe extern "C" fn(*mut c_void) -> *const StringRepr =
            unsafe { ffi::method(preset.raw(), 5) };
        let name = ffi::copy_string(unsafe { name(preset.raw()) }, 512)?;
        let read_only: unsafe extern "C" fn(*mut c_void) -> u8 =
            unsafe { ffi::method(preset.raw(), 3) };
        let read_only = unsafe { read_only(preset.raw()) } != 0;
        let meta: unsafe extern "C" fn(*mut c_void) -> *mut c_void =
            unsafe { ffi::method(preset.raw(), 6) };
        let metadata = unsafe { Owned::hold(meta(preset.raw())) }?;
        let key = ffi::class_key(&metadata)?;
        let url = self.url(&preset)?;
        let availability = if key != class_key {
            Availability::ClassMismatch
        } else if let Some(ref url) = url {
            self.location(url, read_only)?
        } else {
            Availability::NoLocation
        };
        Ok(Entry {
            info: PresetInfo {
                index,
                name,
                class_key: key,
                read_only,
                availability,
            },
            preset,
            url,
        })
    }
}

impl Catalog {
    /// Takes owned host services and an empty URL prototype. No host is discovered.
    ///
    /// # Safety
    /// All six pointers must be the live CCL5.1.2 interfaces named by Inputs,
    /// each with one reference transferred. Validate the exact host ABI and GUI
    /// thread first. The mediator and Presets IDataTarget must belong to the same
    /// already-resolved Device slot. The guard must prove document, Device and
    /// slot/Presets attachments before and after callbacks. Every native call,
    /// callback and final release must remain on this GUI thread.
    pub unsafe fn discover_owned<G: FnMut() -> Result<(), String>>(
        inputs: Inputs,
        g: &mut G,
    ) -> Result<Self, Error> {
        let services = Services {
            manager: unsafe { Owned::take(inputs.manager) },
            mediator: unsafe { Owned::take(inputs.mediator) },
            data_target: unsafe { Owned::take(inputs.data_target) },
            allocator: unsafe { Owned::take(inputs.allocator) },
            empty_url: unsafe { Owned::take(inputs.empty_url) },
            file_system: unsafe { Owned::take(inputs.file_system) },
        };
        let result = (|| {
            guard(g)?;
            let empty: unsafe extern "C" fn(*mut c_void) -> u8 =
                unsafe { ffi::method(services.empty_url.raw(), 6) };
            if unsafe { empty(services.empty_url.raw()) } == 0 {
                return Err(Error::InvalidData("URL prototype must be empty"));
            }
            let target = services.target()?;
            let (attributes, class_key) = services.meta()?;
            guard(g)?;
            let get: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> *mut c_void =
                unsafe { ffi::method(services.manager.raw(), 7) };
            let pointer = unsafe { get(services.manager.raw(), attributes.raw(), ptr::null_mut()) };
            let presets = NonNull::new(pointer)
                .map(|p| unsafe { Owned::take(p) })
                .ok_or(Error::Unavailable("preset catalog unavailable"))?;
            drop(attributes);
            guard(g)?;
            let create: unsafe extern "C" fn(*mut c_void) -> *mut c_void =
                unsafe { ffi::method(presets.raw(), 3) };
            let iterator = NonNull::new(unsafe { create(presets.raw()) })
                .map(|p| unsafe { Owned::take(p) })
                .ok_or(Error::Unavailable("preset catalog iterator unavailable"))?;
            let done: unsafe extern "C" fn(*mut c_void) -> u8 =
                unsafe { ffi::method(iterator.raw(), 3) };
            let next: unsafe extern "C" fn(*mut c_void) -> *mut c_void =
                unsafe { ffi::method(iterator.raw(), 4) };
            let mut entries = Vec::new();
            while entries.len() < MAX_PRESETS && unsafe { done(iterator.raw()) } == 0 {
                guard(g)?;
                let object = unsafe { Owned::hold(next(iterator.raw())) }?;
                let preset = object.query(&ffi::PRESET)?;
                drop(object);
                let entry = services.entry(entries.len() as u32, preset, &class_key)?;
                guard(g)?;
                entries.push(entry);
            }
            let complete = unsafe { done(iterator.raw()) } != 0;
            drop(iterator);
            drop(presets);
            guard(g)?;
            let catalog = Self {
                services,
                target,
                class_key,
                entries,
                complete,
                thread: thread::current().id(),
            };
            catalog.check(g)?;
            Ok(catalog)
        })();
        if result.is_err() {
            guard(g)?;
        }
        result
    }
    fn thread(&self) -> Result<(), Error> {
        if self.thread == thread::current().id() {
            Ok(())
        } else {
            Err(Error::WrongThread)
        }
    }
    pub fn check<G: FnMut() -> Result<(), String>>(&self, g: &mut G) -> Result<(), Error> {
        self.thread()?;
        guard(g)?;
        let result = (|| {
            let target = self.services.target()?;
            if target.identity.raw() != self.target.identity.raw()
                || target.handle.raw() != self.target.handle.raw()
            {
                return Err(Error::Changed);
            }
            let (_, key) = self.services.meta()?;
            if key != self.class_key {
                return Err(Error::Changed);
            }
            Ok(())
        })();
        let result = result.and_then(|()| self.borrowed_target());
        guard(g)?;
        result
    }
    pub fn page<G: FnMut() -> Result<(), String>>(
        &self,
        offset: u32,
        limit: u32,
        g: &mut G,
    ) -> Result<Page, Error> {
        self.check(g)?;
        if limit == 0 || limit > MAX_PAGE || offset as usize > self.entries.len() {
            return Err(Error::OutOfRange);
        }
        let end = (offset as usize + limit as usize).min(self.entries.len());
        let entries = self.entries[offset as usize..end]
            .iter()
            .map(|entry| entry.info.clone())
            .collect();
        self.check(g)?;
        Ok(Page {
            entries,
            observed_count: self.entries.len() as u32,
            offset,
            next: (end < self.entries.len()).then_some(end as u32),
            catalog_complete: self.complete,
        })
    }
    pub fn recall<G: FnMut() -> Result<(), String>>(
        &self,
        index: u32,
        g: &mut G,
    ) -> Result<Recall, RecallError> {
        let mut dispatched = false;
        let result = (|| {
            self.check(g)?;
            let entry = self.entries.get(index as usize).ok_or(Error::OutOfRange)?;
            if !matches!(
                entry.info.availability,
                Availability::LocalReadOnly | Availability::LocalWritable
            ) {
                return Err(Error::Unavailable("preset is not an available local file"));
            }
            self.check_entry(entry, g)?;
            let list = list::one(&entry.preset);
            let can: unsafe extern "C" fn(
                *mut c_void,
                *mut c_void,
                *mut c_void,
                *mut c_void,
                i32,
            ) -> u8 = unsafe { ffi::method(self.services.data_target.raw(), 3) };
            let allowed = unsafe {
                can(
                    self.services.data_target.raw(),
                    list.raw(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    -1,
                )
            } != 0;
            if !allowed {
                drop(list);
                self.check(g)?;
                return Ok(Recall {
                    accepted: false,
                    verified: false,
                });
            }
            self.check(g)?;
            self.check_entry(entry, g)?;
            let insert: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, i32) -> u8 =
                unsafe { ffi::method(self.services.data_target.raw(), 4) };
            self.borrowed_target()?;
            guard(g)?;
            dispatched = true;
            let accepted = unsafe {
                insert(
                    self.services.data_target.raw(),
                    list.raw(),
                    ptr::null_mut(),
                    -1,
                )
            } != 0;
            drop(list);
            self.check(g)?;
            Ok(Recall {
                accepted,
                verified: false,
            })
        })();
        result.map_err(|error| RecallError { error, dispatched })
    }
    fn check_entry<G: FnMut() -> Result<(), String>>(
        &self,
        entry: &Entry,
        g: &mut G,
    ) -> Result<(), Error> {
        let current = self.services.entry(
            entry.info.index,
            unsafe { Owned::hold(entry.preset.raw()) }?,
            &self.class_key,
        )?;
        let same = if let (Some(a), Some(b)) = (&entry.url, &current.url) {
            let equals: unsafe extern "C" fn(*mut c_void, *mut c_void, u8) -> u8 =
                unsafe { ffi::method(a.raw(), 5) };
            (unsafe { equals(a.raw(), b.raw(), 1) }) != 0
        } else {
            false
        };
        let valid = current.info == entry.info && same;
        drop(current);
        guard(g)?;
        if valid { Ok(()) } else { Err(Error::Changed) }
    }
    fn borrowed_target(&self) -> Result<(), Error> {
        let get: unsafe extern "C" fn(*mut c_void) -> *mut c_void =
            unsafe { ffi::method(self.services.mediator.raw(), 4) };
        if unsafe { get(self.services.mediator.raw()) } == self.target.handle.raw() {
            Ok(())
        } else {
            Err(Error::Changed)
        }
    }
}

#[cfg(test)]
mod tests;

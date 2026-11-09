//! Exact-device External Effect editor resolution. No opening, setters or IPC.
#![deny(unsafe_op_in_unsafe_fn)]
mod ffi;
use ffi::{CONTROLLER, NODE, OBJECT, Owned, UNKNOWN, VIEW};
pub use ffi::{EXTERNAL_MONO, EXTERNAL_STEREO, Uid};
use std::{
    ffi::{c_char, c_void},
    ptr::NonNull,
    thread::{self, ThreadId},
};

pub const MAX_WINDOWS: i32 = 64;
pub const MAX_VIEWS: usize = 128;
pub const MAX_DEPTH: usize = 16;
const PATH: [usize; 5] = [0, 0, 0, 0, 1];

#[derive(Clone, Copy)]
pub struct GuiApi {
    pub get_desktop: unsafe extern "C" fn() -> *mut c_void,
    pub get_constant_string: unsafe extern "C" fn(*const c_char) -> *const c_void,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceMatch {
    CanonicalIUnknown,
    StableNodeUid,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Identity {
    pub device_uid: Uid,
    pub device_class: Uid,
    pub matched_by: DeviceMatch,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    WrongThread,
    UnsupportedDevice,
    InvalidIdentity,
    InvalidMetadata,
    MissingInterface,
    MissingObject,
    MissingConstant,
    MissingDesktop,
    WindowLimit,
    MissingEditor,
    AmbiguousEditor,
    MissingExternalController,
    ViewLimit,
    MissingViewPath,
    Stale,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
struct Device {
    canonical: Owned,
    node: Owned,
    uid: Uid,
    class: Uid,
}
impl Device {
    fn new(object: &Owned) -> Result<Self, Error> {
        let canonical = object.query(&UNKNOWN)?;
        let node = object.query(&NODE)?;
        let uid = ffi::uid(&node, 4)?;
        let class = ffi::uid(&node, 5)?;
        if uid.is_empty() || class.is_empty() {
            return Err(Error::InvalidIdentity);
        }
        Ok(Self {
            canonical,
            node,
            uid,
            class,
        })
    }
    fn matches(&self, other: &Self) -> Option<DeviceMatch> {
        if self.canonical.raw() == other.canonical.raw() {
            Some(DeviceMatch::CanonicalIUnknown)
        } else if self.uid == other.uid && self.class == other.class {
            Some(DeviceMatch::StableNodeUid)
        } else {
            None
        }
    }
}
struct Frame {
    controller: Owned,
    component_identity: Owned,
    component_object: Owned,
    component: Owned,
    views: Vec<Owned>,
    device: Device,
    device_ref: Owned,
    slot: Owned,
    editor_node: Owned,
    editor_identity: Owned,
    editor: Owned,
    window_identity: Owned,
    window: Owned,
    windows: i32,
    matched_by: DeviceMatch,
}
/// Retains the exact editor/device/controller identities for one synchronous request.
///
/// ```compile_fail
/// use fender_native_editor_api::ExternalEditorLease;
/// fn worker(lease:ExternalEditorLease){std::thread::spawn(move||drop(lease));}
/// ```
/// ```compile_fail
/// use fender_native_editor_api::ExternalEditorLease;
/// fn sync<T:Sync>(){}
/// sync::<ExternalEditorLease>();
/// ```
pub struct ExternalEditorLease {
    frame: Frame,
    expected: Device,
    _expected_ref: Owned,
    gui: GuiApi,
    thread: ThreadId,
}

/// Resolve an already-open editor for an exact, already resolved native Device.
///
/// # Safety
/// Consumes one owned IUnknown reference, including on error. Before any call,
/// validate the exact 8.1.1 arm64 main/cclgui UUIDs and GUI thread. The supplied
/// functions must be valid CCL5.1.2 APIs. Keep runtime and document/model/service
/// context alive through the lease's Drop, validate them around use, serialize
/// requests and prevent reentrant writes. No native exception may unwind here.
pub unsafe fn resolve_external_owned(
    expected_device: NonNull<c_void>,
    gui: GuiApi,
) -> Result<ExternalEditorLease, Error> {
    let expected_ref = unsafe { Owned::take(expected_device) };
    let expected = Device::new(&expected_ref)?;
    if expected.class != EXTERNAL_MONO && expected.class != EXTERNAL_STEREO {
        return Err(Error::UnsupportedDevice);
    }
    let frame = resolve(&expected, gui)?;
    let lease = ExternalEditorLease {
        frame,
        expected,
        _expected_ref: expected_ref,
        gui,
        thread: thread::current().id(),
    };
    lease.check()?;
    Ok(lease)
}
impl ExternalEditorLease {
    pub fn identity(&self) -> Identity {
        Identity {
            device_uid: self.expected.uid,
            device_class: self.expected.class,
            matched_by: self.frame.matched_by,
        }
    }
    /// Borrowed IController; valid only while the lease and caller context live.
    /// After check, the caller makes final borrowed document/model/Device
    /// comparisons and its service-active atomic check before dispatch.
    pub fn controller(&self) -> NonNull<c_void> {
        NonNull::new(self.frame.controller.raw()).expect("owned nonnull controller")
    }
    pub fn check(&self) -> Result<(), Error> {
        if self.thread != thread::current().id() {
            return Err(Error::WrongThread);
        }
        let fresh = resolve(&self.expected, self.gui)?;
        let same = fresh.window_identity.raw() == self.frame.window_identity.raw()
            && fresh.editor_identity.raw() == self.frame.editor_identity.raw()
            && fresh.device.canonical.raw() == self.frame.device.canonical.raw()
            && fresh.component_identity.raw() == self.frame.component_identity.raw()
            && fresh.controller.query(&UNKNOWN)?.raw()
                == self.frame.controller.query(&UNKNOWN)?.raw();
        // Release validation temporaries before the final pinned relationship
        // checks; release callbacks can close or reassign the editor.
        drop(fresh);
        if !same {
            return Err(Error::Stale);
        }
        self.check_pinned()
    }
    fn check_pinned(&self) -> Result<(), Error> {
        let desktop = desktop(self.gui)?;
        let count = window_count(desktop)?;
        if count != self.frame.windows {
            return Err(Error::Stale);
        }
        let at: unsafe extern "C" fn(*mut c_void, i32) -> *mut c_void =
            unsafe { ffi::method(desktop, 7) };
        let matches = (0..count)
            .filter(|&i| unsafe { at(desktop, i) } == self.frame.window.raw())
            .count();
        if matches != 1 {
            return Err(Error::Stale);
        }
        let f = &self.frame;
        if ffi::borrowed(f.window.raw(), 30) != f.editor.raw()
            || ffi::find_raw(&f.editor_node, self.gui, c"DeviceSlot")? != f.slot.raw()
            || ffi::find_raw(&f.slot, self.gui, c"Device")? != f.device_ref.raw()
        {
            return Err(Error::Stale);
        }
        if ffi::uid(&self.expected.node, 4)? != self.expected.uid
            || ffi::uid(&self.expected.node, 5)? != self.expected.class
            || ffi::uid(&f.device.node, 4)? != f.device.uid
            || ffi::uid(&f.device.node, 5)? != f.device.class
        {
            return Err(Error::Stale);
        }
        for pair in f.views.windows(2) {
            let children = ffi::borrowed(pair[0].raw(), 20);
            if children.is_null() {
                return Err(Error::Stale);
            }
            let is_child: unsafe extern "C" fn(*mut c_void, *mut c_void, u8) -> u8 =
                unsafe { ffi::method(children, 12) };
            if unsafe { is_child(children, pair[1].raw(), 0) } == 0 {
                return Err(Error::Stale);
            }
        }
        if ffi::class_from_object(&f.component_object)? != "ExternalEffectComponent"
            || ffi::borrowed(f.views.last().ok_or(Error::Stale)?.raw(), 32) != f.component.raw()
        {
            return Err(Error::Stale);
        }
        if window_count(desktop)? != count {
            return Err(Error::Stale);
        }
        // A later relationship getter can replace this window without changing
        // the total count. Recheck actual reachability after those native reads.
        let matches = (0..count)
            .filter(|&i| unsafe { at(desktop, i) } == f.window.raw())
            .count();
        if matches != 1 {
            return Err(Error::Stale);
        }
        Ok(())
    }
}
fn desktop(gui: GuiApi) -> Result<*mut c_void, Error> {
    let p = unsafe { (gui.get_desktop)() };
    if p.is_null() {
        Err(Error::MissingDesktop)
    } else {
        Ok(p)
    }
}
fn window_count(desktop: *mut c_void) -> Result<i32, Error> {
    let n = ffi::count(desktop, 6);
    if !(0..=MAX_WINDOWS).contains(&n) {
        Err(Error::WindowLimit)
    } else {
        Ok(n)
    }
}
fn resolve(expected: &Device, gui: GuiApi) -> Result<Frame, Error> {
    let desktop = desktop(gui)?;
    let n = window_count(desktop)?;
    let at: unsafe extern "C" fn(*mut c_void, i32) -> *mut c_void =
        unsafe { ffi::method(desktop, 7) };
    let mut hit = None;
    for index in 0..n {
        let window = unsafe { Owned::hold(at(desktop, index)) }?;
        let Ok(editor) = (unsafe { Owned::hold(ffi::borrowed(window.raw(), 30)) }) else {
            continue;
        };
        if ffi::class(&editor)? != "DeviceEditor" {
            continue;
        }
        let editor_node = editor.query(&NODE)?;
        let slot_raw = ffi::find_raw(&editor_node, gui, c"DeviceSlot")?;
        if slot_raw.is_null() {
            continue;
        }
        let slot = unsafe { Owned::hold(slot_raw) }?;
        let device_raw = ffi::find_raw(&slot, gui, c"Device")?;
        if device_raw.is_null() {
            continue;
        }
        let device_ref = unsafe { Owned::hold(device_raw) }?;
        let device = Device::new(&device_ref)?;
        if expected.class != device.class {
            continue;
        }
        let Some(matched_by) = expected.matches(&device) else {
            continue;
        };
        if hit.is_some() {
            return Err(Error::AmbiguousEditor);
        }
        let mut views = vec![window.query(&VIEW)?];
        let mut budget = 1;
        for index in PATH {
            let next = child_at(
                views.last().ok_or(Error::MissingViewPath)?,
                index,
                &mut budget,
            )?;
            views.push(next);
        }
        let component = unsafe {
            Owned::hold(ffi::borrowed(
                views.last().ok_or(Error::MissingViewPath)?.raw(),
                32,
            ))
        }
        .map_err(|_| Error::MissingExternalController)?;
        if ffi::class(&component)? != "ExternalEffectComponent" {
            return Err(Error::MissingExternalController);
        }
        let controller = component.query(&CONTROLLER)?;
        hit = Some(Frame {
            controller,
            component_identity: component.query(&UNKNOWN)?,
            component_object: component.query(&OBJECT)?,
            component,
            views,
            device,
            device_ref,
            slot,
            editor_identity: editor.query(&UNKNOWN)?,
            editor_node,
            editor,
            window_identity: window.query(&UNKNOWN)?,
            window,
            windows: n,
            matched_by,
        });
    }
    if window_count(desktop)? != n {
        return Err(Error::Stale);
    }
    hit.ok_or(Error::MissingEditor)
}
fn child_at(parent: &Owned, index: usize, budget: &mut usize) -> Result<Owned, Error> {
    if index >= MAX_VIEWS || *budget >= MAX_VIEWS || PATH.len() > MAX_DEPTH {
        return Err(Error::ViewLimit);
    }
    let children = ffi::borrowed(parent.raw(), 20);
    if children.is_null() {
        return Err(Error::MissingViewPath);
    }
    let raw = ffi::borrowed(children, 11);
    let iterator = unsafe { Owned::take(NonNull::new(raw).ok_or(Error::MissingViewPath)?) };
    let done: unsafe extern "C" fn(*mut c_void) -> u8 = unsafe { ffi::method(iterator.raw(), 3) };
    for i in 0..=index {
        *budget += 1;
        if *budget > MAX_VIEWS {
            return Err(Error::ViewLimit);
        }
        if unsafe { done(iterator.raw()) } != 0 {
            return Err(Error::MissingViewPath);
        }
        let child = unsafe { Owned::hold(ffi::borrowed(iterator.raw(), 4)) }?;
        if i == index {
            return child.query(&VIEW);
        }
    }
    Err(Error::MissingViewPath)
}

#[cfg(test)]
mod tests;

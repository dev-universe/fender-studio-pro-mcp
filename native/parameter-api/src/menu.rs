use crate::{
    Description, Error, ParameterBindingReadLease, RawParameter, Value,
    ffi::{self, Owned, Uid},
    value::Variant,
};
use std::{
    collections::HashSet,
    ffi::c_void,
    ptr::{self, NonNull},
};

#[path = "menu_provenance.rs"]
pub(crate) mod provenance;
pub use provenance::{LeafProvenance, MenuProvenance, ProvenanceChainEnd};

const BUILDER_CLASS: Uid = Uid {
    a: 0xb56d5931,
    b: 0x2225,
    c: 0x42bf,
    d: [0x8c, 0x93, 0xe7, 0x61, 0x8e, 0xf1, 0x71, 0x35],
};
pub(crate) const BUILDER_IID: Uid = Uid {
    a: 0x8e75cc94,
    b: 0x8a53,
    c: 0x4ff3,
    d: [0x87, 0xce, 0x79, 0xab, 0x5f, 0x9c, 0xd3, 0xf1],
};
pub const MAX_MENU_ROWS: usize = 256;
pub const MAX_MENU_DEPTH: usize = 4;
pub const MAX_MENU_TEXT: usize = 1024;
pub const MAX_MENU_TEXT_TOTAL: usize = 32768;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternalPort {
    Send,
    Return,
}
impl ExternalPort {
    pub fn name(self) -> &'static str {
        match self {
            Self::Send => "sendPort",
            Self::Return => "receivePort",
        }
    }
    pub fn tag(self) -> i32 {
        match self {
            Self::Send => 102,
            Self::Return => 103,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuPhase {
    Prepared,
    ConstructDispatched,
    BuildDispatched,
    ReleaseDispatched,
    ClosedChecked,
}
impl MenuPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Prepared => "prepared",
            Self::ConstructDispatched => "construct_dispatched",
            Self::BuildDispatched => "build_dispatched",
            Self::ReleaseDispatched => "release_dispatched",
            Self::ClosedChecked => "closed_checked",
        }
    }
    pub fn began(self) -> bool {
        self != Self::Prepared
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuRow {
    pub path: Vec<u32>,
    pub name: Option<String>,
    pub title: Option<String>,
    pub category: Option<String>,
    pub checked: Option<bool>,
    pub enabled: Option<bool>,
    pub has_submenu: bool,
    pub handler_present: Option<bool>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuObservation {
    pub menu_class: String,
    pub rows: Vec<MenuRow>,
    pub provenance: Option<MenuProvenance>,
    pub root_menu_final_release_count: u32,
    pub builder_final_release_count: u32,
}
pub struct MenuReadLease {
    binding: ParameterBindingReadLease,
    before: Description,
    _allocator: Owned,
}
impl MenuReadLease {
    pub fn check<G: FnMut() -> Result<(), String>>(&self, g: &mut G) -> Result<(), Error> {
        self.binding.check(g)?;
        let after = self.binding.parameter().describe();
        self.binding.check(g)?;
        if after? != self.before {
            return Err(Error::MetadataChanged);
        }
        Ok(())
    }
}
pub struct MenuReadAttempt {
    pub result: Result<MenuObservation, Error>,
    pub phase: MenuPhase,
    pub lease: Option<MenuReadLease>,
}
// The observed graph is sequential, not an atomic native snapshot. Retained canonical
// identities let terminal bounded checks reject persistent membership/link drift.
struct ItemSnapshot {
    raw: *mut c_void,
    identity: usize,
    submenu_identity: Option<usize>,
}
struct MenuSnapshot {
    raw: *mut c_void,
    items: Vec<ItemSnapshot>,
}
struct Resources {
    builder: Option<Owned>,
    menu: Option<Owned>,
    borrowed: Vec<Owned>,
    snapshots: Vec<MenuSnapshot>,
}
impl Resources {
    fn release<G: FnMut() -> Result<(), String>>(
        &mut self,
        g: &mut G,
    ) -> Result<(u32, u32), Error> {
        let mut first = None;
        let mut checked_release = |object: Owned| {
            if let Err(e) = g() {
                if first.is_none() {
                    first = Some(Error::Guard(e));
                }
            }
            let count = release(object);
            if let Err(e) = g() {
                if first.is_none() {
                    first = Some(Error::Guard(e));
                }
            }
            count
        };
        while let Some(item) = self.borrowed.pop() {
            checked_release(item);
        }
        let menu = self.menu.take().map(&mut checked_release).unwrap_or(0);
        let builder = self.builder.take().map(&mut checked_release).unwrap_or(0);
        match first {
            Some(e) => Err(e),
            None => Ok((menu, builder)),
        }
    }
}
fn release(object: Owned) -> u32 {
    let p = object.into_raw();
    let f: unsafe extern "C" fn(*mut c_void) -> u32 = unsafe { ffi::method(p, 2) };
    unsafe { f(p) }
}
fn attr<G: FnMut() -> Result<(), String>>(
    item: *mut c_void,
    id: i32,
    g: &mut G,
) -> Result<Option<Value>, Error> {
    g().map_err(Error::Guard)?;
    let f: unsafe extern "C" fn(*mut c_void, *mut Variant, i32) -> u8 =
        unsafe { ffi::method(item, 3) };
    let mut value = Variant::empty();
    let found = unsafe { f(item, &mut value, id) };
    let after = g().map_err(Error::Guard);
    let result = after.and_then(|_| {
        if found == 0 {
            Ok(None)
        } else if value.tag & 15 == 3 && value.payload == 0 {
            Ok(Some(Value::Text(String::new())))
        } else {
            value.decode().map(|x| Some(x.value))
        }
    });
    drop(value);
    g().map_err(Error::Guard)?;
    result
}
fn text<G: FnMut() -> Result<(), String>>(
    item: *mut c_void,
    id: i32,
    total: &mut usize,
    g: &mut G,
) -> Result<Option<String>, Error> {
    match attr(item, id, g)? {
        None | Some(Value::Empty) => Ok(None),
        Some(Value::Text(s))
            if s.len() <= MAX_MENU_TEXT && *total + s.len() <= MAX_MENU_TEXT_TOTAL =>
        {
            *total += s.len();
            Ok(Some(s))
        }
        Some(Value::Text(_)) => Err(Error::InvalidData("menu text bound exceeded")),
        _ => Err(Error::TypeMismatch),
    }
}
fn boolean<G: FnMut() -> Result<(), String>>(
    item: *mut c_void,
    id: i32,
    g: &mut G,
) -> Result<Option<bool>, Error> {
    match attr(item, id, g)? {
        None | Some(Value::Empty) => Ok(None),
        Some(Value::Integer(0)) => Ok(Some(false)),
        Some(Value::Integer(1)) => Ok(Some(true)),
        _ => Err(Error::TypeMismatch),
    }
}
fn handler<G: FnMut() -> Result<(), String>>(
    item: *mut c_void,
    g: &mut G,
) -> Result<Option<bool>, Error> {
    match attr(item, 12, g)? {
        None | Some(Value::Empty) => Ok(None),
        Some(Value::OpaqueObject { present }) => Ok(Some(present)),
        _ => Err(Error::TypeMismatch),
    }
}
fn uid_text(p: *const Uid) -> Result<String, Error> {
    if p.is_null() {
        return Err(Error::InvalidData("menu class is null"));
    }
    let x = unsafe { *p };
    if x.a == 0 && x.b == 0 && x.c == 0 && x.d.iter().all(|&v| v == 0) {
        return Err(Error::InvalidData("menu class is empty"));
    }
    Ok(format!(
        "{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
        x.a, x.b, x.c, x.d[0], x.d[1], x.d[2], x.d[3], x.d[4], x.d[5], x.d[6], x.d[7]
    ))
}
fn retain_identity<G: FnMut() -> Result<(), String>>(
    raw: *mut c_void,
    resources: &mut Resources,
    g: &mut G,
) -> Result<usize, Error> {
    let held = unsafe { Owned::hold(raw) }?;
    resources.borrowed.push(held);
    g().map_err(Error::Guard)?;
    let canonical = resources
        .borrowed
        .last()
        .expect("held object")
        .query(&ffi::UNKNOWN)?;
    let identity = canonical.raw() as usize;
    resources.borrowed.push(canonical);
    g().map_err(Error::Guard)?;
    Ok(identity)
}
fn menu_count<G: FnMut() -> Result<(), String>>(
    menu: *mut c_void,
    g: &mut G,
) -> Result<i32, Error> {
    g().map_err(Error::Guard)?;
    let count: unsafe extern "C" fn(*mut c_void) -> i32 = unsafe { ffi::method(menu, 8) };
    let n = unsafe { count(menu) };
    g().map_err(Error::Guard)?;
    Ok(n)
}
fn submenu<G: FnMut() -> Result<(), String>>(
    item: *mut c_void,
    resources: &mut Resources,
    g: &mut G,
) -> Result<Option<(*mut c_void, usize)>, Error> {
    g().map_err(Error::Guard)?;
    let sub: unsafe extern "C" fn(*mut c_void) -> *mut c_void = unsafe { ffi::method(item, 6) };
    let raw = unsafe { sub(item) };
    let result = if raw.is_null() {
        None
    } else {
        Some((raw, retain_identity(raw, resources, g)?))
    };
    g().map_err(Error::Guard)?;
    Ok(result)
}
fn check_snapshots<G: FnMut() -> Result<(), String>>(
    resources: &mut Resources,
    g: &mut G,
) -> Result<(), Error> {
    let snapshots = std::mem::take(&mut resources.snapshots);
    for snapshot in &snapshots {
        if menu_count(snapshot.raw, g)? != snapshot.items.len() as i32 {
            return Err(Error::InvalidData("menu membership changed"));
        }
        for (index, expected) in snapshot.items.iter().enumerate() {
            g().map_err(Error::Guard)?;
            let get: unsafe extern "C" fn(*mut c_void, i32) -> *mut c_void =
                unsafe { ffi::method(snapshot.raw, 9) };
            let raw = unsafe { get(snapshot.raw, index as i32) };
            if retain_identity(raw, resources, g)? != expected.identity
                || submenu(raw, resources, g)?.map(|(_, id)| id) != expected.submenu_identity
            {
                return Err(Error::InvalidData("menu membership changed"));
            }
        }
        if menu_count(snapshot.raw, g)? != snapshot.items.len() as i32 {
            return Err(Error::InvalidData("menu membership changed"));
        }
    }
    resources.snapshots = snapshots;
    Ok(())
}
fn walk<G: FnMut() -> Result<(), String>>(
    menu: *mut c_void,
    path: &mut Vec<u32>,
    ancestors: &mut HashSet<usize>,
    rows: &mut Vec<MenuRow>,
    total: &mut usize,
    resources: &mut Resources,
    g: &mut G,
) -> Result<(), Error> {
    if path.len() >= MAX_MENU_DEPTH {
        return Err(Error::InvalidData("menu depth bound exceeded"));
    }
    g().map_err(Error::Guard)?;
    let identity = retain_identity(menu, resources, g)?;
    if !ancestors.insert(identity) {
        return Err(Error::InvalidData("menu cycle"));
    }
    let n = menu_count(menu, g)?;
    if n < 0 || rows.len() + n as usize > MAX_MENU_ROWS {
        return Err(Error::InvalidData("menu row bound exceeded"));
    }
    // Retain every original ordered member before any attribute callback can reenter.
    let mut items = Vec::with_capacity(n as usize);
    for index in 0..n {
        g().map_err(Error::Guard)?;
        let get: unsafe extern "C" fn(*mut c_void, i32) -> *mut c_void =
            unsafe { ffi::method(menu, 9) };
        let raw = unsafe { get(menu, index) };
        let identity = retain_identity(raw, resources, g)?;
        items.push(ItemSnapshot {
            raw,
            identity,
            submenu_identity: None,
        });
    }
    if menu_count(menu, g)? != n {
        return Err(Error::InvalidData("menu membership changed"));
    }
    for (index, item) in items.iter_mut().enumerate() {
        if rows.len() == MAX_MENU_ROWS {
            return Err(Error::InvalidData("menu row bound exceeded"));
        }
        path.push(index as u32);
        let name = text(item.raw, 0, total, g)?;
        let title = text(item.raw, 1, total, g)?;
        let category = text(item.raw, 11, total, g)?;
        let checked = boolean(item.raw, 4, g)?;
        let enabled = boolean(item.raw, 5, g)?;
        let handler_present = handler(item.raw, g)?;
        let child = submenu(item.raw, resources, g)?;
        item.submenu_identity = child.map(|(_, id)| id);
        rows.push(MenuRow {
            path: path.clone(),
            name,
            title,
            category,
            checked,
            enabled,
            has_submenu: child.is_some(),
            handler_present,
        });
        if let Some((raw, _)) = child {
            walk(raw, path, ancestors, rows, total, resources, g)?;
        }
        path.pop();
    }
    ancestors.remove(&identity);
    resources.snapshots.push(MenuSnapshot { raw: menu, items });
    Ok(())
}
impl RawParameter {
    /// Builds and closes one fixed External port menu without presenting or selecting it.
    ///
    /// # Safety
    /// The allocator must be the validated runtime's retained IClassAllocator.
    /// Keep the exact original/editor/document/service alive through this attempt and lease.
    pub unsafe fn external_port_menu<G: FnMut() -> Result<(), String>>(
        self,
        allocator: NonNull<c_void>,
        port: ExternalPort,
        g: &mut G,
    ) -> MenuReadAttempt {
        unsafe { self.external_port_menu_inner(allocator, port, g, false) }
    }

    /// Fixed provenance primitive: class_uid comes from the retained native instance,
    /// never a request assertion. Guard must revalidate document/session,
    /// exact instance, open editor/controller and selected parameter.
    /// This performs no selection and observes no menuData typed values.
    ///
    /// # Safety
    /// The allocator/runtime/GUI thread and retained device/editor/document
    /// must satisfy external_port_menu's contract. The class must be read
    /// from the validated native instance, not a caller assertion. The guard
    /// must reject any document, session, instance, editor or route change.
    pub unsafe fn external_stereo_send_menu_provenance<G: FnMut() -> Result<(), String>>(
        self,
        allocator: NonNull<c_void>,
        class_uid: &str,
        g: &mut G,
    ) -> MenuReadAttempt {
        if class_uid != "889DD9F8-14F0-44A5-BCEE-0F2B157FC5D4" {
            return MenuReadAttempt {
                result: Err(Error::InvalidData("exact Stereo source identity required")),
                phase: MenuPhase::Prepared,
                lease: None,
            };
        }
        unsafe { self.external_port_menu_inner(allocator, ExternalPort::Send, g, true) }
    }

    unsafe fn external_port_menu_inner<G: FnMut() -> Result<(), String>>(
        self,
        allocator: NonNull<c_void>,
        port: ExternalPort,
        g: &mut G,
        capture_provenance: bool,
    ) -> MenuReadAttempt {
        let mut phase = MenuPhase::Prepared;
        let mut lease = None;
        let mut resources = Resources {
            builder: None,
            menu: None,
            borrowed: Vec::new(),
            snapshots: Vec::new(),
        };
        let result = (|| {
            let binding = self.pin_binding_read(g)?;
            binding.check(g)?;
            let before = binding.parameter().describe()?;
            binding.check(g)?;
            if before.name != port.name()
                || before.tag != port.tag()
                || before.kind != crate::Kind::List
            {
                return Err(Error::InvalidData("exact External port List required"));
            }
            let allocator = unsafe { Owned::hold(allocator.as_ptr()) }?;
            binding.check(g)?;
            lease = Some(MenuReadLease {
                binding,
                before,
                _allocator: allocator,
            });
            let l = lease.as_ref().expect("captured lease");
            let mut guard = || l.binding.check(g).map_err(|e| e.to_string());
            guard().map_err(Error::Guard)?;
            let create: unsafe extern "C" fn(
                *mut c_void,
                *const Uid,
                *const Uid,
                *mut *mut c_void,
            ) -> i32 = unsafe { ffi::method(l._allocator.raw(), 3) };
            let mut output = ptr::null_mut();
            let status = unsafe {
                create(
                    l._allocator.raw(),
                    &BUILDER_CLASS,
                    &BUILDER_IID,
                    &mut output,
                )
            };
            if status == 0 {
                resources.builder = NonNull::new(output).map(|p| unsafe { Owned::take(p) });
            }
            guard().map_err(Error::Guard)?;
            if status != 0 {
                return Err(Error::NativeFailure("ParameterMenuBuilder factory", status));
            }
            let builder = resources
                .builder
                .as_ref()
                .ok_or(Error::MissingInterface("ParameterMenuBuilder"))?
                .raw();
            let construct: unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32 =
                unsafe { ffi::method(builder, 3) };
            guard().map_err(Error::Guard)?;
            phase = MenuPhase::ConstructDispatched;
            let status =
                unsafe { construct(builder, l.binding.parameter().as_borrowed_raw().as_ptr()) };
            guard().map_err(Error::Guard)?;
            if status != 0 {
                return Err(Error::NativeFailure(
                    "ParameterMenuBuilder construct",
                    status,
                ));
            }
            let build: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                unsafe { ffi::method(builder, 4) };
            guard().map_err(Error::Guard)?;
            phase = MenuPhase::BuildDispatched;
            resources.menu = NonNull::new(unsafe { build(builder, ptr::null_mut()) })
                .map(|p| unsafe { Owned::take(p) });
            guard().map_err(Error::Guard)?;
            let menu = resources
                .menu
                .as_ref()
                .ok_or(Error::MissingInterface("built menu"))?
                .raw();
            guard().map_err(Error::Guard)?;
            let class: unsafe extern "C" fn(*mut c_void) -> *const Uid =
                unsafe { ffi::method(menu, 3) };
            let menu_class = uid_text(unsafe { class(menu) })?;
            guard().map_err(Error::Guard)?;
            let mut rows = Vec::new();
            let mut total = 0;
            walk(
                menu,
                &mut Vec::new(),
                &mut HashSet::new(),
                &mut rows,
                &mut total,
                &mut resources,
                &mut guard,
            )?;
            check_snapshots(&mut resources, &mut guard)?;
            let provenance = if capture_provenance {
                let selected = l.binding.parameter().as_borrowed_raw().as_ptr();
                let first = provenance::capture(
                    menu,
                    builder,
                    selected,
                    &rows,
                    &mut resources,
                    &mut guard,
                )?;
                check_snapshots(&mut resources, &mut guard)?;
                let second = provenance::capture(
                    menu,
                    builder,
                    selected,
                    &rows,
                    &mut resources,
                    &mut guard,
                )?;
                check_snapshots(&mut resources, &mut guard)?;
                if first != second {
                    return Err(Error::MetadataChanged);
                }
                Some(first.observation)
            } else {
                None
            };
            Ok((menu_class, rows, provenance))
        })();
        if phase.began() {
            phase = MenuPhase::ReleaseDispatched;
        }
        let cleaned = if let Some(l) = lease.as_ref() {
            resources.release(&mut || l.binding.check(g).map_err(|e| e.to_string()))
        } else {
            resources.release(g)
        };
        let result = result.and_then(|(menu_class, rows, provenance)| {
            let (m, b) = cleaned?;
            if m != 0 || b != 0 {
                return Err(Error::InvalidData("menu cleanup retained outside owners"));
            }
            lease.as_ref().expect("captured lease").check(g)?;
            phase = MenuPhase::ClosedChecked;
            Ok(MenuObservation {
                menu_class,
                rows,
                provenance,
                root_menu_final_release_count: m,
                builder_final_release_count: b,
            })
        });
        MenuReadAttempt {
            result,
            phase,
            lease,
        }
    }
}

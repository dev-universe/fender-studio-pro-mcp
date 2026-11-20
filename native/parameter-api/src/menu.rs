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
pub use provenance::{LeafCheckOnly, LeafProvenance, MenuProvenance, ProvenanceChainEnd};

const STEREO_CLASS: &str = "889DD9F8-14F0-44A5-BCEE-0F2B157FC5D4";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StereoSendRoute {
    None,
    Main,
}
impl StereoSendRoute {
    fn index(self) -> usize {
        match self {
            Self::None => 0,
            Self::Main => 1,
        }
    }
    fn opposite(self) -> Self {
        match self {
            Self::None => Self::Main,
            Self::Main => Self::None,
        }
    }
    fn display(self) -> &'static str {
        match self {
            Self::None => "없음",
            Self::Main => "메인",
        }
    }
    fn matches_unassigned(self, description: &Description) -> bool {
        description.name == ExternalPort::Send.name()
            && description.tag == ExternalPort::Send.tag()
            && description.kind == crate::Kind::List
            && description.enabled
            && !description.read_only
            && !description.support.raw_number
            && !description.support.normalized
            && !description.support.display_text
            && description.raw
                == Some(crate::NativeValue {
                    variant_tag: 1,
                    value: Value::Integer(-1),
                })
            && description.normalized == crate::Normalized::Finite(1.0)
            && description.display == self.display()
    }
}

fn matches_held_send_description(before: &Description, during: Description) -> bool {
    let integer = |value: &Option<crate::NativeValue>| match value {
        Some(crate::NativeValue {
            variant_tag: 1,
            value: Value::Integer(number),
        }) => Some(*number),
        _ => None,
    };
    let (Some(raw), Some(max)) = (integer(&during.raw), integer(&during.max)) else {
        return false;
    };
    if !(-1..=max).contains(&raw)
        || !matches!(during.normalized, crate::Normalized::Finite(value) if (0.0..=1.0).contains(&value))
    {
        return false;
    }
    // The menu builder supplies a temporary list. Its teardown must restore
    // these four fields exactly. The caller checks the retained route before dispatch.
    Description {
        raw: before.raw.clone(),
        max: before.max.clone(),
        normalized: before.normalized,
        support: before.support,
        ..during
    } == *before
}

#[derive(Clone, Copy)]
enum MenuPurpose {
    Read,
    Prove {
        current: StereoSendRoute,
        query: bool,
    },
    Select(StereoSendRoute),
}
impl MenuPurpose {
    fn current(self) -> Option<StereoSendRoute> {
        match self {
            Self::Read => None,
            Self::Prove { current, .. } => Some(current),
            Self::Select(destination) => Some(destination.opposite()),
        }
    }
    fn query(self) -> bool {
        matches!(self, Self::Select(_) | Self::Prove { query: true, .. })
    }
    fn destination(self) -> Option<StereoSendRoute> {
        match self {
            Self::Select(destination) => Some(destination),
            _ => None,
        }
    }
}

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
    pub check_only: Option<Vec<LeafCheckOnly>>,
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
        self.check_description(&self.before, g)
    }

    fn check_description<G: FnMut() -> Result<(), String>>(
        &self,
        expected: &Description,
        g: &mut G,
    ) -> Result<(), Error> {
        self.binding.check(g)?;
        let after = self.binding.parameter().describe();
        self.binding.check(g)?;
        if &after? != expected {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StereoSendSelectionPhase {
    SourceParameter,
    BuilderFactory,
    Construct,
    Build,
    SourceProfile,
    CheckOnly,
    SourceRebind,
    Select,
    Cleanup,
    TypedDestination,
    FreshDestinationMenu,
    Complete,
}
impl StereoSendSelectionPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SourceParameter => "source_parameter",
            Self::BuilderFactory => "builder_factory",
            Self::Construct => "construct",
            Self::Build => "build",
            Self::SourceProfile => "source_profile",
            Self::CheckOnly => "check_only",
            Self::SourceRebind => "source_rebind",
            Self::Select => "select",
            Self::Cleanup => "cleanup",
            Self::TypedDestination => "typed_destination",
            Self::FreshDestinationMenu => "fresh_destination_menu",
            Self::Complete => "complete",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StereoSendSelectionTrace {
    pub phase: StereoSendSelectionPhase,
    pub select_entered: bool,
    pub select_returned: bool,
    pub cleanup_proven: bool,
}
impl Default for StereoSendSelectionTrace {
    fn default() -> Self {
        Self {
            phase: StereoSendSelectionPhase::SourceParameter,
            select_entered: false,
            select_returned: false,
            cleanup_proven: false,
        }
    }
}

pub struct StereoSendSelectionAttempt {
    /// Any error after a callback-sensitive menu phase began is outcome_unknown.
    pub result: Result<StereoSendSelectionReceipt, Error>,
    pub began: bool,
    pub selection_dispatched: bool,
    pub trace: StereoSendSelectionTrace,
}

pub struct StereoSendSelectionReceipt {
    before: Description,
    after: Description,
    pub route: StereoSendRoute,
    /// The SDK boolean is returned for evidence only. It is not selection proof.
    pub raw_select_result: u8,
    leases: [MenuReadLease; 2],
}
impl StereoSendSelectionReceipt {
    pub fn before(&self) -> &Description {
        &self.before
    }

    pub fn after(&self) -> &Description {
        &self.after
    }

    pub fn check<G: FnMut() -> Result<(), String>>(&self, g: &mut G) -> Result<(), Error> {
        for lease in &self.leases {
            lease.check_description(&self.after, g)?;
        }
        Ok(())
    }
}

struct MenuAttempt {
    read: MenuReadAttempt,
    selection: Option<u8>,
    selection_callbacks_began: bool,
    trace: StereoSendSelectionTrace,
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
        unsafe { self.external_port_menu_inner(allocator, port, g, MenuPurpose::Read, None) }.read
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
        if class_uid != STEREO_CLASS {
            return MenuReadAttempt {
                result: Err(Error::InvalidData("exact Stereo source identity required")),
                phase: MenuPhase::Prepared,
                lease: None,
            };
        }
        unsafe {
            self.external_port_menu_inner(
                allocator,
                ExternalPort::Send,
                g,
                MenuPurpose::Prove {
                    current: StereoSendRoute::None,
                    query: false,
                },
                None,
            )
        }
        .read
    }

    /// Unregistered fixed query primitive; never selects an item or executes flags0.
    /// # Safety
    /// Same validated runtime/GUI-thread/retained Context, exact device/editor/parameter
    /// requirements as external_stereo_send_menu_provenance. The class is a live
    /// Context observation, never a caller assertion. Callback-sensitive, not atomic.
    pub unsafe fn external_stereo_send_menu_check_only<G: FnMut() -> Result<(), String>>(
        self,
        allocator: NonNull<c_void>,
        class_uid: &str,
        g: &mut G,
    ) -> MenuReadAttempt {
        if class_uid != STEREO_CLASS {
            return MenuReadAttempt {
                result: Err(Error::InvalidData("exact Stereo source identity required")),
                phase: MenuPhase::Prepared,
                lease: None,
            };
        }
        unsafe {
            self.external_port_menu_inner(
                allocator,
                ExternalPort::Send,
                g,
                MenuPurpose::Prove {
                    current: StereoSendRoute::None,
                    query: true,
                },
                None,
            )
        }
        .read
    }

    /// Internal native candidate. Select one logical Stereo Send destination,
    /// then close and rebuild a fresh menu to prove the displayed state.
    ///
    /// # Safety
    /// The caller must retain the validated exact class/editor/controller and
    /// the native runtime/GUI thread. The guard must reject target, document
    /// and attachment drift. The final before_select check must independently
    /// prove the retained source route. Verify the complete controller,
    /// stopped transport and silent fixture independently around this attempt.
    pub unsafe fn external_stereo_send_select<G: FnMut() -> Result<(), String>>(
        self,
        allocator: NonNull<c_void>,
        class_uid: &str,
        destination: StereoSendRoute,
        g: &mut G,
        before_select: &mut dyn FnMut() -> Result<(), String>,
    ) -> StereoSendSelectionAttempt {
        let mut began = false;
        let mut trace = StereoSendSelectionTrace::default();
        let result = (|| {
            if class_uid != STEREO_CLASS {
                return Err(Error::InvalidData("exact Stereo source identity required"));
            }
            let first = unsafe {
                self.external_port_menu_inner(
                    allocator,
                    ExternalPort::Send,
                    g,
                    MenuPurpose::Select(destination),
                    Some(before_select),
                )
            };
            began = first.selection_callbacks_began;
            trace = first.trace;
            first.read.result?;
            let raw_select_result = first
                .selection
                .ok_or(Error::InvalidData("selection not dispatched"))?;
            if raw_select_result != 1 {
                trace.phase = StereoSendSelectionPhase::Select;
                return Err(Error::InvalidData(
                    "fixed selectItem acknowledgment differs",
                ));
            }
            trace.phase = StereoSendSelectionPhase::TypedDestination;
            let initial = first
                .read
                .lease
                .ok_or(Error::InvalidData("missing selection lease"))?;
            let before = initial.before.clone();
            let mut after = before.clone();
            after.display = destination.display().into();
            initial.check_description(&after, g)?;

            let parameter = initial.binding.parameter();
            let retained = unsafe { Owned::hold(parameter.as_borrowed_raw().as_ptr()) }?;
            initial.check_description(&after, g)?;
            let verification_parameter = unsafe {
                RawParameter::from_owned(
                    NonNull::new(retained.into_raw()).expect("retained parameter"),
                    parameter.text,
                )
            };
            trace.phase = StereoSendSelectionPhase::FreshDestinationMenu;
            let fresh = unsafe {
                verification_parameter.external_port_menu_inner(
                    allocator,
                    ExternalPort::Send,
                    g,
                    MenuPurpose::Prove {
                        current: destination,
                        query: false,
                    },
                    None,
                )
            };
            let observation = fresh.read.result?;
            provenance::require_selected_parameter(
                observation
                    .provenance
                    .as_ref()
                    .ok_or(Error::InvalidData("missing selection provenance"))?,
            )?;
            let last = fresh
                .read
                .lease
                .ok_or(Error::InvalidData("missing verification lease"))?;
            let receipt = StereoSendSelectionReceipt {
                before,
                after,
                route: destination,
                raw_select_result,
                leases: [initial, last],
            };
            receipt.check(g)?;
            trace.phase = StereoSendSelectionPhase::Complete;
            Ok(receipt)
        })();
        StereoSendSelectionAttempt {
            result,
            began,
            selection_dispatched: trace.select_entered,
            trace,
        }
    }

    unsafe fn external_port_menu_inner<G: FnMut() -> Result<(), String>>(
        self,
        allocator: NonNull<c_void>,
        port: ExternalPort,
        g: &mut G,
        purpose: MenuPurpose,
        before_select: Option<&mut dyn FnMut() -> Result<(), String>>,
    ) -> MenuAttempt {
        let mut phase = MenuPhase::Prepared;
        let mut lease = None;
        let mut selection = None;
        let mut selection_callbacks_began = false;
        let mut trace = StereoSendSelectionTrace::default();
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
            let checked_route = purpose.current();
            if (purpose.destination().is_some()
                || matches!(
                    purpose,
                    MenuPurpose::Prove {
                        current: StereoSendRoute::Main,
                        ..
                    }
                ))
                && !checked_route
                    .expect("fixed route")
                    .matches_unassigned(&before)
            {
                return Err(Error::InvalidData(
                    "exact unassigned-output Send state required",
                ));
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
            trace.phase = StereoSendSelectionPhase::BuilderFactory;
            selection_callbacks_began = purpose.destination().is_some();
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
            trace.phase = StereoSendSelectionPhase::Construct;
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
            trace.phase = StereoSendSelectionPhase::Build;
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
            trace.phase = StereoSendSelectionPhase::SourceProfile;
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
            let mut queries = None;
            let provenance = if let Some(current) = checked_route {
                let selected = l.binding.parameter().as_borrowed_raw().as_ptr();
                let first = provenance::capture(
                    menu,
                    builder,
                    selected,
                    &rows,
                    current,
                    &mut resources,
                    &mut guard,
                )?;
                check_snapshots(&mut resources, &mut guard)?;
                if purpose.query() {
                    trace.phase = StereoSendSelectionPhase::CheckOnly;
                    queries = Some(provenance::query_fixed(
                        menu,
                        builder,
                        selected,
                        &rows,
                        &first,
                        &mut resources,
                        &mut guard,
                    )?);
                    check_snapshots(&mut resources, &mut guard)?;
                }
                trace.phase = StereoSendSelectionPhase::SourceRebind;
                let second = provenance::capture(
                    menu,
                    builder,
                    selected,
                    &rows,
                    current,
                    &mut resources,
                    &mut guard,
                )?;
                check_snapshots(&mut resources, &mut guard)?;
                if first != second {
                    return Err(Error::MetadataChanged);
                }
                if let Some(destination) = purpose.destination() {
                    let eligible = queries
                        .as_ref()
                        .and_then(|values| values.get(destination.index()));
                    if !eligible
                        .is_some_and(|value| value.raw_category == 1 && value.raw_check_only == 1)
                    {
                        trace.phase = StereoSendSelectionPhase::CheckOnly;
                        return Err(Error::InvalidData("fixed destination checkOnly refused"));
                    }
                    guard().map_err(Error::Guard)?;
                    let current_value = l.binding.parameter().describe()?;
                    guard().map_err(Error::Guard)?;
                    if !matches_held_send_description(&l.before, current_value) {
                        return Err(Error::MetadataChanged);
                    }
                    check_snapshots(&mut resources, &mut guard)?;
                    let root = resources
                        .snapshots
                        .iter()
                        .find(|item| item.raw == menu)
                        .ok_or(Error::InvalidData("missing root membership"))?;
                    let item = root
                        .items
                        .get(destination.index())
                        .ok_or(Error::MetadataChanged)?
                        .raw;
                    // IMenuItem::selectItem, SDK 5.1.2 slot 7, tbool. The SDK route
                    // consults the global command filter before handler execution.
                    let select: unsafe extern "C" fn(*mut c_void) -> u8 =
                        unsafe { ffi::method(item, 7) };
                    before_select.ok_or(Error::InvalidData("missing retained source check"))?()
                        .map_err(Error::Guard)?;
                    trace.phase = StereoSendSelectionPhase::Select;
                    trace.select_entered = true;
                    selection = Some(unsafe { select(item) });
                    trace.select_returned = true;
                    guard().map_err(Error::Guard)?;
                    check_snapshots(&mut resources, &mut guard)?;
                }
                Some(first.observation)
            } else {
                None
            };
            Ok((menu_class, rows, provenance, queries))
        })();
        if phase.began() {
            phase = MenuPhase::ReleaseDispatched;
        }
        let had_builder = resources.builder.is_some();
        let cleaned = if let Some(l) = lease.as_ref() {
            resources.release(&mut || l.binding.check(g).map_err(|e| e.to_string()))
        } else {
            resources.release(g)
        };
        trace.cleanup_proven =
            selection_callbacks_began && had_builder && matches!(&cleaned, Ok((0, 0)));
        let result = result.and_then(|(menu_class, rows, provenance, queries)| {
            trace.phase = StereoSendSelectionPhase::Cleanup;
            let (m, b) = cleaned?;
            if m != 0 || b != 0 {
                return Err(Error::InvalidData("menu cleanup retained outside owners"));
            }
            let lease = lease.as_ref().expect("captured lease");
            if let Some(destination) = purpose.destination() {
                trace.phase = StereoSendSelectionPhase::TypedDestination;
                let mut expected = lease.before.clone();
                expected.display = destination.display().into();
                lease.check_description(&expected, g)?;
            } else {
                lease.check(g)?;
            }
            phase = MenuPhase::ClosedChecked;
            Ok(MenuObservation {
                menu_class,
                rows,
                provenance,
                check_only: queries,
                root_menu_final_release_count: m,
                builder_final_release_count: b,
            })
        });
        MenuAttempt {
            read: MenuReadAttempt {
                result,
                phase,
                lease,
            },
            selection,
            selection_callbacks_began,
            trace,
        }
    }
}

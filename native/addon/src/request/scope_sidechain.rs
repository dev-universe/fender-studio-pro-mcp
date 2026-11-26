use super::*;
use node_inspect::DocumentEnvironmentRoute;
use node_inspect::slot_move::splitter_add::{
    RecordingScope, ScopeStorage, recorded_submit, undo_available,
};
use std::ffi::CStr;
use std::mem::MaybeUninit;

const SCOPE_CLASS: &str = "36322412-AD90-4EE0-B63C-37929566B081";
const MAX_CHILDREN: i32 = 32;
const INSERT_SLOT_LAYOUTS: [(usize, usize, usize); 2] = [
    (0x1b78948, 0x1b78d78, 0x1b78e40),
    (0x1b90b50, 0x1b90f80, 0x1b91048),
];
const SIDECHAIN_PRIMARY: usize = 0x1b77470;
const SIDECHAIN_UNKNOWN: usize = 0x1b77668;
const SIDECHAIN_PORT_VIEW: usize = 0x1b77730;
const SEND_LAYOUT: [usize; 3] = [0x1b9d960, 0x1b9de20, 0x1b9dee8];
const SEND_RACK_LAYOUT: [usize; 3] = [0x1b9e520, 0x1b9e900, 0x1b9e9c8];
const SLOT_EDIT_SCOPE: usize = 0x1bd9250;
const ACTION_JOURNAL: Uid = Uid {
    a: 0xd70f5dc5,
    b: 0xd9bd,
    c: 0x4230,
    d: [0xb9, 0x08, 0xf5, 0x57, 0xcc, 0x41, 0x79, 0x3f],
};

unsafe fn field<T: Copy>(object: *mut c_void, offset: usize) -> T {
    unsafe { object.cast::<u8>().add(offset).cast::<T>().read() }
}

fn send_primary(node: &Node, image: usize, layout: [usize; 3]) -> Result<*mut c_void, String> {
    let [primary_table, canonical_table, node_table] = layout;
    let view = node.raw().cast::<u8>();
    if view.wrapping_sub(0x18).cast() != node.identity.0
        || table(node.identity.0) != image + canonical_table
        || table(node.raw()) != image + node_table
    {
        return Err("Unverified Send or rack interface layout".into());
    }
    let primary: *mut c_void = view.wrapping_sub(0x30).cast();
    if table(primary) != image + primary_table {
        return Err("Unverified Send or rack primary layout".into());
    }
    Ok(primary)
}

fn table(pointer: *mut c_void) -> usize {
    unsafe { *pointer.cast::<usize>() }
}

struct Node {
    interface: Owned,
    identity: Owned,
}

impl Node {
    fn capture(object: *mut c_void) -> Result<Self, String> {
        Ok(Self {
            interface: query(object, &NODE).ok_or("missing native node interface")?,
            identity: query(object, &UNKNOWN).ok_or("missing native node identity")?,
        })
    }

    fn raw(&self) -> *mut c_void {
        self.interface.0
    }

    fn parent(&self) -> *mut c_void {
        let get: Borrowed = unsafe { method(self.raw(), 6) };
        unsafe { get(self.raw()) }
    }

    fn count(&self) -> Result<i32, String> {
        let get: Count = unsafe { method(self.raw(), 8) };
        let count = unsafe { get(self.raw()) };
        if !(0..=MAX_CHILDREN).contains(&count) {
            return Err("direct node child count is outside the supported bound".into());
        }
        Ok(count)
    }

    fn at(&self, index: i32) -> *mut c_void {
        let get: unsafe extern "C" fn(*mut c_void, i32) -> *mut c_void =
            unsafe { method(self.raw(), 9) };
        unsafe { get(self.raw(), index) }
    }

    fn object_id(&self) -> Result<String, String> {
        let getter: unsafe extern "C" fn(*mut c_void) -> *const Uid =
            unsafe { method(self.raw(), 4) };
        let id = uid(unsafe { getter(self.raw()) });
        if !valid_uid(&id) {
            return Err("The Send has no canonical node UUID".into());
        }
        Ok(id)
    }

    fn has_id(&self, wanted: &str) -> Result<bool, String> {
        let get: unsafe extern "C" fn(*mut c_void) -> *const PlainString =
            unsafe { method(self.raw(), 3) };
        let id = unsafe { get(self.raw()).as_ref() }.ok_or("node ID unavailable")?;
        let text = hold(id.object).ok_or("node ID owner unavailable")?;
        copy_string(text.0, 128).map(|value| value == wanted)
    }
}

struct Child {
    node: Node,
    index: i32,
    count: i32,
}

impl Child {
    fn find(ctx: &Context<'_>, parent: &Node, wanted: &str, kind: &str) -> Result<Self, String> {
        let count = parent.count()?;
        let mut result = None;
        for index in 0..count {
            ctx.check()?;
            let node = Node::capture(parent.at(index))?;
            if node.has_id(wanted)? && class_name(node.raw()) == kind {
                if result.is_some() {
                    return Err(format!("ambiguous direct {wanted} node"));
                }
                result = Some(Self { node, index, count });
            }
        }
        let child = result.ok_or_else(|| format!("missing already attached {wanted} node"))?;
        child.check(parent)?;
        ctx.check()?;
        Ok(child)
    }

    fn check(&self, parent: &Node) -> Result<(), String> {
        if parent.count()? == self.count
            && parent.at(self.index) == self.node.raw()
            && self.node.parent() == parent.raw()
        {
            return Ok(());
        }
        Err("retained direct parent and child changed".into())
    }
}

struct Parameter {
    controller: Owned,
    parameter: Owned,
    owning_interface: Owned,
    id: *const PlainCString,
}

impl Parameter {
    fn capture(ctx: &Context<'_>, owner: &Node, name: &CStr) -> Result<Self, String> {
        ctx.check()?;
        let controller = query(owner.raw(), &CONTROLLER).ok_or("fixed controller unavailable")?;
        let id = unsafe { (ctx.api.cstring)(name.as_ptr()) };
        if id.is_null() {
            return Err("fixed parameter name unavailable".into());
        }
        let pointer = Self::find(controller.0, id);
        let parameter = hold(pointer).ok_or("fixed controller parameter unavailable")?;
        let owner_of: Borrowed = unsafe { method(parameter.0, 14) };
        let owning_interface =
            hold(unsafe { owner_of(parameter.0) }).ok_or("parameter owner unavailable")?;
        let owning_identity =
            query(owning_interface.0, &UNKNOWN).ok_or("parameter owner identity unavailable")?;
        if owning_identity.0 != owner.identity.0 {
            return Err("fixed parameter owner disagrees".into());
        }
        drop(owning_identity);
        let result = Self {
            controller,
            parameter,
            owning_interface,
            id,
        };
        result.check()?;
        ctx.check()?;
        Ok(result)
    }

    fn find(controller: *mut c_void, id: *const PlainCString) -> *mut c_void {
        let get: unsafe extern "C" fn(*mut c_void, *const PlainCString) -> *mut c_void =
            unsafe { method(controller, 5) };
        unsafe { get(controller, id) }
    }

    fn check(&self) -> Result<(), String> {
        let owner_of: Borrowed = unsafe { method(self.parameter.0, 14) };
        if Self::find(self.controller.0, self.id) != self.parameter.0
            || unsafe { owner_of(self.parameter.0) } != self.owning_interface.0
        {
            return Err("retained fixed parameter changed".into());
        }
        Ok(())
    }
}

struct ScopeEndpoint {
    slot: Node,
    primary: *mut c_void,
    effect: Owned,
    _identity: Owned,
    get_effect: Borrowed,
    image: usize,
}

impl ScopeEndpoint {
    fn capture(ctx: &Context<'_>) -> Result<Self, String> {
        ctx.check()?;
        let header = unsafe { _dyld_get_image_header(0) };
        if image_uuid(header) != Some(EXPECTED_APP_UUID) {
            return Err("The Scope sidechain read requires the verified image".into());
        }
        let image = header as usize;
        let parent: Borrowed = unsafe { method(ctx.node.0, 6) };
        let slot = Node::capture(unsafe { parent(ctx.node.0) })?;
        if class_name(slot.raw()) != "AudioInsertSlot"
            || slot.raw().cast::<u8>().wrapping_sub(0x18).cast() != slot.identity.0
        {
            return Err("The Scope insert slot interface association changed".into());
        }
        let (primary_table, _, _) = INSERT_SLOT_LAYOUTS
            .iter()
            .copied()
            .find(|&(_, unknown, node)| {
                table(slot.identity.0) == image + unknown && table(slot.raw()) == image + node
            })
            .ok_or("The Scope insert slot tables are unverified")?;
        let primary: *mut c_void = slot.raw().cast::<u8>().wrapping_sub(0x30).cast();
        if table(primary) != image + primary_table {
            return Err("The Scope insert slot primary changed".into());
        }
        let get_effect: Borrowed = unsafe { method(primary, 0x200 / 8) };
        if get_effect as usize != image + 0xd75a0c {
            return Err("The current Scope effect getter is unverified".into());
        }
        let effect =
            hold(unsafe { get_effect(primary) }).ok_or("The live Scope effect detached")?;
        let target = query(ctx.target.0, &UNKNOWN).ok_or("The Scope target identity detached")?;
        let identity =
            query(effect.0, &UNKNOWN).ok_or("The exact Scope effect identity detached")?;
        if identity.0 != target.0 {
            return Err("The native getter returned a different effect".into());
        }
        let result = Self {
            slot,
            primary,
            effect,
            _identity: identity,
            get_effect,
            image,
        };
        result.check(ctx)?;
        Ok(result)
    }

    fn check(&self, ctx: &Context<'_>) -> Result<(), String> {
        ctx.check()?;
        let parent: Borrowed = unsafe { method(ctx.node.0, 6) };
        if unsafe { parent(ctx.node.0) } != self.slot.raw()
            || unsafe { (self.get_effect)(self.primary) } != self.effect.0
        {
            return Err("The retained Scope slot or effect changed".into());
        }
        ctx.check()
    }

    fn independent_input(&self, ctx: &Context<'_>) -> Result<RetainedPort, String> {
        self.check(ctx)?;
        const EFFECT_PRIMARY: usize = 0x1b769b0;
        const AUDIO_INPUT_PRIMARY: usize = 0x1b72270;
        let effect = self.effect.0.cast::<u8>();
        if table(effect.cast()) != self.image + EFFECT_PRIMARY {
            return Err("The exact Scope effect layout is unavailable".into());
        }
        let shape = || unsafe {
            // The pinned AudioEffect constructor and stock 0x100ba651c agree on these fields.
            (
                effect.add(0x1d8).cast::<*mut *mut c_void>().read(),
                effect.add(0x1e0).cast::<i32>().read(),
            )
        };
        let (items, count) = shape();
        if items.is_null() || !(1..=8).contains(&count) {
            return Err("Scope input collection is outside the fixed bound".into());
        }
        let mut entries = Vec::with_capacity(count as usize);
        let mut selected = None;
        for index in 0..count as usize {
            let primary = unsafe { items.add(index).read() };
            if primary.is_null() || entries.contains(&primary) {
                return Err("Scope input collection has a missing or duplicate entry".into());
            }
            entries.push(primary);
            let layout = table(primary).wrapping_sub(self.image);
            if !matches!(layout, SIDECHAIN_PRIMARY | AUDIO_INPUT_PRIMARY) {
                return Err("The Scope input collection is outside the verified layouts".into());
            }
            let kind = unsafe { primary.cast::<u8>().add(0x40).cast::<u32>().read() };
            if kind == 1 {
                if layout != SIDECHAIN_PRIMARY || selected.replace(primary).is_some() {
                    return Err(
                        "Scope input collection has multiple or unverified sidechain inputs".into(),
                    );
                }
            } else if layout == SIDECHAIN_PRIMARY {
                return Err("The sidechain input has an unverified kind".into());
            }
        }
        let primary = selected
            .ok_or("The live Scope has no unique sidechain input")?
            .cast::<u8>();
        let view: *mut c_void = primary.wrapping_add(0x30).cast();
        if table(view) != self.image + SIDECHAIN_PORT_VIEW {
            return Err("Unverified current Scope sidechain port facade".into());
        }
        let canonical =
            query(view, &UNKNOWN).ok_or("Scope input canonical identity unavailable")?;
        let retained = RetainedPort { view, canonical };
        self.check(ctx)?;
        if shape() != (items, count)
            || !entries
                .iter()
                .enumerate()
                .all(|(index, &entry)| unsafe { items.add(index).read() } == entry)
            || !self.owns(&retained)?
        {
            return Err("The independently retained Scope input or its owner changed".into());
        }
        self.check(ctx)?;
        Ok(retained)
    }

    fn owns(&self, observed: &RetainedPort) -> Result<bool, String> {
        if table(observed.view) != self.image + SIDECHAIN_PORT_VIEW {
            return Ok(false);
        }
        if observed.view.cast::<u8>().wrapping_sub(0x18).cast() != observed.canonical.0
            || table(observed.canonical.0) != self.image + SIDECHAIN_UNKNOWN
        {
            return Err("The assigned sidechain port canonical view changed".into());
        }
        let primary = observed.view.cast::<u8>().wrapping_sub(0x30);
        if table(primary.cast()) != self.image + SIDECHAIN_PRIMARY {
            return Err("The assigned sidechain port primary view changed".into());
        }
        let (base, notify) = unsafe {
            (
                primary.add(0x60).cast::<*mut c_void>().read(),
                primary.add(0x448).cast::<*mut c_void>().read(),
            )
        };
        if base != notify || base.is_null() {
            return Err("The assigned sidechain port ownership disagrees".into());
        }
        Ok(base == self.effect.0)
    }
}

fn sole_send_slot(
    classes: impl IntoIterator<Item = impl AsRef<str>>,
) -> Result<Option<usize>, String> {
    let mut slot = None;
    for (index, class) in classes.into_iter().enumerate() {
        match class.as_ref() {
            "" => return Err("source Sends rack has an untyped child".into()),
            "AudioSendSlot" if slot.replace(index).is_some() => {
                return Err("source Sends rack has more than one AudioSendSlot".into());
            }
            _ => {}
        }
    }
    Ok(slot)
}

struct Send {
    index: usize,
    parameter: Parameter,
    object: Owned,
    primary: *mut c_void,
    read_port: Borrowed,
}

struct Source {
    document_environment: DocumentEnvironmentRoute,
    ancestry: Vec<Node>,
    rack: Child,
    rack_children: Vec<Node>,
    send: Option<Send>,
}

impl Source {
    fn capture(ctx: &Context<'_>, source: Owned, image: usize) -> Result<Self, String> {
        let mut ancestry = vec![Node::capture(source.0)?];
        if class_name(ancestry[0].raw()) != "AudioTrackChannel" {
            return Err("source is not the exact native AudioTrackChannel family".into());
        }
        let document_environment =
            DocumentEnvironmentRoute::capture_for_document(&ctx.document.manager, &mut || {
                ctx.check()
            })?;
        let wanted = document_environment.canonical_keys()[2];
        for _ in 0..8 {
            ctx.check()?;
            let node = ancestry.last().expect("source nonempty");
            if node.identity.0 as usize == wanted {
                break;
            }
            let parent = Node::capture(node.parent())
                .map_err(|_| "source channel is outside the current document Environment")?;
            if ancestry
                .iter()
                .any(|node| node.identity.0 == parent.identity.0)
            {
                return Err("source ancestry cycle".into());
            }
            ancestry.push(parent);
        }
        if ancestry
            .last()
            .is_none_or(|node| node.identity.0 as usize != wanted)
        {
            return Err("source channel did not reach the current document Environment".into());
        }
        let rack = Child::find(ctx, &ancestry[0], "Sends", "AudioSendFolder")?;
        let count = rack.node.count()?;
        let rack_children = (0..count)
            .map(|index| {
                ctx.check()?;
                let node = Node::capture(rack.node.at(index))?;
                if node.parent() != rack.node.raw() {
                    return Err("source Sends child did not retain the exact parent".into());
                }
                Ok(node)
            })
            .collect::<Result<Vec<_>, String>>()?;
        let classes = rack_children.iter().map(|node| class_name(node.raw()));
        let send = match sole_send_slot(classes)? {
            None => None,
            Some(index) => {
                let slot = &rack_children[index];
                let parameter = Parameter::capture(ctx, slot, c"port")?;
                if class_name(parameter.parameter.0) != "PortParam" {
                    return Err("source port parameter is not exact PortParam".into());
                }
                Some(Send::capture(index, parameter, image)?)
            }
        };
        drop(source);
        let result = Self {
            document_environment,
            ancestry,
            rack,
            rack_children,
            send,
        };
        result.check(ctx)?;
        Ok(result)
    }

    fn check(&self, ctx: &Context<'_>) -> Result<(), String> {
        self.document_environment.check()?;
        for edge in self.ancestry.windows(2) {
            if edge[0].parent() != edge[1].raw() {
                return Err("retained source channel ancestry changed".into());
            }
        }
        self.rack.check(&self.ancestry[0])?;
        if self.rack.node.count()? as usize != self.rack_children.len() {
            return Err("retained Sends rack membership changed".into());
        }
        for (index, node) in self.rack_children.iter().enumerate() {
            if self.rack.node.at(index as i32) != node.raw()
                || node.parent() != self.rack.node.raw()
            {
                return Err("retained Sends rack child changed".into());
            }
        }
        if let Some(send) = &self.send {
            send.parameter.check()?;
        }
        ctx.check()
    }
}

struct RetainedPort {
    view: *mut c_void,
    canonical: Owned,
}

impl Send {
    fn capture(index: usize, parameter: Parameter, image: usize) -> Result<Self, String> {
        let object =
            query(parameter.parameter.0, &OBJECT).ok_or("source PortParam IObject unavailable")?;
        // The pinned PortParam IObject dispatch thunk adjusts the receiver by -0x28.
        if table(object.0) != image + 0x1c2ec38
            || unsafe { method::<Borrowed>(object.0, 7) } as usize != image + 0xf10bd0
        {
            return Err("source PortParam IObject layout is unverified".into());
        }
        let primary: *mut c_void = object.0.cast::<u8>().wrapping_sub(0x28).cast();
        if table(primary) != image + 0x1c2e8e8 {
            return Err("source PortParam primary layout is unverified".into());
        }
        let read_port: Borrowed = unsafe { method(primary, 0x268 / 8) };
        if read_port as usize != image + 0xf0ec8c {
            return Err("source PortParam fixed getter is unverified".into());
        }
        Ok(Self {
            index,
            parameter,
            object,
            primary,
            read_port,
        })
    }

    fn assignment(&self) -> Result<Option<RetainedPort>, String> {
        self.parameter.check()?;
        let object = query(self.parameter.parameter.0, &OBJECT)
            .ok_or("retained PortParam IObject unavailable")?;
        if object.0 != self.object.0 {
            return Err("retained PortParam IObject changed".into());
        }
        let view = unsafe { (self.read_port)(self.primary) };
        if view.is_null() {
            return Ok(None);
        }
        // The PortParam holds the borrowed port. Retain its canonical identity immediately.
        let canonical = query(view, &UNKNOWN).ok_or("assigned port identity unavailable")?;
        self.parameter.check()?;
        Ok(Some(RetainedPort { view, canonical }))
    }
}

struct Ports {
    assignment: Option<RetainedPort>,
    exact_destination: bool,
}

impl Ports {
    fn matches(&self, other: &Self) -> bool {
        let identity = |port: &RetainedPort| (port.view, port.canonical.0);
        self.exact_destination == other.exact_destination
            && self.assignment.as_ref().map(identity) == other.assignment.as_ref().map(identity)
    }

    fn classify(&self, source: &Source) -> &'static str {
        match self.assignment.as_ref() {
            Some(_) if self.exact_destination => "assigned_to_exact_scope",
            Some(_) => "assignment_mismatch",
            None if source.send.is_some() => "source_unassigned",
            None => "source_absent",
        }
    }
}

pub(super) struct Observation {
    input: Option<RetainedPort>,
    send_id: Option<String>,
    initial: Ports,
    source: Source,
    scope: ScopeEndpoint,
}

impl Observation {
    fn check_binding(
        ctx: &Context<'_>,
        source: &Source,
        scope: &ScopeEndpoint,
    ) -> Result<(), String> {
        scope.check(ctx)?;
        source.check(ctx)?;
        scope.check(ctx)
    }

    fn ports(ctx: &Context<'_>, source: &Source, scope: &ScopeEndpoint) -> Result<Ports, String> {
        Self::check_binding(ctx, source, scope)?;
        let assignment = source.send.as_ref().map_or(Ok(None), Send::assignment)?;
        Self::check_binding(ctx, source, scope)?;
        let exact_destination = assignment
            .as_ref()
            .map_or(Ok(false), |port| scope.owns(port))?;
        Self::check_binding(ctx, source, scope)?;
        Ok(Ports {
            assignment,
            exact_destination,
        })
    }

    fn current_send_id(source: &Source) -> Option<String> {
        source
            .send
            .as_ref()
            .and_then(|send| source.rack_children[send.index].object_id().ok())
    }

    pub(super) fn check(&self, ctx: &Context<'_>) -> Result<(), String> {
        let fresh = Self::ports(ctx, &self.source, &self.scope)?;
        if !self.initial.matches(&fresh) || Self::current_send_id(&self.source) != self.send_id {
            return Err("Scope sidechain assignment changed before publication".into());
        }
        if let Some(input) = &self.input {
            let current = self.scope.independent_input(ctx)?;
            if current.canonical.0 != input.canonical.0 || current.view != input.view {
                return Err("The Scope sidechain input changed before publication".into());
            }
        }
        Self::check_binding(ctx, &self.source, &self.scope)
    }

    pub(super) fn value(&self) -> Value {
        json!({
            "scope": "authored_native_assignment",
            "state": self.initial.classify(&self.source),
            "send_node": self.send_id,
        })
    }

    fn capture(ctx: &Context<'_>, source: Owned) -> Result<Self, String> {
        if ctx.instance.class_uid != SCOPE_CLASS || ctx.instance.native_class != "AudioEffect" {
            return Err("Scope assignment requires the exact native Scope instance".into());
        }
        let scope = ScopeEndpoint::capture(ctx)?;
        let source = Source::capture(ctx, source, scope.image)?;
        let initial = Self::ports(ctx, &source, &scope)?;
        let send_id = Self::current_send_id(&source);
        Ok(Self {
            input: None,
            send_id,
            initial,
            source,
            scope,
        })
    }

    fn bind_input(&mut self, ctx: &Context<'_>) -> Result<(), String> {
        let input = self.scope.independent_input(ctx)?;
        if self.initial.exact_destination
            && self
                .initial
                .assignment
                .as_ref()
                .is_none_or(|port| input.canonical.0 != port.canonical.0)
        {
            return Err(
                "The assigned Send does not bind the independently owned Scope input".into(),
            );
        }
        self.input = Some(input);
        self.check(ctx)
    }
}

#[repr(C, align(16))]
struct SendDescriptorStorage([u8; 0x120]);

#[repr(C, align(16))]
struct EmptyAttributesStorage([u8; 0x10]);

struct HostStorage {
    ptr: *mut c_void,
    finish: unsafe extern "C" fn(*mut c_void),
}

impl Drop for HostStorage {
    fn drop(&mut self) {
        unsafe { (self.finish)(self.ptr) };
    }
}

fn record_send_edit<T>(
    ctx: &Context<'_>,
    image: usize,
    owner: *mut c_void,
    check: impl Fn() -> Result<(), String>,
    edit: impl FnOnce(*mut c_void) -> Result<T, String>,
) -> Result<T, String> {
    check()?;
    undo_available(image, owner)?;

    let begin: unsafe extern "C" fn(*mut c_void, *mut c_void) =
        unsafe { std::mem::transmute(image + 0xd6c290) };
    let begin_group: unsafe extern "C" fn(*mut c_void, *const PlainString, u8) -> *mut c_void =
        unsafe { std::mem::transmute(image + 0xd6c924) };
    let finish: unsafe extern "C" fn(*mut c_void) =
        unsafe { std::mem::transmute(image + 0xd6c4a4) };
    let string = crate::constant_string().ok_or("Native host Strings are unavailable")?;
    let empty = unsafe { string(c"".as_ptr()) };
    if empty.is_null() {
        return Err("An Undo label is unavailable".into());
    }
    let mut storage = MaybeUninit::<ScopeStorage>::uninit();
    let scope = storage.as_mut_ptr().cast();
    unsafe { begin(scope, owner) };
    let recording = RecordingScope::new(scope, finish);
    recorded_submit(image, &recording, SLOT_EDIT_SCOPE)?;
    let current_journal = || {
        let manager = unsafe { field::<*mut c_void>(scope, 0x40) };
        if manager.is_null() {
            return std::ptr::null_mut();
        }
        let get: Borrowed = unsafe { method(manager, 3) };
        unsafe { get(manager) }
    };
    let raw_journal = current_journal();
    let journal =
        query(raw_journal, &ACTION_JOURNAL).ok_or("The Send Undo journal is unavailable")?;
    let last_edit: unsafe extern "C" fn(*mut c_void) -> i64 = unsafe { method(journal.0, 3) };
    let performing: unsafe extern "C" fn(*mut c_void) -> u8 = unsafe { method(journal.0, 8) };
    if unsafe { performing(journal.0) } != 0 {
        return Err("The Send Undo journal is busy".into());
    }
    let initial_edit = unsafe { last_edit(journal.0) };
    check()?;
    let group = unsafe { begin_group(scope, empty, 0) };
    if group.is_null() || group != unsafe { field::<*mut c_void>(image as *mut c_void, 0x1e834b0) }
    {
        return Err("The host did not open a recorded Send Undo group".into());
    }
    recorded_submit(image, &recording, SLOT_EDIT_SCOPE)?;
    check()?;
    if current_journal() != raw_journal {
        return Err("The Send Undo journal changed before dispatch".into());
    }
    let outcome = edit(scope);
    drop(recording);
    ctx.check()?;
    if outcome.is_ok() && unsafe { last_edit(journal.0) } == initial_edit {
        return Err("The host did not record the exact Send edit".into());
    }
    outcome
}

pub(super) fn dispatch_create(ctx: Box<Context<'static>>, source: Option<Owned>) -> Reply {
    let mut dispatched = false;
    let result = (|| -> Result<Observation, String> {
        let source = source.ok_or("The current source channel is unavailable")?;
        let fresh_source = query(source.0, &UNKNOWN).ok_or("The exact source channel detached")?;
        let mut before = Observation::capture(&ctx, source)?;
        if before.source.send.is_some() {
            return Err("The exact source already has a Send".into());
        }
        before.bind_input(&ctx)?;
        let input = before
            .input
            .as_ref()
            .ok_or("The Scope input is unavailable")?;
        let image = before.scope.image;
        let rack = send_primary(&before.source.rack.node, image, SEND_RACK_LAYOUT)?;
        let check_empty = || -> Result<(), String> {
            before.check(&ctx)?;
            if unsafe { field::<u8>(rack, 0x2ec) } & 0x60 != 0x20 {
                return Err("The exact Send rack does not support a recorded addition".into());
            }
            let factory: Borrowed = unsafe { method(rack, 0x210 / 8) };
            if factory as usize != image + 0xc7647c {
                return Err("The installed Send rack factory is unverified".into());
            }
            Ok(())
        };
        check_empty()?;
        let port_view: *mut c_void = input.view.cast::<u8>().wrapping_sub(8).cast();
        if table(port_view) != image + 0x1b776e0
            || query(port_view, &UNKNOWN).is_none_or(|id| id.0 != input.canonical.0)
        {
            return Err("The stock AudioInput argument view is unverified".into());
        }

        let init_desc: unsafe extern "C" fn(*mut c_void, *mut c_void) =
            unsafe { std::mem::transmute(image + 0x14a32e0) };
        let finish_desc: unsafe extern "C" fn(*mut c_void) =
            unsafe { std::mem::transmute(image + 0x14a3388) };
        let init_attr: unsafe extern "C" fn(*mut c_void, *mut c_void) =
            unsafe { std::mem::transmute(image + 0x15eec54) };
        let finish_attr: unsafe extern "C" fn(*mut c_void) =
            unsafe { std::mem::transmute(image + 0x15eec9c) };
        let mut desc_memory = MaybeUninit::<SendDescriptorStorage>::uninit();
        let desc_ptr = desc_memory.as_mut_ptr().cast();
        unsafe { init_desc(desc_ptr, std::ptr::null_mut()) };
        let descriptor = HostStorage {
            ptr: desc_ptr,
            finish: finish_desc,
        };
        let descriptor_interface = desc_ptr.cast::<u8>().wrapping_add(0x30).cast();
        if table(desc_ptr) != image + 0x1d49670
            || table(descriptor_interface) != image + 0x1d498a0
            || !unsafe { field::<*mut c_void>(desc_ptr, 0x50) }.is_null()
        {
            return Err("The stock Send descriptor changed".into());
        }
        let descriptor_input = hold(port_view).ok_or("The stock descriptor input detached")?;
        unsafe {
            desc_ptr
                .cast::<u8>()
                .add(0x50)
                .cast::<*mut c_void>()
                .write(descriptor_input.0)
        };
        std::mem::forget(descriptor_input);
        let mut attr_memory = MaybeUninit::<EmptyAttributesStorage>::uninit();
        let attr_ptr = attr_memory.as_mut_ptr().cast();
        unsafe { init_attr(attr_ptr, std::ptr::null_mut()) };
        let attributes = HostStorage {
            ptr: attr_ptr,
            finish: finish_attr,
        };

        let add: unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            *mut c_void,
            i32,
            *mut c_void,
            *mut c_void,
        ) -> *mut c_void = unsafe { std::mem::transmute(image + 0xd6cc98) };
        let (created, id) = record_send_edit(&ctx, image, rack, check_empty, |scope| {
            if unsafe { field::<*mut c_void>(desc_ptr, 0x50) } != port_view {
                return Err("The Send descriptor changed before creation".into());
            }
            dispatched = true;
            let slot = unsafe {
                add(
                    scope,
                    rack,
                    descriptor_interface,
                    -1,
                    std::ptr::null_mut(),
                    attr_ptr,
                )
            };
            if slot.is_null() || table(slot) != image + SEND_LAYOUT[0] {
                return Err("The stock Send creation did not return an exact Send slot".into());
            }
            let created = Node::capture(slot)?;
            if send_primary(&created, image, SEND_LAYOUT)? != slot
                || created.parent() != before.source.rack.node.raw()
                || unsafe { field::<*mut c_void>(slot, 0x58) } != rack
            {
                return Err("The newly created Send belongs to a different rack".into());
            }
            let id = created.object_id()?;
            drop(attributes);
            drop(descriptor);
            let level: unsafe extern "C" fn(*mut c_void, f32) = unsafe { method(slot, 0x3e0 / 8) };
            if level as usize != image + 0xc6f1fc {
                return Err("The created Send's stock 0 dB setter is unverified".into());
            }
            ctx.check()?;
            if created.parent() != before.source.rack.node.raw() || created.object_id()? != id {
                return Err("The new Send detached before its stock level could be set".into());
            }
            unsafe { level(slot, 1.0) };
            Ok((created, id))
        })?;
        let mut after = Observation::capture(&ctx, fresh_source)?;
        after.bind_input(&ctx)?;
        let other_children = after
            .source
            .rack_children
            .iter()
            .filter(|child| child.identity.0 != created.identity.0)
            .map(|child| child.identity.0);
        if after.send_id.as_deref() != Some(id.as_str())
            || !after.initial.exact_destination
            || after
                .input
                .as_ref()
                .is_none_or(|port| port.canonical.0 != input.canonical.0)
            || after.source.rack.node.identity.0 != before.source.rack.node.identity.0
            || after.source.rack_children.len() != before.source.rack_children.len() + 1
            || !other_children.eq(before
                .source
                .rack_children
                .iter()
                .map(|child| child.identity.0))
        {
            return Err(
                "The new Send or Scope endpoint did not satisfy the exact postcondition".into(),
            );
        }
        after.check(&ctx)?;
        Ok(after)
    })();
    let mut reply = match result {
        Ok(after) => {
            let mut reply = Reply::unbound(json!({"ok": true, "outcome": "applied_unverified",
                "operation": "scope_sidechain_assignment_create", "instance": ctx.instance,
                "created_send": after.send_id, "recorded": true}));
            reply.auxiliary = Some(node_inspect::Auxiliary::ScopeSidechain(Box::new(after)));
            reply
        }
        Err(reason) => Reply::error(
            "scope_sidechain_assignment_create_failed",
            &reason,
            if dispatched { "unknown" } else { "not_applied" },
        ),
    };
    reply.mutation = dispatched;
    reply.context = Some(ctx);
    reply
}

pub(super) fn dispatch_remove(
    ctx: Box<Context<'static>>,
    source: Option<Owned>,
    expected_send: &str,
) -> Reply {
    let mut dispatched = false;
    let result = (|| -> Result<Observation, String> {
        let source = source.ok_or("The current source channel is unavailable")?;
        let fresh_source = query(source.0, &UNKNOWN).ok_or("The exact source channel detached")?;
        let mut before = Observation::capture(&ctx, source)?;
        if before.send_id.as_deref() != Some(expected_send) || !before.initial.exact_destination {
            return Err("The exact Send UUID or Scope destination changed".into());
        }
        before.bind_input(&ctx)?;
        let scope = &before.scope;
        let source = &before.source;
        let image = scope.image;
        let send = source
            .send
            .as_ref()
            .ok_or("There is no sole Send to remove")?;
        let node = &source.rack_children[send.index];
        let slot = send_primary(node, image, SEND_LAYOUT)?;
        let rack = send_primary(&source.rack.node, image, SEND_RACK_LAYOUT)?;
        let native_folder: Borrowed = unsafe { std::mem::transmute(image + 0xd737f4) };
        let check_bound = || -> Result<(), String> {
            before.check(&ctx)?;
            if node.object_id()? != expected_send
                || node.parent() != source.rack.node.raw()
                || unsafe { field::<*mut c_void>(slot, 0x58) } != rack
                || unsafe { native_folder(slot) } != rack
            {
                return Err("The Send is no longer bound to its exact source rack".into());
            }
            let slot_flags = unsafe { field::<u8>(slot, 0x1d0) };
            let rack_flags = unsafe { field::<u8>(rack, 0x2ec) };
            if rack_flags & 0x60 != 0x20 || slot_flags & 8 != 0 {
                return Err("The exact Send does not support recorded removal".into());
            }
            ctx.check()
        };
        let remove: unsafe extern "C" fn(*mut c_void, *mut c_void) -> u8 =
            unsafe { std::mem::transmute(image + 0xd6d27c) };
        record_send_edit(&ctx, image, slot, check_bound, |scope| {
            dispatched = true;
            if unsafe { remove(scope, slot) } & 1 != 1 {
                return Err("The stock host did not accept the Send removal".into());
            }
            Ok(())
        })?;
        let mut after = Observation::capture(&ctx, fresh_source)?;
        after.bind_input(&ctx)?;
        let siblings = source
            .rack_children
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != send.index)
            .map(|(_, sibling)| sibling.identity.0);
        if after.source.send.is_some()
            || after.input.as_ref().map(|port| port.canonical.0)
                != before.input.as_ref().map(|port| port.canonical.0)
            || after.source.rack.node.identity.0 != source.rack.node.identity.0
            || !after
                .source
                .rack_children
                .iter()
                .map(|child| child.identity.0)
                .eq(siblings)
        {
            return Err(
                "The source rack or independent Scope endpoint changed after deletion".into(),
            );
        }
        after.check(&ctx)?;
        Ok(after)
    })();
    let mut reply = match result {
        Ok(after) => {
            let mut reply = Reply::unbound(json!({"ok": true, "outcome": "applied_unverified",
                "operation": "scope_sidechain_assignment_remove", "instance": ctx.instance,
                "removed_send": expected_send, "recorded": true}));
            reply.auxiliary = Some(node_inspect::Auxiliary::ScopeSidechain(Box::new(after)));
            reply
        }
        Err(reason) => Reply::error(
            "scope_sidechain_assignment_remove_failed",
            &reason,
            if dispatched { "unknown" } else { "not_applied" },
        ),
    };
    reply.mutation = dispatched;
    reply.context = Some(ctx);
    reply
}

pub(super) fn dispatch(ctx: Box<Context<'static>>, source: Option<Owned>) -> Reply {
    let observed = source
        .ok_or_else(|| "The current source audio channel is unavailable".into())
        .and_then(|source| Observation::capture(&ctx, source));
    let mut reply = match observed {
        Ok(observation) => {
            let mut value = observation.value();
            value["ok"] = json!(true);
            value["operation"] = json!("scope_sidechain_assignment_get");
            value["outcome"] = json!("observed");
            value["instance"] = json!(ctx.instance);
            let mut reply = Reply::unbound(value);
            reply.auxiliary = Some(node_inspect::Auxiliary::ScopeSidechain(Box::new(
                observation,
            )));
            reply
        }
        Err(reason) => Reply::error(
            "scope_sidechain_assignment_unavailable",
            &reason,
            "not_applied",
        ),
    };
    reply.context = Some(ctx);
    reply
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_a_single_typed_send_and_refuses_ambiguous_ownership() {
        assert_eq!(sole_send_slot(["FolderChild", "PanelNode"]).unwrap(), None);
        assert_eq!(
            sole_send_slot(["FolderChild", "AudioSendSlot", "PanelNode"]).unwrap(),
            Some(1)
        );
        assert!(sole_send_slot(["AudioSendSlot", "PanelNode", "AudioSendSlot"]).is_err());
        assert!(sole_send_slot(["FolderChild", ""]).is_err());
    }
}

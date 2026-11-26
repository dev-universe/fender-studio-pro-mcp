use super::*;
use node_inspect::DocumentEnvironmentRoute;
use std::ffi::CStr;

const SCOPE_CLASS: &str = "36322412-AD90-4EE0-B63C-37929566B081";
const MAX_CHILDREN: i32 = 32;
const INSERT_SLOT_LAYOUTS: [(usize, usize, usize); 2] = [
    (0x1b78948, 0x1b78d78, 0x1b78e40),
    (0x1b90b50, 0x1b90f80, 0x1b91048),
];
const SIDECHAIN_PRIMARY: usize = 0x1b77470;
const SIDECHAIN_UNKNOWN: usize = 0x1b77668;
const SIDECHAIN_PORT_VIEW: usize = 0x1b77730;

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
                Some(Send::capture(parameter, image)?)
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
    fn capture(parameter: Parameter, image: usize) -> Result<Self, String> {
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

    pub(super) fn check(&self, ctx: &Context<'_>) -> Result<(), String> {
        let fresh = Self::ports(ctx, &self.source, &self.scope)?;
        if !self.initial.matches(&fresh) {
            return Err("Scope sidechain assignment changed before publication".into());
        }
        Self::check_binding(ctx, &self.source, &self.scope)
    }

    pub(super) fn value(&self) -> Value {
        json!({
            "scope": "authored_native_assignment",
            "state": self.initial.classify(&self.source),
        })
    }

    fn capture(ctx: &Context<'_>, source: Owned) -> Result<Self, String> {
        if ctx.instance.class_uid != SCOPE_CLASS || ctx.instance.native_class != "AudioEffect" {
            return Err("Scope assignment requires the exact native Scope instance".into());
        }
        let scope = ScopeEndpoint::capture(ctx)?;
        let source = Source::capture(ctx, source, scope.image)?;
        let initial = Self::ports(ctx, &source, &scope)?;
        Ok(Self {
            initial,
            source,
            scope,
        })
    }
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

//! Synchronous delivery of Studio Pro's fixed Splitter remove message.
use super::ownership::GraphChildKind;
use super::*;
use std::rc::Rc;

const COMMAND_HANDLER: Uid = Uid {
    a: 0xcb8108a9,
    b: 0xdc88,
    c: 0x4152,
    d: [0xad, 0xe0, 0xc3, 0xae, 0x48, 0x9d, 0x75, 0xb5],
};
const OBSERVER: Uid = Uid {
    a: 0xfbb66ab6,
    b: 0xcdf0,
    c: 0x4e39,
    d: [0x8e, 0x8e, 0x0c, 0xb9, 0x9f, 0x06, 0x2c, 0x46],
};
const MESSAGE: Uid = Uid {
    a: 0xd943e242,
    b: 0xf8c7,
    c: 0x4ed8,
    d: [0x80, 0x5c, 0x4c, 0x47, 0x85, 0x46, 0xb1, 0x14],
};
const GRAPH_TABLE: usize = 0x1b7abb8;
const SPLITTER_TABLE: usize = 0x1b7b110;
const MERGER_TABLE: usize = 0x1b7b6e8;
const SLOT_TABLES: [usize; 2] = [0x1be18d8, 0x1bccdb8];

#[repr(C)]
struct CheckCommand {
    category: PlainCString,
    name: PlainCString,
    invoker: *mut c_void,
    flags: i32,
    padding: i32,
}
const _: () = {
    assert!(std::mem::size_of::<CheckCommand>() == 48);
    assert!(std::mem::offset_of!(CheckCommand, flags) == 40);
};

#[repr(C)]
struct MessageVtable {
    base: UnknownVtable,
    id: unsafe extern "C" fn(*mut c_void) -> *const PlainCString,
    count: unsafe extern "C" fn(*mut c_void) -> i32,
    arg: unsafe extern "C" fn(*mut c_void, i32) -> *const c_void,
}

#[repr(C)]
struct RemoveMessage {
    vtable: &'static MessageVtable,
    id: PlainCString,
}

unsafe extern "C" fn message_query(
    this: *mut c_void,
    iid: *const Uid,
    result: *mut *mut c_void,
) -> i32 {
    if result.is_null() {
        return NO_INTERFACE;
    }
    unsafe { *result = std::ptr::null_mut() };
    if iid.is_null() || !matches!(unsafe { *iid }, UNKNOWN | MESSAGE) {
        return NO_INTERFACE;
    }
    unsafe {
        message_retain(this);
        *result = this;
    }
    OK
}

unsafe extern "C" fn message_retain(this: *mut c_void) -> u32 {
    let raw = this.cast::<RemoveMessage>();
    unsafe { Rc::increment_strong_count(raw) };
    let owner = std::mem::ManuallyDrop::new(unsafe { Rc::from_raw(raw) });
    Rc::strong_count(&owner) as u32
}

unsafe extern "C" fn message_release(this: *mut c_void) -> u32 {
    let owner = unsafe { Rc::from_raw(this.cast::<RemoveMessage>()) };
    let remaining = Rc::strong_count(&owner) - 1;
    drop(owner);
    remaining as u32
}

unsafe extern "C" fn message_id(this: *mut c_void) -> *const PlainCString {
    unsafe { &(*this.cast::<RemoveMessage>()).id }
}

unsafe extern "C" fn message_count(_: *mut c_void) -> i32 {
    0
}

unsafe extern "C" fn message_arg(_: *mut c_void, _: i32) -> *const c_void {
    static EMPTY: [u64; 2] = [0; 2];
    EMPTY.as_ptr().cast()
}

static REMOVE_MESSAGE: MessageVtable = MessageVtable {
    base: UnknownVtable {
        query: message_query,
        retain: message_retain,
        release: message_release,
    },
    id: message_id,
    count: message_count,
    arg: message_arg,
};

fn cstring(api: &Api, name: &std::ffi::CStr) -> Result<PlainCString, String> {
    unsafe { (api.cstring)(name.as_ptr()).as_ref().copied() }
        .ok_or_else(|| "Required Studio Pro command string unavailable".into())
}

unsafe fn field<T: Copy>(object: *mut c_void, offset: usize) -> T {
    unsafe { object.cast::<u8>().add(offset).cast::<T>().read_unaligned() }
}

fn table(object: *mut c_void) -> usize {
    unsafe { field::<usize>(object, 0) }
}

fn adjacent(object: *mut c_void, pointer: usize, count: usize) -> Result<Vec<*mut c_void>, String> {
    bounded_links(object, pointer, count, 1)
}

fn bounded_links(
    object: *mut c_void,
    pointer: usize,
    count: usize,
    minimum: i32,
) -> Result<Vec<*mut c_void>, String> {
    let count = unsafe { field::<i32>(object, count) };
    if count == 0 && minimum == 0 {
        return Ok(Vec::new());
    }
    let members = unsafe { field::<*const *mut c_void>(object, pointer) };
    if !(minimum..=2).contains(&count) || members.is_null() {
        return Err("The expected narrow nested routing edge changed".into());
    }
    let result = unsafe { std::slice::from_raw_parts(members, count as usize) }.to_vec();
    if result.iter().any(|node| node.is_null()) {
        return Err("A required routing edge has no bound node".into());
    }
    Ok(result)
}

fn bound_nodes(ctx: &Context<'_>, route: &GraphRoute, ids: &[&str]) -> Result<Vec<Owned>, String> {
    let graph = route.graph.node.0;
    let count: Count = unsafe { method(graph, 8) };
    let child: unsafe extern "C" fn(*mut c_void, i32) -> *mut c_void = unsafe { method(graph, 9) };
    let count = unsafe { count(graph) };
    if !(4..=32).contains(&count) {
        return Err("Graph child count is outside the verified range".into());
    }
    let mut targets = ids.iter().map(|_| None).collect::<Vec<_>>();
    for index in 0..count {
        ctx.check()?;
        let raw =
            hold(unsafe { child(graph, index) }).ok_or("A bound graph child is unavailable")?;
        let node = query(raw.0, &NODE).ok_or("A bound graph child has no native node")?;
        let get: unsafe extern "C" fn(*mut c_void) -> *const Uid = unsafe { method(node.0, 4) };
        let id = uid(unsafe { get(node.0) });
        if let Some(key) = ids.iter().position(|expected| *expected == id) {
            let parent: Borrowed = unsafe { method(node.0, 6) };
            if unsafe { parent(node.0) } != graph || targets[key].is_some() {
                return Err("Requested graph node is ambiguous or detached".into());
            }
            targets[key] = Some(query(raw.0, &UNKNOWN).ok_or("Graph node identity unavailable")?);
        }
        ctx.check()?;
    }
    targets
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| "The required nested Splitter nodes are unavailable".into())
}

fn matches_identity(object: *mut c_void, identity: &Owned) -> bool {
    query(object, &UNKNOWN).is_some_and(|observed| observed.0 == identity.0)
}

fn check_shape(
    image: usize,
    route: &GraphRoute,
    splitter: *mut c_void,
    nodes: &[Owned],
) -> Result<(), String> {
    let [_, merger_id, first_id, second_id, remaining @ ..] = nodes else {
        return Err("The exact Splitter and lane identities are unavailable".into());
    };
    // IUnknown is the comparison identity. The actual graph receiver is
    // reached through the primary Splitter object verified by the observer.
    let graph = unsafe { field::<*mut c_void>(splitter, 0x178) };
    if graph.is_null()
        || table(graph) != image + GRAPH_TABLE
        || !matches_identity(graph, &route.graph._canonical)
    {
        return Err("The Splitter is not attached to the verified active graph".into());
    }
    let branches = adjacent(splitter, 0x1a8, 0x1b0)?;
    let [first, second] = branches[..] else {
        return Err("The exact Splitter no longer has two branches".into());
    };
    let is_slot = |node| {
        SLOT_TABLES
            .iter()
            .any(|offset| table(node) == image + offset)
    };
    if !is_slot(first)
        || !is_slot(second)
        || !matches_identity(first, first_id)
        || !matches_identity(second, second_id)
    {
        return Err("The Splitter branches do not lead to the bound effect slots".into());
    }
    let first_end = match remaining {
        [] => first,
        [tail_id] => {
            let first_out = adjacent(first, 0x1a8, 0x1b0)?;
            let [tail] = first_out[..] else {
                return Err("The first branch has no unique second effect".into());
            };
            if !is_slot(tail)
                || !matches_identity(tail, tail_id)
                || unsafe { field::<*mut c_void>(tail, 0x178) } != graph
                || adjacent(tail, 0x188, 0x190)? != [first]
            {
                return Err("The first branch second effect changed".into());
            }
            tail
        }
        _ => return Err("The first branch contains more than two requested effects".into()),
    };
    let first_out = adjacent(first_end, 0x1a8, 0x1b0)?;
    let [merger] = first_out[..] else {
        return Err("The first Splitter branch has no exact merger".into());
    };
    if !matches_identity(merger, merger_id)
        || class_name(merger) != "AudioFXGraph::MergerNode"
        || [merger, first, second]
            .iter()
            .any(|node| unsafe { field::<*mut c_void>(*node, 0x178) } != graph)
        || adjacent(first, 0x188, 0x190)? != [splitter]
        || adjacent(second, 0x188, 0x190)? != [splitter]
        || adjacent(second, 0x1a8, 0x1b0)? != [merger]
    {
        return Err("The exact two-branch Splitter and merger shape changed".into());
    }
    let predecessor = adjacent(splitter, 0x188, 0x190)?;
    let successor = adjacent(merger, 0x1a8, 0x1b0)?;
    let merge_inputs = adjacent(merger, 0x188, 0x190)?;
    if !merge_inputs.contains(&first_end) || !merge_inputs.contains(&second) {
        return Err("Another branch now reaches the expected merger".into());
    }
    if predecessor.len() != 1
        || successor.len() != 1
        || table(predecessor[0]) != image + SPLITTER_TABLE
        || class_name(successor[0]) != "AudioFXGraph::MergerNode"
        || [predecessor[0], successor[0]]
            .iter()
            .any(|outer| unsafe { field::<*mut c_void>(*outer, 0x178) } != graph)
        || adjacent(predecessor[0], 0x1a8, 0x1b0)? != [splitter, successor[0]]
        || {
            let outer_inputs = adjacent(successor[0], 0x188, 0x190)?;
            !outer_inputs.contains(&merger) || !outer_inputs.contains(&predecessor[0])
        }
    {
        return Err("Only the verified nested Splitter connection can be removed".into());
    }
    Ok(())
}

const NESTED_SHAPE_CHANGED: &str = "The verified nested Splitter topology changed";

struct Links<Node> {
    incoming: Vec<Node>,
    outgoing: Vec<Node>,
}

fn same_incoming<Node: PartialEq>(actual: &[Node], expected: &[Node]) -> bool {
    actual.len() == expected.len() && expected.iter().all(|n| actual.contains(n))
}

fn nested_topology<Node: Copy + PartialEq>(
    [target, paired, preceding, following]: [Node; 4],
    occupied: Option<Node>,
    mut links: impl FnMut(Node) -> Result<Links<Node>, String>,
) -> Result<[Node; 8], String> {
    let preceding_links = links(preceding)?;
    let [inner] = preceding_links.incoming[..] else {
        return Err(NESTED_SHAPE_CHANGED.into());
    };
    let inner_links = links(inner)?;
    let ([outer], [left, other_lane]) = (&inner_links.incoming[..], &inner_links.outgoing[..])
    else {
        return Err(NESTED_SHAPE_CHANGED.into());
    };
    let outer = *outer;
    let peer = occupied.unwrap_or(*other_lane);
    let following_links = links(following)?;
    let [outer_merger] = following_links.outgoing[..] else {
        return Err(NESTED_SHAPE_CHANGED.into());
    };
    let nodes = [
        outer,
        inner,
        preceding,
        target,
        peer,
        outer_merger,
        paired,
        following,
    ];
    if *left != preceding
        || nodes
            .iter()
            .enumerate()
            .any(|(index, node)| nodes[..index].contains(node))
    {
        return Err(NESTED_SHAPE_CHANGED.into());
    }
    let target_links = links(target)?;
    let paired_links = links(paired)?;
    let peer_links = links(peer)?;
    let outer_links = links(outer)?;
    let outer_merger_links = links(outer_merger)?;
    let inner_lanes_match = if occupied.is_some() {
        *other_lane == following
            && (target_links.outgoing == [paired, peer] || target_links.outgoing == [peer, paired])
            && same_incoming(&paired_links.incoming, &[target, peer])
            && peer_links.incoming == [target]
            && peer_links.outgoing == [paired]
            && same_incoming(&following_links.incoming, &[paired, inner])
    } else {
        target_links.outgoing == [paired, paired]
            && (paired_links.incoming == [target] || paired_links.incoming == [target, target])
            && peer_links.incoming == [inner]
            && peer_links.outgoing == [following]
            && same_incoming(&following_links.incoming, &[paired, peer])
    };
    if preceding_links.outgoing != [target]
        || target_links.incoming != [preceding]
        || !inner_lanes_match
        || paired_links.outgoing != [following]
        || !outer_links.incoming.is_empty()
        || outer_links.outgoing != [inner, outer_merger]
        || !same_incoming(&outer_merger_links.incoming, &[outer, following])
        || !outer_merger_links.outgoing.is_empty()
    {
        return Err(NESTED_SHAPE_CHANGED.into());
    }
    Ok(nodes)
}

struct NestedMember<'a> {
    id: &'a str,
    primary: *mut c_void,
    table: usize,
    _child: Owned,
    _node: Owned,
    canonical: Owned,
}

struct GraphObjectLayout {
    primary: usize,
    node: usize,
    canonical: usize,
}

impl GraphObjectLayout {
    fn primary(
        &self,
        image: usize,
        node: *mut c_void,
        canonical: *mut c_void,
    ) -> Option<*mut c_void> {
        // The verified 8.1.1 QueryInterface paths return INode at +0x30
        // and IUnknown at +0x18. The two retained interfaces must agree
        // before recovering and checking the primary address.
        if node.is_null()
            || canonical.is_null()
            || node.cast::<u8>().wrapping_sub(0x18).cast() != canonical
            || table(node) != image + self.node
            || table(canonical) != image + self.canonical
        {
            return None;
        }
        let primary = unsafe { node.cast::<u8>().sub(0x30).cast() };
        (table(primary) == image + self.primary).then_some(primary)
    }
}

const GRAPH_LAYOUT: GraphObjectLayout = GraphObjectLayout {
    primary: GRAPH_TABLE,
    node: 0x1b7af00,
    canonical: 0x1b7ae38,
};
const SPLITTER_LAYOUT: GraphObjectLayout = GraphObjectLayout {
    primary: SPLITTER_TABLE,
    node: 0x1b7b4a8,
    canonical: 0x1b7b3e0,
};
const MERGER_LAYOUT: GraphObjectLayout = GraphObjectLayout {
    primary: MERGER_TABLE,
    node: 0x1b7ba80,
    canonical: 0x1b7b9b8,
};
const SLOT_LAYOUTS: [GraphObjectLayout; 2] = [
    GraphObjectLayout {
        primary: SLOT_TABLES[0],
        node: 0x1be1c70,
        canonical: 0x1be1ba8,
    },
    GraphObjectLayout {
        primary: SLOT_TABLES[1],
        node: 0x1bcd150,
        canonical: 0x1bcd088,
    },
];

struct NestedGraph<'a> {
    graph: *mut c_void,
    image: usize,
    members: Vec<NestedMember<'a>>,
}

impl<'a> NestedGraph<'a> {
    fn capture(
        ctx: &Context<'_>,
        route: &GraphRoute,
        image: usize,
        expected: [(&'a str, GraphChildKind); 8],
    ) -> Result<Self, String> {
        let graph_node = route.graph.node.0;
        let graph = GRAPH_LAYOUT
            .primary(image, graph_node, route.graph._canonical.0)
            .ok_or("The retained insert graph primary interface changed")?;
        let count: Count = unsafe { method(graph_node, 8) };
        let child: unsafe extern "C" fn(*mut c_void, i32) -> *mut c_void =
            unsafe { method(graph_node, 9) };
        ctx.check()?;
        route.check()?;
        if unsafe { count(graph_node) } != 8 {
            return Err(NESTED_SHAPE_CHANGED.into());
        }

        let mut members: Vec<NestedMember<'_>> = Vec::with_capacity(expected.len());
        for (index, (id, kind)) in expected.into_iter().enumerate() {
            ctx.check()?;
            route.check()?;
            let child = hold(unsafe { child(graph_node, index as i32) })
                .ok_or("The exact graph child detached")?;
            let node = query(child.0, &NODE).ok_or("The graph child has no INode interface")?;
            let canonical =
                query(child.0, &UNKNOWN).ok_or("The graph child has no canonical identity")?;
            let read_uid: unsafe extern "C" fn(*mut c_void) -> *const Uid =
                unsafe { method(node.0, 4) };
            let parent: Borrowed = unsafe { method(node.0, 6) };
            if uid(unsafe { read_uid(node.0) }) != id
                || unsafe { parent(node.0) } != graph_node
                || members
                    .iter()
                    .any(|member| member.canonical.0 == canonical.0)
            {
                return Err("The retained graph child identity or ownership changed".into());
            }
            let supported: &[GraphObjectLayout] = match kind {
                GraphChildKind::Splitter => &[SPLITTER_LAYOUT],
                GraphChildKind::Merger => &[MERGER_LAYOUT],
                GraphChildKind::Slot => &SLOT_LAYOUTS,
            };
            let primary = supported
                .iter()
                .find_map(|layout| layout.primary(image, node.0, canonical.0))
                .ok_or("The retained graph child primary interface changed")?;
            if members.iter().any(|member| member.primary == primary) {
                return Err("Two graph nodes share a primary pointer".into());
            }
            members.push(NestedMember {
                id,
                primary,
                table: table(primary),
                _child: child,
                _node: node,
                canonical,
            });
        }
        Ok(Self {
            graph,
            image,
            members,
        })
    }

    fn member(&self, id: &str) -> Result<&NestedMember<'a>, String> {
        self.members
            .iter()
            .find(|member| member.id == id)
            .ok_or_else(|| NESTED_SHAPE_CHANGED.into())
    }

    fn check(
        &self,
        splitter: *mut c_void,
        [splitter_id, merger_id, preceding_id, following_id]: [&str; 4],
        occupied_id: Option<&str>,
    ) -> Result<(), String> {
        if self.member(splitter_id)?.primary != splitter
            || table(self.graph) != self.image + GRAPH_TABLE
        {
            return Err("The Splitter observer is not on the retained primary".into());
        }
        let paired = self.member(merger_id)?.primary;
        let preceding = self.member(preceding_id)?.primary;
        let following = self.member(following_id)?.primary;
        let occupied = occupied_id
            .map(|id| self.member(id).map(|member| member.primary))
            .transpose()?;
        let [outer, inner, _, _, peer, outer_merger, _, _] = nested_topology(
            [splitter, paired, preceding, following],
            occupied,
            |object| {
                let member = self
                    .members
                    .iter()
                    .find(|member| member.primary == object)
                    .ok_or("An insert edge does not lead to a retained graph child")?;
                if table(object) != member.table
                    || unsafe { field::<*mut c_void>(object, 0x178) } != self.graph
                {
                    return Err(NESTED_SHAPE_CHANGED.into());
                }
                Ok(Links {
                    incoming: bounded_links(object, 0x188, 0x190, 0)?,
                    outgoing: bounded_links(object, 0x1a8, 0x1b0, 0)?,
                })
            },
        )?;
        if table(outer) != self.image + SPLITTER_TABLE
            || table(inner) != self.image + SPLITTER_TABLE
            || table(outer_merger) != self.image + MERGER_TABLE
            || !SLOT_TABLES
                .iter()
                .any(|offset| table(peer) == self.image + offset)
            || [
                (outer, outer_merger),
                (inner, following),
                (splitter, paired),
            ]
            .iter()
            .any(|&(node, merger)| unsafe {
                field::<*mut c_void>(node, 0x1e0) != merger
                    || field::<*mut c_void>(merger, 0x1d0) != node
            })
        {
            return Err(NESTED_SHAPE_CHANGED.into());
        }
        Ok(())
    }
}

enum Binding<'a> {
    Occupied(Vec<Owned>),
    Nested(NestedGraph<'a>),
}

#[derive(Clone, Copy)]
pub(in crate::native_script::request) enum Shape<'a> {
    OccupiedBranches(Option<&'a str>),
    EmptyNested,
    OneLaneNested(&'a str),
}

impl<'a> Shape<'a> {
    fn occupied_slot(self) -> Option<&'a str> {
        match self {
            Self::OneLaneNested(slot) => Some(slot),
            Self::OccupiedBranches(_) | Self::EmptyNested => None,
        }
    }
}

pub(in crate::native_script::request) fn dispatch(
    ctx: Box<Context<'static>>,
    ids @ [graph_id, splitter_id, merger_id, first_id, second_id]: [&str; 5],
    shape: Shape<'_>,
) -> Reply {
    let mut dispatched = false;
    let result = (|| -> Result<(), String> {
        let image = unsafe { _dyld_get_image_header(0) };
        if image_uuid(image) != Some(EXPECTED_APP_UUID) {
            return Err("Splitter removal requires the supported Studio Pro image".into());
        }
        let Inventory::Graph {
            route, ownership, ..
        } = capture(&ctx, Scope::Graph)?
        else {
            return Err("Graph ownership could not be established".into());
        };
        let image = image as usize;
        let nodes = [splitter_id, merger_id, first_id, second_id];
        let binding = match shape {
            Shape::OccupiedBranches(tail) => {
                ownership.check_splitter_remove_targets(ids, &ctx.instance.object_uid, tail)?;
                let mut members = nodes.to_vec();
                members.extend(tail);
                Binding::Occupied(bound_nodes(&ctx, &route, &members)?)
            }
            Shape::EmptyNested | Shape::OneLaneNested(_) => {
                let members = ownership.check_nested_splitter_remove_targets(
                    ids,
                    &ctx.instance.object_uid,
                    shape.occupied_slot(),
                )?;
                Binding::Nested(NestedGraph::capture(&ctx, &route, image, members)?)
            }
        };
        let identity = match &binding {
            Binding::Occupied(nodes) => &nodes[0],
            Binding::Nested(graph) => &graph.member(splitter_id)?.canonical,
        };
        let mut guard = || {
            ctx.check()?;
            route.check()
        };
        guard()?;
        let interpret =
            query(identity.0, &COMMAND_HANDLER).ok_or("Splitter command handler unavailable")?;
        let observer =
            query(identity.0, &OBSERVER).ok_or("Splitter message receiver unavailable")?;
        let check: unsafe extern "C" fn(*mut c_void, *const CheckCommand) -> u8 =
            unsafe { method(interpret.0, 4) };
        let notify: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
            unsafe { method(observer.0, 3) };
        if table(interpret.0) != image + 0x1b7b650
            || check as usize != image + 0xbbc5c8
            || table(observer.0) != image + 0x1b7b428
            || notify as usize != image + 0xbbc904
        {
            return Err("The exact Splitter command or observer entry changed".into());
        }
        // The matched notify thunk subtracts 0x20 from its receiver.
        let splitter = unsafe { observer.0.cast::<u8>().sub(0x20).cast() };
        if let Binding::Occupied(nodes) = &binding
            && (table(splitter) != image + SPLITTER_TABLE || !matches_identity(splitter, &nodes[0]))
        {
            return Err("The primary Splitter receiver changed".into());
        }
        let validate_shape = || match &binding {
            Binding::Occupied(nodes) => check_shape(image, &route, splitter, nodes),
            Binding::Nested(graph) => graph.check(splitter, nodes, shape.occupied_slot()),
        };
        guard()?;
        validate_shape()?;
        guard()?;
        let command = CheckCommand {
            category: cstring(ctx.api, c"Splitter")?,
            name: cstring(ctx.api, c"Remove")?,
            invoker: std::ptr::null_mut(),
            flags: 1,
            padding: 0,
        };
        let message = Rc::new(RemoveMessage {
            vtable: &REMOVE_MESSAGE,
            id: cstring(ctx.api, c"removeSelf")?,
        });
        let message = Owned(Rc::into_raw(message).cast_mut().cast());
        ownership.check(&mut guard, &mut || ctx.check())?;
        guard()?;
        if unsafe { check(interpret.0, &command) } != 1 {
            return Err("The bound Splitter did not recognize its Remove command".into());
        }
        guard()?;
        match &binding {
            Binding::Nested(_) => {
                ownership.check_borrowed(&mut guard)?;
                validate_shape()?;
            }
            Binding::Occupied(_) => {
                validate_shape()?;
                guard()?;
            }
        }
        // The normal menu schedules this fixed message. Deliver it to the same
        // pinned observer under the current document guard instead of queuing it.
        dispatched = true;
        unsafe { notify(observer.0, std::ptr::null_mut(), message.0) };
        ctx.check()?;
        route.check()?;
        Ok(())
    })();
    let (operation, rejection, first_node, second_node) = match shape {
        Shape::OccupiedBranches(_) => (
            "insert_splitter_remove",
            "insert_splitter_remove_rejected",
            "first_slot_node",
            "second_slot_node",
        ),
        Shape::EmptyNested => (
            "insert_splitter_remove_empty_nested",
            "insert_splitter_remove_empty_nested_rejected",
            "predecessor_slot_node",
            "successor_merger_node",
        ),
        Shape::OneLaneNested(_) => (
            "insert_splitter_remove_one_lane_nested",
            "insert_splitter_remove_one_lane_nested_rejected",
            "predecessor_slot_node",
            "successor_merger_node",
        ),
    };
    let mut reply = match result {
        Ok(()) => {
            let mut receipt = json!({
                "ok":true,"outcome":"applied_unverified","operation":operation,
                "verified":false,"instance":ctx.instance,"expected_instance":ctx.instance.object_uid,
                "expected_graph":graph_id,"splitter_node":splitter_id,"merger_node":merger_id
            });
            receipt[first_node] = json!(first_id);
            receipt[second_node] = json!(second_id);
            if let Some(slot) = shape.occupied_slot() {
                receipt["occupied_slot_node"] = json!(slot);
            }
            if let Shape::OccupiedBranches(Some(tail)) = shape {
                receipt["first_branch_tail_node"] = json!(tail);
            }
            Reply::unbound(receipt)
        }
        Err(reason) => Reply::error(
            rejection,
            &reason,
            if dispatched { "unknown" } else { "not_applied" },
        ),
    };
    reply.mutation = dispatched;
    reply.context = Some(ctx);
    reply
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_only_the_primary_of_two_matching_retained_interfaces() {
        let image = 0x100000000;
        for layout in [GRAPH_LAYOUT, SPLITTER_LAYOUT, MERGER_LAYOUT]
            .iter()
            .chain(&SLOT_LAYOUTS)
        {
            let mut object = [0usize; 7];
            object[0] = image + layout.primary;
            object[3] = image + layout.canonical;
            object[6] = image + layout.node;
            let primary = object.as_mut_ptr().cast::<c_void>();
            let node = object.as_mut_ptr().wrapping_add(6).cast();
            let canonical = object.as_mut_ptr().wrapping_add(3).cast();
            assert_eq!(layout.primary(image, node, canonical), Some(primary));
            let mut other_identity = image + layout.canonical;
            assert_eq!(
                layout.primary(image, node, std::ptr::addr_of_mut!(other_identity).cast()),
                None
            );
            assert_eq!(layout.primary(image, canonical, node), None);
            object[6] = image + layout.canonical;
            assert_eq!(layout.primary(image, node, canonical), None);
            object[6] = image + layout.node;
            object[3] = image + layout.node;
            assert_eq!(layout.primary(image, node, canonical), None);
            object[3] = image + layout.canonical;
            object[0] = image + layout.node;
            assert_eq!(layout.primary(image, node, canonical), None);
        }
    }

    fn fixture() -> [Links<u8>; 8] {
        [
            (&[][..], &[1, 5][..]),
            (&[0], &[2, 4]),
            (&[1], &[3]),
            (&[2], &[6, 6]),
            (&[1], &[7]),
            (&[0, 7], &[]),
            (&[3], &[7]),
            (&[6, 4], &[5]),
        ]
        .map(|(incoming, outgoing)| Links {
            incoming: incoming.to_vec(),
            outgoing: outgoing.to_vec(),
        })
    }

    fn check(anchors: [u8; 4], graph: &[Links<u8>; 8]) -> Result<[u8; 8], String> {
        check_lanes(anchors, None, graph)
    }

    fn check_lanes(
        anchors: [u8; 4],
        occupied: Option<u8>,
        graph: &[Links<u8>; 8],
    ) -> Result<[u8; 8], String> {
        nested_topology(anchors, occupied, |id| {
            let links = graph.get(usize::from(id)).ok_or("unknown graph node")?;
            Ok(Links {
                incoming: links.incoming.clone(),
                outgoing: links.outgoing.clone(),
            })
        })
    }

    #[test]
    fn accepts_exact_empty_lanes_with_one_retained_peer() {
        let mut graph = fixture();
        let anchors = [3, 6, 2, 7];
        assert_eq!(check(anchors, &graph).unwrap(), [0, 1, 2, 3, 4, 5, 6, 7]);
        graph[6].incoming.push(3);
        graph[7].incoming.reverse();
        assert_eq!(check(anchors, &graph).unwrap(), [0, 1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn accepts_either_exact_occupied_lane_with_reciprocal_links() {
        let anchors = [3, 6, 2, 7];
        let mut graph = fixture();
        graph[1].outgoing = vec![2, 7];
        graph[3].outgoing = vec![6, 4];
        graph[4] = Links {
            incoming: vec![3],
            outgoing: vec![6],
        };
        graph[6].incoming = vec![4, 3];
        graph[7].incoming = vec![6, 1];
        assert_eq!(
            check_lanes(anchors, Some(4), &graph).unwrap(),
            [0, 1, 2, 3, 4, 5, 6, 7]
        );
        assert!(check_lanes(anchors, Some(2), &graph).is_err());
        assert!(check(anchors, &graph).is_err());
        graph[3].outgoing.reverse();
        assert_eq!(
            check_lanes(anchors, Some(4), &graph).unwrap(),
            [0, 1, 2, 3, 4, 5, 6, 7]
        );
        graph[3].outgoing = vec![4, 4];
        assert!(check_lanes(anchors, Some(4), &graph).is_err());
        graph[3].outgoing = vec![4, 6];
        graph[4].outgoing = vec![7];
        assert!(check_lanes(anchors, Some(4), &graph).is_err());
    }

    #[test]
    fn refuses_wrong_anchors_and_unverified_layouts() {
        for anchors in [[1, 6, 2, 7], [3, 7, 2, 6], [3, 6, 4, 7], [3, 6, 2, 5]] {
            assert!(check(anchors, &fixture()).is_err());
        }

        let anchors = [3, 6, 2, 7];
        for (node, outgoing) in [
            (0, vec![5, 1]),
            (1, vec![4, 2]),
            (2, vec![3, 7]),
            (3, vec![4, 6]),
            (4, vec![6]),
            (5, vec![3]),
            (6, vec![5]),
            (7, vec![0]),
        ] {
            let mut graph = fixture();
            graph[node].outgoing = outgoing;
            assert!(check(anchors, &graph).is_err());
        }
        let mut graph = fixture();
        graph[6].incoming.push(4);
        assert!(check(anchors, &graph).is_err());
    }
}

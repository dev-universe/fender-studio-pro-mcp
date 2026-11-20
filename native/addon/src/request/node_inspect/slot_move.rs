//! Move one exact effect with the host's Routing editor operation.
use super::*;
use std::collections::{HashMap, HashSet};

#[path = "slot_move/splitter_add.rs"]
pub(in crate::native_script::request) mod splitter_add;

const VIEW: Uid = Uid {
    a: 0xf5b09f71,
    b: 0x6f3f,
    c: 0x4f03,
    d: [0xa5, 0x4a, 0xb2, 0x49, 0xb8, 0x29, 0x62, 0xd6],
};
const ITEM_VIEW: Uid = Uid {
    a: 0x2e0176f9,
    b: 0x47d7,
    c: 0x4046,
    d: [0x9d, 0xdb, 0xd5, 0x25, 0x6c, 0x57, 0x2f, 0xcf],
};
const ITEM_MODEL_TABLE: usize = 0x1beaa80;
const MODEL_TABLE: usize = 0x1bea798;
const INSERT_GRAPH_TABLE: usize = 0x1b7abb8;
const SLOT_TABLES: [usize; 2] = [0x1be18d8, 0x1bccdb8];
const PROJECT_DROP_OFFSET: usize = 0x113dc8c;
const PREVIEW_MOVE_OFFSET: usize = 0xbb4e60;
const COMMIT_MOVE_OFFSET: usize = 0xdd29c4;
const MAX_CELLS: usize = 1024;

#[repr(C)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
struct DropCell {
    row: i32,
    column: i32,
    kind: i32,
    padding: i32,
    node: *mut c_void,
    branch: i32,
    trailing_padding: i32,
}

#[repr(C)]
struct MoveDescriptor {
    source: *mut c_void,
    component: *mut c_void,
    reserved: *mut c_void,
    previous: *mut c_void,
    following: *mut c_void,
    branch: i32,
    padding: i32,
}
const _: () = {
    assert!(std::mem::size_of::<DropCell>() == 32);
    assert!(std::mem::offset_of!(DropCell, node) == 16);
    assert!(std::mem::size_of::<MoveDescriptor>() == 48);
    assert!(std::mem::offset_of!(MoveDescriptor, previous) == 24);
    assert!(std::mem::offset_of!(MoveDescriptor, following) == 32);
    assert!(std::mem::offset_of!(MoveDescriptor, branch) == 40);
};

unsafe fn field<T: Copy>(object: *mut c_void, offset: usize) -> T {
    unsafe { object.cast::<u8>().add(offset).cast::<T>().read_unaligned() }
}

fn table(object: *mut c_void) -> usize {
    unsafe { field::<usize>(object, 0) }
}

fn borrowed(object: *mut c_void, slot: usize) -> *mut c_void {
    let get: Borrowed = unsafe { method(object, slot) };
    unsafe { get(object) }
}

fn visible(view: *mut c_void) -> bool {
    let get: unsafe extern "C" fn(*mut c_void, *mut Rect) -> u8 = unsafe { method(view, 9) };
    let mut bounds = Rect {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    (unsafe { get(view, &mut bounds) }) != 0
        && bounds.left < bounds.right
        && bounds.top < bounds.bottom
}

struct Model {
    desktop: *mut c_void,
    window: Owned,
    index: i32,
    view: Owned,
    item: Owned,
    model: Owned,
    graph: *mut c_void,
    image: usize,
}

impl Model {
    fn base(&self) -> *mut c_void {
        // The IItemModel subobject was matched to the exact 8.1.1 vtable.
        unsafe { self.model.0.cast::<u8>().sub(0x30).cast() }
    }

    fn check(&self, ctx: &Context<'_>, route: &GraphRoute) -> Result<(), String> {
        ctx.check()?;
        route.check()?;
        let window_at: unsafe extern "C" fn(*mut c_void, i32) -> *mut c_void =
            unsafe { method(self.desktop, 7) };
        let is_visible: unsafe extern "C" fn(*mut c_void) -> u8 =
            unsafe { method(self.window.0, 20) };
        if unsafe { (ctx.api.desktop)() } != self.desktop
            || unsafe { window_at(self.desktop, self.index) } != self.window.0
            || unsafe { is_visible(self.window.0) } == 0
            || !visible(self.view.0)
            || borrowed(self.item.0, 5) != self.model.0
            || table(self.model.0) != self.image + ITEM_MODEL_TABLE
            || table(self.base()) != self.image + MODEL_TABLE
            || unsafe { field::<*mut c_void>(self.base(), 0x60) } != self.graph
        {
            return Err("active Routing editor or graph changed".into());
        }
        ctx.check()
    }

    fn capture(ctx: &Context<'_>, route: &GraphRoute, image: usize) -> Result<Self, String> {
        let desktop = unsafe { (ctx.api.desktop)() };
        if desktop.is_null() {
            return Err("Routing editor desktop unavailable".into());
        }
        let count: Count = unsafe { method(desktop, 6) };
        let at: unsafe extern "C" fn(*mut c_void, i32) -> *mut c_void =
            unsafe { method(desktop, 7) };
        let windows = unsafe { count(desktop) };
        if !(1..=16).contains(&windows) {
            return Err("Routing editor window count exceeds its bound".into());
        }
        let main = borrowed(desktop, 3);
        let mut found: Option<Model> = None;
        let mut inspected = 0;
        for index in 0..windows {
            ctx.check()?;
            let raw = unsafe { at(desktop, index) };
            if raw == main {
                continue;
            }
            let window = hold(raw).ok_or("Routing window detached")?;
            let is_visible: unsafe extern "C" fn(*mut c_void) -> u8 = unsafe { method(raw, 20) };
            if unsafe { is_visible(raw) } == 0 {
                continue;
            }
            let root = query(raw, &VIEW).ok_or("Routing window view unavailable")?;
            let mut stack = vec![(root, 0)];
            let mut seen = HashSet::new();
            while let Some((view, depth)) = stack.pop() {
                ctx.check()?;
                inspected += 1;
                if inspected > 1024 || depth > 24 || !seen.insert(view.0) {
                    return Err(
                        "Routing view search exceeded its bounds or has duplicate views".into(),
                    );
                }
                if let Some(item) = query(view.0, &ITEM_VIEW) {
                    ctx.check()?;
                    let model = borrowed(item.0, 5);
                    if !model.is_null() && table(model) == image + ITEM_MODEL_TABLE {
                        let model = hold(model).ok_or("Routing item model detached")?;
                        let base: *mut c_void = unsafe { model.0.cast::<u8>().sub(0x30).cast() };
                        if table(base) != image + MODEL_TABLE {
                            return Err("Routing model primary interface changed".into());
                        }
                        let graph = unsafe { field::<*mut c_void>(base, 0x60) };
                        let identity =
                            query(graph, &UNKNOWN).ok_or("Routing graph identity unavailable")?;
                        ctx.check()?;
                        if identity.0 == route.graph._canonical.0 {
                            if let Some(other) = &found {
                                if other.model.0 != model.0 || other.item.0 != item.0 {
                                    return Err(
                                        "More than one Routing model matches the selected graph"
                                            .into(),
                                    );
                                }
                            } else {
                                found = Some(Model {
                                    desktop,
                                    window: hold(window.0).ok_or("Routing window detached")?,
                                    index,
                                    view: hold(view.0).ok_or("Routing view detached")?,
                                    item,
                                    model,
                                    graph,
                                    image,
                                });
                            }
                        }
                    }
                }
                let Some(children) = hold(borrowed(view.0, 20)) else {
                    continue;
                };
                let iterator = borrowed(children.0, 11);
                if iterator.is_null() {
                    return Err("Routing view child iterator unavailable".into());
                }
                let iterator = Owned(iterator);
                let done: unsafe extern "C" fn(*mut c_void) -> u8 =
                    unsafe { method(iterator.0, 3) };
                let mut count = 0;
                while unsafe { done(iterator.0) } == 0 {
                    ctx.check()?;
                    count += 1;
                    if count > 1024 {
                        return Err("Routing view child count exceeds its bound".into());
                    }
                    let child = hold(borrowed(iterator.0, 4)).ok_or("Routing child detached")?;
                    let child =
                        query(child.0, &VIEW).ok_or("Routing child interface unavailable")?;
                    if visible(child.0) {
                        if borrowed(child.0, 17) != view.0 {
                            return Err("Routing child ownership changed".into());
                        }
                        stack.push((child, depth + 1));
                    }
                }
            }
        }
        let found =
            found.ok_or("Open the Routing tab for the exact Inserts graph before moving a slot")?;
        found.check(ctx, route)?;
        Ok(found)
    }

    fn cells(&self) -> Result<Vec<DropCell>, String> {
        let base = self.base();
        let rows = unsafe { field::<*const *mut c_void>(base, 0x80) };
        let count = unsafe { field::<i32>(base, 0x88) };
        if rows.is_null() || !(1..=128).contains(&count) {
            return Err("Routing model row count is unsupported".into());
        }
        let mut cells = Vec::new();
        let mut total_columns = 0;
        for row in 0..count {
            let raw = unsafe { *rows.add(row as usize) };
            if raw.is_null() {
                return Err("Routing row unavailable".into());
            }
            let entries = unsafe { field::<*mut c_void>(raw, 0x18) };
            let columns = unsafe { field::<i32>(raw, 0x20) };
            if !(0..=128).contains(&columns)
                || (columns > 0 && entries.is_null())
                || total_columns + columns as usize > MAX_CELLS
            {
                return Err("Routing model cell count is unsupported".into());
            }
            total_columns += columns as usize;
            for column in 0..columns {
                let offset = column as usize * 24;
                let kind = unsafe { field::<i32>(entries, offset) };
                let node = unsafe { field::<*mut c_void>(entries, offset + 8) };
                let branch = unsafe { field::<i32>(entries, offset + 16) };
                if kind != 0 && !node.is_null() {
                    cells.push(DropCell {
                        row,
                        column,
                        kind,
                        padding: 0,
                        node,
                        branch,
                        trailing_padding: 0,
                    });
                }
            }
        }
        Ok(cells)
    }
}

type BoundNodes = HashMap<*mut c_void, (String, Owned)>;

fn bind_nodes(
    cells: &[DropCell],
    route: &GraphRoute,
    guard: &mut impl FnMut() -> Result<(), String>,
) -> Result<BoundNodes, String> {
    let mut nodes = HashMap::new();
    for cell in cells {
        if let std::collections::hash_map::Entry::Vacant(entry) = nodes.entry(cell.node) {
            guard()?;
            let held = hold(cell.node).ok_or("Routing node detached")?;
            let node = query(held.0, &NODE).ok_or("Routing node identity unavailable")?;
            if borrowed(node.0, 6) != route.graph.node.0 {
                return Err("Routing node belongs to another graph".into());
            }
            let read: unsafe extern "C" fn(*mut c_void) -> *const Uid =
                unsafe { method(node.0, 4) };
            let id = uid(unsafe { read(node.0) });
            if !valid_uid(&id) {
                return Err("Routing node UUID unavailable".into());
            }
            entry.insert((id, held));
            guard()?;
        }
    }
    Ok(nodes)
}

fn resolve_node(nodes: &BoundNodes, id: &str) -> Result<*mut c_void, String> {
    let mut found = nodes.iter().filter(|(_, (uid, _))| uid == id);
    let (&pointer, _) = found
        .next()
        .ok_or("Requested insert graph node is absent from Routing")?;
    if found.next().is_some() {
        return Err("Insert graph node UUID is ambiguous".into());
    }
    Ok(pointer)
}

fn project_branch(
    model: &Model,
    cells: &[DropCell],
    after: *mut c_void,
    following: *mut c_void,
    guard: &mut impl FnMut() -> Result<(), String>,
) -> Result<i32, String> {
    let project: unsafe extern "C" fn(*mut c_void, *const DropCell) -> *mut c_void =
        unsafe { std::mem::transmute(model.image + PROJECT_DROP_OFFSET) };
    let mut branch = None;
    for cell in cells
        .iter()
        .filter(|cell| cell.node == following && cell.kind != 1)
    {
        if !(0..32).contains(&cell.branch) {
            continue;
        }
        guard()?;
        if unsafe { project(model.base(), cell) } == after {
            if branch.is_some_and(|value| value != cell.branch) {
                return Err("Destination edge has multiple native branch positions".into());
            }
            branch = Some(cell.branch);
        }
    }
    branch.ok_or("Routing cannot project that destination edge".into())
}

pub(in crate::native_script::request) fn dispatch(
    ctx: Box<Context<'static>>,
    [graph_id, source_id, after_id, following_id]: [&str; 4],
) -> Reply {
    let mut dispatched = false;
    let result = (|| -> Result<(), String> {
        let image = unsafe { _dyld_get_image_header(0) };
        if image_uuid(image) != Some(EXPECTED_APP_UUID) {
            return Err("Insert slot move requires the supported Studio Pro image".into());
        }
        let Inventory::Graph {
            route, ownership, ..
        } = capture(&ctx, Scope::Graph)?
        else {
            return Err("Insert slot ownership inventory unavailable".into());
        };
        ownership.check_move_targets(
            graph_id,
            source_id,
            after_id,
            following_id,
            &ctx.instance.object_uid,
        )?;
        let model = Model::capture(&ctx, &route, image as usize)?;
        let image = image as usize;
        let graph = model.graph;
        if table(graph) != image + INSERT_GRAPH_TABLE {
            return Err(
                "Current Routing graph family does not have a verified move operation".into(),
            );
        }
        let mut guard = || model.check(&ctx, &route);
        guard()?;
        let cells = model.cells()?;
        let nodes = bind_nodes(&cells, &route, &mut guard)?;
        let source = resolve_node(&nodes, source_id)?;
        let after = resolve_node(&nodes, after_id)?;
        let following = resolve_node(&nodes, following_id)?;
        for slot in [source, after] {
            if !SLOT_TABLES
                .iter()
                .any(|offset| table(slot) == image + offset)
                || unsafe { field::<*mut c_void>(slot, 0x178) } != graph
                || !cells.iter().any(|cell| cell.kind == 1 && cell.node == slot)
            {
                return Err(
                    "Source and destination must be exact effect slots of this graph".into(),
                );
            }
        }
        let successors = unsafe { field::<*const *mut c_void>(after, 0x1a8) };
        if unsafe { field::<i32>(after, 0x1b0) } != 1
            || successors.is_null()
            || unsafe { *successors } != following
        {
            return Err("Requested destination edge has changed".into());
        }
        let branch = project_branch(&model, &cells, after, following, &mut guard)?;
        let input = MoveDescriptor {
            source,
            component: source, // Verified SlotNode variants use no replacement component.
            reserved: std::ptr::null_mut(),
            previous: after,
            following,
            branch,
            padding: 0,
        };
        let preview: unsafe extern "C" fn(*mut c_void, *const MoveDescriptor) -> u8 =
            unsafe { method(graph, 0x200 / 8) };
        let commit: unsafe extern "C" fn(*mut c_void, *const MoveDescriptor) -> u64 =
            unsafe { method(model.base(), 0x158 / 8) };
        if preview as usize != image + PREVIEW_MOVE_OFFSET
            || commit as usize != image + COMMIT_MOVE_OFFSET
            || (unsafe { method::<Borrowed>(source, 0x200 / 8) }) as usize != image + 0xbbfa90
        {
            return Err("Native Routing move entry no longer matches the verified image".into());
        }
        ownership.check(&mut guard, &mut || ctx.check())?;
        guard()?;
        if unsafe { preview(graph, &input) } != 1 {
            return Err("Studio Pro rejected the exact insert routing move".into());
        }
        guard()?;
        if model.cells()? != cells
            || unsafe { field::<i32>(after, 0x1b0) } != 1
            || unsafe { field::<*const *mut c_void>(after, 0x1a8) } != successors
            || unsafe { *successors } != following
        {
            return Err("Routing destination changed after the host preview".into());
        }
        dispatched = true;
        if unsafe { commit(model.base(), &input) } & 1 == 0 {
            return Err("The native move returned without an acceptance receipt".into());
        }
        ctx.check()?;
        route.check()?;
        Ok(())
    })();
    let mut reply = match result {
        Ok(()) => Reply::unbound(json!({
            "ok":true,"outcome":"applied_unverified","operation":"insert_slot_move","verified":false,
            "instance":ctx.instance,
            "expected_instance":ctx.instance.object_uid,"expected_graph":graph_id,"source_node":source_id,
            "after_node":after_id,"following_node":following_id
        })),
        Err(reason) => Reply::error(
            "insert_slot_move_rejected",
            &reason,
            if dispatched { "unknown" } else { "not_applied" },
        ),
    };
    reply.context = Some(ctx);
    reply.mutation = dispatched;
    reply
}

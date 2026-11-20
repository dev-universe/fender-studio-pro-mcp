//! Synchronous delivery of Studio Pro's fixed Splitter remove message.
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
    let count = unsafe { field::<i32>(object, count) };
    let members = unsafe { field::<*const *mut c_void>(object, pointer) };
    if !(1..=2).contains(&count) || members.is_null() {
        return Err("The expected narrow nested routing edge changed".into());
    }
    let result = unsafe { std::slice::from_raw_parts(members, count as usize) }.to_vec();
    if result.iter().any(|node| node.is_null()) {
        return Err("A required routing edge has no bound node".into());
    }
    Ok(result)
}

fn bound_nodes(
    ctx: &Context<'_>,
    route: &GraphRoute,
    ids: [&str; 4],
) -> Result<[Owned; 4], String> {
    let graph = route.graph.node.0;
    let count: Count = unsafe { method(graph, 8) };
    let child: unsafe extern "C" fn(*mut c_void, i32) -> *mut c_void = unsafe { method(graph, 9) };
    let count = unsafe { count(graph) };
    if !(4..=32).contains(&count) {
        return Err("Graph child count is outside the verified range".into());
    }
    let mut targets = std::array::from_fn::<_, 4, _>(|_| None);
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
    let [Some(splitter), Some(merger), Some(first), Some(second)] = targets else {
        return Err("The required nested Splitter nodes are unavailable".into());
    };
    Ok([splitter, merger, first, second])
}

fn check_shape(image: usize, graph: *mut c_void, nodes: &[Owned; 4]) -> Result<(), String> {
    let [splitter, merger, first, second] = nodes.each_ref().map(|node| node.0);
    if table(splitter) != image + SPLITTER_TABLE
        || [first, second].iter().any(|node| {
            !SLOT_TABLES
                .iter()
                .any(|offset| table(*node) == image + offset)
        })
        || class_name(merger) != "AudioFXGraph::MergerNode"
        || [splitter, merger, first, second]
            .iter()
            .any(|node| unsafe { field::<*mut c_void>(*node, 0x178) } != graph)
        || adjacent(splitter, 0x1a8, 0x1b0)? != [first, second]
        || adjacent(first, 0x188, 0x190)? != [splitter]
        || adjacent(second, 0x188, 0x190)? != [splitter]
        || adjacent(first, 0x1a8, 0x1b0)? != [merger]
        || adjacent(second, 0x1a8, 0x1b0)? != [merger]
    {
        return Err("The exact two-branch Splitter and merger shape changed".into());
    }
    let predecessor = adjacent(splitter, 0x188, 0x190)?;
    let successor = adjacent(merger, 0x1a8, 0x1b0)?;
    let merge_inputs = adjacent(merger, 0x188, 0x190)?;
    if !merge_inputs.contains(&first) || !merge_inputs.contains(&second) {
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

pub(in crate::native_script::request) fn dispatch(
    ctx: Box<Context<'static>>,
    ids @ [graph_id, splitter_id, merger_id, first_id, second_id]: [&str; 5],
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
        ownership.check_splitter_remove_targets(ids, &ctx.instance.object_uid)?;
        let graph = route.graph._canonical.0;
        let image = image as usize;
        if table(graph) != image + GRAPH_TABLE {
            return Err(
                "The current graph has no verified nested Splitter removal receiver".into(),
            );
        }
        let nodes = bound_nodes(&ctx, &route, [splitter_id, merger_id, first_id, second_id])?;
        let mut guard = || {
            ctx.check()?;
            route.check()
        };
        guard()?;
        check_shape(image, graph, &nodes)?;
        guard()?;
        let splitter = nodes[0].0;
        let interpret =
            query(splitter, &COMMAND_HANDLER).ok_or("Splitter command handler unavailable")?;
        let observer = query(splitter, &OBSERVER).ok_or("Splitter message receiver unavailable")?;
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
        check_shape(image, graph, &nodes)?;
        guard()?;
        // The normal menu schedules this fixed message. Deliver it to the same
        // pinned observer under the current document guard instead of queuing it.
        dispatched = true;
        unsafe { notify(observer.0, std::ptr::null_mut(), message.0) };
        ctx.check()?;
        route.check()?;
        Ok(())
    })();
    let mut reply = match result {
        Ok(()) => Reply::unbound(json!({
            "ok":true,"outcome":"applied_unverified","operation":"insert_splitter_remove",
            "verified":false,"instance":ctx.instance,"expected_instance":ctx.instance.object_uid,
            "expected_graph":graph_id,"splitter_node":splitter_id,"merger_node":merger_id,
            "first_slot_node":first_id,"second_slot_node":second_id
        })),
        Err(reason) => Reply::error(
            "insert_splitter_remove_rejected",
            &reason,
            if dispatched { "unknown" } else { "not_applied" },
        ),
    };
    reply.mutation = dispatched;
    reply.context = Some(ctx);
    reply
}

//! Remove one exact middle effect through Studio Pro's recorded action.
use super::super::slot_move::splitter_add::{
    RecordingScope, ScopeStorage, recorded_submit, undo_available,
};
use super::*;
use std::mem::MaybeUninit;

// The live routed adapter derives from AudioInsertSlot and inherits Device commands.
const ROUTED_AUDIO_INSERT_SLOT: usize = 0x1b90b50;
const SLOT_HANDLER: usize = 0x1b911f0;
const SLOT_EDIT_SCOPE: usize = 0x1bd9250;
const ACTION_JOURNAL: Uid = Uid {
    a: 0xd70f5dc5,
    b: 0xd9bd,
    c: 0x4230,
    d: [0xb9, 0x08, 0xf5, 0x57, 0xcc, 0x41, 0x79, 0x3f],
};

fn parent(node: *mut c_void) -> *mut c_void {
    let get: Borrowed = unsafe { method(node, 6) };
    unsafe { get(node) }
}

fn object_id(node: *mut c_void) -> String {
    let get: unsafe extern "C" fn(*mut c_void) -> *const Uid = unsafe { method(node, 4) };
    uid(unsafe { get(node) })
}

pub(in crate::native_script::request) fn dispatch(
    ctx: Box<Context<'static>>,
    graph_id: &str,
    ids @ [preceding_id, removed_id, following_id]: [&str; 3],
    removed_instance: &str,
) -> Reply {
    let mut dispatched = false;
    let result = (|| -> Result<(), String> {
        let image = unsafe { _dyld_get_image_header(0) };
        if image_uuid(image) != Some(EXPECTED_APP_UUID) {
            return Err("Middle slot removal requires the supported Studio Pro image".into());
        }
        let image = image as usize;
        let Inventory::Graph {
            route, ownership, ..
        } = capture(&ctx, Scope::Graph)?
        else {
            return Err("The exact insert graph ownership is unavailable".into());
        };
        ownership.check_slot_remove_targets(
            graph_id,
            ids,
            &ctx.instance.object_uid,
            removed_instance,
        )?;
        let mut guard = || {
            ctx.check()?;
            route.check()
        };
        let graph_node = route.graph.node.0;
        let graph = GRAPH_LAYOUT
            .primary(image, graph_node, route.graph._canonical.0)
            .ok_or("The primary insert graph changed")?;
        let identities = bound_nodes(&ctx, &route, &ids)?;
        let nodes = identities
            .iter()
            .map(|id| query(id.0, &NODE).ok_or("A bound slot node lost its INode view"))
            .collect::<Result<Vec<_>, _>>()?;
        let primaries = nodes
            .iter()
            .zip(&identities)
            .map(|(n, id)| {
                SLOT_LAYOUTS
                    .iter()
                    .find_map(|layout| layout.primary(image, n.0, id.0))
                    .ok_or("An exact effect slot changed its graph layout")
            })
            .collect::<Result<Vec<_>, _>>()?;
        let [preceding, removed, following] = primaries[..] else {
            return Err("Three adjacent slot nodes are required".into());
        };
        let check_edges = || -> Result<(), String> {
            if primaries
                .iter()
                .any(|&node| unsafe { field::<*mut c_void>(node, 0x178) } != graph)
                || nodes.iter().any(|node| parent(node.0) != graph_node)
                || adjacent(preceding, 0x1a8, 0x1b0)? != [removed]
                || adjacent(removed, 0x188, 0x190)? != [preceding]
                || adjacent(removed, 0x1a8, 0x1b0)? != [following]
                || adjacent(following, 0x188, 0x190)? != [removed]
            {
                return Err(
                    "The exact middle effect is no longer reciprocally linked to both neighbors"
                        .into(),
                );
            }
            Ok(())
        };
        guard()?;
        check_edges()?;

        let names = constant_string().ok_or("Native String constants are unavailable")?;
        let slot_name = unsafe { names(c"Slot".as_ptr()) };
        let device_name = unsafe { names(c"Device".as_ptr()) };
        if slot_name.is_null() || device_name.is_null() {
            return Err("The stock slot or device delegate name is unavailable".into());
        }
        guard()?;
        let removed_node = NodePin::borrowed(nodes[1].0)?;
        let delegated_slot = removed_node.child(slot_name)?;
        let slot_node = query(delegated_slot.node.0, &NODE)
            .ok_or("The delegated slot has no native INode view")?;
        let delegated_device = NodePin::borrowed(slot_node.0)?.child(device_name)?;
        let removed_device = query(delegated_device.node.0, &NODE)
            .ok_or("The removed effect has no native INode view")?;
        let native_slot = unsafe { field::<*mut c_void>(removed, 0x1f0) };
        let slot_owner = hold(native_slot).ok_or("The resolved native slot detached")?;
        if table(slot_owner.0) != image + ROUTED_AUDIO_INSERT_SLOT {
            return Err("The verified routed AudioInsertSlot primary table changed".into());
        }
        if !matches_identity(slot_owner.0, &delegated_slot._canonical) {
            return Err("The AudioInsertSlot is not the slot node's canonical delegate".into());
        }
        if object_id(removed_device.0) != removed_instance {
            return Err("The exact delegated device UUID changed".into());
        }
        let owning_folder = unsafe { field::<*mut c_void>(slot_owner.0, 0x58) };
        if owning_folder.is_null() || !matches_identity(owning_folder, &route.folder._canonical) {
            return Err("The native removal receiver must have the bound Inserts folder".into());
        }
        let handler = query(slot_owner.0, &COMMAND_HANDLER)
            .ok_or("The slot command handler is unavailable")?;
        let check: unsafe extern "C" fn(*mut c_void, *const CheckCommand) -> u8 =
            unsafe { method(handler.0, 4) };
        if handler.0 != slot_owner.0.cast::<u8>().wrapping_add(0x90).cast()
            || table(handler.0) != image + SLOT_HANDLER
            || check as usize != image + 0xbafc20
        {
            return Err(
                "The exact slot command handler no longer matches the verified image".into(),
            );
        }
        let check_shape = || -> Result<(), String> {
            check_edges()?;
            if parent(removed_device.0) != slot_node.0
                || parent(slot_node.0) != route.folder.node.0
                || unsafe { field::<*mut c_void>(removed, 0x1f0) } != slot_owner.0
                || unsafe { field::<*mut c_void>(slot_owner.0, 0x58) } != owning_folder
                || object_id(removed_device.0) != removed_instance
            {
                return Err("The exact removed device or its Inserts ownership changed".into());
            }
            Ok(())
        };
        let command = CheckCommand {
            category: cstring(ctx.api, c"Device")?,
            name: cstring(ctx.api, c"Remove")?,
            invoker: std::ptr::null_mut(),
            flags: 1,
            padding: 0,
        };
        ownership.check(&mut guard, &mut || ctx.check())?;
        check_shape()?;
        if unsafe { check(handler.0, &command) } != 1 {
            return Err("The bound slot did not approve Device / Remove".into());
        }
        let native_folder: Borrowed = unsafe { std::mem::transmute(image + 0xd737f4) };
        if unsafe { native_folder(slot_owner.0) } != owning_folder {
            return Err("The recorded action resolved a different owning folder".into());
        }
        let check_recorded_branch = || -> Result<(), String> {
            let folder_flags = unsafe { field::<u8>(owning_folder, 0x2ec) };
            let slot_flags = unsafe { field::<u8>(slot_owner.0, 0x1d0) };
            if folder_flags & 0x60 != 0x20 || slot_flags & 8 != 0 {
                return Err("The slot does not support recorded removal".into());
            }
            Ok(())
        };
        check_recorded_branch()?;

        // The stock observer constructs this scope and loops over selection.
        // Use its explicit per-slot helper with the already bound target.
        let begin_scope: unsafe extern "C" fn(*mut c_void, *mut c_void) =
            unsafe { std::mem::transmute(image + 0xd6c290) };
        let begin_group: unsafe extern "C" fn(*mut c_void, *const PlainString, u8) -> *mut c_void =
            unsafe { std::mem::transmute(image + 0xd6c924) };
        let remove: unsafe extern "C" fn(*mut c_void, *mut c_void) -> u8 =
            unsafe { std::mem::transmute(image + 0xd6d27c) };
        let finish: unsafe extern "C" fn(*mut c_void) =
            unsafe { std::mem::transmute(image + 0xd6c4a4) };
        let empty = unsafe { names(c"".as_ptr()) };
        if empty.is_null() {
            return Err("The empty host String is unavailable".into());
        }
        ownership.check(&mut guard, &mut || ctx.check())?;
        check_shape()?;
        undo_available(image, graph)?;
        let mut storage = MaybeUninit::<ScopeStorage>::uninit();
        let scope_ptr = storage.as_mut_ptr().cast();
        unsafe { begin_scope(scope_ptr, slot_owner.0) };
        let scope = RecordingScope::new(scope_ptr, finish);
        recorded_submit(image, &scope, SLOT_EDIT_SCOPE)?;
        let get_journal = || {
            let owner = unsafe { field::<*mut c_void>(scope_ptr, 0x40) };
            if owner.is_null() {
                return std::ptr::null_mut();
            }
            let get: Borrowed = unsafe { method(owner, 3) };
            unsafe { get(owner) }
        };
        let raw_journal = get_journal();
        let journal =
            query(raw_journal, &ACTION_JOURNAL).ok_or("The recorded journal is unavailable")?;
        let last_edit: unsafe extern "C" fn(*mut c_void) -> i64 = unsafe { method(journal.0, 3) };
        let is_performing: unsafe extern "C" fn(*mut c_void) -> u8 =
            unsafe { method(journal.0, 8) };
        if unsafe { is_performing(journal.0) } != 0 {
            return Err("The host Undo journal is already performing an action".into());
        }
        let before_edit = unsafe { last_edit(journal.0) };
        guard()?;
        check_shape()?;
        let group = unsafe { begin_group(scope_ptr, empty, 0) };
        if group.is_null()
            || group != unsafe { field::<*mut c_void>(image as *mut c_void, 0x1e834b0) }
        {
            return Err("The native removal action could not open its recorded group".into());
        }
        recorded_submit(image, &scope, SLOT_EDIT_SCOPE)?;
        ownership.check(&mut guard, &mut || ctx.check())?;
        check_shape()?;
        check_recorded_branch()?;
        if get_journal() != raw_journal {
            return Err("The host Undo journal changed before removal".into());
        }
        dispatched = true;
        let accepted = unsafe { remove(scope_ptr, slot_owner.0) } & 1 == 1;
        drop(scope);
        guard()?;
        if !accepted || unsafe { last_edit(journal.0) } == before_edit {
            return Err("The host did not confirm a recorded removal action".into());
        }
        if adjacent(preceding, 0x1a8, 0x1b0)? != [following]
            || adjacent(following, 0x188, 0x190)? != [preceding]
        {
            return Err("The stock removal did not publish the expected neighbor link".into());
        }
        Ok(())
    })();
    let mut reply = match result {
        Ok(()) => Reply::unbound(json!({
            "ok": true, "outcome": "applied_unverified", "operation": "insert_slot_remove",
            "verified": false, "instance":ctx.instance, "expected_instance":ctx.instance.object_uid,
            "expected_graph":graph_id, "preceding_slot_node":preceding_id, "removed_slot_node":removed_id,
            "following_slot_node":following_id, "expected_removed_instance":removed_instance
        })),
        Err(reason) => Reply::error(
            "insert_slot_remove_rejected",
            &reason,
            if dispatched { "unknown" } else { "not_applied" },
        ),
    };
    reply.mutation = dispatched;
    reply.context = Some(ctx);
    reply
}

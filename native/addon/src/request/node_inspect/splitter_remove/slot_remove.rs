//! Remove one exact middle effect through the owning slot's stock observer.
use super::*;

// The live routed adapter derives from AudioInsertSlot. Its primary and
// observer tables differ from the base while its Device command is inherited.
const ROUTED_AUDIO_INSERT_SLOT: usize = 0x1b90b50;
const SLOT_HANDLER: usize = 0x1b911f0;
const SLOT_OBSERVER: usize = 0x1b90fc8;

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
        let observer =
            query(slot_owner.0, &OBSERVER).ok_or("The slot message receiver is unavailable")?;
        let check: unsafe extern "C" fn(*mut c_void, *const CheckCommand) -> u8 =
            unsafe { method(handler.0, 4) };
        let notify: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
            unsafe { method(observer.0, 3) };
        if handler.0 != slot_owner.0.cast::<u8>().wrapping_add(0x90).cast()
            || observer.0 != slot_owner.0.cast::<u8>().wrapping_add(0x20).cast()
            || table(handler.0) != image + SLOT_HANDLER
            || check as usize != image + 0xbafc20
            || table(observer.0) != image + SLOT_OBSERVER
            || notify as usize != image + 0xc2c280
            || (unsafe { method::<Borrowed>(slot_owner.0, 0x230 / 8) }) as usize != image + 0xd7e50c
        {
            return Err("The exact slot removal entry no longer matches the verified image".into());
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
        let message = Rc::new(RemoveMessage {
            vtable: &REMOVE_MESSAGE,
            id: cstring(ctx.api, c"removeSelf")?,
        });
        let message = Owned(Rc::into_raw(message).cast_mut().cast());
        ownership.check(&mut guard, &mut || ctx.check())?;
        check_shape()?;
        if unsafe { check(handler.0, &command) } != 1 {
            return Err("The bound slot did not approve Device / Remove".into());
        }
        ownership.check(&mut guard, &mut || ctx.check())?;
        check_shape()?;
        // Device / Remove can act on selection. Deliver removeSelf only to
        // the exact retained slot observer after the stock check-only call.
        dispatched = true;
        unsafe { notify(observer.0, std::ptr::null_mut(), message.0) };
        guard()?;
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

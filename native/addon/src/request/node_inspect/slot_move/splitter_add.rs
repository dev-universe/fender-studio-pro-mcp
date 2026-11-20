//! Insert one default two-branch Splitter through the host's recorded edit.
use super::*;
use std::mem::MaybeUninit;

const SCOPE_TABLE: usize = 0x1b7c888;
const ADD_ACTION_TABLE: usize = 0x1b7cc20;

#[repr(C, align(16))]
struct ScopeStorage([u8; 0x50]);

struct RecordingScope {
    object: *mut c_void,
    finish: unsafe extern "C" fn(*mut c_void),
}

impl Drop for RecordingScope {
    fn drop(&mut self) {
        unsafe { (self.finish)(self.object) };
    }
}

fn undo_available(image: usize, graph: *mut c_void) -> Result<(), String> {
    let undo_owner: Borrowed = unsafe { method(graph, 0x148 / 8) };
    let graph_root: Borrowed = unsafe { method(graph, 0x150 / 8) };
    if undo_owner as usize != image + 0x15a0e38 || graph_root as usize != image + 0x15a0e60 {
        return Err("The graph Undo getters changed".into());
    }
    let globals = image as *mut c_void;
    // The matching host predicate initializes the registry on first use.
    // An uninitialized or occupied scope is an ordinary refusal.
    let available = unsafe {
        field::<u8>(globals, 0x1e83520) & 1 == 1
            && field::<u8>(globals, 0x1e83521) == 1
            && field::<u8>(globals, 0x1e834c4) == 0
            && field::<usize>(globals, 0x1e834a8) == 0
            && field::<usize>(globals, 0x1e834b0) == 0
            && field::<i32>(globals, 0x1e834c0) == 0
    };
    let owner = unsafe { undo_owner(graph) };
    if !available || owner.is_null() || owner == unsafe { graph_root(graph) } {
        return Err("A recorded Undo for this graph is unavailable".into());
    }
    Ok(())
}

fn check_wire(model: &Model, previous: *mut c_void, following: *mut c_void) -> Result<(), String> {
    if !SLOT_TABLES
        .iter()
        .any(|offset| table(previous) == model.image + offset)
        || class_name(following) != "AudioFXGraph::MergerNode"
        || unsafe { field::<*mut c_void>(previous, 0x178) } != model.graph
        || unsafe { field::<*mut c_void>(following, 0x178) } != model.graph
    {
        return Err("Splitter insertion requires a live effect slot and its merger".into());
    }
    let successors = unsafe { field::<*const *mut c_void>(previous, 0x1a8) };
    if successors.is_null()
        || unsafe { field::<i32>(previous, 0x1b0) } != 1
        || unsafe { *successors } != following
    {
        return Err("The requested effect no longer leads directly to the merger".into());
    }
    Ok(())
}

type SubmitAction = unsafe extern "C" fn(*mut c_void, *mut c_void) -> u8;

fn recorded_submit(image: usize, scope: &RecordingScope) -> Result<SubmitAction, String> {
    let scope_ptr = scope.object;
    let submit: SubmitAction = unsafe { method(scope_ptr, 0xe8 / 8) };
    let globals = image as *mut c_void;
    if table(scope_ptr) != image + SCOPE_TABLE
        || submit as usize != image + 0x1316d9c
        || unsafe { field::<*mut c_void>(globals, 0x1e834a8) } != scope_ptr
        || unsafe { field::<i32>(globals, 0x1e834c0) } != 1
    {
        return Err("The recorded host edit scope did not bind".into());
    }
    let manager = unsafe { field::<*mut c_void>(scope_ptr, 0x40) };
    if manager.is_null() || borrowed(manager, 3).is_null() {
        return Err("The native scope has no Undo journal".into());
    }
    Ok(submit)
}

pub(in crate::native_script::request) fn dispatch(
    ctx: Box<Context<'static>>,
    [graph_id, after_id, following_id]: [&str; 3],
) -> Reply {
    let mut dispatched = false;
    let result = (|| -> Result<(), String> {
        let header = unsafe { _dyld_get_image_header(0) };
        if image_uuid(header) != Some(EXPECTED_APP_UUID) {
            return Err("Splitter insertion requires the supported Studio Pro image".into());
        }
        let image = header as usize;
        let Inventory::Graph {
            route, ownership, ..
        } = capture(&ctx, Scope::Graph)?
        else {
            return Err("Insert graph ownership unavailable".into());
        };
        ownership.check_splitter_add_targets(
            graph_id,
            after_id,
            following_id,
            &ctx.instance.object_uid,
        )?;
        let model = Model::capture(&ctx, &route, image)?;
        let graph = model.graph;
        if table(graph) != image + INSERT_GRAPH_TABLE {
            return Err("The Routing graph family has no verified Splitter creation".into());
        }
        let mut guard = || model.check(&ctx, &route);
        guard()?;
        let cells = model.cells()?;
        let nodes = bind_nodes(&cells, &route, &mut guard)?;
        let after = resolve_node(&nodes, after_id)?;
        let following = resolve_node(&nodes, following_id)?;
        check_wire(&model, after, following)?;
        let branch = project_branch(&model, &cells, after, following, None, &mut guard)?;
        if branch != 0 {
            return Err("The supported insertion is on the effect slot's single outlet".into());
        }
        guard()?;
        ownership.check(&mut guard, &mut || ctx.check())?;
        undo_available(image, graph)?;

        // These are the calls in the host's recorded Insert Splitter branch.
        // The alternate direct insertion branch has no Undo and is excluded.
        let allocate: unsafe extern "C" fn(usize) -> *mut c_void =
            unsafe { std::mem::transmute(image + 0x15f3744) };
        let new_splitter: unsafe extern "C" fn(*mut c_void) =
            unsafe { std::mem::transmute(image + 0xbb9fb8) };
        let edit: unsafe extern "C" fn(*mut c_void, *mut c_void) =
            unsafe { std::mem::transmute(image + 0xbc163c) };
        let finish: unsafe extern "C" fn(*mut c_void) =
            unsafe { std::mem::transmute(image + 0xd6c4a4) };
        let title: unsafe extern "C" fn(*const c_void, u8) -> *const c_void =
            unsafe { std::mem::transmute(image + 0x15f329c) };
        let action: unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            *mut c_void,
            *mut c_void,
            *mut c_void,
            i32,
            *const c_void,
        ) = unsafe { std::mem::transmute(image + 0xbc2334) };
        let title = unsafe { title((image + 0x1f0bd98) as *const c_void, 0) };
        if title.is_null() {
            return Err("The Studio Pro Insert Splitter title is unavailable".into());
        }
        guard()?;
        undo_available(image, graph)?;

        let mut storage = MaybeUninit::<ScopeStorage>::uninit();
        let scope_ptr = storage.as_mut_ptr().cast();
        unsafe { edit(scope_ptr, graph) };
        let scope = RecordingScope {
            object: scope_ptr,
            finish,
        };
        recorded_submit(image, &scope)?;
        guard()?;
        check_wire(&model, after, following)?;
        let allocation = unsafe { allocate(0x1f0) };
        if allocation.is_null() {
            return Err("Studio Pro could not allocate a Splitter".into());
        }
        unsafe { new_splitter(allocation) };
        let splitter = Owned(allocation);
        if table(splitter.0) != image + SPLITTER_TABLE
            || !unsafe { field::<*mut c_void>(splitter.0, 0x178) }.is_null()
        {
            return Err("The new Splitter is not the verified unattached object".into());
        }
        guard()?;
        check_wire(&model, after, following)?;
        let submit = recorded_submit(image, &scope)?;
        let allocation = unsafe { allocate(0x1e0) };
        if allocation.is_null() {
            return Err("Studio Pro could not allocate a Splitter action".into());
        }
        unsafe {
            action(
                allocation, graph, splitter.0, after, following, branch, title,
            )
        };
        let action = Owned(allocation);
        // The host action owns the newly constructed Splitter from this point.
        std::mem::forget(splitter);
        if table(action.0) != image + ADD_ACTION_TABLE {
            return Err("The native creation action changed".into());
        }
        let pointer = action.0;
        std::mem::forget(action);
        dispatched = true;
        let accepted = unsafe { submit(scope.object, pointer) } & 1 == 1;
        drop(scope);
        if !accepted {
            return Err("The native insertion did not acknowledge the action".into());
        }
        ctx.check()?;
        route.check()?;
        Ok(())
    })();
    let mut reply = match result {
        Ok(()) => Reply::unbound(json!({
            "ok":true,"outcome":"applied_unverified","operation":"insert_splitter_add",
            "verified":false,"instance":ctx.instance,"expected_instance":ctx.instance.object_uid,
            "expected_graph":graph_id,"after_node":after_id,"following_node":following_id
        })),
        Err(reason) => Reply::error(
            "insert_splitter_add_rejected",
            &reason,
            if dispatched { "unknown" } else { "not_applied" },
        ),
    };
    reply.mutation = dispatched;
    reply.context = Some(ctx);
    reply
}

//! Bound insertion of a second stock Mai Tai.
use super::*;
use std::rc::Rc;

const MESSAGE: Uid = Uid {
    a: 0xd943e242,
    b: 0xf8c7,
    c: 0x4ed8,
    d: [0x80, 0x5c, 0x4c, 0x47, 0x85, 0x46, 0xb1, 0x14],
};
const GRAPH_INDEX: i32 = 2;
const VARIANT_STRING: i16 = 3;
const VARIANT_OBJECT: i16 = 4;
const VARIANT_TYPE_MASK: i16 = 15;
const VARIANT_SHARED: i16 = 16;
const _: () = {
    assert!(std::mem::size_of::<VariantOut>() == 16);
    assert!(std::mem::offset_of!(VariantOut, kind) == 0);
    assert!(std::mem::offset_of!(VariantOut, payload) == 8);
};

#[repr(C)]
struct MessageVtable {
    base: UnknownVtable,
    id: unsafe extern "C" fn(*mut c_void) -> *const PlainCString,
    count: unsafe extern "C" fn(*mut c_void) -> i32,
    arg: unsafe extern "C" fn(*mut c_void, i32) -> *const VariantOut,
}

#[repr(C)]
struct AddMessage {
    vtable: &'static MessageVtable,
    id: PlainCString,
    argument: VariantOut,
    _class: Owned,
}

unsafe extern "C" fn message_query(
    this: *mut c_void,
    iid: *const Uid,
    result: *mut *mut c_void,
) -> i32 {
    if result.is_null() {
        return NO_INTERFACE;
    }
    unsafe { *result = ptr::null_mut() };
    if !iid.is_null() && matches!(unsafe { *iid }, UNKNOWN | MESSAGE) {
        unsafe {
            message_retain(this);
            *result = this;
        }
        OK
    } else {
        NO_INTERFACE
    }
}

unsafe extern "C" fn message_retain(this: *mut c_void) -> u32 {
    let raw = this.cast::<AddMessage>();
    unsafe { Rc::increment_strong_count(raw) };
    let owner = std::mem::ManuallyDrop::new(unsafe { Rc::from_raw(raw) });
    Rc::strong_count(&owner) as u32
}

unsafe extern "C" fn message_release(this: *mut c_void) -> u32 {
    let owner = unsafe { Rc::from_raw(this.cast::<AddMessage>()) };
    (Rc::strong_count(&owner) - 1) as u32
}

unsafe extern "C" fn message_id(this: *mut c_void) -> *const PlainCString {
    unsafe { &(*this.cast::<AddMessage>()).id }
}

unsafe extern "C" fn message_count(_: *mut c_void) -> i32 {
    1
}

unsafe extern "C" fn message_arg(this: *mut c_void, index: i32) -> *const VariantOut {
    static EMPTY: VariantOut = VariantOut {
        kind: 0,
        payload: 0,
    };
    if index == 0 {
        unsafe { &(*this.cast::<AddMessage>()).argument }
    } else {
        &EMPTY
    }
}

static ADD_MESSAGE: MessageVtable = MessageVtable {
    base: UnknownVtable {
        query: message_query,
        retain: message_retain,
        release: message_release,
    },
    id: message_id,
    count: message_count,
    arg: message_arg,
};

impl AddMessage {
    fn prepare(ctx: &Context<'_>) -> Result<Rc<Self>, String> {
        let class = constant(c"{B625F134-4485-4A50-A3C8-C9CF0C5495E1}")?;
        let class =
            hold(unsafe { (*class).object }).ok_or("stock Mai Tai UID string unavailable")?;
        ctx.check()?;
        let id = unsafe { (ctx.api.cstring)(c"insertDeviceClass".as_ptr()).as_ref() }
            .copied()
            .ok_or("Multi Instrument fixed message name unavailable")?;
        Ok(Rc::new(Self {
            vtable: &ADD_MESSAGE,
            id,
            argument: VariantOut {
                kind: VARIANT_STRING,
                payload: class.0 as u64,
            },
            _class: class,
        }))
    }
}

fn constant(name: &std::ffi::CStr) -> Result<*const PlainString, String> {
    let getter = constant_string().ok_or("native Multi Instrument constant strings unavailable")?;
    let result = unsafe { getter(name.as_ptr()) };
    (!result.is_null())
        .then_some(result)
        .ok_or_else(|| "native Multi Instrument constant string unavailable".into())
}

fn distinct_ids(ids: &[&str]) -> bool {
    ids.iter()
        .enumerate()
        .all(|(index, id)| valid_uid(id) && !ids[..index].contains(id))
}

fn has_insert_method(receiver: &Owned, ctx: &Context<'_>) -> Result<bool, String> {
    let type_info: Borrowed = unsafe { method(receiver.0, 3) };
    let mut info = unsafe { type_info(receiver.0) };
    for _ in 0..8 {
        if info.is_null() {
            return Ok(false);
        }
        let name: unsafe extern "C" fn(*mut c_void) -> *const c_char = unsafe { method(info, 5) };
        if ascii(unsafe { name(info) }) == "DeviceFolder" {
            let members: unsafe extern "C" fn(*mut c_void) -> *const MethodDefinition =
                unsafe { method(info, 8) };
            let definitions = unsafe { members(info) };
            if definitions.is_null() {
                return Ok(false);
            }
            for index in 0..32 {
                let definition = unsafe { &*definitions.add(index) };
                if definition.name.is_null() {
                    return Ok(false);
                }
                if ascii(definition.name) == "insertDeviceClass" {
                    return Ok(ascii(definition.args)
                        == "classID: UID | string, position: int = -1"
                        && ascii(definition.result) == "DeviceSlot");
                }
            }
            return Ok(false);
        }
        let parent: Borrowed = unsafe { method(info, 3) };
        info = unsafe { parent(info) };
        ctx.check()?;
    }
    Ok(false)
}

const SINGLE_CHILD_NODES: i32 = 3;

// The Instruments folder has two host housekeeping children before Inst01.
fn single_child_shape(folder_count: i32, graph_count: i32, tracked_slots: usize) -> bool {
    folder_count == 3 && graph_count == SINGLE_CHILD_NODES && tracked_slots == 1
}

struct GraphChild {
    node: Node,
    id: String,
    slot: *mut c_void,
}

struct Route {
    child: Binding,
    folder_receiver: Owned,
    graph: Node,
    members: Vec<GraphChild>,
    graph_name: *const PlainString,
    slot_name: *const PlainString,
    graph_id: String,
    folder_id: String,
}

impl Route {
    fn resolve(
        ctx: &Context<'_>,
        folder_id: &str,
        graph_id: &str,
        slot_id: String,
        device_id: &str,
    ) -> Result<Self, String> {
        let child = Binding::resolve(ctx, Slot::Inst01, slot_id, device_id)?;
        let guard = &mut || child.check(ctx);
        let graph_name = constant(c"Graph")?;
        let slot_name = constant(c"Slot")?;
        let graph_object = indexed_child(
            GRAPH_INDEX,
            || child.root.child_count(),
            |index| child.root.child(index),
            guard,
        )?;
        let graph = Node::capture(graph_object, guard)?;
        if child.folder.uid(4) != folder_id
            || graph.uid(4) != graph_id
            || child.folder.child_count() != 3
            || graph.child_count() != SINGLE_CHILD_NODES
        {
            return Err(
                "the exact Multi Instrument must still own one child and three graph nodes".into(),
            );
        }
        let mut members = Vec::with_capacity(SINGLE_CHILD_NODES as usize);
        let mut tracked_slots = 0;
        for index in 0..SINGLE_CHILD_NODES {
            let pointer = indexed_child(
                index,
                || graph.child_count(),
                |index| graph.child(index),
                guard,
            )?;
            let node = Node::capture(pointer, guard)?;
            let id = node.uid(4);
            let slot = node.find(slot_name);
            if !slot.is_null() {
                if slot != child.slot.view.0 {
                    return Err("the Multi Instrument graph points to an unexpected Slot".into());
                }
                tracked_slots += 1;
            }
            members.push(GraphChild { node, id, slot });
        }
        let ids: Vec<_> = members.iter().map(|member| member.id.as_str()).collect();
        if !single_child_shape(
            child.folder.child_count(),
            graph.child_count(),
            tracked_slots,
        ) || !distinct_ids(&ids)
        {
            return Err(
                "the original Mai Tai does not own the sole attached Multi Instrument Slot".into(),
            );
        }
        let folder_receiver = interface(child.folder.object.0, &OBJECT, guard)?;
        let canonical = interface(folder_receiver.0, &UNKNOWN, guard)?;
        let same_receiver = canonical.0 == child.folder.canonical.0;
        release(canonical, guard)?;
        if !same_receiver || class_name(folder_receiver.0) != "CombinatorSynthFolder" {
            return Err("the exact CombinatorSynthFolder receiver is unavailable".into());
        }
        guard()?;
        if !has_insert_method(&folder_receiver, ctx)? {
            return Err("the Multi Instrument stock insertion descriptor is unavailable".into());
        }
        Ok(Self {
            child,
            folder_receiver,
            graph,
            members,
            graph_name,
            slot_name,
            graph_id: graph_id.into(),
            folder_id: folder_id.into(),
        })
    }

    fn check(&self, ctx: &Context<'_>) -> Result<(), String> {
        self.child.check(ctx)?;
        // Each graph member is retained. The final witness releases no temporary owner.
        if self.child.folder.uid(4) != self.folder_id
            || self.child.folder.child_count() != 3
            || self.graph.uid(4) != self.graph_id
            || self.graph.parent() != self.child.root.view.0
            || self.child.root.child(GRAPH_INDEX) != self.graph.object.0
            || self.child.root.find(self.graph_name) != self.graph.view.0
            || self.graph.child_count() != SINGLE_CHILD_NODES
        {
            return Err("the exact one-child Multi Instrument folder or graph changed".into());
        }
        for (index, member) in self.members.iter().enumerate() {
            if self.graph.child(index as i32) != member.node.object.0
                || member.node.uid(4) != member.id
                || member.node.parent() != self.graph.view.0
                || member.node.find(self.slot_name) != member.slot
            {
                return Err(
                    "a retained Multi Instrument graph node or Slot attachment changed".into(),
                );
            }
        }
        Ok(())
    }
}

pub(in crate::native_script::request) fn dispatch(
    ctx: Box<Context<'static>>,
    token: &str,
    folder_id: &str,
    graph_id: &str,
    slot_id: String,
    device_id: &str,
) -> Reply {
    let mut dispatched = false;
    let result = (|| -> Result<Value, String> {
        if !runtime_supported()
            || !distinct_ids(&[
                &ctx.instance.object_uid,
                folder_id,
                graph_id,
                &slot_id,
                device_id,
            ])
        {
            return Err(
                "Multi Instrument requires the exact arm64 host and distinct native identities"
                    .into(),
            );
        }
        document_context::check_current_token(&ctx, token)?;
        let route = Route::resolve(&ctx, folder_id, graph_id, slot_id, device_id)?;
        let message = AddMessage::prepare(&ctx)?;
        let invoke: unsafe extern "C" fn(*mut c_void, *mut VariantOut, *mut c_void) -> u8 =
            unsafe { method(route.folder_receiver.0, 7) };
        let mut output = VariantOut {
            kind: 0,
            payload: 0,
        };
        route.check(&ctx)?;
        document_context::check_current_token(&ctx, token)?;
        dispatched = true;
        let accepted = unsafe {
            invoke(
                route.folder_receiver.0,
                &mut output,
                Rc::as_ptr(&message).cast_mut().cast(),
            ) != 0
        };
        let returned_object =
            output.kind & VARIANT_TYPE_MASK == VARIANT_OBJECT && output.payload != 0;
        if output.kind & VARIANT_SHARED != 0
            && matches!(
                output.kind & VARIANT_TYPE_MASK,
                VARIANT_STRING | VARIANT_OBJECT
            )
            && output.payload != 0
        {
            let value = output.payload as *mut c_void;
            unsafe { (unknown_vtable(value).release)(value) };
        }
        drop(message);
        drop(route);
        document_context::check_current_token(&ctx, token)?;
        if !accepted || !returned_object {
            return Err("Multi Instrument fixed method result did not report an object".into());
        }
        Ok(json!({
            "ok": true,
            "operation": "multi_add_mai_tai",
            "outcome": "applied_unverified",
            "instance": ctx.instance,
            "dispatch_attempted": true,
            "instrument": "Mai Tai",
            "graph_attachment_verified": false,
            "undo_verified": false,
        }))
    })();
    let mut reply = match result {
        Ok(value) => Reply::unbound(value),
        Err(reason) => Reply::multi_add_mai_tai_error(&reason, dispatched),
    };
    if dispatched {
        reply.context = Some(ctx);
        reply.mutation = true;
    }
    reply
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_three_graph_nodes_and_one_old_slot() {
        assert!(single_child_shape(3, 3, 1));
        assert!(!single_child_shape(2, 0, 0));
        assert!(!single_child_shape(4, 7, 2));
        assert!(!single_child_shape(3, 3, 0));
        assert!(!single_child_shape(3, 3, 2));
        assert!(!single_child_shape(3, 1, 1));
        assert!(!single_child_shape(4, 3, 1));
    }

    #[test]
    fn ambiguous_identity_is_refused() {
        let id = "12345678-1234-1234-1234-123456789ABC";
        assert!(distinct_ids(&[id, "22345678-1234-1234-1234-123456789ABC"]));
        assert!(!distinct_ids(&[id, id]));
        assert!(!distinct_ids(&[id, "22345678-1234-1234-1234-123456789abc"]));
    }
}

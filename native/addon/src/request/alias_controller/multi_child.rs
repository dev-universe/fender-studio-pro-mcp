use super::*;

#[path = "multi_child/add_mai_tai.rs"]
mod add_mai_tai;
pub(in crate::native_script::request) use add_mai_tai::dispatch as dispatch_add_mai_tai;

const MULTI: &str = "B9D410AD-E47D-495E-8AEE-10D2B2987CC7";
const INSTRUMENTS: &str = "8BECF8A0-B995-4376-BFDD-1BFCC373C5FD";
const MAI_TAI: &str = "B625F134-4485-4A50-A3C8-C9CF0C5495E1";
const MAX_CHILDREN: i32 = 32;
const INSTRUMENTS_INDEX: i32 = 0;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub(in crate::native_script::request) enum Slot {
    Inst01,
    Inst02,
}

impl Slot {
    fn id(self) -> &'static str {
        match self {
            Self::Inst01 => "Inst01",
            Self::Inst02 => "Inst02",
        }
    }

    fn index(self) -> i32 {
        match self {
            Self::Inst01 => 2,
            Self::Inst02 => 3,
        }
    }
}

struct Node {
    view: Owned,
    canonical: Owned,
    object: Owned,
}

impl Node {
    fn capture(
        pointer: *mut c_void,
        guard: &mut impl FnMut() -> Result<(), String>,
    ) -> Result<Self, String> {
        let object = pin(pointer, guard)?;
        let canonical = interface(object.0, &UNKNOWN, guard)?;
        let view = interface(object.0, &NODE, guard)?;
        let view_identity = interface(view.0, &UNKNOWN, guard)?;
        let same = canonical.0 == view_identity.0;
        release(view_identity, guard)?;
        if !same {
            return Err("Multi Instrument object and node identities differ".into());
        }
        Ok(Self {
            view,
            canonical,
            object,
        })
    }

    fn parent(&self) -> *mut c_void {
        borrowed(self.view.0, 6)
    }

    fn find(&self, name: *const PlainString) -> *mut c_void {
        let find: unsafe extern "C" fn(*mut c_void, *const PlainString) -> *mut c_void =
            unsafe { method(self.view.0, 10) };
        unsafe { find(self.view.0, name) }
    }

    fn child_count(&self) -> i32 {
        let get: Count = unsafe { method(self.view.0, 8) };
        unsafe { get(self.view.0) }
    }

    fn child(&self, index: i32) -> *mut c_void {
        let get: unsafe extern "C" fn(*mut c_void, i32) -> *mut c_void =
            unsafe { method(self.view.0, 9) };
        unsafe { get(self.view.0, index) }
    }

    fn uid(&self, slot: usize) -> String {
        let get: unsafe extern "C" fn(*mut c_void) -> *const Uid =
            unsafe { method(self.view.0, slot) };
        uid(unsafe { get(self.view.0) })
    }

    fn id(&self) -> Result<String, String> {
        let get: unsafe extern "C" fn(*mut c_void) -> *const PlainString =
            unsafe { method(self.view.0, 3) };
        let id = unsafe { get(self.view.0) };
        if id.is_null() {
            return Err("Multi Instrument node identifier unavailable".into());
        }
        let text =
            hold(unsafe { (*id).object }).ok_or("Multi Instrument node string unavailable")?;
        copy_string(text.0, 512)
    }
}

fn indexed_child<T>(
    index: i32,
    count: impl FnOnce() -> i32,
    get_child: impl FnOnce(i32) -> T,
    guard: &mut impl FnMut() -> Result<(), String>,
) -> Result<T, String> {
    guard()?;
    let total = count();
    if index >= total || total > MAX_CHILDREN {
        return Err("Multi Instrument indexed attachment is outside its current bound".into());
    }
    guard()?;
    Ok(get_child(index))
}

fn matching_attachment<T: Eq>(
    observed_device: T,
    expected_device: T,
    observed_parent: T,
    expected_parent: T,
) -> Result<(), String> {
    if observed_device != expected_device || observed_parent != expected_parent {
        return Err("Multi Instrument indexed object or parent attachment changed".into());
    }
    Ok(())
}

fn matching_child(instance: &Instance, expected_uid: &str) -> Result<(), String> {
    if instance.object_uid != expected_uid
        || instance.class_uid != MAI_TAI
        || instance.native_class != "AudioSynth"
    {
        return Err("expected Mai Tai AudioSynth child identity changed".into());
    }
    Ok(())
}

pub(in crate::native_script::request) struct Binding {
    device: Node,
    slot: Node,
    folder: Node,
    root: Node,
    controller: ControllerBinding,
    device_name: *const PlainString,
    index: i32,
    slot_uid: String,
    child: Instance,
}

impl Binding {
    fn resolve(
        ctx: &Context<'_>,
        slot: Slot,
        slot_uid: String,
        child_uid: &str,
    ) -> Result<Self, String> {
        ctx.check()?;
        if ctx.instance.class_uid != MULTI || ctx.instance.native_class != "AudioSynth" {
            return Err("exact Multi Instrument AudioSynth required".into());
        }
        let guard = &mut || ctx.check();
        let alias = AliasBinding::capture(ctx.target.0, ctx.node.0, guard)?;
        let controller = ControllerBinding::capture(alias, guard)?;
        let guard = &mut || controller.check(guard);
        let constant = constant_string().ok_or("Multi Instrument name constants unavailable")?;
        let device_name = unsafe { constant(c"Device".as_ptr()) };
        if device_name.is_null() {
            return Err("Multi Instrument name constants unavailable".into());
        }
        guard()?;
        let root = Node::capture(controller.controller.0, guard)?;
        if root.canonical.0 != controller.identity.0 {
            return Err("Multi Instrument declared controller identity changed".into());
        }
        let folder = indexed_child(
            INSTRUMENTS_INDEX,
            || root.child_count(),
            |index| root.child(index),
            guard,
        )?;
        let folder = Node::capture(folder, guard)?;
        if folder.uid(5) != INSTRUMENTS
            || folder.id()? != "Instruments"
            || folder.parent() != root.view.0
        {
            return Err("exact attached Instruments folder required".into());
        }
        guard()?;
        let index = slot.index();
        let slot_id = slot.id();
        let slot = indexed_child(
            index,
            || folder.child_count(),
            |index| folder.child(index),
            guard,
        )?;
        let slot = Node::capture(slot, guard)?;
        if slot.uid(4) != slot_uid || slot.id()? != slot_id || slot.parent() != folder.view.0 {
            return Err("expected Multi Instrument slot attachment changed".into());
        }
        guard()?;
        let device = Node::capture(slot.find(device_name), guard)?;
        let child = instance(device.object.0)?;
        guard()?;
        matching_child(&child, child_uid)?;
        let route = Self {
            device,
            slot,
            folder,
            root,
            controller,
            device_name,
            index,
            slot_uid,
            child,
        };
        route.check(ctx)?;
        Ok(route)
    }

    pub(in crate::native_script::request) fn check(&self, ctx: &Context<'_>) -> Result<(), String> {
        self.controller.check(&mut || ctx.check())?;
        let guard = &mut || ctx.check();
        let folder = indexed_child(
            INSTRUMENTS_INDEX,
            || self.root.child_count(),
            |index| self.root.child(index),
            guard,
        )?;
        matching_attachment(
            folder,
            self.folder.object.0,
            self.folder.parent(),
            self.root.view.0,
        )?;
        let slot = indexed_child(
            self.index,
            || self.folder.child_count(),
            |index| self.folder.child(index),
            guard,
        )?;
        matching_attachment(
            slot,
            self.slot.object.0,
            self.slot.parent(),
            self.folder.view.0,
        )?;
        guard()?;
        matching_attachment(
            self.slot.find(self.device_name),
            self.device.object.0,
            self.device.parent(),
            self.slot.view.0,
        )?;
        if self.folder.uid(5) != INSTRUMENTS
            || self.slot.uid(4) != self.slot_uid
            || self.device.uid(4) != self.child.object_uid
            || self.device.uid(5) != MAI_TAI
        {
            return Err("Multi Instrument child path identity changed".into());
        }
        guard()
    }
}

pub(in crate::native_script::request) fn dispatch(
    ctx: Box<Context<'static>>,
    slot: Slot,
    slot_uid: String,
    child_uid: &str,
    offset: u32,
    limit: u32,
) -> Reply {
    let result = (|| {
        let binding = Binding::resolve(&ctx, slot, slot_uid, child_uid)?;
        let guard = &mut || binding.check(&ctx);
        let metadata = dsp_for(&ctx, binding.device.object.0)?;
        guard()?;
        let direct = Controller::direct(binding.device.object.0);
        guard()?;
        let provider = DspProvider {
            metadata,
            direct: Ok(direct?),
        };
        let result = dsp_page(ctx.api, &provider, offset, limit, guard);
        release(provider, guard)?;
        let mut value = result?;
        value["ok"] = json!(true);
        value["outcome"] = json!("observed");
        value["operation"] = json!("multi_child_parameters");
        value["source"] = json!("dsp");
        value["instance"] = json!(ctx.instance);
        value["child_slot"] = json!(slot);
        value["child_slot_uid"] = json!(binding.slot_uid);
        value["child_instance"] = json!(binding.child);
        value["mutation_capability"] = json!(false);
        Ok::<_, String>((value, binding))
    })();
    let mut reply = match result {
        Ok((value, binding)) => {
            let mut reply = Reply::unbound(value);
            reply.provider = Some(Provider::MultiChild(Box::new(binding)));
            reply
        }
        Err(reason) => Reply::error("multi_child_parameters_unavailable", &reason, "not_applied"),
    };
    reply.context = Some(ctx);
    reply
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn identical_mai_tai_classes_still_require_the_issued_child_uid() {
        let first = Instance {
            object_uid: "12345678-1234-1234-1234-123456789ABC".into(),
            class_uid: MAI_TAI.into(),
            native_class: "AudioSynth".into(),
            object_id: "Saved preset title".into(),
        };
        let mut second = first.clone();
        second.object_uid = "22345678-1234-1234-1234-123456789ABC".into();
        assert_eq!(Slot::Inst01.index(), 2);
        assert_eq!(Slot::Inst02.index(), 3);
        assert!(matching_child(&first, &first.object_uid).is_ok());
        assert!(matching_child(&second, &second.object_uid).is_ok());
        assert!(matching_child(&first, &second.object_uid).is_err());
        assert!(matching_child(&second, &first.object_uid).is_err());
    }

    #[test]
    fn indexed_attachment_refuses_replacement_and_wrong_parent() {
        let children = Cell::new([0, 1, 12, 13]);
        let parent = Cell::new(5);
        let check = || {
            let item = indexed_child(
                Slot::Inst01.index(),
                || 4,
                |index| children.get()[index as usize],
                &mut || Ok(()),
            )?;
            matching_attachment(item, 12, parent.get(), 5)
        };
        assert!(check().is_ok());
        parent.set(6);
        assert!(check().is_err());
        parent.set(5);
        children.set([0, 1, 13, 12]);
        assert!(check().is_err());
    }

    #[test]
    fn stale_context_prevents_indexed_child_access_even_after_count() {
        for stale_on_check in [1, 2] {
            let count_calls = Cell::new(0);
            let indexed_calls = Cell::new(0);
            let mut checks = 0;
            let result = indexed_child(
                Slot::Inst01.index(),
                || {
                    count_calls.set(count_calls.get() + 1);
                    4
                },
                |_| indexed_calls.set(indexed_calls.get() + 1),
                &mut || {
                    checks += 1;
                    if checks == stale_on_check {
                        return Err("document changed".into());
                    }
                    Ok(())
                },
            );
            assert_eq!(result, Err("document changed".into()));
            assert_eq!(count_calls.get(), stale_on_check - 1);
            assert_eq!(indexed_calls.get(), 0);
        }
    }
}

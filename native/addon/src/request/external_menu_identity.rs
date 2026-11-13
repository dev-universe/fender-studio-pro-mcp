//! Held target identity plus fresh direct Inserts membership; no menuData reflection.
use super::*;

pub(super) struct Identity {
    object: Owned,
    info: Owned,
    id: Owned,
    slot: Owned,
    slot_id: Owned,
    folder: Owned,
    channel: Owned,
    slot_name: PlainString,
    slot_id_value: String,
    inserts_name: *const PlainString,
    device_name: *const PlainString,
}
fn parent(node: *mut c_void, ctx: &Context<'_>) -> Result<Owned, String> {
    ctx.check()?;
    let f: Borrowed = unsafe { method(node, 6) };
    let owner = hold(unsafe { f(node) }); // retain borrowed view before a callback guard
    ctx.check()?;
    owner.ok_or("External provenance parent unavailable".into())
}
fn read_id(node: *mut c_void, ctx: &Context<'_>) -> Result<(Owned, *const PlainString), String> {
    ctx.check()?;
    let f: unsafe extern "C" fn(*mut c_void) -> *const PlainString = unsafe { method(node, 3) };
    let value = unsafe { f(node) };
    let owner = if value.is_null() {
        None
    } else {
        hold(unsafe { (*value).object })
    };
    ctx.check()?;
    Ok((
        owner.ok_or("External provenance identifier unavailable")?,
        value,
    ))
}
fn text(owner: &Owned, ctx: &Context<'_>) -> Result<String, String> {
    ctx.check()?;
    let length: unsafe extern "C" fn(*mut c_void) -> i32 = unsafe { method(owner.0, 4) };
    let before = unsafe { length(owner.0) };
    ctx.check()?;
    if !(0..=512).contains(&before) {
        return Err("External identifier length exceeds bound".into());
    }
    let mut units = vec![0xffff; before as usize + 1];
    let copy: unsafe extern "C" fn(*mut c_void, *mut u16, i32) -> i32 =
        unsafe { method(owner.0, 8) };
    let status = unsafe { copy(owner.0, units.as_mut_ptr(), units.len() as i32) };
    ctx.check()?;
    let after = unsafe { length(owner.0) };
    ctx.check()?;
    if status != OK || before != after || units[before as usize] != 0 {
        return Err("External identifier changed or copy failed".into());
    }
    String::from_utf16(&units[..before as usize])
        .map_err(|_| "External identifier is invalid UTF-16".into())
}
impl Identity {
    pub(super) fn capture(ctx: &Context<'_>) -> Result<Self, String> {
        ctx.check()?;
        if ctx.instance.class_uid != "889DD9F8-14F0-44A5-BCEE-0F2B157FC5D4"
            || ctx.instance.native_class != "AudioEffect"
            || ctx.instance.object_id != "External Effect Stereo"
        {
            return Err("External provenance requires the held Stereo AudioEffect identity".into());
        }
        let object = query(ctx.target.0, &OBJECT).ok_or("External target IObject unavailable")?;
        ctx.check()?;
        let get: Borrowed = unsafe { method(object.0, 3) };
        let info = hold(unsafe { get(object.0) });
        ctx.check()?;
        let info = info.ok_or("External target ITypeInfo unavailable")?;
        let (id, _) = read_id(ctx.node.0, ctx)?;
        let slot = parent(ctx.node.0, ctx)?;
        let (slot_id, _) = read_id(slot.0, ctx)?;
        let slot_id_value = text(&slot_id, ctx)?;
        ctx.check()?;
        let slot_name = PlainString {
            object: slot_id.0,
            opaque_debug: std::ptr::null_mut(),
        };
        let folder = parent(slot.0, ctx)?;
        let channel = parent(folder.0, ctx)?;
        let constants = constant_string().ok_or("native string constants unavailable")?;
        let inserts_name = unsafe { constants(c"Inserts".as_ptr()) };
        let device_name = unsafe { constants(c"Device".as_ptr()) };
        ctx.check()?;
        if inserts_name.is_null() || device_name.is_null() {
            return Err("External route constants unavailable".into());
        }
        let result = Self {
            object,
            info,
            id,
            slot,
            slot_id,
            folder,
            channel,
            slot_name,
            slot_id_value,
            inserts_name,
            device_name,
        };
        result.check(ctx)?;
        Ok(result)
    }
    pub(super) fn check(&self, ctx: &Context<'_>) -> Result<(), String> {
        ctx.check()?;
        let get_info: Borrowed = unsafe { method(self.object.0, 3) };
        let info = unsafe { get_info(self.object.0) };
        ctx.check()?;
        if info != self.info.0 {
            return Err("External target type-info identity changed".into());
        }
        let get_name: unsafe extern "C" fn(*mut c_void) -> *const c_char =
            unsafe { method(self.info.0, 5) };
        let native_class = ascii(unsafe { get_name(self.info.0) });
        ctx.check()?;
        let get_id: unsafe extern "C" fn(*mut c_void) -> *const PlainString =
            unsafe { method(ctx.node.0, 3) };
        let current = unsafe { get_id(ctx.node.0) };
        if current.is_null() || unsafe { (*current).object } != self.id.0 {
            return Err("External target identifier identity changed".into());
        }
        ctx.check()?;
        let object_id = text(&self.id, ctx)?;
        ctx.check()?;
        if native_class != ctx.instance.native_class || object_id != ctx.instance.object_id {
            return Err("External four-field instance changed".into());
        }
        let slot_id: unsafe extern "C" fn(*mut c_void) -> *const PlainString =
            unsafe { method(self.slot.0, 3) };
        let current = unsafe { slot_id(self.slot.0) };
        let current_object = if current.is_null() {
            std::ptr::null_mut()
        } else {
            unsafe { (*current).object }
        };
        ctx.check()?;
        if current_object != self.slot_id.0 || text(&self.slot_id, ctx)? != self.slot_id_value {
            return Err("External slot identifier changed".into());
        }
        for (child, parent) in [
            (ctx.node.0, self.slot.0),
            (self.slot.0, self.folder.0),
            (self.folder.0, self.channel.0),
        ] {
            let get: Borrowed = unsafe { method(child, 6) };
            let current = unsafe { get(child) };
            ctx.check()?;
            if current != parent {
                return Err("External parent attachment changed".into());
            }
        }
        for (owner, name, expected) in [
            (self.slot.0, self.device_name, ctx.node.0),
            (
                self.folder.0,
                &self.slot_name as *const PlainString,
                self.slot.0,
            ),
            (self.channel.0, self.inserts_name, self.folder.0),
        ] {
            let find: unsafe extern "C" fn(*mut c_void, *const PlainString) -> *mut c_void =
                unsafe { method(owner, 10) };
            let current = unsafe { find(owner, name) };
            ctx.check()?;
            if current != expected {
                return Err("External current Inserts membership changed".into());
            }
        }
        ctx.check()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[repr(C)]
    struct Fake {
        table: *const *const c_void,
        refs: AtomicU32,
        parent: *mut c_void,
        found: *mut c_void,
        info: *mut c_void,
        id: PlainString,
        units: Vec<u16>,
        name: *const c_char,
    }
    unsafe extern "C" fn retain(p: *mut c_void) -> u32 {
        unsafe { &*p.cast::<Fake>() }
            .refs
            .fetch_add(1, Ordering::Relaxed)
            + 1
    }
    unsafe extern "C" fn release(p: *mut c_void) -> u32 {
        unsafe { &*p.cast::<Fake>() }
            .refs
            .fetch_sub(1, Ordering::Relaxed)
            - 1
    }
    unsafe extern "C" fn get_info(p: *mut c_void) -> *mut c_void {
        unsafe { (*p.cast::<Fake>()).info }
    }
    unsafe extern "C" fn get_name(p: *mut c_void) -> *const c_char {
        unsafe { (*p.cast::<Fake>()).name }
    }
    unsafe extern "C" fn get_id(p: *mut c_void) -> *const PlainString {
        unsafe { &(*p.cast::<Fake>()).id }
    }
    unsafe extern "C" fn get_parent(p: *mut c_void) -> *mut c_void {
        unsafe { (*p.cast::<Fake>()).parent }
    }
    unsafe extern "C" fn find(p: *mut c_void, _: *const PlainString) -> *mut c_void {
        unsafe { (*p.cast::<Fake>()).found }
    }
    unsafe extern "C" fn length(p: *mut c_void) -> i32 {
        unsafe { (*p.cast::<Fake>()).units.len() as i32 }
    }
    unsafe extern "C" fn copy(p: *mut c_void, out: *mut u16, cap: i32) -> i32 {
        let v = unsafe { &(*p.cast::<Fake>()).units };
        if cap <= v.len() as i32 {
            return FAILED;
        }
        unsafe {
            ptr::copy_nonoverlapping(v.as_ptr(), out, v.len());
            *out.add(v.len()) = 0;
        }
        OK
    }
    static DEVICE_ID: AtomicPtr<PlainString> = AtomicPtr::new(ptr::null_mut());
    static DEVICE_PARENT: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
    unsafe extern "C" fn device_id(_: *mut c_void) -> *const PlainString {
        DEVICE_ID.load(Ordering::Acquire)
    }
    unsafe extern "C" fn device_parent(_: *mut c_void) -> *mut c_void {
        DEVICE_PARENT.load(Ordering::Acquire)
    }
    fn fake(table: &[*const c_void; 11], text: &str) -> Box<Fake> {
        let mut v = Box::new(Fake {
            table: table.as_ptr(),
            refs: AtomicU32::new(1),
            parent: ptr::null_mut(),
            found: ptr::null_mut(),
            info: ptr::null_mut(),
            id: PlainString {
                object: ptr::null_mut(),
                opaque_debug: ptr::null_mut(),
            },
            units: text.encode_utf16().collect(),
            name: c"AudioEffect".as_ptr(),
        });
        v.id.object = ptr::addr_of_mut!(*v).cast();
        v
    }
    #[test]
    fn provenance_held_strings_do_not_hide_fresh_parent_member_or_context_loss() {
        let _serial = super::super::super::super::tests::TEST_LOCK.lock().unwrap();
        let mut fixture = super::super::super::guard_tests::Fixture::new();
        let mut ctx = fixture.context();
        ctx.instance.native_class = "AudioEffect".into();
        ctx.instance.object_id = "External Effect Stereo".into();
        let mut object_table = [ptr::null(); 11];
        object_table[1] = retain as *const c_void;
        object_table[2] = release as *const c_void;
        object_table[3] = get_info as *const c_void;
        let mut info_table = object_table;
        info_table[5] = get_name as *const c_void;
        let mut string_table = object_table;
        string_table[4] = length as *const c_void;
        string_table[8] = copy as *const c_void;
        let mut node_table = object_table;
        node_table[3] = get_id as *const c_void;
        node_table[6] = get_parent as *const c_void;
        node_table[10] = find as *const c_void;
        let mut object = fake(&object_table, "");
        let mut info = fake(&info_table, "");
        let mut id = fake(&string_table, "External Effect Stereo");
        let mut slot_text = fake(&string_table, "FX01");
        let mut slot = fake(&node_table, "");
        let mut folder = fake(&node_table, "");
        let mut channel = fake(&node_table, "");
        let object_p = ptr::addr_of_mut!(*object).cast();
        let info_p = ptr::addr_of_mut!(*info).cast();
        let id_p = ptr::addr_of_mut!(*id).cast();
        let slot_p = ptr::addr_of_mut!(*slot).cast();
        let folder_p = ptr::addr_of_mut!(*folder).cast();
        let channel_p = ptr::addr_of_mut!(*channel).cast();
        object.info = info_p;
        slot.id.object = ptr::addr_of_mut!(*slot_text).cast();
        slot.parent = folder_p;
        folder.parent = channel_p;
        slot.found = ctx.node.0;
        folder.found = slot_p;
        channel.found = folder_p;
        DEVICE_ID.store(ptr::addr_of_mut!(id.id), Ordering::Release);
        DEVICE_PARENT.store(slot_p, Ordering::Release);
        let old = unsafe { *ctx.node.0.cast::<*const *const c_void>() };
        let mut device_table = [ptr::null(); 10];
        unsafe {
            ptr::copy_nonoverlapping(old, device_table.as_mut_ptr(), 10);
        }
        device_table[3] = device_id as *const c_void;
        device_table[6] = device_parent as *const c_void;
        unsafe {
            *ctx.node.0.cast::<*const *const c_void>() = device_table.as_ptr();
        }
        let anchor = Identity {
            object: hold(object_p).unwrap(),
            info: hold(info_p).unwrap(),
            id: hold(id_p).unwrap(),
            slot: hold(slot_p).unwrap(),
            slot_id: hold(slot.id.object).unwrap(),
            folder: hold(folder_p).unwrap(),
            channel: hold(channel_p).unwrap(),
            slot_name: PlainString {
                object: slot.id.object,
                opaque_debug: ptr::null_mut(),
            },
            slot_id_value: "FX01".into(),
            inserts_name: ptr::addr_of!(slot.id),
            device_name: ptr::addr_of!(slot.id),
        };
        assert!(anchor.check(&ctx).is_ok());
        for role in 0..3 {
            let node = match role {
                0 => &mut slot,
                1 => &mut folder,
                _ => &mut channel,
            };
            let before = node.found;
            node.found = ptr::null_mut();
            assert!(anchor.check(&ctx).unwrap_err().contains("membership"));
            node.found = before;
            assert!(anchor.check(&ctx).is_ok());
        }
        slot_text.units = "FX02".encode_utf16().collect();
        assert!(anchor.check(&ctx).unwrap_err().contains("slot identifier"));
        slot_text.units = "FX01".encode_utf16().collect();
        DEVICE_PARENT.store(ptr::null_mut(), Ordering::Release);
        assert!(
            anchor
                .check(&ctx)
                .unwrap_err()
                .contains("parent attachment")
        );
        DEVICE_PARENT.store(slot_p, Ordering::Release);
        id.units = "Other".encode_utf16().collect();
        assert!(anchor.check(&ctx).unwrap_err().contains("four-field"));
        id.units = "External Effect Stereo".encode_utf16().collect();
        id.units = vec![0xd800];
        assert!(anchor.check(&ctx).unwrap_err().contains("UTF-16"));
        id.units = vec![65; 513];
        assert!(anchor.check(&ctx).unwrap_err().contains("length"));
        id.units = "External Effect Stereo".encode_utf16().collect();
        info.name = c"Other".as_ptr();
        assert!(anchor.check(&ctx).unwrap_err().contains("four-field"));
        info.name = c"AudioEffect".as_ptr();
        object.info = ptr::null_mut();
        assert!(anchor.check(&ctx).unwrap_err().contains("type-info"));
        object.info = info_p;
        fixture.switch_document();
        assert!(anchor.check(&ctx).is_err());
        drop(anchor);
        for v in [&object, &info, &id, &slot_text, &slot, &folder, &channel] {
            assert_eq!(v.refs.load(Ordering::Acquire), 1);
        }
        unsafe {
            *ctx.node.0.cast::<*const *const c_void>() = old;
        }
        drop(ctx);
        DEVICE_ID.store(ptr::null_mut(), Ordering::Release);
        DEVICE_PARENT.store(ptr::null_mut(), Ordering::Release);
    }
}

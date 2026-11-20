use super::*;

// Studio Pro 8.1.1.113103 arm64 entries, relative to the pinned main image.
const PORT_SETUP_GET_OBJECT: usize = 0xc27db4;
const PORT_SETUP_GET_SYNTH: usize = 0xc26fac;
const OUTPUT_LIST_TABLE: usize = 0x1be1118;
const CONTROLLER_FACET: usize = 0x78;
const OUTPUT_LIST_HEAD: usize = 0x338;
const MAX_OUTPUT_GROUPS: usize = 16;
const OUTPUT_COUNT: i32 = 32;
const MAIN_OUTPUT_GROUP: [u8; 16] = [
    0xa1, 0x6a, 0xdf, 0xa9, 0x4f, 0xd1, 0xb5, 0x40, 0x8b, 0x90, 0x8d, 0xc6, 0x88, 0xe7, 0xbc, 0x75,
];

#[derive(PartialEq, Eq, Serialize)]
struct Output {
    index: i32,
    name: String,
    requested: bool,
    acknowledged: bool,
}

pub(super) struct OutputInventory {
    outputs: Vec<Output>,
    controller: Owned,
    setup: NodePin,
    slot: NodePin,
    device: NodePin,
    setup_name: *const PlainString,
}

impl OutputInventory {
    pub(super) fn capture(ctx: &Context<'_>) -> Result<Self, String> {
        ctx.check()?;
        let instance = &ctx.instance;
        if instance.class_uid != "3713E26C-2FCA-4024-9F25-17E9D2BE2B9B"
            || instance.native_class != "AudioSynth"
            || instance.object_id != "Impact"
            || !valid_uid(&instance.object_uid)
        {
            return Err(
                "Impact output inspection requires exact Impact AudioSynth identity".into(),
            );
        }
        let device = NodePin::borrowed(ctx.node.0)?;
        let slot = device.parent()?;
        if class_name(slot.node.0) != "AudioSynthSlot" {
            return Err("Impact Device is outside its AudioSynthSlot".into());
        }
        let text = constant_string().ok_or("native string constants unavailable")?;
        let setup_name = unsafe { text(c"PortSetup".as_ptr()) };
        if setup_name.is_null() {
            return Err("Impact PortSetup identifier unavailable".into());
        }
        let setup = slot.child(setup_name)?;
        if setup.id()? != "PortSetup" || class_name(setup.node.0) != "AudioFXPortSetup" {
            return Err("Impact PortSetup identity unavailable".into());
        }
        let controller =
            query(setup.node.0, &CONTROLLER).ok_or("Impact PortSetup controller unavailable")?;
        let identity = query(controller.0, &UNKNOWN)
            .ok_or("Impact PortSetup controller identity unavailable")?;
        if identity.0 != setup._canonical.0 {
            return Err("Impact PortSetup controller belongs to a different object".into());
        }
        drop(identity);
        let mut inventory = Self {
            outputs: Vec::new(),
            controller,
            setup,
            slot,
            device,
            setup_name,
        };
        inventory.outputs = inventory.read(&mut || ctx.check())?;
        inventory.check(&mut || ctx.check())?;
        Ok(inventory)
    }

    fn read(&self, guard: &mut impl FnMut() -> Result<(), String>) -> Result<Vec<Output>, String> {
        self.check_attachment()?;
        guard()?;
        let image = unsafe { _dyld_get_image_header(0) };
        if image_uuid(image) != Some(EXPECTED_APP_UUID) {
            return Err("Impact output inspection requires the supported Studio Pro image".into());
        }
        let base = image as usize;
        let get_object: *const c_void = unsafe { method(self.controller.0, 7) };
        if get_object as usize != base + PORT_SETUP_GET_OBJECT {
            return Err("Impact PortSetup is outside the supported controller layout".into());
        }
        let resolver = unsafe { image.cast::<u8>().add(PORT_SETUP_GET_SYNTH) };
        let resolve: Borrowed = unsafe { std::mem::transmute(resolver) };
        // The pinned IController thunk establishes this adjustment to the
        // owning PortSetup. Read the synth through its existing resolver.
        let setup = unsafe { self.controller.0.cast::<u8>().sub(CONTROLLER_FACET).cast() };
        let synth = unsafe { resolve(setup) };
        if synth.is_null() {
            return Err("Impact synth owner unavailable".into());
        }
        guard()?;
        self.check_attachment()?;
        let head: *mut c_void = unsafe { field(synth, OUTPUT_LIST_HEAD) };
        let table = base + OUTPUT_LIST_TABLE;
        let list = existing_main_list(synth, head, table)?;
        if unsafe { field::<i32>(list, 0xa0) } != OUTPUT_COUNT {
            return Err("Impact main output list must have 32 entries".into());
        }
        let entries: *mut c_void = unsafe { field(list, 0x98) };
        if entries.is_null() {
            return Err("Impact main output entry storage unavailable".into());
        }
        let outputs = (0..OUTPUT_COUNT)
            .map(|index| read_output(entries, index))
            .collect::<Result<Vec<_>, _>>()?;
        guard()?;
        self.check_attachment()?;
        if unsafe { resolve(setup) } != synth
            || unsafe { field::<*mut c_void>(synth, OUTPUT_LIST_HEAD) } != head
            || existing_main_list(synth, head, table)? != list
            || unsafe { field::<*mut c_void>(list, 0x98) } != entries
        {
            return Err("Impact main output list changed during inspection".into());
        }
        self.check_attachment()?;
        guard()?;
        Ok(outputs)
    }

    fn check_attachment(&self) -> Result<(), String> {
        let parent: Borrowed = unsafe { method(self.device.node.0, 6) };
        let setup_parent: Borrowed = unsafe { method(self.setup.node.0, 6) };
        let find: unsafe extern "C" fn(*mut c_void, *const PlainString) -> *mut c_void =
            unsafe { method(self.slot.node.0, 10) };
        if unsafe { parent(self.device.node.0) } != self.slot.node.0
            || unsafe { setup_parent(self.setup.node.0) } != self.slot.node.0
            || unsafe { find(self.slot.node.0, self.setup_name) } != self.setup.node.0
        {
            return Err("Impact PortSetup attachment changed".into());
        }
        Ok(())
    }

    pub(super) fn check(
        &self,
        guard: &mut impl FnMut() -> Result<(), String>,
    ) -> Result<(), String> {
        if self.read(guard)? != self.outputs {
            return Err("Impact output state changed during inspection".into());
        }
        Ok(())
    }

    pub(super) fn value(&self) -> Value {
        json!({
            "scope":"impact_outputs",
            "value_semantics":"sequential_snapshot",
            "complete":true,
            "output_count":self.outputs.len(),
            "outputs":self.outputs
        })
    }
}

// Fixed offsets of the verified native types on the guarded host thread.
unsafe fn field<T: Copy>(object: *mut c_void, offset: usize) -> T {
    unsafe { ptr::read_unaligned(object.cast::<u8>().add(offset).cast::<T>()) }
}

fn existing_main_list(
    synth: *mut c_void,
    head: *mut c_void,
    expected_table: usize,
) -> Result<*mut c_void, String> {
    let mut node = head;
    let mut previous = ptr::null_mut();
    let mut main = None;
    for _ in 0..MAX_OUTPUT_GROUPS {
        if node.is_null() {
            break;
        }
        if unsafe { field::<*mut c_void>(node, 16) } != previous {
            return Err("Impact output list links changed".into());
        }
        let list: *mut c_void = unsafe { field(node, 0) };
        if list.is_null()
            || unsafe { field::<usize>(list, 0) } != expected_table
            || unsafe { field::<*mut c_void>(list, 0x30) } != synth
            || unsafe { field::<i32>(list, 0x48) } != 1
        {
            return Err("Impact output list type, owner or direction changed".into());
        }
        if unsafe { field::<[u8; 16]>(list, 0x38) } == MAIN_OUTPUT_GROUP
            && main.replace(list).is_some()
        {
            return Err("Impact main output group is ambiguous".into());
        }
        previous = node;
        node = unsafe { field(node, 8) };
    }
    if !node.is_null() {
        return Err("Impact exceeds the supported output group count".into());
    }
    main.ok_or("Impact existing main output group is unavailable".into())
}

fn read_output(entries: *mut c_void, index: i32) -> Result<Output, String> {
    let entry: *mut c_void =
        unsafe { field(entries, index as usize * std::mem::size_of::<*mut c_void>()) };
    if entry.is_null() {
        return Err("Impact output slot is empty".into());
    }
    let string: PlainString = unsafe { field(entry, 0x40) };
    let name = copy_string(string.object, 128)?;
    if name.trim().is_empty() || name.chars().any(char::is_control) {
        return Err("Impact output name is malformed".into());
    }
    Ok(Output {
        index,
        name,
        requested: unsafe { field::<u8>(entry, 0x70) } & 1 != 0,
        acknowledged: unsafe { field::<u8>(entry, 0x74) } & 1 != 0,
    })
}

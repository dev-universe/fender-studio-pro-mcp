//! Exact-build Pedalboard slot exchange through the host editor interface.
use super::*;

const AMP_IMAGE: [u8; 16] = [
    0xe8, 0x3b, 0x81, 0x67, 0xcf, 0x19, 0x3b, 0x5b, 0xa5, 0xc6, 0xe7, 0xeb, 0xd7, 0x43, 0xab, 0x31,
];
const SUB_ITEMS: usize = 10;
const AMP_SUB_ITEMS: usize = 0x20330;
const ITEM_MODEL_TABLE: usize = 0xd07360;
const MODEL_OFFSET: usize = 0x178;
const MODEL_COMPONENT_OFFSET: usize = 0xa0;
const SLOT_EDITOR: Uid = Uid {
    a: 0x7f0cf11d,
    b: 0xee10,
    c: 0x4bf4,
    d: [0x92, 0x20, 0xe9, 0x89, 0xfb, 0x29, 0x65, 0x8f],
};

fn select_model<G: FnMut() -> Result<(), String>>(
    binding: &ControllerBinding,
    text: TextApi,
    guard: &mut G,
) -> Result<Owned, String> {
    binding.check(guard)?;
    let root = interface(binding.controller.0, &NODE, guard)?;
    let key = unsafe { (text.get_constant_string)(c"stomps".as_ptr()) };
    if key.is_null() {
        return Err("Pedalboard stomp alias name unavailable".into());
    }
    binding.check(guard)?;
    let stomps = pin(catalog::find(root.0, key.cast()), guard)
        .map_err(|reason| format!("Find Pedalboard stomp alias: {reason}"))?;
    let alias = interface(stomps.0, &ALIAS, guard)
        .map_err(|reason| format!("Query Pedalboard stomp alias: {reason}"))?;
    binding.check(guard)?;
    let delegate = pin(borrowed(alias.0, 3), guard)
        .map_err(|reason| format!("Resolve Pedalboard stomp model delegate: {reason}"))?;
    let model = interface(delegate.0, &catalog::ITEM_MODEL, guard)?;
    let delegate_identity = interface(delegate.0, &UNKNOWN, guard)?;
    let model_identity = interface(model.0, &UNKNOWN, guard)?;
    if delegate_identity.0 != model_identity.0 {
        return Err("Pedalboard item model canonical identity differs".into());
    }
    release(
        (
            model_identity,
            delegate_identity,
            delegate,
            alias,
            stomps,
            root,
        ),
        guard,
    )?;
    Ok(model)
}

fn location(entry: *const c_void) -> Result<DlInfo, String> {
    let mut info = DlInfo {
        file: ptr::null(),
        base: ptr::null_mut(),
        symbol: ptr::null(),
        address: ptr::null_mut(),
    };
    if entry.is_null() || unsafe { dladdr(entry, &mut info) } == 0 {
        return Err("Pedalboard method has no mapped executable image".into());
    }
    Ok(info)
}

fn model_image(model: &Owned) -> Result<usize, String> {
    let entry: *const c_void = unsafe { method(model.0, SUB_ITEMS) };
    let info = location(entry)?;
    let image = info.base as usize;
    let table = read_word(model.0 as usize, 0);
    if image_uuid(info.base) != Some(AMP_IMAGE)
        || (entry as usize).checked_sub(image) != Some(AMP_SUB_ITEMS)
        || table != image.checked_add(ITEM_MODEL_TABLE)
        || read_word(
            table
                .ok_or("Pedalboard item table unavailable")?
                .saturating_sub(16),
            0,
        ) != Some(0usize.wrapping_sub(MODEL_OFFSET))
    {
        return Err("Pedalboard item model layout or image differs".into());
    }
    Ok(image)
}

fn resolve_slot_editor<G: FnMut() -> Result<(), String>>(
    model: &Owned,
    guard: &mut G,
) -> Result<Editor, String> {
    guard()?;
    let _image = model_image(model)?;
    let base = (model.0 as usize)
        .checked_sub(MODEL_OFFSET)
        .ok_or("Pedalboard item model base unavailable")?;
    let component = read_word(base, MODEL_COMPONENT_OFFSET)
        .filter(|value| *value != 0)
        .ok_or("Pedalboard model component reference unavailable")?
        as *mut c_void;
    let component_table =
        read_word(component as usize, 0).ok_or("Pedalboard model component table unavailable")?;
    location(component_table as *const c_void)?;
    let alias = interface(component, &ALIAS, guard)?;
    let delegate = pin(borrowed(alias.0, 3), guard)?;
    let editor = interface(delegate.0, &SLOT_EDITOR, guard)
        .map_err(|reason| format!("Pedalboard slot-edit interface: {reason}"))?;
    let canonical = interface(editor.0, &UNKNOWN, guard)?;
    let expected = interface(delegate.0, &UNKNOWN, guard)?;
    if canonical.0 != expected.0 {
        return Err("Pedalboard slot-edit provider belongs to a different object".into());
    }
    release((canonical, expected), guard)?;
    let result = Editor {
        base,
        component,
        alias,
        delegate,
        editor,
    };
    result.check(guard)?;
    Ok(result)
}

struct Editor {
    base: usize,
    component: *mut c_void,
    alias: Owned,
    delegate: Owned,
    editor: Owned,
}

impl Editor {
    fn check<G: FnMut() -> Result<(), String>>(&self, guard: &mut G) -> Result<(), String> {
        guard()?;
        if read_word(self.base, MODEL_COMPONENT_OFFSET) != Some(self.component as usize)
            || borrowed(self.alias.0, 3) != self.delegate.0
        {
            return Err("Pedalboard model component changed".into());
        }
        guard()
    }

    fn method(&self) -> Result<(*const c_void, [u8; 16], usize), String> {
        let address: *const c_void = unsafe { method(self.editor.0, 8) };
        let info = location(address)?;
        let uuid = image_uuid(info.base).ok_or("Pedalboard slot editor image has no UUID")?;
        let offset = (address as usize)
            .checked_sub(info.base as usize)
            .ok_or("Pedalboard slot editor's method lies outside its image")?;
        Ok((address, uuid, offset))
    }
}

const DSP_SHELL: [u8; 16] = [
    0x23, 0x02, 0xb0, 0xe7, 0xed, 0xf3, 0x3a, 0x00, 0xa5, 0xb6, 0x48, 0x1a, 0x8b, 0x85, 0x44, 0x7b,
];
const VACANT: &str = " - ";

pub(in crate::native_script::request) fn swap_slots(
    ctx: &Context<'_>,
    slots: [u8; 2],
    names: [&str; 2],
    expected_rack_sha256: Option<&str>,
) -> Value {
    let mut dispatched = false;
    let result = (|| {
        if ctx.instance.object_id != "Pedalboard"
            || ctx.instance.native_class != "AudioEffect"
            || ctx.instance.class_uid != "BC9129B4-EC67-41B8-96DE-EC128D5FE54D"
        {
            return Err("The exact Pedalboard AudioEffect is required".to_owned());
        }
        let [first, second] = slots;
        let vacant = names.map(|name| name == VACANT);
        if !(1..=8).contains(&first)
            || !(1..=8).contains(&second)
            || first == second
            || vacant == [true, true]
            || names.iter().any(|name| {
                name.is_empty() || name.len() > 128 || name.chars().any(char::is_control)
            })
            || match expected_rack_sha256 {
                None => {
                    first.abs_diff(second) != 1 || names[0] == names[1] || vacant.contains(&true)
                }
                Some(fingerprint) => {
                    fingerprint.len() != 64
                        || !fingerprint
                            .bytes()
                            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                }
            }
        {
            return Err("Supply two distinct Pedalboard positions, exact model labels, and the required rack fingerprint".into());
        }
        let mut guard = || ctx.check();
        let dsp = direct_controller(ctx)?;
        let alias = AliasBinding::capture(ctx.target.0, ctx.node.0, &mut guard)?;
        let controller = pin(borrowed(alias.provider.0, 3), &mut guard)?;
        let identity = interface(controller.0, &UNKNOWN, &mut guard)?;
        let binding = ControllerBinding {
            identity,
            controller,
            alias,
        };
        let constant = constant_string().ok_or("Host text API unavailable")?;
        let text = TextApi {
            get_constant_string: unsafe {
                std::mem::transmute::<
                    ConstantString,
                    unsafe extern "C" fn(*const c_char) -> *const c_void,
                >(constant)
            },
        };
        let model = select_model(&binding, text, &mut guard)?;
        let mut checked = || binding.check(&mut guard);
        let editor = resolve_slot_editor(&model, &mut checked)?;
        if editor.method().map(|(_, uuid, offset)| (uuid, offset))? != (DSP_SHELL, 0x35960) {
            return Err("Pedalboard slot editor method or image differs".into());
        }
        let observe = |slot: u8| -> Result<(String, u64), String> {
            ctx.check()?;
            let row = dsp
                .find(&format!("opt.stompmodel{slot}"), ctx.api)?
                .describe()
                .map_err(|e| e.to_string())?;
            ctx.check()?;
            let Some(parameters::Value::Float(value)) = row.raw.map(|raw| raw.value) else {
                return Err("Pedalboard model is unavailable".into());
            };
            if row.tag != 40961 + i32::from(slot)
                || !value.is_finite()
                || value < 0.0
                || (value == 0.0) != (row.display == VACANT)
            {
                return Err("Pedalboard slot model and vacancy could not be established".into());
            }
            Ok((row.display, value.to_bits()))
        };
        let before = [observe(first)?, observe(second)?];
        if [before[0].0.as_str(), before[1].0.as_str()] != names
            || expected_rack_sha256.is_none() && before[0].1 == before[1].1
            || before != [observe(first)?, observe(second)?]
        {
            return Err("Pedalboard models changed or do not match the expected names".into());
        }
        let left = CString::new(format!("stomp{first}")).expect("numeric pedal slot");
        let right = CString::new(format!("stomp{second}")).expect("numeric pedal slot");
        let left = PlainCString {
            text: left.as_ptr(),
            owner: ptr::null_mut(),
        };
        let right = PlainCString {
            text: right.as_ptr(),
            owner: ptr::null_mut(),
        };
        let swap: unsafe extern "C" fn(
            *mut c_void,
            *const PlainCString,
            *const PlainCString,
        ) -> i32 = unsafe { method(editor.editor.0, 8) };
        let exchange = expected_rack_sha256
            .map(|fingerprint| state::PedalboardExchange::prepare(ctx, slots, vacant, fingerprint))
            .transpose()?;
        editor.check(&mut checked)?;
        dispatched = true;
        if unsafe { swap(editor.editor.0, &left, &right) } != OK {
            return Err("The host slot editor returned an error".into());
        }
        editor.check(&mut checked)?;
        let after = [observe(first)?, observe(second)?];
        if after[0] != before[1] || after[1] != before[0] {
            return Err("The current Pedalboard models do not match the requested exchange".into());
        }
        if let Some(exchange) = exchange {
            exchange.verify(ctx)?;
        }
        release((editor, model), &mut checked)?;
        release((binding, dsp), &mut guard)?;
        let mut receipt = json!({"ok":true, "operation":"pedalboard_slot_swap", "outcome":"applied_unverified",
                  "verified":false, "effects_verified":false, "slots":slots,
                  "before":names, "after":[names[1],names[0]]});
        if expected_rack_sha256.is_some() {
            receipt["serialized_payload_verified"] = json!(true);
        }
        Ok(receipt)
    })();
    result.unwrap_or_else(|reason| {
        error(
            "pedalboard_swap_failed",
            &reason,
            if dispatched { "unknown" } else { "not_applied" },
        )
    })
}

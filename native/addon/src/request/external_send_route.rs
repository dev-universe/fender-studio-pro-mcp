use super::*;

const WRAPPER_VTABLE: usize = 0x19b38c0;
const COMPONENT_VTABLE: usize = 0x19b2d78;
const HOLDER_VTABLE: usize = 0x19b3dd0;

/// Recheck both links to the shared routing owner before reading its Send slot.
fn snapshot(wrapper: usize, image: usize) -> Option<(usize, usize, usize)> {
    if read_word(wrapper, 0)? != image + WRAPPER_VTABLE {
        return None;
    }
    let component = read_word(wrapper, 0x150)?;
    let holder = read_word(wrapper, 0x158)?;
    if read_word(component, 0)? != image + COMPONENT_VTABLE
        || read_word(holder, 0)? != image + HOLDER_VTABLE
        || read_word(component, 0x248)? != holder
    {
        return None;
    }
    Some((component, holder, read_word(holder, 0x60)?))
}

pub(super) fn assigned(ctx: &Context<'_>, alias: *mut c_void) -> Option<bool> {
    if ctx.instance.class_uid != "889DD9F8-14F0-44A5-BCEE-0F2B157FC5D4"
        || ctx.instance.native_class != "AudioEffect"
        || ctx.instance.object_id != "External Effect Stereo"
    {
        return None;
    }
    ctx.check().ok()?;
    let image = unsafe { _dyld_get_image_header(0) };
    if image_uuid(image) != Some(EXPECTED_APP_UUID) {
        return None;
    }
    let canonical = query(alias, &UNKNOWN)?;
    let wrapper = canonical.0 as usize;
    let before = snapshot(wrapper, image as usize)?;
    ctx.check().ok()?;
    let after = snapshot(wrapper, image as usize)?;
    drop(canonical);
    ctx.check().ok()?;
    // Observe the slot's assignment without borrowing the referenced bus.
    (before == after).then_some(after.2 != 0)
}

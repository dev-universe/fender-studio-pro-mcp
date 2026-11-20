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

fn exact_stereo(ctx: &Context<'_>) -> bool {
    ctx.instance.class_uid == "889DD9F8-14F0-44A5-BCEE-0F2B157FC5D4"
        && ctx.instance.native_class == "AudioEffect"
        && ctx.instance.object_id == "External Effect Stereo"
}

pub(super) struct HeldRoute {
    canonical: Owned,
    image: usize,
    original: (usize, usize, usize),
}

impl HeldRoute {
    fn capture(ctx: &Context<'_>, alias: *mut c_void) -> Option<Self> {
        if !exact_stereo(ctx) {
            return None;
        }
        ctx.check().ok()?;
        let image = unsafe { _dyld_get_image_header(0) };
        if image_uuid(image) != Some(EXPECTED_APP_UUID) {
            return None;
        }
        let canonical = query(alias, &UNKNOWN)?;
        let original = snapshot(canonical.0 as usize, image as usize)?;
        Some(Self {
            canonical,
            image: image as usize,
            original,
        })
    }

    pub(super) fn from_context(ctx: &Context<'_>) -> Option<Self> {
        if !exact_stereo(ctx) {
            return None;
        }
        ctx.check().ok()?;
        let alias = query(ctx.target.0, &ALIAS)?;
        let get: Borrowed = unsafe { method(alias.0, 3) };
        let plugin = hold(unsafe { get(alias.0) })?;
        let route = Self::capture(ctx, plugin.0)?;
        drop(plugin);
        drop(alias);
        route.check(ctx).ok()?;
        Some(route)
    }

    pub(super) fn assigned(&self) -> bool {
        // Observe the slot's assignment without borrowing the referenced bus.
        self.original.2 != 0
    }

    pub(super) fn check(&self, ctx: &Context<'_>) -> Result<(), String> {
        ctx.check()?;
        if snapshot(self.canonical.0 as usize, self.image) != Some(self.original) {
            return Err("The retained External Effect Stereo Send route changed".into());
        }
        Ok(())
    }
}

pub(super) fn assigned(ctx: &Context<'_>, alias: *mut c_void) -> Option<bool> {
    let route = HeldRoute::capture(ctx, alias)?;
    route.check(ctx).ok()?;
    let assigned = route.assigned();
    drop(route);
    ctx.check().ok()?;
    Some(assigned)
}

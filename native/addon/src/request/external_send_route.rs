use super::*;

const WRAPPER_VTABLE: usize = 0x19b38c0;
const COMPONENT_VTABLE: usize = 0x19b2d78;
const HOLDER_VTABLE: usize = 0x19b3dd0;
const RECEIVE_BUS_VTABLE: usize = 0x1b71ee8;
const RECEIVE_CONNECTION_AT: usize = 0x0d9cdec;

fn routing_owner(wrapper: usize, image: usize) -> Option<(usize, usize)> {
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
    Some((component, holder))
}

/// Recheck both links to the shared routing owner before reading its Send slot.
fn snapshot(wrapper: usize, image: usize) -> Option<(usize, usize, usize)> {
    let (component, holder) = routing_owner(wrapper, image)?;
    Some((component, holder, read_word(holder, 0x60)?))
}

#[derive(PartialEq, Eq)]
struct ReceiveConnection {
    component: usize,
    holder: usize,
    receive_bus: usize,
    storage: usize,
    count: u32,
    first: usize,
}

fn receive_connection(wrapper: usize, image: usize) -> Option<ReceiveConnection> {
    let (component, holder) = routing_owner(wrapper, image)?;
    let receive_bus = read_word(holder, 0x58)?;
    let table = read_word(receive_bus, 0)?;
    if table != image + RECEIVE_BUS_VTABLE
        || read_word(table, 0x148)? != image + RECEIVE_CONNECTION_AT
    {
        return None;
    }
    // The app reads the first entry only when its signed count is positive.
    let count = read_word(receive_bus, 0xd8)? as u32;
    let storage = read_word(receive_bus, 0xd0)?;
    if count > 1 || storage == 0 {
        return None;
    }
    let first = if count == 1 {
        read_word(storage, 0)?
    } else {
        0
    };
    if count == 1 && first == 0 {
        return None;
    }
    Some(ReceiveConnection {
        component,
        holder,
        receive_bus,
        storage,
        count,
        first,
    })
}

pub(super) fn exact_stereo(ctx: &Context<'_>) -> bool {
    if ctx.instance.class_uid != "889DD9F8-14F0-44A5-BCEE-0F2B157FC5D4"
        || ctx.instance.native_class != "AudioEffect"
    {
        return false;
    }
    let object_id = ctx.instance.object_id.as_str();
    object_id == "External Effect Stereo"
        || object_id
            .strip_prefix("External Effect Stereo ")
            .is_some_and(|suffix| {
                !suffix.starts_with('0')
                    && suffix.bytes().all(|byte| byte.is_ascii_digit())
                    && suffix.parse::<u16>().is_ok_and(|ordinal| ordinal >= 2)
            })
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

pub(super) fn return_assigned(ctx: &Context<'_>, alias: *mut c_void) -> Option<bool> {
    if !exact_stereo(ctx) {
        return None;
    }
    ctx.check().ok()?;
    let image = unsafe { _dyld_get_image_header(0) };
    if image_uuid(image) != Some(EXPECTED_APP_UUID) {
        return None;
    }
    let canonical = query(alias, &UNKNOWN)?;
    let wrapper = canonical.0 as usize;
    let image_address = image as usize;
    let before = receive_connection(wrapper, image_address)?;
    ctx.check().ok()?;
    if receive_connection(wrapper, image_address)? != before {
        return None;
    }
    drop(canonical);
    ctx.check().ok()?;
    Some(before.first != 0)
}

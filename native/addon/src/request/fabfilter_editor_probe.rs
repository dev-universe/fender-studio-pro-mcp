use super::*;

// Identifies Studio Pro 8.1.1 arm64's VST3 host image.
const HOST_UUID: [u8; 16] = [
    0x20, 0xd9, 0xad, 0xcb, 0xd6, 0x23, 0x32, 0xa7, 0x91, 0x56, 0xa1, 0x1c, 0xb7, 0x05, 0xff, 0x75,
];
const PRO_Q4: Image = Image {
    uuid: [
        0xc9, 0x95, 0x75, 0x92, 0x31, 0xc6, 0x3c, 0x10, 0x82, 0xc6, 0xba, 0xc4, 0x7f, 0xae, 0xa4,
        0xa8,
    ],
    controller_table: 0x222eb0,
    state_member: 0xd0,
    state_table: 0x224740,
    state_file_method: 0x988e8,
    view_table: 0x2276f0,
    editor_table: 0x223a50,
    product: "pro_q_4",
};
const TWIN3: Image = Image {
    uuid: [
        0xbd, 0x81, 0xb8, 0x7c, 0xa2, 0x84, 0x3c, 0x20, 0x85, 0xaf, 0xd6, 0x32, 0xf4, 0xbb, 0x5c,
        0x86,
    ],
    controller_table: 0x28fe08,
    state_member: 0xd8,
    state_table: 0x281cf8,
    state_file_method: 0xd6f5c,
    view_table: 0x29a778,
    editor_table: 0x2813b8,
    product: "twin_3",
};

struct Image {
    uuid: [u8; 16],
    controller_table: usize,
    state_member: usize,
    state_table: usize,
    state_file_method: usize,
    view_table: usize,
    editor_table: usize,
    product: &'static str,
}

#[link(name = "System")]
unsafe extern "C" {
    static mach_task_self_: u32;
    fn mach_vm_read_overwrite(
        task: u32,
        address: u64,
        size: u64,
        data: u64,
        count: *mut u64,
    ) -> i32;
}

// All addresses come from retained host objects or version-checked image metadata.
fn read_word(base: usize, offset: usize) -> Option<usize> {
    let address = base.checked_add(offset)?;
    if base == 0 || address % 8 != 0 {
        return None;
    }
    let mut data = 0usize;
    let mut count = 0;
    let status = unsafe {
        mach_vm_read_overwrite(
            mach_task_self_,
            address as u64,
            8,
            &mut data as *mut usize as u64,
            &mut count,
        )
    };
    (status == 0 && count == 8).then_some(data)
}

fn mapped_image(table: usize) -> Option<(usize, [u8; 16])> {
    let mut info = DlInfo {
        file: ptr::null(),
        base: ptr::null_mut(),
        symbol: ptr::null(),
        address: ptr::null_mut(),
    };
    if table == 0 || unsafe { dladdr(table as *const c_void, &mut info) } == 0 {
        return None;
    }
    Some((info.base as usize, image_uuid(info.base)?))
}

fn count_attached(
    host: usize,
    instance: usize,
    controller: usize,
    product: &Image,
    image: usize,
    state: usize,
) -> Option<(usize, usize)> {
    // Studio Pro uses this same list to find an editor from its effect object.
    let mut node = read_word(host, 0x16b120)?;
    let mut matched = 0;
    let mut other = 0;
    let mut previous = 0;
    for _ in 0..64 {
        if node == 0 {
            return Some((matched, other));
        }
        if read_word(node, 16)? != previous {
            return None;
        }
        let editor_host = read_word(node, 0)?;
        if read_word(editor_host, 0)? != host + 0x1353e8 {
            return None;
        }
        let effect = read_word(editor_host, 0x50)?;
        let view = read_word(editor_host, 0x60)?;
        if read_word(view, 0)? == image + product.view_table {
            let primary = view.checked_sub(0x10)?;
            let product_editor = read_word(primary, 0x70)?;
            if product_editor != 0 && effect == instance {
                if read_word(primary, 0x58)? != controller
                    || read_word(product_editor, 0)? != image + product.editor_table
                    || read_word(product_editor, 0x50)? != state
                {
                    return None;
                }
                matched += 1;
            } else if product_editor != 0 {
                if read_word(primary, 0x58)? == controller
                    || read_word(product_editor, 0x50)? == state
                {
                    return None;
                }
                other += 1;
            }
        } else if effect == instance && view != 0 {
            return None;
        }
        previous = node;
        node = read_word(node, 8)?;
    }
    None
}

pub(super) fn observe(target: *mut c_void) -> Option<Value> {
    let canonical = query(target, &UNKNOWN)?;
    let instance = canonical.0 as usize;
    let table = read_word(instance, 0)?;
    let (host, host_uuid) = mapped_image(table)?;
    if host_uuid != HOST_UUID || table != host + 0x1358a8 {
        return None;
    }
    let storable = query(canonical.0, &STORABLE)?;
    if storable.0 as usize != instance + 0xb8 {
        return None;
    }
    let controller = read_word(instance, 0x1b0)?;
    let table = read_word(controller, 0)?;
    let (image, uuid) = mapped_image(table)?;
    let product = [&PRO_Q4, &TWIN3]
        .into_iter()
        .find(|product| product.uuid == uuid)?;
    if table != image + product.controller_table {
        return None;
    }
    // The public controller interface lies inside the FabFilter controller object.
    let controller_primary = controller.checked_sub(0x30)?;
    let state = read_word(controller_primary, product.state_member)?;
    let state_table = read_word(state, 0)?;
    if state_table != image + product.state_table
        || read_word(state_table, 0x220)? != image + product.state_file_method
    {
        return None;
    }
    let first = count_attached(host, instance, controller_primary, product, image, state)?;
    let second = count_attached(host, instance, controller_primary, product, image, state)?;
    if first != second || first.0 > 1 || read_word(instance, 0x1b0)? != controller {
        return None;
    }
    Some(
        json!({"product":product.product,"effect_and_controller_match":true,
        "matching_attached_editor_count":first.0,"other_same_product_attached_editors":first.1,
        "checked_twice":true,"file_loader_calls":0}),
    )
}

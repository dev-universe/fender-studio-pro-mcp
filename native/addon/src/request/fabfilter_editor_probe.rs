use super::*;
use sha2::{Digest, Sha256};
use std::{
    ffi::CStr,
    fs,
    io::{Read, Write},
    os::unix::{
        ffi::OsStrExt,
        fs::{DirBuilderExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
};

#[path = "vendor_preset_abi.rs"]
mod vendor_preset_abi;

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
    executable: "FabFilter Pro-Q 4",
    binary_sha256: "1044282fd91649368283553e3a2f3905d7eb051bff40d52888a0907aa53bdb65",
    binary_size: 5_025_104,
    constructor: 0x39a18,
    loader: 0x9a4d0,
    destructor: 0x4418c,
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
    executable: "FabFilter Twin 3",
    binary_sha256: "d6ebeb10274eeafef08e2d7c8aba6d920c99ed5ff68c617907213e1ba091f371",
    binary_size: 5_999_776,
    constructor: 0x107cf4,
    loader: 0x96e4c,
    destructor: 0xa5058,
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
    executable: &'static str,
    binary_sha256: &'static str,
    binary_size: u64,
    constructor: usize,
    loader: usize,
    destructor: usize,
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
) -> Option<(usize, usize, usize)> {
    // Studio Pro uses this same list to find an editor from its effect object.
    let mut node = read_word(host, 0x16b120)?;
    let mut matched = 0;
    let mut editor = 0;
    let mut other = 0;
    let mut previous = 0;
    for _ in 0..64 {
        if node == 0 {
            return Some((matched, other, editor));
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
                editor = product_editor;
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

struct Owner {
    canonical: Owned,
    _storable: Owned,
    host: usize,
    controller: usize,
    image: usize,
    state: usize,
    product: &'static Image,
    first: (usize, usize, usize),
}
impl Owner {
    fn check(&self) -> bool {
        let instance = self.canonical.0 as usize;
        let controller = self.controller;
        read_word(instance, 0x1b0) == Some(controller + 0x30)
            && read_word(controller, self.product.state_member) == Some(self.state)
            && count_attached(
                self.host,
                instance,
                controller,
                self.product,
                self.image,
                self.state,
            ) == Some(self.first)
    }
}

fn owner(target: *mut c_void) -> Option<Owner> {
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
    Some(Owner {
        canonical,
        _storable: storable,
        host,
        controller: controller_primary,
        image,
        state,
        product,
        first,
    })
}

pub(super) fn observe(target: *mut c_void) -> Option<Value> {
    let owner = owner(target)?;
    Some(
        json!({"product":owner.product.product,"effect_and_controller_match":true,
        "matching_attached_editor_count":owner.first.0,"other_same_product_attached_editors":owner.first.1,
        "checked_twice":true,"file_loader_calls":0}),
    )
}

fn image_file(base: usize) -> Option<PathBuf> {
    let mut info = DlInfo {
        file: ptr::null(),
        base: ptr::null_mut(),
        symbol: ptr::null(),
        address: ptr::null_mut(),
    };
    if unsafe { dladdr(base as *const c_void, &mut info) } == 0
        || info.base as usize != base
        || info.file.is_null()
    {
        return None;
    }
    let file = unsafe { CStr::from_ptr(info.file) };
    Some(PathBuf::from(std::ffi::OsStr::from_bytes(file.to_bytes())))
}

fn signed_image_matches(binding: &Owner) -> bool {
    let Some(binary) = image_file(binding.image) else {
        return false;
    };
    if binary.file_name().and_then(|name| name.to_str()) != Some(binding.product.executable) {
        return false;
    }
    let Ok(file) = fs::File::open(binary) else {
        return false;
    };
    if !file
        .metadata()
        .is_ok_and(|metadata| metadata.is_file() && metadata.len() == binding.product.binary_size)
    {
        return false;
    }
    let mut contents = Vec::with_capacity(binding.product.binary_size as usize);
    file.take(binding.product.binary_size + 1)
        .read_to_end(&mut contents)
        .is_ok()
        && contents.len() == binding.product.binary_size as usize
        && format!("{:x}", Sha256::digest(contents)) == binding.product.binary_sha256
}

const MAX_PRESET_BYTES: usize = 64 * 1024;

fn preset_bytes(
    product: &Image,
    filename: &str,
    expected: &str,
    data_hex: &str,
) -> Option<Vec<u8>> {
    if filename.is_empty()
        || filename.len() > 255
        || filename.starts_with('.')
        || filename.chars().any(char::is_control)
        || filename.contains('/')
        || !Path::new(filename)
            .extension()
            .and_then(|name| name.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("ffp"))
        || expected.len() != 64
        || data_hex.is_empty()
        || data_hex.len() % 2 != 0
        || data_hex.len() > 2 * MAX_PRESET_BYTES
    {
        return None;
    }
    fn digit(byte: u8) -> Option<u8> {
        match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + 10),
            _ => None,
        }
    }
    let bytes: Vec<u8> = data_hex
        .as_bytes()
        .chunks_exact(2)
        .map(|chunk| Some(digit(chunk[0])? * 16 + digit(chunk[1])?))
        .collect::<Option<_>>()?;
    let correct_format = if product.product == "twin_3" {
        bytes.len() >= 16 && bytes.starts_with(b"FT3i")
    } else {
        let mut header = bytes
            .split(|byte| *byte == b'\n')
            .map(|line| line.strip_suffix(b"\r").unwrap_or(line));
        header.next() == Some(b"[Preset]".as_slice())
            && header.next() == Some(b"Signature=FQ4p".as_slice())
            && header
                .next()
                .is_some_and(|line| line.starts_with(b"Version="))
            && header.next().is_some()
    };
    if !correct_format || format!("{:x}", Sha256::digest(&bytes)) != expected {
        return None;
    }
    Some(bytes)
}

#[link(name = "System")]
unsafe extern "C" {
    fn arc4random_buf(buffer: *mut c_void, bytes: usize);
}

struct PresetFile {
    directory: PathBuf,
    path: PathBuf,
}
impl PresetFile {
    fn new(filename: &str, bytes: &[u8]) -> std::io::Result<Self> {
        let mut nonce = [0u8; 16];
        unsafe {
            arc4random_buf(nonce.as_mut_ptr().cast(), nonce.len());
        }
        let directory =
            std::env::temp_dir().join(format!("fsp-fabfilter-{:032x}", u128::from_be_bytes(nonce)));
        fs::DirBuilder::new().mode(0o700).create(&directory)?;
        let path = directory.join(filename);
        let staged = Self { directory, path };
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&staged.path)?;
        file.write_all(bytes)?;
        Ok(staged)
    }
}
impl Drop for PresetFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
        let _ = fs::remove_dir(&self.directory);
    }
}

pub(super) fn apply_preset(
    ctx: &Context<'_>,
    target: *mut c_void,
    filename: &str,
    sha256: &str,
    data_hex: &str,
) -> Value {
    let refused = |reason: &str| error("fabfilter_preset_apply_failed", reason, "not_applied");
    let Some(owner) = owner(target) else {
        return refused("the exact effect and controller are unavailable");
    };
    if owner.first.0 != 1 {
        return refused("Open the editor of the expected FabFilter instance and try again");
    }
    if !signed_image_matches(&owner) {
        return refused(
            "the installed FabFilter executable differs from the verified signed version",
        );
    }
    let Some(bytes) = preset_bytes(owner.product, filename, sha256, data_hex) else {
        return refused("the bounded FabFilter preset bytes or their digest did not match");
    };
    let Ok(file) = PresetFile::new(filename, &bytes) else {
        return refused("the preset could not be staged for the plug-in");
    };
    if ctx.check().is_err() || !owner.check() {
        return refused("the document or exact attached editor changed before apply");
    }
    let args = vendor_preset_abi::Apply {
        receiver: owner.state as *mut c_void,
        constructor: (owner.image + owner.product.constructor) as *const c_void,
        activate: (owner.image + owner.product.loader) as *const c_void,
        destructor: (owner.image + owner.product.destructor) as *const c_void,
    };
    let status = unsafe { vendor_preset_abi::apply(args, file.path.as_os_str().as_bytes()) };
    match status {
        Err(reason) => refused(reason),
        Ok(0) if ctx.check().is_ok() && owner.check() => {
            json!({"ok":true,"outcome":"applied_unverified",
            "file_sha256":sha256,"filename":filename,"status":0,"source":"fabfilter_vst3_preset_file"})
        }
        Ok(_) => error(
            "fabfilter_preset_apply_failed",
            "the preset effect or active target could not be confirmed",
            "unknown",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staged_preset_requires_a_digest_and_a_single_filename() {
        let bytes = b"[Preset]\nSignature=FQ4p\nVersion=4\nAuthor=Fixture\n";
        let digest = format!("{:x}", Sha256::digest(bytes));
        let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        assert_eq!(
            preset_bytes(&PRO_Q4, "Air & Space.ffp", &digest, &hex).unwrap(),
            bytes
        );
        assert!(preset_bytes(&PRO_Q4, "../Air.ffp", &digest, &hex).is_none());
        assert!(preset_bytes(&PRO_Q4, ".Air.ffp", &digest, &hex).is_none());
        assert!(preset_bytes(&PRO_Q4, "Air.vstpreset", &digest, &hex).is_none());
        assert!(preset_bytes(&PRO_Q4, "Air.ffp", &"0".repeat(64), &hex).is_none());
        assert!(preset_bytes(&PRO_Q4, "Air.ffp", &digest, &(hex.clone() + "0")).is_none());
        assert!(
            preset_bytes(
                &PRO_Q4,
                "Air.ffp",
                &digest,
                &"00".repeat(MAX_PRESET_BYTES + 1)
            )
            .is_none()
        );
    }

    #[test]
    fn twin_preset_bytes_are_limited_to_the_twin_profile() {
        let bytes = b"FT3i012345678901";
        let digest = format!("{:x}", Sha256::digest(bytes));
        let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        assert_eq!(
            preset_bytes(&TWIN3, "Twin.ffp", &digest, &hex).unwrap(),
            bytes
        );
        assert!(preset_bytes(&PRO_Q4, "Twin.ffp", &digest, &hex).is_none());
    }
}

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
const MICRO: Image = Image {
    class_uid: "751AFC34-1543-4CBC-AA73-672A109BE0FF",
    uuid: [
        0x9b, 0x6e, 0xf3, 0x6f, 0xdd, 0x13, 0x3c, 0x54, 0x95, 0x1c, 0x70, 0x00, 0x8f, 0x55, 0x5b,
        0x8c,
    ],
    controller_table: 0x101a00,
    state_member: 0xd0,
    state_table: 0x102690,
    state_file_method: 0x6e7e0,
    view_table: 0x103848,
    editor_table: 0x1022b0,
    product: "micro",
    executable: "FabFilter Micro",
    binary_sha256: "6034d2fc31144fe223bad86d8608a3b711f9cbb42a25e89eec9f6b7c7b331636",
    binary_size: 2_417_168,
    preset_format: PresetFormat::Binary(b"FMic\x01\0\0\0"),
    constructor: 0x1c2e0,
    loader: 0x702e8,
    destructor: 0x25098,
};

const PRO_C2: Image = Image {
    class_uid: "79F415E3-C8E7-4807-AD5D-A3CF7024F618",
    uuid: [
        0xba, 0x3a, 0x9c, 0x3c, 0x82, 0x20, 0x37, 0x49, 0x8a, 0xc5, 0x91, 0x5e, 0x04, 0xed, 0x66,
        0xa6,
    ],
    controller_table: 0x1730e0,
    state_member: 0xd0,
    state_table: 0x178c30,
    state_file_method: 0x12c7c8,
    view_table: 0x174768,
    editor_table: 0x16fd18,
    product: "pro_c_2",
    executable: "FabFilter Pro-C 2",
    binary_sha256: "18fb9b61c25e8058cc9ae64167c219e34c8a224b7e824f73faf2c34c2f3d619a",
    binary_size: 3_440_576,
    preset_format: PresetFormat::BinaryHeaders(&[*b"FC2p\x02\0\0\0", *b"FPrC\x01\0\0\0"]),
    constructor: 0x9f918,
    loader: 0x61bc4,
    destructor: 0x6f0ec,
};

const PRO_DS: Image = Image {
    class_uid: "59E324D0-8EE8-11E1-B857-8101BBE59B18",
    uuid: [
        0x7d, 0x93, 0x3c, 0x0a, 0xa7, 0xd6, 0x36, 0xdc, 0x91, 0xff, 0x41, 0x6a, 0xec, 0x52, 0x03,
        0x92,
    ],
    controller_table: 0x175d80,
    state_member: 0xd0,
    state_table: 0x179148,
    state_file_method: 0x65984,
    view_table: 0x176f98,
    editor_table: 0x178c88,
    product: "pro_ds",
    executable: "FabFilter Pro-DS",
    binary_sha256: "474ffc06b8fd6ddd536639e78d79cbb6b510f448cad7ad98c5721d9c365ad4c3",
    binary_size: 3_474_528,
    preset_format: PresetFormat::Binary(b"FPSr\x02\0\0\0"),
    constructor: 0x1b7a8,
    loader: 0x67514,
    destructor: 0x2601c,
};

const PRO_G: Image = Image {
    class_uid: "9D1DCCF7-918A-436F-A3CD-D7115D8B193B",
    uuid: [
        0x92, 0x95, 0x43, 0xe9, 0xc8, 0xe5, 0x39, 0x7d, 0xb8, 0xc2, 0x0e, 0x0b, 0xdc, 0xe7, 0x0f,
        0x02,
    ],
    controller_table: 0x1b9e28,
    state_member: 0xd0,
    state_table: 0x1c4288,
    state_file_method: 0x77574,
    view_table: 0x1bee38,
    editor_table: 0x1c3dd8,
    product: "pro_g",
    executable: "FabFilter Pro-G",
    binary_sha256: "b1c1e1939700ab729bdb46011c889ff3c313373d561b6d82f1b5f80475ad8579",
    binary_size: 4_118_752,
    preset_format: PresetFormat::Binary(b"FPGr\x02\0\0\0"),
    constructor: 0x284e4,
    loader: 0x79104,
    destructor: 0x32d58,
};

const PRO_L: Image = Image {
    class_uid: "5AF2703A-E65F-11DE-B804-7C9C56D89593",
    uuid: [
        0x21, 0xb4, 0x12, 0x3c, 0x66, 0x5f, 0x3f, 0x78, 0xb9, 0x97, 0x83, 0x8f, 0x46, 0x5c, 0x33,
        0x75,
    ],
    controller_table: 0x1708c0,
    state_member: 0xd0,
    state_table: 0x169af8,
    state_file_method: 0x90d40,
    view_table: 0x175dc8,
    editor_table: 0x16eec8,
    product: "pro_l",
    executable: "FabFilter Pro-L",
    binary_sha256: "f8511b64b3533d61533f7055adba10f0bc66da2249e8ec4aebc1d18877e82c8d",
    binary_size: 3_441_776,
    preset_format: PresetFormat::Binary(b"FPLr\x03\0\0\0"),
    constructor: 0x1e870,
    loader: 0x68520,
    destructor: 0x27628,
};

const PRO_L2: Image = Image {
    class_uid: "AFD92F72-9A04-47B7-B5E8-D1D568DEA985",
    uuid: [
        0xec, 0x32, 0x5c, 0x96, 0x05, 0x4e, 0x38, 0x19, 0xbc, 0x25, 0x46, 0x58, 0xdd, 0xe0, 0xcb,
        0x47,
    ],
    controller_table: 0x18e650,
    state_member: 0xd0,
    state_table: 0x187708,
    state_file_method: 0x76b9c,
    view_table: 0x193d20,
    editor_table: 0x18caa0,
    product: "pro_l_2",
    executable: "FabFilter Pro-L 2",
    binary_sha256: "82d8e6bcaccf38a7286908d9f92e8858c51c2e33dd438d74aaa52209745d9199",
    binary_size: 3_705_904,
    preset_format: PresetFormat::Binary(b"FL2p\x02\0\0\0"),
    constructor: 0x1fb30,
    loader: 0x7872c,
    destructor: 0x28a20,
};

const PRO_MB: Image = Image {
    class_uid: "C3B68142-C798-46F2-82B7-3CABDF139076",
    uuid: [
        0x5e, 0x42, 0xba, 0x87, 0xd5, 0x2c, 0x3a, 0xc9, 0x9f, 0x99, 0x2b, 0x36, 0x80, 0x64, 0x2e,
        0x00,
    ],
    controller_table: 0x162d30,
    state_member: 0xd0,
    state_table: 0x168ae8,
    state_file_method: 0x66b5c,
    view_table: 0x164af0,
    editor_table: 0x168638,
    product: "pro_mb",
    executable: "FabFilter Pro-MB",
    binary_sha256: "a029c442f945873a0a751f58f4eb186157b3ccd9dcb91d70b0f788d489b52f1f",
    binary_size: 3_309_088,
    preset_format: PresetFormat::Binary(b"FPMb\x03\0\0\0"),
    constructor: 0x1c82c,
    loader: 0x68734,
    destructor: 0x255e4,
};

const PRO_Q2: Image = Image {
    class_uid: "55FD08E6-C00B-44A6-97DA-68F61C6FD576",
    uuid: [
        0x65, 0x27, 0xd9, 0x9c, 0x5f, 0x35, 0x3f, 0xa3, 0x9a, 0x04, 0x64, 0x94, 0x1b, 0xdc, 0x5c,
        0x51,
    ],
    controller_table: 0x16ef78,
    state_member: 0xd0,
    state_table: 0x170320,
    state_file_method: 0xdba30,
    view_table: 0x172838,
    editor_table: 0x16fd18,
    product: "pro_q_2",
    executable: "FabFilter Pro-Q 2",
    binary_sha256: "86aa746b73d5a703ae12d18c74251268ec25a3f8c15c15b81458da504c64de10",
    binary_size: 3_407_696,
    preset_format: PresetFormat::Binary(b"FQ2p\x02\0\0\0"),
    constructor: 0x1dfa0,
    loader: 0x6d754,
    destructor: 0x26ecc,
};

const PRO_Q3: Image = Image {
    class_uid: "72C4DB71-7A4D-459A-B97E-51745D84B39D",
    uuid: [
        0xa8, 0x46, 0xe0, 0x98, 0xe4, 0x9a, 0x3d, 0x4a, 0xb1, 0x7c, 0x6c, 0x98, 0x62, 0x45, 0xf2,
        0xc8,
    ],
    controller_table: 0x194388,
    state_member: 0xd0,
    state_table: 0x1958f8,
    state_file_method: 0xf231c,
    view_table: 0x1980e8,
    editor_table: 0x195000,
    product: "pro_q_3",
    executable: "FabFilter Pro-Q 3",
    binary_sha256: "b29cbe786625089de086b915c5ea1a5ee5986fdcedbd1a8ca14ac611e631f0e9",
    binary_size: 3_721_088,
    preset_format: PresetFormat::Binary(b"FQ3p\x04\0\0\0"),
    constructor: 0x1e0e8,
    loader: 0x75234,
    destructor: 0x27014,
};

const PRO_Q4: Image = Image {
    class_uid: "ED57BD72-5C60-467E-A64D-D2F400758B6F",
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
    preset_format: PresetFormat::Text(b"Signature=FQ4p"),
    constructor: 0x39a18,
    loader: 0x9a4d0,
    destructor: 0x4418c,
};
const TWIN3: Image = Image {
    class_uid: "91AF218E-111A-44CB-ABFE-F7CB065FF157",
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
    preset_format: PresetFormat::Binary(b"FT3i"),
    constructor: 0x107cf4,
    loader: 0x96e4c,
    destructor: 0xa5058,
};

const PRO_C3: Image = Image {
    class_uid: "B0A765D6-4EFE-42FA-8C7E-FA87BF5DA0A9",
    uuid: [
        0x97, 0x32, 0x3f, 0x1e, 0xa0, 0x3e, 0x3e, 0xca, 0xaf, 0xf8, 0x76, 0x31, 0x12, 0x5f, 0x65,
        0x6a,
    ],
    controller_table: 0x1f45e0,
    state_member: 0xd0,
    state_table: 0x1fd4e8,
    state_file_method: 0x6e620,
    view_table: 0x1f6a90,
    editor_table: 0x1ef990,
    product: "pro_c_3",
    executable: "FabFilter Pro-C 3",
    binary_sha256: "5b193d2016a240f0fe7cb10f8db92c3bc41cb28316cbcad5b85856efc22f3492",
    binary_size: 4_612_864,
    preset_format: PresetFormat::Text(b"Signature=FC3p"),
    constructor: 0xb5044,
    loader: 0x70208,
    destructor: 0x6a4c0,
};

const PRO_R_BINARY_HEADER: &[u8] = b"FPRr\x04\0\0\0";

const PRO_R: Image = Image {
    class_uid: "036EB530-2A59-11E2-A281-70076288709B",
    uuid: [
        0x2d, 0x52, 0x4f, 0x19, 0xa7, 0xa0, 0x3d, 0xce, 0x93, 0xf6, 0x1d, 0x17, 0xd8, 0x1b, 0xfa,
        0x44,
    ],
    controller_table: 0x15a398,
    state_member: 0xd0,
    state_table: 0x152378,
    state_file_method: 0x53030,
    view_table: 0x15af48,
    editor_table: 0x15d268,
    product: "pro_r",
    executable: "FabFilter Pro-R",
    binary_sha256: "f3eb810af2ed22ed9ec8877ae665ed659d0305c084e632812395026a54b214a6",
    binary_size: 3_226_192,
    preset_format: PresetFormat::Binary(PRO_R_BINARY_HEADER),
    constructor: 0x26d50,
    loader: 0x6cd80,
    destructor: 0x2fb44,
};

const PRO_R2: Image = Image {
    class_uid: "6070873C-802A-4B07-8FC0-6AB5459154E9",
    uuid: [
        0x3e, 0xb0, 0x6c, 0x9c, 0x10, 0xc4, 0x39, 0x22, 0x88, 0x81, 0x1a, 0x6a, 0x63, 0xa6, 0x5d,
        0x46,
    ],
    controller_table: 0x1e2f98,
    state_member: 0xd0,
    state_table: 0x1d7ba8,
    state_file_method: 0x956ac,
    view_table: 0x1e5f10,
    editor_table: 0x1e9710,
    product: "pro_r_2",
    executable: "FabFilter Pro-R 2",
    binary_sha256: "3d152617f492a85f8cb5dea592ca87e83850909ad327f1d0e46e0ae82a11cab0",
    binary_size: 4_447_632,
    preset_format: PresetFormat::ProR2,
    constructor: 0x3dc84,
    loader: 0x97294,
    destructor: 0x48630,
};

const SIMPLON: Image = Image {
    class_uid: "3317F470-55E8-47E4-896B-59C21B843FAE",
    uuid: [
        0x97, 0xee, 0x58, 0x8d, 0x3a, 0xdb, 0x33, 0xb4, 0x91, 0x10, 0x63, 0x0a, 0x7c, 0x4e, 0xa2,
        0x47,
    ],
    controller_table: 0x1086a8,
    state_member: 0xd0,
    state_table: 0x107408,
    state_file_method: 0x57a58,
    view_table: 0x10b250,
    editor_table: 0x1068d8,
    product: "simplon",
    executable: "FabFilter Simplon",
    binary_sha256: "d5f6fa163eb3ee78431bf61db9e3ed19b5054e361fd54d54e0dcf3da88458bca",
    binary_size: 2_499_792,
    preset_format: PresetFormat::Binary(b"FSim\x01\0\0\0"),
    constructor: 0x86e48,
    loader: 0x59560,
    destructor: 0x66858,
};

const SATURN: Image = Image {
    class_uid: "5460882A-C2AA-4861-88CC-F15B4ACC25C6",
    uuid: [
        0x3c, 0x1b, 0xe7, 0x2d, 0x77, 0x18, 0x39, 0x7f, 0x84, 0xb2, 0x79, 0x6d, 0xfe, 0xfb, 0xbc,
        0x95,
    ],
    controller_table: 0x18f298,
    state_member: 0xd0,
    state_table: 0x1970b8,
    state_file_method: 0x6679c,
    view_table: 0x194c90,
    editor_table: 0x195080,
    product: "saturn",
    executable: "FabFilter Saturn",
    binary_sha256: "9f7fe79c9d36586ef9da9b602df0399371fdcedd528f45d476930720d4929c54",
    binary_size: 3_755_648,
    preset_format: PresetFormat::Saturn,
    constructor: 0x9fbdc,
    loader: 0x682a4,
    destructor: 0x753ec,
};

const SATURN2: Image = Image {
    class_uid: "8D067533-D8A0-491D-BAA3-6C064C6ABBFB",
    uuid: [
        0x9a, 0xad, 0xca, 0xa0, 0x8c, 0xc5, 0x3a, 0xe2, 0x99, 0xe1, 0xfd, 0x1b, 0x6e, 0x06, 0x35,
        0xfc,
    ],
    controller_table: 0x1f0110,
    state_member: 0xd0,
    state_table: 0x1f9480,
    state_file_method: 0x6eaf4,
    view_table: 0x1f7970,
    editor_table: 0x1f8540,
    product: "saturn_2",
    executable: "FabFilter Saturn 2",
    binary_sha256: "0f1e34b7a09a5db3e4df6236ba830fc0463825011e498b0dc3095d28ef9b0cc7",
    binary_size: 4_597_872,
    preset_format: PresetFormat::Saturn2,
    constructor: 0xd1660,
    loader: 0x70644,
    destructor: 0x7db58,
};

enum PresetFormat {
    Text(&'static [u8]),
    Binary(&'static [u8]),
    BinaryHeaders(&'static [[u8; 8]]),
    ProR2,
    Saturn,
    Saturn2,
}

struct Image {
    class_uid: &'static str,
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
    preset_format: PresetFormat,
    constructor: usize,
    loader: usize,
    destructor: usize,
}

fn image_for_class(class_uid: &str) -> Option<&'static Image> {
    [
        &MICRO, &PRO_C2, &PRO_DS, &PRO_G, &PRO_L, &PRO_L2, &PRO_MB, &PRO_Q2, &PRO_Q3, &PRO_Q4,
        &TWIN3, &PRO_C3, &PRO_R, &PRO_R2, &SATURN, &SATURN2, &SIMPLON,
    ]
    .into_iter()
    .find(|product| product.class_uid == class_uid)
}

pub(super) fn supports_class(class_uid: &str) -> bool {
    image_for_class(class_uid).is_some()
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

fn owner(target: *mut c_void, class_uid: &str) -> Option<Owner> {
    let product = image_for_class(class_uid)?;
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
    if uuid != product.uuid || table != image + product.controller_table {
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

pub(super) fn observe(target: *mut c_void, class_uid: &str) -> Option<Value> {
    let owner = owner(target, class_uid)?;
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
    let matches_text = |signature: &[u8], version: Option<&[u8]>| {
        let mut header = bytes
            .split(|byte| *byte == b'\n')
            .map(|line| line.strip_suffix(b"\r").unwrap_or(line));
        header.next() == Some(b"[Preset]".as_slice())
            && header.next() == Some(signature)
            && header.next().is_some_and(|line| {
                version.map_or_else(
                    || line.starts_with(b"Version="),
                    |expected| line == expected,
                )
            })
            && header.next().is_some()
    };
    let correct_format = match product.preset_format {
        PresetFormat::Binary(magic) => bytes.len() >= 16 && bytes.starts_with(magic),
        PresetFormat::BinaryHeaders(headers) => {
            bytes.len() >= 16 && headers.iter().any(|header| bytes.starts_with(header))
        }
        PresetFormat::Text(signature) => matches_text(signature, None),
        PresetFormat::ProR2 => {
            (bytes.len() >= 16 && bytes.starts_with(PRO_R_BINARY_HEADER))
                || matches_text(b"Signature=FR2p", Some(b"Version=6"))
        }
        PresetFormat::Saturn => {
            bytes.len() >= 16
                && matches!(
                    &bytes[..8],
                    b"FSat\x01\0\0\0" | b"FSat\x02\0\0\0" | b"FSat\x04\0\0\0"
                )
        }
        PresetFormat::Saturn2 => {
            bytes.len() >= 16
                && matches!(
                    &bytes[..8],
                    b"FS2a\x02\0\0\0" | b"FS2a\x03\0\0\0" | b"FS2a\x04\0\0\0" | b"FSat\x04\0\0\0"
                )
        }
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
    let Some(owner) = owner(target, &ctx.instance.class_uid) else {
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

    #[test]
    fn fabfilter_admission_maps_each_exact_class_to_its_verified_image() {
        for (class_uid, product) in [
            ("ED57BD72-5C60-467E-A64D-D2F400758B6F", "pro_q_4"),
            ("B0A765D6-4EFE-42FA-8C7E-FA87BF5DA0A9", "pro_c_3"),
            ("036EB530-2A59-11E2-A281-70076288709B", "pro_r"),
            ("6070873C-802A-4B07-8FC0-6AB5459154E9", "pro_r_2"),
            ("8D067533-D8A0-491D-BAA3-6C064C6ABBFB", "saturn_2"),
            ("91AF218E-111A-44CB-ABFE-F7CB065FF157", "twin_3"),
        ] {
            assert!(supports_class(class_uid));
            assert_eq!(image_for_class(class_uid).unwrap().product, product);
        }
        assert!(!supports_class("6070873C-802A-4B07-8FC0-6AB5459154E8"));
    }

    #[test]
    fn pro_r_2_preset_bytes_accept_both_observed_factory_headers() {
        let cases: [(&[u8], bool); 5] = [
            (
                b"[Preset]\r\nSignature=FR2p\r\nVersion=6\r\nAuthor=Fixture\r\n",
                true,
            ),
            (b"FPRr\x04\0\0\0abcdefgh", true),
            (b"FPRr\x05\0\0\0abcdefgh", false),
            (b"FPRr\x04\0\0\0", false),
            (
                b"[Preset]\nSignature=FR2p\nVersion=5\nAuthor=Fixture",
                false,
            ),
        ];
        for (bytes, valid) in cases {
            let digest = format!("{:x}", Sha256::digest(bytes));
            let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
            assert_eq!(
                preset_bytes(&PRO_R2, "Room.ffp", &digest, &hex).is_some(),
                valid
            );
            assert!(preset_bytes(&PRO_C3, "Room.ffp", &digest, &hex).is_none());
            assert_eq!(
                preset_bytes(&PRO_R, "Room.ffp", &digest, &hex).is_some(),
                valid && bytes.starts_with(PRO_R_BINARY_HEADER),
            );
        }
    }

    #[test]
    fn pro_c_2_preset_bytes_accept_the_two_observed_factory_headers() {
        for (bytes, valid) in [
            (b"FC2p\x02\0\0\0abcdefgh", true),
            (b"FPrC\x01\0\0\0abcdefgh", true),
            (b"FC2p\x01\0\0\0abcdefgh", false),
            (b"FPrC\x02\0\0\0abcdefgh", false),
        ] {
            let digest = format!("{:x}", Sha256::digest(bytes));
            let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
            assert_eq!(
                preset_bytes(&PRO_C2, "Drums.ffp", &digest, &hex).is_some(),
                valid
            );
        }
    }

    #[test]
    fn pro_ds_preset_bytes_are_limited_to_observed_factory_format() {
        for (bytes, valid) in [
            (b"FPSr\x02\0\0\0abcdefgh", true),
            (b"FPSr\x01\0\0\0abcdefgh", false),
            (b"FL2p\x02\0\0\0abcdefgh", false),
        ] {
            let digest = format!("{:x}", Sha256::digest(bytes));
            let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
            assert_eq!(
                preset_bytes(&PRO_DS, "Vocal.ffp", &digest, &hex).is_some(),
                valid
            );
        }
    }

    #[test]
    fn pro_g_preset_bytes_are_limited_to_observed_factory_format() {
        for (bytes, valid) in [
            (b"FPGr\x02\0\0\0abcdefgh", true),
            (b"FPGr\x01\0\0\0abcdefgh", false),
            (b"FL2p\x02\0\0\0abcdefgh", false),
        ] {
            let digest = format!("{:x}", Sha256::digest(bytes));
            let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
            assert_eq!(
                preset_bytes(&PRO_G, "Vocal.ffp", &digest, &hex).is_some(),
                valid
            );
        }
    }

    #[test]
    fn pro_mb_preset_bytes_are_limited_to_observed_factory_format() {
        for (bytes, valid) in [
            (b"FPMb\x03\0\0\0abcdefgh", true),
            (b"FPMb\x02\0\0\0abcdefgh", false),
            (b"FL2p\x02\0\0\0abcdefgh", false),
        ] {
            let digest = format!("{:x}", Sha256::digest(bytes));
            let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
            assert_eq!(
                preset_bytes(&PRO_MB, "Vocal.ffp", &digest, &hex).is_some(),
                valid
            );
        }
    }

    #[test]
    fn simplon_preset_bytes_are_limited_to_observed_factory_format() {
        for (bytes, valid) in [
            (b"FSim\x01\0\0\0abcdefgh", true),
            (b"FSim\x02\0\0\0abcdefgh", false),
            (b"FL2p\x02\0\0\0abcdefgh", false),
        ] {
            let digest = format!("{:x}", Sha256::digest(bytes));
            let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
            assert_eq!(
                preset_bytes(&SIMPLON, "Vocal.ffp", &digest, &hex).is_some(),
                valid
            );
        }
    }

    #[test]
    fn pro_l_preset_bytes_are_limited_to_observed_factory_format() {
        for (bytes, valid) in [
            (b"FPLr\x03\0\0\0abcdefgh", true),
            (b"FPLr\x02\0\0\0abcdefgh", false),
            (b"FL2p\x02\0\0\0abcdefgh", false),
        ] {
            let digest = format!("{:x}", Sha256::digest(bytes));
            let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
            assert_eq!(
                preset_bytes(&PRO_L, "Vocal.ffp", &digest, &hex).is_some(),
                valid
            );
        }
    }

    #[test]
    fn pro_l_2_preset_bytes_are_limited_to_observed_factory_format() {
        for (bytes, valid) in [
            (b"FL2p\x02\0\0\0abcdefgh", true),
            (b"FL2p\x03\0\0\0abcdefgh", false),
            (b"FPLr\x02\0\0\0abcdefgh", false),
        ] {
            let digest = format!("{:x}", Sha256::digest(bytes));
            let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
            assert_eq!(
                preset_bytes(&PRO_L2, "Punchy.ffp", &digest, &hex).is_some(),
                valid
            );
        }
    }

    #[test]
    fn micro_preset_bytes_are_limited_to_observed_factory_format() {
        for (bytes, valid) in [
            (b"FMic\x01\0\0\0abcdefgh", true),
            (b"FMic\x02\0\0\0abcdefgh", false),
            (b"FSat\x01\0\0\0abcdefgh", false),
        ] {
            let digest = format!("{:x}", Sha256::digest(bytes));
            let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
            assert_eq!(
                preset_bytes(&MICRO, "Speakers.ffp", &digest, &hex).is_some(),
                valid
            );
        }
    }

    #[test]
    fn pro_q_3_preset_bytes_are_limited_to_observed_factory_format() {
        for (bytes, valid) in [
            (b"FQ3p\x04\0\0\0abcdefgh", true),
            (b"FQ3p\x03\0\0\0abcdefgh", false),
            (b"FQ2p\x04\0\0\0abcdefgh", false),
        ] {
            let digest = format!("{:x}", Sha256::digest(bytes));
            let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
            assert_eq!(
                preset_bytes(&PRO_Q3, "Flat.ffp", &digest, &hex).is_some(),
                valid
            );
        }
    }

    #[test]
    fn pro_q_2_preset_bytes_are_limited_to_observed_factory_format() {
        for (bytes, valid) in [
            (b"FQ2p\x02\0\0\0abcdefgh", true),
            (b"FQ2p\x03\0\0\0abcdefgh", false),
            (b"FQ3p\x02\0\0\0abcdefgh", false),
        ] {
            let digest = format!("{:x}", Sha256::digest(bytes));
            let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
            assert_eq!(
                preset_bytes(&PRO_Q2, "Flat.ffp", &digest, &hex).is_some(),
                valid
            );
        }
    }

    #[test]
    fn saturn_preset_bytes_admit_its_three_observed_factory_versions() {
        for (bytes, valid) in [
            (b"FSat\x01\0\0\0abcdefgh", true),
            (b"FSat\x02\0\0\0abcdefgh", true),
            (b"FSat\x04\0\0\0abcdefgh", true),
            (b"FSat\x03\0\0\0abcdefgh", false),
            (b"FS2a\x04\0\0\0abcdefgh", false),
        ] {
            let digest = format!("{:x}", Sha256::digest(bytes));
            let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
            assert_eq!(
                preset_bytes(&SATURN, "Drive.ffp", &digest, &hex).is_some(),
                valid
            );
        }
    }

    #[test]
    fn saturn_2_preset_bytes_admit_only_observed_factory_versions() {
        for (bytes, valid) in [
            (b"FS2a\x02\0\0\0abcdefgh", true),
            (b"FS2a\x03\0\0\0abcdefgh", true),
            (b"FS2a\x04\0\0\0abcdefgh", true),
            (b"FSat\x04\0\0\0abcdefgh", true),
            (b"FSat\x03\0\0\0abcdefgh", false),
            (b"FS2a\x05\0\0\0abcdefgh", false),
        ] {
            let digest = format!("{:x}", Sha256::digest(bytes));
            let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
            assert_eq!(
                preset_bytes(&SATURN2, "Drive.ffp", &digest, &hex).is_some(),
                valid
            );
            assert!(preset_bytes(&PRO_R, "Drive.ffp", &digest, &hex).is_none());
        }
    }

    #[test]
    fn pro_c_3_preset_bytes_are_limited_to_its_profile() {
        let bytes = b"[Preset]\r\nSignature=FC3p\r\nVersion=3\r\nAuthor=Fixture\r\n";
        let digest = format!("{:x}", Sha256::digest(bytes));
        let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        assert_eq!(
            preset_bytes(&PRO_C3, "Punch.ffp", &digest, &hex).unwrap(),
            bytes
        );
        assert!(preset_bytes(&PRO_Q4, "Punch.ffp", &digest, &hex).is_none());
    }
}

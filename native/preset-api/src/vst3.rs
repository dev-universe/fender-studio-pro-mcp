//! Applies one exact Serum host carrier through the CCL 5.1.2 preset APIs.
use crate::{
    Error, Recall, RecallError, attributes, ffi,
    folder::{
        HANDLER, REGISTRY,
        collection::{PrivateDestination, validate_private_path, verify_preset_url},
        query, same_interface,
    },
    guard, list, mediator_target,
};
use ffi::{CStringRepr, Owned};
use sha2::{Digest, Sha256};
use std::{
    ffi::c_void,
    fs::{self, File, Metadata, OpenOptions},
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
    ptr::{self, NonNull},
};

const MAX_FILE_BYTES: usize = 16 * 1024 * 1024;
const HEADER_BYTES: usize = 48;
const TABLE_BYTES: usize = 28;
const EXTENSION: &str = ".vstpreset";

#[derive(Clone, Copy)]
pub enum SerumVariant {
    Instrument,
    Fx,
}

impl SerumVariant {
    pub fn class_uid(self) -> &'static str {
        match self {
            Self::Instrument => "56534558-6673-5073-6572-756D20320000",
            Self::Fx => "56534558-6673-5173-6572-756D20322066",
        }
    }

    fn carrier_class(self) -> &'static [u8; 32] {
        match self {
            Self::Instrument => b"56534558667350736572756D20320000",
            Self::Fx => b"56534558667351736572756D20322066",
        }
    }

    fn matches_class(self, class: &str) -> bool {
        let key = class
            .strip_prefix('{')
            .and_then(|key| key.strip_suffix('}'))
            .unwrap_or(class);
        key.eq_ignore_ascii_case(self.class_uid())
            || key.as_bytes().eq_ignore_ascii_case(self.carrier_class())
    }
}

/// Each pointer transfers one owned reference to the named CCL interface.
pub struct Inputs {
    pub registry: NonNull<c_void>,
    pub mediator: NonNull<c_void>,
    pub data_target: NonNull<c_void>,
    pub allocator: NonNull<c_void>,
    pub empty_url: NonNull<c_void>,
}

/// Applies a staged carrier to the already bound Device slot.
///
/// # Safety
/// All interfaces must be live CCL 5.1.2 pointers on the validated GUI thread,
/// with one transferred reference each. The mediator and IDataTarget must
/// belong to the exact live Serum instance and slot. The guard must prove the
/// current document, instance and slot/Presets attachments around callbacks and
/// releases. The server owns the absolute path and keeps the staged file until
/// completion. All native calls, callbacks and releases stay on this GUI thread.
pub unsafe fn apply_owned<G: FnMut() -> Result<(), String>>(
    inputs: Inputs,
    path: &str,
    expected_sha256: &str,
    variant: SerumVariant,
    g: &mut G,
) -> Result<Recall, RecallError> {
    let mut dispatched = false;
    let result = (|| {
        let registry = unsafe { Owned::take(inputs.registry) };
        let mediator = unsafe { Owned::take(inputs.mediator) };
        let data_target = unsafe { Owned::take(inputs.data_target) };
        let allocator = unsafe { Owned::take(inputs.allocator) };
        let empty_url = unsafe { Owned::take(inputs.empty_url) };
        guard(g)?;
        validate_private_path(path, EXTENSION)?;
        let staged = StagedCarrier::open(Path::new(path), expected_sha256, variant)?;
        let url = PrivateDestination::from_empty_url(empty_url, path, EXTENSION, g)?;
        let registry = same_interface(&registry, &REGISTRY, g)?;

        let target = unsafe { Owned::hold(mediator_target(&mediator)) }?;
        guard(g)?;
        check_target_class(&mediator, &allocator, variant, g)?;
        let handler = vst3_handler(&registry, &url, g)?;
        let flags: unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32 =
            unsafe { ffi::method(handler.raw(), 11) };
        let can_import = unsafe { flags(handler.raw(), target.raw()) } & 1 != 0;
        guard(g)?;
        if !can_import {
            return Err(Error::Unavailable("VST3 preset handler refused the target"));
        }

        url.check(g)?;
        staged.check()?;
        let open: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> *mut c_void =
            unsafe { ffi::method(handler.raw(), 9) };
        let pointer = unsafe { open(handler.raw(), url.raw(), ptr::null_mut()) };
        // Unlike borrowed registry lookups, openPreset transfers its result.
        let opened = NonNull::new(pointer)
            .map(|pointer| unsafe { Owned::take(pointer) })
            .ok_or(Error::Unavailable(
                "VST3 preset handler could not open the carrier",
            ));
        guard(g)?;
        let opened = opened?;
        let preset = same_interface(&opened, &ffi::PRESET, g)?;
        drop(opened);
        check_preset(&preset, &url, variant, g)?;

        let list = list::one(&preset);
        check_target_class(&mediator, &allocator, variant, g)?;
        if mediator_target(&mediator) != target.raw() {
            return Err(Error::Changed);
        }
        guard(g)?;
        let can: unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            *mut c_void,
            *mut c_void,
            i32,
        ) -> u8 = unsafe { ffi::method(data_target.raw(), 3) };
        let allowed = unsafe {
            can(
                data_target.raw(),
                list.raw(),
                ptr::null_mut(),
                ptr::null_mut(),
                -1,
            )
        };
        guard(g)?;
        if allowed != 1 {
            return Err(Error::Unavailable(
                "VST3 preset data target refused the carrier",
            ));
        }

        // The capability query may call back into the host. Recheck the
        // carrier, preset and fixed target before the single mutation.
        check_preset(&preset, &url, variant, g)?;
        check_target_class(&mediator, &allocator, variant, g)?;
        url.check(g)?;
        let current_handler = vst3_handler(&registry, &url, g)?;
        if current_handler.raw() != handler.raw() {
            return Err(Error::Changed);
        }
        drop(current_handler);
        if mediator_target(&mediator) != target.raw() {
            return Err(Error::Changed);
        }
        guard(g)?;
        staged.check()?;
        let insert: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, i32) -> u8 =
            unsafe { ffi::method(data_target.raw(), 4) };
        dispatched = true;
        let accepted = unsafe { insert(data_target.raw(), list.raw(), ptr::null_mut(), -1) };
        guard(g)?;
        if accepted != 1 {
            return Err(Error::Unavailable(
                "VST3 preset dispatch did not confirm acceptance",
            ));
        }
        if mediator_target(&mediator) != target.raw() {
            return Err(Error::Changed);
        }
        staged.check()?;
        Ok(Recall {
            accepted: true,
            verified: false,
        })
    })();
    // The URL, handler, list, opened preset and other native handles have all
    // released. Their cleanup may have changed the active document or slot.
    guard(g)
        .and(result)
        .map_err(|error| RecallError { error, dispatched })
}

fn vst3_handler<G: FnMut() -> Result<(), String>>(
    registry: &Owned,
    url: &PrivateDestination,
    g: &mut G,
) -> Result<Owned, Error> {
    let mime = CStringRepr {
        text: c"application/x-steinberg-vstpreset".as_ptr(),
        owner: ptr::null_mut(),
    };
    let by_mime: unsafe extern "C" fn(*mut c_void, *const CStringRepr) -> *mut c_void =
        unsafe { ffi::method(registry.raw(), 9) };
    let by_file: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
        unsafe { ffi::method(registry.raw(), 7) };
    let mime_handler = unsafe { Owned::hold(by_mime(registry.raw(), &mime)) }?;
    guard(g)?;
    let file_handler = unsafe { Owned::hold(by_file(registry.raw(), url.raw())) }?;
    guard(g)?;
    let handler = same_interface(&file_handler, &HANDLER, g)?;
    let same_handler =
        query(&mime_handler, &ffi::UNKNOWN, g)?.raw() == query(&handler, &ffi::UNKNOWN, g)?.raw();
    drop(file_handler);
    drop(mime_handler);
    guard(g)?;
    if same_handler {
        Ok(handler)
    } else {
        Err(Error::Changed)
    }
}

fn check_target_class<G: FnMut() -> Result<(), String>>(
    mediator: &Owned,
    allocator: &Owned,
    variant: SerumVariant,
    g: &mut G,
) -> Result<(), Error> {
    let attributes = attributes(allocator)?;
    guard(g)?;
    let get: unsafe extern "C" fn(*mut c_void, *mut c_void) -> u8 =
        unsafe { ffi::method(mediator.raw(), 3) };
    let available = unsafe { get(mediator.raw(), attributes.raw()) };
    guard(g)?;
    if available != 1 {
        return Err(Error::Unavailable(
            "Serum target preset metadata unavailable",
        ));
    }
    check_class(attributes, variant, g)
}

fn check_preset<G: FnMut() -> Result<(), String>>(
    preset: &Owned,
    url: &PrivateDestination,
    variant: SerumVariant,
    g: &mut G,
) -> Result<(), Error> {
    let get: unsafe extern "C" fn(*mut c_void) -> *mut c_void =
        unsafe { ffi::method(preset.raw(), 6) };
    let metadata = unsafe { Owned::hold(get(preset.raw())) }?;
    guard(g)?;
    check_class(metadata, variant, g)?;
    verify_preset_url(preset, url, g)
}

fn check_class<G: FnMut() -> Result<(), String>>(
    attributes: Owned,
    variant: SerumVariant,
    g: &mut G,
) -> Result<(), Error> {
    let key = ffi::class_key(&attributes);
    drop(attributes);
    guard(g)?;
    if variant.matches_class(&key?) {
        Ok(())
    } else {
        Err(Error::InvalidData(
            "preset metadata class does not match the Serum target",
        ))
    }
}

struct StagedCarrier<'a> {
    path: &'a Path,
    file: File,
    metadata: Metadata,
}

impl<'a> StagedCarrier<'a> {
    fn open(path: &'a Path, digest: &str, variant: SerumVariant) -> Result<Self, Error> {
        if !path.is_absolute()
            || !valid_staged_name(path)
            || digest.len() != 64
            || !lower_hex(digest)
        {
            return Err(Error::InvalidData("invalid staged carrier identity"));
        }
        let metadata = fs::symlink_metadata(path).map_err(file_unavailable)?;
        if !metadata.is_file()
            || metadata.mode() & 0o7777 != 0o600
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.nlink() != 1
            || metadata.len() > MAX_FILE_BYTES as u64
        {
            return Err(Error::InvalidData("invalid staged carrier file"));
        }
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(path)
            .map_err(file_unavailable)?;
        let staged = Self {
            path,
            file,
            metadata,
        };
        staged.check()?;
        let mut bytes = Vec::new();
        (&staged.file)
            .take(MAX_FILE_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(file_unavailable)?;
        staged.check()?;
        if bytes.len() as u64 != staged.metadata.len() || !valid_carrier(&bytes, variant) {
            return Err(Error::InvalidData("invalid Serum VST3 processor carrier"));
        }
        if format!("{:x}", Sha256::digest(&bytes)) != digest {
            return Err(Error::InvalidData("staged carrier SHA-256 mismatch"));
        }
        Ok(staged)
    }

    fn check(&self) -> Result<(), Error> {
        let parent = self.path.parent().ok_or(Error::Changed)?;
        let directory = fs::symlink_metadata(parent).map_err(file_unavailable)?;
        if !directory.is_dir()
            || directory.uid() != self.metadata.uid()
            || directory.mode() & 0o7777 != 0o700
        {
            return Err(Error::InvalidData("staged carrier parent is not private"));
        }
        for metadata in [
            self.file.metadata().map_err(file_unavailable)?,
            fs::symlink_metadata(self.path).map_err(file_unavailable)?,
        ] {
            let original = &self.metadata;
            if !metadata.is_file()
                || metadata.dev() != original.dev()
                || metadata.ino() != original.ino()
                || metadata.mode() != original.mode()
                || metadata.uid() != original.uid()
                || metadata.nlink() != original.nlink()
                || metadata.len() != original.len()
                || metadata.mtime() != original.mtime()
                || metadata.mtime_nsec() != original.mtime_nsec()
                || metadata.ctime() != original.ctime()
                || metadata.ctime_nsec() != original.ctime_nsec()
            {
                return Err(Error::Changed);
            }
        }
        Ok(())
    }
}

fn file_unavailable(_: std::io::Error) -> Error {
    Error::Unavailable("staged carrier file unavailable")
}

fn valid_staged_name(path: &Path) -> bool {
    let Some(id) = path
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_prefix("mcp-serum2-preset-"))
        .and_then(|name| name.strip_suffix(EXTENSION))
    else {
        return false;
    };
    let Ok([a, b, c, d, e]): Result<[&str; 5], _> = id.split('-').collect::<Vec<_>>().try_into()
    else {
        return false;
    };
    [a, b, c, d, e].map(str::len) == [8, 4, 4, 4, 12]
        && [a, b, c, d, e].into_iter().all(lower_hex)
        && c.starts_with('4')
        && d.starts_with(['8', '9', 'a', 'b'])
}

fn lower_hex(value: &str) -> bool {
    value
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn valid_carrier(bytes: &[u8], variant: SerumVariant) -> bool {
    let Some(table_offset) = bytes.len().checked_sub(TABLE_BYTES) else {
        return false;
    };
    if !(HEADER_BYTES + 9 < table_offset && bytes.len() <= MAX_FILE_BYTES) {
        return false;
    }
    let table = &bytes[table_offset..];
    &bytes[..8] == b"VST3\x01\0\0\0"
        && &bytes[8..40] == variant.carrier_class()
        && bytes[40..48] == (table_offset as u64).to_le_bytes()
        && bytes[HEADER_BYTES..table_offset].starts_with(b"XferJson\0")
        && &table[..12] == b"List\x01\0\0\0Comp"
        && table[12..20] == (HEADER_BYTES as u64).to_le_bytes()
        && table[20..] == ((table_offset - HEADER_BYTES) as u64).to_le_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, os::unix::fs::DirBuilderExt, path::PathBuf};

    fn carrier(variant: SerumVariant) -> Vec<u8> {
        let comp = b"XferJson\0placeholder";
        let mut bytes = b"VST3\x01\0\0\0".to_vec();
        bytes.extend_from_slice(variant.carrier_class());
        bytes.extend_from_slice(&((HEADER_BYTES + comp.len()) as u64).to_le_bytes());
        bytes.extend_from_slice(comp);
        bytes.extend_from_slice(b"List\x01\0\0\0Comp");
        bytes.extend_from_slice(&(HEADER_BYTES as u64).to_le_bytes());
        bytes.extend_from_slice(&(comp.len() as u64).to_le_bytes());
        bytes
    }

    #[test]
    fn admits_only_the_exact_processor_carrier_for_each_class() {
        let instrument = SerumVariant::Instrument;
        let effect = SerumVariant::Fx;
        assert!(valid_carrier(&carrier(instrument), instrument));
        assert!(valid_carrier(&carrier(effect), effect));
        assert!(!valid_carrier(&carrier(instrument), effect));
        for offset in [4, 40, 48, carrier(instrument).len() - 8] {
            let mut changed = carrier(instrument);
            changed[offset] ^= 1;
            assert!(!valid_carrier(&changed, instrument));
        }
        assert!(instrument.matches_class("{56534558-6673-5073-6572-756d20320000}"));
        assert!(!instrument.matches_class(effect.class_uid()));
    }

    #[test]
    fn staged_file_requires_the_digest_and_keeps_the_original_inode() {
        struct Scratch(PathBuf);
        impl Drop for Scratch {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
        let directory =
            std::env::temp_dir().join(format!("fender-serum-carrier-test-{}", std::process::id()));
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&directory)
            .unwrap();
        let scratch = Scratch(directory);
        let path = scratch
            .0
            .join("mcp-serum2-preset-00000000-0000-4000-a000-000000000000.vstpreset");
        let bytes = carrier(SerumVariant::Instrument);
        let digest = format!("{:x}", Sha256::digest(&bytes));
        let write_carrier = || {
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&path)
                .unwrap()
                .write_all(&bytes)
                .unwrap();
        };
        write_carrier();
        assert!(StagedCarrier::open(&path, &"0".repeat(64), SerumVariant::Instrument).is_err());
        let staged = StagedCarrier::open(&path, &digest, SerumVariant::Instrument).unwrap();
        fs::rename(&path, scratch.0.join("original")).unwrap();
        write_carrier();
        assert_eq!(staged.check(), Err(Error::Changed));
    }
}

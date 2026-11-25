use std::{
    fs::OpenOptions,
    io::Read,
    path::{Path, PathBuf},
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::model::OperationError;

const MAX_FILE_BYTES: usize = 16 * 1024 * 1024;
const MAX_METADATA_BYTES: usize = 64 * 1024;
const HEADER_BYTES: usize = 48;
const CHUNK_TABLE_BYTES: usize = 28;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Variant {
    Instrument,
    Fx,
}

impl Variant {
    pub(crate) fn plugin_name(self) -> &'static str {
        match self {
            Self::Instrument => "Serum 2",
            Self::Fx => "Serum 2 FX",
        }
    }

    pub(crate) fn class_uid(self) -> &'static str {
        match self {
            Self::Instrument => "56534558-6673-5073-6572-756D20320000",
            Self::Fx => "56534558-6673-5173-6572-756D20322066",
        }
    }

    fn state_product(self) -> &'static str {
        match self {
            Self::Instrument => "Serum2",
            Self::Fx => "Serum2FX",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PresetApply {
    pub target: PresetTarget,
    #[schemars(
        length(equal = 36),
        regex(pattern = "^[0-9A-F]{8}(-[0-9A-F]{4}){3}-[0-9A-F]{12}$")
    )]
    pub expected_instance: String,
    /// Absolute local path to a processor-only VST3 preset.
    pub path: PathBuf,
    /// SHA-256 of the intended file, encoded as 64 lowercase hex digits.
    #[schemars(length(equal = 64), regex(pattern = "^[0-9a-f]{64}$"))]
    pub expected_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "variant", rename_all = "snake_case", deny_unknown_fields)]
pub enum PresetTarget {
    Instrument {
        #[schemars(length(min = 1, max = 256))]
        track: String,
    },
    Fx {
        #[schemars(length(min = 1, max = 256))]
        channel: String,
        #[schemars(range(max = 15))]
        slot: u32,
    },
}

impl PresetTarget {
    pub(crate) fn variant(&self) -> Variant {
        match self {
            Self::Instrument { .. } => Variant::Instrument,
            Self::Fx { .. } => Variant::Fx,
        }
    }
}

fn valid_destination(value: &str) -> bool {
    !value.trim().is_empty()
        && value.encode_utf16().count() <= 256
        && !value.chars().any(char::is_control)
}

fn valid_digest(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn valid_path(path: &Path) -> bool {
    path.is_absolute()
        && path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("vstpreset"))
}

impl PresetApply {
    pub(crate) fn validate(&self) -> Result<(), OperationError> {
        let target_valid = match &self.target {
            PresetTarget::Instrument { track } => valid_destination(track),
            PresetTarget::Fx { channel, slot } => valid_destination(channel) && *slot < 16,
        };
        if !target_valid
            || self.expected_instance == "00000000-0000-0000-0000-000000000000"
            || self
                .expected_instance
                .bytes()
                .any(|b| b.is_ascii_lowercase())
            || !valid_path(&self.path)
            || !valid_digest(&self.expected_sha256, 64)
        {
            return Err(OperationError::new(
                "invalid_argument",
                "Supply an exact Serum target, a nonzero uppercase instance UUID, an absolute .vstpreset path, and its lowercase SHA-256",
            ));
        }
        Ok(())
    }

    pub(crate) fn read_preset(&self) -> Result<Vec<u8>, OperationError> {
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
        }
        let unavailable = || {
            OperationError::new(
                "serum_preset_unavailable",
                "Cannot open the local Serum preset as a regular file",
            )
        };
        let input = options.open(&self.path).map_err(|_| unavailable())?;
        let metadata = input.metadata().map_err(|_| unavailable())?;
        if !metadata.is_file() {
            return Err(unavailable());
        }
        let too_large = || {
            OperationError::new(
                "serum_preset_too_large",
                "A Serum VST3 preset cannot exceed 16 MiB",
            )
        };
        if metadata.len() > MAX_FILE_BYTES as u64 {
            return Err(too_large());
        }
        let mut bytes = Vec::new();
        input
            .take(MAX_FILE_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| unavailable())?;
        if bytes.len() > MAX_FILE_BYTES {
            return Err(too_large());
        }
        if format!("{:x}", Sha256::digest(&bytes)) != self.expected_sha256 {
            return Err(OperationError::new(
                "serum_preset_digest_changed",
                "The Serum preset SHA-256 differs from the requested digest",
            ));
        }
        if !valid_carrier(&bytes, self.target.variant()) {
            return Err(OperationError::new(
                "invalid_serum_preset",
                "Expected a version 1 VST3 preset with the exact target class and one Serum 2.1.5 processor state chunk",
            ));
        }
        Ok(bytes)
    }
}

fn valid_carrier(bytes: &[u8], variant: Variant) -> bool {
    if bytes.len() <= HEADER_BYTES + CHUNK_TABLE_BYTES
        || bytes.len() > MAX_FILE_BYTES
        || &bytes[..8] != b"VST3\x01\0\0\0"
        || !bytes[8..40]
            .iter()
            .copied()
            .eq(variant.class_uid().bytes().filter(|&value| value != b'-'))
    {
        return false;
    }
    // The documented VST3 chunk table must describe the complete range
    // between the header and the table as one processor component chunk.
    let table_offset = bytes.len() - CHUNK_TABLE_BYTES;
    let table = &bytes[table_offset..];
    if bytes[40..48] != (table_offset as u64).to_le_bytes()
        || &table[..12] != b"List\x01\0\0\0Comp"
        || table[12..20] != (HEADER_BYTES as u64).to_le_bytes()
        || table[20..28] != ((table_offset - HEADER_BYTES) as u64).to_le_bytes()
    {
        return false;
    }
    valid_processor(&bytes[HEADER_BYTES..table_offset], variant)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StateMetadata<'a> {
    component: &'a str,
    product: &'a str,
    product_version: &'a str,
    version: f64,
    hash: &'a str,
}

fn valid_processor(bytes: &[u8], variant: Variant) -> bool {
    let Some(data) = bytes.strip_prefix(b"XferJson\0") else {
        return false;
    };
    let Some((length, data)) = data.split_first_chunk::<8>() else {
        return false;
    };
    let Ok(length) = usize::try_from(u64::from_le_bytes(*length)) else {
        return false;
    };
    if !(1..=MAX_METADATA_BYTES).contains(&length) {
        return false;
    }
    let Some((metadata, payload)) = data.split_at_checked(length) else {
        return false;
    };
    let Ok(metadata) = serde_json::from_slice::<StateMetadata<'_>>(metadata) else {
        return false;
    };
    let Some((decoded, payload)) = payload.split_first_chunk::<4>() else {
        return false;
    };
    let decoded = u32::from_le_bytes(*decoded);
    metadata.component == "processor"
        && metadata.product == variant.state_product()
        && metadata.product_version == "2.1.5"
        && metadata.version == 11.0
        && valid_digest(metadata.hash, 32)
        && (1..=MAX_FILE_BYTES as u32).contains(&decoded)
        && valid_frame(payload, decoded)
}

fn valid_frame(bytes: &[u8], decoded: u32) -> bool {
    let Some((&descriptor, bytes)) = bytes
        .strip_prefix(b"\x02\0\0\0\x28\xb5\x2f\xfd")
        .and_then(|frame| frame.split_first())
    else {
        return false;
    };
    // Bound the Zstandard window and require the embedded content size from
    // the supported encoder. Serum's decoder remains responsible for the data.
    // https://github.com/meta/zstd/blob/dev/doc/zstd_compression_format.md#frame_header
    if descriptor & 0x1b != 0 {
        return false;
    }
    let single_segment = descriptor & 0x20 != 0;
    let width = 1 << (descriptor >> 6);
    let bytes = if single_segment {
        bytes
    } else {
        let Some((&window, bytes)) = bytes.split_first() else {
            return false;
        };
        let base = 1_u64 << (10 + (window >> 3));
        let window_size = base + (base / 8) * u64::from(window & 7);
        if width == 1 || window_size > MAX_FILE_BYTES as u64 {
            return false;
        }
        bytes
    };
    let Some((size, blocks)) = bytes.split_at_checked(width) else {
        return false;
    };
    let mut little_endian = [0; 8];
    little_endian[..width].copy_from_slice(size);
    let frame_size = u64::from_le_bytes(little_endian) + if width == 2 { 256 } else { 0 };
    let checksum_bytes = if descriptor & 4 != 0 { 4 } else { 0 };
    frame_size == u64::from(decoded) && blocks.len() >= 4 + checksum_bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn carrier(variant: Variant, component: &str, product: &str, product_version: &str) -> Vec<u8> {
        let metadata = serde_json::to_vec(&serde_json::json!({
            "component": component,
            "product": product,
            "productVersion": product_version,
            "version": 11.0,
            "hash": "0".repeat(32),
        }))
        .unwrap();
        let mut state = b"XferJson\0".to_vec();
        state.extend((metadata.len() as u64).to_le_bytes());
        state.extend(metadata);
        // One valid single-segment Zstandard frame containing the byte 'x'.
        state.extend(b"\x01\0\0\0\x02\0\0\0\x28\xb5\x2f\xfd\x20\x01\x09\0\0x");
        let mut preset = b"VST3\x01\0\0\0".to_vec();
        preset.extend(variant.class_uid().bytes().filter(|&byte| byte != b'-'));
        preset.extend(((HEADER_BYTES + state.len()) as u64).to_le_bytes());
        preset.extend(&state);
        preset.extend(b"List\x01\0\0\0Comp");
        preset.extend((HEADER_BYTES as u64).to_le_bytes());
        preset.extend((state.len() as u64).to_le_bytes());
        preset
    }

    #[test]
    fn accepts_only_the_matching_single_processor_class_and_version() {
        for (target, peer) in [
            (Variant::Instrument, Variant::Fx),
            (Variant::Fx, Variant::Instrument),
        ] {
            let bytes = carrier(target, "processor", target.state_product(), "2.1.5");
            assert!(valid_carrier(&bytes, target));
            assert!(!valid_carrier(&bytes, peer));
            for (component, product, version) in [
                ("controller", target.state_product(), "2.1.5"),
                ("processor", peer.state_product(), "2.1.5"),
                ("processor", target.state_product(), "2.1.4"),
            ] {
                assert!(!valid_carrier(
                    &carrier(target, component, product, version),
                    target
                ));
            }
        }
    }

    #[test]
    fn rejects_changed_header_offsets_chunks_and_frame_bounds() {
        let variant = Variant::Instrument;
        let bytes = carrier(variant, "processor", variant.state_product(), "2.1.5");
        let table = bytes.len() - CHUNK_TABLE_BYTES;
        for offset in [
            4,          // VST3 carrier format version.
            8,          // Exact uppercase class.
            40,         // Chunk table offset.
            49,         // XferJson tag.
            57,         // Xfer metadata length.
            table - 18, // Xfer decoded size.
            table - 6,  // Zstandard frame descriptor.
            table - 5,  // Zstandard encoded content size.
            table + 4,  // Additional chunk entry.
            table + 8,  // Wrong chunk kind.
            table + 12, // Chunk start.
            table + 20, // Chunk length.
        ] {
            let mut wrong = bytes.clone();
            wrong[offset] ^= 1;
            assert!(!valid_carrier(&wrong, variant), "offset {offset}");
        }
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(!valid_carrier(&trailing, variant));
        for end in 0..bytes.len() {
            assert!(
                !valid_carrier(&bytes[..end], variant),
                "prefix length {end}"
            );
        }
    }

    #[test]
    fn reads_only_bounded_regular_files_with_the_requested_digest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fixture.vstpreset");
        let variant = Variant::Fx;
        let bytes = carrier(variant, "processor", variant.state_product(), "2.1.5");
        fs::write(&path, &bytes).unwrap();
        let mut query = PresetApply {
            target: PresetTarget::Fx {
                channel: "FX".into(),
                slot: 0,
            },
            expected_instance: "DF52AB13-A21E-4EDA-855C-CA68851BACA2".into(),
            path: path.clone(),
            expected_sha256: format!("{:x}", Sha256::digest(&bytes)),
        };
        assert!(query.validate().is_ok());
        assert_eq!(query.read_preset().unwrap(), bytes);
        #[cfg(unix)]
        {
            let symlink = dir.path().join("link.vstpreset");
            std::os::unix::fs::symlink(&path, &symlink).unwrap();
            let mut linked = query.clone();
            linked.path = symlink;
            assert_eq!(
                linked.read_preset().unwrap_err().code,
                "serum_preset_unavailable"
            );
        }
        query.expected_sha256 = "0".repeat(64);
        assert_eq!(
            query.read_preset().unwrap_err().code,
            "serum_preset_digest_changed"
        );
        OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .set_len(MAX_FILE_BYTES as u64 + 1)
            .unwrap();
        assert_eq!(
            query.read_preset().unwrap_err().code,
            "serum_preset_too_large"
        );
    }

    #[test]
    fn larger_encoder_frames_may_declare_a_separate_bounded_window() {
        // A non-single-segment frame with a 1 KiB window and 256-byte output.
        let mut bytes = b"\x02\0\0\0\x28\xb5\x2f\xfd\x40\0\0\0\x03\x08\0x".to_vec();
        assert!(valid_frame(&bytes, 256));
        bytes[9] = 0xff;
        assert!(!valid_frame(&bytes, 256));
        bytes[9] = 0;
        bytes[8] = 0;
        assert!(!valid_frame(&bytes, 256));
    }
}

use std::{
    collections::BTreeMap,
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
const REFERENCE_VERSION: &str = "2.1.5";
const SCHEMA_VERSION: u8 = 11;

#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Variant {
    Instrument,
    Fx,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum PresetFormat {
    Vst3ProcessorPreset,
    SerumPreset,
}

impl PresetFormat {
    fn from_path(path: &Path) -> Option<Self> {
        if !path.is_absolute() {
            return None;
        }
        let extension = path.extension()?.to_str()?;
        if extension.eq_ignore_ascii_case("vstpreset") {
            Some(Self::Vst3ProcessorPreset)
        } else if extension.eq_ignore_ascii_case("serumpreset") {
            Some(Self::SerumPreset)
        } else {
            None
        }
    }

    fn apply_method(self) -> &'static str {
        match self {
            Self::Vst3ProcessorPreset => "serum2_preset_apply",
            Self::SerumPreset => "serum_editor",
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct PresetInspection {
    format: PresetFormat,
    variant: Variant,
    product_version: &'static str,
    schema_version: u8,
    sha256: String,
    size_bytes: usize,
    apply_method: &'static str,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct MidiMappingQuery {
    variant: Variant,
    /// Reference MIDI 1.0 controller number (CC 0–127, channel pressure 128, pitch bend 129).
    #[schemars(range(max = 129))]
    controller: u8,
}

impl MidiMappingQuery {
    pub(crate) fn lookup(self) -> Result<serde_json::Value, &'static str> {
        let (control, vst3_id) = match self.controller {
            1 => ("control_change", 516_000_000),
            0..=127 => ("control_change", 500_000_000 + u32::from(self.controller)),
            128 => ("channel_pressure", 500_000_128),
            129 => ("pitch_bend", 516_000_001),
            _ => return Err("controller must be in the reference MIDI range 0..129"),
        };
        Ok(serde_json::json!({
            "scope": "reference_vst3_sdk_midi_mapping",
            "variant": self.variant,
            "product": self.variant.plugin_name(),
            "reference_version": REFERENCE_VERSION,
            "reference_architecture": "arm64",
            "class_uid": self.variant.class_uid(),
            "bus": 0,
            "channel": 0,
            "controller": self.controller,
            "control": control,
            "parameter": {
                "vst3_id": vst3_id,
                "studio_pro_name": format!("vstp{vst3_id}"),
            },
        }))
    }
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
            || !matches!(
                PresetFormat::from_path(&self.path),
                Some(PresetFormat::Vst3ProcessorPreset)
            )
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
        let bytes = read_preset_file(&self.path, "A Serum VST3 preset cannot exceed 16 MiB")?;
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

pub(crate) fn inspect_preset(path: &Path) -> Result<PresetInspection, OperationError> {
    let format = PresetFormat::from_path(path).ok_or_else(|| {
        OperationError::new(
            "invalid_argument",
            "Supply an absolute .vstpreset or .SerumPreset path",
        )
    })?;
    let bytes = read_preset_file(path, "A Serum preset cannot exceed 16 MiB")?;
    let variant = match format {
        PresetFormat::Vst3ProcessorPreset => [Variant::Instrument, Variant::Fx]
            .into_iter()
            .find(|&variant| valid_carrier(&bytes, variant)),
        PresetFormat::SerumPreset => native_preset_variant(&bytes),
    }
    .ok_or_else(|| {
        OperationError::new(
            "invalid_serum_preset",
            "Expected a Serum 2 or Serum 2 FX 2.1.5 preset with schema 11 and a matching file format",
        )
    })?;
    Ok(PresetInspection {
        format,
        variant,
        product_version: REFERENCE_VERSION,
        schema_version: SCHEMA_VERSION,
        sha256: format!("{:x}", Sha256::digest(&bytes)),
        size_bytes: bytes.len(),
        apply_method: format.apply_method(),
    })
}

fn read_preset_file(path: &Path, size_message: &str) -> Result<Vec<u8>, OperationError> {
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
    let input = options.open(path).map_err(|_| unavailable())?;
    let metadata = input.metadata().map_err(|_| unavailable())?;
    if !metadata.is_file() {
        return Err(unavailable());
    }
    let too_large = || OperationError::new("serum_preset_too_large", size_message);
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
    Ok(bytes)
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
    let Some(metadata) = bounded_state_metadata(bytes, PresetFormat::Vst3ProcessorPreset) else {
        return false;
    };
    let Ok(metadata) = serde_json::from_slice::<StateMetadata<'_>>(metadata) else {
        return false;
    };
    metadata.component == "processor"
        && metadata.product == variant.state_product()
        && supported_metadata(metadata.product_version, metadata.version, metadata.hash)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativePresetMetadata<'a> {
    file_type: &'a str,
    product: &'a str,
    product_version: &'a str,
    version: f64,
    hash: &'a str,
    #[serde(flatten)]
    extra: BTreeMap<String, serde::de::IgnoredAny>,
}

fn native_preset_variant(bytes: &[u8]) -> Option<Variant> {
    let metadata: NativePresetMetadata<'_> =
        serde_json::from_slice(bounded_state_metadata(bytes, PresetFormat::SerumPreset)?).ok()?;
    if metadata.file_type != "SerumPreset"
        || metadata.extra.contains_key("component")
        || !supported_metadata(metadata.product_version, metadata.version, metadata.hash)
    {
        return None;
    }
    [Variant::Instrument, Variant::Fx]
        .into_iter()
        .find(|variant| variant.state_product() == metadata.product)
}

fn supported_metadata(product_version: &str, version: f64, hash: &str) -> bool {
    product_version == REFERENCE_VERSION
        && version == f64::from(SCHEMA_VERSION)
        && valid_digest(hash, 32)
}

fn bounded_state_metadata(bytes: &[u8], format: PresetFormat) -> Option<&[u8]> {
    let data = bytes.strip_prefix(b"XferJson\0")?;
    let (length, data) = data.split_first_chunk::<8>()?;
    let length = usize::try_from(u64::from_le_bytes(*length)).ok()?;
    if !(1..=MAX_METADATA_BYTES).contains(&length) {
        return None;
    }
    let (metadata, payload) = data.split_at_checked(length)?;
    let (decoded, frame) = payload.split_first_chunk::<4>()?;
    let decoded = u32::from_le_bytes(*decoded);
    ((1..=MAX_FILE_BYTES as u32).contains(&decoded) && valid_frame(frame, decoded, format))
        .then_some(metadata)
}

fn valid_frame(bytes: &[u8], decoded: u32, format: PresetFormat) -> bool {
    let Some((&descriptor, bytes)) = bytes
        .strip_prefix(b"\x02\0\0\0\x28\xb5\x2f\xfd")
        .and_then(|frame| frame.split_first())
    else {
        return false;
    };
    // The editor can omit frame content size. The outer size and window
    // still bound the frame, and the VST3 carrier requires an embedded size.
    // https://github.com/meta/zstd/blob/dev/doc/zstd_compression_format.md#frame_header
    if descriptor & 0x1b != 0 {
        return false;
    }
    let single_segment = descriptor & 0x20 != 0;
    let size_flag = descriptor >> 6;
    let width = if single_segment || size_flag != 0 {
        1 << size_flag
    } else {
        0
    };
    let bytes = if single_segment {
        bytes
    } else {
        let Some((&window, bytes)) = bytes.split_first() else {
            return false;
        };
        let base = 1_u64 << (10 + (window >> 3));
        let window_size = base + (base / 8) * u64::from(window & 7);
        if window_size > MAX_FILE_BYTES as u64 {
            return false;
        }
        bytes
    };
    let Some((size, blocks)) = bytes.split_at_checked(width) else {
        return false;
    };
    let checksum_bytes = if descriptor & 4 != 0 { 4 } else { 0 };
    if blocks.len() < 4 + checksum_bytes {
        return false;
    }
    if width == 0 {
        return matches!(format, PresetFormat::SerumPreset);
    }
    let mut little_endian = [0; 8];
    little_endian[..width].copy_from_slice(size);
    let frame_size = u64::from_le_bytes(little_endian) + if width == 2 { 256 } else { 0 };
    frame_size == u64::from(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use std::fs;

    fn state(metadata: Value) -> Vec<u8> {
        // One valid single-segment Zstandard frame containing the byte 'x'.
        state_with_frame(metadata, b"\x28\xb5\x2f\xfd\x20\x01\x09\0\0x")
    }

    fn state_with_frame(metadata: Value, frame: &[u8]) -> Vec<u8> {
        let metadata = serde_json::to_vec(&metadata).unwrap();
        let mut state = b"XferJson\0".to_vec();
        state.extend((metadata.len() as u64).to_le_bytes());
        state.extend(metadata);
        state.extend(b"\x01\0\0\0\x02\0\0\0");
        state.extend(frame);
        state
    }

    fn native_metadata(variant: Variant) -> Value {
        json!({
            "fileType": "SerumPreset",
            "product": variant.state_product(),
            "productVersion": "2.1.5",
            "version": 11,
            "hash": "0".repeat(32),
        })
    }

    fn carrier(variant: Variant, component: &str, product: &str, product_version: &str) -> Vec<u8> {
        let state = state(json!({
            "component": component,
            "product": product,
            "productVersion": product_version,
            "version": 11.0,
            "hash": "0".repeat(32),
        }));
        wrap_carrier(variant, &state)
    }

    fn wrap_carrier(variant: Variant, state: &[u8]) -> Vec<u8> {
        let mut preset = b"VST3\x01\0\0\0".to_vec();
        preset.extend(variant.class_uid().bytes().filter(|&byte| byte != b'-'));
        preset.extend(((HEADER_BYTES + state.len()) as u64).to_le_bytes());
        preset.extend(state);
        preset.extend(b"List\x01\0\0\0Comp");
        preset.extend((HEADER_BYTES as u64).to_le_bytes());
        preset.extend((state.len() as u64).to_le_bytes());
        preset
    }

    #[test]
    fn inspects_both_formats_and_variants_without_changing_the_file() {
        let dir = tempfile::tempdir().unwrap();
        for (variant, variant_name) in [(Variant::Instrument, "instrument"), (Variant::Fx, "fx")] {
            for (extension, bytes, format, apply_method) in [
                (
                    "VSTPRESET",
                    carrier(variant, "processor", variant.state_product(), "2.1.5"),
                    "vst3_processor_preset",
                    "serum2_preset_apply",
                ),
                (
                    "SerumPreset",
                    state(native_metadata(variant)),
                    "serum_preset",
                    "serum_editor",
                ),
            ] {
                let path = dir.path().join(format!("{variant_name}.{extension}"));
                fs::write(&path, &bytes).unwrap();
                let inspected = serde_json::to_value(inspect_preset(&path).unwrap()).unwrap();
                assert_eq!(
                    inspected,
                    json!({
                        "format": format,
                        "variant": variant_name,
                        "product_version": "2.1.5",
                        "schema_version": 11,
                        "sha256": format!("{:x}", Sha256::digest(&bytes)),
                        "size_bytes": bytes.len(),
                        "apply_method": apply_method,
                    })
                );
                assert_eq!(fs::read(path).unwrap(), bytes);
            }
        }
    }

    #[test]
    fn inspection_rejects_wrong_extensions_metadata_and_versions() {
        let dir = tempfile::tempdir().unwrap();
        let native = dir.path().join("fixture.SerumPreset");
        let vst = dir.path().join("fixture.vstpreset");
        let variant = Variant::Instrument;
        let invalid = |path: &Path, bytes: Vec<u8>| {
            fs::write(path, bytes).unwrap();
            assert_eq!(
                inspect_preset(path).unwrap_err().code,
                "invalid_serum_preset"
            );
        };
        invalid(&vst, state(native_metadata(variant)));
        invalid(
            &native,
            carrier(variant, "processor", variant.state_product(), "2.1.5"),
        );
        invalid(
            &vst,
            carrier(variant, "processor", variant.state_product(), "2.1.4"),
        );
        for (field, value) in [
            ("component", Value::Null),
            ("component", json!("processor")),
            ("productVersion", json!("2.1.4")),
            ("version", json!(12)),
            ("fileType", json!("Processor")),
            ("hash", json!("incorrect")),
        ] {
            let mut metadata = native_metadata(variant);
            metadata[field] = value;
            invalid(&native, state(metadata));
        }
        for path in [
            PathBuf::from("relative.SerumPreset"),
            dir.path().join("wrong.fxp"),
        ] {
            assert_eq!(inspect_preset(&path).unwrap_err().code, "invalid_argument");
        }
    }

    #[test]
    fn inspection_accepts_a_bounded_native_frame_without_an_embedded_size() {
        let dir = tempfile::tempdir().unwrap();
        let native = dir.path().join("fixture.SerumPreset");
        let vst = dir.path().join("fixture.vstpreset");
        let variant = Variant::Instrument;
        let mut frame = *b"\x28\xb5\x2f\xfd\x04\x58\x09\0\0x\0\0\0\0";
        fs::write(&native, state_with_frame(native_metadata(variant), &frame)).unwrap();
        assert_eq!(
            serde_json::to_value(inspect_preset(&native).unwrap()).unwrap()["format"],
            "serum_preset"
        );
        let processor = state_with_frame(
            json!({
                "component": "processor",
                "product": "Serum2",
                "productVersion": "2.1.5",
                "version": 11,
                "hash": "0".repeat(32),
            }),
            &frame,
        );
        fs::write(&vst, wrap_carrier(variant, &processor)).unwrap();
        assert_eq!(
            inspect_preset(&vst).unwrap_err().code,
            "invalid_serum_preset"
        );
        fs::write(
            &native,
            state_with_frame(native_metadata(variant), &frame[..13]),
        )
        .unwrap();
        assert_eq!(
            inspect_preset(&native).unwrap_err().code,
            "invalid_serum_preset"
        );
        frame[5] = 0xff;
        fs::write(&native, state_with_frame(native_metadata(variant), &frame)).unwrap();
        assert_eq!(
            inspect_preset(&native).unwrap_err().code,
            "invalid_serum_preset"
        );
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
        assert!(valid_frame(&bytes, 256, PresetFormat::Vst3ProcessorPreset));
        bytes[9] = 0xff;
        assert!(!valid_frame(&bytes, 256, PresetFormat::Vst3ProcessorPreset));
        bytes[9] = 0;
        bytes[8] = 0;
        assert!(!valid_frame(&bytes, 256, PresetFormat::Vst3ProcessorPreset));
    }
}

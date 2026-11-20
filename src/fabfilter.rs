use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MAX_ENTRIES: usize = 16_384;
const MAX_DEPTH: usize = 16;
const MAX_PAGE: usize = 128;
const MAX_PRESET_BYTES: usize = 64 * 1024;

macro_rules! products {
    ($($variant:ident => ($name:literal, $version:literal)),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, Deserialize, JsonSchema, Serialize)]
        pub enum Product {
            $(#[serde(rename = $name)] $variant),+
        }

        impl Product {
            fn directory(self) -> &'static str {
                match self { $(Self::$variant => $name),+ }
            }

            fn vst3_reference_version(self) -> &'static str {
                match self { $(Self::$variant => $version),+ }
            }
        }
    };
}

products! {
    Micro => ("Micro", "1.32"), One => ("One", "3.52"),
    ProC2 => ("Pro-C 2", "2.22"), ProC3 => ("Pro-C 3", "3.02"),
    ProDS => ("Pro-DS", "1.32"), ProG => ("Pro-G", "1.42"),
    ProL => ("Pro-L", "1.37"), ProL2 => ("Pro-L 2", "2.26"),
    ProMB => ("Pro-MB", "1.33"), ProQ2 => ("Pro-Q 2", "2.30"),
    ProQ3 => ("Pro-Q 3", "3.29"), ProQ4 => ("Pro-Q 4", "4.13"),
    ProR => ("Pro-R", "1.20"), ProR2 => ("Pro-R 2", "2.06"),
    Saturn => ("Saturn", "1.31"), Saturn2 => ("Saturn 2", "2.13"),
    Simplon => ("Simplon", "1.42"), Timeless2 => ("Timeless 2", "2.41"),
    Timeless3 => ("Timeless 3", "3.10"), Twin2 => ("Twin 2", "2.41"),
    Twin3 => ("Twin 3", "3.10"), Volcano2 => ("Volcano 2", "2.41"),
    Volcano3 => ("Volcano 3", "3.09"),
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MidiMappingQuery {
    product: Product,
    /// VST3 controller number. CC 0–127, channel pressure 128, pitch bend 129, program change 130.
    #[schemars(range(max = 130))]
    controller: u8,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum MidiControl {
    ControlChange,
    ChannelPressure,
    PitchBend,
    ProgramChange,
}

#[derive(Serialize)]
struct MidiParameter {
    vst3_id: u32,
    studio_pro_name: String,
}

#[derive(Serialize)]
pub struct MidiMapping {
    scope: &'static str,
    product: Product,
    reference_version: &'static str,
    bus: u8,
    channel: u8,
    controller: u8,
    control: MidiControl,
    parameter: Option<MidiParameter>,
}

impl MidiMappingQuery {
    pub fn lookup(self) -> Result<MidiMapping, &'static str> {
        let (control, id) = match self.controller {
            0..=127 => (
                MidiControl::ControlChange,
                Some(0x4000_0007 + u32::from(self.controller)),
            ),
            128 => (MidiControl::ChannelPressure, Some(0x4000_0006)),
            129 => (MidiControl::PitchBend, Some(0x4000_0005)),
            130 => (MidiControl::ProgramChange, None),
            _ => return Err("controller must be in the VST3 range 0..130"),
        };
        Ok(MidiMapping {
            scope: "reference_vst3_sdk_midi_mapping",
            product: self.product,
            reference_version: self.product.vst3_reference_version(),
            bus: 0,
            channel: 0,
            controller: self.controller,
            control,
            parameter: id.map(|vst3_id| MidiParameter {
                vst3_id,
                studio_pro_name: format!("vstp{vst3_id}"),
            }),
        })
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PresetQuery {
    pub product: Product,
    #[serde(default)]
    #[schemars(range(max = 16384))]
    pub offset: usize,
    #[serde(default = "default_limit")]
    #[schemars(range(min = 1, max = 128))]
    pub limit: usize,
    /// Digest returned with the first page. Required when offset is greater than zero.
    #[schemars(length(min = 64, max = 64))]
    pub expected_catalog: Option<String>,
}

fn default_limit() -> usize {
    64
}

impl PresetQuery {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.offset > MAX_ENTRIES || !(1..=MAX_PAGE).contains(&self.limit) {
            return Err("offset or limit is outside the supported range");
        }
        match self.expected_catalog.as_deref() {
            Some(digest) if digest.len() == 64 && digest.bytes().all(|c| c.is_ascii_hexdigit()) => {
                Ok(())
            }
            None if self.offset == 0 => Ok(()),
            _ => Err("expected_catalog must be a 64-digit hexadecimal digest"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PresetApply {
    pub product: Product,
    #[schemars(range(max = 16383))]
    pub index: usize,
    #[schemars(length(min = 64, max = 64))]
    pub expected_catalog: String,
    pub channel: Option<String>,
    #[schemars(range(max = 15))]
    pub slot: Option<u32>,
    pub track: Option<String>,
    pub expected_instance: String,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct Preset {
    folders: Vec<String>,
    name: String,
    #[serde(skip)]
    filename: String,
}

pub(crate) struct PresetAsset {
    pub filename: String,
    pub contents: Vec<u8>,
    pub sha256: String,
}

impl PresetAsset {
    pub(crate) fn wire_data(&self) -> String {
        const HEX: &[u8] = b"0123456789abcdef";
        let mut out = String::with_capacity(self.contents.len() * 2);
        for byte in &self.contents {
            out.push(char::from(HEX[usize::from(byte >> 4)]));
            out.push(char::from(HEX[usize::from(byte & 15)]));
        }
        out
    }
}

#[derive(Debug, Serialize)]
pub struct PresetPage {
    product: Product,
    catalog_sha256: String,
    total: usize,
    offset: usize,
    next_offset: Option<usize>,
    /// A snapshot of file names. It carries no program numbers or load authorization.
    presets: Vec<Preset>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CatalogError {
    LibraryUnavailable,
    ProductUnavailable,
    Unreadable,
    LimitExceeded,
    CatalogChanged,
    OffsetOutsideCatalog,
    UnsupportedApplyProduct,
    AmbiguousPreset,
    InvalidPreset,
}

impl CatalogError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::LibraryUnavailable => "preset_library_unavailable",
            Self::ProductUnavailable => "preset_product_unavailable",
            Self::Unreadable => "preset_library_unreadable",
            Self::LimitExceeded => "preset_library_limit_exceeded",
            Self::CatalogChanged => "preset_catalog_changed",
            Self::OffsetOutsideCatalog => "preset_offset_outside_catalog",
            Self::UnsupportedApplyProduct => "preset_product_apply_unsupported",
            Self::AmbiguousPreset => "preset_entry_ambiguous",
            Self::InvalidPreset => "preset_file_invalid",
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            Self::LibraryUnavailable => "The configured FabFilter preset library is unavailable.",
            Self::ProductUnavailable => "The preset library has no directory for this product.",
            Self::Unreadable => "The preset folders could not be read as one bounded catalog.",
            Self::LimitExceeded => {
                "The preset library exceeded the supported entry or depth limit."
            }
            Self::CatalogChanged => "The preset names changed. Request the first page again.",
            Self::OffsetOutsideCatalog => "The requested offset is past the end of the catalog.",
            Self::UnsupportedApplyProduct => {
                "File application supports Pro-Q 4 4.13, Pro-C 3 3.02, and Twin 3 3.10 VST3."
            }
            Self::AmbiguousPreset => "More than one preset file has the requested catalog name.",
            Self::InvalidPreset => {
                "The selected file does not match the requested FabFilter preset format."
            }
        }
    }
}

fn library_root() -> Result<PathBuf, CatalogError> {
    std::env::var_os("FSP_FABFILTER_PRESET_LIBRARY")
        .map(PathBuf::from)
        .or_else(|| std::env::home_dir().map(|home| home.join("Documents/FabFilter/Presets")))
        .ok_or(CatalogError::LibraryUnavailable)
}

pub fn list(query: &PresetQuery) -> Result<PresetPage, CatalogError> {
    list_from(&library_root()?, query)
}

pub(crate) fn load_for_apply(query: &PresetApply) -> Result<PresetAsset, CatalogError> {
    load_from(&library_root()?, query)
}

fn load_from(library: &Path, query: &PresetApply) -> Result<PresetAsset, CatalogError> {
    if !matches!(
        query.product,
        Product::ProQ4 | Product::ProC3 | Product::ProR | Product::ProR2 | Product::Twin3
    ) {
        return Err(CatalogError::UnsupportedApplyProduct);
    }
    let (directory, presets, digest) = catalog(library, query.product)?;
    if !query.expected_catalog.eq_ignore_ascii_case(&digest) {
        return Err(CatalogError::CatalogChanged);
    }
    let selected = presets
        .get(query.index)
        .ok_or(CatalogError::OffsetOutsideCatalog)?;
    if presets
        .iter()
        .filter(|preset| preset.name == selected.name && preset.folders == selected.folders)
        .take(2)
        .count()
        != 1
    {
        return Err(CatalogError::AmbiguousPreset);
    }
    if selected.filename.len() > 255 || selected.filename.chars().any(char::is_control) {
        return Err(CatalogError::InvalidPreset);
    }
    let mut file_path = directory;
    file_path.extend(&selected.folders);
    file_path.push(&selected.filename);
    if file_path
        .canonicalize()
        .map_err(|_| CatalogError::Unreadable)?
        != file_path
    {
        return Err(CatalogError::Unreadable);
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options
        .open(&file_path)
        .map_err(|_| CatalogError::Unreadable)?;
    let metadata = file.metadata().map_err(|_| CatalogError::Unreadable)?;
    if !metadata.is_file() || metadata.len() > MAX_PRESET_BYTES as u64 {
        return Err(CatalogError::InvalidPreset);
    }
    let mut contents = Vec::with_capacity(metadata.len() as usize);
    file.take(MAX_PRESET_BYTES as u64 + 1)
        .read_to_end(&mut contents)
        .map_err(|_| CatalogError::Unreadable)?;
    if contents.len() != metadata.len() as usize || !matches_preset(query.product, &contents) {
        return Err(CatalogError::InvalidPreset);
    }
    let sha256 = format!("{:x}", Sha256::digest(&contents));
    Ok(PresetAsset {
        filename: selected.filename.clone(),
        contents,
        sha256,
    })
}

fn matches_preset(product: Product, bytes: &[u8]) -> bool {
    let signature = match product {
        Product::ProQ4 => b"Signature=FQ4p",
        Product::ProC3 => b"Signature=FC3p",
        Product::ProR | Product::ProR2 if bytes.starts_with(b"FPRr") => {
            return bytes.len() >= 16 && bytes.starts_with(b"FPRr\x04\0\0\0");
        }
        Product::ProR2 => b"Signature=FR2p",
        Product::Twin3 => return bytes.len() >= 16 && bytes.starts_with(b"FT3i"),
        _ => return false,
    };
    let mut header = bytes
        .split(|byte| *byte == b'\n')
        .map(|line| line.strip_suffix(b"\r").unwrap_or(line));
    header.next() == Some(b"[Preset]".as_slice())
        && header.next() == Some(signature.as_slice())
        && header.next().is_some_and(|line| match product {
            Product::ProR2 => line == b"Version=6",
            _ => line.starts_with(b"Version="),
        })
        && header.next().is_some()
}

fn catalog(
    library: &Path,
    product: Product,
) -> Result<(PathBuf, Vec<Preset>, String), CatalogError> {
    if !library.is_absolute() {
        return Err(CatalogError::LibraryUnavailable);
    }
    let library = library.canonicalize().map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            CatalogError::LibraryUnavailable
        } else {
            CatalogError::Unreadable
        }
    })?;
    let directory = library.join(product.directory());
    let meta = fs::symlink_metadata(&directory).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            CatalogError::ProductUnavailable
        } else {
            CatalogError::Unreadable
        }
    })?;
    if !meta.is_dir() {
        return Err(CatalogError::ProductUnavailable);
    }
    let mut presets = Vec::new();
    let mut remaining = MAX_ENTRIES;
    collect(&directory, &mut Vec::new(), &mut remaining, &mut presets)?;
    presets.sort_unstable();

    let digest_input = serde_json::to_vec(&(product, &presets)).expect("preset names serialize");
    let digest = format!("{:x}", Sha256::digest(digest_input));
    Ok((directory, presets, digest))
}

fn list_from(library: &Path, query: &PresetQuery) -> Result<PresetPage, CatalogError> {
    let (_, mut presets, catalog_sha256) = catalog(library, query.product)?;
    if query
        .expected_catalog
        .as_ref()
        .is_some_and(|expected| !expected.eq_ignore_ascii_case(&catalog_sha256))
    {
        return Err(CatalogError::CatalogChanged);
    }
    let total = presets.len();
    if query.offset > total {
        return Err(CatalogError::OffsetOutsideCatalog);
    }
    let end = (query.offset + query.limit).min(total);
    Ok(PresetPage {
        product: query.product,
        catalog_sha256,
        total,
        offset: query.offset,
        next_offset: (end < total).then_some(end),
        presets: presets.drain(query.offset..end).collect(),
    })
}

fn collect(
    path: &Path,
    folders: &mut Vec<String>,
    remaining: &mut usize,
    presets: &mut Vec<Preset>,
) -> Result<(), CatalogError> {
    if folders.len() > MAX_DEPTH {
        return Err(CatalogError::LimitExceeded);
    }
    if path.canonicalize().map_err(|_| CatalogError::Unreadable)? != path {
        return Err(CatalogError::Unreadable);
    }
    let entries = fs::read_dir(path).map_err(|_| CatalogError::Unreadable)?;
    for entry in entries {
        *remaining = remaining
            .checked_sub(1)
            .ok_or(CatalogError::LimitExceeded)?;
        let entry = entry.map_err(|_| CatalogError::Unreadable)?;
        let kind = entry.file_type().map_err(|_| CatalogError::Unreadable)?;
        let filename = entry.file_name();
        let name = filename.to_str().ok_or(CatalogError::Unreadable)?;
        if name.starts_with('.') || kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            folders.push(name.to_owned());
            collect(&entry.path(), folders, remaining, presets)?;
            folders.pop();
        } else if kind.is_file() {
            let file = Path::new(name);
            if file
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("ffp"))
            {
                let stem = file
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .ok_or(CatalogError::Unreadable)?;
                presets.push(Preset {
                    folders: folders.clone(),
                    name: stem.to_owned(),
                    filename: name.to_owned(),
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sdk_controller_numbers_distinguish_program_change_from_named_parameters() {
        let lookup = |controller| {
            MidiMappingQuery {
                product: Product::ProQ4,
                controller,
            }
            .lookup()
        };
        for (controller, id) in [
            (0, 1073741831),
            (127, 1073741958),
            (128, 1073741830),
            (129, 1073741829),
        ] {
            let mapping = lookup(controller).unwrap();
            assert_eq!((mapping.bus, mapping.channel), (0, 0));
            assert_eq!(mapping.reference_version, "4.13");
            let parameter = mapping.parameter.unwrap();
            assert_eq!(parameter.vst3_id, id);
            assert_eq!(parameter.studio_pro_name, format!("vstp{id}"));
        }
        let program = lookup(130).unwrap();
        assert!(matches!(program.control, MidiControl::ProgramChange));
        assert!(program.parameter.is_none());
        assert!(lookup(131).is_err());
    }

    #[test]
    fn pages_keep_relative_folders_and_reject_changed_names() {
        let library = tempfile::tempdir().unwrap();
        let product = library.path().join("Pro-Q 4");
        let sub = product.join("Lead/Live");
        fs::create_dir_all(&sub).unwrap();
        fs::write(product.join("Flat.ffp"), b"").unwrap();
        fs::write(sub.join("Flat.FFP"), b"").unwrap();
        fs::write(sub.join("notes.txt"), b"").unwrap();
        fs::write(sub.join("._Flat.FFP"), b"").unwrap();
        let mut query = PresetQuery {
            product: Product::ProQ4,
            offset: 0,
            limit: 1,
            expected_catalog: None,
        };
        let first = list_from(library.path(), &query).unwrap();
        assert_eq!(first.total, 2);
        assert_eq!(first.presets[0].name, "Flat");
        assert_eq!(first.next_offset, Some(1));
        query.offset = 1;
        assert!(query.validate().is_err());
        query.expected_catalog = Some(first.catalog_sha256);
        query.validate().unwrap();
        let second = list_from(library.path(), &query).unwrap();
        assert_eq!(second.presets[0].folders, ["Lead", "Live"]);
        assert_eq!(second.next_offset, None);
        fs::rename(sub.join("Flat.FFP"), sub.join("Mid.ffp")).unwrap();
        assert_eq!(
            list_from(library.path(), &query).unwrap_err(),
            CatalogError::CatalogChanged
        );
    }

    #[test]
    fn apply_resolves_one_index_and_refuses_stale_catalog_or_wrong_product_file() {
        const FFP: &[u8] = b"[Preset]\r\nSignature=FQ4p\r\nVersion=4\r\nAuthor=Fixture\r\n";
        let library = tempfile::tempdir().unwrap();
        let product = library.path().join("Pro-Q 4");
        let nested = product.join("Effects");
        fs::create_dir_all(&nested).unwrap();
        fs::write(
            product.join("Same.ffp"),
            b"[Preset]\nSignature=FQ3p\nVersion=3\n",
        )
        .unwrap();
        fs::write(nested.join("Same.FFP"), FFP).unwrap();
        let page = list_from(
            library.path(),
            &PresetQuery {
                product: Product::ProQ4,
                offset: 0,
                limit: 64,
                expected_catalog: None,
            },
        )
        .unwrap();
        let mut query = PresetApply {
            product: Product::ProQ4,
            index: 1,
            expected_catalog: page.catalog_sha256,
            channel: Some("Scratch".to_owned()),
            slot: Some(0),
            track: None,
            expected_instance: "AA50C180-3980-4EE0-A76B-3195182C91E0".to_owned(),
        };
        let selected = load_from(library.path(), &query).unwrap();
        assert_eq!(selected.filename, "Same.FFP");
        assert_eq!(selected.contents, FFP);
        assert_eq!(selected.sha256, format!("{:x}", Sha256::digest(FFP)));
        assert_eq!(selected.wire_data().len(), FFP.len() * 2);
        query.index = 0;
        assert!(matches!(
            load_from(library.path(), &query),
            Err(CatalogError::InvalidPreset)
        ));
        fs::rename(nested.join("Same.FFP"), nested.join("Renamed.ffp")).unwrap();
        assert!(matches!(
            load_from(library.path(), &query),
            Err(CatalogError::CatalogChanged)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn product_and_nested_discovery_do_not_follow_artifact_symlinks() {
        use std::os::unix::fs::symlink;

        let library = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        fs::write(elsewhere.path().join("Outside.ffp"), b"").unwrap();
        symlink(elsewhere.path(), library.path().join("Twin 3")).unwrap();
        let query = PresetQuery {
            product: Product::Twin3,
            offset: 0,
            limit: 1,
            expected_catalog: None,
        };
        assert_eq!(
            list_from(library.path(), &query).unwrap_err(),
            CatalogError::ProductUnavailable
        );
        let product = library.path().join("One");
        fs::create_dir(&product).unwrap();
        symlink(elsewhere.path(), product.join("outside")).unwrap();
        symlink(
            elsewhere.path().join("Outside.ffp"),
            product.join("outside.ffp"),
        )
        .unwrap();
        let page = list_from(
            library.path(),
            &PresetQuery {
                product: Product::One,
                ..query
            },
        )
        .unwrap();
        assert_eq!(page.total, 0);
    }

    #[test]
    fn twin_preset_format_requires_a_bounded_binary_header() {
        assert!(matches_preset(Product::Twin3, b"FT3i012345678901"));
        assert!(!matches_preset(Product::Twin3, b"FT3i01234567890"));
        assert!(!matches_preset(Product::Twin3, b"FQ4p012345678901"));
    }

    #[test]
    fn pro_r_products_accept_their_observed_factory_formats() {
        let text = b"[Preset]\r\nSignature=FR2p\r\nVersion=6\r\nAuthor=Fixture\r\n";
        let binary = b"FPRr\x04\0\0\0abcdefgh";
        assert!(matches_preset(Product::ProR2, text));
        assert!(matches_preset(Product::ProR2, binary));
        assert!(matches_preset(Product::ProR, binary));
        assert!(!matches_preset(Product::ProR, text));
        assert!(!matches_preset(Product::ProR, b"FPRr\x05\0\0\0abcdefgh"));
        assert!(!matches_preset(Product::ProR2, b"FPRr\x05\0\0\0abcdefgh"));
        assert!(!matches_preset(Product::ProR2, b"FPRr\x04\0\0\0"));
        assert!(!matches_preset(
            Product::ProR2,
            b"[Preset]\nSignature=FR2p\nVersion=5\nAuthor=Fixture"
        ));
    }

    #[test]
    fn pro_c_3_requires_its_own_text_signature() {
        let bytes = b"[Preset]\r\nSignature=FC3p\r\nVersion=3\r\nAuthor=Fixture\r\n";
        assert!(matches_preset(Product::ProC3, bytes));
        assert!(!matches_preset(Product::ProQ4, bytes));
    }
}

use std::{fs, io, path::Path};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MAX_ENTRIES: usize = 16_384;
const MAX_DEPTH: usize = 16;
const MAX_PAGE: usize = 128;

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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct Preset {
    folders: Vec<String>,
    name: String,
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
        }
    }
}

pub fn list(query: &PresetQuery) -> Result<PresetPage, CatalogError> {
    let library = std::env::var_os("FSP_FABFILTER_PRESET_LIBRARY")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::home_dir().map(|home| home.join("Documents/FabFilter/Presets")))
        .ok_or(CatalogError::LibraryUnavailable)?;
    list_from(&library, query)
}

fn list_from(library: &Path, query: &PresetQuery) -> Result<PresetPage, CatalogError> {
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
    let directory = library.join(query.product.directory());
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

    let digest_input =
        serde_json::to_vec(&(query.product, &presets)).expect("preset names serialize");
    let catalog_sha256 = format!("{:x}", Sha256::digest(digest_input));
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
}

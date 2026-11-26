use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{
    MAX_FILE_BYTES, MAX_METADATA_BYTES, NativePresetMetadata, PresetFormat, REFERENCE_VERSION,
    SCHEMA_VERSION, Variant, bounded_state_metadata, open_preset_file, valid_digest,
};
use crate::model::OperationError;

const DEFAULT_LIBRARY: &str = "/Library/Audio/Presets/Xfer Records/Serum 2 Presets/Presets/Factory";
const MAX_ENTRIES: usize = 16_384;
const MAX_DEPTH: usize = 16;
const MAX_PAGE: usize = 128;
const MAX_PRESET_PREFIX_BYTES: usize = MAX_METADATA_BYTES + 48;

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct FactoryPresetQuery {
    variant: Option<Variant>,
    /// Case-insensitive match against the preset name and relative path.
    #[serde(default)]
    #[schemars(length(max = 128))]
    search: String,
    #[serde(default)]
    #[schemars(range(max = 16384))]
    offset: usize,
    #[serde(default = "default_limit")]
    #[schemars(range(min = 1, max = 128))]
    limit: usize,
    /// Catalog SHA-256 from the first page of the same query. Required for later pages.
    #[schemars(length(equal = 64), regex(pattern = "^[0-9a-f]{64}$"))]
    expected_catalog: Option<String>,
}

fn default_limit() -> usize {
    64
}

#[derive(Serialize)]
pub(crate) struct FactoryPresetPage {
    scope: &'static str,
    apply_method: &'static str,
    catalog_sha256: String,
    total: usize,
    offset: usize,
    next_offset: Option<usize>,
    presets: Vec<FactoryPreset>,
}

#[derive(Serialize)]
struct FactoryPreset {
    variant: Variant,
    name: String,
    relative_path: String,
    path: PathBuf,
    source_product_version: String,
    source_schema_version: u8,
}

pub(crate) fn list_factory_presets(
    query: &FactoryPresetQuery,
) -> Result<FactoryPresetPage, OperationError> {
    let library = std::env::var_os("FSP_SERUM_FACTORY_PRESET_LIBRARY")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_LIBRARY));
    list_from(&library, query)
}

fn list_from(
    library: &Path,
    query: &FactoryPresetQuery,
) -> Result<FactoryPresetPage, OperationError> {
    if query.search.chars().count() > 128
        || query.search.chars().any(char::is_control)
        || query.offset > MAX_ENTRIES
        || !(1..=MAX_PAGE).contains(&query.limit)
        || query
            .expected_catalog
            .as_deref()
            .map_or(query.offset > 0, |digest| !valid_digest(digest, 64))
    {
        return Err(OperationError::new(
            "invalid_argument",
            "Use a search of up to 128 characters, a limit of 1 to 128, and the first page's lowercase catalog SHA-256 for later pages",
        ));
    }
    let unavailable = || {
        OperationError::new(
            "serum_factory_library_unavailable",
            "The local Serum 2 Factory preset directory cannot be read",
        )
    };
    if !library.is_absolute() {
        return Err(unavailable());
    }
    let library = library.canonicalize().map_err(|_| unavailable())?;
    let search = query.search.trim().to_lowercase();
    let mut visited = 0;
    let mut pending = vec![(library.clone(), 0)];
    let mut presets = Vec::new();
    while let Some((directory, depth)) = pending.pop() {
        if directory.canonicalize().map_err(|_| unavailable())? != directory {
            return Err(unavailable());
        }
        for entry in fs::read_dir(directory).map_err(|_| unavailable())? {
            visited += 1;
            if visited > MAX_ENTRIES || depth > MAX_DEPTH {
                return Err(OperationError::new(
                    "serum_factory_catalog_limit",
                    "The local Serum 2 Factory directory exceeds the catalog bounds",
                ));
            }
            let entry = entry.map_err(|_| unavailable())?;
            let kind = entry.file_type().map_err(|_| unavailable())?;
            let path = entry.path();
            if kind.is_dir() {
                pending.push((path, depth + 1));
            } else if kind.is_file()
                && let Some(preset) = read_entry(&library, path)
                && query
                    .variant
                    .is_none_or(|variant| variant == preset.variant)
                && (preset.relative_path.to_lowercase().contains(&search)
                    || preset.name.to_lowercase().contains(&search))
            {
                presets.push(preset);
            }
        }
    }
    presets.sort_by_cached_key(|preset| {
        (
            preset.relative_path.to_lowercase(),
            preset.relative_path.clone(),
        )
    });
    let mut hash = Sha256::new();
    serde_json::to_writer(&mut hash, &presets).map_err(|_| unavailable())?;
    let catalog_sha256 = format!("{:x}", hash.finalize());
    if query
        .expected_catalog
        .as_ref()
        .is_some_and(|digest| digest != &catalog_sha256)
    {
        return Err(OperationError::new(
            "serum_factory_catalog_changed",
            "The Serum Factory results changed. Start again at offset zero",
        ));
    }
    let total = presets.len();
    let end = query.offset + query.limit;
    Ok(FactoryPresetPage {
        scope: "local_serum_factory_preset_headers",
        apply_method: PresetFormat::SerumPreset.apply_method(),
        catalog_sha256,
        total,
        offset: query.offset,
        next_offset: (end < total).then_some(end),
        presets: presets
            .into_iter()
            .skip(query.offset)
            .take(query.limit)
            .collect(),
    })
}

fn read_entry(library: &Path, path: PathBuf) -> Option<FactoryPreset> {
    if !matches!(
        PresetFormat::from_path(&path),
        Some(PresetFormat::SerumPreset)
    ) || path.canonicalize().ok()? != path
    {
        return None;
    }
    let relative_path = path.strip_prefix(library).ok()?.to_str()?.to_owned();
    if relative_path.chars().any(char::is_control) {
        return None;
    }
    let (file, size) = open_preset_file(&path).ok()?;
    if size > MAX_FILE_BYTES as u64 {
        return None;
    }
    let mut prefix = Vec::new();
    file.take(MAX_PRESET_PREFIX_BYTES as u64)
        .read_to_end(&mut prefix)
        .ok()?;
    let header = bounded_state_metadata(&prefix, PresetFormat::SerumPreset)?;
    let metadata: NativePresetMetadata<'_> = serde_json::from_slice(header).ok()?;
    let variant = [Variant::Instrument, Variant::Fx]
        .into_iter()
        .find(|variant| variant.state_product() == metadata.product)?;
    let schema = metadata.version as u8;
    if metadata.file_type != "SerumPreset"
        || metadata.extra.contains_key("component")
        || metadata.version != f64::from(schema)
        || !valid_digest(metadata.hash, 32)
        || !known_schema(variant, metadata.product_version, schema)
    {
        return None;
    }
    let details: serde_json::Value = serde_json::from_slice(header).ok()?;
    let name = details["presetName"]
        .as_str()
        .filter(|name| {
            !name.trim().is_empty() && name.len() <= 255 && !name.chars().any(char::is_control)
        })
        .unwrap_or(path.file_stem()?.to_str()?)
        .to_owned();
    Some(FactoryPreset {
        variant,
        name,
        relative_path,
        path,
        source_product_version: metadata.product_version.to_owned(),
        source_schema_version: schema,
    })
}

fn known_schema(variant: Variant, product_version: &str, schema: u8) -> bool {
    matches!(
        (variant, product_version, schema),
        (_, REFERENCE_VERSION, SCHEMA_VERSION)
            | (Variant::Instrument, "2.0.11", 3 | 4)
            | (Variant::Instrument, "2.0.12", 4 | 5)
            | (Variant::Instrument, "2.0.13", 5 | 6)
            | (Variant::Instrument, "2.0.14" | "2.0.15", 6)
            | (Variant::Fx, "2.0.13", 5)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serum::tests::state;
    use serde_json::{Value, json};

    fn write_preset(path: &Path, variant: Variant, version: &str, schema: u8, name: &str) {
        let bytes = state(json!({
            "fileType": "SerumPreset", "product": variant.state_product(),
            "productVersion": version, "version": schema, "presetName": name,
            "hash": "0".repeat(32),
        }));
        fs::write(path, bytes).unwrap();
    }

    fn query(library: &Path, args: Value) -> Result<FactoryPresetPage, OperationError> {
        list_from(library, &serde_json::from_value(args).unwrap())
    }

    #[test]
    fn catalog_recognizes_legacy_headers_without_expanding_processor_support() {
        let temp = tempfile::tempdir().unwrap();
        let library = temp.path();
        let subdir = library.join("Filter");
        fs::create_dir(&subdir).unwrap();
        write_preset(
            &subdir.join("z.SerumPreset"),
            Variant::Fx,
            "2.0.13",
            5,
            "FX Lead",
        );
        let older = library.join("A.serumpreset");
        write_preset(&older, Variant::Instrument, "2.0.11", 3, "Lead");
        write_preset(
            &library.join("wrong.SerumPreset"),
            Variant::Fx,
            "2.0.11",
            3,
            "Lead",
        );
        write_preset(
            &library.join("ignored.vstpreset"),
            Variant::Fx,
            "2.0.13",
            5,
            "FX Lead",
        );
        fs::write(library.join("malformed.SerumPreset"), b"XferJson\0").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&older, library.join("Link.SerumPreset")).unwrap();

        let page = query(library, json!({"search":" LEAD ", "limit": 1})).unwrap();
        assert_eq!(page.total, 2);
        assert_eq!(page.next_offset, Some(1));
        assert_eq!(page.presets[0].relative_path, "A.serumpreset");
        assert_eq!(page.presets[0].source_schema_version, 3);

        let later = json!({"search":"lead", "offset":1, "expected_catalog":page.catalog_sha256});
        let second = query(library, later.clone()).unwrap();
        assert_eq!(second.next_offset, None);
        assert_eq!(second.presets[0].name, "FX Lead");
        assert_eq!(second.presets[0].variant, Variant::Fx);
        assert_eq!(
            query(library, json!({"variant":"fx", "search":"filter"}))
                .unwrap()
                .total,
            1
        );
        assert_eq!(
            super::super::inspect_preset(&older).unwrap_err().code,
            "invalid_serum_preset"
        );

        fs::rename(older, library.join("B.SerumPreset")).unwrap();
        assert!(
            matches!(query(library, later), Err(error) if error.code == "serum_factory_catalog_changed")
        );
        assert!(
            matches!(query(library, json!({"offset":1})), Err(error) if error.code == "invalid_argument")
        );
    }
}

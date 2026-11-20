use std::{fs::File, io::Read};

use serde_json::{Value, json};
use zip::ZipArchive;

const MAX_ENTRIES: usize = 256;
const MAX_METADATA_BYTES: u64 = 256 * 1024;
const MAX_OUTPUT_BYTES: usize = 128 * 1024;

pub(super) fn inspect(file: File, expected_parts: u32) -> Value {
    match read(file, expected_parts) {
        Ok(Some(value)) if value.to_string().len() <= MAX_OUTPUT_BYTES => value,
        Ok(None) => json!({"status":"absent"}),
        _ => json!({"status":"unavailable"}),
    }
}

fn read(file: File, expected_parts: u32) -> Result<Option<Value>, ()> {
    let mut archive = ZipArchive::new(file).map_err(|_| ())?;
    if archive.len() > MAX_ENTRIES || archive.has_duplicate_names() {
        return Err(());
    }
    if archive.index_for_name("fxgraph.xml").is_none() {
        return Ok(None);
    }
    let mut member = |name| {
        let entry = archive.by_name(name).map_err(|_| ())?;
        if entry.size() > MAX_METADATA_BYTES {
            return Err(());
        }
        let mut bytes = Vec::new();
        entry
            .take(MAX_METADATA_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| ())?;
        if bytes.len() as u64 > MAX_METADATA_BYTES {
            return Err(());
        }
        Ok(bytes)
    };
    let graph = fender_routing_graph::read_collection(
        &member("fxgraph.xml")?,
        &member("presetparts.xml")?,
        expected_parts as usize,
    )
    .map_err(|_| ())?;
    Ok(Some(json!({"status":"observed","nodes":graph.nodes})))
}

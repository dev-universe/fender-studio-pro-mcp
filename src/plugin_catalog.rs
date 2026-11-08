use quick_xml::{Reader, events::Event};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Cursor, Read},
    path::{Path, PathBuf},
};

const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_ARCHIVE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_ENTRIES: usize = 100_000;
const MAX_XML_NODES: usize = 100_000;
const MAX_CANDIDATES: usize = 16_384;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalCatalog {
    pub completeness: String,
    pub files_scanned: usize,
    pub plugins: Vec<PluginCandidates>,
    pub warnings: Vec<CatalogWarning>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCandidates {
    pub name: String,
    pub aliases: Vec<String>,
    pub class_ids: Vec<String>,
    pub category: Option<String>,
    pub candidates: Vec<ParameterCandidate>,
    pub sources: Vec<CandidateSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterCandidate {
    pub name: String,
    pub scope: CandidateScope,
    pub parameter_type: Option<String>,
    pub units: Option<String>,
    pub read_only_hint: bool,
    pub sources: Vec<CandidateSource>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateScope {
    PluginInstance,
    SelectedFocus,
    UnexpandedTemplate,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CandidateSource {
    pub path: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogWarning {
    pub source: String,
    pub message: String,
}

impl LocalCatalog {
    pub fn candidate_names(&self, plugin_name: &str, class_id: Option<&str>) -> Vec<String> {
        self.matching(plugin_name, class_id)
            .flat_map(|p| p.candidates.iter().map(|c| c.name.clone()))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn matching<'a>(
        &'a self,
        plugin_name: &'a str,
        class_id: Option<&'a str>,
    ) -> impl Iterator<Item = &'a PluginCandidates> + 'a {
        let normalized = normalize(plugin_name);
        self.plugins.iter().filter(move |p| {
            class_id.is_some_and(|id| p.class_ids.iter().any(|c| c.eq_ignore_ascii_case(id)))
                || normalize(&p.name) == normalized
                || p.aliases.iter().any(|a| normalize(a) == normalized)
        })
    }
}

pub fn discover(app_contents: &Path) -> Result<LocalCatalog, String> {
    if !app_contents.is_dir() {
        return Err("application Contents directory does not exist".into());
    }
    let root = app_contents
        .canonicalize()
        .map_err(|e| format!("cannot resolve application directory: {e}"))?;
    let mut builder = CatalogBuilder::default();
    let mut files = Vec::new();
    let mut visited = 0;
    for relative in [
        "PlugIns",
        "Extensions",
        "Presets/Fender",
        "Presets/PreSonus",
    ] {
        walk(&root.join(relative), 0, &mut visited, &mut files)?;
    }
    files.sort();
    for path in files {
        let source = relative_source(&root, &path);
        match read_metadata(&path) {
            Ok(bytes) => {
                builder.files_scanned += 1;
                let parsed = if path.extension().is_some_and(|x| x == "dspdef") {
                    parse_dsp(&bytes, &source, &mut builder)
                } else if path.file_name().is_some_and(|x| x == "default.preset") {
                    parse_preset(&bytes, &source, &mut builder)
                } else if path.extension().is_some_and(|x| x == "surfacedata") {
                    parse_remote_assignments(&bytes, &source, &mut builder)
                } else if path.components().any(|x| x.as_os_str() == "remotepages") {
                    parse_remote_page(&bytes, &source, &mut builder)
                } else {
                    parse_definition(&bytes, &source, &mut builder)
                };
                if let Err(message) = parsed {
                    builder.warnings.push(CatalogWarning { source, message });
                }
            }
            Err(message) => builder.warnings.push(CatalogWarning { source, message }),
        }
    }
    Ok(builder.finish())
}

#[derive(Default)]
struct CatalogBuilder {
    records: BTreeMap<String, PluginCandidates>,
    warnings: Vec<CatalogWarning>,
    files_scanned: usize,
}

impl CatalogBuilder {
    fn record(&mut self, name: &str, class_id: Option<&str>) -> &mut PluginCandidates {
        let key = normalize(name);
        let entry = self.records.entry(key).or_insert_with(|| PluginCandidates {
            name: name.into(),
            aliases: Vec::new(),
            class_ids: Vec::new(),
            category: None,
            candidates: Vec::new(),
            sources: Vec::new(),
        });
        if let Some(id) = class_id.filter(|x| !x.is_empty()) {
            push_unique(&mut entry.class_ids, id.into());
        }
        entry
    }

    fn finish(mut self) -> LocalCatalog {
        let mut plugins: Vec<PluginCandidates> = Vec::new();
        for mut record in self.records.into_values() {
            let existing = plugins.iter_mut().find(|p| {
                record.class_ids.iter().any(|id| p.class_ids.contains(id))
                    || p.aliases
                        .iter()
                        .any(|a| normalize(a) == normalize(&record.name))
                    || record
                        .aliases
                        .iter()
                        .any(|a| normalize(a) == normalize(&p.name))
            });
            if let Some(existing) = existing {
                push_unique(&mut existing.aliases, record.name.clone());
                for alias in record.aliases.drain(..) {
                    push_unique(&mut existing.aliases, alias);
                }
                for id in record.class_ids.drain(..) {
                    push_unique(&mut existing.class_ids, id);
                }
                if existing.category.is_none() {
                    existing.category = record.category.take();
                }
                for source in record.sources.drain(..) {
                    push_unique(&mut existing.sources, source);
                }
                for candidate in record.candidates.drain(..) {
                    add_candidate(existing, candidate);
                }
            } else {
                plugins.push(record);
            }
        }
        for p in &mut plugins {
            p.aliases.sort();
            p.class_ids.sort();
            p.sources.sort();
            p.candidates.sort_by(|a, b| a.name.cmp(&b.name));
        }
        plugins.sort_by(|a, b| a.name.cmp(&b.name));
        self.warnings.sort_by(|a, b| a.source.cmp(&b.source));
        LocalCatalog {
            completeness: "partial_metadata_candidates".into(),
            files_scanned: self.files_scanned,
            plugins,
            warnings: self.warnings,
            limitations: vec![
                "Metadata records are not the runtime installed-plugin inventory.".into(),
                "Candidates require lookup on the actual instance; absence and writability are unknown.".into(),
                "Focus parameters address the selected pad, sample, or model only.".into(),
                "Definition inheritance across files and dynamic model or sample state may add parameters.".into(),
            ],
        }
    }
}

fn normalize(value: &str) -> String {
    value
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect()
}

fn push_unique<T: PartialEq>(target: &mut Vec<T>, value: T) {
    if !target.contains(&value) {
        target.push(value);
    }
}

fn add_candidate(plugin: &mut PluginCandidates, candidate: ParameterCandidate) {
    if candidate.name.is_empty()
        || candidate.name.len() > 512
        || plugin.candidates.len() >= MAX_CANDIDATES
    {
        return;
    }
    if let Some(existing) = plugin
        .candidates
        .iter_mut()
        .find(|p| p.name == candidate.name)
    {
        for source in candidate.sources {
            push_unique(&mut existing.sources, source);
        }
        existing.read_only_hint |= candidate.read_only_hint;
        if existing.parameter_type.is_none() {
            existing.parameter_type = candidate.parameter_type;
        }
        if existing.units.is_none() {
            existing.units = candidate.units;
        }
    } else {
        plugin.candidates.push(candidate);
    }
}

fn candidate(
    name: String,
    source: CandidateSource,
    attrs: Option<&BTreeMap<String, String>>,
) -> ParameterCandidate {
    ParameterCandidate {
        scope: if name.starts_with("focus.") {
            CandidateScope::SelectedFocus
        } else if name.contains('$') {
            CandidateScope::UnexpandedTemplate
        } else {
            CandidateScope::PluginInstance
        },
        name,
        parameter_type: attrs.and_then(|a| a.get("type")).cloned(),
        units: attrs.and_then(|a| a.get("units")).cloned(),
        read_only_hint: attrs
            .and_then(|a| a.get("flags"))
            .is_some_and(|s| s.split_whitespace().any(|x| x == "readonly")),
        sources: vec![source],
    }
}

fn source(path: &str, kind: &str) -> CandidateSource {
    CandidateSource {
        path: path.into(),
        kind: kind.into(),
    }
}

fn relative_source(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn walk(
    path: &Path,
    depth: usize,
    visited: &mut usize,
    files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    if depth > 24 {
        return Err("application metadata directory depth exceeded".into());
    }
    let metadata =
        fs::symlink_metadata(path).map_err(|e| format!("cannot inspect metadata entry: {e}"))?;
    if metadata.file_type().is_symlink() {
        return Ok(());
    }
    *visited += 1;
    if *visited > MAX_ENTRIES {
        return Err("application metadata entry limit exceeded".into());
    }
    if metadata.is_dir() {
        let mut entries = fs::read_dir(path)
            .map_err(|e| format!("cannot read metadata directory: {e}"))?
            .map(|e| e.map(|e| e.path()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("cannot inspect directory entry: {e}"))?;
        entries.sort();
        for child in entries {
            walk(&child, depth + 1, visited, files)?;
        }
    } else if metadata.is_file() && selected(path) {
        files.push(path.into());
    }
    Ok(())
}

fn selected(path: &Path) -> bool {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    name == "default.preset"
        || name == "remotedevice.surfacedata"
        || path.extension().is_some_and(|e| e == "dspdef")
        || (path.extension().is_some_and(|e| e == "xml")
            && (path.components().any(|c| c.as_os_str() == "remotepages")
                || name.ends_with("def.xml")
                || matches!(
                    name.as_ref(),
                    "commondefs.xml" | "fxparams.xml" | "synthchar.xml"
                )))
}

fn read_metadata(path: &Path) -> Result<Vec<u8>, String> {
    let mut file = fs::File::open(path).map_err(|e| format!("cannot open metadata: {e}"))?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("cannot read metadata: {e}"))?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err("metadata file exceeds size limit".into());
    }
    Ok(bytes)
}

#[derive(Debug, Clone, Default)]
struct XmlNode {
    name: String,
    attrs: BTreeMap<String, String>,
    children: Vec<XmlNode>,
}

fn parse_xml(bytes: &[u8]) -> Result<XmlNode, String> {
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().trim_text(true);
    let mut stack = vec![XmlNode::default()];
    let mut nodes = 0;
    loop {
        match reader.read_event() {
            Ok(event @ (Event::Start(_) | Event::Empty(_))) => {
                let empty = matches!(event, Event::Empty(_));
                let e = match event {
                    Event::Start(e) | Event::Empty(e) => e,
                    _ => unreachable!(),
                };
                nodes += 1;
                if nodes > MAX_XML_NODES || stack.len() > 128 {
                    return Err("XML metadata structure exceeds limit".into());
                }
                let name = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                let mut attrs = BTreeMap::new();
                for attr in e.attributes() {
                    let attr = attr.map_err(|e| format!("invalid XML attribute: {e}"))?;
                    let key = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
                    let value = attr
                        .decoded_and_normalized_value(
                            quick_xml::XmlVersion::Implicit1_0,
                            reader.decoder(),
                        )
                        .map_err(|e| format!("invalid XML value: {e}"))?;
                    attrs.insert(key, value.into_owned());
                }
                let node = XmlNode {
                    name,
                    attrs,
                    children: Vec::new(),
                };
                if empty {
                    stack
                        .last_mut()
                        .ok_or("invalid XML stack")?
                        .children
                        .push(node);
                } else {
                    stack.push(node);
                }
            }
            Ok(Event::End(_)) => {
                if stack.len() < 2 {
                    return Err("unbalanced XML metadata".into());
                }
                let node = stack.pop().ok_or("invalid XML stack")?;
                stack
                    .last_mut()
                    .ok_or("invalid XML stack")?
                    .children
                    .push(node);
            }
            Ok(Event::DocType(_)) => {
                return Err("XML document type declarations are not accepted".into());
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("invalid XML metadata: {e}")),
            _ => {}
        }
    }
    if stack.len() != 1 {
        return Err("unclosed XML metadata element".into());
    }
    stack.pop().ok_or_else(|| "empty XML parser stack".into())
}

fn visit(node: &XmlNode, f: &mut impl FnMut(&XmlNode)) {
    f(node);
    for child in &node.children {
        visit(child, f);
    }
}

fn stem(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

fn parse_remote_page(bytes: &[u8], path: &str, builder: &mut CatalogBuilder) -> Result<(), String> {
    let root = parse_xml(bytes)?;
    let src = source(path, "remote_page");
    let record = builder.record(&stem(path), None);
    push_unique(&mut record.sources, src.clone());
    visit(&root, &mut |node| {
        if let Some(name) = node.attrs.get("param") {
            add_candidate(record, candidate(name.clone(), src.clone(), None));
        }
    });
    Ok(())
}

fn parse_remote_assignments(
    bytes: &[u8],
    path: &str,
    builder: &mut CatalogBuilder,
) -> Result<(), String> {
    let root = parse_xml(bytes)?;
    let src = source(path, "remote_assignment");
    visit(&root, &mut |node| {
        if node.name != "SurfaceDeviceAssignment" {
            return;
        }
        let Some(name) = node.attrs.get("friendlyName") else {
            return;
        };
        let class_id = node.attrs.get("deviceID").map(String::as_str);
        let record = builder.record(name, class_id);
        push_unique(&mut record.sources, src.clone());
        visit(node, &mut |n| {
            if n.name == "Association"
                && let Some((_, param)) = n.attrs.get("value").and_then(|v| v.split_once('/'))
            {
                add_candidate(record, candidate(param.into(), src.clone(), None));
            }
        });
    });
    Ok(())
}

fn parse_dsp(bytes: &[u8], path: &str, builder: &mut CatalogBuilder) -> Result<(), String> {
    let json: Value =
        serde_json::from_slice(bytes).map_err(|e| format!("invalid DSP JSON: {e}"))?;
    let name = json
        .get("classname")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if name.is_empty() {
        return Ok(());
    }
    let record = builder.record(name, json.get("cid").and_then(Value::as_str));
    if let Some(id) = json.get("altClassId").and_then(Value::as_str) {
        push_unique(&mut record.class_ids, id.into());
    }
    push_unique(&mut record.aliases, stem(path));
    record.category = json
        .get("category")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let src = source(path, "dsp_definition");
    push_unique(&mut record.sources, src.clone());
    if let Some(units) = json.get("units").and_then(Value::as_array) {
        for unit in units {
            let prefix = unit.get("id").and_then(Value::as_str).unwrap_or_default();
            if let Some(parameters) = unit.get("parameters").and_then(Value::as_array) {
                for param in parameters {
                    if let Some(id) = param.get("id").and_then(Value::as_str) {
                        let name = dotted(prefix, id);
                        let mut c = candidate(name, src.clone(), None);
                        c.parameter_type =
                            param.get("type").and_then(Value::as_str).map(str::to_owned);
                        add_candidate(record, c);
                    }
                }
            }
        }
    }
    for key in ["gainReductionRole", "processingOrderRole"] {
        if let Some(value) = json.get(key) {
            match value {
                Value::String(name) => {
                    add_candidate(record, candidate(name.clone(), src.clone(), None))
                }
                Value::Array(items) => {
                    for name in items.iter().filter_map(Value::as_str) {
                        add_candidate(record, candidate(name.into(), src.clone(), None));
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn parse_preset(bytes: &[u8], path: &str, builder: &mut CatalogBuilder) -> Result<(), String> {
    let mut archive =
        zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| format!("invalid preset ZIP: {e}"))?;
    if archive.len() > 256 {
        return Err("preset ZIP entry limit exceeded".into());
    }
    let mut total = 0_u64;
    let mut members = BTreeMap::new();
    for index in 0..archive.len() {
        let mut member = archive
            .by_index(index)
            .map_err(|e| format!("cannot read preset ZIP entry: {e}"))?;
        total = total
            .checked_add(member.size())
            .ok_or("preset size overflow")?;
        if total > MAX_ARCHIVE_BYTES || member.size() > MAX_FILE_BYTES {
            return Err("preset uncompressed size limit exceeded".into());
        }
        let name = member.name().to_owned();
        if name == "metainfo.xml" || name.starts_with("data.") {
            let mut content = Vec::new();
            member
                .by_ref()
                .take(MAX_FILE_BYTES + 1)
                .read_to_end(&mut content)
                .map_err(|e| format!("cannot read preset data: {e}"))?;
            if content.len() as u64 > MAX_FILE_BYTES {
                return Err("preset member exceeds size limit".into());
            }
            members.insert(name, content);
        }
    }
    let mut metadata = BTreeMap::new();
    if let Some(bytes) = members.get("metainfo.xml") {
        let root = parse_xml(bytes)?;
        visit(&root, &mut |n| {
            if n.name == "Attribute"
                && let (Some(id), Some(value)) = (n.attrs.get("id"), n.attrs.get("value"))
            {
                metadata.insert(id.clone(), value.clone());
            }
        });
    }
    let fallback = Path::new(path)
        .parent()
        .and_then(Path::file_name)
        .unwrap_or_default()
        .to_string_lossy();
    let name = metadata
        .get("Class:Name")
        .map(String::as_str)
        .unwrap_or(&fallback);
    let record = builder.record(name, metadata.get("Class:ID").map(String::as_str));
    record.category = metadata
        .get("Class:Category")
        .cloned()
        .or(record.category.take());
    for (member, bytes) in members {
        if member == "metainfo.xml" {
            continue;
        }
        let src = source(&format!("{path}!/{member}"), "default_preset");
        push_unique(&mut record.sources, src.clone());
        let trimmed = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes);
        if trimmed.iter().copied().find(|b| !b.is_ascii_whitespace()) == Some(b'{') {
            let json: Value =
                serde_json::from_slice(trimmed).map_err(|e| format!("invalid preset JSON: {e}"))?;
            if let Some(parameters) = json.get("parameters") {
                json_parameters(parameters, "", &src, record, 0);
            }
        } else {
            let root = parse_xml(trimmed)?;
            preset_parameters(&root, "", false, &src, record);
        }
    }
    Ok(())
}

fn json_parameters(
    value: &Value,
    prefix: &str,
    src: &CandidateSource,
    record: &mut PluginCandidates,
    depth: usize,
) {
    if depth > 32 {
        return;
    }
    if let Value::Object(fields) = value {
        for (name, value) in fields {
            let full = dotted(prefix, name);
            if value.is_object() {
                json_parameters(value, &full, src, record, depth + 1);
            } else if value.is_number() || value.is_boolean() || value.is_string() {
                add_candidate(record, candidate(full, src.clone(), None));
            }
        }
    }
}

fn preset_parameters(
    node: &XmlNode,
    prefix: &str,
    active: bool,
    src: &CandidateSource,
    record: &mut PluginCandidates,
) {
    let id = node.attrs.get("x:id").or_else(|| node.attrs.get("id"));
    let wrapper = id.is_some_and(|id| id == "ParameterData" || id == "ComponentData");
    let active = active || wrapper;
    let prefix = if active && node.name == "Attributes" && !wrapper {
        id.map(|id| dotted(prefix, id))
            .unwrap_or_else(|| prefix.into())
    } else {
        prefix.into()
    };
    if active && node.name == "Attributes" {
        for key in node
            .attrs
            .keys()
            .filter(|key| !key.contains(':') && key.as_str() != "id")
        {
            add_candidate(record, candidate(dotted(&prefix, key), src.clone(), None));
        }
    }
    for child in node
        .children
        .iter()
        .filter(|c| !active || c.name == "Attributes")
    {
        preset_parameters(child, &prefix, active, src, record);
    }
}

fn dotted(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.into()
    } else {
        format!("{prefix}.{name}")
    }
}

fn parse_definition(bytes: &[u8], path: &str, builder: &mut CatalogBuilder) -> Result<(), String> {
    let root = parse_xml(bytes)?;
    let src = source(path, "component_definition");
    let filename = stem(path);
    let alias = match filename.as_str() {
        "polysynthdef" => "Mai Tai",
        "samplerdef" => "SampleOne",
        "cvsynthdef" => "CV Instrument",
        "impactdef" => "Impact",
        "inputfilterdef" => "Input Filter",
        "arpeggiatordef" => "Arpeggiator",
        "chorderdef" => "Chorder",
        "echodef" => "Repeater",
        _ => &filename,
    };
    let mut lists = BTreeMap::new();
    let mut templates = Vec::new();
    visit(&root, &mut |n| {
        if n.name == "ParamList"
            && let Some(id) = n.attrs.get("id")
        {
            lists.insert(id.clone(), n.clone());
        }
        if n.name == "Template" {
            templates.push(n.clone());
        }
    });
    let record = builder.record(alias, None);
    push_unique(&mut record.aliases, filename);
    push_unique(&mut record.sources, src.clone());
    for template in &templates {
        let id = template
            .attrs
            .get("id")
            .map(String::as_str)
            .unwrap_or_default();
        if id.starts_with('{') || id == "main" {
            if id.starts_with('{') {
                push_unique(&mut record.class_ids, id.into());
            }
            definition_parameters(template, "", &lists, &src, record, 0);
        }
    }
    Ok(())
}

fn definition_parameters(
    node: &XmlNode,
    prefix: &str,
    lists: &BTreeMap<String, XmlNode>,
    src: &CandidateSource,
    record: &mut PluginCandidates,
    depth: usize,
) {
    if depth > 16 {
        return;
    }
    let prefix = if node.name == "Component" {
        node.attrs
            .get("id")
            .map(|id| dotted(prefix, id))
            .unwrap_or_else(|| prefix.into())
    } else {
        prefix.into()
    };
    if let Some(list) = node.attrs.get("paramlist").and_then(|id| lists.get(id)) {
        definition_parameters(list, &prefix, lists, src, record, depth + 1);
    }
    if node.name == "ParamList"
        && let Some(list) = node.attrs.get("inherit").and_then(|id| lists.get(id))
    {
        definition_parameters(list, &prefix, lists, src, record, depth + 1);
    }
    if matches!(node.name.as_str(), "Param" | "ParamAlias")
        && let Some(id) = node.attrs.get("id")
    {
        add_candidate(
            record,
            candidate(dotted(&prefix, id), src.clone(), Some(&node.attrs)),
        );
    }
    for child in &node.children {
        definition_parameters(child, &prefix, lists, src, record, depth + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    #[test]
    fn combines_remote_dsp_and_preset_without_claiming_completeness() {
        let mut builder = CatalogBuilder::default();
        parse_dsp(br#"{"cid":"{new}","altClassId":"{old}","classname":"Test EQ","units":[{"id":"opt","parameters":[{"id":"active","type":"toggle"}]}]}"#, "PlugIns/testeq.dspdef", &mut builder).unwrap();
        parse_remote_page(br#"<RemotePages><Slot param="band.frequency"/><Slot param="band.frequency"/></RemotePages>"#, "PlugIns/remotepages/testeq.xml", &mut builder).unwrap();
        let mut bytes = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut bytes);
            zip.start_file("metainfo.xml", SimpleFileOptions::default())
                .unwrap();
            zip.write_all(br#"<MetaInformation><Attribute id="Class:ID" value="{old}"/><Attribute id="Class:Name" value="Test EQ"/></MetaInformation>"#).unwrap();
            zip.start_file("data.fxpreset", SimpleFileOptions::default())
                .unwrap();
            zip.write_all(br#"<AudioEffectPreset><Attributes x:id="ParameterData" gain="1"/></AudioEffectPreset>"#).unwrap();
            zip.finish().unwrap();
        }
        parse_preset(
            bytes.get_ref(),
            "Presets/Fender/Test EQ/default.preset",
            &mut builder,
        )
        .unwrap();
        let catalog = builder.finish();
        assert_eq!(catalog.completeness, "partial_metadata_candidates");
        assert_eq!(
            catalog.candidate_names("TEST EQ", Some("{new}")),
            vec!["band.frequency", "gain", "opt.active"]
        );
        assert_eq!(catalog.plugins.len(), 1);
    }

    #[test]
    fn marks_selected_pad_scope_and_definition_readonly_hints() {
        let mut builder = CatalogBuilder::default();
        parse_remote_page(
            br#"<RemotePages><Slot param="focus.filter.cutoff"/></RemotePages>"#,
            "PlugIns/remotepages/impact.xml",
            &mut builder,
        )
        .unwrap();
        parse_definition(br#"<ComponentModel><Parameters><ParamList id="main"><Param id="meter" flags="readonly"/><Param id="gain" type="float"/></ParamList></Parameters><Templates><Template id="{synth}" paramlist="main"><Component id="osc1"><ParamList><Param id="tune"/></ParamList></Component></Template></Templates></ComponentModel>"#, "PlugIns/testdef.xml", &mut builder).unwrap();
        let catalog = builder.finish();
        let impact = catalog.matching("Impact", None).next().unwrap();
        assert_eq!(impact.candidates[0].scope, CandidateScope::SelectedFocus);
        let synth = catalog.matching("unknown", Some("{synth}")).next().unwrap();
        assert!(synth.candidates.iter().any(|p| p.name == "osc1.tune"));
        assert!(
            synth
                .candidates
                .iter()
                .find(|p| p.name == "meter")
                .unwrap()
                .read_only_hint
        );
    }

    #[test]
    fn extracts_nested_component_and_json_presets_without_values() {
        let xml = parse_xml(br#"<Preset><Attributes x:id="ComponentData" bypass="0"><Attributes x:id="osc1" tune="2"><Attributes x:id="amp" gain="0.5"/></Attributes></Attributes></Preset>"#).unwrap();
        let mut builder = CatalogBuilder::default();
        let record = builder.record("Synth", None);
        preset_parameters(&xml, "", false, &source("synthetic", "test"), record);
        json_parameters(
            &serde_json::json!({"filter":{"cutoff":200,"active":true}}),
            "",
            &source("synthetic", "test"),
            record,
            0,
        );
        let catalog = builder.finish();
        assert_eq!(
            catalog.candidate_names("Synth", None),
            vec![
                "bypass",
                "filter.active",
                "filter.cutoff",
                "osc1.amp.gain",
                "osc1.tune"
            ]
        );
        let encoded = serde_json::to_string(&catalog).unwrap();
        assert!(!encoded.contains("0.5"));
    }

    #[test]
    fn rejects_entities_and_does_not_follow_resource_symlinks() {
        assert!(parse_xml(br#"<!DOCTYPE x [<!ENTITY x "bad">]><x/>"#).is_err());
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path().join("PlugIns")).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("/Applications", temp.path().join("PlugIns/escape")).unwrap();
        let catalog = discover(temp.path()).unwrap();
        assert_eq!(catalog.files_scanned, 0);
        assert!(catalog.plugins.is_empty());
    }
}

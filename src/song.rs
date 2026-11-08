use quick_xml::{Reader, events::Event};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::Read,
    path::Path,
    time::UNIX_EPOCH,
};
use zip::ZipArchive;

const MAX_ARCHIVE_BYTES: u64 = 128 * 1024 * 1024;
const MAX_ENTRY_BYTES: u64 = 16 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 512 * 1024 * 1024;
const MAX_ENTRIES: usize = 16_384;
const MAX_XML_NODES: usize = 200_000;
const MAX_XML_DEPTH: usize = 96;
const MAX_JSON_DEPTH: usize = 64;

#[derive(Debug, Clone, Serialize)]
pub struct SavedSong {
    pub source: String,
    pub title: String,
    pub generator: Option<String>,
    pub modified_unix_ms: Option<u64>,
    pub sample_rate: Option<f64>,
    pub bit_depth: Option<u32>,
    pub length_seconds: Option<f64>,
    pub tempo: Vec<SavedTempo>,
    pub time_signatures: Vec<SavedMeter>,
    pub tracks: Vec<SavedTrack>,
    pub mixer: Vec<SavedChannel>,
    pub plugins: Vec<SavedPlugin>,
    pub transport: BTreeMap<String, Value>,
    pub transport_time_unit: String,
    pub notes: Option<String>,
    pub unsupported_entries: Vec<String>,
    pub warnings: Vec<String>,
    pub limitations: Vec<String>,
    pub archive: ArchiveInfo,
}

#[derive(Debug, Clone, Serialize)]
pub struct SavedTempo {
    pub at_quarter_note_beat: f64,
    pub seconds_per_quarter_note: f64,
    pub bpm: f64,
    pub curve_type: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct SavedMeter {
    pub at_quarter_note_beat: f64,
    pub numerator: u32,
    pub denominator: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct SavedTrack {
    pub track_type: String,
    pub name: String,
    pub media_type: Option<String>,
    pub track_id: Option<String>,
    pub channel_id: Option<String>,
    pub hidden: bool,
    pub active_layer: Option<u32>,
    pub layers: Vec<SavedLayer>,
    pub events: Vec<SavedEvent>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SavedLayer {
    pub id: u32,
    pub name: String,
    pub active: bool,
    pub events: Vec<SavedEvent>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SavedEvent {
    pub event_type: String,
    pub name: String,
    pub start: f64,
    pub length: Option<f64>,
    pub time_unit: String,
    pub clip_id: Option<String>,
    pub muted: bool,
    pub attributes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SavedChannel {
    pub kind: String,
    pub group: String,
    pub label: String,
    pub unique_id: Option<String>,
    pub linear_gain: Option<f64>,
    pub volume_db: Option<f64>,
    pub pan_normalized: Option<f64>,
    pub mute: bool,
    pub solo: bool,
    pub output: Option<String>,
    pub automation_mode: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SavedPlugin {
    pub kind: String,
    pub name: String,
    pub channel: Option<String>,
    pub slot: Option<u32>,
    pub slot_name: Option<String>,
    pub class_id: Option<String>,
    pub instance_id: Option<String>,
    pub slot_id: Option<String>,
    pub source_entry: String,
    pub preset_path: Option<String>,
    pub preset_format: Option<String>,
    pub state_status: String,
    pub parameter_source: String,
    pub parameter_completeness: String,
    pub parameter_units: String,
    pub parameters: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArchiveInfo {
    pub entries: usize,
    pub compressed_bytes: u64,
    pub declared_uncompressed_bytes: u64,
    pub metadata_bytes_read: u64,
}

#[derive(Debug, Default)]
struct XmlNode {
    name: String,
    attrs: BTreeMap<String, String>,
    children: Vec<XmlNode>,
}

impl XmlNode {
    fn attr(&self, key: &str) -> Option<&str> {
        self.attrs.get(key).map(String::as_str)
    }

    fn child_id(&self, id: &str) -> Option<&Self> {
        self.children.iter().find(|n| n.attr("x:id") == Some(id))
    }

    fn descendant_id(&self, id: &str) -> Option<&Self> {
        if self.attr("x:id") == Some(id) {
            return Some(self);
        }
        self.children.iter().find_map(|n| n.descendant_id(id))
    }

    fn uid(&self, id: &str) -> Option<String> {
        self.child_id(id)?.attr("uid").map(str::to_owned)
    }
}

struct SongArchive {
    modified_unix_ms: Option<u64>,
    zip: ZipArchive<File>,
    names: BTreeSet<String>,
    info: ArchiveInfo,
}

impl SongArchive {
    fn open(path: &Path) -> Result<Self, String> {
        let file = File::open(path).map_err(|e| format!("cannot open song: {e}"))?;
        let metadata = file.metadata().map_err(|e| e.to_string())?;
        let size = metadata.len();
        let modified_unix_ms = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .and_then(|d| u64::try_from(d.as_millis()).ok());
        if size > MAX_ARCHIVE_BYTES {
            return Err("song archive exceeds the 128 MiB compressed limit".into());
        }
        let mut zip = ZipArchive::new(file).map_err(|e| format!("invalid song ZIP: {e}"))?;
        if zip.len() > MAX_ENTRIES {
            return Err("song archive exceeds the entry-count limit".into());
        }
        let mut names = BTreeSet::new();
        let mut total = 0u64;
        for i in 0..zip.len() {
            let entry = zip
                .by_index(i)
                .map_err(|e| format!("invalid ZIP entry: {e}"))?;
            let name = entry.name();
            if !safe_entry_name(name) {
                return Err("song archive contains an unsafe member path".into());
            }
            if !names.insert(name.to_owned()) {
                return Err("song archive contains duplicate member paths".into());
            }
            total = total.checked_add(entry.size()).ok_or("ZIP size overflow")?;
            if total > MAX_TOTAL_BYTES {
                return Err("song archive exceeds the 512 MiB uncompressed limit".into());
            }
        }
        Ok(Self {
            modified_unix_ms,
            info: ArchiveInfo {
                entries: zip.len(),
                compressed_bytes: size,
                declared_uncompressed_bytes: total,
                metadata_bytes_read: 0,
            },
            zip,
            names,
        })
    }

    fn bytes(&mut self, name: &str) -> Result<Option<Vec<u8>>, String> {
        if !self.names.contains(name) {
            return Ok(None);
        }
        let entry = self
            .zip
            .by_name(name)
            .map_err(|e| format!("cannot read ZIP member: {e}"))?;
        if entry.size() > MAX_ENTRY_BYTES {
            return Err(format!("metadata member {name} exceeds 16 MiB"));
        }
        let mut bytes = Vec::new();
        entry
            .take(MAX_ENTRY_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| format!("cannot decompress {name}: {e}"))?;
        if bytes.len() as u64 > MAX_ENTRY_BYTES {
            return Err(format!("metadata member {name} exceeds 16 MiB"));
        }
        self.info.metadata_bytes_read = self
            .info
            .metadata_bytes_read
            .checked_add(bytes.len() as u64)
            .ok_or("metadata size overflow")?;
        if self.info.metadata_bytes_read > MAX_TOTAL_BYTES {
            return Err("metadata read exceeds the uncompressed limit".into());
        }
        Ok(Some(bytes))
    }

    fn xml(&mut self, name: &str) -> Result<Option<XmlNode>, String> {
        self.bytes(name)?
            .map(|bytes| parse_xml(&bytes).map_err(|e| format!("invalid {name}: {e}")))
            .transpose()
    }
}

pub fn read_song(path: &Path) -> Result<SavedSong, String> {
    let mut archive = SongArchive::open(path)?;
    let song = archive
        .xml("Song/song.xml")?
        .ok_or("not a Studio Pro song: Song/song.xml is missing")?;
    let meta = archive.xml("metainfo.xml")?;
    let metadata = meta.as_ref().map(metadata_values).unwrap_or_default();
    let root = song.descendant_id("Root").unwrap_or(&song);
    let time = root.child_id("timeContext");
    let mut warnings = Vec::new();
    let tempo = read_tempo(time, &mut warnings);
    let time_signatures = read_meter(time);
    let tracks = root
        .child_id("Tracks")
        .map(|n| n.children.iter().map(read_track).collect())
        .unwrap_or_default();
    let transport = archive
        .xml("Devices/transportdevice.xml")?
        .map(|n| n.attrs.into_iter().map(|(k, v)| (k, scalar(&v))).collect())
        .unwrap_or_default();
    let notes = archive
        .bytes("notes.txt")?
        .map(|bytes| String::from_utf8(bytes).map(|s| s.trim_start_matches('\u{feff}').to_owned()))
        .transpose()
        .map_err(|_| "notes.txt is not UTF-8")?;
    let mut mixer = Vec::new();
    let mut plugins = Vec::new();
    let device_entries: Vec<_> = archive
        .names
        .iter()
        .filter(|n| n.starts_with("Devices/") && n.ends_with(".xml"))
        .cloned()
        .collect();
    for entry in device_entries {
        if let Some(doc) = archive.xml(&entry)? {
            if entry == "Devices/audiomixer.xml" {
                read_channels(&doc, "", &mut mixer);
            }
            scan_plugins(&doc, &entry, None, "", &mut plugins);
        }
    }
    for plugin in &mut plugins {
        if let Some(preset_path) = plugin.preset_path.clone() {
            match archive.bytes(&preset_path) {
                Ok(Some(bytes)) => match read_plugin_state(&bytes) {
                    Ok((format, values)) => {
                        plugin.preset_format = Some(format);
                        plugin.state_status = if values.is_empty() {
                            "no_parameter_data"
                        } else {
                            "parsed"
                        }
                        .into();
                        plugin.parameters = values;
                    }
                    Err(reason) => {
                        plugin.state_status = "unparsed".into();
                        warnings.push(format!(
                            "{} state could not be parsed: {reason}",
                            plugin.name
                        ));
                    }
                },
                Ok(None) => {
                    plugin.state_status = "missing".into();
                    warnings.push(format!("{} preset member is missing", plugin.name));
                }
                Err(reason) => return Err(reason),
            }
        }
    }
    let unsupported_entries = archive
        .names
        .iter()
        .filter(|n| n.ends_with(".musicx") || n.ends_with(".envelopex"))
        .cloned()
        .collect::<Vec<_>>();
    if !unsupported_entries.is_empty() {
        warnings.push(
            "Binary note performances and automation envelopes are listed but not decoded".into(),
        );
    }
    Ok(SavedSong {
        source: "saved".into(),
        title: metadata.get("Document:Title").cloned().unwrap_or_else(|| path.file_stem().unwrap_or_default().to_string_lossy().into_owned()),
        generator: metadata.get("Document:Generator").cloned(),
        modified_unix_ms: archive.modified_unix_ms,
        sample_rate: metadata.get("Media:SampleRate").and_then(|s| finite_number(s)),
        bit_depth: metadata.get("Media:BitDepth").and_then(|s| s.parse().ok()),
        length_seconds: metadata.get("Media:Length").and_then(|s| finite_number(s)).or_else(|| number(root, "length")),
        tempo, time_signatures, tracks, mixer, plugins, transport,
        transport_time_unit: "seconds".into(), notes, unsupported_entries, warnings,
        limitations: vec![
            "This is the last saved document, not live DAW state".into(),
            "Plugin parameters cover persisted state only; non-storable meters, actions and hidden controls are not enumerated".into(),
            "Preset values use preset-native units and must not be sent as live raw values without host validation".into(),
            "MIDI performance and automation UBJSON chunks are not decoded".into(),
            "Event positions retain their stored units; tempo ramps are not approximated or converted".into(),
        ],
        archive: archive.info,
    })
}

fn safe_entry_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('/')
        && !name.contains('\\')
        && !name.contains('\0')
        && !name.split('/').any(|p| p == ".." || p.contains(':'))
}

fn parse_xml(bytes: &[u8]) -> Result<XmlNode, String> {
    std::str::from_utf8(bytes).map_err(|_| "XML metadata is not UTF-8")?;
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().trim_text(true);
    let mut stack = vec![XmlNode::default()];
    let mut count = 0usize;
    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            event @ (Event::Start(_) | Event::Empty(_)) => {
                let empty = matches!(&event, Event::Empty(_));
                let tag = match event {
                    Event::Start(tag) | Event::Empty(tag) => tag,
                    _ => unreachable!(),
                };
                count += 1;
                if count > MAX_XML_NODES || stack.len() > MAX_XML_DEPTH {
                    return Err("XML node or nesting limit exceeded".into());
                }
                let mut node = XmlNode {
                    name: String::from_utf8_lossy(tag.name().as_ref()).into_owned(),
                    ..XmlNode::default()
                };
                for attr in tag.attributes() {
                    let attr = attr.map_err(|e| e.to_string())?;
                    node.attrs.insert(
                        String::from_utf8_lossy(attr.key.as_ref()).into_owned(),
                        attr.decoded_and_normalized_value(
                            quick_xml::XmlVersion::Implicit1_0,
                            reader.decoder(),
                        )
                        .map_err(|e| e.to_string())?
                        .into_owned(),
                    );
                }
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
            Event::End(_) => {
                if stack.len() < 2 {
                    return Err("unexpected XML closing tag".into());
                }
                let node = stack.pop().ok_or("invalid XML stack")?;
                stack
                    .last_mut()
                    .ok_or("invalid XML stack")?
                    .children
                    .push(node);
            }
            Event::DocType(_) => return Err("DTD declarations are not supported".into()),
            Event::Eof => break,
            _ => {}
        }
    }
    if stack.len() != 1 {
        return Err("unclosed XML element".into());
    }
    let mut doc = stack.pop().ok_or("empty XML")?;
    if doc.children.len() != 1 {
        return Err("XML must contain one root element".into());
    }
    Ok(doc.children.remove(0))
}

fn metadata_values(node: &XmlNode) -> BTreeMap<String, String> {
    node.children
        .iter()
        .filter_map(|n| Some((n.attr("id")?.to_owned(), n.attr("value")?.to_owned())))
        .collect()
}

fn read_tempo(time: Option<&XmlNode>, warnings: &mut Vec<String>) -> Vec<SavedTempo> {
    time.and_then(|n| n.child_id("tempoMap"))
        .into_iter()
        .flat_map(|n| &n.children)
        .filter(|n| n.name == "TempoMapSegment")
        .filter_map(|n| {
            let seconds = number(n, "tempo")?;
            if seconds <= 0.0 {
                warnings.push("Invalid non-positive tempo segment was skipped".into());
                return None;
            }
            let bpm = 60.0 / seconds;
            if !bpm.is_finite() {
                warnings.push("Unrepresentable tempo segment was skipped".into());
                return None;
            }
            Some(SavedTempo {
                at_quarter_note_beat: number(n, "start").unwrap_or(0.0),
                seconds_per_quarter_note: seconds,
                bpm,
                curve_type: unsigned(n, "curveType").unwrap_or(0),
            })
        })
        .collect()
}

fn read_meter(time: Option<&XmlNode>) -> Vec<SavedMeter> {
    time.and_then(|n| n.child_id("timeSignatureMap"))
        .into_iter()
        .flat_map(|n| &n.children)
        .filter(|n| n.name == "TimeSignatureMapSegment")
        .filter_map(|n| {
            Some(SavedMeter {
                at_quarter_note_beat: number(n, "start").unwrap_or(0.0),
                numerator: unsigned(n, "numerator")?,
                denominator: unsigned(n, "denominator")?,
            })
        })
        .collect()
}

fn read_track(node: &XmlNode) -> SavedTrack {
    let active_layer = unsigned(node, "activeLayer");
    let time_format = node.attr("timeFormat").unwrap_or("2");
    let layers = node
        .child_id("Layers")
        .map(|n| {
            n.children
                .iter()
                .enumerate()
                .map(|(i, layer)| {
                    let id = unsigned(layer, "id").unwrap_or(i as u32);
                    SavedLayer {
                        id,
                        name: layer.attr("layerName").unwrap_or("").into(),
                        active: id == active_layer.unwrap_or(0),
                        events: read_events(layer, time_format),
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let events = if layers.is_empty() {
        read_events(node, time_format)
    } else {
        layers
            .iter()
            .find(|l| l.active)
            .map(|l| l.events.clone())
            .unwrap_or_default()
    };
    SavedTrack {
        track_type: node.name.clone(),
        name: node.attr("name").unwrap_or("").into(),
        media_type: node.attr("mediaType").map(str::to_owned),
        track_id: node.attr("trackID").map(str::to_owned),
        channel_id: node.uid("channelID"),
        hidden: node
            .child_id("attributes")
            .is_some_and(|n| n.attr("hidden") == Some("1")),
        active_layer,
        layers,
        events,
    }
}

fn read_events(node: &XmlNode, inherited_format: &str) -> Vec<SavedEvent> {
    let mut events = Vec::new();
    for child in &node.children {
        if child.name.ends_with("Event")
            || matches!(child.name.as_str(), "MusicPart" | "AudioPart" | "Part")
        {
            let format = child.attr("timeFormat").unwrap_or(inherited_format);
            events.push(SavedEvent {
                event_type: child.name.clone(),
                name: child.attr("name").unwrap_or("").into(),
                start: number(child, "start").unwrap_or(0.0),
                length: number(child, "length"),
                time_unit: time_unit(format).into(),
                clip_id: child.attr("clipID").map(str::to_owned),
                muted: child.attr("mute") == Some("1"),
                attributes: child.attrs.clone(),
            });
        } else if child.attr("x:id") == Some("Events") || child.name == "List" {
            events.extend(read_events(child, inherited_format));
        }
    }
    events
}

fn time_unit(format: &str) -> &str {
    match format {
        "0" => "seconds",
        "1" => "samples",
        "2" => "quarter_note_beats",
        _ => "unknown",
    }
}

fn read_channels(node: &XmlNode, inherited_group: &str, output: &mut Vec<SavedChannel>) {
    let group = if node.name == "ChannelGroup" {
        node.attr("name").unwrap_or(inherited_group)
    } else {
        inherited_group
    };
    if node.name.ends_with("Channel") {
        let gain = number(node, "gain");
        output.push(SavedChannel {
            kind: node.name.clone(),
            group: group.into(),
            label: node
                .attr("label")
                .or_else(|| node.attr("name"))
                .unwrap_or("")
                .into(),
            unique_id: node.uid("uniqueID"),
            linear_gain: gain,
            volume_db: gain.filter(|g| *g > 0.0).map(|g| 20.0 * g.log10()),
            pan_normalized: number(node, "pan"),
            mute: node.attr("mute") == Some("1"),
            solo: node.attr("solo") == Some("1"),
            output: node
                .child_id("destination")
                .and_then(|n| n.attr("friendlyName"))
                .map(str::to_owned),
            automation_mode: node
                .child_id("Automation")
                .and_then(|n| unsigned(n, "mode")),
        });
    }
    for child in &node.children {
        read_channels(child, group, output);
    }
}

fn scan_plugins(
    node: &XmlNode,
    entry: &str,
    inherited_channel: Option<&str>,
    inherited_rack: &str,
    output: &mut Vec<SavedPlugin>,
) {
    let channel = if node.name.ends_with("Channel") {
        node.attr("label").or_else(|| node.attr("name"))
    } else {
        inherited_channel
    };
    let rack = node
        .attr("x:id")
        .filter(|id| {
            matches!(
                *id,
                "Inserts" | "PostFaderInserts" | "Sends" | "NoteFX" | "AudioBusMixFX"
            )
        })
        .unwrap_or(inherited_rack);
    let class_info = node
        .child_id("ghostData")
        .and_then(|n| n.child_id("classInfo"));
    let preset_path = node.child_id("presetPath").and_then(|n| n.attr("text"));
    if preset_path.is_some() || node.uid("deviceClassID").is_some() {
        let device = node.child_id("deviceData");
        let class_id = node.uid("deviceClassID").or_else(|| {
            class_info
                .and_then(|n| n.attr("classID"))
                .map(str::to_owned)
        });
        let name = device
            .and_then(|n| n.attr("name"))
            .or_else(|| class_info.and_then(|n| n.attr("name")))
            .or_else(|| node.attr("name"))
            .unwrap_or("");
        let category = class_info.and_then(|n| n.attr("category"));
        let kind = if category == Some("AudioSynth") || entry == "Devices/audiosynthfolder.xml" {
            "instrument"
        } else if category == Some("MusicEffect") || rack == "NoteFX" {
            "note_fx"
        } else if rack == "AudioBusMixFX" {
            "mix_fx"
        } else if rack == "PostFaderInserts" {
            "post_fader_insert"
        } else if rack == "Sends" {
            "send"
        } else {
            "insert"
        };
        let slot_name = node.attr("name").map(str::to_owned);
        let slot = slot_name
            .as_ref()
            .and_then(|s| {
                s.trim_start_matches(|c: char| !c.is_ascii_digit())
                    .parse::<u32>()
                    .ok()
            })
            .and_then(|i| i.checked_sub(1));
        output.push(SavedPlugin {
            kind: kind.into(),
            name: name.into(),
            channel: channel.map(str::to_owned),
            slot,
            slot_name,
            class_id,
            instance_id: device.and_then(|n| n.uid("uniqueID")),
            slot_id: node.uid("uniqueID"),
            source_entry: entry.into(),
            preset_path: preset_path.map(str::to_owned),
            preset_format: None,
            state_status: "not_stored".into(),
            parameter_source: "persisted_plugin_state".into(),
            parameter_completeness: "persisted_subset".into(),
            parameter_units: "preset_native_units_not_live_raw".into(),
            parameters: BTreeMap::new(),
        });
    }
    for child in &node.children {
        scan_plugins(child, entry, channel, rack, output);
    }
}

fn read_plugin_state(bytes: &[u8]) -> Result<(String, BTreeMap<String, Value>), String> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| "opaque binary plugin state")?
        .trim_start_matches('\u{feff}')
        .trim_start();
    if text.starts_with('{') {
        let value: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let mut parameters = BTreeMap::new();
        if let Some(values) = value.get("parameters") {
            flatten_json(values, "", 0, &mut parameters)?;
        }
        Ok(("json".into(), parameters))
    } else if text.starts_with('<') {
        let root = parse_xml(text.as_bytes())?;
        let mut parameters = BTreeMap::new();
        collect_xml_parameters(&root, &mut parameters)?;
        Ok(("xml".into(), parameters))
    } else {
        Err("opaque binary or unrecognized plugin state".into())
    }
}

fn flatten_json(
    value: &Value,
    prefix: &str,
    depth: usize,
    output: &mut BTreeMap<String, Value>,
) -> Result<(), String> {
    if depth > MAX_JSON_DEPTH {
        return Err("JSON parameter nesting limit exceeded".into());
    }
    if let Value::Object(values) = value {
        for (key, value) in values {
            if key.starts_with("__") {
                continue;
            }
            let name = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{prefix}.{key}")
            };
            flatten_json(value, &name, depth + 1, output)?;
        }
    } else if !prefix.is_empty() && output.insert(prefix.into(), value.clone()).is_some() {
        return Err("parameter names collide after dotted-path flattening".into());
    }
    Ok(())
}

fn collect_xml_parameters(
    node: &XmlNode,
    output: &mut BTreeMap<String, Value>,
) -> Result<(), String> {
    if node.attr("x:id") == Some("ParameterData") {
        for (key, value) in &node.attrs {
            if key == "x:id" || key.starts_with("xmlns") {
                continue;
            }
            if output.insert(key.clone(), scalar(value)).is_some() {
                return Err("scoped ParameterData blocks have duplicate parameter names".into());
            }
        }
    }
    for child in &node.children {
        collect_xml_parameters(child, output)?;
    }
    Ok(())
}

fn finite_number(text: &str) -> Option<f64> {
    text.parse::<f64>().ok().filter(|n| n.is_finite())
}
fn number(node: &XmlNode, key: &str) -> Option<f64> {
    node.attr(key).and_then(finite_number)
}
fn unsigned(node: &XmlNode, key: &str) -> Option<u32> {
    node.attr(key)?.parse().ok()
}
fn scalar(text: &str) -> Value {
    finite_number(text).map_or_else(|| Value::String(text.into()), |n| json!(n))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};
    use zip::{ZipWriter, write::SimpleFileOptions};

    fn write_archive(entries: &[(&str, &str)]) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        for (name, body) in entries {
            zip.start_file(*name, SimpleFileOptions::default()).unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        }
        file.write_all(&zip.finish().unwrap().into_inner()).unwrap();
        file
    }

    const SONG: &str = r#"<Song><Attributes x:id="Root" length="10"><Attributes x:id="timeContext"><TempoMap x:id="tempoMap"><TempoMapSegment start="0" tempo="0.5" curveType="0"/><TempoMapSegment start="8" tempo="0.6" curveType="1"/></TempoMap><TimeSignatureMap x:id="timeSignatureMap"><TimeSignatureMapSegment start="0" numerator="7" denominator="8"/></TimeSignatureMap></Attributes><List x:id="Tracks"><MediaTrack name="Keys" mediaType="Music" trackID="track-a" timeFormat="2"><UID x:id="channelID" uid="channel-a"/><List x:id="Events"><MusicPart name="Part" start="4" length="8" clipID="clip-a"/></List></MediaTrack></List></Attributes></Song>"#;

    #[test]
    fn reads_saved_units_and_marks_binary_notes_unsupported() {
        let file = write_archive(&[
            ("Song/song.xml", SONG),
            ("Performances/Keys/Part.musicx", "binary"),
            (
                "Devices/transportdevice.xml",
                r#"<TransportDevice position="1.25" loopStart="0" loopEnd="4"/>"#,
            ),
        ]);
        let song = read_song(file.path()).unwrap();
        assert_eq!(song.source, "saved");
        assert_eq!(song.tempo[0].bpm, 120.0);
        assert_eq!(song.tempo[1].curve_type, 1);
        assert_eq!(song.time_signatures[0].denominator, 8);
        assert_eq!(song.tracks[0].events[0].start, 4.0);
        assert_eq!(song.tracks[0].events[0].time_unit, "quarter_note_beats");
        assert_eq!(song.transport["position"], json!(1.25));
        assert_eq!(song.unsupported_entries.len(), 1);
    }

    #[test]
    fn plugin_identity_and_json_parameter_flattening_exclude_component_state() {
        let mixer = r#"<AudioMixer><ChannelGroup name="AudioTrack"><AudioTrackChannel label="Vox" gain="0.5" pan="0.2"><UID x:id="uniqueID" uid="channel-a"/><Attributes x:id="Inserts"><Attributes name="FX01"><UID x:id="uniqueID" uid="slot-a"/><UID x:id="deviceClassID" uid="class-a"/><Attributes x:id="deviceData" name="Equalizer"><UID x:id="uniqueID" uid="instance-a"/></Attributes><String x:id="presetPath" text="Presets/eq.dsppreset"/></Attributes></Attributes></AudioTrackChannel></ChannelGroup></AudioMixer>"#;
        let preset = r#"{"parameters":{"low":{"frequency":100,"__classid":"internal"},"enabled":true},"component":{"windowX":20}}"#;
        let file = write_archive(&[
            ("Song/song.xml", SONG),
            ("Devices/audiomixer.xml", mixer),
            ("Presets/eq.dsppreset", preset),
        ]);
        let song = read_song(file.path()).unwrap();
        let plugin = &song.plugins[0];
        assert_eq!(plugin.name, "Equalizer");
        assert_eq!(plugin.channel.as_deref(), Some("Vox"));
        assert_eq!(plugin.slot, Some(0));
        assert_eq!(plugin.class_id.as_deref(), Some("class-a"));
        assert_eq!(plugin.instance_id.as_deref(), Some("instance-a"));
        assert_eq!(plugin.parameters["low.frequency"], json!(100));
        assert_eq!(plugin.parameters.len(), 2);
        assert_eq!(plugin.parameter_completeness, "persisted_subset");
        assert_eq!(song.mixer[0].linear_gain, Some(0.5));
    }

    #[test]
    fn reads_instrument_xml_parameter_data_without_claiming_live_units() {
        let synth = r#"<AudioSynthFolder><Attributes name="Inst02"><UID x:id="deviceClassID" uid="synth-class"/><Attributes x:id="deviceData" name="Synth"><UID x:id="uniqueID" uid="synth-instance"/></Attributes><String x:id="presetPath" text="Presets/synth.fxpreset"/></Attributes></AudioSynthFolder>"#;
        let preset = r#"<AudioEffectPreset><Attributes x:id="ParameterData" cutoff="200" enabled="1" label="Soft &amp; warm"/><Attributes x:id="WindowState" x="10"/></AudioEffectPreset>"#;
        let file = write_archive(&[
            ("Song/song.xml", SONG),
            ("Devices/audiosynthfolder.xml", synth),
            ("Presets/synth.fxpreset", preset),
        ]);
        let song = read_song(file.path()).unwrap();
        let plugin = &song.plugins[0];
        assert_eq!(plugin.kind, "instrument");
        assert_eq!(plugin.slot, Some(1));
        assert_eq!(plugin.parameters["cutoff"], json!(200.0));
        assert_eq!(plugin.parameters["label"], json!("Soft & warm"));
        assert!(!plugin.parameters.contains_key("x"));
        assert_eq!(plugin.parameter_units, "preset_native_units_not_live_raw");
    }

    #[test]
    fn missing_and_binary_states_remain_explicit() {
        let synth = r#"<AudioSynthFolder><Attributes name="Inst01"><Attributes x:id="deviceData" name="Missing"/><String x:id="presetPath" text="Presets/missing.fxpreset"/></Attributes><Attributes name="Inst02"><Attributes x:id="deviceData" name="Opaque"/><String x:id="presetPath" text="Presets/binary.fxpreset"/></Attributes></AudioSynthFolder>"#;
        let file = write_archive(&[
            ("Song/song.xml", SONG),
            ("Devices/audiosynthfolder.xml", synth),
            ("Presets/binary.fxpreset", "opaque data"),
        ]);
        let song = read_song(file.path()).unwrap();
        assert_eq!(song.plugins[0].state_status, "missing");
        assert_eq!(song.plugins[1].state_status, "unparsed");
        assert_eq!(song.warnings.len(), 2);
    }

    #[test]
    fn rejects_unsafe_paths_and_xml_entities() {
        let file = write_archive(&[("Song/song.xml", SONG), ("../outside", "bad")]);
        assert!(read_song(file.path()).unwrap_err().contains("unsafe"));
        assert!(
            parse_xml(br#"<!DOCTYPE Song [<!ENTITY x SYSTEM "file:///private">]><Song/>"#)
                .unwrap_err()
                .contains("DTD")
        );
        let deep = format!(
            "{}{}",
            "<a>".repeat(MAX_XML_DEPTH + 1),
            "</a>".repeat(MAX_XML_DEPTH + 1)
        );
        assert!(parse_xml(deep.as_bytes()).is_err());
    }

    #[test]
    fn rejects_flattening_collisions_instead_of_losing_scoped_values() {
        let xml = br#"<AudioEffectPreset><Attributes x:id="ParameterData" gain="1"/><Attributes x:id="ParameterData" gain="2"/></AudioEffectPreset>"#;
        assert!(read_plugin_state(xml).unwrap_err().contains("scoped"));
        let json = br#"{"parameters":{"a.b":1,"a":{"b":2}}}"#;
        assert!(read_plugin_state(json).unwrap_err().contains("collide"));
    }

    #[test]
    fn validates_song_structure_and_metadata_entry_limits() {
        let missing = write_archive(&[("notes.txt", "not a song")]);
        assert!(read_song(missing.path()).is_err());
        let body = format!("<Song>{}</Song>", "x".repeat(MAX_ENTRY_BYTES as usize));
        let oversized = write_archive(&[("Song/song.xml", &body)]);
        assert!(read_song(oversized.path()).unwrap_err().contains("16 MiB"));
    }
}

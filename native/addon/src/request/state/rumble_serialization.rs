use quick_xml::{
    Reader,
    events::{BytesStart, Event},
};
use serde::Serialize;
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::{Value, json};
use std::collections::BTreeSet;

const UNITS: usize = 8192;
const DEPTH: usize = 32;
const STRING: usize = 4096;
const TAGS: [&str; 4] = [
    "AudioEffectPreset",
    "MusicEffectPreset",
    "Attributes",
    "Attribute",
];
const IDS: [&str; 30] = [
    "ComponentData",
    "AlgorithmData",
    "ParameterData",
    "parameters",
    "effect1",
    "effect2",
    "effect3",
    "effect4",
    "effect5",
    "effect6",
    "effect7",
    "effect8",
    "effect9",
    "effect10",
    "effect11",
    "effect12",
    "effect13",
    "effect14",
    "effect15",
    "effect16",
    "effect17",
    "effect18",
    "effect19",
    "effect20",
    "effect21",
    "effect22",
    "effect23",
    "effect24",
    "effect25",
    "amp",
];
#[derive(Default, Serialize)]
struct Counts {
    known_tags: [u32; 4],
    other_tags: u32,
    known_ids: [u32; 30],
    classids_total: u32,
    classids_valid_guid: u32,
    classids_zero_guid: u32,
    classids_invalid: u32,
}
impl Counts {
    fn id(&mut self, value: &str) {
        if let Some(i) = IDS.iter().position(|v| *v == value) {
            self.known_ids[i] += 1;
        }
    }
    fn classid(&mut self, value: Option<&str>) {
        self.classids_total += 1;
        let value = value.unwrap_or("");
        let value = value
            .strip_prefix('{')
            .and_then(|v| v.strip_suffix('}'))
            .unwrap_or(value);
        let valid = value.len() == 36
            && value.bytes().enumerate().all(|(i, b)| {
                if [8, 13, 18, 23].contains(&i) {
                    b == b'-'
                } else {
                    b.is_ascii_hexdigit()
                }
            });
        if valid {
            self.classids_valid_guid += 1;
            if value.bytes().all(|b| b == b'0' || b == b'-') {
                self.classids_zero_guid += 1;
            }
        } else {
            self.classids_invalid += 1;
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reason {
    UnsupportedContainer,
    Opaque,
    InvalidUtf8,
    InvalidBom,
    Syntax,
    DuplicateKey,
    DuplicateAttribute,
    UnsupportedXml,
    TrailingDocument,
    Limit,
}
impl Reason {
    fn wire(self) -> &'static str {
        match self {
            Self::UnsupportedContainer => "unsupported_container",
            Self::Opaque => "opaque",
            Self::InvalidUtf8 => "invalid_utf8",
            Self::InvalidBom => "invalid_bom",
            Self::Syntax => "syntax",
            Self::DuplicateKey => "duplicate_key",
            Self::DuplicateAttribute => "duplicate_attribute",
            Self::UnsupportedXml => "unsupported_xml",
            Self::TrailingDocument => "trailing_document",
            Self::Limit => "limit",
        }
    }
}
enum Parse {
    Complete { root: &'static str, counts: Counts },
    Incomplete { reason: Reason },
}
pub(super) struct Summary {
    format: &'static str,
    parse: Parse,
}
impl Summary {
    pub(super) fn fields(self) -> Value {
        match self.parse {
            Parse::Complete { root, counts } => {
                json!({"format":self.format,"parse_complete":true,"reason":null,"root_kind":root,"counts":counts})
            }
            Parse::Incomplete { reason } => {
                json!({"format":self.format,"parse_complete":false,"reason":reason.wire(),"root_kind":null,"counts":null})
            }
        }
    }
}
struct Budget {
    units: usize,
    counts: Counts,
    reason: Reason,
}
impl Budget {
    fn new() -> Self {
        Self {
            units: 0,
            counts: Counts::default(),
            reason: Reason::Syntax,
        }
    }
    fn charge(&mut self) -> Result<(), Reason> {
        self.units += 1;
        if self.units > UNITS {
            Err(Reason::Limit)
        } else {
            Ok(())
        }
    }
    fn reject<E: de::Error>(&mut self, reason: Reason) -> E {
        self.reason = reason;
        E::custom("bounded structural parse rejected")
    }
}
struct Json<'a> {
    budget: &'a mut Budget,
    depth: usize,
    classid: bool,
}
impl<'de> DeserializeSeed<'de> for Json<'_> {
    type Value = &'static str;
    fn deserialize<D: de::Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        if self.depth > DEPTH || self.budget.charge().is_err() {
            return Err(self.budget.reject(Reason::Limit));
        }
        d.deserialize_any(self)
    }
}
impl<'de> Visitor<'de> for Json<'_> {
    type Value = &'static str;
    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("bounded JSON")
    }
    fn visit_bool<E: de::Error>(self, _: bool) -> Result<Self::Value, E> {
        self.visit_unit()
    }
    fn visit_i64<E: de::Error>(self, _: i64) -> Result<Self::Value, E> {
        self.visit_unit()
    }
    fn visit_u64<E: de::Error>(self, _: u64) -> Result<Self::Value, E> {
        self.visit_unit()
    }
    fn visit_f64<E: de::Error>(self, _: f64) -> Result<Self::Value, E> {
        self.visit_unit()
    }
    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        if self.classid {
            self.budget.counts.classid(None);
        }
        Ok("json_primitive")
    }
    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        if value.len() > STRING {
            return Err(self.budget.reject(Reason::Limit));
        }
        if self.classid {
            self.budget.counts.classid(Some(value));
        }
        Ok("json_primitive")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        if self.classid {
            self.budget.counts.classid(None);
        }
        while seq
            .next_element_seed(Json {
                budget: self.budget,
                depth: self.depth + 1,
                classid: false,
            })?
            .is_some()
        {}
        Ok("json_array")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        if self.classid {
            self.budget.counts.classid(None);
        }
        let mut keys = BTreeSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if key.len() > STRING || self.budget.charge().is_err() {
                return Err(self.budget.reject(Reason::Limit));
            }
            if !keys.insert(key.clone()) {
                return Err(self.budget.reject(Reason::DuplicateKey));
            }
            self.budget.counts.id(&key);
            map.next_value_seed(Json {
                budget: self.budget,
                depth: self.depth + 1,
                classid: key == "__classid",
            })?;
        }
        Ok("json_object")
    }
}
fn json(text: &str) -> Result<(&'static str, Counts), Reason> {
    // Bound encoded tokens before serde allocates their decoded strings.
    let mut in_string = false;
    let mut escaped = false;
    let mut len = 0;
    for byte in text.bytes() {
        if !in_string {
            if byte == b'"' {
                in_string = true;
                len = 0;
            }
        } else if byte == b'"' && !escaped {
            in_string = false;
        } else {
            len += 1;
            if len > STRING {
                return Err(Reason::Limit);
            }
            escaped = byte == b'\\' && !escaped;
        }
    }
    let mut budget = Budget::new();
    let mut deserializer = serde_json::Deserializer::from_str(text);
    let root = Json {
        budget: &mut budget,
        depth: 1,
        classid: false,
    }
    .deserialize(&mut deserializer)
    .map_err(|_| budget.reason)?;
    deserializer.end().map_err(|_| Reason::TrailingDocument)?;
    Ok((root, budget.counts))
}
fn xml_char(c: char) -> bool {
    matches!(c as u32, 9|10|13|0x20..=0xd7ff|0xe000..=0xfffd|0x10000..=0x10ffff)
}
fn start_char(c: char) -> bool {
    matches!(c as u32, 65..=90|95|97..=122|0xc0..=0xd6|0xd8..=0xf6|0xf8..=0x2ff|0x370..=0x37d|0x37f..=0x1fff|0x200c..=0x200d|0x2070..=0x218f|0x2c00..=0x2fef|0x3001..=0xd7ff|0xf900..=0xfdcf|0xfdf0..=0xfffd|0x10000..=0xeffff)
}
fn name(bytes: &[u8], attribute: bool) -> Result<&str, Reason> {
    let text = std::str::from_utf8(bytes).map_err(|_| Reason::InvalidUtf8)?;
    if text.len() > STRING {
        return Err(Reason::Limit);
    }
    if text == "xmlns"
        || text.starts_with("xmlns:")
        || text.contains(':') && !(attribute && text == "x:id")
    {
        return Err(Reason::UnsupportedXml);
    }
    if attribute && text == "x:id" {
        return Ok(text);
    }
    let mut chars = text.chars();
    if !chars.next().is_some_and(start_char)
        || !chars.all(|c| {
            start_char(c) || matches!(c as u32,45|46|48..=57|0xb7|0x300..=0x36f|0x203f..=0x2040)
        })
    {
        return Err(Reason::Syntax);
    }
    Ok(text)
}
fn decoded(text: &str) -> Result<std::borrow::Cow<'_, str>, Reason> {
    if text.len() > STRING {
        return Err(Reason::Limit);
    }
    let value = quick_xml::escape::unescape_with(text, quick_xml::escape::resolve_xml_entity)
        .map_err(|_| Reason::UnsupportedXml)?;
    if value.len() > STRING {
        return Err(Reason::Limit);
    }
    if !value.chars().all(xml_char) {
        return Err(Reason::Syntax);
    }
    Ok(value)
}
fn attribute_grammar(mut raw: &[u8]) -> Result<(), Reason> {
    fn space(byte: u8) -> bool {
        matches!(byte, b' ' | b'\n' | b'\r' | b'\t')
    }
    fn trim_space(raw: &[u8]) -> &[u8] {
        &raw[raw.iter().take_while(|b| space(**b)).count()..]
    }
    while !raw.is_empty() {
        if !space(raw[0]) {
            return Err(Reason::Syntax);
        }
        raw = trim_space(raw);
        if raw.is_empty() {
            break;
        }
        let key_end = raw
            .iter()
            .position(|b| space(*b) || *b == b'=')
            .ok_or(Reason::Syntax)?;
        if key_end == 0 {
            return Err(Reason::Syntax);
        }
        raw = trim_space(&raw[key_end..]);
        raw = raw.strip_prefix(b"=").ok_or(Reason::Syntax)?;
        raw = trim_space(raw);
        let quote = *raw.first().ok_or(Reason::Syntax)?;
        if !matches!(quote, b'\'' | b'"') {
            return Err(Reason::Syntax);
        }
        raw = &raw[1..];
        let end = raw.iter().position(|b| *b == quote).ok_or(Reason::Syntax)?;
        if end > STRING {
            return Err(Reason::Limit);
        }
        raw = &raw[end + 1..];
    }
    Ok(())
}
fn element(element: &BytesStart<'_>, budget: &mut Budget) -> Result<&'static str, Reason> {
    attribute_grammar(element.attributes_raw())?;
    let binding = element.name();
    let tag = name(binding.as_ref(), false)?;
    let root = if let Some(i) = TAGS.iter().position(|v| *v == tag) {
        budget.counts.known_tags[i] += 1;
        [
            "audio_effect_preset",
            "music_effect_preset",
            "attributes",
            "attribute",
        ][i]
    } else {
        budget.counts.other_tags += 1;
        "other_xml"
    };
    let mut seen = BTreeSet::new();
    for (index, attr) in element.attributes().with_checks(false).enumerate() {
        if index >= 64 {
            return Err(Reason::Limit);
        }
        budget.charge()?;
        let attr = attr.map_err(|_| Reason::Syntax)?;
        let key = name(attr.key.as_ref(), true)?;
        if !seen.insert(key.to_owned()) {
            return Err(Reason::DuplicateAttribute);
        }
        let raw = std::str::from_utf8(&attr.value).map_err(|_| Reason::InvalidUtf8)?;
        if raw.contains('<') {
            return Err(Reason::Syntax);
        }
        let value = decoded(raw)?;
        if key == "x:id" {
            budget.counts.id(&value);
        }
        if key == "__classid" {
            budget.counts.classid(Some(&value));
        }
    }
    Ok(root)
}
fn xml(text: &str) -> Result<(&'static str, Counts), Reason> {
    if !text.chars().all(xml_char) {
        return Err(Reason::Syntax);
    }
    let mut reader = Reader::from_str(text);
    let mut budget = Budget::new();
    let mut stack: Vec<Vec<u8>> = Vec::new();
    let mut root = None;
    let mut first = true;
    let mut text_raw = 0;
    let mut text_decoded = 0;
    loop {
        let event = reader.read_event().map_err(|_| Reason::Syntax)?;
        if matches!(event, Event::Eof) {
            break;
        }
        budget.charge()?;
        if !matches!(
            event,
            Event::Text(_) | Event::GeneralRef(_) | Event::CData(_)
        ) {
            text_raw = 0;
            text_decoded = 0;
        }
        match event {
            Event::Decl(decl) if first => {
                if decl.version().map_err(|_| Reason::Syntax)?.as_ref() != b"1.0" {
                    return Err(Reason::UnsupportedXml);
                }
                let raw = std::str::from_utf8(&decl).map_err(|_| Reason::InvalidUtf8)?;
                if raw.len() > STRING {
                    return Err(Reason::Limit);
                }
                let attrs = BytesStart::from_content(raw, 3);
                attribute_grammar(attrs.attributes_raw())?;
                let mut seen = BTreeSet::new();
                let mut previous = 0;
                for attr in attrs.attributes() {
                    let attr = attr.map_err(|_| Reason::Syntax)?;
                    let key = std::str::from_utf8(attr.key.as_ref()).map_err(|_| Reason::Syntax)?;
                    let value = std::str::from_utf8(&attr.value).map_err(|_| Reason::Syntax)?;
                    if !seen.insert(key.to_owned())
                        || match key {
                            "version" => value != "1.0",
                            "encoding" => !value.eq_ignore_ascii_case("UTF-8"),
                            "standalone" => !matches!(value, "yes" | "no"),
                            _ => true,
                        }
                    {
                        return Err(Reason::UnsupportedXml);
                    }
                    let position = match key {
                        "version" => 1,
                        "encoding" => 2,
                        "standalone" => 3,
                        _ => return Err(Reason::UnsupportedXml),
                    };
                    if position <= previous || previous == 0 && position != 1 {
                        return Err(Reason::Syntax);
                    }
                    previous = position;
                }
            }
            Event::Start(ref el) | Event::Empty(ref el) => {
                if stack.is_empty() && root.is_some() {
                    return Err(Reason::TrailingDocument);
                }
                if stack.len() >= DEPTH {
                    return Err(Reason::Limit);
                }
                let kind = element(el, &mut budget)?;
                if root.is_none() {
                    root = Some(kind);
                }
                if matches!(event, Event::Start(_)) {
                    stack.push(el.name().as_ref().to_vec());
                }
            }
            Event::End(end) => {
                if stack.pop().as_deref() != Some(end.name().as_ref()) {
                    return Err(Reason::Syntax);
                }
            }
            Event::Text(value) => {
                let raw = std::str::from_utf8(&value).map_err(|_| Reason::InvalidUtf8)?;
                if stack.is_empty()
                    && !raw
                        .bytes()
                        .all(|b| matches!(b, b' ' | b'\n' | b'\r' | b'\t'))
                {
                    return Err(Reason::TrailingDocument);
                }
                if raw.contains("]]>") {
                    return Err(Reason::Syntax);
                }
                text_raw += raw.len();
                text_decoded += decoded(raw)?.len();
                if text_raw > STRING || text_decoded > STRING {
                    return Err(Reason::Limit);
                }
            }
            Event::GeneralRef(value) => {
                if stack.is_empty() {
                    return Err(Reason::TrailingDocument);
                }
                let raw = std::str::from_utf8(&value).map_err(|_| Reason::InvalidUtf8)?;
                if raw.len() > STRING - 2 {
                    return Err(Reason::Limit);
                }
                let encoded = format!("&{raw};");
                text_raw += encoded.len();
                text_decoded += decoded(&encoded)?.len();
                if text_raw > STRING || text_decoded > STRING {
                    return Err(Reason::Limit);
                }
            }
            Event::CData(value) => {
                if stack.is_empty() {
                    return Err(Reason::Syntax);
                }
                let raw = std::str::from_utf8(&value).map_err(|_| Reason::InvalidUtf8)?;
                text_raw += raw.len();
                text_decoded += raw.len();
                if text_raw > STRING || text_decoded > STRING {
                    return Err(Reason::Limit);
                }
            }
            Event::Comment(value) => {
                if value.len() > STRING {
                    return Err(Reason::Limit);
                }
                if value.windows(2).any(|v| v == b"--") || value.last() == Some(&b'-') {
                    return Err(Reason::Syntax);
                }
            }
            _ => return Err(Reason::UnsupportedXml),
        }
        first = false;
    }
    if !stack.is_empty() {
        return Err(Reason::Syntax);
    }
    Ok((root.ok_or(Reason::Syntax)?, budget.counts))
}
pub(super) fn summarize(bytes: &[u8]) -> Summary {
    let mut format = "opaque";
    let parsed = (|| {
        if bytes.len() > super::MAX_CAPSULE_BYTES {
            return Err(Reason::Limit);
        }
        if [
            b"PK\x03\x04".as_slice(),
            b"PK\x05\x06".as_slice(),
            b"PK\x07\x08".as_slice(),
        ]
        .iter()
        .any(|magic| bytes.starts_with(magic))
        {
            format = "zip_signature_only";
            return Err(Reason::UnsupportedContainer);
        }
        let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
        if bytes.starts_with(&[0xff, 0xfe])
            || bytes.starts_with(&[0xfe, 0xff])
            || bytes.starts_with(&[0, 0, 0xfe, 0xff])
            || bytes.starts_with(&[0xef, 0xbb, 0xbf])
        {
            return Err(Reason::InvalidBom);
        }
        let leading = bytes
            .iter()
            .copied()
            .find(|b| !matches!(b, b' ' | b'\n' | b'\r' | b'\t'));
        format = match leading {
            Some(b'<') => "utf8_xml",
            Some(b'{' | b'[' | b'"' | b'-' | b'0'..=b'9' | b't' | b'f' | b'n') => "utf8_json",
            _ => "opaque",
        };
        let text = std::str::from_utf8(bytes).map_err(|_| Reason::InvalidUtf8)?;
        match format {
            "utf8_xml" => xml(text),
            "utf8_json" => json(text),
            _ => Err(Reason::Opaque),
        }
    })();
    Summary {
        format,
        parse: match parsed {
            Ok((root, counts)) => Parse::Complete { root, counts },
            Err(reason) => Parse::Incomplete { reason },
        },
    }
}

pub(super) fn validated_json(bytes: &[u8]) -> Result<Value, String> {
    let fields = summarize(bytes).fields();
    if fields["format"] != "utf8_json"
        || fields["parse_complete"] != true
        || fields["root_kind"] != "json_object"
    {
        return Err("Bounded Rumble JSON unavailable".into());
    }
    serde_json::from_slice(bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes))
        .map_err(|_| "Bounded Rumble JSON unavailable".into())
}

pub(super) fn valid_receipt(instance: &Value, value: &Value) -> bool {
    fn exact_gui_object(value: &Value, keys: &[&str]) -> bool {
        value.as_object().is_some_and(|object| {
            object.len() == keys.len() && keys.iter().all(|key| object.contains_key(*key))
        })
    }
    let fields = [
        "ok",
        "outcome",
        "operation",
        "schema",
        "serialization",
        "instance",
        "source",
        "capture_sha256",
        "capture_byte_count",
        "format",
        "parse_complete",
        "reason",
        "root_kind",
        "counts",
        "value_semantics",
        "mutation_capability",
        "model_identity_verified",
        "audio_verified",
        "persistence_verified",
    ];
    if !exact_gui_object(value, &fields)
        || value.to_string().len() + 60 > 4096
        || value["ok"] != true
        || value["outcome"] != "observed"
        || value["operation"] != "rumble_serialization_inspect"
        || value["schema"] != "rumble_alias_serialization_summary_v1"
        || value["serialization"] != "component_alias_IStorable"
        || value["source"] != "component_alias"
        || value["value_semantics"] != "sequential_snapshot"
        || [
            "mutation_capability",
            "model_identity_verified",
            "audio_verified",
            "persistence_verified",
        ]
        .iter()
        .any(|key| value[*key] != false)
        || !exact_gui_object(
            &value["instance"],
            &["object_uid", "class_uid", "native_class", "object_id"],
        )
        || value["instance"]["object_uid"] != instance["object_uid"]
        || value["instance"]["class_uid"] != "B9D7536C-A010-457D-A182-36E5531164C9"
        || value["instance"]["native_class"] != "AudioEffect"
        || value["instance"]["object_id"] != "Rumble Native"
        || !value["capture_sha256"].as_str().is_some_and(|s| {
            s.len() == 64
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
        || !value["capture_byte_count"]
            .as_u64()
            .is_some_and(|n| n <= 16 * 1024 * 1024)
    {
        return false;
    }
    let format = value["format"].as_str().unwrap_or("");
    if value["parse_complete"] == false {
        if !value["root_kind"].is_null() || !value["counts"].is_null() {
            return false;
        }
        let reason = value["reason"].as_str().unwrap_or("");
        return match format {
            "utf8_json" => matches!(
                reason,
                "syntax" | "duplicate_key" | "trailing_document" | "limit" | "invalid_utf8"
            ),
            "utf8_xml" => matches!(
                reason,
                "syntax"
                    | "duplicate_attribute"
                    | "unsupported_xml"
                    | "invalid_utf8"
                    | "trailing_document"
                    | "limit"
            ),
            "zip_signature_only" => reason == "unsupported_container",
            "opaque" => matches!(reason, "opaque" | "invalid_utf8" | "invalid_bom" | "limit"),
            _ => false,
        };
    }
    if value["parse_complete"] != true || !value["reason"].is_null() {
        return false;
    }
    if !match format {
        "utf8_json" => matches!(
            value["root_kind"].as_str(),
            Some("json_object" | "json_array" | "json_primitive")
        ),
        "utf8_xml" => matches!(
            value["root_kind"].as_str(),
            Some(
                "audio_effect_preset"
                    | "music_effect_preset"
                    | "attributes"
                    | "attribute"
                    | "other_xml"
            )
        ),
        _ => false,
    } {
        return false;
    }
    let counts = &value["counts"];
    if !exact_gui_object(
        counts,
        &[
            "known_tags",
            "other_tags",
            "known_ids",
            "classids_total",
            "classids_valid_guid",
            "classids_zero_guid",
            "classids_invalid",
        ],
    ) {
        return false;
    }
    let mut units = 0;
    for (key, len) in [("known_tags", 4), ("known_ids", 30)] {
        let Some(items) = counts[key].as_array().filter(|a| a.len() == len) else {
            return false;
        };
        for item in items {
            let Some(n) = item.as_u64().filter(|n| *n <= 8192) else {
                return false;
            };
            units += n;
        }
    }
    for key in [
        "other_tags",
        "classids_total",
        "classids_valid_guid",
        "classids_zero_guid",
        "classids_invalid",
    ] {
        if !counts[key].as_u64().is_some_and(|n| n <= 8192) {
            return false;
        }
    }
    units += counts["other_tags"].as_u64().unwrap() + counts["classids_total"].as_u64().unwrap();
    let root = value["root_kind"].as_str().unwrap_or("");
    let root_count = match root {
        "audio_effect_preset" => Some(0),
        "music_effect_preset" => Some(1),
        "attributes" => Some(2),
        "attribute" => Some(3),
        _ => None,
    };
    if root_count.is_some_and(|index| counts["known_tags"][index].as_u64().unwrap() == 0)
        || root == "other_xml" && counts["other_tags"] == 0
        || root == "json_primitive" && units != 0
    {
        return false;
    }
    units <= 8192
        && counts["classids_total"].as_u64().unwrap()
            == counts["classids_valid_guid"].as_u64().unwrap()
                + counts["classids_invalid"].as_u64().unwrap()
        && counts["classids_zero_guid"].as_u64() <= counts["classids_valid_guid"].as_u64()
        && (format != "utf8_json"
            || (counts["other_tags"] == 0
                && counts["known_tags"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|n| *n == 0)))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fields(bytes: &[u8]) -> Value {
        summarize(bytes).fields()
    }
    #[test]
    fn json_census_is_structural_and_private() {
        let value = fields(br#"{"secret_name":"secret_value effect1", "effect1":{"__classid":"00000000-0000-0000-0000-000000000000"},"amp":null}"#);
        assert_eq!(value["parse_complete"], true);
        assert_eq!(value["counts"]["known_ids"][4], 1);
        assert_eq!(value["counts"]["classids_zero_guid"], 1);
        assert!(!value.to_string().contains("secret"));
    }
    #[test]
    fn incomplete_discards_all_counts() {
        for (bytes, reason) in [
            (br#"{"amp":1,"amp":2}"#.as_slice(), "duplicate_key"),
            (b"{} {}", "trailing_document"),
            (b"<a x='1' x='2'/>", "duplicate_attribute"),
            (b"<a/><b/>", "trailing_document"),
            (b"<!DOCTYPE a><a/>", "unsupported_xml"),
            (b"<a xmlns:x='secret'/>", "unsupported_xml"),
            (b"<x:a/>", "unsupported_xml"),
            (b"<a>&unknown;</a>", "unsupported_xml"),
            (b"<a>", "syntax"),
            (b"<?xml version='2.0'?><a/>", "unsupported_xml"),
        ] {
            let value = fields(bytes);
            assert_eq!(value["reason"], reason, "{value}");
            assert!(value["counts"].is_null());
            assert!(value["root_kind"].is_null());
        }
    }
    #[test]
    fn xml_accepts_only_literal_host_id_and_safe_references() {
        let value = fields(b"<?xml version='1.0' encoding='UTF-8'?><AudioEffectPreset><Attributes x:id='effect&#49;' __classid='not-a-guid' value='&amp;&quot;&apos;&lt;&gt;'>&#65;&amp;</Attributes></AudioEffectPreset>");
        assert_eq!(value["parse_complete"], true, "{value}");
        assert_eq!(value["counts"]["known_ids"][4], 1);
        assert_eq!(value["counts"]["classids_invalid"], 1);
    }
    #[test]
    fn xml_requires_attribute_separators_and_ordered_declarations() {
        for text in [
            "<a a='x'b='y'/>",
            "<a a='x' b=unquoted/>",
            "<a a='unterminated/>",
            "<?xml version='1.0'encoding='UTF-8'?><a/>",
            "<?xml version='1.0' standalone='yes' encoding='UTF-8'?><a/>",
        ] {
            let value = fields(text.as_bytes());
            assert_eq!(value["format"], "utf8_xml");
            assert_eq!(value["parse_complete"], false, "{text}: {value}");
            assert_eq!(value["reason"], "syntax");
            assert!(value["counts"].is_null());
            assert!(value["root_kind"].is_null());
        }
        for text in [
            "<a a='x' b='y'/>",
            "<a\ta = 'x'\nb=\"y\"\r/>",
            "<?xml version='1.0' encoding='UTF-8' standalone='yes'?><a/>",
            "<?xml\tversion = \"1.0\"\nencoding='utf-8'\rstandalone = 'no' ?><a/>",
            "<?xml version='1.0' standalone='yes'?><a/>",
        ] {
            let value = fields(text.as_bytes());
            assert_eq!(value["parse_complete"], true, "{text}: {value}");
            assert_eq!(value["counts"]["other_tags"], 1);
        }
    }
    #[test]
    fn exact_limits_bom_guid_partitions_and_text_grammar_are_verified() {
        for text in [
            format!("\"{}\"", "x".repeat(4096)),
            format!("[{}]", vec!["0"; 8191].join(",")),
            format!("{}0{}", "[".repeat(31), "]".repeat(31)),
            format!("<a>{}</a>", "x".repeat(4096)),
            format!("<a value='{}'/>", "x".repeat(4096)),
            format!(
                "<a {} />",
                (0..64)
                    .map(|i| format!("a{i}='0'"))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            format!("{}<a/>{}", "<a>".repeat(31), "</a>".repeat(31)),
        ] {
            assert_eq!(fields(text.as_bytes())["parse_complete"], true);
        }
        let xml =
            fields(b"\xef\xbb\xbf<Attributes __classid='{12345678-1234-1234-1234-123456789abc}'/>");
        assert_eq!(xml["counts"]["classids_valid_guid"], 1);
        for bytes in [
            b"\xef\xbb\xbf\xef\xbb\xbf{}".as_slice(),
            b"\xff\xfeX",
            b"\x00\x00\xfe\xffX",
        ] {
            assert_eq!(fields(bytes)["reason"], "invalid_bom");
        }
        for text in [
            "<a>&#0;</a>",
            "<a>&#xD800;</a>",
            "<a>&#+65;</a>",
            "<a>]]></a>",
            "<?xml encoding='UTF-8' version='1.0'?><a/>",
            "<a/><?work secret?>",
            "<a a='&custom;'/>",
            "<a a='0' a='1'/>",
        ] {
            assert_eq!(fields(text.as_bytes())["parse_complete"], false, "{text}");
        }
        assert_eq!(
            fields(&vec![b'x'; super::super::MAX_CAPSULE_BYTES])["reason"],
            "opaque"
        );
        assert_eq!(
            fields(&vec![b'x'; super::super::MAX_CAPSULE_BYTES + 1])["reason"],
            "limit"
        );
    }
    #[test]
    fn all_limits_are_hard_and_rejections_keep_format() {
        for text in [
            format!("\"{}\"", "a".repeat(4097)),
            format!("{}0{}", "[".repeat(33), "]".repeat(33)),
            format!("[{}]", vec!["0"; 8193].join(",")),
            format!(
                "<a {} />",
                (0..65)
                    .map(|i| format!("a{i}='0'"))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            format!("<a>{}</a>", "x".repeat(4097)),
            format!("<a value='{}'/>", "x".repeat(4097)),
            format!("{}{}", "<a>".repeat(33), "</a>".repeat(33)),
        ] {
            let value = fields(text.as_bytes());
            assert_eq!(value["reason"], "limit", "{value}");
            assert!(value["counts"].is_null());
        }
        assert_eq!(fields(b"{broken")["format"], "utf8_json");
        for (bytes, format) in [
            (b"<a>\xff</a>".as_slice(), "utf8_xml"),
            (b"{\xff}", "utf8_json"),
        ] {
            let value = fields(bytes);
            assert_eq!(value["format"], format);
            assert_eq!(value["reason"], "invalid_utf8");
        }
        assert_eq!(fields(b"PK\x03\x04secret")["format"], "zip_signature_only");
        assert_eq!(fields(b"binary-secret")["reason"], "opaque");
    }
}

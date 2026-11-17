use super::*;
use quick_xml::{Reader, events::BytesStart, events::Event};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub(super) const MAX_CAPTURE_BYTES: usize = 1_048_576;
const CLASS: &str = "D2D9B002-12FC-4F1D-AFDA-70336D1DC108";
const SCOPE: &str = "component_alias_IStorable_chorder_chords";
const SCHEMA: &str = "chorder_music_effect_preset_v1";

#[derive(Debug)]
pub(super) enum Failure {
    Unavailable(String),
    Unsupported,
}
impl Failure {
    pub(super) fn code(&self) -> &'static str {
        match self {
            Self::Unavailable(_) => "chorder_chords_unavailable",
            Self::Unsupported => "chorder_schema_unsupported",
        }
    }
    pub(super) fn message(&self) -> &str {
        match self {
            Self::Unavailable(message) => message,
            Self::Unsupported => {
                "Captured Chorder state does not match the bounded supported schema"
            }
        }
    }
}

struct Chords {
    outputs: [Vec<u8>; 128],
}
impl Chords {
    fn digest(&self) -> String {
        let mut hash = Sha256::new();
        hash.update(b"chorder-model-v1\0");
        for (input, outputs) in self.outputs.iter().enumerate() {
            hash.update([input as u8]);
            hash.update((outputs.len() as u16).to_be_bytes());
            hash.update(outputs);
        }
        format!("{:x}", hash.finalize())
    }
    fn page(&self, offset: u32, limit: u32, bytes: &[u8], instance: &Instance) -> Value {
        let end = (offset + limit).min(128);
        let chords: Vec<_> = (offset..end)
            .map(|input| json!({"input_pitch":input,"output_pitches":self.outputs[input as usize]}))
            .collect();
        json!({"ok":true,"outcome":"observed","operation":"chorder_chords_get",
            "scope":SCOPE,"schema":SCHEMA,"serialization":"component_alias_IStorable",
            "instance":instance,"source":"component_alias","total":128,"offset":offset,"limit":limit,
            "count":end-offset,"next":(end<128).then_some(end),"chords":chords,
            "capture_sha256":format!("{:x}",Sha256::digest(bytes)),"capture_byte_count":bytes.len(),
            "model_sha256":self.digest(),"value_semantics":"sequential_snapshot",
            "mutation_capability":false,"persistence_verified":false,"audio_verified":false})
    }
}

#[derive(Clone, Copy)]
enum Node {
    Root,
    Component,
    Configuration,
    Algorithm,
    Model,
    Input(u8),
    Output,
}
struct Decoder {
    stack: Vec<Node>,
    root: bool,
    component: bool,
    configuration: bool,
    algorithm: bool,
    model: bool,
    inputs: [bool; 128],
    chords: Chords,
}
fn attributes(element: &BytesStart<'_>) -> Result<BTreeMap<String, String>, ()> {
    let mut result = BTreeMap::new();
    for item in element.attributes() {
        let item = item.map_err(|_| ())?;
        let key = std::str::from_utf8(item.key.as_ref()).map_err(|_| ())?;
        let value = std::str::from_utf8(&item.value).map_err(|_| ())?;
        if value.bytes().any(|b| matches!(b, b'&' | b'<' | 0..=31))
            || result.insert(key.to_owned(), value.to_owned()).is_some()
        {
            return Err(());
        }
    }
    Ok(result)
}
fn only<'a>(values: &'a BTreeMap<String, String>, name: &str) -> Result<&'a str, ()> {
    if values.len() != 1 {
        return Err(());
    }
    values.get(name).map(String::as_str).ok_or(())
}
fn pitch(value: &str) -> Result<u8, ()> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(());
    }
    value.parse::<u8>().ok().filter(|n| *n < 128).ok_or(())
}
fn configuration(values: &BTreeMap<String, String>) -> Result<(), ()> {
    if values.len() != 6 || values.get("x:id").map(String::as_str) != Some("chorder") {
        return Err(());
    }
    for (name, min, max) in [
        ("transpose", -127, 127),
        ("loKey", 0, 127),
        ("hiKey", 0, 127),
        ("autofill", 0, 1),
        ("filterOutsideRange", 0, 1),
    ] {
        let text = values.get(name).ok_or(())?;
        let digits = text.strip_prefix('-').unwrap_or(text);
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(());
        }
        let value = text.parse::<i32>().map_err(|_| ())?;
        if !(min..=max).contains(&value) {
            return Err(());
        }
    }
    Ok(())
}
impl Decoder {
    fn new() -> Self {
        Self {
            stack: Vec::with_capacity(5),
            root: false,
            component: false,
            configuration: false,
            algorithm: false,
            model: false,
            inputs: [false; 128],
            chords: Chords {
                outputs: std::array::from_fn(|_| Vec::new()),
            },
        }
    }
    fn start(&mut self, element: &BytesStart<'_>) -> Result<(), ()> {
        if self.stack.len() >= 5 {
            return Err(());
        }
        let values = attributes(element)?;
        let name = element.name();
        let next = match self.stack.last().copied() {
            None if !self.root && name.as_ref() == b"MusicEffectPreset" => {
                if only(&values, "cid")? != format!("{{{CLASS}}}") {
                    return Err(());
                }
                self.root = true;
                Node::Root
            }
            Some(Node::Root) if name.as_ref() == b"Attributes" => match only(&values, "x:id")? {
                "ComponentData" if !self.component => {
                    self.component = true;
                    Node::Component
                }
                "AlgorithmData" if !self.algorithm => {
                    self.algorithm = true;
                    Node::Algorithm
                }
                _ => return Err(()),
            },
            Some(Node::Component) if name.as_ref() == b"Attributes" && !self.configuration => {
                configuration(&values)?;
                self.configuration = true;
                Node::Configuration
            }
            Some(Node::Algorithm) if name.as_ref() == b"Attributes" && !self.model => {
                if only(&values, "x:id")? != "model" {
                    return Err(());
                }
                self.model = true;
                Node::Model
            }
            Some(Node::Model) if name.as_ref() == b"Attributes" => {
                let input = pitch(only(&values, "inPitch")?)?;
                if std::mem::replace(&mut self.inputs[input as usize], true) {
                    return Err(());
                }
                Node::Input(input)
            }
            Some(Node::Input(input)) if name.as_ref() == b"Attributes" => {
                let output = pitch(only(&values, "outPitch")?)?;
                let outputs = &mut self.chords.outputs[input as usize];
                if outputs.contains(&output) {
                    return Err(());
                }
                outputs.push(output);
                Node::Output
            }
            _ => return Err(()),
        };
        self.stack.push(next);
        Ok(())
    }
    fn end(&mut self) -> Result<(), ()> {
        self.stack.pop().map(|_| ()).ok_or(())
    }
    fn finish(mut self) -> Result<Chords, ()> {
        if !self.stack.is_empty()
            || !self.root
            || !self.component
            || !self.configuration
            || !self.algorithm
            || !self.model
        {
            return Err(());
        }
        for outputs in &mut self.chords.outputs {
            outputs.sort_unstable();
        }
        Ok(self.chords)
    }
}
fn decode(bytes: &[u8]) -> Result<Chords, ()> {
    if bytes.is_empty() || bytes.len() > MAX_CAPTURE_BYTES {
        return Err(());
    }
    let text = std::str::from_utf8(bytes).map_err(|_| ())?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    if text.starts_with('\u{feff}') {
        return Err(());
    }
    let mut reader = Reader::from_str(text);
    let mut decoder = Decoder::new();
    let mut first = true;
    loop {
        match reader.read_event().map_err(|_| ())? {
            Event::Decl(declaration) if first => {
                if declaration.version().map_err(|_| ())?.as_ref() != b"1.0" {
                    return Err(());
                }
                let text = std::str::from_utf8(&declaration).map_err(|_| ())?;
                let values = attributes(&BytesStart::from_content(text, 3))?;
                if values.get("version").map(String::as_str) != Some("1.0")
                    || values.iter().any(|(key, value)| match key.as_str() {
                        "version" => false,
                        "encoding" => !value.eq_ignore_ascii_case("UTF-8"),
                        "standalone" => value != "yes" && value != "no",
                        _ => true,
                    })
                {
                    return Err(());
                }
            }
            Event::Start(element) => decoder.start(&element)?,
            Event::Empty(element) => {
                decoder.start(&element)?;
                decoder.end()?;
            }
            Event::End(_) => decoder.end()?,
            Event::Text(value)
                if value
                    .iter()
                    .all(|b| matches!(b, b' ' | b'\n' | b'\r' | b'\t')) => {}
            Event::Eof => return decoder.finish(),
            _ => return Err(()),
        }
        first = false;
    }
}

pub(super) fn validate_page(offset: u32, limit: u32) -> Result<(), String> {
    if offset < 128 && (1..=16).contains(&limit) {
        Ok(())
    } else {
        Err("chorder_chords_get requires offset0..127 and limit1..16".into())
    }
}
pub(super) fn get(
    ctx: &Context<'_>,
    offset: u32,
    limit: u32,
) -> Result<(Value, state::AliasLease), Failure> {
    validate_page(offset, limit).map_err(Failure::Unavailable)?;
    if ctx.instance.class_uid != CLASS {
        return Err(Failure::Unavailable(
            "The exact Chorder class is required".into(),
        ));
    }
    let (bytes, alias) = state::read_chorder(ctx).map_err(Failure::Unavailable)?;
    let chords = decode(&bytes).map_err(|_| Failure::Unsupported)?;
    let value = chords.page(offset, limit, &bytes, &ctx.instance);
    alias.check().map_err(Failure::Unavailable)?;
    ctx.check().map_err(Failure::Unavailable)?;
    Ok((value, alias))
}
pub(super) fn invalidate(value: &mut Value) {
    if value["operation"] == "chorder_chords_get" {
        for key in [
            "chords",
            "capture_sha256",
            "capture_byte_count",
            "model_sha256",
            "schema",
            "total",
            "offset",
            "limit",
            "count",
            "next",
        ] {
            value
                .as_object_mut()
                .expect("Chorder reply object")
                .remove(key);
        }
    }
}

#[cfg(test)]
#[path = "chorder_chords_tests.rs"]
mod tests;

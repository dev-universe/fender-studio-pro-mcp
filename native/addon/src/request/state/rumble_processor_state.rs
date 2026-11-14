use super::*;
use std::collections::BTreeMap;

pub(super) const ENCODING: &str = "json_parsed_u64_i64_f64_bits_v1";
pub(super) const MAX_RESULT_BYTES: usize = 32768;
const CLASS: &str = "B9D7536C-A010-457D-A182-36E5531164C9";
const ZERO: &str = "00000000-0000-0000-0000-000000000000";

#[derive(Clone)]
enum Number {
    Unsigned(u64),
    Signed(i64),
    Float(u64),
}
impl Number {
    fn parsed(value: &Value) -> Option<Self> {
        let n = value.as_number()?;
        if let Some(v) = n.as_u64() {
            Some(Self::Unsigned(v))
        } else if let Some(v) = n.as_i64() {
            Some(Self::Signed(v))
        } else {
            let v = n.as_f64()?;
            v.is_finite().then_some(Self::Float(v.to_bits()))
        }
    }
    fn wire(&self) -> Value {
        match self {
            Self::Unsigned(v) => json!({"kind":"unsigned_integer","decimal":v.to_string()}),
            Self::Signed(v) => json!({"kind":"signed_integer","decimal":v.to_string()}),
            Self::Float(v) => json!({"kind":"float64","bits_hex":format!("{v:016x}")}),
        }
    }
    fn from_wire(value: &Value) -> Option<Self> {
        let obj = value.as_object()?;
        if obj.len() != 2 {
            return None;
        }
        match value["kind"].as_str()? {
            "unsigned_integer" => {
                let s = value["decimal"].as_str()?;
                let v = s.parse::<u64>().ok()?;
                (v.to_string() == s).then_some(Self::Unsigned(v))
            }
            "signed_integer" => {
                let s = value["decimal"].as_str()?;
                let v = s.parse::<i64>().ok()?;
                (v < 0 && v.to_string() == s).then_some(Self::Signed(v))
            }
            "float64" => {
                let s = value["bits_hex"].as_str()?;
                if s.len() != 16
                    || !s
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                {
                    return None;
                }
                let v = u64::from_str_radix(s, 16).ok()?;
                f64::from_bits(v).is_finite().then_some(Self::Float(v))
            }
            _ => None,
        }
    }
    fn hash(&self, hash: &mut Sha256) {
        let (tag, bytes) = match self {
            Self::Unsigned(v) => (b'U', v.to_be_bytes()),
            Self::Signed(v) => (b'I', v.to_be_bytes()),
            Self::Float(v) => (b'F', v.to_be_bytes()),
        };
        hash.update([tag]);
        hash.update(bytes);
    }
}
struct Processor {
    slot: String,
    class: String,
    uid: [u8; 16],
    settings: BTreeMap<String, Number>,
}
struct Projection {
    processors: Vec<Processor>,
}
fn slot(index: usize) -> String {
    if index == 25 {
        "amp".into()
    } else {
        format!("effect{}", index + 1)
    }
}
fn identifier(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name.as_bytes()[0].is_ascii_alphabetic()
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}
fn guid(text: &str) -> Option<(String, [u8; 16])> {
    let text = if text.starts_with('{') && text.ends_with('}') {
        &text[1..text.len() - 1]
    } else {
        text
    };
    if text.len() != 36
        || text.bytes().enumerate().any(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b != b'-'
            } else {
                !b.is_ascii_hexdigit()
            }
        })
    {
        return None;
    }
    let canonical = text.to_ascii_uppercase();
    let compact = canonical.replace('-', "");
    let mut bytes = [0; 16];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = u8::from_str_radix(&compact[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some((canonical, bytes))
}
impl Projection {
    fn decode(bytes: &[u8]) -> Result<Self, String> {
        let root = super::rumble_serialization::validated_json(bytes)?;
        let reject = || "Rumble processor schema unavailable".to_owned();
        let object = root.as_object().ok_or_else(reject)?;
        if object.len() != 4
            || !["cid", "classname", "parameters", "component"]
                .iter()
                .all(|k| object.contains_key(*k))
            || root["classname"] != "Rumble Native"
            || !root["component"].is_object()
            || guid(root["cid"].as_str().ok_or_else(reject)?)
                .map(|g| g.0)
                .as_deref()
                != Some(CLASS)
        {
            return Err(reject());
        }
        let parameters = root["parameters"].as_object().ok_or_else(reject)?;
        if parameters.len() != 29
            || !["input", "output", "global"]
                .iter()
                .all(|k| parameters.get(*k).is_some_and(Value::is_object))
        {
            return Err(reject());
        }
        let mut processors = Vec::with_capacity(26);
        let mut total = 0;
        for i in 0..26 {
            let slot = slot(i);
            let obj = parameters
                .get(&slot)
                .and_then(Value::as_object)
                .ok_or_else(reject)?;
            let (class, uid) = guid(
                obj.get("__classid")
                    .and_then(Value::as_str)
                    .ok_or_else(reject)?,
            )
            .ok_or_else(reject)?;
            if obj.len() > 65 {
                return Err(reject());
            }
            let mut settings = BTreeMap::new();
            for (name, value) in obj {
                if name == "__classid" {
                    continue;
                }
                if !identifier(name) {
                    return Err(reject());
                }
                settings.insert(name.clone(), Number::parsed(value).ok_or_else(reject)?);
                total += 1;
                if total > 256 {
                    return Err(reject());
                }
            }
            processors.push(Processor {
                slot,
                class,
                uid,
                settings,
            });
        }
        Ok(Self { processors })
    }
    fn digests(&self) -> (String, String) {
        let mut models = Sha256::new();
        models.update(b"fsp.rumble.models.v1\0");
        let mut settings = Sha256::new();
        settings.update(b"fsp.rumble.processor-settings.v1\0");
        for p in &self.processors {
            for h in [&mut models, &mut settings] {
                h.update([p.slot.len() as u8]);
                h.update(p.slot.as_bytes());
                h.update(p.uid);
            }
            settings.update((p.settings.len() as u16).to_be_bytes());
            for (name, value) in &p.settings {
                settings.update([name.len() as u8]);
                settings.update(name.as_bytes());
                value.hash(&mut settings);
            }
        }
        (
            format!("{:x}", models.finalize()),
            format!("{:x}", settings.finalize()),
        )
    }
    fn fields(&self) -> Value {
        let (models, settings) = self.digests();
        json!({"processors":self.processors.iter().map(|p|json!({"slot":p.slot,"class_uid":p.class,"zero_guid":p.class==ZERO,"settings":p.settings.iter().map(|(name,value)|json!({"name":name,"value":value.wire()})).collect::<Vec<_>>()})).collect::<Vec<_>>(),"processor_count":26,"numeric_setting_count":self.processors.iter().map(|p|p.settings.len()).sum::<usize>(),"models_sha256":models,"settings_sha256":settings,"numeric_encoding":ENCODING,"complete":true,"scope":"direct_serialized_processor_ids_and_numeric_settings"})
    }
    fn from_wire(value: &Value) -> Option<Self> {
        let rows = value["processors"].as_array()?;
        if rows.len() != 26 {
            return None;
        }
        let mut processors = Vec::new();
        let mut total = 0;
        for (i, row) in rows.iter().enumerate() {
            let obj = row.as_object()?;
            if obj.len() != 4
                || !["slot", "class_uid", "zero_guid", "settings"]
                    .iter()
                    .all(|k| obj.contains_key(*k))
                || row["slot"] != slot(i)
            {
                return None;
            }
            let text = row["class_uid"].as_str()?;
            let (class, uid) = guid(text)?;
            if class != text || row["zero_guid"] != (class == ZERO) {
                return None;
            }
            let values = row["settings"].as_array()?;
            if values.len() > 64 {
                return None;
            }
            let mut settings = BTreeMap::new();
            let mut previous = "";
            for v in values {
                if v.as_object()?.len() != 2 {
                    return None;
                }
                let name = v["name"].as_str()?;
                if !identifier(name) || name <= previous {
                    return None;
                }
                previous = name;
                settings.insert(name.to_owned(), Number::from_wire(&v["value"])?);
                total += 1;
                if total > 256 {
                    return None;
                }
            }
            processors.push(Processor {
                slot: slot(i),
                class,
                uid,
                settings,
            });
        }
        if value["numeric_setting_count"].as_u64() != Some(total) {
            return None;
        }
        Some(Self { processors })
    }
}
pub(super) fn read(bytes: &[u8]) -> Result<Value, String> {
    Ok(Projection::decode(bytes)?.fields())
}
pub(super) fn valid_receipt(instance: &Value, value: &Value) -> bool {
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
        "scope",
        "complete",
        "processors",
        "processor_count",
        "numeric_setting_count",
        "models_sha256",
        "settings_sha256",
        "numeric_encoding",
        "value_semantics",
        "native_access_metadata_verified",
        "mutation_capability",
        "audio_verified",
        "persistence_verified",
    ];
    let Some(obj) = value.as_object() else {
        return false;
    };
    let mut adapted = value.clone();
    adapted["plugin"] = json!("Rumble Native");
    adapted["native_outcome"] = json!("observed");
    if obj.len() != fields.len()
        || !fields.iter().all(|k| obj.contains_key(*k))
        || adapted.to_string().len() > MAX_RESULT_BYTES
        || value["ok"] != true
        || value["outcome"] != "observed"
        || value["operation"] != "rumble_model_state_read"
        || value["schema"] != "rumble_processor_state_v1"
        || value["serialization"] != "component_alias_IStorable"
        || value["instance"] != *instance
        || value["source"] != "component_alias"
        || value["scope"] != "direct_serialized_processor_ids_and_numeric_settings"
        || value["complete"] != true
        || value["processor_count"] != 26
        || value["numeric_encoding"] != ENCODING
        || value["value_semantics"] != "sequential_snapshot"
        || [
            "native_access_metadata_verified",
            "mutation_capability",
            "audio_verified",
            "persistence_verified",
        ]
        .iter()
        .any(|k| value[*k] != false)
        || value["capture_byte_count"]
            .as_u64()
            .is_none_or(|n| n > super::MAX_CAPSULE_BYTES as u64)
        || !value["capture_sha256"].as_str().is_some_and(|s| {
            s.len() == 64
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
    {
        return false;
    }
    let Some(projection) = Projection::from_wire(value) else {
        return false;
    };
    let (m, s) = projection.digests();
    value["models_sha256"] == m && value["settings_sha256"] == s
}

#[cfg(test)]
pub(super) fn fixture() -> Value {
    let mut parameters = serde_json::Map::new();
    for i in 0..26 {
        parameters.insert(
            slot(i),
            json!({"__classid":"{00000000-0000-0000-0000-000000000000}"}),
        );
    }
    parameters.insert("input".into(), json!({"level":0.5}));
    parameters.insert("output".into(), json!({"level":0.5}));
    parameters.insert("global".into(), json!({}));
    parameters["effect1"]["__classid"] = json!("{7C15CAA7-3DE6-4862-8449-94576D48F211}");
    parameters["effect1"]["gain"] = json!(0.5);
    json!({"cid":format!("{{{CLASS}}}"),"classname":"Rumble Native","parameters":parameters,"component":{"privatePreset":"SYNTHETIC_PRIVATE_MARKER"}})
}
#[cfg(test)]
mod tests {
    use super::*;
    fn parse(v: &Value) -> Result<Value, String> {
        read(&serde_json::to_vec(v).unwrap())
    }
    #[test]
    fn rumble_processor_state_direct_ancestry_complete_and_private() {
        let v = parse(&fixture()).unwrap();
        assert_eq!(
            v["models_sha256"],
            "75dc7be06952ed2456c743ba389e83c124a547ef46f663695e10e37efd4249bb"
        );
        assert_eq!(
            v["settings_sha256"],
            "8c0e57e075635a79546ffebd799378d6d34ca88a92b34c5d928ab6885b90f0ad"
        );
        assert_eq!(v["processor_count"], 26);
        assert_eq!(v["numeric_setting_count"], 1);
        assert_eq!(
            v["processors"][0]["class_uid"],
            "7C15CAA7-3DE6-4862-8449-94576D48F211"
        );
        assert!(!v.to_string().contains("PRIVATE_MARKER"));
        let mut bad = fixture();
        bad["parameters"].as_object_mut().unwrap().remove("amp");
        bad["component"]["amp"] = json!({"__classid":ZERO});
        assert!(parse(&bad).is_err());
        let mut bad = fixture();
        bad["parameters"]["effect1"]["__classid"] = Value::Null;
        assert!(parse(&bad).is_err());
        for field in ["cid", "classname", "parameters", "component"] {
            let mut bad = fixture();
            bad[field] = json!("other");
            assert!(parse(&bad).is_err());
        }
    }
    #[test]
    fn rumble_processor_state_digest_is_projection_not_bytes() {
        let value = fixture();
        let a = parse(&value).unwrap();
        let mut bytes = serde_json::to_vec_pretty(&value).unwrap();
        bytes.push(b'\n');
        let b = read(&bytes).unwrap();
        let reordered = format!(
            r#"{{"parameters":{},"component":{},"classname":"Rumble Native","cid":"{{B9D7536C-A010-457D-A182-36E5531164C9}}"}}"#,
            value["parameters"], value["component"]
        );
        assert_eq!(a, read(reordered.as_bytes()).unwrap());
        assert_eq!(a, b);
        let mut changed = value.clone();
        changed["component"]["privatePreset"] = json!("OTHER_PRIVATE_MARKER");
        assert_eq!(a, parse(&changed).unwrap());
        changed["parameters"]["effect1"]["gain"] = json!(0.75);
        let c = parse(&changed).unwrap();
        assert_eq!(a["models_sha256"], c["models_sha256"]);
        assert_ne!(a["settings_sha256"], c["settings_sha256"]);
        changed["parameters"]["effect1"]["__classid"] =
            json!("{11111111-1111-4111-8111-111111111111}");
        let c = parse(&changed).unwrap();
        assert_ne!(a["models_sha256"], c["models_sha256"]);
        assert_ne!(a["settings_sha256"], c["settings_sha256"]);
    }
    #[test]
    fn rumble_processor_state_numeric_domains_preserve_bits_and_bounds() {
        let mut v = fixture();
        v["parameters"]["amp"]["maximum"] = json!(u64::MAX);
        v["parameters"]["amp"]["minimum"] = json!(i64::MIN);
        v["parameters"]["amp"]["zero"] = json!(-0.0);
        let a = parse(&v).unwrap();
        let rows = a["processors"][25]["settings"].as_array().unwrap();
        assert_eq!(rows[0]["value"]["decimal"], u64::MAX.to_string());
        assert_eq!(rows[1]["value"]["decimal"], i64::MIN.to_string());
        assert_eq!(rows[2]["value"]["bits_hex"], "8000000000000000");
        let text = serde_json::to_string(&v)
            .unwrap()
            .replace("18446744073709551615", "18446744073709551616");
        let a = read(text.as_bytes()).unwrap();
        assert_eq!(
            a["processors"][25]["settings"][0]["value"]["kind"],
            "float64"
        );
        for bad in [
            json!(true),
            json!("private"),
            Value::Null,
            json!({"nested":0}),
            json!([1]),
        ] {
            let mut v = fixture();
            v["parameters"]["amp"]["bad"] = bad;
            assert!(parse(&v).is_err());
        }
    }
    #[test]
    fn rumble_processor_state_full_input_and_resource_limits() {
        let bytes = serde_json::to_string(&fixture()).unwrap();
        assert!(read(format!("{bytes} {{}}").as_bytes()).is_err());
        assert!(
            read(
                bytes
                    .replace("\"cid\":", "\"cid\":\"duplicate\",\"cid\":")
                    .as_bytes()
            )
            .is_err()
        );
        let mut v = fixture();
        v["component"]["private"] = json!("x".repeat(4097));
        assert!(parse(&v).is_err());
        let mut v = fixture();
        for i in 0..65 {
            v["parameters"]["amp"][format!("v{i}")] = json!(0.5);
        }
        assert!(parse(&v).is_err());
        let mut v = fixture();
        for i in 0..5 {
            for j in 0..64 {
                v["parameters"][slot(i)][format!("v{j}")] = json!(0.5);
            }
        }
        assert!(parse(&v).is_err());
        let mut v = fixture();
        let mut deep = json!({});
        for _ in 0..34 {
            deep = json!({"a":deep});
        }
        v["component"] = deep;
        assert!(parse(&v).is_err());
        let mut v = fixture();
        v["parameters"]["amp"]["invalid-name"] = json!(1);
        assert!(parse(&v).is_err());
    }
}

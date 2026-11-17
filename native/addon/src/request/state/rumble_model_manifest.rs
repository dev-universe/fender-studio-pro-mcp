use super::*;
use serde_json::Map;

pub(super) const CLASS: &str = "B9D7536C-A010-457D-A182-36E5531164C9";
const MODEL_HASH_DOMAIN: &[u8] = b"fsp.rumble.models.v1\0";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Model {
    pub(super) slot: String,
    pub(super) class: String,
    pub(super) uid: [u8; 16],
}

pub(super) struct Document {
    root: Value,
    pub(super) models: Vec<Model>,
}

pub(super) fn slot(index: usize) -> String {
    if index == 25 {
        "amp".into()
    } else {
        format!("effect{}", index + 1)
    }
}
pub(super) fn guid(text: &str) -> Option<(String, [u8; 16])> {
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

pub(super) fn model_hasher() -> Sha256 {
    let mut hash = Sha256::new();
    hash.update(MODEL_HASH_DOMAIN);
    hash
}

pub(super) fn hash_record(hash: &mut Sha256, slot: &str, uid: &[u8; 16]) {
    hash.update([slot.len() as u8]);
    hash.update(slot.as_bytes());
    hash.update(uid);
}

impl Document {
    pub(super) fn decode(bytes: &[u8]) -> Result<Self, String> {
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
        let mut models = Vec::with_capacity(26);
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
            models.push(Model { slot, class, uid });
        }
        Ok(Self { root, models })
    }

    pub(super) fn processor(&self, index: usize) -> &Map<String, Value> {
        self.root["parameters"][&self.models[index].slot]
            .as_object()
            .expect("validated direct processor")
    }

    pub(super) fn models_sha256(&self) -> String {
        let mut hash = model_hasher();
        for model in &self.models {
            hash_record(&mut hash, &model.slot, &model.uid);
        }
        format!("{:x}", hash.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::super::rumble_processor_state::{fixture, read};
    use super::*;

    fn parse(value: &Value) -> Result<Document, String> {
        Document::decode(&serde_json::to_vec(value).unwrap())
    }

    #[test]
    fn manifest_has_exact_order_canonical_ids_and_pinned_digest() {
        let document = parse(&fixture()).unwrap();
        assert_eq!(document.models.len(), 26);
        for (index, model) in document.models.iter().enumerate() {
            assert_eq!(model.slot, slot(index));
            assert_eq!(guid(&model.class).unwrap().1, model.uid);
            assert!(!model.class.contains('{'));
        }
        assert_eq!(
            document.models_sha256(),
            "75dc7be06952ed2456c743ba389e83c124a547ef46f663695e10e37efd4249bb"
        );
        assert_eq!(
            document.models_sha256(),
            read(&serde_json::to_vec(&fixture()).unwrap()).unwrap()["models_sha256"]
        );
    }

    #[test]
    fn manifest_identity_accepts_nonnumeric_siblings_without_relaxing_numeric_reader() {
        let baseline = parse(&fixture()).unwrap();
        for sibling in [
            json!("SYNTHETIC_PRIVATE_TEXT"),
            json!({"private":[null,true,"text"]}),
            json!([0, "private"]),
            json!(true),
            Value::Null,
        ] {
            let mut value = fixture();
            value["parameters"]["amp"]["futureValue"] = sibling;
            let bytes = serde_json::to_vec(&value).unwrap();
            let document = Document::decode(&bytes).unwrap();
            assert_eq!(document.models, baseline.models);
            assert_eq!(document.models_sha256(), baseline.models_sha256());
            assert!(read(&bytes).is_err());
            assert!(!document.models_sha256().contains("PRIVATE"));
        }
    }

    #[test]
    fn manifest_identity_is_independent_of_numeric_limits_and_setting_keys() {
        let baseline = parse(&fixture()).unwrap();
        let mut value = fixture();
        for i in 0..5 {
            for j in 0..65 {
                value["parameters"][slot(i)][format!("future-setting-{j}")] = json!("private");
            }
        }
        let bytes = serde_json::to_vec(&value).unwrap();
        let document = Document::decode(&bytes).unwrap();
        assert_eq!(document.models, baseline.models);
        assert_eq!(document.models_sha256(), baseline.models_sha256());
        assert!(read(&bytes).is_err());
        assert_eq!(document.processor(0).len(), 67);
    }

    #[test]
    fn manifest_model_identity_changes_only_for_direct_selected_ids() {
        let mut value = fixture();
        let baseline = parse(&value).unwrap();
        value["parameters"]["effect1"]["gain"] = json!(0.5000000000000001_f64);
        value["parameters"]["amp"]["other"] = json!(-0.0_f64);
        value["component"]["amp"] = json!({"__classid":"11111111-1111-4111-8111-111111111111"});
        assert_eq!(parse(&value).unwrap().models, baseline.models);
        for index in 0..26 {
            let mut changed = value.clone();
            changed["parameters"][slot(index)]["__classid"] =
                json!("11111111-1111-4111-8111-111111111111");
            let document = parse(&changed).unwrap();
            assert_ne!(document.models, baseline.models);
            assert_ne!(document.models_sha256(), baseline.models_sha256());
        }
    }

    #[test]
    fn manifest_requires_exact_header_and_all_immediate_slots() {
        for key in ["cid", "classname", "parameters", "component"] {
            let mut value = fixture();
            value.as_object_mut().unwrap().remove(key);
            assert!(parse(&value).is_err());
            let mut value = fixture();
            value[key] = json!("invalid");
            assert!(parse(&value).is_err());
        }
        let mut value = fixture();
        value["extra"] = json!(0);
        assert!(parse(&value).is_err());
        for key in ["input", "output", "global"] {
            let mut value = fixture();
            value["parameters"][key] = Value::Null;
            assert!(parse(&value).is_err());
        }
        for index in 0..26 {
            let mut value = fixture();
            let key = slot(index);
            let removed = value["parameters"]
                .as_object_mut()
                .unwrap()
                .remove(&key)
                .unwrap();
            value["component"][&key] = removed;
            assert!(parse(&value).is_err());
            let mut value = fixture();
            value["parameters"][&key]["__classid"] = json!("not-a-guid");
            assert!(parse(&value).is_err());
            let mut value = fixture();
            value["parameters"][&key] = json!([]);
            assert!(parse(&value).is_err());
        }
    }

    #[test]
    fn manifest_rejects_duplicate_trailing_and_non_utf8_whole_input() {
        let text = serde_json::to_string(&fixture()).unwrap();
        for bytes in [
            format!("{text} {{}}").into_bytes(),
            text.replace("\"cid\":", "\"cid\":\"private\",\"cid\":")
                .into_bytes(),
            text.replace("\"gain\":", "\"gain\":1,\"gain\":")
                .into_bytes(),
            text.replace(
                "\"privatePreset\":",
                "\"privatePreset\":0,\"privatePreset\":",
            )
            .into_bytes(),
            vec![0xff, 0xfe, 0x7b, 0x7d],
        ] {
            assert!(Document::decode(&bytes).is_err());
        }
    }

    #[test]
    fn manifest_preserves_whole_input_resource_limits() {
        let mut value = fixture();
        value["component"]["private"] = json!("x".repeat(4097));
        assert!(parse(&value).is_err());
        let mut value = fixture();
        let mut deep = json!({});
        for _ in 0..34 {
            deep = json!({"a":deep});
        }
        value["component"] = deep;
        assert!(parse(&value).is_err());
        let mut value = fixture();
        value["component"]["many"] = json!(vec![0; 8193]);
        assert!(parse(&value).is_err());
        assert!(Document::decode(&vec![b' '; super::super::MAX_CAPSULE_BYTES + 1]).is_err());
    }

    #[test]
    fn manifest_canonicalization_does_not_change_hash_grammar() {
        let mut value = fixture();
        let baseline = parse(&value).unwrap();
        value["cid"] = json!(CLASS.to_ascii_lowercase());
        value["parameters"]["effect1"]["__classid"] = json!("7c15caa7-3de6-4862-8449-94576d48f211");
        assert_eq!(parse(&value).unwrap().models, baseline.models);
        assert_eq!(
            parse(&value).unwrap().models_sha256(),
            baseline.models_sha256()
        );
        let mut bytes = serde_json::to_vec_pretty(&value).unwrap();
        bytes.push(b'\n');
        assert_eq!(
            Document::decode(&bytes).unwrap().models_sha256(),
            baseline.models_sha256()
        );
        for invalid in [
            "",
            "{",
            "{00000000-0000-0000-0000-000000000000",
            "00000000000000000000000000000000",
            "00000000-0000-0000-0000-00000000000G",
        ] {
            value["parameters"]["amp"]["__classid"] = json!(invalid);
            assert!(parse(&value).is_err());
        }
    }
}

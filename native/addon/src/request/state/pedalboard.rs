use super::{rumble_model_manifest::guid, rumble_serialization::validated_json, sha256};
use serde_json::{Value, json};

pub(super) const CLASS: &str = "BC9129B4-EC67-41B8-96DE-EC128D5FE54D";
const MAX_RACK_BYTES: usize = 64 * 1024;

pub(super) fn rack(bytes: &[u8]) -> Option<Value> {
    if bytes.len() > MAX_RACK_BYTES {
        return None;
    }
    let value = validated_json(bytes).ok()?;
    let object = value.as_object()?;
    let parameters = object.get("parameters")?.as_object()?;
    if object.len() != 3
        || object.get("classname")?.as_str()? != "Pedalboard"
        || guid(object.get("cid")?.as_str()?)?.0 != CLASS
        || parameters.len() != 10
        || !parameters.get("input")?.is_object()
        || !parameters.get("output")?.is_object()
    {
        return None;
    }

    let mut slots = Vec::with_capacity(8);
    for slot in 1..=8 {
        let state = parameters.get(&format!("stomp{slot}"))?.as_object()?;
        let (class, uid) = guid(state.get("__classid")?.as_str()?)?;
        let (model_class, payload_sha256) = if uid == [0; 16] {
            if state.len() != 1 {
                return None;
            }
            (None, None)
        } else {
            let bytes = serde_json::to_vec(state).ok()?;
            (Some(class), Some(sha256(&bytes)))
        };
        slots.push(json!({"slot":slot,"model_class":model_class,"payload_sha256":payload_sha256}));
    }
    Some(json!({"schema":"pedalboard_slot_fingerprints_v1","slots":slots}))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MODEL: &str = "12345678-1234-1234-1234-123456789ABC";

    fn fixture() -> Value {
        let mut value = json!({"classname":"Pedalboard","cid":CLASS,"parameters":{
            "input":{},"output":{}
        }});
        for slot in 1..=8 {
            value["parameters"][format!("stomp{slot}")] =
                json!({"__classid":"{00000000-0000-0000-0000-000000000000}"});
        }
        value["parameters"]["stomp1"] = json!({"__classid":MODEL,"gain":32.4,"active":1});
        value["parameters"]["stomp6"] = json!({"__classid":MODEL,"gain":68.3,"active":0});
        value
    }

    fn observe(value: &Value) -> Option<Value> {
        rack(&serde_json::to_vec(value).unwrap())
    }

    #[test]
    fn slot_fingerprints_follow_payload_between_equal_models_and_vacancies() {
        let mut state = fixture();
        let baseline = observe(&state).unwrap();
        assert_eq!(baseline["schema"], "pedalboard_slot_fingerprints_v1");
        let slots = baseline["slots"].as_array().unwrap();
        assert_eq!(slots.len(), 8);
        for (i, slot) in slots.iter().enumerate() {
            assert_eq!(slot["slot"], i + 1);
        }
        assert_eq!(slots[0]["model_class"], MODEL);
        assert_eq!(slots[0]["model_class"], slots[5]["model_class"]);
        assert_ne!(slots[0]["payload_sha256"], slots[5]["payload_sha256"]);
        assert!(slots[1]["model_class"].is_null());
        assert!(slots[1]["payload_sha256"].is_null());

        let parameters = &mut state["parameters"];
        let first = parameters["stomp1"].take();
        parameters["stomp1"] = parameters["stomp6"].take();
        parameters["stomp6"] = parameters["stomp2"].take();
        parameters["stomp2"] = first;
        let moved = observe(&state).unwrap();
        for (original, new) in [(0, 1), (5, 0), (1, 5), (7, 7)] {
            for key in ["model_class", "payload_sha256"] {
                assert_eq!(slots[original][key], moved["slots"][new][key]);
            }
        }
    }

    #[test]
    fn incomplete_or_ambiguous_serialization_does_not_project_a_rack() {
        for (key, payload) in [
            ("stomp2", json!({"__classid":"invalid"})),
            (
                "stomp2",
                json!({"__classid":"00000000-0000-0000-0000-000000000000","active":1}),
            ),
            ("stomp3", Value::Null),
            ("extra_slot", json!({"__classid":MODEL})),
        ] {
            let mut value = fixture();
            value["parameters"][key] = payload;
            assert!(observe(&value).is_none());
        }
        let source = serde_json::to_string(&fixture()).unwrap();
        let duplicate = source.replacen("\"gain\":32.4", "\"gain\":32.4,\"gain\":16.2", 1);
        assert_ne!(source, duplicate);
        assert!(rack(duplicate.as_bytes()).is_none());
        assert!(rack(&vec![b' '; MAX_RACK_BYTES + 1]).is_none());
    }
}

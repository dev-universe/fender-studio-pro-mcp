use super::*;
use crate::operation::{PluginState, PluginStateAction};
use sha2::{Digest, Sha256};

pub(super) fn requested(operation: &Operation) -> bool {
    matches!(operation, Operation::PluginState(q) if matches!(q.action,PluginStateAction::ReadRumbleState))
}
pub(super) fn invalidate(value: &mut Value) {
    *value = json!({"ok":false,"outcome":"not_applied","native_outcome":"not_applied","native_access_metadata_verified":false,"mutation_capability":false,"audio_verified":false,"persistence_verified":false,"error":{"code":"rumble_processor_state_unavailable","message":"The bounded Rumble processor-state observation was not established"}});
}
fn number(value: &Value) -> Option<(u8, [u8; 8])> {
    if value.as_object()?.len() != 2 {
        return None;
    }
    match value["kind"].as_str()? {
        "unsigned_integer" => {
            let s = value["decimal"].as_str()?;
            let n = s.parse::<u64>().ok()?;
            (n.to_string() == s).then_some((b'U', n.to_be_bytes()))
        }
        "signed_integer" => {
            let s = value["decimal"].as_str()?;
            let n = s.parse::<i64>().ok()?;
            (n < 0 && n.to_string() == s).then_some((b'I', n.to_be_bytes()))
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
            let n = u64::from_str_radix(s, 16).ok()?;
            f64::from_bits(n)
                .is_finite()
                .then_some((b'F', n.to_be_bytes()))
        }
        _ => None,
    }
}
fn hashes(value: &Value) -> Option<(String, String)> {
    let rows = value["processors"].as_array()?;
    if rows.len() != 26 {
        return None;
    }
    let mut models = b"fsp.rumble.models.v1\0".to_vec();
    let mut settings = b"fsp.rumble.processor-settings.v1\0".to_vec();
    let mut total = 0u64;
    for (i, row) in rows.iter().enumerate() {
        let slot = if i == 25 {
            "amp".to_owned()
        } else {
            format!("effect{}", i + 1)
        };
        if !exact_gui_object(row, &["slot", "class_uid", "zero_guid", "settings"])
            || row["slot"] != slot
        {
            return None;
        }
        let uid = row["class_uid"].as_str()?;
        if uid.len() != 36
            || uid.bytes().enumerate().any(|(j, b)| {
                if [8, 13, 18, 23].contains(&j) {
                    b != b'-'
                } else {
                    !b.is_ascii_digit() && !(b'A'..=b'F').contains(&b)
                }
            })
            || row["zero_guid"] != (uid == "00000000-0000-0000-0000-000000000000")
        {
            return None;
        }
        let compact = uid.replace('-', "");
        let mut raw = Vec::with_capacity(16);
        for j in 0..16 {
            raw.push(u8::from_str_radix(&compact[j * 2..j * 2 + 2], 16).ok()?);
        }
        for bytes in [&mut models, &mut settings] {
            bytes.push(slot.len() as u8);
            bytes.extend_from_slice(slot.as_bytes());
            bytes.extend_from_slice(&raw);
        }
        let values = row["settings"].as_array()?;
        if values.len() > 64 {
            return None;
        }
        settings.extend_from_slice(&(values.len() as u16).to_be_bytes());
        let mut previous = "";
        for v in values {
            if !exact_gui_object(v, &["name", "value"]) {
                return None;
            }
            let name = v["name"].as_str()?;
            if name.is_empty()
                || name.len() > 64
                || !name.as_bytes()[0].is_ascii_alphabetic()
                || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                || name <= previous
            {
                return None;
            }
            previous = name;
            let (tag, bytes) = number(&v["value"])?;
            settings.push(name.len() as u8);
            settings.extend_from_slice(name.as_bytes());
            settings.push(tag);
            settings.extend_from_slice(&bytes);
            total += 1;
            if total > 256 {
                return None;
            }
        }
    }
    if value["numeric_setting_count"].as_u64() != Some(total) {
        return None;
    }
    Some((
        format!("{:x}", Sha256::digest(models)),
        format!("{:x}", Sha256::digest(settings)),
    ))
}
pub(super) fn valid_get(query: &PluginState, value: &Value) -> bool {
    let fields = [
        "ok",
        "outcome",
        "native_outcome",
        "operation",
        "schema",
        "serialization",
        "instance",
        "source",
        "plugin",
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
    if !exact_gui_object(value, &fields)
        || value.to_string().len() > 32768
        || value["ok"] != true
        || value["outcome"] != "observed"
        || value["native_outcome"] != "observed"
        || value["operation"] != "rumble_model_state_read"
        || value["schema"] != "rumble_processor_state_v1"
        || value["serialization"] != "component_alias_IStorable"
        || value["source"] != "component_alias"
        || value["plugin"] != "Rumble Native"
        || value["scope"] != "direct_serialized_processor_ids_and_numeric_settings"
        || value["complete"] != true
        || value["processor_count"] != 26
        || value["numeric_encoding"] != "json_parsed_u64_i64_f64_bits_v1"
        || value["value_semantics"] != "sequential_snapshot"
        || [
            "native_access_metadata_verified",
            "mutation_capability",
            "audio_verified",
            "persistence_verified",
        ]
        .iter()
        .any(|k| value[*k] != false)
        || !exact_gui_object(
            &value["instance"],
            &["object_uid", "class_uid", "native_class", "object_id"],
        )
        || value["instance"]["object_uid"] != query.expected_instance
        || value["instance"]["class_uid"] != "B9D7536C-A010-457D-A182-36E5531164C9"
        || value["instance"]["native_class"] != "AudioEffect"
        || value["instance"]["object_id"] != "Rumble Native"
        || value["capture_byte_count"]
            .as_u64()
            .is_none_or(|n| n > 16777216)
        || !value["capture_sha256"].as_str().is_some_and(|s| {
            s.len() == 64
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
    {
        return false;
    }
    let Some((models, settings)) = hashes(value) else {
        return false;
    };
    value["models_sha256"] == models && value["settings_sha256"] == settings
}

#[cfg(test)]
mod tests {
    use super::*;
    fn query() -> PluginState {
        serde_json::from_value(json!({"channel":"Scratch","slot":0,"plugin":"Rumble Native","expected_instance":"12345678-1234-1234-1234-123456789ABC","action":"read_rumble_state"})).unwrap()
    }
    fn receipt() -> Value {
        let rows=(0..26).map(|i|json!({"slot":if i==25 {"amp".to_owned()}else{format!("effect{}",i+1)},"class_uid":"00000000-0000-0000-0000-000000000000","zero_guid":true,"settings":[]})).collect::<Vec<_>>();
        let mut v = json!({"ok":true,"outcome":"observed","native_outcome":"observed","operation":"rumble_model_state_read","schema":"rumble_processor_state_v1","serialization":"component_alias_IStorable","instance":{"object_uid":query().expected_instance,"class_uid":"B9D7536C-A010-457D-A182-36E5531164C9","native_class":"AudioEffect","object_id":"Rumble Native"},"source":"component_alias","plugin":"Rumble Native","capture_sha256":"a".repeat(64),"capture_byte_count":100,"scope":"direct_serialized_processor_ids_and_numeric_settings","complete":true,"processors":rows,"processor_count":26,"numeric_setting_count":0,"numeric_encoding":"json_parsed_u64_i64_f64_bits_v1","value_semantics":"sequential_snapshot","native_access_metadata_verified":false,"mutation_capability":false,"audio_verified":false,"persistence_verified":false});
        let (m, s) = hashes(&v).unwrap();
        v["models_sha256"] = json!(m);
        v["settings_sha256"] = json!(s);
        v
    }
    #[test]
    fn rumble_processor_state_checks_complete_projection_and_digest() {
        let v = receipt();
        assert_eq!(
            v["models_sha256"],
            "a7995d1e57ccc8f7983c4c45a2c6e8a0931cdda5617d1a7453d34d716302269b"
        );
        assert_eq!(
            v["settings_sha256"],
            "628ed201c83827459491e2ff4db0818e9c4f01731e44e803a0da290d3461503e"
        );
        assert!(valid_get(&query(), &v));
        for (key, value) in [
            ("settings_sha256", json!("b".repeat(64))),
            ("models_sha256", json!("b".repeat(64))),
            ("complete", json!(false)),
            ("native_access_metadata_verified", json!(true)),
            ("token", json!("private")),
            ("numeric_setting_count", json!(1)),
        ] {
            let mut bad = v.clone();
            bad[key] = value;
            assert!(!valid_get(&query(), &bad));
        }
        let mut bad = v.clone();
        bad["processors"][0]["class_uid"] = json!("11111111-1111-4111-8111-111111111111");
        bad["processors"][0]["zero_guid"] = json!(false);
        assert!(!valid_get(&query(), &bad));
        let mut raw = v.clone();
        raw["capture_sha256"] = json!("b".repeat(64));
        assert!(valid_get(&query(), &raw));
    }
    #[test]
    fn rumble_processor_state_numbers_are_canonical_and_exact() {
        for v in [
            json!({"kind":"unsigned_integer","decimal":"18446744073709551615"}),
            json!({"kind":"signed_integer","decimal":"-9223372036854775808"}),
            json!({"kind":"float64","bits_hex":"8000000000000000"}),
        ] {
            assert!(number(&v).is_some());
        }
        for v in [
            json!({"kind":"signed_integer","decimal":"0"}),
            json!({"kind":"unsigned_integer","decimal":"01"}),
            json!({"kind":"float64","bits_hex":"7ff0000000000000"}),
            json!({"kind":"unsigned_integer","decimal":"18446744073709551616"}),
        ] {
            assert!(number(&v).is_none());
        }
    }
    #[test]
    fn rumble_processor_state_late_invalidation_erases_whole_payload() {
        let mut v = receipt();
        super::super::invalidate_gui_evidence(&Operation::PluginState(query()), &mut v);
        assert_eq!(v["ok"], false);
        for key in [
            "processors",
            "instance",
            "models_sha256",
            "settings_sha256",
            "capture_sha256",
            "schema",
            "source",
        ] {
            assert!(v.get(key).is_none());
        }
    }
    #[test]
    fn rumble_processor_state_document_guard_precedes_owner_queue() {
        use std::{
            future::Future,
            task::{Context, Poll, Waker},
        };
        for expected_document in [None, Some(String::new())] {
            let request = Request {
                operation: Operation::PluginState(query()),
                expected_document,
            };
            let (sender, receiver) = mpsc::sync_channel(1);
            let handle = RuntimeHandle { sender };
            let mut future = std::pin::pin!(handle.execute(request));
            assert!(matches!(
                future
                    .as_mut()
                    .poll(&mut Context::from_waker(Waker::noop())),
                Poll::Ready(Err(_))
            ));
            assert!(receiver.try_recv().is_err());
        }
    }
}

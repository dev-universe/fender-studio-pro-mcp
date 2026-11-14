use super::*;
use crate::operation::ChorderChordsGet;

fn digest(value: &Value) -> bool {
    value.as_str().is_some_and(|text| {
        text.len() == 64
            && text
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

pub(super) fn valid_get(query: &ChorderChordsGet, value: &Value) -> bool {
    let count = query.limit.min(128u32.saturating_sub(query.offset));
    let next = query.offset + count;
    if !exact_gui_object(
        value,
        &[
            "ok",
            "outcome",
            "native_outcome",
            "operation",
            "scope",
            "schema",
            "serialization",
            "instance",
            "source",
            "total",
            "offset",
            "limit",
            "count",
            "next",
            "chords",
            "capture_sha256",
            "capture_byte_count",
            "model_sha256",
            "value_semantics",
            "mutation_capability",
            "persistence_verified",
            "audio_verified",
            "plugin",
        ],
    ) || value["ok"] != true
        || value["outcome"] != "observed"
        || value["native_outcome"] != "observed"
        || value["operation"] != "chorder_chords_get"
        || value["scope"] != "component_alias_IStorable_chorder_chords"
        || value["schema"] != "chorder_music_effect_preset_v1"
        || value["serialization"] != "component_alias_IStorable"
        || value["source"] != "component_alias"
        || value["plugin"] != "Chorder"
        || value["total"] != 128
        || value["offset"] != query.offset
        || value["limit"] != query.limit
        || value["count"] != count
        || value["next"] != if next < 128 { json!(next) } else { Value::Null }
        || value["value_semantics"] != "sequential_snapshot"
        || value["mutation_capability"] != false
        || value["persistence_verified"] != false
        || value["audio_verified"] != false
        || !exact_gui_object(
            &value["instance"],
            &["object_uid", "class_uid", "native_class", "object_id"],
        )
        || value["instance"]["object_uid"] != query.expected_instance
        || value["instance"]["class_uid"] != "D2D9B002-12FC-4F1D-AFDA-70336D1DC108"
        || ["native_class", "object_id"].iter().any(|key| {
            value["instance"][*key]
                .as_str()
                .is_none_or(|text| text.encode_utf16().count() > 512)
        })
        || !digest(&value["capture_sha256"])
        || !digest(&value["model_sha256"])
        || !value["capture_byte_count"]
            .as_u64()
            .is_some_and(|count| (1..=1_048_576).contains(&count))
    {
        return false;
    }
    let Some(chords) = value["chords"]
        .as_array()
        .filter(|chords| chords.len() == count as usize)
    else {
        return false;
    };
    chords.iter().enumerate().all(|(index, chord)| {
        exact_gui_object(chord, &["input_pitch", "output_pitches"])
            && chord["input_pitch"] == query.offset + index as u32
            && chord["output_pitches"].as_array().is_some_and(|pitches| {
                pitches.len() <= 128
                    && pitches
                        .iter()
                        .all(|pitch| pitch.as_u64().is_some_and(|pitch| pitch <= 127))
                    && pitches
                        .windows(2)
                        .all(|pair| pair[0].as_u64() < pair[1].as_u64())
            })
    })
}

pub(super) fn invalidate_get(value: &mut Value) {
    let error = value.get("error").filter(|error| {
        exact_gui_object(error, &["code", "message"])
            && error["code"].as_str().is_some_and(|text| !text.is_empty() && text.len() <= 64)
            && error["message"].as_str().is_some_and(|text| text.encode_utf16().count() <= 512)
    }).cloned().unwrap_or_else(|| json!({"code":"invalid_chorder_chords_read","message":"The stored Chorder model observation was not established"}));
    *value = json!({"ok":false,"outcome":"not_applied","native_outcome":"not_applied",
        "mutation_capability":false,"persistence_verified":false,"audio_verified":false,"error":error});
}

#[cfg(test)]
mod tests;

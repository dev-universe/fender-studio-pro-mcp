use serde_json::{Value, json};

use crate::{
    bridge::WireResponse,
    model::{HostContext, OperationError, Outcome},
    operation::MidiProgramPartAdd,
};

const TWIN_CLASS: &str = "91AF218E-111A-44CB-ABFE-F7CB065FF157";
const SELECTION_PREFLIGHT: &str = "MIDI program parts require a known empty arrangement selection";

pub(super) fn smf(change: &MidiProgramPartAdd) -> [u8; 46] {
    // Format 0, one track, 480 ticks per quarter note. Keep the bank bytes
    // spaced by 120 ticks. A provisional velocity-1 note at beat 2.5 makes
    // 8.1.1 create a part. The guarded host operation removes that note.
    [
        b'M',
        b'T',
        b'h',
        b'd',
        0,
        0,
        0,
        6,
        0,
        0,
        0,
        1,
        0x01,
        0xe0,
        b'M',
        b'T',
        b'r',
        b'k',
        0,
        0,
        0,
        24,
        0,
        0xb0,
        0x00,
        change.bank_msb,
        120,
        0xb0,
        0x20,
        change.bank_lsb,
        120,
        0xc0,
        change.program,
        0x87,
        0x40,
        0x90,
        60,
        1,
        30,
        0x80,
        60,
        0,
        0,
        0xff,
        0x2f,
        0,
    ]
}

fn valid(change: &MidiProgramPartAdd, basename: &str, result: &Value) -> bool {
    let Some(stem) = basename.strip_suffix(".mid") else {
        return false;
    };
    let part = &result["part"];
    result.as_object().is_some_and(|value| value.len() == 13)
        && part.as_object().is_some_and(|value| value.len() == 4)
        && result["stage"] == "complete"
        && result["import_return_type"] == "number"
        && result["track"] == change.track
        && result["expected_instance"] == change.expected_instance
        && result["class_uid"] == TWIN_CLASS
        && result["channel"] == 1
        && result["bank_msb"] == change.bank_msb
        && result["bank_lsb"] == change.bank_lsb
        && result["program"] == change.program
        && result["atomic"] == false
        && result["program_events_verified"] == false
        && result["delivery_verified"] == false
        && part["name"] == format!("{stem} CH1")
        && part["start"].as_f64() == Some(0.0)
        && part["end"]
            .as_f64()
            .is_some_and(|end| end.is_finite() && end > 0.0)
        && part["note_count"] == 0
}

pub(super) fn unverified(change: &MidiProgramPartAdd, basename: &str, stage: &str) -> Value {
    let part_name = basename
        .strip_suffix(".mid")
        .map(|stem| format!("{stem} CH1"));
    json!({"stage":stage,"track":change.track,"expected_part_name":part_name,"atomic":false,"program_events_verified":false,"delivery_verified":false})
}

pub(super) fn classify(
    change: &MidiProgramPartAdd,
    basename: &str,
    context: &HostContext,
    response: &WireResponse,
) -> (Outcome, Value, Option<OperationError>) {
    let same_context = response.session == context.session && response.document == context.document;
    let result = &response.result;
    let detail_code = result["detail_code"].as_str();
    let selection_detail =
        detail_code.filter(|code| matches!(*code, "selection_unavailable" | "selection_not_empty"));
    if response.ok && same_context && valid(change, basename, result) {
        return (Outcome::AppliedVerified, result.clone(), None);
    }
    if response.ok
        && same_context
        && result.as_object().is_some_and(|value| {
            (value.len() == 3 && !value.contains_key("detail_code"))
                || (value.len() == 4
                    && selection_detail.is_some()
                    && result["reason"] == SELECTION_PREFLIGHT)
        })
        && result["stage"] == "preflight"
        && result["native_outcome"] == "not_applied"
    {
        let reason = result["reason"]
            .as_str()
            .filter(|reason| reason.len() <= 256 && !reason.chars().any(char::is_control))
            .unwrap_or("invalid_precondition");
        let mut observation = json!({"stage":"preflight","reason":reason});
        if let Some(code) = selection_detail {
            observation["detail_code"] = json!(code);
        }
        return (
            Outcome::NotApplied,
            observation,
            Some(OperationError::new(
                "missing_precondition",
                format!("MIDI program part preflight: {reason}"),
            )),
        );
    }
    if !response.ok
        && matches!(
            response.code.as_deref(),
            Some(
                "session_changed"
                    | "document_changed"
                    | "missing_precondition"
                    | "context_required"
                    | "protocol_mismatch"
            )
        )
    {
        return (
            Outcome::NotApplied,
            Value::Null,
            Some(super::host_error(response)),
        );
    }
    let stage = result["stage"]
        .as_str()
        .filter(|stage| {
            matches!(
                *stage,
                "import"
                    | "seed_verification"
                    | "note_cleanup"
                    | "final_verification"
                    | "publication"
            )
        })
        .unwrap_or("verification");
    let mut observation = unverified(change, basename, stage);
    if response.ok
        && same_context
        && stage == "import"
        && result["native_outcome"] == "unknown"
        && let Some(code) = detail_code.filter(|code| {
            matches!(
                *code,
                "fixture_request_invalid"
                    | "target_track_unresolved"
                    | "target_track_not_music"
                    | "track_iterator_unavailable"
                    | "track_has_events"
                    | "track_iterator_exhaustion_unknown"
                    | "importer_unavailable"
                    | "import_time_unavailable"
                    | "fixture_url_unavailable"
                    | "fixture_file_unavailable"
                    | "import_native_lease_changed"
                    | "import_receipt_invalid"
                    | "track_task_unavailable"
                    | "track_task_arm_failed"
                    | "track_task_dispatch_unconfirmed"
                    | "track_task_receipt_unavailable"
                    | "track_task_lease_rejected"
            )
        })
    {
        observation["detail_code"] = json!(code);
    }
    (
        Outcome::Unknown,
        observation,
        Some(OperationError::new(
            "outcome_unknown",
            "The note-free part was not verified. Inspect the named track and its notes before retrying.",
        )),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operation::Operation;

    const INSTANCE: &str = "12345678-1234-1234-1234-123456789ABC";
    const FIXTURE: &str = "mcp-midi-program-12345678-1234-4000-8000-123456789abc.mid";

    fn input() -> Value {
        json!({"track":"Keys","expected_instance":INSTANCE,"bank_msb":0,"bank_lsb":0,"program":5})
    }

    fn change() -> MidiProgramPartAdd {
        serde_json::from_value(input()).unwrap()
    }

    #[test]
    fn only_typed_midi_values_are_accepted_and_the_smf_is_fixed() {
        let expected: &[u8] = b"MThd\0\0\0\x06\0\0\0\x01\x01\xe0MTrk\0\0\0\x18\0\xb0\0\0\x78\xb0\x20\0\x78\xc0\x05\x87\x40\x90\x3c\x01\x1e\x80\x3c\0\0\xff\x2f\0";
        assert_eq!(smf(&change()), expected);
        let changed = smf(&MidiProgramPartAdd {
            bank_msb: 127,
            bank_lsb: 16,
            program: 0,
            ..change()
        });
        assert_eq!(
            changed
                .iter()
                .zip(expected)
                .enumerate()
                .filter_map(|(i, (a, b))| (a != b).then_some((i, *a)))
                .collect::<Vec<_>>(),
            [(25, 127), (29, 16), (32, 0)]
        );
        for name in ["bank_msb", "bank_lsb", "program"] {
            let mut request = input();
            request[name] = json!(128);
            assert!(
                Operation::from_parts("midi_program_part_add", request)
                    .unwrap()
                    .validate()
                    .is_err()
            );
        }
        for name in ["fixture", "path", "url", "channel", "raw_midi", "position"] {
            let mut request = input();
            request[name] = json!("value");
            assert!(Operation::from_parts("midi_program_part_add", request).is_err());
        }
    }

    #[test]
    fn a_numeric_import_return_without_exact_no_note_readback_is_unknown() {
        let context: HostContext = serde_json::from_value(json!({
            "session":"session","document":"document","open":true,"protocol":2
        }))
        .unwrap();
        let evidence = json!({
            "stage":"complete","import_return_type":"number","track":"Keys","class_uid":TWIN_CLASS,
            "expected_instance":INSTANCE,"channel":1,"bank_msb":0,"bank_lsb":0,"program":5,
            "part":{"name":FIXTURE.replace(".mid", " CH1"),"start":0,"end":1.281,"note_count":0},
            "atomic":false,"program_events_verified":false,"delivery_verified":false
        });
        let mut response: WireResponse = serde_json::from_value(json!({
            "protocol":2,"id":"test","session":"session","document":"document","ok":true
        }))
        .unwrap();
        let status = |response: &WireResponse| classify(&change(), FIXTURE, &context, response).0;
        response.result =
            json!({"stage":"preflight","native_outcome":"not_applied","reason":"missing track"});
        assert_eq!(status(&response), Outcome::NotApplied);
        response.result = json!({"stage":"import","import_return_type":"number"});
        assert_eq!(status(&response), Outcome::Unknown);
        response.result = evidence.clone();
        assert_eq!(status(&response), Outcome::AppliedVerified);
        for (field, value) in [
            ("class_uid", json!("other")),
            ("expected_instance", json!("other")),
            ("delivery_verified", json!(true)),
            ("native_outcome", json!("unknown")),
            (
                "part",
                json!({"name":FIXTURE.replace(".mid", " CH1"),"start":0,"end":1.281,"note_count":1}),
            ),
        ] {
            response.result = evidence.clone();
            response.result[field] = value;
            assert_eq!(status(&response), Outcome::Unknown, "{field}");
        }
        response.result = evidence;
        response.document = "other".into();
        assert_eq!(status(&response), Outcome::Unknown);
    }

    #[test]
    fn import_details_preserve_certainty_and_reject_unknown_text() {
        let context: HostContext = serde_json::from_value(json!({
            "session":"session","document":"document","open":true,"protocol":2
        }))
        .unwrap();
        let mut response: WireResponse = serde_json::from_value(json!({
            "protocol":2,"id":"test","session":"session","document":"document","ok":true
        }))
        .unwrap();
        let observe = |response: &WireResponse| classify(&change(), FIXTURE, &context, response);
        response.result = json!({"stage":"import","native_outcome":"unknown","detail_code":"fixture_file_unavailable"});
        let (outcome, evidence, _) = observe(&response);
        assert_eq!(outcome, Outcome::Unknown);
        assert_eq!(evidence["detail_code"], "fixture_file_unavailable");
        response.result["detail_code"] = json!("file:///private/fixture.mid");
        let (outcome, evidence, _) = observe(&response);
        assert_eq!(outcome, Outcome::Unknown);
        assert!(evidence.get("detail_code").is_none());
        response.result = json!({"stage":"note_cleanup","native_outcome":"unknown","detail_code":"track_has_events"});
        assert!(observe(&response).1.get("detail_code").is_none());
        response.result = json!({"stage":"preflight","native_outcome":"not_applied","reason":SELECTION_PREFLIGHT,"detail_code":"selection_not_empty"});
        let (outcome, evidence, _) = observe(&response);
        assert_eq!(outcome, Outcome::NotApplied);
        assert_eq!(evidence["detail_code"], "selection_not_empty");
        response.result["reason"] = json!("file:///private/fixture.mid");
        let (outcome, evidence, _) = observe(&response);
        assert_eq!(outcome, Outcome::Unknown);
        assert!(evidence.get("detail_code").is_none());
        assert!(evidence.get("reason").is_none());
    }
}

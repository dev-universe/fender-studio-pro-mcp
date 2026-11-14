use super::*;
use std::cell::Cell;

fn observed<T>(value: T) -> Observation<T> {
    Observation::Observed { value }
}

struct Fake {
    audio_reads: Cell<u32>,
    source_reads: Cell<u32>,
    change_audio: bool,
    source_error: bool,
}
impl Backend for Fake {
    fn audio_ids(&self) -> Observation<Vec<u32>> {
        let turn = self.audio_reads.get();
        self.audio_reads.set(turn + 1);
        observed(vec![if turn > 0 && self.change_audio { 7 } else { 5 }])
    }
    fn audio(&self, object_id: u32) -> AudioDevice {
        AudioDevice {
            object_id,
            uid: observed("audio-device".into()),
            name: observed("Test USB".into()),
            manufacturer: Observation::from_result(Err(-50)),
            alive: observed(true),
            nominal_sample_rate: observed(48_000.0),
            buffer_frame_size: observed(256),
            input_channels: observed(2),
            output_channels: observed(2),
        }
    }
    fn default_audio(&self, _: u32) -> Observation<u32> {
        observed(5)
    }
    fn midi_ids(&self, source: bool) -> Observation<Vec<u32>> {
        if source {
            self.source_reads.set(self.source_reads.get() + 1);
            if self.source_error {
                Observation::from_result(Err(-10830))
            } else {
                observed(vec![42])
            }
        } else {
            observed(Vec::new())
        }
    }
    fn midi(&self, endpoint_ref: u32) -> MidiEndpoint {
        MidiEndpoint {
            endpoint_ref,
            unique_id: observed(-123_456_789),
            name: observed("Transport".into()),
            display_name: observed("Test Keyboard Transport".into()),
            manufacturer: observed("Test".into()),
            model: observed("Keyboard".into()),
            offline: observed(false),
        }
    }
}
fn fake() -> Fake {
    Fake {
        audio_reads: Cell::new(0),
        source_reads: Cell::new(0),
        change_audio: false,
        source_error: false,
    }
}

#[test]
fn inventory_preserves_signed_uid_errors_and_truthful_scope() {
    let value = serde_json::to_value(collect(&fake())).unwrap();
    assert_eq!(value["scope"], "operating_system_coreaudio_coremidi");
    assert_eq!(
        value["midi_sources"]["records"][0]["unique_id"]["value"],
        -123_456_789i32
    );
    assert_eq!(
        value["audio_devices"]["records"][0]["manufacturer"]["os_status"],
        -50
    );
    assert!(
        value["audio_devices"]["records"][0]["manufacturer"]
            .get("value")
            .is_none()
    );
    assert_eq!(value["audio_devices"]["membership_unchanged"], true);
    assert_eq!(value["audio_devices"]["count_before"], 1);
    assert_eq!(value["audio_devices"]["count_after"], 1);
    assert_eq!(value["midi_destinations"]["records"], serde_json::json!([]));
    assert_eq!(value["midi_destinations"]["count_before"], 0);
    assert_eq!(value["midi_destinations"]["count_after"], 0);
    assert_eq!(value["mutation_capability"], false);
    assert!(value.get("daw_selected_device").is_none());
    assert!(value.get("key_events_observed").is_none());
}

#[test]
fn equal_counts_with_changed_ids_never_claim_unchanged_membership() {
    let mut backend = fake();
    backend.change_audio = true;
    let value = serde_json::to_value(collect(&backend)).unwrap();
    assert_eq!(
        value["audio_devices"]["ids_before"]["value"],
        serde_json::json!([5])
    );
    assert_eq!(
        value["audio_devices"]["ids_after"]["value"],
        serde_json::json!([7])
    );
    assert_eq!(value["audio_devices"]["membership_unchanged"], false);
    assert_eq!(value["audio_devices"]["records"][0]["object_id"], 5);
}

#[test]
fn unavailable_membership_is_not_an_empty_complete_inventory() {
    let mut backend = fake();
    backend.source_error = true;
    let value = serde_json::to_value(collect(&backend)).unwrap();
    assert_eq!(value["midi_sources"]["records"], serde_json::json!([]));
    assert_eq!(value["midi_sources"]["ids_before"]["status"], "unavailable");
    assert_eq!(
        value["midi_sources"]["count_before"],
        serde_json::Value::Null
    );
    assert_eq!(
        value["midi_sources"]["count_after"],
        serde_json::Value::Null
    );
    assert_eq!(
        value["midi_sources"]["membership_unchanged"],
        serde_json::Value::Null
    );
}

#[test]
fn duplicate_null_and_over_limit_membership_is_refused() {
    for ids in [vec![0], vec![1, 1], (1..=129).collect()] {
        assert!(bounded_ids(observed(ids), MAX_DEVICES).value().is_none());
    }
    assert_eq!(
        bounded_ids(observed(Vec::new()), MAX_DEVICES).value(),
        Some(&vec![])
    );
}

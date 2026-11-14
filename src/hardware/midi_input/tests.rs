use super::*;
use std::cell::{Cell, RefCell};

fn query() -> ObserveMidiInput {
    ObserveMidiInput {
        source_uid: -123_456_789,
        duration_ms: 100,
        max_events: 8,
    }
}
fn property() -> Property {
    Property {
        os_status: Some(0),
        value: Some("Synthetic keyboard".into()),
        reason: None,
    }
}
fn binding() -> Binding {
    Binding {
        endpoint_ref: 42,
        source_uid: query().source_uid,
        entity_ref: 43,
        device_ref: 44,
        offline: false,
        name: property(),
        display_name: property(),
        manufacturer: property(),
        model: property(),
    }
}
struct Fake {
    calls: RefCell<Vec<&'static str>>,
    bindings: Cell<usize>,
    change_at: usize,
    open_error: bool,
    connect_status: i32,
    disconnect_status: i32,
    dispose_status: i32,
    short_wait: bool,
    shared: Shared,
    generation: Cell<u64>,
}
impl Fake {
    fn new() -> Self {
        Self {
            calls: RefCell::new(Vec::new()),
            bindings: Cell::new(0),
            change_at: 0,
            open_error: false,
            connect_status: 0,
            disconnect_status: 0,
            dispose_status: 0,
            short_wait: false,
            shared: Shared::new(),
            generation: Cell::new(0),
        }
    }
}
impl Backend for Fake {
    fn binding(&self, _uid: i32) -> Result<Binding, OperationError> {
        self.calls.borrow_mut().push("binding");
        let turn = self.bindings.get() + 1;
        self.bindings.set(turn);
        let mut value = binding();
        if turn == self.change_at {
            value.device_ref += 1;
        }
        Ok(value)
    }
    fn open_port(&self, generation: u64) -> Result<u32, OperationError> {
        self.calls.borrow_mut().push("open");
        self.generation.set(generation);
        if self.open_error {
            Err(error("midi_port_unavailable", "synthetic failure"))
        } else {
            Ok(7)
        }
    }
    fn connect(&self, _port: u32, _source: u32) -> i32 {
        self.calls.borrow_mut().push("connect");
        self.connect_status
    }
    fn disconnect(&self, _port: u32, _source: u32) -> i32 {
        self.calls.borrow_mut().push("disconnect");
        self.shared
            .packet(self.generation.get(), 5, &[0x90, 99, 100]);
        self.disconnect_status
    }
    fn dispose(&self, _port: u32) -> i32 {
        self.calls.borrow_mut().push("dispose");
        self.shared
            .packet(self.generation.get(), 6, &[0x90, 98, 100]);
        self.dispose_status
    }
    fn wait(&self, duration: Duration, shared: &Shared, generation: u64) {
        self.calls.borrow_mut().push("wait");
        shared.packet(
            generation,
            9_007_199_254_740_993,
            &[0x90, 60, 110, 0x90, 60, 0],
        );
        if !self.short_wait {
            std::thread::sleep(duration);
        }
    }
}

#[test]
fn request_is_closed_bounded_and_keeps_signed_uids() {
    let base = json!({"source_uid":-123456789,"duration_ms":100,"max_events":8});
    assert_eq!(
        serde_json::from_value::<ObserveMidiInput>(base.clone())
            .unwrap()
            .source_uid,
        -123_456_789
    );
    for (field, item) in [
        ("source_uid", json!(0)),
        ("duration_ms", json!(99)),
        ("duration_ms", json!(5001)),
        ("max_events", json!(0)),
        ("max_events", json!(257)),
        ("source_uid", json!(true)),
        ("duration_ms", json!(1.5)),
        ("max_events", json!(null)),
        ("path", json!("unchecked")),
    ] {
        let mut value = base.clone();
        value[field] = item;
        let parsed = serde_json::from_value::<ObserveMidiInput>(value);
        assert!(parsed.is_err() || parsed.is_ok_and(|query| validate(&query).is_err()));
    }
    for uid in [i32::MIN, -1, 1, i32::MAX] {
        let mut value = query();
        value.source_uid = uid;
        validate(&value).unwrap();
    }
}

#[test]
fn source_selection_refuses_ambiguity_missing_and_membership_corruption() {
    assert_eq!(unique_source(&[(7, -1), (8, 12)], -1).unwrap(), 7);
    assert!(unique_source(&[(7, -1), (8, -1)], -1).is_err());
    assert!(unique_source(&[(7, 12)], -1).is_err());
    assert!(unique_source(&[(0, -1)], -1).is_err());
    assert!(unique_source(&[(7, -1), (7, 12)], -1).is_err());
    assert!(unique_source(&vec![(7, -1); 4097], -1).is_err());
}

#[test]
fn complete_channel_messages_decode_without_exporting_system_payloads() {
    let mut capture = Capture {
        limit: 256,
        ..Capture::empty()
    };
    capture.packet(
        u64::MAX,
        &[
            0x80, 60, 2, 0x90, 61, 100, 0x90, 61, 0, 0xa1, 62, 3, 0xb2, 64, 127, 0xc3, 7, 0xd4, 9,
            0xe5, 0, 64, 0xf8, 0xfe,
        ],
    );
    let values: Vec<_> = capture.events[..capture.length]
        .iter()
        .copied()
        .map(Event::value)
        .collect();
    assert_eq!(values.len(), 8);
    assert_eq!(values[0]["timestamp_native_ticks"], u64::MAX.to_string());
    assert_eq!(values[2]["message"]["kind"], "note_off");
    assert_eq!(values[3]["channel"], 2);
    assert_eq!(values[7]["message"]["value_14bit"], 8192);
    assert_eq!(capture.counts.system_messages_skipped, 2);
    capture.packet(0, &[0xf0, 1, 2, 3, 0xf7]);
    capture.packet(0, &[1, 2, 3]);
    assert_eq!(capture.length, 8);
    assert_eq!(capture.counts.unsupported_packets, 2);
}

#[test]
fn realtime_at_every_data_position_preserves_both_channel_message_widths() {
    for status in 0x80u8..0xf0 {
        let message = if matches!(status >> 4, 12 | 13) {
            vec![status, 45]
        } else {
            vec![status, 45, 100]
        };
        for realtime in 0xf8u8..=0xff {
            for position in 0..=message.len() {
                let mut packet = message.clone();
                packet.insert(position, realtime);
                let mut capture = Capture {
                    limit: 1,
                    ..Capture::empty()
                };
                capture.packet(123, &packet);
                assert_eq!(capture.counts.malformed_packets, 0, "{packet:02x?}");
                assert_eq!(capture.counts.unsupported_packets, 0, "{packet:02x?}");
                assert_eq!(capture.counts.system_messages_skipped, 1, "{packet:02x?}");
                assert_eq!(capture.counts.channel_messages_seen, 1, "{packet:02x?}");
                assert_eq!(capture.length, 1, "{packet:02x?}");
                let event = capture.events[0];
                assert_eq!(event.timestamp, 123);
                assert_eq!(event.status, status);
                assert_eq!(event.first, 45);
                assert_eq!(event.second, if message.len() == 3 { 100 } else { 0 });
            }
        }
    }
}

#[test]
fn realtime_never_creates_running_status_or_exports_system_exclusive_payloads() {
    for suffix in [&[61, 110][..], &[0xf0, 1, 2, 3, 0xf7][..]] {
        let mut packet = vec![0xf8, 0x90, 0xf9, 60, 0xfa, 0xfb, 100, 0xfc];
        packet.extend_from_slice(suffix);
        let mut capture = Capture {
            limit: 8,
            ..Capture::empty()
        };
        capture.packet(5, &packet);
        assert_eq!(capture.length, 1);
        assert_eq!(
            capture.events[0].value()["message"],
            json!({"kind":"note_on","note":60,"velocity":100})
        );
        assert_eq!(capture.counts.system_messages_skipped, 5);
        assert_eq!(capture.counts.malformed_packets, 0);
        assert_eq!(capture.counts.unsupported_packets, 1);
    }
    let mut capture = Capture {
        limit: 8,
        ..Capture::empty()
    };
    capture.packet(0, &[0x90, 60, 0xf8, 0xff]);
    assert_eq!(capture.length, 0);
    assert_eq!(capture.counts.system_messages_skipped, 2);
    assert_eq!(capture.counts.malformed_packets, 1);
}

#[test]
fn malformed_packets_and_fixed_storage_limit_cannot_create_extra_events() {
    let mut capture = Capture {
        limit: 2,
        ..Capture::empty()
    };
    for bytes in [&[0x90, 60][..], &[0x90, 60, 0xff][..], &[0xc0][..]] {
        capture.packet(0, bytes);
    }
    assert_eq!(capture.length, 0);
    assert_eq!(capture.counts.malformed_packets, 3);
    for _ in 0..20 {
        capture.packet(0, &[0x90, 60, 10]);
    }
    assert_eq!(capture.length, 2);
    assert_eq!(capture.counts.events_dropped_at_limit, 18);
    capture.packet(0, &vec![0x90; MAX_PACKET_BYTES + 1]);
    assert_eq!(capture.counts.packet_budget_drops, 1);
}

#[test]
fn old_generation_callback_cannot_write_after_cleanup_or_during_a_new_window() {
    let shared = Shared::new();
    shared.begin(1, Duration::from_secs(1), 8).unwrap();
    shared.packet(1, 1, &[0x90, 60, 1]);
    assert_eq!(shared.end().unwrap().length, 1);
    shared.packet(1, 2, &[0x90, 61, 1]);
    shared.begin(2, Duration::from_secs(1), 8).unwrap();
    shared.packet(1, 3, &[0x90, 62, 1]);
    shared.packet(2, 4, &[0x90, 63, 1]);
    let capture = shared.end().unwrap();
    assert_eq!(capture.length, 1);
    assert_eq!(capture.events[0].first, 63);
}

#[test]
fn callback_contention_is_nonblocking_and_expired_window_ignores_data() {
    let shared = Shared::new();
    shared.begin(1, Duration::from_secs(1), 8).unwrap();
    let locked = shared.capture.lock().unwrap();
    shared.packet(1, 1, &[0x90, 60, 100]);
    assert_eq!(shared.contention_total.load(Ordering::Acquire), 1);
    drop(locked);
    shared.capture.lock().unwrap().deadline = Some(Instant::now() - Duration::from_millis(1));
    shared.packet(1, 2, &[0x90, 60, 100]);
    assert_eq!(shared.end().unwrap().length, 0);
}

#[test]
fn completed_observation_releases_port_and_does_not_claim_physical_keys() {
    let fake = Fake::new();
    let value = run(&query(), &fake, &fake.shared).unwrap();
    assert_eq!(value["events"].as_array().unwrap().len(), 2);
    assert_eq!(
        value["events"][0]["timestamp_native_ticks"],
        "9007199254740993"
    );
    assert_eq!(value["physical_key_activity_verified"], false);
    assert_eq!(value["midi_messages_sent"], 0);
    assert_eq!(value["midi_messages_forwarded"], 0);
    assert_eq!(value["port_dispose_status"], 0);
    assert_eq!(
        fake.calls.into_inner(),
        [
            "binding",
            "open",
            "connect",
            "binding",
            "wait",
            "disconnect",
            "dispose",
            "binding"
        ]
    );
}

#[test]
fn connect_open_and_identity_failures_never_skip_owned_port_cleanup() {
    let mut fake = Fake::new();
    fake.connect_status = -50;
    assert!(run(&query(), &fake, &fake.shared).is_err());
    assert_eq!(
        fake.calls.borrow().as_slice(),
        ["binding", "open", "connect", "dispose"]
    );
    let mut fake = Fake::new();
    fake.open_error = true;
    assert!(run(&query(), &fake, &fake.shared).is_err());
    assert_eq!(fake.calls.borrow().as_slice(), ["binding", "open"]);
    for turn in [2, 3] {
        let mut fake = Fake::new();
        fake.change_at = turn;
        assert_eq!(
            run(&query(), &fake, &fake.shared).unwrap_err().code,
            "midi_source_changed"
        );
        assert!(fake.calls.borrow().ends_with(if turn == 2 {
            &["disconnect", "dispose"][..]
        } else {
            &["dispose", "binding"][..]
        }));
    }
}

#[test]
fn failed_cleanup_quarantines_the_adapter_and_prevents_followon_native_calls() {
    for dispose in [false, true] {
        let mut fake = Fake::new();
        if dispose {
            fake.dispose_status = -50;
        } else {
            fake.disconnect_status = -50;
        }
        assert_eq!(
            run(&query(), &fake, &fake.shared).unwrap_err().code,
            "midi_cleanup_unconfirmed"
        );
        let calls = fake.calls.borrow().len();
        assert_eq!(
            run(&query(), &fake, &fake.shared).unwrap_err().code,
            "midi_observer_poisoned"
        );
        assert_eq!(fake.calls.borrow().len(), calls);
        assert_eq!(fake.shared.active.load(Ordering::Acquire), 0);
    }
}

#[test]
fn overlap_generation_exhaustion_and_invalid_query_refuse_before_subscription() {
    let fake = Fake::new();
    fake.shared.occupied.store(true, Ordering::Release);
    assert_eq!(
        run(&query(), &fake, &fake.shared).unwrap_err().code,
        "midi_observer_busy"
    );
    fake.shared.occupied.store(false, Ordering::Release);
    fake.shared.generation.store(u64::MAX, Ordering::Release);
    assert_eq!(
        run(&query(), &fake, &fake.shared).unwrap_err().code,
        "midi_observer_exhausted"
    );
    let mut bad = query();
    bad.source_uid = 0;
    assert!(run(&bad, &fake, &fake.shared).is_err());
    assert!(fake.calls.borrow().is_empty());
}

#[test]
fn incomplete_window_returns_no_success_after_closing_input() {
    let mut fake = Fake::new();
    fake.short_wait = true;
    assert_eq!(
        run(&query(), &fake, &fake.shared).unwrap_err().code,
        "midi_window_incomplete"
    );
    assert!(fake.calls.borrow().ends_with(&["disconnect", "dispose"]));
    assert_eq!(fake.shared.active.load(Ordering::Acquire), 0);
}

#[test]
fn capture_poison_refuses_before_any_native_resource_is_opened() {
    let fake = Fake::new();
    let _ = std::panic::catch_unwind(|| {
        let _locked = fake.shared.capture.lock().unwrap();
        panic!("synthetic capture failure");
    });
    assert_eq!(
        run(&query(), &fake, &fake.shared).unwrap_err().code,
        "midi_observer_poisoned"
    );
    assert!(fake.calls.borrow().is_empty());
    assert!(!fake.shared.occupied.load(Ordering::Acquire));
}

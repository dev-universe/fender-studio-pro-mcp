use crate::model::OperationError;
use schemars::JsonSchema;
use serde::Deserialize;
#[cfg(any(target_os = "macos", test))]
use serde::Serialize;
use serde_json::Value;
#[cfg(any(target_os = "macos", test))]
use serde_json::json;
#[cfg(any(target_os = "macos", test))]
use std::sync::{
    Mutex, TryLockError,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
#[cfg(any(target_os = "macos", test))]
use std::time::{Duration, Instant};

#[cfg(any(target_os = "macos", test))]
const MAX_EVENTS: usize = 256;
#[cfg(any(target_os = "macos", test))]
const MAX_PACKET_BYTES: usize = 65_536;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ObserveMidiInput {
    pub source_uid: i32,
    #[schemars(range(min = 100, max = 5000))]
    pub duration_ms: u32,
    #[schemars(range(min = 1, max = 256))]
    pub max_events: u32,
}

pub fn validate(query: &ObserveMidiInput) -> Result<(), OperationError> {
    if query.source_uid == 0
        || !(100..=5000).contains(&query.duration_ms)
        || !(1..=256).contains(&query.max_events)
    {
        Err(OperationError::new(
            "invalid_midi_observation",
            "Use a nonzero signed source_uid, duration_ms100..5000 and max_events1..256",
        ))
    } else {
        Ok(())
    }
}

pub fn observe(query: &ObserveMidiInput) -> Result<Value, OperationError> {
    validate(query)?;
    #[cfg(target_os = "macos")]
    {
        run(query, &native::Native, &SHARED)
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err(error(
            "unsupported_platform",
            "MIDI input observation requires macOS",
        ))
    }
}

fn error(code: &str, message: impl Into<String>) -> OperationError {
    OperationError::new(code, message)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg(any(target_os = "macos", test))]
struct Property {
    os_status: Option<i32>,
    value: Option<String>,
    reason: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg(any(target_os = "macos", test))]
struct Binding {
    endpoint_ref: u32,
    source_uid: i32,
    entity_ref: u32,
    device_ref: u32,
    offline: bool,
    name: Property,
    display_name: Property,
    manufacturer: Property,
    model: Property,
}

#[derive(Clone, Copy)]
#[cfg(any(target_os = "macos", test))]
struct Event {
    timestamp: u64,
    status: u8,
    first: u8,
    second: u8,
}

#[cfg(any(target_os = "macos", test))]
impl Event {
    const EMPTY: Self = Self {
        timestamp: 0,
        status: 0,
        first: 0,
        second: 0,
    };

    fn value(self) -> Value {
        let message = match self.status >> 4 {
            8 => json!({"kind":"note_off","note":self.first,"velocity":self.second}),
            9 if self.second == 0 => {
                json!({"kind":"note_off","note":self.first,"velocity":0,"encoding":"note_on_zero_velocity"})
            }
            9 => json!({"kind":"note_on","note":self.first,"velocity":self.second}),
            10 => json!({"kind":"poly_pressure","note":self.first,"pressure":self.second}),
            11 => json!({"kind":"control_change","controller":self.first,"value":self.second}),
            12 => json!({"kind":"program_change","program":self.first}),
            13 => json!({"kind":"channel_pressure","pressure":self.first}),
            14 => {
                json!({"kind":"pitch_bend","value_14bit":u16::from(self.first)|(u16::from(self.second)<<7)})
            }
            _ => unreachable!(),
        };
        json!({"timestamp_native_ticks":self.timestamp.to_string(),"channel":(self.status&15)+1,"message":message})
    }
}

#[derive(Clone, Copy, Default, Serialize)]
#[cfg(any(target_os = "macos", test))]
struct Counts {
    packets_seen: u64,
    channel_messages_seen: u64,
    events_dropped_at_limit: u64,
    unsupported_packets: u64,
    system_messages_skipped: u64,
    malformed_packets: u64,
    packet_budget_drops: u64,
}

#[derive(Clone)]
#[cfg(any(target_os = "macos", test))]
struct Capture {
    generation: u64,
    deadline: Option<Instant>,
    limit: usize,
    length: usize,
    events: [Event; MAX_EVENTS],
    counts: Counts,
}

#[cfg(any(target_os = "macos", test))]
impl Capture {
    const fn empty() -> Self {
        Self {
            generation: 0,
            deadline: None,
            limit: 0,
            length: 0,
            events: [Event::EMPTY; MAX_EVENTS],
            counts: Counts {
                packets_seen: 0,
                channel_messages_seen: 0,
                events_dropped_at_limit: 0,
                unsupported_packets: 0,
                system_messages_skipped: 0,
                malformed_packets: 0,
                packet_budget_drops: 0,
            },
        }
    }

    fn packet(&mut self, timestamp: u64, bytes: &[u8]) {
        self.counts.packets_seen = self.counts.packets_seen.saturating_add(1);
        if bytes.len() > MAX_PACKET_BYTES {
            self.counts.packet_budget_drops = self.counts.packet_budget_drops.saturating_add(1);
            return;
        }
        if bytes
            .first()
            .is_some_and(|b| *b < 0x80 || (0xf0..=0xf7).contains(b))
        {
            self.counts.unsupported_packets = self.counts.unsupported_packets.saturating_add(1);
            return;
        }
        let mut at = 0;
        while at < bytes.len() {
            let status = bytes[at];
            if status >= 0xf8 {
                self.counts.system_messages_skipped =
                    self.counts.system_messages_skipped.saturating_add(1);
                at += 1;
                continue;
            }
            if !(0x80..0xf0).contains(&status) {
                self.counts.unsupported_packets = self.counts.unsupported_packets.saturating_add(1);
                break;
            }
            let size = if matches!(status >> 4, 12 | 13) { 1 } else { 2 };
            let mut data = [0u8; 2];
            let mut length = 0;
            at += 1;
            while length < size {
                let Some(&byte) = bytes.get(at) else {
                    self.counts.malformed_packets = self.counts.malformed_packets.saturating_add(1);
                    return;
                };
                if byte >= 0xf8 {
                    self.counts.system_messages_skipped =
                        self.counts.system_messages_skipped.saturating_add(1);
                } else if byte >= 0x80 {
                    self.counts.malformed_packets = self.counts.malformed_packets.saturating_add(1);
                    return;
                } else {
                    data[length] = byte;
                    length += 1;
                }
                at += 1;
            }
            self.counts.channel_messages_seen = self.counts.channel_messages_seen.saturating_add(1);
            if self.length < self.limit {
                self.events[self.length] = Event {
                    timestamp,
                    status,
                    first: data[0],
                    second: data[1],
                };
                self.length += 1;
            } else {
                self.counts.events_dropped_at_limit =
                    self.counts.events_dropped_at_limit.saturating_add(1);
            }
        }
    }
}

#[cfg(any(target_os = "macos", test))]
struct Shared {
    occupied: AtomicBool,
    poisoned: AtomicBool,
    generation: AtomicU64,
    active: AtomicU64,
    contention_total: AtomicU64,
    capture: Mutex<Capture>,
}

#[cfg(any(target_os = "macos", test))]
impl Shared {
    const fn new() -> Self {
        Self {
            occupied: AtomicBool::new(false),
            poisoned: AtomicBool::new(false),
            generation: AtomicU64::new(0),
            active: AtomicU64::new(0),
            contention_total: AtomicU64::new(0),
            capture: Mutex::new(Capture::empty()),
        }
    }

    fn packet(&self, generation: u64, timestamp: u64, bytes: &[u8]) {
        self.update(generation, |capture| capture.packet(timestamp, bytes));
    }

    fn update(&self, generation: u64, update: impl FnOnce(&mut Capture)) {
        if generation == 0 || self.active.load(Ordering::Acquire) != generation {
            return;
        }
        let now = Instant::now();
        match self.capture.try_lock() {
            Ok(mut capture) => {
                if self.active.load(Ordering::Acquire) == generation
                    && capture.generation == generation
                    && capture.deadline.is_some_and(|deadline| now < deadline)
                {
                    update(&mut capture);
                }
            }
            Err(TryLockError::WouldBlock) => {
                self.contention_total.fetch_add(1, Ordering::Relaxed);
            }
            Err(TryLockError::Poisoned(_)) => {
                self.poisoned.store(true, Ordering::Release);
            }
        }
    }

    fn begin(
        &self,
        generation: u64,
        duration: Duration,
        limit: usize,
    ) -> Result<(), OperationError> {
        let mut capture = self
            .capture
            .lock()
            .map_err(|_| error("midi_observer_poisoned", "Capture storage is unavailable"))?;
        *capture = Capture {
            generation,
            deadline: Some(Instant::now() + duration),
            limit,
            ..Capture::empty()
        };
        drop(capture);
        self.active.store(generation, Ordering::Release);
        Ok(())
    }

    fn end(&self) -> Result<Capture, OperationError> {
        self.active.store(0, Ordering::Release);
        self.capture
            .lock()
            .map(|value| value.clone())
            .map_err(|_| error("midi_observer_poisoned", "Capture storage is unavailable"))
    }
}

#[cfg(target_os = "macos")]
static SHARED: Shared = Shared::new();

#[cfg(any(target_os = "macos", test))]
struct Occupied<'a>(&'a Shared);
#[cfg(any(target_os = "macos", test))]
impl Drop for Occupied<'_> {
    fn drop(&mut self) {
        self.0.occupied.store(false, Ordering::Release);
    }
}

#[cfg(any(target_os = "macos", test))]
trait Backend {
    fn binding(&self, uid: i32) -> Result<Binding, OperationError>;
    fn open_port(&self, generation: u64) -> Result<u32, OperationError>;
    fn connect(&self, port: u32, source: u32) -> i32;
    fn disconnect(&self, port: u32, source: u32) -> i32;
    fn dispose(&self, port: u32) -> i32;
    fn wait(&self, duration: Duration, shared: &Shared, generation: u64);
}

#[cfg(any(target_os = "macos", test))]
fn unique_source(entries: &[(u32, i32)], uid: i32) -> Result<u32, OperationError> {
    if entries.len() > 4096
        || entries.iter().enumerate().any(|(at, (reference, _))| {
            *reference == 0 || entries[..at].iter().any(|(other, _)| other == reference)
        })
    {
        return Err(error(
            "midi_sources_changed",
            "The source inventory is missing or ambiguous",
        ));
    }
    let mut found = entries.iter().filter(|(_, value)| *value == uid);
    let endpoint = found
        .next()
        .map(|(reference, _)| *reference)
        .ok_or_else(|| {
            error(
                "midi_source_missing",
                "The selected signed MIDI source UID was not found",
            )
        })?;
    if found.next().is_some() {
        return Err(error(
            "midi_source_ambiguous",
            "The selected signed MIDI source UID is not unique",
        ));
    }
    Ok(endpoint)
}

#[cfg(any(target_os = "macos", test))]
fn run(
    query: &ObserveMidiInput,
    backend: &impl Backend,
    shared: &Shared,
) -> Result<Value, OperationError> {
    validate(query)?;
    if shared
        .occupied
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        return Err(error(
            "midi_observer_busy",
            "One MIDI observation is already active",
        ));
    }
    let _occupied = Occupied(shared);
    if shared.poisoned.load(Ordering::Acquire) || shared.capture.is_poisoned() {
        shared.poisoned.store(true, Ordering::Release);
        return Err(error(
            "midi_observer_poisoned",
            "A previous cleanup or capture failed; restart the owner",
        ));
    }
    let generation = shared
        .generation
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
            value.checked_add(1)
        })
        .map_err(|_| {
            error(
                "midi_observer_exhausted",
                "Observation generation is exhausted; restart the owner",
            )
        })?
        + 1;
    let before = backend.binding(query.source_uid)?;
    let port = backend.open_port(generation)?;
    let connected = backend.connect(port, before.endpoint_ref);
    let capture = (|| {
        if connected != 0 {
            return Err(error(
                "midi_connect_failed",
                format!("MIDIPortConnectSource status {connected}"),
            ));
        }
        if backend.binding(query.source_uid)? != before {
            return Err(error(
                "midi_source_changed",
                "The source changed during input subscription",
            ));
        }
        let duration = Duration::from_millis(u64::from(query.duration_ms));
        let contention_start = shared.contention_total.load(Ordering::Acquire);
        shared.begin(generation, duration, query.max_events as usize)?;
        let started = Instant::now();
        backend.wait(duration, shared, generation);
        let elapsed = started.elapsed();
        let capture = shared.end()?;
        if elapsed < duration {
            return Err(error(
                "midi_window_incomplete",
                "The input window did not reach its requested duration",
            ));
        }
        Ok((capture, elapsed.as_millis(), contention_start))
    })();
    shared.active.store(0, Ordering::Release);
    let disconnect = (connected == 0).then(|| backend.disconnect(port, before.endpoint_ref));
    let disposed = backend.dispose(port);
    if disconnect.is_some_and(|status| status != 0) || disposed != 0 {
        shared.poisoned.store(true, Ordering::Release);
        return Err(error(
            "midi_cleanup_unconfirmed",
            format!(
                "Input cleanup failed: disconnect={disconnect:?}, dispose={disposed}; restart the owner"
            ),
        ));
    }
    let (capture, elapsed_ms, contention_start) = capture?;
    let after = backend.binding(query.source_uid)?;
    if after != before {
        return Err(error(
            "midi_source_changed",
            "The selected source changed before publication",
        ));
    }
    if shared.poisoned.load(Ordering::Acquire) {
        return Err(error(
            "midi_observer_poisoned",
            "Callback storage failed during observation",
        ));
    }
    let events: Vec<_> = capture.events[..capture.length]
        .iter()
        .copied()
        .map(Event::value)
        .collect();
    let note_activity = capture.events[..capture.length]
        .iter()
        .any(|event| matches!(event.status >> 4, 8 | 9));
    Ok(
        json!({"ok":true,"scope":"operating_system_selected_midi_input","observation":"bounded_midi_1_0_projection",
        "source_before":before,"source_after":after,"requested_duration_ms":query.duration_ms,
        "elapsed_window_ms":elapsed_ms,"max_events":query.max_events,"events":events,"counts":capture.counts,
        "subscription_started":true,"window_completed":true,"callback_capture_closed":true,
        "disconnect_status":disconnect,"port_dispose_status":disposed,"observer_client_lifetime":"process",
        "callback_contention_total":shared.contention_total.load(Ordering::Acquire),
        "callback_contention_during_window":shared.contention_total.load(Ordering::Acquire).saturating_sub(contention_start),
        "note_activity_observed":note_activity,"physical_key_activity_observed":false,"physical_key_activity_verified":false,
        "daw_routing_verified":false,"audio_verified":false,"midi_messages_sent":0,"midi_messages_forwarded":0,
        "limitations":["Only complete explicit-status MIDI1 channel messages are decoded; system, SysEx and unsupported packets are counted without payloads.",
            "The operating-system endpoint identity does not prove a physical keystroke or DAW routing.",
            "The process retains one CoreMIDI observer client until exit; each input port is disconnected and disposed."]}),
    )
}

#[cfg(target_os = "macos")]
mod native {
    use super::*;
    use std::ffi::{CStr, c_char, c_void};
    use std::ptr;
    use std::sync::OnceLock;

    #[repr(C, packed(4))]
    struct Packet {
        timestamp: u64,
        length: u16,
        data: [u8; 256],
    }
    #[repr(C, packed(4))]
    struct PacketList {
        count: u32,
        packet: [Packet; 1],
    }
    type Read = unsafe extern "C" fn(*const PacketList, *mut c_void, *mut c_void);
    type Notify = unsafe extern "C" fn(*const c_void, *mut c_void);

    #[link(name = "CoreMIDI", kind = "framework")]
    unsafe extern "C" {
        fn MIDIGetNumberOfSources() -> usize;
        fn MIDIGetSource(index: usize) -> u32;
        fn MIDIObjectGetIntegerProperty(object: u32, key: *const c_void, value: *mut i32) -> i32;
        fn MIDIObjectGetStringProperty(
            object: u32,
            key: *const c_void,
            value: *mut *const c_void,
        ) -> i32;
        fn MIDIEndpointGetEntity(endpoint: u32, entity: *mut u32) -> i32;
        fn MIDIEntityGetDevice(entity: u32, device: *mut u32) -> i32;
        fn MIDIClientCreate(
            name: *const c_void,
            notify: Option<Notify>,
            context: *mut c_void,
            client: *mut u32,
        ) -> i32;
        fn MIDIInputPortCreate(
            client: u32,
            name: *const c_void,
            read: Read,
            context: *mut c_void,
            port: *mut u32,
        ) -> i32;
        fn MIDIPortConnectSource(port: u32, source: u32, context: *mut c_void) -> i32;
        fn MIDIPortDisconnectSource(port: u32, source: u32) -> i32;
        fn MIDIPortDispose(port: u32) -> i32;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFStringCreateWithCString(
            allocator: *const c_void,
            bytes: *const c_char,
            encoding: u32,
        ) -> *const c_void;
        fn CFStringGetTypeID() -> usize;
        fn CFGetTypeID(value: *const c_void) -> usize;
        fn CFStringGetLength(value: *const c_void) -> isize;
        fn CFStringGetCString(
            value: *const c_void,
            bytes: *mut c_char,
            capacity: isize,
            encoding: u32,
        ) -> u8;
        fn CFRelease(value: *const c_void);
    }
    const UTF8: u32 = 0x0800_0100;
    static CLIENT: OnceLock<Result<u32, i32>> = OnceLock::new();

    struct Cf(*const c_void);
    impl Drop for Cf {
        fn drop(&mut self) {
            unsafe {
                CFRelease(self.0);
            }
        }
    }
    fn key(name: &'static CStr) -> Result<Cf, OperationError> {
        let value = unsafe { CFStringCreateWithCString(ptr::null(), name.as_ptr(), UTF8) };
        if value.is_null() {
            Err(error(
                "midi_property_unavailable",
                "A fixed CoreMIDI property key could not be created",
            ))
        } else {
            Ok(Cf(value))
        }
    }
    fn integer(object: u32, name: &'static CStr) -> Result<i32, OperationError> {
        let key = key(name)?;
        let mut value = 0;
        let status = unsafe { MIDIObjectGetIntegerProperty(object, key.0, &mut value) };
        if status == 0 {
            Ok(value)
        } else {
            Err(error(
                "midi_property_unavailable",
                format!("CoreMIDI integer property status {status}"),
            ))
        }
    }
    fn string(object: u32, name: &'static CStr) -> Property {
        let Ok(key) = key(name) else {
            return Property {
                os_status: None,
                value: None,
                reason: Some("key_unavailable"),
            };
        };
        let mut value = ptr::null();
        let status = unsafe { MIDIObjectGetStringProperty(object, key.0, &mut value) };
        if status != 0 {
            return Property {
                os_status: Some(status),
                value: None,
                reason: Some("property_read_failed"),
            };
        }
        if value.is_null() {
            return Property {
                os_status: Some(0),
                value: None,
                reason: Some("null_property"),
            };
        }
        string_value(Cf(value))
    }
    fn string_value(value: Cf) -> Property {
        if unsafe { CFGetTypeID(value.0) != CFStringGetTypeID() } {
            return Property {
                os_status: Some(0),
                value: None,
                reason: Some("wrong_property_type"),
            };
        }
        let length = unsafe { CFStringGetLength(value.0) };
        if !(0..=512).contains(&length) {
            return Property {
                os_status: Some(0),
                value: None,
                reason: Some("excessive_property_length"),
            };
        }
        let mut bytes = [0u8; 2049];
        let copied = unsafe {
            CFStringGetCString(
                value.0,
                bytes.as_mut_ptr().cast(),
                bytes.len() as isize,
                UTF8,
            )
        };
        if copied == 0 {
            return Property {
                os_status: Some(0),
                value: None,
                reason: Some("property_conversion_failed"),
            };
        }
        let Some(end) = bytes.iter().position(|byte| *byte == 0) else {
            return Property {
                os_status: Some(0),
                value: None,
                reason: Some("property_terminator_missing"),
            };
        };
        match std::str::from_utf8(&bytes[..end]) {
            Ok(text) if text.encode_utf16().count() == length as usize => Property {
                os_status: Some(0),
                value: Some(text.to_owned()),
                reason: None,
            },
            Ok(_) => Property {
                os_status: Some(0),
                value: None,
                reason: Some("incomplete_property_conversion"),
            },
            Err(_) => Property {
                os_status: Some(0),
                value: None,
                reason: Some("invalid_property_utf8"),
            },
        }
    }

    fn ids() -> Result<Vec<u32>, OperationError> {
        let count = unsafe { MIDIGetNumberOfSources() };
        if count > 4096 {
            return Err(error(
                "midi_sources_excessive",
                "More than4096 MIDI sources were reported",
            ));
        }
        let mut ids = Vec::with_capacity(count);
        for index in 0..count {
            let endpoint = unsafe { MIDIGetSource(index) };
            if endpoint == 0 || ids.contains(&endpoint) {
                return Err(error(
                    "midi_sources_changed",
                    "The source inventory is missing or ambiguous",
                ));
            }
            ids.push(endpoint);
        }
        Ok(ids)
    }

    pub(super) struct Native;
    impl Backend for Native {
        fn binding(&self, uid: i32) -> Result<Binding, OperationError> {
            let before = ids()?;
            let entries = before
                .iter()
                .map(|endpoint| Ok((*endpoint, integer(*endpoint, c"uniqueID")?)))
                .collect::<Result<Vec<_>, OperationError>>()?;
            let endpoint = unique_source(&entries, uid)?;
            if integer(endpoint, c"offline")? != 0 {
                return Err(error(
                    "midi_source_offline",
                    "The selected source is not online",
                ));
            }
            let mut entity = 0;
            let entity_status = unsafe { MIDIEndpointGetEntity(endpoint, &mut entity) };
            let mut device = 0;
            let device_status = if entity_status == 0 && entity != 0 {
                unsafe { MIDIEntityGetDevice(entity, &mut device) }
            } else {
                -1
            };
            if entity_status != 0 || device_status != 0 || entity == 0 || device == 0 {
                return Err(error(
                    "midi_source_not_device_owned",
                    "The selected source has no verified owning device and entity",
                ));
            }
            let binding = Binding {
                endpoint_ref: endpoint,
                source_uid: uid,
                entity_ref: entity,
                device_ref: device,
                offline: false,
                name: string(endpoint, c"name"),
                display_name: string(endpoint, c"displayName"),
                manufacturer: string(endpoint, c"manufacturer"),
                model: string(endpoint, c"model"),
            };
            let after = ids()?;
            let after_entries = after
                .iter()
                .map(|endpoint| Ok((*endpoint, integer(*endpoint, c"uniqueID")?)))
                .collect::<Result<Vec<_>, OperationError>>()?;
            if before != after || entries != after_entries || integer(endpoint, c"offline")? != 0 {
                return Err(error(
                    "midi_sources_changed",
                    "The source inventory changed during binding",
                ));
            }
            Ok(binding)
        }

        fn open_port(&self, generation: u64) -> Result<u32, OperationError> {
            // Apple discourages disposing the process's last MIDI client. This
            // fixed observer client is intentionally reused until process exit.
            let client = CLIENT
                .get_or_init(|| {
                    let name = key(c"fender-mcp-passive-observer").map_err(|_| -1)?;
                    let mut client = 0;
                    let status =
                        unsafe { MIDIClientCreate(name.0, None, ptr::null_mut(), &mut client) };
                    if status == 0 && client != 0 {
                        Ok(client)
                    } else {
                        Err(status)
                    }
                })
                .map_err(|status| {
                    error(
                        "midi_client_unavailable",
                        format!("Observer MIDIClientCreate status {status}"),
                    )
                })?;
            let name = key(c"fender-mcp-selected-input")?;
            let mut port = 0;
            let status = unsafe {
                MIDIInputPortCreate(
                    client,
                    name.0,
                    receive,
                    generation as usize as *mut c_void,
                    &mut port,
                )
            };
            if status == 0 && port != 0 {
                Ok(port)
            } else {
                Err(error(
                    "midi_port_unavailable",
                    format!("MIDIInputPortCreate status {status}"),
                ))
            }
        }
        fn connect(&self, port: u32, source: u32) -> i32 {
            unsafe { MIDIPortConnectSource(port, source, ptr::null_mut()) }
        }
        fn disconnect(&self, port: u32, source: u32) -> i32 {
            unsafe { MIDIPortDisconnectSource(port, source) }
        }
        fn dispose(&self, port: u32) -> i32 {
            unsafe { MIDIPortDispose(port) }
        }
        fn wait(&self, duration: Duration, _shared: &Shared, _generation: u64) {
            std::thread::sleep(duration);
        }
    }

    // Refcon is only an opaque generation number. Callback storage has process
    // lifetime, including after a failed disposal or a delayed native callback.
    unsafe extern "C" fn receive(
        list: *const PacketList,
        context: *mut c_void,
        _source: *mut c_void,
    ) {
        let generation = context as usize as u64;
        if generation == 0 || list.is_null() || SHARED.active.load(Ordering::Acquire) != generation
        {
            return;
        }
        let count = unsafe { ptr::addr_of!((*list).count).read_unaligned() } as usize;
        if count > 1024 {
            SHARED.update(generation, |capture| {
                capture.counts.packet_budget_drops =
                    capture.counts.packet_budget_drops.saturating_add(1)
            });
            return;
        }
        let mut packet = unsafe { ptr::addr_of!((*list).packet).cast::<Packet>() };
        let mut bytes_read = 0usize;
        for _ in 0..count {
            if SHARED.active.load(Ordering::Acquire) != generation {
                return;
            }
            let length = unsafe { ptr::addr_of!((*packet).length).read_unaligned() } as usize;
            bytes_read += length;
            if bytes_read > MAX_PACKET_BYTES {
                SHARED.update(generation, |capture| {
                    capture.counts.packet_budget_drops =
                        capture.counts.packet_budget_drops.saturating_add(1)
                });
                return;
            }
            let timestamp = unsafe { ptr::addr_of!((*packet).timestamp).read_unaligned() };
            let data = unsafe { ptr::addr_of!((*packet).data).cast::<u8>() };
            let bytes = unsafe { std::slice::from_raw_parts(data, length) };
            SHARED.packet(generation, timestamp, bytes);
            let next = data as usize + length;
            let next = if cfg!(target_arch = "aarch64") {
                (next + 3) & !3
            } else {
                next
            };
            packet = next as *const Packet;
        }
    }

    #[cfg(test)]
    mod callback_tests {
        use super::*;
        static SERIAL: Mutex<()> = Mutex::new(());

        unsafe extern "C" {
            fn CFStringCreateWithCharacters(
                allocator: *const c_void,
                characters: *const u16,
                count: isize,
            ) -> *const c_void;
        }

        fn converted(text: &str) -> Property {
            let units: Vec<_> = text.encode_utf16().collect();
            let value = unsafe {
                CFStringCreateWithCharacters(ptr::null(), units.as_ptr(), units.len() as isize)
            };
            assert!(!value.is_null());
            string_value(Cf(value))
        }

        #[test]
        fn bounded_cf_metadata_never_equates_an_embedded_nul_suffix_with_its_prefix() {
            let prefix = converted("same-prefix");
            assert_eq!(prefix.value.as_deref(), Some("same-prefix"));
            for text in [
                "\0same-prefix",
                "same-prefix\0",
                "same-prefix\0different-suffix",
                "same-prefix\0another-suffix",
            ] {
                let value = converted(text);
                assert_eq!(value.os_status, Some(0));
                assert!(value.value.is_none(), "silently shortened {text:?}");
                assert_eq!(value.reason, Some("incomplete_property_conversion"));
                assert_ne!(value, prefix);
            }
            for text in ["", "한글 😀", &"😀".repeat(256)] {
                let value = converted(text);
                assert_eq!(value.value.as_deref(), Some(text));
                assert_eq!(value.reason, None);
            }
            let excessive = converted(&"😀".repeat(257));
            assert_eq!(excessive.value, None);
            assert_eq!(excessive.reason, Some("excessive_property_length"));
        }

        fn packet_list(messages: &[(u64, &[u8])]) -> Vec<u64> {
            let mut storage = vec![0u64; 16_400];
            let bytes = storage.as_mut_ptr().cast::<u8>();
            unsafe { bytes.cast::<u32>().write_unaligned(messages.len() as u32) };
            let mut at = 4;
            for (timestamp, payload) in messages {
                unsafe {
                    bytes.add(at).cast::<u64>().write_unaligned(*timestamp);
                    bytes
                        .add(at + 8)
                        .cast::<u16>()
                        .write_unaligned(payload.len() as u16);
                    ptr::copy_nonoverlapping(payload.as_ptr(), bytes.add(at + 10), payload.len());
                }
                at += 10 + payload.len();
                if cfg!(target_arch = "aarch64") {
                    at = (at + 3) & !3;
                }
            }
            storage
        }

        fn call(storage: &[u64], generation: u64) {
            unsafe {
                receive(
                    storage.as_ptr().cast(),
                    generation as usize as *mut c_void,
                    ptr::null_mut(),
                );
            }
        }

        #[test]
        fn sdk_packet_layout_and_variable_packet_alignment_decode_as_defined() {
            let _serial = SERIAL.lock().unwrap();
            assert_eq!(std::mem::size_of::<Packet>(), 268);
            assert_eq!(std::mem::align_of::<Packet>(), 4);
            assert_eq!(std::mem::offset_of!(Packet, data), 10);
            assert_eq!(std::mem::offset_of!(PacketList, packet), 4);
            assert_eq!(std::mem::size_of::<PacketList>(), 272);
            let storage = packet_list(&[(91, &[0x90, 60, 100]), (92, &[0xc0, 7])]);
            SHARED.begin(71, Duration::from_secs(1), 8).unwrap();
            call(&storage, 0);
            call(&storage, 70);
            call(&storage, 71);
            let capture = SHARED.end().unwrap();
            assert_eq!(capture.length, 2);
            assert_eq!(capture.events[0].timestamp, 91);
            assert_eq!(capture.events[1].first, 7);
            call(&storage, 71);
            SHARED.begin(72, Duration::from_secs(1), 8).unwrap();
            call(&storage, 71);
            assert_eq!(SHARED.end().unwrap().length, 0);
        }

        #[test]
        fn native_callback_bounds_packet_count_and_total_bytes_without_payloads() {
            let _serial = SERIAL.lock().unwrap();
            SHARED.begin(81, Duration::from_secs(1), 8).unwrap();
            let mut excessive_count = [0u64; 1];
            unsafe {
                excessive_count
                    .as_mut_ptr()
                    .cast::<u32>()
                    .write_unaligned(1025);
            }
            call(&excessive_count, 81);
            let payload = vec![0xf0; 40_000];
            let storage = packet_list(&[(0, &payload), (0, &payload)]);
            call(&storage, 81);
            let capture = SHARED.end().unwrap();
            assert_eq!(capture.length, 0);
            assert_eq!(capture.counts.packet_budget_drops, 2);
            assert_eq!(capture.counts.unsupported_packets, 1);
        }
    }
}

#[cfg(test)]
#[path = "midi_input/tests.rs"]
mod tests;

#[cfg(any(target_os = "macos", test))]
mod inventory {
    use serde::Serialize;

    const MAX_DEVICES: usize = 128;
    const MAX_ENDPOINTS: usize = 256;
    #[cfg(target_os = "macos")]
    const MAX_PROPERTY_BYTES: usize = 1_048_576;
    #[cfg(target_os = "macos")]
    const MAX_STRING_UNITS: usize = 512;

    #[derive(Debug, Clone, Serialize, PartialEq)]
    #[serde(tag = "status", rename_all = "snake_case")]
    enum Observation<T> {
        Observed {
            value: T,
        },
        Unavailable {
            os_status: Option<i32>,
            reason: &'static str,
        },
    }

    impl<T> Observation<T> {
        fn from_result(value: Result<T, i32>) -> Self {
            match value {
                Ok(value) => Self::Observed { value },
                Err(status) => Self::Unavailable {
                    os_status: Some(status),
                    reason: "property_read_failed",
                },
            }
        }

        fn invalid(reason: &'static str) -> Self {
            Self::Unavailable {
                os_status: None,
                reason,
            }
        }

        fn value(&self) -> Option<&T> {
            match self {
                Self::Observed { value } => Some(value),
                _ => None,
            }
        }
    }

    #[derive(Debug, Serialize)]
    struct AudioDevice {
        object_id: u32,
        uid: Observation<String>,
        name: Observation<String>,
        manufacturer: Observation<String>,
        alive: Observation<bool>,
        nominal_sample_rate: Observation<f64>,
        buffer_frame_size: Observation<u32>,
        input_channels: Observation<u32>,
        output_channels: Observation<u32>,
    }

    #[derive(Debug, Serialize)]
    struct MidiEndpoint {
        endpoint_ref: u32,
        unique_id: Observation<i32>,
        name: Observation<String>,
        display_name: Observation<String>,
        manufacturer: Observation<String>,
        model: Observation<String>,
        offline: Observation<bool>,
    }

    #[derive(Debug, Serialize)]
    struct Membership<T> {
        ids_before: Observation<Vec<u32>>,
        ids_after: Observation<Vec<u32>>,
        count_before: Option<usize>,
        count_after: Option<usize>,
        membership_unchanged: Option<bool>,
        records: Vec<T>,
    }

    #[derive(Debug, Serialize)]
    struct Inventory {
        scope: &'static str,
        observation: &'static str,
        audio_devices: Membership<AudioDevice>,
        default_audio_input: Observation<u32>,
        default_audio_output: Observation<u32>,
        default_system_audio_output: Observation<u32>,
        midi_sources: Membership<MidiEndpoint>,
        midi_destinations: Membership<MidiEndpoint>,
        mutation_capability: bool,
        limitations: [&'static str; 4],
    }

    trait Backend {
        fn audio_ids(&self) -> Observation<Vec<u32>>;
        fn audio(&self, id: u32) -> AudioDevice;
        fn default_audio(&self, selector: u32) -> Observation<u32>;
        fn midi_ids(&self, source: bool) -> Observation<Vec<u32>>;
        fn midi(&self, id: u32) -> MidiEndpoint;
    }

    fn membership<T>(
        before: Observation<Vec<u32>>,
        records: Vec<T>,
        after: Observation<Vec<u32>>,
    ) -> Membership<T> {
        let unchanged = before.value().zip(after.value()).map(|(a, b)| a == b);
        Membership {
            count_before: before.value().map(Vec::len),
            count_after: after.value().map(Vec::len),
            ids_before: before,
            ids_after: after,
            membership_unchanged: unchanged,
            records,
        }
    }

    fn bounded_ids(value: Observation<Vec<u32>>, limit: usize) -> Observation<Vec<u32>> {
        if value.value().is_some_and(|ids| {
            ids.len() > limit
                || ids.contains(&0)
                || ids
                    .iter()
                    .enumerate()
                    .any(|(index, id)| ids[..index].contains(id))
        }) {
            Observation::invalid("invalid_or_excessive_membership")
        } else {
            value
        }
    }

    fn collect(backend: &impl Backend) -> Inventory {
        let audio_before = bounded_ids(backend.audio_ids(), MAX_DEVICES);
        let sources_before = bounded_ids(backend.midi_ids(true), MAX_ENDPOINTS);
        let destinations_before = bounded_ids(backend.midi_ids(false), MAX_ENDPOINTS);
        let audio = audio_before
            .value()
            .into_iter()
            .flatten()
            .map(|id| backend.audio(*id))
            .collect();
        let sources = sources_before
            .value()
            .into_iter()
            .flatten()
            .map(|id| backend.midi(*id))
            .collect();
        let destinations = destinations_before
            .value()
            .into_iter()
            .flatten()
            .map(|id| backend.midi(*id))
            .collect();
        Inventory {
            scope: "operating_system_coreaudio_coremidi",
            observation: "sequential_metadata_snapshot",
            default_audio_input: backend.default_audio(u32::from_be_bytes(*b"dIn ")),
            default_audio_output: backend.default_audio(u32::from_be_bytes(*b"dOut")),
            default_system_audio_output: backend.default_audio(u32::from_be_bytes(*b"sOut")),
            audio_devices: membership(
                audio_before,
                audio,
                bounded_ids(backend.audio_ids(), MAX_DEVICES),
            ),
            midi_sources: membership(
                sources_before,
                sources,
                bounded_ids(backend.midi_ids(true), MAX_ENDPOINTS),
            ),
            midi_destinations: membership(
                destinations_before,
                destinations,
                bounded_ids(backend.midi_ids(false), MAX_ENDPOINTS),
            ),
            mutation_capability: false,
            limitations: [
                "OS devices and defaults do not identify the DAW's selected engine or routing.",
                "Membership equality is sequential metadata, not an atomic identity or connection guarantee.",
                "This inventory routine creates no audio stream, MIDI client, port, observer, message, or physical key test.",
                "UIDs and endpoint references must be resolved afresh; signed MIDI unique IDs may be negative.",
            ],
        }
    }

    #[cfg(target_os = "macos")]
    pub(super) fn inspect() -> Result<serde_json::Value, String> {
        serde_json::to_value(collect(&macos::Native)).map_err(|error| error.to_string())
    }

    #[cfg(target_os = "macos")]
    mod macos {
        use super::*;
        use std::{
            ffi::{c_char, c_void},
            mem, ptr,
        };

        #[repr(C)]
        #[derive(Clone, Copy)]
        struct Address {
            selector: u32,
            scope: u32,
            element: u32,
        }
        #[repr(C)]
        struct AudioBuffer {
            channels: u32,
            byte_size: u32,
            data: *mut c_void,
        }
        #[repr(C)]
        struct AudioBufferList {
            count: u32,
            buffers: [AudioBuffer; 1],
        }
        type Size = unsafe extern "C" fn(u32, *const Address, u32, *const c_void, *mut u32) -> i32;
        type Data = unsafe extern "C" fn(
            u32,
            *const Address,
            u32,
            *const c_void,
            *mut u32,
            *mut c_void,
        ) -> i32;

        #[link(name = "CoreAudio", kind = "framework")]
        unsafe extern "C" {
            fn AudioObjectGetPropertyDataSize(
                object: u32,
                address: *const Address,
                qualifier_size: u32,
                qualifier: *const c_void,
                size: *mut u32,
            ) -> i32;
            fn AudioObjectGetPropertyData(
                object: u32,
                address: *const Address,
                qualifier_size: u32,
                qualifier: *const c_void,
                size: *mut u32,
                value: *mut c_void,
            ) -> i32;
        }
        #[link(name = "CoreFoundation", kind = "framework")]
        unsafe extern "C" {
            fn CFRelease(object: *const c_void);
            fn CFGetTypeID(object: *const c_void) -> usize;
            fn CFStringGetTypeID() -> usize;
            fn CFStringGetLength(string: *const c_void) -> isize;
            fn CFStringGetCString(
                string: *const c_void,
                buffer: *mut c_char,
                size: isize,
                encoding: u32,
            ) -> u8;
            fn CFStringCreateWithBytes(
                allocator: *const c_void,
                bytes: *const u8,
                count: isize,
                encoding: u32,
                external: u8,
            ) -> *const c_void;
        }
        #[link(name = "CoreMIDI", kind = "framework")]
        unsafe extern "C" {
            fn MIDIGetNumberOfSources() -> usize;
            fn MIDIGetNumberOfDestinations() -> usize;
            fn MIDIGetSource(index: usize) -> u32;
            fn MIDIGetDestination(index: usize) -> u32;
            fn MIDIObjectGetStringProperty(
                object: u32,
                key: *const c_void,
                value: *mut *const c_void,
            ) -> i32;
            fn MIDIObjectGetIntegerProperty(
                object: u32,
                key: *const c_void,
                value: *mut i32,
            ) -> i32;
        }
        const UTF8: u32 = 0x0800_0100;
        const GLOBAL: u32 = u32::from_be_bytes(*b"glob");
        struct OwnedCf(*const c_void);
        impl Drop for OwnedCf {
            fn drop(&mut self) {
                unsafe { CFRelease(self.0) };
            }
        }

        fn cf_string(value: OwnedCf) -> Observation<String> {
            if unsafe { CFGetTypeID(value.0) } != unsafe { CFStringGetTypeID() } {
                return Observation::invalid("unexpected_cf_type");
            }
            let length = unsafe { CFStringGetLength(value.0) };
            if !(0..=MAX_STRING_UNITS as isize).contains(&length) {
                return Observation::invalid("string_limit_exceeded");
            }
            let mut buffer = vec![0u8; length as usize * 3 + 1];
            if unsafe {
                CFStringGetCString(
                    value.0,
                    buffer.as_mut_ptr().cast(),
                    buffer.len() as isize,
                    UTF8,
                )
            } == 0
            {
                return Observation::invalid("string_conversion_failed");
            }
            let end = buffer
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(buffer.len());
            match String::from_utf8(buffer[..end].to_vec()) {
                Ok(value) if value.encode_utf16().count() == length as usize => {
                    Observation::Observed { value }
                }
                Ok(_) => Observation::invalid("incomplete_string_conversion"),
                Err(_) => Observation::invalid("invalid_utf8"),
            }
        }

        struct AudioApi {
            size: Size,
            data: Data,
        }
        impl AudioApi {
            fn bytes(
                &self,
                object: u32,
                selector: u32,
                scope: u32,
                cap: usize,
            ) -> Result<Vec<u8>, Observation<Vec<u8>>> {
                let address = Address {
                    selector,
                    scope,
                    element: 0,
                };
                let mut size = 0;
                let status = unsafe { (self.size)(object, &address, 0, ptr::null(), &mut size) };
                if status != 0 {
                    return Err(Observation::from_result(Err(status)));
                }
                if size as usize > cap || size as usize > MAX_PROPERTY_BYTES {
                    return Err(Observation::invalid("property_limit_exceeded"));
                }
                let mut buffer = vec![0u64; (size as usize).div_ceil(mem::size_of::<u64>()).max(1)];
                let capacity = size;
                let status = unsafe {
                    (self.data)(
                        object,
                        &address,
                        0,
                        ptr::null(),
                        &mut size,
                        buffer.as_mut_ptr().cast(),
                    )
                };
                if status != 0 {
                    return Err(Observation::from_result(Err(status)));
                }
                if size > capacity {
                    return Err(Observation::invalid("property_size_changed"));
                }
                Ok(unsafe {
                    std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), size as usize)
                }
                .to_vec())
            }
            fn raw(
                &self,
                object: u32,
                selector: u32,
                scope: u32,
                cap: usize,
            ) -> Observation<Vec<u8>> {
                match self.bytes(object, selector, scope, cap) {
                    Ok(value) => Observation::Observed { value },
                    Err(error) => error,
                }
            }
            fn scalar<const N: usize>(&self, object: u32, selector: u32) -> Observation<[u8; N]> {
                match self.bytes(object, selector, GLOBAL, N) {
                    Ok(bytes) => match bytes.try_into() {
                        Ok(value) => Observation::Observed { value },
                        Err(_) => Observation::invalid("unexpected_property_size"),
                    },
                    Err(Observation::Unavailable { os_status, reason }) => {
                        Observation::Unavailable { os_status, reason }
                    }
                    _ => unreachable!(),
                }
            }
            fn integer(&self, object: u32, selector: u32) -> Observation<u32> {
                match self.scalar::<4>(object, selector) {
                    Observation::Observed { value } => Observation::Observed {
                        value: u32::from_ne_bytes(value),
                    },
                    Observation::Unavailable { os_status, reason } => {
                        Observation::Unavailable { os_status, reason }
                    }
                }
            }
            fn boolean(&self, object: u32, selector: u32) -> Observation<bool> {
                boolean(self.integer(object, selector))
            }
            fn string(&self, object: u32, selector: u32) -> Observation<String> {
                match self.scalar::<{ mem::size_of::<usize>() }>(object, selector) {
                    Observation::Observed { value } => {
                        let pointer = usize::from_ne_bytes(value) as *const c_void;
                        if pointer.is_null() {
                            Observation::invalid("null_cf_string")
                        } else {
                            cf_string(OwnedCf(pointer))
                        }
                    }
                    Observation::Unavailable { os_status, reason } => {
                        Observation::Unavailable { os_status, reason }
                    }
                }
            }
            fn rate(&self, object: u32) -> Observation<f64> {
                match self.scalar::<8>(object, u32::from_be_bytes(*b"nsrt")) {
                    Observation::Observed { value } => {
                        let value = f64::from_ne_bytes(value);
                        if value.is_finite() && value > 0.0 {
                            Observation::Observed { value }
                        } else {
                            Observation::invalid("invalid_sample_rate")
                        }
                    }
                    Observation::Unavailable { os_status, reason } => {
                        Observation::Unavailable { os_status, reason }
                    }
                }
            }
            fn channels(&self, object: u32, scope: u32) -> Observation<u32> {
                match self.raw(
                    object,
                    u32::from_be_bytes(*b"slay"),
                    scope,
                    MAX_PROPERTY_BYTES,
                ) {
                    Observation::Observed { value } => parse_channels(&value),
                    Observation::Unavailable { os_status, reason } => {
                        Observation::Unavailable { os_status, reason }
                    }
                }
            }
        }

        fn boolean(value: Observation<u32>) -> Observation<bool> {
            match value {
                Observation::Observed { value: 0 } => Observation::Observed { value: false },
                Observation::Observed { value: 1 } => Observation::Observed { value: true },
                Observation::Observed { .. } => Observation::invalid("invalid_boolean_property"),
                Observation::Unavailable { os_status, reason } => {
                    Observation::Unavailable { os_status, reason }
                }
            }
        }
        fn parse_ids(bytes: Vec<u8>, limit: usize) -> Observation<Vec<u32>> {
            if bytes.len() % 4 != 0 || bytes.len() / 4 > limit {
                return Observation::invalid("invalid_membership_size");
            }
            bounded_ids(
                Observation::Observed {
                    value: bytes
                        .chunks_exact(4)
                        .map(|value| u32::from_ne_bytes(value.try_into().unwrap()))
                        .collect(),
                },
                limit,
            )
        }
        fn parse_channels(bytes: &[u8]) -> Observation<u32> {
            let offset = mem::offset_of!(AudioBufferList, buffers);
            if bytes.len() < 4 {
                return Observation::invalid("truncated_buffer_list");
            }
            let count = u32::from_ne_bytes(bytes[..4].try_into().unwrap()) as usize;
            if count == 0 && (bytes.len() == 4 || bytes.len() == offset) {
                return Observation::Observed { value: 0 };
            }
            let expected = count
                .checked_mul(mem::size_of::<AudioBuffer>())
                .and_then(|size| size.checked_add(offset));
            if count > 128 || expected.is_none_or(|size| size != bytes.len()) {
                return Observation::invalid("invalid_buffer_list_size");
            }
            let mut channels = 0u32;
            for buffer in bytes[offset..].chunks_exact(mem::size_of::<AudioBuffer>()) {
                let count = u32::from_ne_bytes(buffer[..4].try_into().unwrap());
                let Some(total) = channels.checked_add(count).filter(|sum| *sum <= 4096) else {
                    return Observation::invalid("channel_limit_exceeded");
                };
                channels = total;
            }
            Observation::Observed { value: channels }
        }

        fn key(name: &str) -> Option<OwnedCf> {
            let value = unsafe {
                CFStringCreateWithBytes(ptr::null(), name.as_ptr(), name.len() as isize, UTF8, 0)
            };
            (!value.is_null()).then_some(OwnedCf(value))
        }
        fn midi_string(object: u32, name: &str) -> Observation<String> {
            let Some(key) = key(name) else {
                return Observation::invalid("property_key_unavailable");
            };
            let mut value = ptr::null();
            let status = unsafe { MIDIObjectGetStringProperty(object, key.0, &mut value) };
            if status != 0 {
                return Observation::from_result(Err(status));
            }
            if value.is_null() {
                Observation::invalid("null_cf_string")
            } else {
                cf_string(OwnedCf(value))
            }
        }
        fn midi_integer(object: u32, name: &str) -> Observation<i32> {
            let Some(key) = key(name) else {
                return Observation::invalid("property_key_unavailable");
            };
            let mut value = 0;
            let status = unsafe { MIDIObjectGetIntegerProperty(object, key.0, &mut value) };
            Observation::from_result(if status == 0 { Ok(value) } else { Err(status) })
        }
        fn midi_boolean(object: u32, name: &str) -> Observation<bool> {
            match midi_integer(object, name) {
                Observation::Observed { value } => boolean(Observation::Observed {
                    value: value as u32,
                }),
                Observation::Unavailable { os_status, reason } => {
                    Observation::Unavailable { os_status, reason }
                }
            }
        }

        pub(super) struct Native;
        impl Backend for Native {
            fn audio_ids(&self) -> Observation<Vec<u32>> {
                let api = AudioApi {
                    size: AudioObjectGetPropertyDataSize,
                    data: AudioObjectGetPropertyData,
                };
                match api.raw(1, u32::from_be_bytes(*b"dev#"), GLOBAL, MAX_DEVICES * 4) {
                    Observation::Observed { value } => parse_ids(value, MAX_DEVICES),
                    Observation::Unavailable { os_status, reason } => {
                        Observation::Unavailable { os_status, reason }
                    }
                }
            }
            fn audio(&self, id: u32) -> AudioDevice {
                let api = AudioApi {
                    size: AudioObjectGetPropertyDataSize,
                    data: AudioObjectGetPropertyData,
                };
                AudioDevice {
                    object_id: id,
                    uid: api.string(id, u32::from_be_bytes(*b"uid ")),
                    name: api.string(id, u32::from_be_bytes(*b"lnam")),
                    manufacturer: api.string(id, u32::from_be_bytes(*b"lmak")),
                    alive: api.boolean(id, u32::from_be_bytes(*b"livn")),
                    nominal_sample_rate: api.rate(id),
                    buffer_frame_size: api.integer(id, u32::from_be_bytes(*b"fsiz")),
                    input_channels: api.channels(id, u32::from_be_bytes(*b"inpt")),
                    output_channels: api.channels(id, u32::from_be_bytes(*b"outp")),
                }
            }
            fn default_audio(&self, selector: u32) -> Observation<u32> {
                AudioApi {
                    size: AudioObjectGetPropertyDataSize,
                    data: AudioObjectGetPropertyData,
                }
                .integer(1, selector)
            }
            fn midi_ids(&self, source: bool) -> Observation<Vec<u32>> {
                let count = unsafe {
                    if source {
                        MIDIGetNumberOfSources()
                    } else {
                        MIDIGetNumberOfDestinations()
                    }
                };
                if count > MAX_ENDPOINTS {
                    return Observation::invalid("endpoint_limit_exceeded");
                }
                let ids = (0..count)
                    .map(|index| unsafe {
                        if source {
                            MIDIGetSource(index)
                        } else {
                            MIDIGetDestination(index)
                        }
                    })
                    .collect();
                bounded_ids(Observation::Observed { value: ids }, MAX_ENDPOINTS)
            }
            fn midi(&self, id: u32) -> MidiEndpoint {
                MidiEndpoint {
                    endpoint_ref: id,
                    unique_id: midi_integer(id, "uniqueID"),
                    name: midi_string(id, "name"),
                    display_name: midi_string(id, "displayName"),
                    manufacturer: midi_string(id, "manufacturer"),
                    model: midi_string(id, "model"),
                    offline: midi_boolean(id, "offline"),
                }
            }
        }

        #[cfg(test)]
        mod tests {
            include!("hardware/macos_tests.rs");
        }
    }

    #[cfg(test)]
    mod tests {
        include!("hardware/tests.rs");
    }
}

pub fn inspect() -> Result<serde_json::Value, String> {
    #[cfg(target_os = "macos")]
    {
        inventory::inspect()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("CoreAudio/CoreMIDI hardware inventory currently supports macOS only".into())
    }
}

#[path = "hardware/midi_input.rs"]
pub mod midi_input;

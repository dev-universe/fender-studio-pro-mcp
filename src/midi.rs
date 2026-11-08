use crate::model::OperationError;

pub const PORT_NAME: &str = "fender-studio-pro-mcp";

#[cfg(target_os = "macos")]
pub struct MidiWakeup {
    source: coremidi::VirtualSource,
    _client: coremidi::Client,
}

#[cfg(target_os = "macos")]
impl MidiWakeup {
    pub fn new(profile: &str) -> Result<Self, OperationError> {
        let client = coremidi::Client::new(PORT_NAME).map_err(|e| {
            OperationError::new("midi_unavailable", format!("CoreMIDI client status {e}"))
        })?;
        let source = client.virtual_source(PORT_NAME).map_err(|e| {
            OperationError::new("midi_unavailable", format!("Virtual source status {e}"))
        })?;
        let uid = profile.bytes().fold(0x465350u32, |h, b| {
            (h ^ u32::from(b)).wrapping_mul(16_777_619)
        }) & 0x7fff_ffff;
        source
            .set_property_integer("uniqueID", uid as i32)
            .map_err(|e| {
                OperationError::new(
                    "midi_identity",
                    format!("Cannot set stable endpoint ID: {e}"),
                )
            })?;
        Ok(Self {
            source,
            _client: client,
        })
    }

    pub fn nudge(&mut self) -> Result<(), OperationError> {
        let packets = coremidi::PacketBuffer::new(0, &[0x90, 119, 127, 0x90, 119, 0]);
        self.source
            .received(&packets)
            .map_err(|e| OperationError::new("midi_send", format!("MIDIReceived status {e}")))
    }
}

#[cfg(not(target_os = "macos"))]
pub struct MidiWakeup;

#[cfg(not(target_os = "macos"))]
impl MidiWakeup {
    pub fn new(_profile: &str) -> Result<Self, OperationError> {
        Err(OperationError::new(
            "unsupported_platform",
            "Native MIDI control currently requires macOS",
        ))
    }

    pub fn nudge(&mut self) -> Result<(), OperationError> {
        Err(OperationError::new(
            "unsupported_platform",
            "Native MIDI control currently requires macOS",
        ))
    }
}

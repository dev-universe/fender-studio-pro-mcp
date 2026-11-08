use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::model::OperationError;

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Empty {}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "operation", content = "args")]
pub enum Operation {
    #[serde(rename = "ping")]
    Ping(Empty),
    #[serde(rename = "context")]
    Context(Empty),
    #[serde(rename = "hostProbe", alias = "host_probe")]
    HostProbe(HostProbeArgs),
    #[serde(rename = "song", alias = "session_get")]
    Song(Empty),
    #[serde(rename = "tracks", alias = "tracks_list")]
    Tracks(TrackQuery),
    #[serde(rename = "channels", alias = "channels_list")]
    Channels(Empty),
    #[serde(rename = "panner", alias = "panner_get")]
    Panner(PannerQuery),
    #[serde(rename = "setPanner", alias = "panner_set")]
    SetPanner(PannerSet),
    #[serde(rename = "meters")]
    Meters(Empty),
    #[serde(rename = "notes")]
    Notes(NotesQuery),
    #[serde(rename = "plugins", alias = "plugins_list")]
    Plugins(PluginQuery),
    #[serde(rename = "pluginCatalog", alias = "plugin_catalog")]
    PluginCatalog(PluginCatalogQuery),
    #[serde(rename = "inserts")]
    Inserts(ChannelQuery),
    #[serde(rename = "sends")]
    Sends(ChannelQuery),
    #[serde(rename = "setSend", alias = "send_set")]
    SetSend(SendSet),
    #[serde(rename = "setInsertBypass", alias = "plugin_bypass")]
    SetInsertBypass(InsertBypass),
    #[serde(rename = "setAutomation", alias = "automation_mode_set")]
    SetAutomation(AutomationSet),
    #[serde(rename = "metronome")]
    Metronome(Metronome),
    #[serde(rename = "takes")]
    Takes(Takes),
    #[serde(rename = "trackState")]
    TrackState(TrackState),
    #[serde(rename = "editEvents", alias = "events_edit")]
    EditEvents(EditEvents),
    #[serde(rename = "pluginParams", alias = "plugin_parameters")]
    PluginParams(PluginParams),
    #[serde(rename = "pluginInspect", alias = "plugin_inspect")]
    PluginInspect(PluginParams),
    #[serde(rename = "pluginEditorOpen", alias = "plugin_editor_open")]
    PluginEditorOpen(PluginEditor),
    #[serde(rename = "setPluginParam", alias = "plugin_parameter_set")]
    SetPluginParam(PluginSet),
    #[serde(rename = "setChannel", alias = "channel_set")]
    SetChannel(ChannelSet),
    #[serde(rename = "setChannelLabel")]
    SetChannelLabel(ChannelLabel),
    #[serde(rename = "setChannelColor")]
    SetChannelColor(ChannelColor),
    #[serde(rename = "setTransport", alias = "transport_set")]
    SetTransport(TransportSet),
    #[serde(rename = "transport")]
    Transport(TransportAction),
    #[serde(rename = "setLoop", alias = "loop_set")]
    SetLoop(LoopSet),
    #[serde(rename = "addTrack")]
    AddTrack(AddTrack),
    #[serde(rename = "selectTrack")]
    SelectTrack(SelectTrack),
    #[serde(rename = "listCommands", alias = "commands_search")]
    ListCommands(CommandQuery),
    #[serde(rename = "command", alias = "command_run")]
    Command(Command),
    #[serde(rename = "trackTask", alias = "track_task")]
    TrackTask(TrackTask),
    #[serde(rename = "editNotes", alias = "notes_edit")]
    EditNotes(EditNotes),
    #[serde(rename = "markers")]
    Markers(Empty),
    #[serde(rename = "addMarker")]
    AddMarker(AddMarker),
    #[serde(rename = "deleteMarker")]
    DeleteMarker(DeleteMarker),
    #[serde(rename = "selectEvents")]
    SelectEvents(SelectEvents),
    #[serde(rename = "save")]
    Save(Save),
    #[serde(rename = "undo")]
    Undo(Steps),
    #[serde(rename = "redo")]
    Redo(Steps),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrackQuery {
    pub name: Option<String>,
    pub events: Option<bool>,
    pub max_events: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HostProbeArgs {
    pub channel: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NotesQuery {
    pub track: String,
    pub max_notes: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PluginQuery {
    pub kind: Option<PluginKind>,
    pub filter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PluginKind {
    Effect,
    Instrument,
    NoteEffect,
    MixFx,
    Panner,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PluginCatalogQuery {
    pub category: PluginCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum PluginCategory {
    AudioEffect,
    AudioSynth,
    MusicEffect,
    AudioPanner,
    #[serde(alias = "MixEffect")]
    AudioMixEffect,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChannelQuery {
    pub channel: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PannerQuery {
    pub channel: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PannerSet {
    pub channel: String,
    pub param: PannerField,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum PannerField {
    PannerType,
    StereoMode,
    PanX1,
    PanX2,
    PanWidth,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SendSet {
    pub channel: String,
    pub index: u32,
    pub level: Option<f64>,
    pub muted: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InsertBypass {
    pub channel: String,
    pub slot: InsertSlot,
    pub plugin: Option<String>,
    pub bypassed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum InsertSlot {
    Index(u32),
    All(AllSlots),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum AllSlots {
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AutomationSet {
    pub channel: String,
    pub mode: AutomationMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum AutomationMode {
    Off,
    Read,
    Touch,
    Latch,
    Write,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Metronome {
    pub click: Option<bool>,
    pub precount: Option<bool>,
    pub preroll: Option<bool>,
    pub precount_bars: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Takes {
    pub track: String,
    pub action: TakeAction,
    pub take: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum TakeAction {
    List,
    Next,
    Previous,
    Unpack,
    Goto,
    Add,
    Duplicate,
    Retrospective,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrackState {
    pub track: String,
    pub action: TrackAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum TrackAction {
    Hide,
    Duplicate,
    ShowAll,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EditEvents {
    pub track: String,
    pub action: EventAction,
    pub at: Option<TimePoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum EventAction {
    Mute,
    Unmute,
    Quantize,
    TransposeUp,
    TransposeDown,
    Split,
    TrimStart,
    TrimEnd,
    Merge,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PluginParams {
    pub channel: Option<String>,
    pub slot: Option<u32>,
    pub kind: Option<PluginKind>,
    pub track: Option<String>,
    pub plugin: Option<String>,
    #[serde(default)]
    pub names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PluginEditor {
    pub channel: Option<String>,
    pub slot: Option<u32>,
    pub kind: Option<PluginKind>,
    pub track: Option<String>,
    pub plugin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PluginSet {
    pub channel: Option<String>,
    pub slot: Option<u32>,
    pub kind: Option<PluginKind>,
    pub track: Option<String>,
    pub plugin: String,
    pub param: String,
    pub value: Option<f64>,
    pub normalized: Option<f64>,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChannelSet {
    pub channel: String,
    pub field: ChannelField,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ChannelField {
    #[serde(rename = "volume")]
    Volume,
    #[serde(rename = "pan")]
    Pan,
    #[serde(rename = "mute")]
    Mute,
    #[serde(rename = "solo")]
    Solo,
    #[serde(rename = "recordArmed")]
    RecordArmed,
    #[serde(rename = "monitor")]
    Monitor,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChannelLabel {
    pub channel: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChannelColor {
    pub channel: String,
    pub argb: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TransportSet {
    pub tempo: Option<f64>,
    pub position: Option<TimePoint>,
    #[serde(rename = "loop")]
    pub looping: Option<bool>,
    pub precount: Option<bool>,
    pub preroll: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum TimePoint {
    Seconds(f64),
    Bars(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TransportAction {
    pub action: TransportButton,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum TransportButton {
    Play,
    Stop,
    Record,
    ReturnToZero,
    Rewind,
    Forward,
    LoopStart,
    LoopEnd,
    #[serde(rename = "toggleLoop", alias = "loop")]
    Loop,
    #[serde(rename = "toggleClick", alias = "click")]
    Click,
    #[serde(rename = "togglePrecount", alias = "precount")]
    Precount,
    #[serde(rename = "togglePreroll", alias = "preroll")]
    Preroll,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LoopSet {
    pub start: TimePoint,
    pub end: TimePoint,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddTrack {
    #[serde(rename = "type")]
    pub kind: TrackKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum TrackKind {
    AudioMono,
    AudioStereo,
    Instrument,
    Folder,
    Automation,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectTrack {
    pub name: String,
    pub exclusive: Option<bool>,
    pub media_type: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandQuery {
    pub filter: Option<String>,
    pub with_state: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Command {
    pub category: String,
    pub name: String,
    pub args: Option<serde_json::Map<String, Value>>,
    pub check_only: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrackTask {
    pub ops: Vec<TrackEdit>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "op",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum TrackEdit {
    Probe {
        channel: Option<String>,
    },
    Route {
        channel: String,
        to: String,
    },
    Folder {
        folder: String,
        tracks: Vec<String>,
        create: Option<bool>,
    },
    RenameEvents {
        track: String,
        name: String,
        numbered: Option<bool>,
    },
    AddPlugin {
        channel: String,
        plugin: String,
    },
    AddMixFx {
        channel: String,
        plugin: String,
    },
    AddFxSend {
        channel: String,
        plugin: String,
    },
    AddInstrumentTrack {
        instrument: String,
        name: Option<String>,
    },
    MoveTrack {
        track: String,
        before: Option<String>,
        after: Option<String>,
    },
    Events {
        track: String,
    },
    EditEvent {
        track: String,
        event: Value,
        to: Option<f64>,
        to_track: Option<String>,
        gain_db: Option<f64>,
        add_gain_db: Option<f64>,
        fade_in: Option<f64>,
        fade_out: Option<f64>,
    },
    SelectEvent {
        track: String,
        event: Value,
    },
    FocusTrack {
        track: String,
    },
    Sections {},
    AddSection {
        name: String,
        start: f64,
        end: f64,
    },
    EditSection {
        section: Value,
        name: Option<String>,
        start: Option<f64>,
        end: Option<f64>,
        remove: Option<bool>,
    },
    SelectSection {
        section: Value,
    },
    Markers {},
    RenameMarker {
        marker: Value,
        name: String,
    },
    Signatures {
        at: Option<Vec<f64>>,
    },
    CreatePart {
        track: String,
        start: f64,
        end: f64,
        name: Option<String>,
        #[serde(default)]
        notes: Vec<Note>,
    },
    AddNotes {
        track: String,
        part: Option<Value>,
        notes: Vec<Note>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Note {
    pub pitch: u8,
    pub beat: f64,
    pub length: f64,
    pub velocity: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EditNotes {
    pub track: String,
    pub ops: Vec<NoteEdit>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "op", rename_all = "camelCase", deny_unknown_fields)]
pub enum NoteEdit {
    Transpose {
        semitones: i16,
        filter: Option<NoteFilter>,
    },
    Velocity {
        set: Option<u8>,
        add: Option<i16>,
        filter: Option<NoteFilter>,
    },
    Move {
        beats: f64,
        filter: Option<NoteFilter>,
    },
    Length {
        set: Option<f64>,
        scale: Option<f64>,
        filter: Option<NoteFilter>,
    },
    Quantize {
        grid: f64,
        strength: Option<f64>,
        filter: Option<NoteFilter>,
    },
    Delete {
        filter: NoteFilter,
    },
    Add {
        notes: Vec<Note>,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NoteFilter {
    pub pitch: Option<u8>,
    pub pitches: Option<Vec<u8>>,
    pub from: Option<f64>,
    pub to: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddMarker {
    pub position: TimePoint,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeleteMarker {
    pub number: Option<u32>,
    pub position: Option<TimePoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectEvents {
    pub tracks: Option<Vec<String>>,
    pub track: Option<String>,
    pub all: Option<bool>,
    pub clear: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Save {
    pub new_version: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Steps {
    pub steps: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Request {
    #[serde(flatten)]
    pub operation: Operation,
    pub expected_document: Option<String>,
}

impl Operation {
    pub fn from_parts(name: &str, args: Value) -> Result<Self, OperationError> {
        serde_json::from_value(json!({"operation":name,"args":args}))
            .map_err(|e| OperationError::new("invalid_argument", e.to_string()))
    }

    pub fn wire(&self) -> (String, Value) {
        let value = serde_json::to_value(self).expect("operation serialization");
        let name = value["operation"].as_str().unwrap().to_owned();
        let mut args = value["args"].clone();
        remove_nulls(&mut args);
        match self {
            Self::SetTransport(transport) => {
                if let Some(position) = &transport.position {
                    args.as_object_mut().unwrap().remove("position");
                    match position {
                        TimePoint::Seconds(seconds) => args["positionSeconds"] = json!(seconds),
                        TimePoint::Bars(bars) => args["positionBars"] = json!(bars),
                    }
                }
            }
            Self::SetLoop(loop_set) => {
                args.as_object_mut().unwrap().remove("enabled");
                if let Some(enabled) = loop_set.enabled {
                    args["enable"] = json!(enabled);
                }
            }
            Self::AddMarker(marker) => {
                args.as_object_mut().unwrap().remove("position");
                if let TimePoint::Seconds(seconds) = marker.position {
                    args["seconds"] = json!(seconds);
                }
            }
            Self::DeleteMarker(marker) => {
                args.as_object_mut().unwrap().remove("position");
                if let Some(TimePoint::Seconds(seconds)) = marker.position {
                    args["seconds"] = json!(seconds);
                }
            }
            Self::SelectEvents(selection) => {
                args.as_object_mut().unwrap().remove("clear");
                if let Some(clear) = selection.clear {
                    args["none"] = json!(clear);
                }
            }
            _ => {}
        }
        (name, args)
    }

    pub fn is_read_only(&self) -> bool {
        match self {
            Self::Ping(_)
            | Self::Context(_)
            | Self::HostProbe(_)
            | Self::Song(_)
            | Self::Tracks(_)
            | Self::Channels(_)
            | Self::Panner(_)
            | Self::Meters(_)
            | Self::Notes(_)
            | Self::Plugins(_)
            | Self::PluginCatalog(_)
            | Self::Inserts(_)
            | Self::Sends(_)
            | Self::PluginParams(_)
            | Self::PluginInspect(_)
            | Self::ListCommands(_) => true,
            Self::Command(command) => command.check_only == Some(true),
            Self::Metronome(settings) => {
                settings.click.is_none()
                    && settings.precount.is_none()
                    && settings.preroll.is_none()
                    && settings.precount_bars.is_none()
            }
            Self::Takes(takes) => matches!(takes.action, TakeAction::List),
            _ => false,
        }
    }

    pub fn validate(&self) -> Result<(), OperationError> {
        let (_, args) = self.wire();
        if serde_json::to_vec(&args).is_ok_and(|v| v.len() > 900_000) {
            return Err(invalid("Operation arguments exceed the supported size"));
        }
        match self {
            Self::Panner(p) if p.channel.trim().is_empty() => {
                return Err(invalid(
                    "A channel name is required for standard panner access",
                ));
            }
            Self::SetPanner(p) if p.channel.trim().is_empty() || !p.value.is_finite() => {
                return Err(invalid(
                    "A channel name and finite raw panner value are required",
                ));
            }
            Self::SetInsertBypass(p) => {
                if let InsertSlot::Index(index) = p.slot
                    && (index >= 16
                        || p.plugin
                            .as_deref()
                            .is_none_or(|name| name.trim().is_empty()))
                {
                    return Err(invalid(
                        "A slot from 0 to 15 and expected plugin class or instance are required",
                    ));
                }
            }
            Self::SetPluginParam(p) => {
                if p.plugin.trim().is_empty() {
                    return Err(invalid(
                        "An expected plugin class or instance name is required",
                    ));
                }
                validate_plugin_target(
                    p.channel.as_deref(),
                    p.slot,
                    p.kind.as_ref(),
                    p.track.as_deref(),
                )?;
                if usize::from(p.value.is_some())
                    + usize::from(p.normalized.is_some())
                    + usize::from(p.text.is_some())
                    != 1
                {
                    return Err(invalid("Specify exactly one of value, normalized, or text"));
                }
                if p.normalized
                    .is_some_and(|v| !v.is_finite() || !(0.0..=1.0).contains(&v))
                    || p.value.is_some_and(|v| !v.is_finite())
                {
                    return Err(invalid("Parameter value is outside its supported range"));
                }
            }
            Self::PluginEditorOpen(p) => {
                if p.plugin.trim().is_empty() {
                    return Err(invalid(
                        "An expected plugin class or instance name is required",
                    ));
                }
                validate_plugin_target(
                    p.channel.as_deref(),
                    p.slot,
                    p.kind.as_ref(),
                    p.track.as_deref(),
                )?;
            }
            Self::PluginParams(p) | Self::PluginInspect(p) => {
                validate_plugin_target(
                    p.channel.as_deref(),
                    p.slot,
                    p.kind.as_ref(),
                    p.track.as_deref(),
                )?;
                if p.names.len() > 10_000 {
                    return Err(invalid(
                        "At most 10,000 parameter names are supported per lookup",
                    ));
                }
            }
            Self::SetChannel(c) => match c.field {
                ChannelField::Volume => validate_number(&c.value, 0.0, 16.0)?,
                ChannelField::Pan => validate_number(&c.value, 0.0, 1.0)?,
                _ => {
                    if !c.value.is_boolean() && !matches!(c.value.as_u64(), Some(0 | 1)) {
                        return Err(invalid("Channel switches accept a boolean or 0/1"));
                    }
                }
            },
            Self::SetSend(send) => {
                if send
                    .level
                    .is_some_and(|v| !v.is_finite() || !(0.0..=1.0).contains(&v))
                {
                    return Err(invalid("Send level must be normalized 0..1"));
                }
            }
            Self::Metronome(settings)
                if settings
                    .precount_bars
                    .is_some_and(|v| !(1..=16).contains(&v)) =>
            {
                return Err(invalid("Precount bars must be 1..16"));
            }
            Self::AddMarker(marker) => {
                if matches!(marker.position, TimePoint::Bars(_)) || marker.name.is_some() {
                    return Err(invalid(
                        "Marker creation currently takes seconds. Rename through a renameMarker track task.",
                    ));
                }
            }
            Self::DeleteMarker(marker) => {
                if marker.number.is_some() == marker.position.is_some()
                    || matches!(marker.position, Some(TimePoint::Bars(_)))
                {
                    return Err(invalid(
                        "Delete marker requires exactly one number or seconds position",
                    ));
                }
            }
            Self::SetTransport(t) => {
                if t.tempo
                    .is_some_and(|v| !v.is_finite() || !(10.0..=999.0).contains(&v))
                {
                    return Err(invalid("Tempo must be between 10 and 999 BPM"));
                }
                if let Some(TimePoint::Seconds(value)) = &t.position
                    && (!value.is_finite() || *value < 0.0)
                {
                    return Err(invalid("Position must be nonnegative seconds"));
                }
            }
            Self::TrackTask(task) if task.ops.is_empty() || task.ops.len() > 100 => {
                return Err(invalid("Track tasks require between 1 and 100 operations"));
            }
            Self::EditNotes(task) if task.ops.is_empty() || task.ops.len() > 100 => {
                return Err(invalid("Note edits require between 1 and 100 operations"));
            }
            Self::Undo(s) | Self::Redo(s) if s.steps.is_some_and(|v| v == 0 || v > 50) => {
                return Err(invalid("Undo or redo steps must be between 1 and 50"));
            }
            _ => {}
        }
        if let Self::TrackTask(task) = self {
            for op in &task.ops {
                match op {
                    TrackEdit::CreatePart {
                        start, end, notes, ..
                    } => {
                        if !start.is_finite() || !end.is_finite() || *start < 0.0 || *end <= *start
                        {
                            return Err(invalid(
                                "Part start/end must be finite seconds with end greater than start",
                            ));
                        }
                        validate_notes(notes)?;
                    }
                    TrackEdit::AddNotes { notes, .. } => validate_notes(notes)?,
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

fn validate_plugin_target(
    channel: Option<&str>,
    slot: Option<u32>,
    kind: Option<&PluginKind>,
    track: Option<&str>,
) -> Result<(), OperationError> {
    if matches!(kind, Some(PluginKind::Instrument | PluginKind::NoteEffect)) {
        if track.is_none_or(str::is_empty) {
            return Err(invalid(
                "Instrument or note-effect parameter access requires track",
            ));
        }
        if matches!(kind, Some(PluginKind::Instrument)) && slot.is_some_and(|v| v != 0) {
            return Err(invalid("The focused instrument supports only slot 0"));
        }
        if matches!(kind, Some(PluginKind::NoteEffect)) && slot.is_some_and(|v| v >= 16) {
            return Err(invalid("Note-effect slots must be between 0 and 15"));
        }
    } else if matches!(kind, Some(PluginKind::MixFx | PluginKind::Panner)) {
        if channel.is_none_or(str::is_empty) {
            return Err(invalid(
                "Mix FX or panner parameter access requires channel",
            ));
        }
        if matches!(kind, Some(PluginKind::MixFx)) && slot.is_some_and(|v| v != 0) {
            return Err(invalid("The native Mix FX rack supports only slot 0"));
        }
    } else if channel.is_none_or(str::is_empty) || slot.is_none_or(|v| v >= 16) {
        return Err(invalid(
            "Insert access requires channel and an explicit slot from 0 to 15",
        ));
    }
    Ok(())
}

fn validate_notes(notes: &[Note]) -> Result<(), OperationError> {
    if notes.len() > 10_000 {
        return Err(invalid("At most 10,000 notes are supported in one batch"));
    }
    for note in notes {
        if note.pitch > 127
            || !note.beat.is_finite()
            || note.beat < 0.0
            || !note.length.is_finite()
            || note.length <= 0.0
            || note.velocity.is_some_and(|v| v == 0 || v > 127)
        {
            return Err(invalid(
                "Notes need MIDI pitch 0..127, velocity 1..127, nonnegative beat and positive length",
            ));
        }
    }
    Ok(())
}

fn validate_number(value: &Value, min: f64, max: f64) -> Result<(), OperationError> {
    if value
        .as_f64()
        .is_none_or(|v| !v.is_finite() || v < min || v > max)
    {
        return Err(invalid(format!("Value must be between {min} and {max}")));
    }
    Ok(())
}

fn invalid(message: impl Into<String>) -> OperationError {
    OperationError::new("invalid_argument", message)
}

fn remove_nulls(value: &mut Value) {
    match value {
        Value::Object(object) => {
            object.retain(|_, v| !v.is_null());
            object.values_mut().for_each(remove_nulls);
        }
        Value::Array(array) => array.iter_mut().for_each(remove_nulls),
        _ => {}
    }
}

pub fn schema() -> Value {
    serde_json::to_value(schemars::schema_for!(Request)).expect("operation schema")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_operations_and_fields_are_rejected() {
        assert!(Operation::from_parts("eval", json!({"code":"arbitrary"})).is_err());
        assert!(
            Operation::from_parts(
                "trackTask",
                json!({"ops":[{"op":"eval","code":"arbitrary"}]})
            )
            .is_err()
        );
        assert!(
            Operation::from_parts(
                "setChannel",
                json!({"channel":"a","field":"mute","value":true,"unexpected":42})
            )
            .is_err()
        );
    }

    #[test]
    fn plugin_write_needs_one_value_and_normalized_range() {
        let op = Operation::from_parts(
            "setPluginParam",
            json!({"channel":"a","slot":0,"plugin":"Limiter","param":"gain","normalized":2.0}),
        )
        .unwrap();
        assert!(op.validate().is_err());
        let op = Operation::from_parts(
            "setPluginParam",
            json!({"channel":"a","slot":0,"plugin":"Limiter","param":"gain","normalized":0.5,"value":1.0}),
        )
        .unwrap();
        assert!(op.validate().is_err());
    }

    #[test]
    fn plugin_write_requires_an_expected_class_or_instance() {
        assert!(
            Operation::from_parts(
                "setPluginParam",
                json!({"channel":"a","slot":0,"param":"gain","value":0})
            )
            .is_err()
        );
        let blank = Operation::from_parts(
            "setPluginParam",
            json!({"channel":"a","slot":0,"plugin":"  ","param":"gain","value":0}),
        )
        .unwrap();
        assert!(blank.validate().is_err());
    }

    #[test]
    fn plugin_editor_open_is_a_guarded_finite_mutation() {
        let operation = Operation::from_parts(
            "plugin_editor_open",
            json!({"channel":"Scratch","slot":0,"plugin":"Phase Meter"}),
        )
        .unwrap();
        assert!(operation.validate().is_ok());
        assert!(!operation.is_read_only());
        assert_eq!(operation.wire().0, "pluginEditorOpen");
        for args in [
            json!({"channel":"Scratch","slot":0,"plugin":"  "}),
            json!({"channel":"Scratch","plugin":"Phase Meter"}),
            json!({"kind":"instrument","plugin":"Mojito"}),
        ] {
            let invalid = Operation::from_parts("pluginEditorOpen", args).unwrap();
            assert!(invalid.validate().is_err());
        }
        assert!(
            Operation::from_parts("pluginEditorOpen", json!({"channel":"Scratch","slot":0}))
                .is_err()
        );
        assert!(
            Operation::from_parts(
                "pluginEditorOpen",
                json!({"channel":"Scratch","slot":0,"plugin":"Phase Meter","method":"arbitrary"})
            )
            .is_err()
        );
    }

    #[test]
    fn standard_panner_operations_accept_only_the_five_native_fields() {
        let read = Operation::from_parts("panner_get", json!({"channel":"Scratch"})).unwrap();
        assert!(read.validate().is_ok());
        assert!(read.is_read_only());
        assert_eq!(read.wire().0, "panner");
        for param in ["pannerType", "stereoMode", "panX1", "panX2", "panWidth"] {
            let write = Operation::from_parts(
                "panner_set",
                json!({"channel":"Scratch","param":param,"value":0.5}),
            )
            .unwrap();
            assert!(write.validate().is_ok());
            assert!(!write.is_read_only());
            assert_eq!(write.wire().0, "setPanner");
            assert_eq!(write.wire().1["param"], param);
        }
        for args in [
            json!({"channel":"Scratch","param":"gain","value":0.5}),
            json!({"channel":"Scratch","param":"panWidth","value":"0.5"}),
            json!({"channel":"Scratch","param":"panWidth","value":0.5,"method":"arbitrary"}),
        ] {
            assert!(Operation::from_parts("panner_set", args).is_err());
        }
        let blank = Operation::from_parts("panner", json!({"channel":"  "})).unwrap();
        assert!(blank.validate().is_err());
        let nonfinite = Operation::SetPanner(PannerSet {
            channel: "Scratch".into(),
            param: PannerField::PanWidth,
            value: f64::NAN,
        });
        assert!(nonfinite.validate().is_err());
    }

    #[test]
    fn plugin_target_slots_match_the_native_bank_boundaries() {
        let cases = [
            (None, None, false),
            (None, Some(0), true),
            (None, Some(15), true),
            (None, Some(16), false),
            (Some("effect"), None, false),
            (Some("effect"), Some(15), true),
            (Some("effect"), Some(16), false),
            (Some("note_effect"), None, true),
            (Some("note_effect"), Some(15), true),
            (Some("note_effect"), Some(16), false),
            (Some("mix_fx"), None, true),
            (Some("mix_fx"), Some(0), true),
            (Some("mix_fx"), Some(1), false),
            (Some("instrument"), None, true),
            (Some("instrument"), Some(0), true),
            (Some("instrument"), Some(1), false),
        ];
        for name in ["pluginInspect", "pluginEditorOpen", "setPluginParam"] {
            for (kind, slot, valid) in cases {
                let mut args = json!({"channel":"Scratch","track":"Keys","kind":kind,"slot":slot,"plugin":"Expected class"});
                if name == "setPluginParam" {
                    args["param"] = json!("gain");
                    args["value"] = json!(0.5);
                }
                let operation = Operation::from_parts(name, args).unwrap();
                assert_eq!(
                    operation.validate().is_ok(),
                    valid,
                    "{name}, {kind:?}, {slot:?}"
                );
            }
        }
    }

    #[test]
    fn null_optional_arguments_do_not_reach_host() {
        let op = Operation::from_parts("setTransport", json!({"tempo":120})).unwrap();
        assert_eq!(op.wire().1, json!({"tempo":120.0}));
    }

    #[test]
    fn request_with_expected_document_deserializes() {
        let request: Request = serde_json::from_value(json!({
            "operation":"setChannel","args":{"channel":"a","field":"mute","value":true},
            "expected_document":"session:1"
        }))
        .unwrap();
        assert_eq!(request.expected_document.as_deref(), Some("session:1"));
    }

    #[test]
    fn public_time_and_loop_values_translate_to_the_host_contract() {
        let position =
            Operation::from_parts("setTransport", json!({"position":"9.1.1.0","loop":true}))
                .unwrap();
        assert_eq!(
            position.wire().1,
            json!({"positionBars":"9.1.1.0","loop":true})
        );
        let loop_set =
            Operation::from_parts("setLoop", json!({"start":0,"end":4,"enabled":false})).unwrap();
        assert_eq!(
            loop_set.wire().1,
            json!({"start":0.0,"end":4.0,"enable":false})
        );
    }
}

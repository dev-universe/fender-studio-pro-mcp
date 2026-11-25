use std::collections::{BTreeMap, BTreeSet};

use fender_routing_graph::{uid, unique_child};
use serde::Serialize;
use uuid::Uuid;

use super::{SavedPlugin, SavedTrack, XmlNode};

const INPUT: &str = "MusicInput00";

#[derive(Debug, Clone, Serialize)]
pub struct SavedMidiDestination {
    pub port: &'static str,
    pub kind: String,
    pub plugin_name: String,
    pub plugin_instance_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot: Option<u32>,
}

pub(super) fn attach(
    track_nodes: &XmlNode,
    device: &XmlNode,
    tracks: &mut [SavedTrack],
    plugins: &[SavedPlugin],
) {
    if device.name != "MusicTrackDevice" {
        return;
    }
    let Ok(Some(channels)) = unique_child(device, "channels") else {
        return;
    };
    let mut music_channels = BTreeMap::new();
    index_channels(channels, &mut music_channels);
    let mut instances = BTreeMap::new();
    for plugin in plugins {
        if let Some(id) = plugin.instance_id.as_deref().and_then(instance_id) {
            index_unique(&mut instances, id, plugin);
        }
    }
    let mut seen_tracks = BTreeSet::new();
    let mut duplicate_tracks = BTreeSet::new();
    for node in &track_nodes.children {
        if let Some(id) = named_uid(node, "channelID")
            && !seen_tracks.insert(id)
        {
            duplicate_tracks.insert(id);
        }
    }
    for (node, track) in track_nodes.children.iter().zip(tracks) {
        if node.name != "MediaTrack" || track.media_type.as_deref() != Some("Music") {
            continue;
        }
        let Some(id) = named_uid(node, "channelID").filter(|id| !duplicate_tracks.contains(id))
        else {
            continue;
        };
        let Some(channel) = music_channels.get(&id).copied().flatten() else {
            continue;
        };
        let Some(owner) = destination_id(channel) else {
            continue;
        };
        if let Some(plugin) = instances.get(&owner).copied().flatten() {
            track.midi_destination = resolved(plugin);
        }
    }
}

fn named_uid(node: &XmlNode, name: &str) -> Option<Uuid> {
    uid(unique_child(node, name).ok().flatten())
}

fn instance_id(raw: &str) -> Option<Uuid> {
    Uuid::parse_str(raw).ok().filter(|id| !id.is_nil())
}

fn destination_id(channel: &XmlNode) -> Option<Uuid> {
    let connection = unique_child(channel, "destination").ok().flatten()?;
    if connection.name != "Connection" {
        return None;
    }
    let (id, port) = connection.attr("objectID")?.split_once('/')?;
    if port != INPUT {
        return None;
    }
    instance_id(id.strip_prefix('{')?.strip_suffix('}')?)
}

fn index_channels<'a>(node: &'a XmlNode, channels: &mut BTreeMap<Uuid, Option<&'a XmlNode>>) {
    if node.name == "MusicTrackChannel"
        && let Some(id) = named_uid(node, "uniqueID")
    {
        index_unique(channels, id, node);
    }
    for child in &node.children {
        index_channels(child, channels);
    }
}

fn index_unique<T>(index: &mut BTreeMap<Uuid, Option<T>>, id: Uuid, entry: T) {
    index
        .entry(id)
        .and_modify(|row| *row = None)
        .or_insert(Some(entry));
}

fn resolved(plugin: &SavedPlugin) -> Option<SavedMidiDestination> {
    match (plugin.kind.as_str(), plugin.source_entry.as_str()) {
        ("instrument", "Devices/audiosynthfolder.xml") => {}
        ("insert" | "post_fader_insert", "Devices/audiomixer.xml")
            if plugin.channel.is_some() && plugin.slot.is_some() => {}
        _ => return None,
    }
    if plugin.name.is_empty() || plugin.class_id.as_deref().and_then(instance_id).is_none() {
        return None;
    }
    Some(SavedMidiDestination {
        port: INPUT,
        kind: plugin.kind.clone(),
        plugin_name: plugin.name.clone(),
        plugin_instance_id: plugin.instance_id.clone()?,
        channel: plugin.channel.clone(),
        slot: plugin.channel.as_ref().and(plugin.slot),
    })
}

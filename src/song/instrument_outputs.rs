use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use uuid::Uuid;

use super::{SavedChannel, SavedPlugin, XmlNode};

const IMPACT_CLASS: Uuid = Uuid::from_u128(0x3713e26c_2fca_4024_9f25_17e9d2be2b9b);
const OUTPUT_COUNT: usize = 32;

#[derive(Debug, Clone, Serialize)]
pub struct SavedInstrumentOutputs {
    pub ports: Vec<SavedOutputPort>,
    pub instrument_bus_channel_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SavedOutputPort {
    pub index: usize,
    pub label: String,
    pub speaker_layout: &'static str,
    pub enabled: bool,
    pub mixer_channel_id: Option<String>,
    pub mixer_channel_label: Option<String>,
}

pub(super) fn attach_impact_outputs(
    folder: &XmlNode,
    mixer: &[SavedChannel],
    plugins: &mut [SavedPlugin],
    warnings: &mut Vec<String>,
) {
    for plugin in plugins.iter_mut().filter(|plugin| {
        plugin.kind == "instrument"
            && plugin.source_entry == "Devices/audiosynthfolder.xml"
            && plugin
                .class_id
                .as_deref()
                .and_then(|id| Uuid::parse_str(id).ok())
                == Some(IMPACT_CLASS)
    }) {
        let outputs = (|| {
            let id = plugin.slot_id.as_deref().ok_or("missing slot ID")?;
            let mut slots = folder.children.iter().filter(|slot| {
                slot.child_id("uniqueID").and_then(|uid| uid.attr("uid")) == Some(id)
            });
            let slot = slots.next().ok_or("missing slot")?;
            if slots.next().is_some() {
                return Err("duplicate slot ID");
            }
            read_outputs(slot, mixer)
        })();
        match outputs {
            Ok(outputs) => plugin.instrument_outputs = Some(outputs),
            Err(reason) => {
                warnings.push(format!("Impact saved output graph is incomplete: {reason}"))
            }
        }
    }
}

fn read_outputs(
    slot: &XmlNode,
    mixer: &[SavedChannel],
) -> Result<SavedInstrumentOutputs, &'static str> {
    let outputs = slot
        .child_id("ghostData")
        .and_then(|node| node.child_id("IO"))
        .and_then(|node| node.child_id("audioOutputs"))
        .filter(|node| node.name == "List" && node.children.len() == OUTPUT_COUNT)
        .ok_or("unsupported output list")?;
    let mut members = mixer_members(slot, mixer)?;
    let ports = outputs
        .children
        .iter()
        .enumerate()
        .map(|(index, port)| {
            let label = port
                .attr("label")
                .filter(|label| !label.is_empty())
                .ok_or("missing output label")?;
            let speaker_layout = match (port.name.as_str(), port.attr("subType")) {
                ("PortDescription", Some("3")) => "stereo",
                ("PortDescription", Some("1")) => "mono",
                _ => return Err("unsupported output descriptor"),
            };
            let flags: u32 = port
                .attr("flags")
                .unwrap_or("0")
                .parse()
                .map_err(|_| "invalid output flags")?;
            let enabled = flags & 1 == 1;
            let member = members.remove(&index);
            if enabled != member.is_some() {
                return Err("saved output flags disagree with mixer membership");
            }
            Ok(SavedOutputPort {
                index,
                label: label.into(),
                speaker_layout,
                enabled,
                mixer_channel_id: member.and_then(|member| member.unique_id.clone()),
                mixer_channel_label: member.map(|member| member.label.clone()),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if !members.is_empty() {
        return Err("mixer membership uses an out-of-range output index");
    }
    if slot.child_id("synthBusChannelData").is_some() {
        return Err("unsupported inline instrument bus");
    }
    let instrument_bus_channel_id = slot
        .child_id("synthBusChannel")
        .map(|node| {
            node.attr("uid")
                .filter(|id| Uuid::parse_str(id).is_ok())
                .map(str::to_owned)
                .ok_or("invalid instrument bus reference")
        })
        .transpose()?;
    Ok(SavedInstrumentOutputs {
        ports,
        instrument_bus_channel_id,
    })
}

fn mixer_members<'a>(
    slot: &XmlNode,
    mixer: &'a [SavedChannel],
) -> Result<BTreeMap<usize, &'a SavedChannel>, &'static str> {
    let list = slot
        .child_id("synthChannels")
        .filter(|node| node.name == "List" && node.children.len() <= OUTPUT_COUNT)
        .ok_or("unsupported mixer membership")?;
    let mut members = BTreeMap::new();
    let mut ids = BTreeSet::new();
    for entry in &list.children {
        let id = entry
            .attr("uid")
            .filter(|id| Uuid::parse_str(id).is_ok())
            .ok_or("invalid mixer channel reference")?;
        if entry.name != "UID" || !ids.insert(id) {
            return Err("malformed or duplicate mixer channel reference");
        }
        let mut matches = mixer
            .iter()
            .filter(|channel| channel.unique_id.as_deref() == Some(id));
        let channel = matches
            .next()
            .filter(|channel| channel.kind == "AudioSynthChannel")
            .ok_or("missing instrument mixer channel")?;
        if matches.next().is_some() {
            return Err("duplicate mixer channel ID");
        }
        let index = channel
            .synth_port_index
            .ok_or("missing mixer channel output index")? as usize;
        if members.insert(index, channel).is_some() {
            return Err("multiple mixer channels refer to one output");
        }
    }
    Ok(members)
}

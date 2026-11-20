use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use uuid::Uuid;

use super::XmlNode;

const MAX_NODES: usize = 1_024;
const MAX_SUCCESSORS: usize = 32;

#[derive(Debug, Clone, Serialize)]
pub struct SavedInsertRouting {
    pub nodes: Vec<SavedRoutingNode>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SavedRoutingNode {
    pub id: Uuid,
    pub successors: Vec<Uuid>,
    #[serde(flatten)]
    pub kind: SavedRoutingNodeKind,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SavedRoutingNodeKind {
    Splitter {
        number: Option<u32>,
    },
    Merger,
    Slot {
        slot_index: usize,
        slot_id: Uuid,
        device_name: String,
    },
}

pub(super) fn read(channel: &XmlNode) -> Result<Option<SavedInsertRouting>, &'static str> {
    let Some(inserts) = unique_child(channel, "Inserts")? else {
        return Ok(None);
    };
    let Some(graph) = unique_child(inserts, "Combinator")? else {
        return Ok(None);
    };
    let records: Vec<_> = graph
        .children
        .iter()
        .filter(|child| child.attr("x:id") != Some("uniqueID"))
        .take(MAX_NODES + 1)
        .collect();
    if records.is_empty() {
        return Ok(None);
    }
    if inserts.name != "Attributes" || graph.name != "Attributes" || records.len() > MAX_NODES {
        return Err("unsupported graph container");
    }
    let slots = read_slots(inserts)?;
    let mut node_ids = BTreeSet::new();
    let nodes = records
        .into_iter()
        .map(|record| {
            let node = read_node(record, &slots)?;
            if !node_ids.insert(node.id) {
                return Err("duplicate routing node ID");
            }
            Ok(node)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if nodes.iter().any(|node| {
        node.successors
            .iter()
            .any(|id| *id == node.id || !node_ids.contains(id))
    }) {
        return Err("invalid routing successor reference");
    }
    Ok(Some(SavedInsertRouting { nodes }))
}

type SavedSlots<'a> = BTreeMap<Uuid, (usize, &'a XmlNode)>;

fn read_slots(inserts: &XmlNode) -> Result<SavedSlots<'_>, &'static str> {
    let mut slots = BTreeMap::new();
    for (index, slot) in inserts
        .children
        .iter()
        .filter(|child| child.name == "Attributes" && child.attr("x:id").is_none())
        .enumerate()
    {
        let id = uid(unique_child(slot, "uniqueID")?).ok_or("invalid insert slot ID")?;
        if index >= MAX_NODES || slots.insert(id, (index, slot)).is_some() {
            return Err("too many or duplicate insert slots");
        }
    }
    Ok(slots)
}

fn read_node(record: &XmlNode, slots: &SavedSlots<'_>) -> Result<SavedRoutingNode, &'static str> {
    let node = unique_child(record, "node")?
        .filter(|_| record.name == "Attributes" && record.attr("x:id").is_none())
        .ok_or("invalid routing node record")?;
    let id = uid(unique_child(node, "nodeID")?).ok_or("invalid routing node ID")?;
    let kind = match node.name.as_str() {
        "AudioFXGraph::SplitterNode" => SavedRoutingNodeKind::Splitter {
            number: node
                .attr("number")
                .map(str::parse)
                .transpose()
                .map_err(|_| "invalid Splitter number")?,
        },
        "AudioFXGraph::MergerNode" => SavedRoutingNodeKind::Merger,
        "ProcessGraph::SlotNode" => {
            let slot_id = uid(unique_child(node, "slotID")?).ok_or("invalid routing slot ID")?;
            let (index, slot) = slots
                .get(&slot_id)
                .ok_or("routing slot is absent from this rack")?;
            let slot_index = node
                .attr("slotIndex")
                .and_then(|value| value.parse::<usize>().ok())
                .filter(|stored| stored == index)
                .ok_or("routing slot index disagrees with rack order")?;
            let device_name = unique_child(slot, "deviceData")?
                .and_then(|device| device.attr("name"))
                .filter(|name| !name.is_empty())
                .ok_or("routing slot device has no saved name")?
                .to_owned();
            SavedRoutingNodeKind::Slot {
                slot_index,
                slot_id,
                device_name,
            }
        }
        _ => return Err("unsupported routing node kind"),
    };
    let successors = match unique_child(record, "successors")? {
        Some(list) if list.name == "List" && list.children.len() <= MAX_SUCCESSORS => list
            .children
            .iter()
            .map(|child| uid(Some(child)).ok_or("invalid routing successor ID"))
            .collect::<Result<_, _>>()?,
        Some(_) => return Err("invalid routing successor list"),
        None => Vec::new(),
    };
    Ok(SavedRoutingNode {
        id,
        successors,
        kind,
    })
}

fn unique_child<'a>(parent: &'a XmlNode, id: &str) -> Result<Option<&'a XmlNode>, &'static str> {
    let mut named = parent
        .children
        .iter()
        .filter(|child| child.attr("x:id") == Some(id));
    let first = named.next();
    if named.next().is_some() {
        return Err("duplicate routing metadata field");
    }
    Ok(first)
}

fn uid(node: Option<&XmlNode>) -> Option<Uuid> {
    let node = node.filter(|node| node.name == "UID")?;
    Uuid::parse_str(node.attr("uid")?)
        .ok()
        .filter(|id| !id.is_nil())
}

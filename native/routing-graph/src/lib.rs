mod xml;
pub use xml::{MAX_XML_DEPTH, XmlNode, parse_xml};

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use uuid::Uuid;

pub const MAX_NODES: usize = 1_024;
const MAX_SUCCESSORS: usize = 32;

#[derive(Debug, Clone, Serialize)]
pub struct RoutingGraph {
    pub nodes: Vec<RoutingNode>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoutingNode {
    pub id: Uuid,
    pub successors: Vec<Uuid>,
    #[serde(flatten)]
    pub kind: RoutingNodeKind,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RoutingNodeKind {
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

pub struct Slot<'a> {
    pub index: usize,
    pub name: Option<&'a str>,
}

pub fn read_nodes(
    records: Vec<&XmlNode>,
    slots: &BTreeMap<Uuid, Slot<'_>>,
) -> Result<RoutingGraph, &'static str> {
    if records.len() > MAX_NODES {
        return Err("too many routing nodes");
    }
    let mut node_ids = BTreeSet::new();
    let nodes = records
        .into_iter()
        .map(|record| {
            let node = read_node(record, slots)?;
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
    Ok(RoutingGraph { nodes })
}

fn read_node(
    record: &XmlNode,
    slots: &BTreeMap<Uuid, Slot<'_>>,
) -> Result<RoutingNode, &'static str> {
    let node = unique_child(record, "node")?
        .filter(|_| record.name == "Attributes" && record.attr("x:id").is_none())
        .ok_or("invalid routing node record")?;
    let id = uid(unique_child(node, "nodeID")?).ok_or("invalid routing node ID")?;
    let kind = match node.name.as_str() {
        "AudioFXGraph::SplitterNode" => RoutingNodeKind::Splitter {
            number: node
                .attr("number")
                .map(str::parse)
                .transpose()
                .map_err(|_| "invalid Splitter number")?,
        },
        "AudioFXGraph::MergerNode" => RoutingNodeKind::Merger,
        "ProcessGraph::SlotNode" => {
            let slot_id = uid(unique_child(node, "slotID")?).ok_or("invalid routing slot ID")?;
            let slot = slots
                .get(&slot_id)
                .ok_or("routing slot is absent from this rack")?;
            let slot_index = node
                .attr("slotIndex")
                .and_then(|value| value.parse::<usize>().ok())
                .filter(|stored| *stored == slot.index)
                .ok_or("routing slot index disagrees with rack order")?;
            let device_name = slot
                .name
                .ok_or("routing slot device has no saved name")?
                .to_owned();
            RoutingNodeKind::Slot {
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
    Ok(RoutingNode {
        id,
        successors,
        kind,
    })
}

pub fn unique_child<'a>(
    parent: &'a XmlNode,
    id: &str,
) -> Result<Option<&'a XmlNode>, &'static str> {
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

pub fn uid(node: Option<&XmlNode>) -> Option<Uuid> {
    let node = node.filter(|node| node.name == "UID")?;
    Uuid::parse_str(node.attr("uid")?)
        .ok()
        .filter(|id| !id.is_nil())
}

/// Reads the two routing metadata members of a host-created insert-chain archive.
pub fn read_collection(
    graph_xml: &[u8],
    parts_xml: &[u8],
    expected_part_count: usize,
) -> Result<RoutingGraph, String> {
    let graph = parse_xml(graph_xml)?;
    let parts = parse_xml(parts_xml)?;
    if graph.name != "FXGraph"
        || parts.name != "PresetParts"
        || parts.children.len() != expected_part_count
        || expected_part_count > MAX_NODES
    {
        return Err("invalid insert-chain routing metadata".into());
    }
    let mut slots = BTreeMap::new();
    for (index, part) in parts.children.iter().enumerate() {
        if part.name != "PresetPart" {
            return Err("invalid insert-chain part".into());
        }
        let attribute = |id| -> Result<Option<&str>, String> {
            let mut matches = part
                .children
                .iter()
                .filter(|child| child.name == "Attribute" && child.attr("id") == Some(id));
            let first = matches.next().and_then(|child| child.attr("value"));
            if matches.next().is_some() {
                return Err("duplicate insert-chain part metadata".into());
            }
            Ok(first)
        };
        let slot_id = attribute("DeviceSlot:slotUID")?
            .and_then(|text| Uuid::parse_str(text).ok())
            .filter(|id| !id.is_nil())
            .ok_or("invalid insert-chain part slot ID")?;
        let name = attribute("Class:Name")?.filter(|name| {
            !name.is_empty() && name.len() <= 512 && !name.chars().any(char::is_control)
        });
        if slots.insert(slot_id, Slot { index, name }).is_some() {
            return Err("duplicate insert-chain part slot ID".into());
        }
    }
    let records = graph.children.iter().collect();
    read_nodes(records, &slots).map_err(str::to_owned)
}

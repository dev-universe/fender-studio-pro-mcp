use std::collections::BTreeMap;

use uuid::Uuid;

use fender_routing_graph::{MAX_NODES, Slot, XmlNode, read_nodes, uid, unique_child};

pub use fender_routing_graph::RoutingGraph as SavedInsertRouting;

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
    read_nodes(records, &slots).map(Some)
}

fn read_slots(inserts: &XmlNode) -> Result<BTreeMap<Uuid, Slot<'_>>, &'static str> {
    let mut slots = BTreeMap::new();
    for (index, slot) in inserts
        .children
        .iter()
        .filter(|child| child.name == "Attributes" && child.attr("x:id").is_none())
        .enumerate()
    {
        let id = uid(unique_child(slot, "uniqueID")?).ok_or("invalid insert slot ID")?;
        let name = unique_child(slot, "deviceData")?
            .and_then(|device| device.attr("name"))
            .filter(|name| !name.is_empty());
        if index >= MAX_NODES || slots.insert(id, Slot { index, name }).is_some() {
            return Err("too many or duplicate insert slots");
        }
    }
    Ok(slots)
}

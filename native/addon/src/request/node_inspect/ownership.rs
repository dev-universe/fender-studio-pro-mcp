use super::*;
use fender_native_node_api::{Delegate, NodeRecord};

#[derive(Clone, Debug, Serialize)]
struct CopiedIdentity {
    id: String,
    native_class: String,
    object_uid: Option<String>,
    class_uid: Option<String>,
}
impl CopiedIdentity {
    fn read(record: &NodeRecord, object_uid_required: bool) -> Result<Self, String> {
        let identity = record
            .identity
            .as_ref()
            .ok_or("graph node identity unavailable")?;
        if object_uid_required && !identity.object_uid.as_deref().is_some_and(valid_uid) {
            return Err("graph node object UID unavailable".into());
        }
        if !record.reflection.available
            || record.reflection.types.len() != 1
            || record.reflection.issues != [MetadataIssue::MembersUnobserved]
        {
            return Err("focused graph class metadata unavailable".into());
        }
        let native_class = &record.reflection.types[0].name;
        if native_class.is_empty() {
            return Err("graph node native class unavailable".into());
        }
        Ok(Self {
            id: identity.id.clone(),
            native_class: native_class.clone(),
            object_uid: identity.object_uid.clone(),
            class_uid: identity.class_uid.clone(),
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum GraphChild {
    Splitter {
        child_index: u32,
        node: CopiedIdentity,
    },
    Merger {
        child_index: u32,
        node: CopiedIdentity,
    },
    Slot {
        child_index: u32,
        node: CopiedIdentity,
        slot: CopiedIdentity,
        device: CopiedIdentity,
    },
}

#[derive(Debug, Serialize)]
struct GraphOwnershipValue {
    folder: folder_context::FolderIdentity,
    graph: CopiedIdentity,
    children: Vec<GraphChild>,
}
impl GraphOwnershipValue {
    fn read(report: &Report, folder: folder_context::FolderIdentity) -> Result<Self, String> {
        if !report.complete || !report.diagnostics.is_empty() {
            return Err("focused graph ownership incomplete".into());
        }
        let root = report.nodes.first().ok_or("graph root unavailable")?;
        let graph = CopiedIdentity::read(root, true)?;
        if root.index != 0 || graph.id != "Graph" || graph.native_class != "AudioFXGraph" {
            return Err("graph root family mismatch".into());
        }
        let count = root.child_count.ok_or("graph child count unavailable")?;
        if count > 32 {
            return Err("graph child limit exceeded".into());
        }
        let relation = |from, kind| -> Result<Option<&NodeRecord>, String> {
            let mut edges = report
                .edges
                .iter()
                .filter(|edge| edge.from == from && edge.relation == kind);
            let node = edges
                .next()
                .map(|edge| {
                    report
                        .nodes
                        .get(edge.to)
                        .ok_or("graph mapping index unavailable")
                })
                .transpose()?;
            if edges.next().is_some() {
                return Err("ambiguous graph mapping".into());
            }
            Ok(node)
        };
        let mut children = Vec::new();
        for child_index in 0..count {
            let row =
                relation(0, Relation::Child(child_index))?.ok_or("graph child unavailable")?;
            let node = CopiedIdentity::read(row, true)?;
            let slot_row = relation(row.index, Relation::Delegate(Delegate::Slot))?;
            children.push(match node.native_class.as_str() {
                "AudioFXGraph::SplitterNode" | "AudioFXGraph::MergerNode" => {
                    if slot_row.is_some() {
                        return Err("unexpected Slot on graph junction".into());
                    }
                    if node.native_class == "AudioFXGraph::SplitterNode" {
                        GraphChild::Splitter { child_index, node }
                    } else {
                        GraphChild::Merger { child_index, node }
                    }
                }
                "ProcessGraph::SlotNode" => {
                    let slot_row = slot_row.ok_or("graph Slot delegate unavailable")?;
                    let slot = CopiedIdentity::read(slot_row, false)?;
                    if slot.native_class != "AudioInsertSlot" {
                        return Err("graph Slot family mismatch".into());
                    }
                    let device_row =
                        relation(slot_row.index, Relation::Delegate(Delegate::Device))?
                            .ok_or("graph Device delegate unavailable")?;
                    let device = CopiedIdentity::read(device_row, true)?;
                    if !device.class_uid.as_deref().is_some_and(valid_uid) {
                        return Err("graph Device class UID unavailable".into());
                    }
                    GraphChild::Slot {
                        child_index,
                        node,
                        slot,
                        device,
                    }
                }
                _ => return Err("unrecognized graph child family".into()),
            });
        }
        Ok(Self {
            folder,
            graph,
            children,
        })
    }
    fn value(&self) -> Value {
        let mut value = serde_json::to_value(self).expect("owned graph metadata serialization");
        let object = value.as_object_mut().expect("graph object");
        object.extend([
            ("schema".into(), json!(1)),
            (
                "scope".into(),
                json!("direct_graph_children_and_slot_devices"),
            ),
            (
                "metadata_scope".into(),
                json!("identity_and_first_native_class_header_only"),
            ),
            ("inherited_types_and_members_observed".into(), json!(false)),
            ("child_count".into(), json!(self.children.len())),
            ("child_order".into(), json!("CCL_child_enumeration")),
            ("complete".into(), json!(true)),
            ("atomic_snapshot".into(), json!(false)),
            ("audio_connections_enumerated".into(), json!(false)),
            ("mutation_capability".into(), json!(false)),
        ]);
        value
    }
}

pub(in super::super) struct GraphOwnership {
    focused: Inspection,
    folder: folder_context::FolderContext<'static>,
    value: GraphOwnershipValue,
}
impl GraphOwnership {
    pub(super) fn capture(
        ctx: &Context<'static>,
        route: &GraphRoute,
        text: TextApi,
    ) -> Result<Self, String> {
        let folder_owner = hold(route.folder.node.0).ok_or("graph Inserts unavailable")?;
        let folder =
            folder_context::FolderContext::capture(ctx.service, folder_owner, None, ctx.api)?;
        ctx.check()?;
        if !folder.binding_key().current_document(ctx) {
            return Err("graph folder document binding mismatch".into());
        }
        route.check()?;
        let root = hold(route.graph.node.0).ok_or("graph root unavailable")?;
        let pointer = NonNull::new(root.0).expect("held graph");
        std::mem::forget(root);
        let inspector = unsafe { NodeInspector::from_owned(pointer, text) };
        let result = inspector.inspect_children_with_slots(&mut || {
            folder.check()?;
            route.check()?;
            ctx.check()
        });
        drop(inspector);
        if let Err(error) = ctx.check() {
            drop(result);
            ctx.check()?;
            return Err(error);
        }
        let focused = result.map_err(|error| error.to_string())?;
        let value = GraphOwnershipValue::read(focused.report(), folder.identity().clone())?;
        let ownership = Self {
            focused,
            folder,
            value,
        };
        ownership.check(
            &mut || {
                route.check()?;
                ctx.check()
            },
            &mut || {
                route.check()?;
                ctx.check()
            },
        )?;
        Ok(ownership)
    }
    pub(super) fn check<G, B>(&self, guard: &mut G, borrowed_guard: &mut B) -> Result<(), String>
    where
        G: FnMut() -> Result<(), String>,
        B: FnMut() -> Result<(), String>,
    {
        self.folder.check()?;
        self.focused
            .check(&mut || {
                self.folder.check()?;
                guard()
            })
            .map_err(|error| error.to_string())?;
        self.folder.check()?;
        guard()?;
        self.focused
            .check_borrowed_attachments(&mut || {
                self.folder.check_borrowed_attachments()?;
                borrowed_guard()
            })
            .map_err(|error| error.to_string())?;
        self.folder.check_borrowed_attachments()?;
        borrowed_guard()
    }
    pub(super) fn value(&self) -> Value {
        self.value.value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fender_native_node_api::{Edge, NodeIdentity, TypeInfo, TypeInventory};
    const UID: &str = "12345678-1234-1234-1212-121212121212";
    fn row(index: usize, id: &str, native_class: &str, count: u32) -> NodeRecord {
        NodeRecord {
            index,
            identity: Some(NodeIdentity {
                id: id.into(),
                object_uid: Some(UID.into()),
                class_uid: None,
            }),
            child_count: Some(count),
            reflection: TypeInventory {
                available: true,
                types: vec![TypeInfo {
                    name: native_class.into(),
                    namespace: None,
                    class_uid: None,
                    flags: 0,
                    methods: vec![],
                    properties: vec![],
                }],
                issues: vec![MetadataIssue::MembersUnobserved],
            },
            declared_interfaces: None,
        }
    }
    fn folder() -> folder_context::FolderIdentity {
        folder_context::FolderIdentity {
            id: "Inserts".into(),
            native_class: "AudioInsertFolder".into(),
            object_uid: UID.into(),
            class_uid: None,
        }
    }
    fn report() -> Report {
        let mut report = Report {
            nodes: vec![
                row(0, "Graph", "AudioFXGraph", 3),
                row(1, "Split", "AudioFXGraph::SplitterNode", 0),
                row(2, "Merge", "AudioFXGraph::MergerNode", 0),
                row(3, "Node05", "ProcessGraph::SlotNode", 0),
                row(4, "FX03", "AudioInsertSlot", 0),
                row(5, "Mixtool", "AudioEffect", 0),
            ],
            edges: vec![
                Edge {
                    from: 0,
                    to: 1,
                    relation: Relation::Child(0),
                },
                Edge {
                    from: 0,
                    to: 2,
                    relation: Relation::Child(1),
                },
                Edge {
                    from: 0,
                    to: 3,
                    relation: Relation::Child(2),
                },
                Edge {
                    from: 3,
                    to: 4,
                    relation: Relation::Delegate(Delegate::Slot),
                },
                Edge {
                    from: 4,
                    to: 5,
                    relation: Relation::Delegate(Delegate::Device),
                },
            ],
            diagnostics: vec![],
            complete: true,
        };
        report.nodes[4].identity.as_mut().unwrap().object_uid = None;
        report.nodes[5].identity.as_mut().unwrap().class_uid = Some(UID.into());
        report
    }
    #[test]
    fn graph_manifest_keeps_all_families_and_explicit_scope_with_nullable_slot_uid() {
        let value = GraphOwnershipValue::read(&report(), folder())
            .unwrap()
            .value();
        assert_eq!(value["child_count"], 3);
        assert_eq!(value["children"][0]["kind"], "splitter");
        assert_eq!(value["children"][1]["kind"], "merger");
        assert_eq!(value["children"][2]["kind"], "slot");
        assert_eq!(value["children"][2]["slot"]["object_uid"], Value::Null);
        assert_eq!(value["children"][2]["device"]["id"], "Mixtool");
        assert_eq!(value["inherited_types_and_members_observed"], false);
        assert_eq!(value["audio_connections_enumerated"], false);
        assert_eq!(value["mutation_capability"], false);
        assert_eq!(value["atomic_snapshot"], false);
        assert_eq!(value["complete"], true);
        let legacy = super::super::report(
            &Report {
                complete: false,
                ..report()
            },
            Scope::Graph,
        );
        assert_eq!(legacy["complete"], false);
    }
    #[test]
    fn graph_manifest_maps_all_32_slot_nodes_slots_and_devices() {
        let mut r = Report {
            nodes: vec![row(0, "Graph", "AudioFXGraph", 32)],
            edges: vec![],
            diagnostics: vec![],
            complete: true,
        };
        for child_index in 0..32 {
            let node = r.nodes.len();
            r.nodes.push(row(node, "Node", "ProcessGraph::SlotNode", 0));
            r.nodes.push(row(node + 1, "Slot", "AudioInsertSlot", 0));
            r.nodes.push(row(node + 2, "Device", "AudioEffect", 0));
            r.nodes[node + 1].identity.as_mut().unwrap().object_uid = None;
            r.nodes[node + 2].identity.as_mut().unwrap().class_uid = Some(UID.into());
            r.edges.extend([
                Edge {
                    from: 0,
                    to: node,
                    relation: Relation::Child(child_index),
                },
                Edge {
                    from: node,
                    to: node + 1,
                    relation: Relation::Delegate(Delegate::Slot),
                },
                Edge {
                    from: node + 1,
                    to: node + 2,
                    relation: Relation::Delegate(Delegate::Device),
                },
            ]);
        }
        assert_eq!(r.nodes.len(), 97);
        let value = GraphOwnershipValue::read(&r, folder()).unwrap().value();
        assert_eq!(value["child_count"], 32);
        for (index, child) in value["children"].as_array().unwrap().iter().enumerate() {
            assert_eq!(child["kind"], "slot");
            assert_eq!(child["child_index"], index);
            assert_eq!(child["slot"]["object_uid"], Value::Null);
            assert_eq!(child["device"]["class_uid"], UID);
        }
    }

    #[test]
    fn graph_manifest_refuses_missing_required_uid_family_or_mapping() {
        for index in [0, 1, 2, 3, 5] {
            let mut r = report();
            r.nodes[index].identity.as_mut().unwrap().object_uid = None;
            assert!(GraphOwnershipValue::read(&r, folder()).is_err());
        }
        for index in [0, 1, 2, 3, 4] {
            let mut r = report();
            r.nodes[index].reflection.types[0].name = "UnknownFamily".into();
            assert!(GraphOwnershipValue::read(&r, folder()).is_err());
        }
        let mut r = report();
        r.nodes[5].identity.as_mut().unwrap().class_uid = None;
        assert!(GraphOwnershipValue::read(&r, folder()).is_err());
        let mut r = report();
        r.edges.pop();
        assert!(GraphOwnershipValue::read(&r, folder()).is_err());
        let mut r = report();
        r.edges.push(Edge {
            from: 1,
            to: 4,
            relation: Relation::Delegate(Delegate::Slot),
        });
        assert!(GraphOwnershipValue::read(&r, folder()).is_err());
    }
    #[test]
    fn retained_folder_switch_invalidates_graph_ownership_and_reply_removes_whole_report() {
        let _serial = super::super::super::super::tests::TEST_LOCK.lock().unwrap();
        let folder_fixture = folder_context::tests::Fixture::new();
        let folder = folder_fixture.capture().unwrap();
        let mut node_fixture = super::super::tests::Fixture::new();
        let route = node_fixture.route(&mut || Ok(())).unwrap();
        let ownership = GraphOwnership {
            focused: node_fixture.ancestors(),
            value: GraphOwnershipValue::read(&report(), folder.identity().clone()).unwrap(),
            folder,
        };
        ownership.folder.check().unwrap();
        folder_fixture.switch_document();
        assert!(ownership.check(&mut || Ok(()), &mut || Ok(())).is_err());
        let inventory = Inventory::Graph {
            inspection: node_fixture.ancestors(),
            route,
            ownership,
        };
        let mut reply =
            Reply::unbound(json!({"ok":true,"outcome":"observed","node_report":inventory.value()}));
        reply.nodes = Some(inventory);
        reply.invalidate("document switched before receipt".into());
        assert_eq!(reply.value["ok"], false);
        assert_eq!(reply.value["outcome"], "not_applied");
        assert!(reply.value.get("node_report").is_none());
        drop(reply);
        node_fixture.released();
    }
    #[test]
    fn reply_graph_final_witness_has_no_late_provider_owner_cleanup() {
        let _serial = super::super::super::super::tests::TEST_LOCK.lock().unwrap();
        let mut fixture = folder_context::tests::Fixture::new();
        let folder = fixture.capture().unwrap();
        let (focused, junction) = fixture.focused_graph();
        let value = GraphOwnershipValue::read(focused.report(), folder.identity().clone()).unwrap();
        let mut route_fixture = super::super::tests::Fixture::new();
        let inventory = Inventory::Graph {
            inspection: route_fixture.ancestors(),
            route: route_fixture.route(&mut || Ok(())).unwrap(),
            ownership: GraphOwnership {
                focused,
                folder,
                value,
            },
        };
        let context = fixture.graph_context();
        let mut reply =
            Reply::unbound(json!({"ok":true,"outcome":"observed","node_report":inventory.value()}));
        reply.provider = Some(Provider::Controller(Controller {
            handle: hold(context.target.0).unwrap(),
            lease: None,
        }));
        reply.context = Some(context);
        reply.nodes = Some(inventory);
        fixture.reset_slot_reads();
        assert!(reply.final_check(fixture.service()));
        let final_slot_read = fixture.slot_reads();
        assert!(final_slot_read > 0);
        fixture.reset_slot_reads();
        fixture.cleanup_adds_slot_after(junction, final_slot_read);
        assert!(reply.final_check(fixture.service()));
        assert!(
            fixture.slot_absent(junction),
            "provider cleanup mutated Slot after its final witness"
        );
        assert_eq!(
            reply.value["node_report"]["ownership_manifest"]["complete"],
            true
        );
        reply.context.as_ref().unwrap().check().unwrap();
        drop(reply);
        route_fixture.released();
    }
    #[test]
    fn reply_graph_rejects_provider_cleanup_slot_change_before_final_witness() {
        let _serial = super::super::super::super::tests::TEST_LOCK.lock().unwrap();
        let mut fixture = folder_context::tests::Fixture::new();
        let folder = fixture.capture().unwrap();
        let (focused, junction) = fixture.focused_graph();
        let value = GraphOwnershipValue::read(focused.report(), folder.identity().clone()).unwrap();
        let mut route_fixture = super::super::tests::Fixture::new();
        let inventory = Inventory::Graph {
            inspection: route_fixture.ancestors(),
            route: route_fixture.route(&mut || Ok(())).unwrap(),
            ownership: GraphOwnership {
                focused,
                folder,
                value,
            },
        };
        let context = fixture.graph_context();
        let mut reply =
            Reply::unbound(json!({"ok":true,"outcome":"observed","node_report":inventory.value()}));
        reply.provider = Some(Provider::Controller(Controller {
            handle: hold(context.target.0).unwrap(),
            lease: None,
        }));
        reply.context = Some(context);
        reply.nodes = Some(inventory);
        fixture.reset_slot_reads();
        fixture.cleanup_adds_slot_after(junction, 1);
        assert!(!reply.final_check(fixture.service()));
        assert!(!fixture.slot_absent(junction));
        assert_eq!(reply.value["ok"], false);
        assert_eq!(reply.value["outcome"], "not_applied");
        assert!(reply.value.get("node_report").is_none());
        reply.context.as_ref().unwrap().check().unwrap();
        drop(reply);
        route_fixture.released();
    }
    #[test]
    fn graph_ownership_rejects_provider_detached_by_final_folder_cleanup() {
        let _serial = super::super::super::super::tests::TEST_LOCK.lock().unwrap();
        let provider_valid = std::cell::Cell::new(true);
        let mut fixture = folder_context::tests::Fixture::new();
        let folder = fixture.capture().unwrap();
        let (focused, _) = fixture.focused_graph();
        let value = GraphOwnershipValue::read(focused.report(), folder.identity().clone()).unwrap();
        let ownership = GraphOwnership {
            focused,
            folder,
            value,
        };
        let mut focused_guards = 0;
        ownership
            .focused
            .check(&mut || {
                focused_guards += 1;
                Ok(())
            })
            .unwrap();
        let context = fixture.graph_context();
        let mut guards = 0;
        let result = ownership.check(
            &mut || {
                guards += 1;
                if guards == focused_guards {
                    fixture.channel_cleanup_detaches_provider(&provider_valid);
                }
                if provider_valid.get() {
                    Ok(())
                } else {
                    Err("provider attachment changed".into())
                }
            },
            &mut || context.check(),
        );
        assert!(
            !provider_valid.get(),
            "native folder cleanup did not detach provider"
        );
        assert_eq!(result, Err("provider attachment changed".into()));
        context.check().unwrap();
    }
}

use super::*;
use fender_native_node_api::{
    Diagnostic, Inspection, Limits, MetadataIssue, NodeInspector, Relation, Report, TextApi,
};

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum Scope {
    #[default]
    Target,
    Graph,
}

fn limits(scope: Scope) -> Limits {
    Limits {
        nodes: 48,
        depth: 3,
        ancestors: if scope == Scope::Graph { 0 } else { 3 },
        children_per_node: 32,
    }
}

struct NodePin {
    node: Owned,
    _canonical: Owned,
}
impl NodePin {
    fn borrowed(pointer: *mut c_void) -> Result<Self, String> {
        let node = hold(pointer).ok_or("native graph route node unavailable")?;
        let canonical = query(node.0, &UNKNOWN).ok_or("native graph route identity unavailable")?;
        Ok(Self {
            node,
            _canonical: canonical,
        })
    }
    fn parent(&self) -> Result<Self, String> {
        let get: Borrowed = unsafe { method(self.node.0, 6) };
        Self::borrowed(unsafe { get(self.node.0) })
    }
    fn child(&self, name: *const PlainString) -> Result<Self, String> {
        let get: unsafe extern "C" fn(*mut c_void, *const PlainString) -> *mut c_void =
            unsafe { method(self.node.0, 10) };
        Self::borrowed(unsafe { get(self.node.0, name) })
    }
    fn id(&self) -> Result<String, String> {
        let get: unsafe extern "C" fn(*mut c_void) -> *const PlainString =
            unsafe { method(self.node.0, 3) };
        let id = unsafe { get(self.node.0) };
        if id.is_null() {
            return Err("native graph route identifier unavailable".into());
        }
        let owner = hold(unsafe { (*id).object }).ok_or("native graph route string unavailable")?;
        copy_string(owner.0, 512)
    }
}

struct GraphRoute {
    graph: NodePin,
    combinator: NodePin,
    folder: NodePin,
    slot: NodePin,
    device: NodePin,
    combinator_name: *const PlainString,
    graph_name: *const PlainString,
}
impl GraphRoute {
    fn resolve<G: FnMut() -> Result<(), String>>(
        device: NodePin,
        slot: NodePin,
        folder: NodePin,
        combinator_name: *const PlainString,
        graph_name: *const PlainString,
        guard: &mut G,
    ) -> Result<Self, String> {
        guard()?;
        let combinator = folder.child(combinator_name)?;
        guard()?;
        let graph = combinator.child(graph_name)?;
        guard()?;
        let route = Self {
            graph,
            combinator,
            folder,
            slot,
            device,
            combinator_name,
            graph_name,
        };
        route.check()?;
        guard()?;
        Ok(route)
    }
    fn capture(ctx: &Context<'_>) -> Result<Self, String> {
        let get_string = constant_string().ok_or("native string constants unavailable")?;
        let combinator_name = unsafe { get_string(c"Combinator".as_ptr()) };
        let graph_name = unsafe { get_string(c"Graph".as_ptr()) };
        if combinator_name.is_null() || graph_name.is_null() {
            return Err("native graph route constants unavailable".into());
        }
        let device = NodePin::borrowed(ctx.node.0)?;
        ctx.check()?;
        let slot = device.parent()?;
        ctx.check()?;
        let folder = slot.parent()?;
        ctx.check()?;
        let slot_class = class_name(slot.node.0);
        let folder_class = class_name(folder.node.0);
        let folder_id = folder.id()?;
        ctx.check()?;
        validate_insert_family(&slot_class, &folder_class, &folder_id)?;
        let route = Self::resolve(
            device,
            slot,
            folder,
            combinator_name,
            graph_name,
            &mut || ctx.check(),
        )?;
        let combinator_id = route.combinator.id()?;
        let graph_id = route.graph.id()?;
        route.check()?;
        ctx.check()?;
        if combinator_id != "Combinator" || graph_id != "Graph" {
            return Err("native graph route identifiers do not match Combinator/Graph".into());
        }
        Ok(route)
    }
    fn check(&self) -> Result<(), String> {
        let parent: Borrowed = unsafe { method(self.device.node.0, 6) };
        let slot_parent: Borrowed = unsafe { method(self.slot.node.0, 6) };
        let find: unsafe extern "C" fn(*mut c_void, *const PlainString) -> *mut c_void =
            unsafe { method(self.folder.node.0, 10) };
        let graph_find: unsafe extern "C" fn(*mut c_void, *const PlainString) -> *mut c_void =
            unsafe { method(self.combinator.node.0, 10) };
        if unsafe { parent(self.device.node.0) } != self.slot.node.0
            || unsafe { slot_parent(self.slot.node.0) } != self.folder.node.0
            || unsafe { find(self.folder.node.0, self.combinator_name) } != self.combinator.node.0
            || unsafe { graph_find(self.combinator.node.0, self.graph_name) } != self.graph.node.0
        {
            Err("native graph route attachment changed".into())
        } else {
            Ok(())
        }
    }
}

fn validate_insert_family(slot: &str, folder: &str, folder_id: &str) -> Result<(), String> {
    if slot == "AudioInsertSlot" && folder == "AudioInsertFolder" && folder_id == "Inserts" {
        Ok(())
    } else {
        Err(
            "graph scope requires the verified AudioInsertSlot/Inserts/Combinator/Graph family"
                .into(),
        )
    }
}

pub(super) struct Inventory {
    inspection: Inspection,
    route: Option<GraphRoute>,
    scope: Scope,
}

/// Read-only metadata of a bridge-created object. This never authorizes a
/// subsequent mutation and makes no claim that the auxiliary owns the Device.
pub(super) struct Auxiliary {
    inspection: Inspection,
}
impl Auxiliary {
    pub(super) fn check(&self, ctx: &Context<'_>) -> Result<(), String> {
        self.inspection
            .check(&mut || ctx.check())
            .map_err(|e| e.to_string())?;
        ctx.check()
    }
    pub(super) fn value(&self) -> Value {
        let mut value = report(self.inspection.report(), Scope::Target);
        value["coverage"] = json!("bounded_readonly_auxiliary_type_metadata");
        value["scope"] = json!("auxiliary_type_observation");
        value["mutation_capability"] = json!(false);
        value["limits"] = json!({"nodes":1,"depth":0,"ancestors":0,"children_per_node":1});
        value
    }
}
pub(super) fn capture_auxiliary(ctx: &Context<'_>, auxiliary: Owned) -> Result<Auxiliary, String> {
    ctx.check()?;
    let get = constant_string().ok_or("native string constants unavailable")?;
    let text = TextApi {
        get_constant_string: unsafe {
            std::mem::transmute::<
                ConstantString,
                unsafe extern "C" fn(*const c_char) -> *const c_void,
            >(get)
        },
    };
    let pointer = NonNull::new(auxiliary.0).expect("held auxiliary");
    std::mem::forget(auxiliary);
    let inspector = unsafe { NodeInspector::from_owned(pointer, text) };
    let result = inspector.inspect(
        Limits {
            nodes: 1,
            depth: 0,
            ancestors: 0,
            children_per_node: 1,
        },
        &mut || ctx.check(),
    );
    drop(inspector);
    if let Err(reason) = ctx.check() {
        drop(result);
        ctx.check()?;
        return Err(reason);
    }
    let auxiliary = Auxiliary {
        inspection: result.map_err(|e| e.to_string())?,
    };
    auxiliary.check(ctx)?;
    Ok(auxiliary)
}
impl Inventory {
    pub(super) fn report(&self) -> &Report {
        self.inspection.report()
    }
    pub(super) fn value(&self) -> Value {
        report(self.report(), self.scope)
    }
    pub(super) fn check<G: FnMut() -> Result<(), String>>(
        &self,
        guard: &mut G,
    ) -> Result<(), String> {
        let mut checked = || {
            if let Some(route) = &self.route {
                route.check()?;
            }
            guard()
        };
        checked()?;
        self.inspection
            .check(&mut checked)
            .map_err(|error| error.to_string())?;
        checked()
    }
    #[cfg(test)]
    pub(super) fn target(inspection: Inspection) -> Self {
        Self {
            inspection,
            route: None,
            scope: Scope::Target,
        }
    }
    #[cfg(test)]
    pub(super) fn into_auxiliary(self) -> Auxiliary {
        Auxiliary {
            inspection: self.inspection,
        }
    }
}

pub(super) fn capture(ctx: &Context<'_>, scope: Scope) -> Result<Inventory, String> {
    ctx.check()?;
    let get_string = constant_string().ok_or("native string constants unavailable")?;
    let text = TextApi {
        get_constant_string: unsafe {
            std::mem::transmute::<
                ConstantString,
                unsafe extern "C" fn(*const c_char) -> *const c_void,
            >(get_string)
        },
    };
    let route = if scope == Scope::Graph {
        Some(GraphRoute::capture(ctx)?)
    } else {
        None
    };
    let root = hold(
        route
            .as_ref()
            .map_or(ctx.target.0, |route| route.graph.node.0),
    )
    .ok_or("native target unavailable")?;
    let pointer = NonNull::new(root.0).expect("held target");
    std::mem::forget(root);
    let inspector = unsafe { NodeInspector::from_owned(pointer, text) };
    let result = inspector.inspect(limits(scope), &mut || {
        if let Some(route) = &route {
            route.check()?;
        }
        ctx.check()
    });
    drop(inspector);
    if let Err(reason) = ctx.check() {
        drop(result);
        ctx.check()?;
        return Err(reason);
    }
    let inventory = Inventory {
        inspection: result.map_err(|error| error.to_string())?,
        route,
        scope,
    };
    inventory.check(&mut || ctx.check())?;
    Ok(inventory)
}

fn issue(issue: &MetadataIssue) -> &'static str {
    match issue {
        MetadataIssue::NoObjectInterface => "no_object_interface",
        MetadataIssue::TypeCycle => "type_cycle",
        MetadataIssue::TypeDepth => "type_depth_limit",
        MetadataIssue::TypeLimit => "type_limit",
        MetadataIssue::MethodLimit => "method_limit",
        MetadataIssue::PropertyLimit => "property_limit",
    }
}

pub(super) fn report(report: &Report, scope: Scope) -> Value {
    let limits = limits(scope);
    json!({
        "coverage":"bounded_native_nodes_and_reflection",
        "scope":if scope == Scope::Graph { "graph" } else { "target" },
        "complete":report.complete,
        "limits":{"nodes":limits.nodes,"depth":limits.depth,"ancestors":limits.ancestors,"children_per_node":limits.children_per_node},
        "nodes":report.nodes.iter().map(|node| json!({
            "index":node.index,
            "identity":node.identity.as_ref().map(|identity|json!({"id":identity.id,"object_uid":identity.object_uid,"class_uid":identity.class_uid})),
            "child_count":node.child_count,
            "reflection":{
                "available":node.reflection.available,
                "types":node.reflection.types.iter().map(|info|json!({
                    "name":info.name,"namespace":info.namespace,"class_uid":info.class_uid,"flags":info.flags,
                    "methods":info.methods.iter().map(|method|json!({"name":method.name,"arguments":method.arguments,"return_type":method.return_type})).collect::<Vec<_>>(),
                    "properties":info.properties.iter().map(|property|json!({"name":property.name,"data_type":property.data_type,"type_name":property.type_name,"has_type_info":property.has_type_info})).collect::<Vec<_>>()
                })).collect::<Vec<_>>(),
                "issues":node.reflection.issues.iter().map(issue).collect::<Vec<_>>()
            }
        })).collect::<Vec<_>>(),
        "edges":report.edges.iter().map(|edge|json!({
            "from":edge.from,"to":edge.to,"relation":match edge.relation {
                Relation::Parent=>json!({"kind":"parent"}),
                Relation::Child(index)=>json!({"kind":"child","index":index}),
                Relation::Delegate(delegate)=>json!({"kind":"delegate","name":delegate.name()})
            }
        })).collect::<Vec<_>>(),
        "diagnostics":report.diagnostics.iter().map(|diagnostic|match diagnostic {
            Diagnostic::NodeLimit=>json!({"kind":"node_limit"}),
            Diagnostic::DepthLimit(node)=>json!({"kind":"depth_limit","node":node}),
            Diagnostic::ChildLimit(node)=>json!({"kind":"child_limit","node":node}),
            Diagnostic::AncestorLimit=>json!({"kind":"ancestor_limit"}),
            Diagnostic::NodeCycle{from,to}=>json!({"kind":"node_cycle","from":from,"to":to}),
        }).collect::<Vec<_>>()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use fender_native_node_api::{
        Delegate, Edge, Method, NodeIdentity, NodeRecord, Property, TypeInfo, TypeInventory,
    };

    #[repr(C)]
    struct Mock {
        table: *const *const c_void,
        refs: AtomicU32,
        parent: *mut c_void,
        child: *mut c_void,
        name: *const PlainString,
        bad_names: AtomicU32,
    }
    unsafe extern "C" fn retain(pointer: *mut c_void) -> u32 {
        unsafe { &*pointer.cast::<Mock>() }
            .refs
            .fetch_add(1, Ordering::Relaxed)
            + 1
    }
    unsafe extern "C" fn release(pointer: *mut c_void) -> u32 {
        unsafe { &*pointer.cast::<Mock>() }
            .refs
            .fetch_sub(1, Ordering::Relaxed)
            - 1
    }
    unsafe extern "C" fn query_mock(
        pointer: *mut c_void,
        iid: *const Uid,
        out: *mut *mut c_void,
    ) -> i32 {
        unsafe {
            *out = ptr::null_mut();
        }
        if unsafe { *iid } != UNKNOWN {
            return NO_INTERFACE;
        }
        unsafe {
            retain(pointer);
            *out = pointer;
        }
        OK
    }
    unsafe extern "C" fn parent(pointer: *mut c_void) -> *mut c_void {
        unsafe { (*pointer.cast::<Mock>()).parent }
    }
    unsafe extern "C" fn child(pointer: *mut c_void, name: *const PlainString) -> *mut c_void {
        let mock = unsafe { &*pointer.cast::<Mock>() };
        if mock.name == name {
            mock.child
        } else {
            mock.bad_names.fetch_add(1, Ordering::Relaxed);
            ptr::null_mut()
        }
    }
    struct Fixture {
        _table: Box<[*const c_void; 11]>,
        nodes: Box<[Mock; 5]>,
        names: [Box<PlainString>; 2],
    }
    impl Fixture {
        fn new() -> Self {
            let mut table = Box::new([ptr::null(); 11]);
            table[0] = query_mock as *const c_void;
            table[1] = retain as *const c_void;
            table[2] = release as *const c_void;
            table[6] = parent as *const c_void;
            table[10] = child as *const c_void;
            let names = [0, 1].map(|_| {
                Box::new(PlainString {
                    object: ptr::null_mut(),
                    opaque_debug: ptr::null_mut(),
                })
            });
            let nodes = Box::new(std::array::from_fn(|_| Mock {
                table: table.as_ptr(),
                refs: AtomicU32::new(1),
                parent: ptr::null_mut(),
                child: ptr::null_mut(),
                name: ptr::null(),
                bad_names: AtomicU32::new(0),
            }));
            let mut fixture = Self {
                _table: table,
                nodes,
                names,
            };
            fixture.nodes[0].parent = fixture.pointer(1);
            fixture.nodes[1].parent = fixture.pointer(2);
            fixture.nodes[2].name = ptr::addr_of!(*fixture.names[0]);
            fixture.nodes[2].child = fixture.pointer(3);
            fixture.nodes[3].name = ptr::addr_of!(*fixture.names[1]);
            fixture.nodes[3].child = fixture.pointer(4);
            fixture
        }
        fn pointer(&mut self, index: usize) -> *mut c_void {
            ptr::addr_of_mut!(self.nodes[index]).cast()
        }
        fn route<G: FnMut() -> Result<(), String>>(
            &mut self,
            guard: &mut G,
        ) -> Result<GraphRoute, String> {
            let device = NodePin::borrowed(self.pointer(0))?;
            let slot = NodePin::borrowed(self.pointer(1))?;
            let folder = NodePin::borrowed(self.pointer(2))?;
            GraphRoute::resolve(
                device,
                slot,
                folder,
                ptr::addr_of!(*self.names[0]),
                ptr::addr_of!(*self.names[1]),
                guard,
            )
        }
        fn released(&self) {
            assert!(
                self.nodes
                    .iter()
                    .all(|node| node.refs.load(Ordering::Acquire) == 1)
            );
        }
    }

    #[test]
    fn graph_route_retains_the_fixed_attachment_chain_without_requiring_class_uids() {
        let mut fixture = Fixture::new();
        let route = fixture.route(&mut || Ok(())).unwrap();
        assert_eq!(route.graph.node.0, fixture.pointer(4));
        assert!(
            fixture
                .nodes
                .iter()
                .all(|node| node.refs.load(Ordering::Acquire) == 3)
        );
        assert!(
            fixture
                .nodes
                .iter()
                .all(|node| node.bad_names.load(Ordering::Acquire) == 0)
        );
        route.check().unwrap();
        drop(route);
        fixture.released();
    }

    #[test]
    fn graph_route_refuses_replacement_at_every_fixed_edge() {
        let mut fixture = Fixture::new();
        let route = fixture.route(&mut || Ok(())).unwrap();
        for index in 0..4 {
            if index < 2 {
                let original = fixture.nodes[index].parent;
                fixture.nodes[index].parent = fixture.pointer(4);
                assert!(route.check().is_err());
                fixture.nodes[index].parent = original;
            } else {
                let original = fixture.nodes[index].child;
                fixture.nodes[index].child = fixture.pointer(0);
                assert!(route.check().is_err());
                fixture.nodes[index].child = original;
            }
            route.check().unwrap();
        }
        drop(route);
        fixture.released();
    }

    #[test]
    fn graph_route_failure_releases_new_pins_and_does_not_search_other_paths() {
        let mut fixture = Fixture::new();
        fixture.nodes[3].child = ptr::null_mut();
        assert!(fixture.route(&mut || Ok(())).is_err());
        fixture.released();
        assert!(
            fixture
                .nodes
                .iter()
                .all(|node| node.bad_names.load(Ordering::Acquire) == 0)
        );
        fixture.nodes[3].child = fixture.pointer(4);
        let mut calls = 0;
        assert!(
            fixture
                .route(&mut || {
                    calls += 1;
                    if calls == 3 {
                        Err("document changed".into())
                    } else {
                        Ok(())
                    }
                })
                .is_err()
        );
        fixture.released();
    }

    #[test]
    fn graph_scope_rejects_unobserved_insert_families_and_avoids_ancestor_expansion() {
        assert!(validate_insert_family("AudioInsertSlot", "AudioInsertFolder", "Inserts").is_ok());
        for (slot, folder, id) in [
            ("AudioSynthSlot", "AudioInsertFolder", "Inserts"),
            ("AudioInsertSlot", "SynthFolder", "Inserts"),
            ("AudioInsertSlot", "AudioInsertFolder", "other"),
        ] {
            assert!(validate_insert_family(slot, folder, id).is_err());
        }
        assert_eq!(limits(Scope::Graph).ancestors, 0);
        assert_eq!(limits(Scope::Target).ancestors, 3);
        let empty = Report {
            nodes: vec![],
            edges: vec![],
            diagnostics: vec![],
            complete: true,
        };
        let value = report(&empty, Scope::Graph);
        assert_eq!(value["scope"], "graph");
        assert_eq!(value["limits"]["ancestors"], 0);
        assert!(value.get("instance").is_none());
    }

    #[test]
    fn bounded_node_report_keeps_missing_class_ids_and_partial_metadata_explicit() {
        let source = Report {
            nodes: vec![NodeRecord {
                index: 0,
                identity: Some(NodeIdentity {
                    id: "Slot".into(),
                    object_uid: None,
                    class_uid: None,
                }),
                child_count: Some(2),
                reflection: TypeInventory {
                    available: true,
                    types: vec![TypeInfo {
                        name: "SlotController".into(),
                        namespace: Some("Host".into()),
                        class_uid: None,
                        flags: 2,
                        methods: vec![Method {
                            name: "observedMethod".into(),
                            arguments: Some("value".into()),
                            return_type: None,
                        }],
                        properties: vec![Property {
                            name: "observedProperty".into(),
                            data_type: 1,
                            type_name: None,
                            has_type_info: false,
                        }],
                    }],
                    issues: vec![MetadataIssue::MethodLimit],
                },
            }],
            edges: vec![Edge {
                from: 0,
                to: 0,
                relation: Relation::Delegate(Delegate::Slot),
            }],
            diagnostics: vec![
                Diagnostic::NodeCycle { from: 0, to: 0 },
                Diagnostic::DepthLimit(0),
            ],
            complete: false,
        };
        let value = report(&source, Scope::Target);
        assert_eq!(value["complete"], false);
        assert_eq!(value["limits"]["nodes"], 48);
        assert!(value["nodes"][0]["identity"]["class_uid"].is_null());
        assert_eq!(
            value["nodes"][0]["reflection"]["types"][0]["methods"][0]["arguments"],
            "value"
        );
        assert_eq!(value["nodes"][0]["reflection"]["issues"][0], "method_limit");
        assert_eq!(
            value["edges"][0]["relation"],
            json!({"kind":"delegate","name":"Slot"})
        );
        assert_eq!(
            value["diagnostics"][0],
            json!({"kind":"node_cycle","from":0,"to":0})
        );
    }
}

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
    PluginController,
}

pub(super) fn limits(scope: Scope) -> Limits {
    let (nodes, depth) = if scope == Scope::PluginController {
        (96, 5)
    } else {
        (48, 3)
    };
    Limits {
        nodes,
        depth,
        ancestors: if scope == Scope::Target { 3 } else { 0 },
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

pub(super) struct GraphRoute {
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

pub(super) enum Inventory {
    Target(Inspection),
    Graph {
        inspection: Inspection,
        route: GraphRoute,
    },
    PluginController(alias_controller::Inventory),
}

/// Read-only metadata of a bridge-created object. This never authorizes a
/// subsequent mutation and makes no claim that the auxiliary owns the Device.
pub(super) enum Auxiliary {
    TypeMetadata(Inspection),
    InsertFolderAncestry {
        inspection: Inspection,
        ancestor_matches_active_model: bool,
        document_environment: DocumentEnvironmentObservation,
    },
}

pub(super) enum DocumentEnvironmentObservation {
    Unavailable(String),
    Observed {
        route: DocumentEnvironmentRoute,
        ancestor_matches: bool,
    },
}

pub(super) struct DocumentEnvironmentRoute {
    manager: NodePin,
    controller: NodePin,
    environment: NodePin,
    active_document_name: *const PlainString,
    environment_name: *const PlainString,
}
impl DocumentEnvironmentRoute {
    fn manager(pointer: *mut c_void) -> Result<NodePin, String> {
        let manager_node =
            query(pointer, &NODE).ok_or("document manager node interface unavailable")?;
        let manager = NodePin::borrowed(manager_node.0)?;
        let canonical =
            query(pointer, &UNKNOWN).ok_or("document manager canonical identity unavailable")?;
        let same = canonical.0 == manager._canonical.0;
        drop(canonical);
        drop(manager_node);
        if !same {
            return Err("document manager node canonical identity mismatch".into());
        }
        Ok(manager)
    }
    fn resolve<G: FnMut() -> Result<(), String>>(
        manager: NodePin,
        active_document_name: *const PlainString,
        environment_name: *const PlainString,
        guard: &mut G,
    ) -> Result<Self, String> {
        guard()?;
        let controller = manager.child(active_document_name)?;
        guard()?;
        let environment = controller.child(environment_name)?;
        guard()?;
        let route = Self {
            manager,
            controller,
            environment,
            active_document_name,
            environment_name,
        };
        route.check()?;
        guard()?;
        Ok(route)
    }
    fn capture(ctx: &Context<'_>) -> Result<Self, String> {
        Self::capture_for_document(&ctx.document.manager, &mut || ctx.check())
    }
    pub(super) fn capture_for_document<G: FnMut() -> Result<(), String>>(
        document_manager: &Owned,
        guard: &mut G,
    ) -> Result<Self, String> {
        guard()?;
        let get_string = constant_string().ok_or("native string constants unavailable")?;
        Self::capture_with_text(document_manager, get_string, guard)
    }
    pub(super) fn capture_with_text<G: FnMut() -> Result<(), String>>(
        document_manager: &Owned,
        get_string: ConstantString,
        guard: &mut G,
    ) -> Result<Self, String> {
        guard()?;
        let active_document_name = unsafe { get_string(c"ActiveDocument".as_ptr()) };
        let environment_name = unsafe { get_string(c"Environment".as_ptr()) };
        if active_document_name.is_null() || environment_name.is_null() {
            return Err("fixed document environment identifiers unavailable".into());
        }
        let manager = Self::manager(document_manager.0)?;
        guard()?;
        let route = Self::resolve(manager, active_document_name, environment_name, guard)?;
        let id = route.environment.id()?;
        let family = class_name(route.environment.node.0);
        route.check()?;
        guard()?;
        validate_document_environment_family(&id, &family)?;
        Ok(route)
    }
    pub(super) fn check(&self) -> Result<(), String> {
        let find: unsafe extern "C" fn(*mut c_void, *const PlainString) -> *mut c_void =
            unsafe { method(self.manager.node.0, 10) };
        let child: unsafe extern "C" fn(*mut c_void, *const PlainString) -> *mut c_void =
            unsafe { method(self.controller.node.0, 10) };
        if unsafe { find(self.manager.node.0, self.active_document_name) } != self.controller.node.0
            || unsafe { child(self.controller.node.0, self.environment_name) }
                != self.environment.node.0
        {
            Err("active document Environment attachment changed".into())
        } else {
            Ok(())
        }
    }
    pub(super) fn matches(&self, inspection: &Inspection) -> Result<bool, String> {
        inspection
            .ancestor_matches_canonical(
                NonNull::new(self.environment._canonical.0).expect("retained Environment identity"),
            )
            .map_err(|error| error.to_string())
    }
    pub(super) fn canonical_keys(&self) -> [usize; 3] {
        [
            self.manager._canonical.0 as usize,
            self.controller._canonical.0 as usize,
            self.environment._canonical.0 as usize,
        ]
    }
    pub(super) fn check_family<G: FnMut() -> Result<(), String>>(
        &self,
        guard: &mut G,
    ) -> Result<(), String> {
        guard()?;
        let id = self.environment.id()?;
        guard()?;
        let family = class_name(self.environment.node.0);
        guard()?;
        validate_document_environment_family(&id, &family)
    }
}

fn validate_document_environment_family(id: &str, family: &str) -> Result<(), String> {
    if id == "SongEnvironment" && family == "UltimateStudioEnvironment" {
        Ok(())
    } else {
        Err("fixed document Environment has an unverified node family".into())
    }
}

impl DocumentEnvironmentObservation {
    fn capture(ctx: &Context<'_>, inspection: &Inspection) -> Result<Self, String> {
        let observed = DocumentEnvironmentRoute::capture(ctx).and_then(|route| {
            let ancestor_matches = route.matches(inspection)?;
            Ok(Self::Observed {
                route,
                ancestor_matches,
            })
        });
        match observed {
            Ok(observed) => {
                if let Err(reason) = observed.check(inspection) {
                    drop(observed);
                    ctx.check()?;
                    return Err(reason);
                }
                ctx.check()?;
                Ok(observed)
            }
            Err(reason) => {
                ctx.check()?;
                Ok(Self::Unavailable(reason))
            }
        }
    }
    fn check(&self, inspection: &Inspection) -> Result<(), String> {
        if let Self::Observed {
            route,
            ancestor_matches,
        } = self
        {
            route.check()?;
            if route.matches(inspection)? != *ancestor_matches {
                return Err("retained document Environment ancestor observation changed".into());
            }
        }
        Ok(())
    }
    fn value(&self) -> Value {
        match self {
            Self::Unavailable(reason) => json!({"status":"unavailable","reason":reason,
                "scope":"fixed_active_document_controller_environment","mutation_capability":false,"atomic_snapshot":false}),
            Self::Observed {
                ancestor_matches, ..
            } => json!({"status":"observed",
                "scope":"fixed_active_document_controller_environment","ancestor_matches":ancestor_matches,
                "mutation_capability":false,"atomic_snapshot":false}),
        }
    }
}
#[derive(Clone, Copy)]
enum AuxiliaryRole {
    TypeMetadata,
    InsertFolderAncestry,
}
impl Auxiliary {
    pub(super) fn document_environment_matched(&self) -> bool {
        matches!(
            self,
            Self::InsertFolderAncestry {
                document_environment: DocumentEnvironmentObservation::Observed {
                    ancestor_matches: true,
                    ..
                },
                ..
            }
        )
    }
    fn inspection(&self) -> &Inspection {
        match self {
            Self::TypeMetadata(inspection) | Self::InsertFolderAncestry { inspection, .. } => {
                inspection
            }
        }
    }
    pub(super) fn check(&self, ctx: &Context<'_>) -> Result<(), String> {
        self.inspection()
            .check(&mut || ctx.check())
            .map_err(|e| e.to_string())?;
        let observation = if let Self::InsertFolderAncestry {
            inspection,
            ancestor_matches_active_model,
            document_environment,
        } = self
        {
            ancestry_matches_model(inspection, ctx).and_then(|current| {
                if current == *ancestor_matches_active_model {
                    document_environment.check(inspection)
                } else {
                    Err("retained Inserts ancestor observation changed".into())
                }
            })
        } else {
            Ok(())
        };
        // Attachment reads create no temporary owners. Keep Context last.
        ctx.check()?;
        observation
    }
    pub(super) fn value(&self) -> Value {
        let mut value = report(self.inspection().report(), Scope::Target);
        value["mutation_capability"] = json!(false);
        match self {
            Self::TypeMetadata(_) => {
                value["coverage"] = json!("bounded_readonly_auxiliary_type_metadata");
                value["scope"] = json!("auxiliary_type_observation");
                value["limits"] = json!({"nodes":1,"depth":0,"ancestors":0,"children_per_node":1});
            }
            Self::InsertFolderAncestry {
                ancestor_matches_active_model,
                document_environment,
                ..
            } => {
                value["coverage"] = json!("bounded_readonly_insert_folder_ancestor_metadata");
                value["scope"] = json!("insert_folder_ancestry_observation");
                value["limits"] = json!({"nodes":9,"depth":0,"ancestors":8,"children_per_node":1});
                value["traversal"] = json!("parent_chain_only");
                value["descendants_observed"] = json!(false);
                value["atomic_snapshot"] = json!(false);
                value["ancestor_matches_active_model"] = json!(ancestor_matches_active_model);
                value["document_environment_observation"] = document_environment.value();
                value["negative_match_scope"] = json!("no_match_in_bounded_parent_chain");
            }
        }
        value
    }
}
pub(super) fn capture_auxiliary(ctx: &Context<'_>, auxiliary: Owned) -> Result<Auxiliary, String> {
    let inspection = capture_auxiliary_inspection(ctx, auxiliary, AuxiliaryRole::TypeMetadata)?;
    let auxiliary = Auxiliary::TypeMetadata(inspection);
    auxiliary.check(ctx)?;
    Ok(auxiliary)
}

fn ancestry_matches_model(inspection: &Inspection, ctx: &Context<'_>) -> Result<bool, String> {
    inspection
        .ancestor_matches_canonical(
            NonNull::new(ctx.document._model_identity.0).expect("retained model identity"),
        )
        .map_err(|e| e.to_string())
}

fn validate_insert_folder_root(report: &Report) -> Result<(), String> {
    let root = report
        .nodes
        .first()
        .ok_or("Inserts root was not observed")?;
    let identity = root
        .identity
        .as_ref()
        .ok_or("Inserts auxiliary has no IObjectNode identity")?;
    if root.index != 0
        || identity.id != "Inserts"
        || !identity.object_uid.as_deref().is_some_and(valid_uid)
        || identity
            .class_uid
            .as_deref()
            .is_some_and(|uid| !valid_uid(uid))
        || !root.reflection.available
        || root
            .reflection
            .types
            .first()
            .is_none_or(|info| info.name != "AudioInsertFolder")
    {
        return Err(
            "ancestry requires the exact AudioInsertFolder/Inserts family and stable object UID"
                .into(),
        );
    }
    Ok(())
}

pub(super) fn capture_insert_folder_ancestry(
    ctx: &Context<'_>,
    auxiliary: Owned,
) -> Result<Auxiliary, String> {
    let inspection =
        capture_auxiliary_inspection(ctx, auxiliary, AuxiliaryRole::InsertFolderAncestry)?;
    let observed = validate_insert_folder_root(inspection.report())
        .and_then(|_| ancestry_matches_model(&inspection, ctx));
    let ancestor_matches_active_model = match observed {
        Ok(observed) => observed,
        Err(reason) => {
            drop(inspection);
            ctx.check()?;
            return Err(reason);
        }
    };
    let document_environment = match DocumentEnvironmentObservation::capture(ctx, &inspection) {
        Ok(observed) => observed,
        Err(reason) => {
            drop(inspection);
            ctx.check()?;
            return Err(reason);
        }
    };
    let auxiliary = Auxiliary::InsertFolderAncestry {
        document_environment,
        inspection,
        ancestor_matches_active_model,
    };
    if let Err(reason) = auxiliary.check(ctx) {
        drop(auxiliary);
        ctx.check()?;
        return Err(reason);
    }
    Ok(auxiliary)
}

fn capture_auxiliary_inspection(
    ctx: &Context<'_>,
    auxiliary: Owned,
    role: AuxiliaryRole,
) -> Result<Inspection, String> {
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
    let result = match role {
        AuxiliaryRole::TypeMetadata => inspector.inspect(
            Limits {
                nodes: 1,
                depth: 0,
                ancestors: 0,
                children_per_node: 1,
            },
            &mut || ctx.check(),
        ),
        AuxiliaryRole::InsertFolderAncestry => inspector.inspect_ancestors(&mut || ctx.check()),
    };
    drop(inspector);
    if let Err(reason) = ctx.check() {
        drop(result);
        ctx.check()?;
        return Err(reason);
    }
    result.map_err(|e| e.to_string())
}
impl Inventory {
    #[cfg(test)]
    pub(super) fn report(&self) -> &Report {
        match self {
            Self::Target(inspection) | Self::Graph { inspection, .. } => inspection.report(),
            Self::PluginController(inventory) => {
                inventory.report().expect("synthetic present controller")
            }
        }
    }
    pub(super) fn value(&self) -> Value {
        match self {
            Self::Target(inspection) => report(inspection.report(), Scope::Target),
            Self::Graph { inspection, .. } => report(inspection.report(), Scope::Graph),
            Self::PluginController(inventory) => inventory.value(),
        }
    }
    pub(super) fn is_plugin_controller(&self) -> bool {
        matches!(self, Self::PluginController(_))
    }
    pub(super) fn check<G: FnMut() -> Result<(), String>>(
        &self,
        guard: &mut G,
    ) -> Result<(), String> {
        match self {
            Self::PluginController(inventory) => inventory.check(guard),
            Self::Target(inspection) => {
                inspection.check(guard).map_err(|error| error.to_string())?;
                guard()
            }
            Self::Graph { inspection, route } => {
                let mut checked = || {
                    route.check()?;
                    guard()
                };
                checked()?;
                inspection
                    .check(&mut checked)
                    .map_err(|error| error.to_string())?;
                checked()
            }
        }
    }
    #[cfg(test)]
    pub(super) fn target(inspection: Inspection) -> Self {
        Self::Target(inspection)
    }
    #[cfg(test)]
    pub(super) fn into_auxiliary(self) -> Auxiliary {
        match self {
            Self::Target(inspection) | Self::Graph { inspection, .. } => {
                Auxiliary::TypeMetadata(inspection)
            }
            Self::PluginController(_) => panic!("synthetic target inventory required"),
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
    if scope == Scope::PluginController {
        return alias_controller::capture(ctx, text, limits(scope))
            .map(Inventory::PluginController);
    }
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
    let inspection = result.map_err(|error| error.to_string())?;
    let inventory = match route {
        Some(route) => Inventory::Graph { inspection, route },
        None => Inventory::Target(inspection),
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
        "scope":match scope { Scope::Target=>"target", Scope::Graph=>"graph", Scope::PluginController=>"alias_declared_controller_observation" },
        "complete":report.complete,
        "limits":{"nodes":limits.nodes,"depth":limits.depth,"ancestors":limits.ancestors,"children_per_node":limits.children_per_node},
        "nodes":report.nodes.iter().map(|node| json!({
            "index":node.index,
            "identity":node.identity.as_ref().map(|identity|json!({"id":identity.id,"object_uid":identity.object_uid,"class_uid":identity.class_uid})),
            "child_count":node.child_count,
            "declared_interfaces":node.declared_interfaces.as_ref().map(|observed|json!({
                "preset_collector":observed.preset_collector,
                "preset_mediator":observed.preset_mediator,
                "storable":observed.storable,
            })),
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
        parent_reads: AtomicU32,
        identity: *const PlainString,
        reject_canonical: bool,
        replace_on_query: *mut Mock,
        replacement: *mut c_void,
        foreign_node_interface: *mut c_void,
    }
    #[repr(C)]
    struct EmptyText {
        table: *const *const c_void,
        refs: AtomicU32,
        replace_on_release: *mut Mock,
        replacement: *mut c_void,
    }
    unsafe extern "C" fn text_retain(pointer: *mut c_void) -> u32 {
        unsafe { &*pointer.cast::<EmptyText>() }
            .refs
            .fetch_add(1, Ordering::Relaxed)
            + 1
    }
    unsafe extern "C" fn text_release(pointer: *mut c_void) -> u32 {
        let text = unsafe { &mut *pointer.cast::<EmptyText>() };
        let refs = text.refs.fetch_sub(1, Ordering::Relaxed) - 1;
        if !text.replace_on_release.is_null() {
            unsafe {
                (*text.replace_on_release).child = text.replacement;
            }
            text.replace_on_release = ptr::null_mut();
        }
        refs
    }
    unsafe extern "C" fn text_length(_: *mut c_void) -> i32 {
        0
    }
    unsafe extern "C" fn text_copy(_: *mut c_void, out: *mut u16, count: i32) -> i32 {
        assert_eq!(count, 1);
        unsafe {
            *out = 0;
        }
        OK
    }
    unsafe extern "C" fn identity(pointer: *mut c_void) -> *const PlainString {
        unsafe { &*pointer.cast::<Mock>() }.identity
    }
    unsafe extern "C" fn missing_uid(_: *mut c_void) -> *const Uid {
        static EMPTY: Uid = Uid {
            a: 0,
            b: 0,
            c: 0,
            d: [0; 8],
        };
        &EMPTY
    }
    unsafe extern "C" fn no_children(_: *mut c_void) -> i32 {
        0
    }
    unsafe extern "C" fn unused_text(_: *const c_char) -> *const c_void {
        ptr::null()
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
        let mock = unsafe { &*pointer.cast::<Mock>() };
        if unsafe { *iid } == UNKNOWN && mock.reject_canonical {
            unsafe {
                *out = ptr::dangling_mut::<c_void>();
            }
            return NO_INTERFACE;
        }
        if unsafe { *iid } != UNKNOWN && unsafe { *iid } != NODE {
            return NO_INTERFACE;
        }
        if unsafe { *iid } == NODE && !mock.foreign_node_interface.is_null() {
            unsafe {
                retain(mock.foreign_node_interface);
                *out = mock.foreign_node_interface;
            }
            return OK;
        }
        unsafe {
            retain(pointer);
            *out = pointer;
            if !mock.replace_on_query.is_null() {
                (*mock.replace_on_query).child = mock.replacement;
            }
        }
        OK
    }
    unsafe extern "C" fn parent(pointer: *mut c_void) -> *mut c_void {
        let mock = unsafe { &*pointer.cast::<Mock>() };
        mock.parent_reads.fetch_add(1, Ordering::Relaxed);
        mock.parent
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
        _text_table: Box<[*const c_void; 9]>,
        text: Box<EmptyText>,
        _identity: Box<PlainString>,
    }
    impl Fixture {
        fn new() -> Self {
            let mut table = Box::new([ptr::null(); 11]);
            table[0] = query_mock as *const c_void;
            table[1] = retain as *const c_void;
            table[2] = release as *const c_void;
            table[3] = identity as *const c_void;
            table[4] = missing_uid as *const c_void;
            table[5] = missing_uid as *const c_void;
            table[6] = parent as *const c_void;
            table[8] = no_children as *const c_void;
            table[10] = child as *const c_void;
            let mut text_table = Box::new([ptr::null(); 9]);
            text_table[1] = text_retain as *const c_void;
            text_table[2] = text_release as *const c_void;
            text_table[4] = text_length as *const c_void;
            text_table[8] = text_copy as *const c_void;
            let mut text = Box::new(EmptyText {
                table: text_table.as_ptr(),
                refs: AtomicU32::new(1),
                replace_on_release: ptr::null_mut(),
                replacement: ptr::null_mut(),
            });
            let identity = Box::new(PlainString {
                object: ptr::addr_of_mut!(*text).cast(),
                opaque_debug: ptr::null_mut(),
            });
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
                parent_reads: AtomicU32::new(0),
                identity: ptr::addr_of!(*identity),
                reject_canonical: false,
                replace_on_query: ptr::null_mut(),
                replacement: ptr::null_mut(),
                foreign_node_interface: ptr::null_mut(),
            }));
            let mut fixture = Self {
                _table: table,
                nodes,
                names,
                _text_table: text_table,
                text,
                _identity: identity,
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
            assert_eq!(self.text.refs.load(Ordering::Acquire), 1);
        }
        fn environment_route<G: FnMut() -> Result<(), String>>(
            &mut self,
            guard: &mut G,
        ) -> Result<DocumentEnvironmentRoute, String> {
            let manager = NodePin::borrowed(self.pointer(2))?;
            DocumentEnvironmentRoute::resolve(
                manager,
                ptr::addr_of!(*self.names[0]),
                ptr::addr_of!(*self.names[1]),
                guard,
            )
        }
        fn ancestors(&mut self) -> Inspection {
            let owner = hold(self.pointer(0)).unwrap();
            let pointer = NonNull::new(owner.0).unwrap();
            std::mem::forget(owner);
            let inspector = unsafe {
                NodeInspector::from_owned(
                    pointer,
                    TextApi {
                        get_constant_string: unused_text,
                    },
                )
            };
            inspector.inspect_ancestors(&mut || Ok(())).unwrap()
        }
    }

    #[test]
    fn document_environment_uses_only_two_fixed_child_edges_and_retains_their_owners() {
        let mut fixture = Fixture::new();
        let route = fixture.environment_route(&mut || Ok(())).unwrap();
        assert_eq!(route.manager.node.0, fixture.pointer(2));
        assert_eq!(route.controller.node.0, fixture.pointer(3));
        assert_eq!(route.environment.node.0, fixture.pointer(4));
        for (index, node) in fixture.nodes.iter().enumerate() {
            assert_eq!(
                node.refs.load(Ordering::Acquire),
                if index < 2 { 1 } else { 3 }
            );
            assert_eq!(node.parent_reads.load(Ordering::Acquire), 0);
            assert_eq!(node.bad_names.load(Ordering::Acquire), 0);
        }
        route.check().unwrap();
        drop(route);
        fixture.released();
    }

    #[test]
    fn document_environment_manager_node_must_have_the_original_manager_canonical_identity() {
        let mut fixture = Fixture::new();
        let manager = DocumentEnvironmentRoute::manager(fixture.pointer(2)).unwrap();
        assert_eq!(manager.node.0, fixture.pointer(2));
        drop(manager);
        fixture.released();
        fixture.nodes[2].foreign_node_interface = fixture.pointer(3);
        assert!(DocumentEnvironmentRoute::manager(fixture.pointer(2)).is_err());
        fixture.released();
        fixture.nodes[2].foreign_node_interface = ptr::null_mut();
        fixture.nodes[2].reject_canonical = true;
        assert!(DocumentEnvironmentRoute::manager(fixture.pointer(2)).is_err());
        fixture.released();
    }

    #[test]
    fn document_environment_match_is_only_the_existing_retained_parent_chain() {
        let mut fixture = Fixture::new();
        let inspection = fixture.ancestors();
        let unrelated = fixture.environment_route(&mut || Ok(())).unwrap();
        assert!(!unrelated.matches(&inspection).unwrap());
        drop(unrelated);
        fixture.nodes[3].child = fixture.pointer(1);
        let route = fixture.environment_route(&mut || Ok(())).unwrap();
        assert!(route.matches(&inspection).unwrap());
        let observation = DocumentEnvironmentObservation::Observed {
            route,
            ancestor_matches: true,
        };
        observation.check(&inspection).unwrap();
        assert_eq!(observation.value()["ancestor_matches"], true);
        assert_eq!(
            observation.value()["scope"],
            "fixed_active_document_controller_environment"
        );
        assert_eq!(observation.value()["mutation_capability"], false);
        assert_eq!(observation.value()["atomic_snapshot"], false);
        drop(observation);
        drop(inspection);
        fixture.released();
    }

    #[test]
    fn document_environment_missing_fixed_children_never_uses_a_fallback() {
        for missing in [2, 3] {
            let mut fixture = Fixture::new();
            fixture.nodes[missing].child = ptr::null_mut();
            assert!(fixture.environment_route(&mut || Ok(())).is_err());
            fixture.released();
            assert!(
                fixture
                    .nodes
                    .iter()
                    .all(|node| node.bad_names.load(Ordering::Acquire) == 0)
            );
        }
        let unavailable =
            DocumentEnvironmentObservation::Unavailable("fixed child unavailable".into()).value();
        assert_eq!(unavailable["status"], "unavailable");
        assert!(unavailable.get("ancestor_matches").is_none());
    }

    #[test]
    fn document_environment_refuses_both_edge_replacements_and_query_reentry() {
        let mut fixture = Fixture::new();
        let route = fixture.environment_route(&mut || Ok(())).unwrap();
        for index in [2, 3] {
            let original = fixture.nodes[index].child;
            fixture.nodes[index].child = fixture.pointer(0);
            assert!(route.check().is_err());
            fixture.nodes[index].child = original;
            route.check().unwrap();
        }
        drop(route);
        fixture.released();
        fixture.nodes[3].replace_on_query = ptr::addr_of_mut!(fixture.nodes[2]);
        fixture.nodes[3].replacement = fixture.pointer(4);
        assert!(fixture.environment_route(&mut || Ok(())).is_err());
        fixture.released();
    }

    #[test]
    fn document_environment_rejects_failed_nonnull_canonical_qi_without_touching_the_sentinel() {
        let mut fixture = Fixture::new();
        fixture.nodes[3].reject_canonical = true;
        assert!(fixture.environment_route(&mut || Ok(())).is_err());
        fixture.released();
    }

    #[test]
    fn document_environment_guard_failure_releases_every_new_route_owner() {
        for fail_at in 1..=4 {
            let mut fixture = Fixture::new();
            let mut calls = 0;
            assert!(
                fixture
                    .environment_route(&mut || {
                        calls += 1;
                        if calls == fail_at {
                            Err("context changed".into())
                        } else {
                            Ok(())
                        }
                    })
                    .is_err()
            );
            fixture.released();
        }
    }

    #[test]
    fn document_environment_requires_the_actual_observed_song_family() {
        validate_document_environment_family("SongEnvironment", "UltimateStudioEnvironment")
            .unwrap();
        for (id, class) in [
            ("Environment", "UltimateStudioEnvironment"),
            ("SongEnvironment", "StudioEnvironment"),
            ("", ""),
        ] {
            assert!(validate_document_environment_family(id, class).is_err());
        }
    }

    #[test]
    fn document_environment_refuses_metadata_owner_cleanup_that_replaces_the_fixed_child() {
        let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
        let mut context_fixture = super::super::guard_tests::Fixture::new();
        context_fixture.canonical_queries();
        let context = context_fixture.context();
        let mut fixture = Fixture::new();
        let inspection = fixture.ancestors();
        fixture.nodes[3].child = fixture.pointer(1);
        let route = fixture.environment_route(&mut || Ok(())).unwrap();
        let auxiliary = Auxiliary::InsertFolderAncestry {
            inspection,
            ancestor_matches_active_model: false,
            document_environment: DocumentEnvironmentObservation::Observed {
                route,
                ancestor_matches: true,
            },
        };
        fixture.text.replace_on_release = ptr::addr_of_mut!(fixture.nodes[3]);
        fixture.text.replacement = fixture.pointer(4);
        assert!(auxiliary.check(&context).is_err());
        assert!(context.check().is_ok());
        assert!(fixture.text.replace_on_release.is_null());
        drop(auxiliary);
        fixture.released();
        drop(context);
    }

    #[test]
    fn document_environment_owners_survive_publication_and_context_loss_removes_the_proof() {
        let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
        let mut context_fixture = super::super::guard_tests::Fixture::new();
        context_fixture.canonical_queries();
        let context = context_fixture.context();
        let service = context.service;
        let mut fixture = Fixture::new();
        let inspection = fixture.ancestors();
        fixture.nodes[3].child = fixture.pointer(1);
        let route = fixture.environment_route(&mut || Ok(())).unwrap();
        let auxiliary = Auxiliary::InsertFolderAncestry {
            inspection,
            ancestor_matches_active_model: false,
            document_environment: DocumentEnvironmentObservation::Observed {
                route,
                ancestor_matches: true,
            },
        };
        auxiliary.check(&context).unwrap();
        let report = auxiliary.value();
        let mut reply = Reply::unbound(
            json!({"ok":true,"outcome":"observed","scope":"insert_folder_ancestry_observation",
            "ancestor_matches_active_model":false,"document_environment_observation":report["document_environment_observation"],
            "insert_folder_ancestry_report":report}),
        );
        reply.auxiliary = Some(auxiliary);
        reply.context = Some(context);
        let mut out = VariantOut {
            kind: 0,
            payload: 0,
        };
        assert!(publish_reply(&mut out, reply, api().unwrap(), service));
        let value: Value =
            serde_json::from_str(&super::super::super::tests::output_text(&out)).unwrap();
        assert_eq!(value["ok"], true);
        assert_eq!(value["ancestor_matches_active_model"], false);
        assert_eq!(
            value["document_environment_observation"]["ancestor_matches"],
            true
        );
        assert!(fixture.nodes[3].refs.load(Ordering::Acquire) > 1);
        release_receipt(service);
        fixture.released();
        super::super::super::tests::clear_output(&mut out);

        let context = context_fixture.context();
        let inspection = fixture.ancestors();
        let route = fixture.environment_route(&mut || Ok(())).unwrap();
        let auxiliary = Auxiliary::InsertFolderAncestry {
            inspection,
            ancestor_matches_active_model: false,
            document_environment: DocumentEnvironmentObservation::Observed {
                route,
                ancestor_matches: true,
            },
        };
        let report = auxiliary.value();
        let mut reply = Reply::unbound(
            json!({"ok":true,"outcome":"observed","scope":"insert_folder_ancestry_observation",
            "ancestor_matches_active_model":false,"document_environment_observation":report["document_environment_observation"],
            "insert_folder_ancestry_report":report}),
        );
        reply.auxiliary = Some(auxiliary);
        reply.context = Some(context);
        context_fixture.switch_document();
        assert!(!reply.final_check(service));
        assert!(
            reply
                .value
                .get("document_environment_observation")
                .is_none()
        );
        assert!(reply.value.get("insert_folder_ancestry_report").is_none());
        drop(reply);
        fixture.released();
    }

    #[test]
    fn document_environment_also_rejects_a_model_switch_under_the_same_document() {
        let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
        let mut context_fixture = super::super::guard_tests::Fixture::new();
        context_fixture.canonical_queries();
        let context = context_fixture.context();
        let mut fixture = Fixture::new();
        let inspection = fixture.ancestors();
        fixture.nodes[3].child = fixture.pointer(1);
        let route = fixture.environment_route(&mut || Ok(())).unwrap();
        let auxiliary = Auxiliary::InsertFolderAncestry {
            inspection,
            ancestor_matches_active_model: false,
            document_environment: DocumentEnvironmentObservation::Observed {
                route,
                ancestor_matches: true,
            },
        };
        auxiliary.check(&context).unwrap();
        context_fixture.switch_model();
        assert!(auxiliary.check(&context).is_err());
        drop(auxiliary);
        fixture.released();
        drop(context);
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
    fn only_plugin_controller_inventory_has_the_deeper_report_bounds() {
        let empty = Report {
            nodes: vec![],
            edges: vec![],
            diagnostics: vec![],
            complete: false,
        };
        for (scope, nodes, depth, ancestors) in [
            (Scope::Target, 48, 3, 3),
            (Scope::Graph, 48, 3, 0),
            (Scope::PluginController, 96, 5, 0),
        ] {
            let bounded = limits(scope);
            assert_eq!(
                (
                    bounded.nodes,
                    bounded.depth,
                    bounded.ancestors,
                    bounded.children_per_node
                ),
                (nodes, depth, ancestors, 32)
            );
            assert_eq!(
                report(&empty, scope)["limits"],
                json!({
                    "nodes":nodes,"depth":depth,"ancestors":ancestors,"children_per_node":32,
                })
            );
        }
    }

    #[test]
    fn bounded_node_report_keeps_missing_class_ids_and_partial_metadata_explicit() {
        let mut source = Report {
            nodes: vec![NodeRecord {
                index: 0,
                identity: Some(NodeIdentity {
                    id: "Slot".into(),
                    object_uid: None,
                    class_uid: None,
                }),
                child_count: Some(2),
                declared_interfaces: None,
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
        assert!(value["nodes"][0]["declared_interfaces"].is_null());
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
        source.nodes[0].declared_interfaces = Some(fender_native_node_api::DeclaredInterfaces {
            preset_collector: true,
            preset_mediator: false,
            storable: true,
        });
        assert_eq!(
            report(&source, Scope::Target)["nodes"][0]["declared_interfaces"],
            json!({"preset_collector":true,"preset_mediator":false,"storable":true})
        );
    }

    fn insert_folder_root() -> Report {
        Report {
            nodes: vec![NodeRecord {
                index: 0,
                identity: Some(NodeIdentity {
                    id: "Inserts".into(),
                    object_uid: Some(uid(&SERVICE_CID)),
                    class_uid: None,
                }),
                child_count: Some(2),
                declared_interfaces: None,
                reflection: TypeInventory {
                    available: true,
                    types: vec![TypeInfo {
                        name: "AudioInsertFolder".into(),
                        namespace: None,
                        class_uid: None,
                        flags: 0,
                        methods: vec![],
                        properties: vec![],
                    }],
                    issues: vec![],
                },
            }],
            edges: vec![],
            diagnostics: vec![],
            complete: true,
        }
    }

    #[test]
    fn insert_folder_ancestry_requires_exact_node_family_and_object_uid_but_allows_null_class_uid()
    {
        let source = insert_folder_root();
        validate_insert_folder_root(&source).unwrap();
        for invalid in 0..8 {
            let mut source = insert_folder_root();
            let root = &mut source.nodes[0];
            match invalid {
                0 => root.identity = None,
                1 => root.identity.as_mut().unwrap().id = "Other".into(),
                2 => root.identity.as_mut().unwrap().object_uid = None,
                3 => root.identity.as_mut().unwrap().object_uid = Some(ZERO_UID.into()),
                4 => root.identity.as_mut().unwrap().class_uid = Some("malformed".into()),
                5 => root.reflection.types[0].name = "AudioSynthFolder".into(),
                6 => root.reflection.available = false,
                7 => root.reflection.types.clear(),
                _ => unreachable!(),
            }
            assert!(
                validate_insert_folder_root(&source).is_err(),
                "case {invalid}"
            );
        }
        let mut missing = source;
        missing.nodes.clear();
        assert!(validate_insert_folder_root(&missing).is_err());
    }
}

use super::*;
use fender_native_editor_api::{DeviceMatch, EditorRootLease, GuiApi};
use std::cell::RefCell;
#[path = "gui_parameter/controller_probe.rs"]
mod controller_probe;

pub(super) const PARAMETER: &str = "chorder.autofill";
const GUI_PARAMETER: &str = "autofill";
const CHORDER: &str = "D2D9B002-12FC-4F1D-AFDA-70336D1DC108";
const MAX_VIEWS: usize = 128;
const MAX_DEPTH: usize = 32;
const VIEW: Uid = Uid {
    a: 0xf5b09f71,
    b: 0x6f3f,
    c: 0x4f03,
    d: [0xa5, 0x4a, 0xb2, 0x49, 0xb8, 0x29, 0x62, 0xd6],
};
const CONTROL: Uid = Uid {
    a: 0x8186d4bb,
    b: 0x76b6,
    c: 0x46f6,
    d: [0x88, 0x9f, 0x9a, 0xd0, 0xef, 0x50, 0x19, 0x81],
};

#[derive(Clone, Copy, Default)]
struct Coverage {
    complete: bool,
    candidates: usize,
    views: usize,
}
impl Coverage {
    fn value(self) -> Value {
        json!({"complete":self.complete,"candidate_count":self.candidates,
            "visited_views":self.views,"max_views":MAX_VIEWS,"max_depth":MAX_DEPTH})
    }
}
fn common(coverage: Coverage) -> Value {
    json!({"scope":"exact_device_named_control_candidate","parameter_name":PARAMETER,
        "semantic_binding":"named_candidate_only","mutation_capability":false,
        "coverage":coverage.value()})
}
fn unresolved(reason: &str, coverage: Coverage) -> Value {
    let mut value = common(coverage);
    value["status"] = json!("unresolved");
    value["reason"] = json!(reason);
    value
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum WalkKind {
    ViewLimit,
    DepthLimit,
    RepeatedView,
    DetachedView,
    InterfaceQueryFailed,
    MissingInterface,
    MissingIterator,
    MissingView,
    ParameterMetadata,
    ContextGuardFailed,
    InspectionFailed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum WalkStage {
    Root,
    ControlQuery,
    ParameterMetadata,
    ControlIdentity,
    Children,
    Iterator,
    ChildQuery,
    ViewIdentity,
    Edge,
    Cleanup,
    Context,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
struct WalkFailure {
    kind: WalkKind,
    stage: WalkStage,
    depth: usize,
    query_status: Option<i32>,
}
impl WalkFailure {
    fn new(kind: WalkKind, stage: WalkStage, depth: usize) -> Self {
        Self {
            kind,
            stage,
            depth,
            query_status: None,
        }
    }
    fn context(stage: WalkStage, depth: usize) -> Self {
        Self::new(WalkKind::ContextGuardFailed, stage, depth)
    }
    fn query(error: QueryFailure, stage: WalkStage, depth: usize) -> Self {
        match error {
            QueryFailure::Guard(_) => Self::context(stage, depth),
            QueryFailure::Native(status) => Self {
                kind: WalkKind::InterfaceQueryFailed,
                stage,
                depth,
                query_status: Some(status),
            },
        }
    }
}
pub(super) fn invalidate(value: &mut Value) {
    if value["operation"] == "edit_chorder_autofill" {
        let object = value.as_object_mut().expect("object GUI edit response");
        object.remove("gui_parameter");
        object.insert("gui_value_verified".into(), json!(false));
    }
    if let Some(previous) = value.get("gui_comparison") {
        let coverage = previous.get("coverage").cloned();
        let mut replacement = unresolved("stale_association", Coverage::default());
        if let Some(coverage) = coverage {
            replacement["coverage"] = coverage;
        }
        value["gui_comparison"] = replacement;
    }
}

struct ParameterPin {
    observer: Option<parameters::ControllerOwner>,
    observer_canonical: Option<Owned>,
    canonical: Owned,
    parameter: RawParameter,
}
impl ParameterPin {
    fn capture<G: FnMut() -> Result<(), String>>(
        parameter: RawParameter,
        guard: &mut G,
    ) -> Result<Self, String> {
        let canonical = required(parameter.as_borrowed_raw().as_ptr(), &UNKNOWN, guard)?;
        let observer = parameter
            .controller_owner(guard)
            .map_err(|e| e.to_string())?;
        let observer_canonical = match observer.as_ref() {
            Some(owner) => {
                let raw = unsafe { owner.as_borrowed_unknown() }.map_err(|e| e.to_string())?;
                Some(required(raw.as_ptr(), &UNKNOWN, guard)?)
            }
            None => None,
        };
        let pin = Self {
            observer,
            observer_canonical,
            canonical,
            parameter,
        };
        pin.check(guard, &mut Vec::new())?;
        Ok(pin)
    }
    fn check<G: FnMut() -> Result<(), String>>(
        &self,
        guard: &mut G,
        witnesses: &mut Vec<Owned>,
    ) -> Result<(), String> {
        let canonical = required(self.parameter.as_borrowed_raw().as_ptr(), &UNKNOWN, guard)?;
        let same = canonical.0 == self.canonical.0;
        witnesses.push(canonical);
        if !same {
            return Err("parameter canonical identity changed".into());
        }
        match &self.observer {
            Some(owner) => {
                owner.check(guard).map_err(|e| e.to_string())?;
                let raw = unsafe { owner.as_borrowed_unknown() }.map_err(|e| e.to_string())?;
                let canonical = required(raw.as_ptr(), &UNKNOWN, guard)?;
                let same = Some(canonical.0) == self.observer_canonical.as_ref().map(|x| x.0);
                witnesses.push(canonical);
                if !same {
                    return Err("declared observer identity changed".into());
                }
            }
            None => {
                let fresh = self
                    .parameter
                    .controller_owner(guard)
                    .map_err(|e| e.to_string())?;
                let absent = fresh.is_none();
                release(fresh, guard)?;
                if !absent {
                    return Err("declared observer appeared".into());
                }
            }
        }
        guard()
    }
    fn check_borrowed(&self) -> Result<(), String> {
        let get: Borrowed = unsafe { method(self.parameter.as_borrowed_raw().as_ptr(), 14) };
        let current = unsafe { get(self.parameter.as_borrowed_raw().as_ptr()) };
        let expected = match self.observer.as_ref() {
            Some(owner) => unsafe { owner.as_borrowed_unknown() }
                .map_err(|e| e.to_string())?
                .as_ptr(),
            None => ptr::null_mut(),
        };
        if current == expected {
            Ok(())
        } else {
            Err("declared observer association changed".into())
        }
    }
}
struct ViewPin {
    canonical: Owned,
    children: Option<Owned>,
    view: Owned,
    parent: Option<usize>,
    depth: usize,
}
struct Candidate {
    parameter: RawParameter,
    control: Owned,
    canonical: Owned,
    node: usize,
}
struct Scan {
    candidate: Option<Candidate>,
    views: Vec<ViewPin>,
    coverage: Coverage,
}
struct Compared {
    controller_probe: Option<controller_probe::ControllerProbe>,
    witnesses: RefCell<Vec<Owned>>,
    direct_controller_canonical: Owned,
    direct: ParameterPin,
    gui: ParameterPin,
    control: Owned,
    control_canonical: Owned,
    node: usize,
    views: Vec<ViewPin>,
    root: EditorRootLease,
}
pub(super) struct ComparisonRead {
    value: Value,
    pins: Option<Compared>,
}
impl ComparisonRead {
    pub(super) fn capture(
        ctx: &Context<'_>,
        direct: &Controller,
        parameter: RawParameter,
    ) -> Result<Self, String> {
        if ctx.instance.class_uid != CHORDER {
            return Err("GUI comparison requires exact Chorder class".into());
        }
        let mut guard = || {
            direct.check()?;
            ctx.check()
        };
        guard()?;
        let gui = GuiApi {
            get_desktop: ctx.api.desktop,
            get_constant_string: unsafe {
                std::mem::transmute::<
                    ConstantString,
                    unsafe extern "C" fn(*const c_char) -> *const c_void,
                >(constant_string().ok_or("native string constants unavailable")?)
            },
        };
        let owned = hold(ctx.target.0).ok_or("native target unavailable")?;
        let pointer = NonNull::new(owned.0).expect("held target");
        std::mem::forget(owned);
        let root = unsafe { fender_native_editor_api::resolve_read_only_root_owned(pointer, gui) };
        guard()?;
        let root = match root {
            Ok(root) => root,
            Err(error) => {
                release(parameter, &mut guard)?;
                let reason = match error {
                    fender_native_editor_api::Error::MissingEditor => "missing_editor",
                    fender_native_editor_api::Error::AmbiguousEditor => "ambiguous_editor",
                    _ => "inspection_failed",
                };
                return Ok(Self {
                    value: unresolved(reason, Coverage::default()),
                    pins: None,
                });
            }
        };
        let raw = unsafe { root.as_borrowed_root() }.map_err(|e| e.to_string())?;
        let mut observed_coverage = Coverage::default();
        let scan = scan(raw.as_ptr(), ctx.api, &mut observed_coverage, &mut guard);
        guard()?;
        let scan = match scan {
            Ok(scan) => scan,
            Err(failure) => {
                release((root, parameter), &mut guard)?;
                let mut value = unresolved("incomplete_walk", observed_coverage);
                value["walk_failure"] = json!(failure);
                return Ok(Self { value, pins: None });
            }
        };
        let coverage = scan.coverage;
        if coverage.candidates != 1 {
            release((scan, root, parameter), &mut guard)?;
            let reason = if coverage.candidates == 0 {
                "no_exact_named_control"
            } else {
                "ambiguous_named_controls"
            };
            return Ok(Self {
                value: unresolved(reason, coverage),
                pins: None,
            });
        }
        let candidate = scan.candidate.expect("unique candidate");
        let direct_controller_canonical = required(direct.handle.0, &UNKNOWN, &mut guard)?;
        let direct_pin = ParameterPin::capture(parameter, &mut guard)?;
        let gui = ParameterPin::capture(candidate.parameter, &mut guard)?;
        let mut value = common(coverage);
        value["status"] = json!("compared");
        value["topParameterSame"] = json!(direct_pin.canonical.0 == gui.canonical.0);
        let (observer, presence) = match (&direct_pin.observer_canonical, &gui.observer_canonical) {
            (Some(a), Some(b)) => (json!(a.0 == b.0), "both_present"),
            (None, None) => (Value::Null, "both_absent"),
            _ => (Value::Null, "one_absent"),
        };
        value["declaredObserverSame"] = observer;
        value["declaredObserverPresence"] = json!(presence);
        value["originalSame"] = Value::Null;
        value["originalBasis"] = json!("not_compared");
        value["originalUnknownReason"] = json!("not_observed");
        value["editorMatch"] = json!(match root.identity().matched_by {
            DeviceMatch::CanonicalIUnknown => "canonical_iunknown",
            DeviceMatch::StableNodeUid => "stable_node_uid",
        });
        let mut result = Self {
            value,
            pins: Some(Compared {
                controller_probe: None,
                witnesses: RefCell::new(Vec::new()),
                direct_controller_canonical,
                direct: direct_pin,
                gui,
                control: candidate.control,
                control_canonical: candidate.canonical,
                node: candidate.node,
                views: scan.views,
                root,
            }),
        };
        result.check(ctx, direct)?;
        let descriptor = result.gui_description(ctx, direct)?;
        result.value["gui_parameter"] = description(&descriptor);
        result.check(ctx, direct)?;
        Ok(result)
    }
    pub(super) fn value(&self) -> Value {
        self.value.clone()
    }
    pub(super) fn observe_controller(
        &mut self,
        ctx: &Context<'_>,
        controller: &Controller,
    ) -> Result<(), String> {
        self.check(ctx, controller)?;
        let Some(pins) = self.pins.as_mut() else {
            return Ok(());
        };
        let mut guard = || {
            controller.check()?;
            ctx.check()
        };
        let probe = controller_probe::ControllerProbe::capture(
            &pins.gui,
            pins.direct_controller_canonical.0,
            ctx.api,
            &mut guard,
        )?;
        self.value["controller_probe"] = probe.value();
        pins.controller_probe = Some(probe);
        self.check(ctx, controller)
    }
    fn gui_description(
        &self,
        ctx: &Context<'_>,
        controller: &Controller,
    ) -> Result<parameters::Description, String> {
        self.check(ctx, controller)?;
        let observed = self
            .pins
            .as_ref()
            .ok_or("GUI parameter binding is unresolved")?
            .gui
            .parameter
            .describe()
            .map_err(|error| error.to_string());
        self.check(ctx, controller)?;
        let observed = observed?;
        if observed.name != GUI_PARAMETER {
            return Err("GUI parameter name changed during observation".into());
        }
        Ok(observed)
    }
    pub(super) fn check(&self, ctx: &Context<'_>, controller: &Controller) -> Result<(), String> {
        let mut guard = || {
            controller.check()?;
            ctx.check()
        };
        let Some(pins) = &self.pins else {
            return guard();
        };
        let previous = std::mem::take(&mut *pins.witnesses.borrow_mut());
        release(previous, &mut guard)?;
        pins.root.check().map_err(|e| e.to_string())?;
        guard()?;
        let mut witnesses = Vec::new();
        pins.direct.check(&mut guard, &mut witnesses)?;
        pins.gui.check(&mut guard, &mut witnesses)?;
        let canonical = required(pins.control.0, &UNKNOWN, &mut guard)?;
        let same = canonical.0 == pins.control_canonical.0;
        witnesses.push(canonical);
        if !same {
            return Err("control canonical identity changed".into());
        }
        let lookup = controller.find(PARAMETER, ctx.api)?;
        let lookup_identity = required(lookup.as_borrowed_raw().as_ptr(), &UNKNOWN, &mut guard)?;
        let same = lookup_identity.0 == pins.direct.canonical.0;
        witnesses.push(lookup_identity);
        let owned = hold(lookup.as_borrowed_raw().as_ptr()).ok_or("direct lookup unavailable")?;
        witnesses.push(owned);
        release(lookup, &mut guard)?;
        if !same {
            return Err("direct named parameter changed".into());
        }
        pins.root.check().map_err(|e| e.to_string())?;
        guard()?;
        let attached = required(ctx.target.0, &CONTROLLER, &mut guard)?;
        let attached_identity = required(attached.0, &UNKNOWN, &mut guard)?;
        let same = attached_identity.0 == pins.direct_controller_canonical.0;
        witnesses.push(attached_identity);
        witnesses.push(attached);
        if !same {
            return Err("direct controller attachment changed".into());
        }
        let attached_control = required(pins.views[pins.node].view.0, &CONTROL, &mut guard)?;
        let attached_identity = required(attached_control.0, &UNKNOWN, &mut guard)?;
        let same = attached_identity.0 == pins.control_canonical.0;
        witnesses.push(attached_identity);
        witnesses.push(attached_control);
        if !same {
            return Err("view control attachment changed".into());
        }
        if let Some(probe) = pins.controller_probe.as_ref() {
            probe.check(&pins.gui, ctx.api, &mut guard, &mut witnesses)?;
        }
        let previous = pins.witnesses.replace(witnesses);
        release(previous, &mut guard)?;
        pins.check_borrowed(controller)?;
        pins.root.check_pinned().map_err(|e| e.to_string())?;
        pins.check_borrowed(controller)?;
        ctx.check_document()
    }
}

struct BoundChorderAutofill {
    comparison: ComparisonRead,
    direct: Controller,
}
impl BoundChorderAutofill {
    fn capture(ctx: &Context<'_>) -> Result<Self, String> {
        if ctx.instance.class_uid != CHORDER {
            return Err("GUI Auto Fill edit requires the exact Chorder class".into());
        }
        let direct = direct_controller(ctx)?;
        let parameter = direct.find(PARAMETER, ctx.api)?;
        ctx.check()?;
        let comparison = ComparisonRead::capture(ctx, &direct, parameter)?;
        let pins = comparison
            .pins
            .as_ref()
            .ok_or("GUI Auto Fill edit requires a complete unique control binding")?;
        if pins.root.identity().matched_by != DeviceMatch::CanonicalIUnknown {
            return Err("GUI Auto Fill edit requires canonical editor identity".into());
        }
        let binding = Self { comparison, direct };
        let (gui, direct) = binding.states(ctx)?;
        if gui != direct {
            return Err("GUI and direct Auto Fill Boolean values disagree".into());
        }
        binding.check(ctx)?;
        Ok(binding)
    }
    fn states(&self, ctx: &Context<'_>) -> Result<(bool, bool), String> {
        let gui = self.comparison.gui_description(ctx, &self.direct)?;
        self.comparison.check(ctx, &self.direct)?;
        let direct = self
            .comparison
            .pins
            .as_ref()
            .ok_or("GUI Auto Fill binding is unresolved")?
            .direct
            .parameter
            .describe()
            .map_err(|error| error.to_string());
        self.comparison.check(ctx, &self.direct)?;
        Ok((
            toggle_state(&gui, GUI_PARAMETER)?,
            toggle_state(&direct?, PARAMETER)?,
        ))
    }
    fn check(&self, ctx: &Context<'_>) -> Result<(), String> {
        self.comparison.check(ctx, &self.direct)?;
        ctx.check_document()
    }
    fn check_pinned(&self, ctx: &Context<'_>) -> Result<(), String> {
        let pins = self
            .comparison
            .pins
            .as_ref()
            .ok_or("GUI Auto Fill binding is unresolved")?;
        pins.check_borrowed(&self.direct)?;
        pins.root
            .check_pinned()
            .map_err(|error| error.to_string())?;
        pins.check_borrowed(&self.direct)?;
        ctx.check_document()
    }
    fn parameter(&self, ctx: &Context<'_>) -> Result<RawParameter, String> {
        self.check(ctx)?;
        let pointer = self
            .comparison
            .pins
            .as_ref()
            .ok_or("GUI Auto Fill binding is unresolved")?
            .gui
            .parameter
            .as_borrowed_raw()
            .as_ptr();
        let duplicate = retained_parameter(pointer, ctx.api);
        if let Err(reason) = self.check(ctx) {
            release(duplicate, &mut || self.check(ctx))?;
            return Err(reason);
        }
        duplicate
    }
}

fn toggle_state(value: &parameters::Description, name: &str) -> Result<bool, String> {
    let scalar = |value: Option<&parameters::NativeValue>| match value {
        Some(parameters::NativeValue {
            variant_tag,
            value: parameters::Value::Integer(number),
        }) if *variant_tag == 1 && (*number == 0 || *number == 1) => Some((*number == 1, 1)),
        Some(parameters::NativeValue {
            variant_tag,
            value: parameters::Value::Float(number),
        }) if *variant_tag == 2 && matches!(number.to_bits(), 0 | 0x3ff0_0000_0000_0000) => {
            Some((*number == 1.0, 2))
        }
        _ => None,
    };
    let raw = scalar(value.raw.as_ref());
    let min = scalar(value.min.as_ref());
    let max = scalar(value.max.as_ref());
    if value.name != name
        || value.kind != parameters::Kind::Toggle
        || !value.enabled
        || value.read_only
        || !value.support.raw_number
        || raw.is_none()
        || min != raw.map(|(_, carrier)| (false, carrier))
        || max != raw.map(|(_, carrier)| (true, carrier))
    {
        return Err("Auto Fill edit requires a complete enabled writable Toggle01 profile".into());
    }
    Ok(raw.expect("validated raw Toggle").0)
}

pub(super) struct AutofillEditLease {
    numeric: parameters::NumericEditLease,
    binding: BoundChorderAutofill,
}
impl AutofillEditLease {
    pub(super) fn phase(&self) -> &'static str {
        self.numeric.phase().as_str()
    }
    pub(super) fn began(&self) -> bool {
        self.numeric.phase().effect_may_have_started()
    }
    pub(super) fn check(&self, ctx: &Context<'_>) -> Result<(), String> {
        self.numeric
            .check(&mut || self.binding.check(ctx))
            .map_err(|error| error.to_string())?;
        self.binding.check_pinned(ctx)
    }
}

pub(super) fn edit_autofill(
    ctx: &Context<'_>,
    enabled: bool,
) -> (Value, Option<AutofillEditLease>) {
    let prepared = (|| {
        let binding = BoundChorderAutofill::capture(ctx)?;
        let parameter = binding.parameter(ctx)?;
        Ok::<_, String>((binding, parameter))
    })();
    let (binding, parameter) = match prepared {
        Ok(prepared) => prepared,
        Err(reason) => {
            return (
                error("gui_autofill_unavailable", &reason, "not_applied"),
                None,
            );
        }
    };
    let attempt = parameter.edit_raw_toggle(GUI_PARAMETER, enabled, &mut || binding.check(ctx));
    let lease = AutofillEditLease {
        numeric: attempt.lease,
        binding,
    };
    let mut value = match attempt.result {
        Ok(readback) => json!({
            "ok":true,"outcome":"applied_unverified","operation":"edit_chorder_autofill",
            "binding_source":"dsp","logical_parameter":PARAMETER,"receiver":"editor_control",
            "requested_enabled":enabled,"gui_parameter":{"name":GUI_PARAMETER,
                "before":description(&readback.before),"after":description(&readback.after)},
            "gui_value_verified":readback.verified,"verification_scope":"gui_receiver_current_value",
            "persistence_verified":false,"effects_verified":false,"dsp_propagation_verified":false
        }),
        Err(failure) => error(
            "gui_autofill_edit_failed",
            &failure.error.to_string(),
            if lease.began() || failure.outcome == parameters::Outcome::Unknown {
                "unknown"
            } else {
                "not_applied"
            },
        ),
    };
    value["edit_phase"] = json!(lease.phase());
    (value, Some(lease))
}

impl Compared {
    fn check_borrowed(&self, controller: &Controller) -> Result<(), String> {
        if control_parameter(self.control.0) != self.gui.parameter.as_borrowed_raw().as_ptr() {
            return Err("GUI parameter association changed".into());
        }
        let name = PlainCString {
            text: c"chorder.autofill".as_ptr(),
            owner: ptr::null_mut(),
        };
        let find: unsafe extern "C" fn(*mut c_void, *const PlainCString) -> *mut c_void =
            unsafe { method(controller.handle.0, 5) };
        if unsafe { find(controller.handle.0, &name) }
            != self.direct.parameter.as_borrowed_raw().as_ptr()
        {
            return Err("direct named parameter association changed".into());
        }
        self.direct.check_borrowed()?;
        self.gui.check_borrowed()?;
        if let Some(probe) = self.controller_probe.as_ref() {
            probe.check_borrowed(&self.gui)?;
        }
        for node in &self.views {
            if let Some(parent) = node.parent {
                let parent = &self.views[parent];
                let current = borrowed(parent.view.0, 20);
                if Some(current) != parent.children.as_ref().map(|x| x.0) {
                    return Err("view children changed".into());
                }
                let child: unsafe extern "C" fn(*mut c_void, *mut c_void, u8) -> u8 =
                    unsafe { method(current, 12) };
                if unsafe { child(current, node.view.0, 0) } == 0 {
                    return Err("view detached".into());
                }
            }
        }
        Ok(())
    }
}
fn borrowed(pointer: *mut c_void, slot: usize) -> *mut c_void {
    let get: Borrowed = unsafe { method(pointer, slot) };
    unsafe { get(pointer) }
}
fn control_parameter(pointer: *mut c_void) -> *mut c_void {
    // Public CCL5.1.2 compiler probe abi_control_parameter loads byte24.
    borrowed(pointer, 3)
}
fn required<G: FnMut() -> Result<(), String>>(
    pointer: *mut c_void,
    iid: &Uid,
    guard: &mut G,
) -> Result<Owned, String> {
    optional(pointer, iid, guard)?.ok_or("required interface unavailable".into())
}
fn optional<G: FnMut() -> Result<(), String>>(
    pointer: *mut c_void,
    iid: &Uid,
    guard: &mut G,
) -> Result<Option<Owned>, String> {
    query(pointer, iid, guard).map_err(|error| match error {
        QueryFailure::Guard(reason) => reason,
        QueryFailure::Native(_) => "GUI interface query failed".into(),
    })
}
enum QueryFailure {
    Guard(String),
    Native(i32),
}
fn query<G: FnMut() -> Result<(), String>>(
    pointer: *mut c_void,
    iid: &Uid,
    guard: &mut G,
) -> Result<Option<Owned>, QueryFailure> {
    guard().map_err(QueryFailure::Guard)?;
    let mut output = ptr::null_mut();
    let status = unsafe { (unknown_vtable(pointer).query)(pointer, iid, &mut output) };
    let owned = (status == OK && !output.is_null()).then(|| Owned(output));
    if let Err(reason) = guard() {
        release(owned, guard).map_err(QueryFailure::Guard)?;
        return Err(QueryFailure::Guard(reason));
    }
    match (status, owned) {
        (OK, Some(owned)) => Ok(Some(owned)),
        (NO_INTERFACE, None) if output.is_null() => Ok(None),
        (_, owned) => {
            release(owned, guard).map_err(QueryFailure::Guard)?;
            Err(QueryFailure::Native(status))
        }
    }
}
fn walk_required<G: FnMut() -> Result<(), String>>(
    pointer: *mut c_void,
    iid: &Uid,
    stage: WalkStage,
    depth: usize,
    guard: &mut G,
) -> Result<Owned, WalkFailure> {
    query(pointer, iid, guard)
        .map_err(|error| WalkFailure::query(error, stage, depth))?
        .ok_or(WalkFailure {
            kind: WalkKind::MissingInterface,
            stage,
            depth,
            query_status: Some(NO_INTERFACE),
        })
}
fn release<T, G: FnMut() -> Result<(), String>>(value: T, guard: &mut G) -> Result<(), String> {
    let before = guard();
    drop(value);
    let after = guard();
    before.and(after)
}
fn retained_parameter(pointer: *mut c_void, api: &Api) -> Result<RawParameter, String> {
    let owned = hold(pointer).ok_or("GUI parameter unavailable")?;
    let raw = NonNull::new(owned.0).expect("held parameter");
    std::mem::forget(owned);
    Ok(unsafe { RawParameter::from_owned(raw, raw_text(api)) })
}
fn scan<G: FnMut() -> Result<(), String>>(
    root: *mut c_void,
    api: &Api,
    coverage: &mut Coverage,
    guard: &mut G,
) -> Result<Scan, WalkFailure> {
    let mut views = Vec::<ViewPin>::new();
    let mut candidate = None;
    let result = (|| -> Result<(), WalkFailure> {
        let view = hold(root).ok_or(WalkFailure::new(WalkKind::MissingView, WalkStage::Root, 0))?;
        guard().map_err(|_| WalkFailure::context(WalkStage::Root, 0))?;
        let canonical = walk_required(view.0, &UNKNOWN, WalkStage::Root, 0, guard)?;
        views.push(ViewPin {
            canonical,
            children: None,
            view,
            parent: None,
            depth: 0,
        });
        coverage.views = 1;
        let mut index = 0;
        while index < views.len() {
            let depth = views[index].depth;
            guard().map_err(|_| WalkFailure::context(WalkStage::Context, depth))?;
            if let Some(control) = query(views[index].view.0, &CONTROL, guard)
                .map_err(|error| WalkFailure::query(error, WalkStage::ControlQuery, depth))?
            {
                let raw = control_parameter(control.0);
                let parameter = if raw.is_null() {
                    None
                } else {
                    Some(retained_parameter(raw, api).map_err(|_| {
                        WalkFailure::new(
                            WalkKind::InspectionFailed,
                            WalkStage::ParameterMetadata,
                            depth,
                        )
                    })?)
                };
                guard().map_err(|_| WalkFailure::context(WalkStage::ParameterMetadata, depth))?;
                if let Some(parameter) = parameter {
                    let description = parameter.describe();
                    guard()
                        .map_err(|_| WalkFailure::context(WalkStage::ParameterMetadata, depth))?;
                    let matches = description
                        .map_err(|_| {
                            WalkFailure::new(
                                WalkKind::ParameterMetadata,
                                WalkStage::ParameterMetadata,
                                depth,
                            )
                        })?
                        .name
                        == GUI_PARAMETER;
                    if matches {
                        coverage.candidates += 1;
                        let canonical = walk_required(
                            control.0,
                            &UNKNOWN,
                            WalkStage::ControlIdentity,
                            depth,
                            guard,
                        )?;
                        if candidate.is_none() {
                            candidate = Some(Candidate {
                                control,
                                canonical,
                                parameter,
                                node: index,
                            });
                        } else {
                            release((control, canonical, parameter), guard)
                                .map_err(|_| WalkFailure::context(WalkStage::Cleanup, depth))?;
                        }
                    } else {
                        release((control, parameter), guard)
                            .map_err(|_| WalkFailure::context(WalkStage::Cleanup, depth))?;
                    }
                } else {
                    release(control, guard)
                        .map_err(|_| WalkFailure::context(WalkStage::Cleanup, depth))?;
                }
            }
            let children = borrowed(views[index].view.0, 20);
            let children = hold(children);
            guard().map_err(|_| WalkFailure::context(WalkStage::Children, depth))?;
            if let Some(children) = children {
                let pointer = borrowed(children.0, 11);
                let iterator =
                    NonNull::new(pointer)
                        .map(|p| Owned(p.as_ptr()))
                        .ok_or(WalkFailure::new(
                            WalkKind::MissingIterator,
                            WalkStage::Iterator,
                            depth,
                        ))?;
                guard().map_err(|_| WalkFailure::context(WalkStage::Iterator, depth))?;
                let done: unsafe extern "C" fn(*mut c_void) -> u8 =
                    unsafe { method(iterator.0, 3) };
                while unsafe { done(iterator.0) } == 0 {
                    guard().map_err(|_| WalkFailure::context(WalkStage::Iterator, depth))?;
                    if coverage.views >= MAX_VIEWS {
                        return Err(WalkFailure::new(
                            WalkKind::ViewLimit,
                            WalkStage::Children,
                            depth,
                        ));
                    }
                    if depth >= MAX_DEPTH {
                        return Err(WalkFailure::new(
                            WalkKind::DepthLimit,
                            WalkStage::Children,
                            depth,
                        ));
                    }
                    let raw = borrowed(iterator.0, 4);
                    let child_depth = depth + 1;
                    let child = hold(raw).ok_or(WalkFailure::new(
                        WalkKind::MissingView,
                        WalkStage::Iterator,
                        child_depth,
                    ))?;
                    coverage.views += 1;
                    guard()
                        .map_err(|_| WalkFailure::context(WalkStage::ChildQuery, child_depth))?;
                    let view =
                        walk_required(child.0, &VIEW, WalkStage::ChildQuery, child_depth, guard)?;
                    let canonical = walk_required(
                        view.0,
                        &UNKNOWN,
                        WalkStage::ViewIdentity,
                        child_depth,
                        guard,
                    )?;
                    if views.iter().any(|p| p.canonical.0 == canonical.0) {
                        return Err(WalkFailure::new(
                            WalkKind::RepeatedView,
                            WalkStage::ViewIdentity,
                            child_depth,
                        ));
                    }
                    let edge: unsafe extern "C" fn(*mut c_void, *mut c_void, u8) -> u8 =
                        unsafe { method(children.0, 12) };
                    if unsafe { edge(children.0, view.0, 0) } == 0 {
                        return Err(WalkFailure::new(
                            WalkKind::DetachedView,
                            WalkStage::Edge,
                            child_depth,
                        ));
                    }
                    guard().map_err(|_| WalkFailure::context(WalkStage::Edge, child_depth))?;
                    views.push(ViewPin {
                        canonical,
                        children: None,
                        view,
                        parent: Some(index),
                        depth: child_depth,
                    });
                    release(child, guard)
                        .map_err(|_| WalkFailure::context(WalkStage::Cleanup, child_depth))?;
                }
                release(iterator, guard)
                    .map_err(|_| WalkFailure::context(WalkStage::Cleanup, depth))?;
                views[index].children = Some(children);
            }
            index += 1;
        }
        coverage.complete = true;
        guard().map_err(|_| WalkFailure::context(WalkStage::Context, 0))
    })();
    match result {
        Ok(()) => Ok(Scan {
            views,
            candidate,
            coverage: *coverage,
        }),
        Err(failure) => {
            coverage.complete = false;
            let _ = release((candidate, views), guard);
            Err(failure)
        }
    }
}

#[cfg(test)]
#[path = "gui_parameter_tests.rs"]
mod tests;

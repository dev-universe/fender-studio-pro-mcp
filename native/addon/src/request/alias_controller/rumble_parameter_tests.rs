use super::*;
use std::cell::{Cell, RefCell};

#[repr(C)]
struct Fake {
    table: *const *const c_void,
    refs: Cell<u32>,
    canonical: Cell<*mut c_void>,
    controller: Cell<*mut c_void>,
    status: Cell<i32>,
    node: bool,
    name: PlainString,
    units: Vec<u16>,
    parent: Cell<*mut c_void>,
    children: RefCell<Vec<*mut c_void>>,
    count: Cell<i32>,
    parameter: Cell<*mut c_void>,
    owner: Cell<*mut c_void>,
    object: Cell<*mut c_void>,
    kind: Cell<i32>,
    indexed_calls: Cell<usize>,
    find_calls: Cell<usize>,
    materialize: Cell<bool>,
}
unsafe fn fake<'a>(p: *mut c_void) -> &'a Fake {
    unsafe { &*p.cast::<Fake>() }
}
unsafe extern "C" fn retain(p: *mut c_void) -> u32 {
    let f = unsafe { fake(p) };
    f.refs.set(f.refs.get() + 1);
    f.refs.get()
}
unsafe extern "C" fn drop_ref(p: *mut c_void) -> u32 {
    let f = unsafe { fake(p) };
    f.refs.set(f.refs.get() - 1);
    f.refs.get()
}
unsafe extern "C" fn query(p: *mut c_void, id: *const Uid, out: *mut *mut c_void) -> i32 {
    let f = unsafe { fake(p) };
    let id = unsafe { *id };
    let (status, pointer) = if id == UNKNOWN {
        (OK, f.canonical.get())
    } else if id == NODE && f.node {
        (OK, p)
    } else if id == OBJECT && !f.object.get().is_null() {
        (OK, f.object.get())
    } else if id == CONTROLLER {
        (f.status.get(), f.controller.get())
    } else {
        (NO_INTERFACE, ptr::null_mut())
    };
    unsafe {
        *out = pointer;
        if !pointer.is_null() {
            retain(pointer);
        }
    }
    status
}
unsafe extern "C" fn id(p: *mut c_void) -> *const PlainString {
    unsafe { ptr::addr_of!(fake(p).name) }
}
unsafe extern "C" fn length(p: *mut c_void) -> i32 {
    unsafe { fake(p) }.units.len() as i32
}
unsafe extern "C" fn copy(p: *mut c_void, out: *mut u16, capacity: i32) -> i32 {
    let f = unsafe { fake(p) };
    assert!(capacity as usize > f.units.len());
    unsafe {
        ptr::copy_nonoverlapping(f.units.as_ptr(), out, f.units.len());
        *out.add(f.units.len()) = 0;
    }
    OK
}
unsafe extern "C" fn parent(p: *mut c_void) -> *mut c_void {
    unsafe { fake(p) }.parent.get()
}
unsafe extern "C" fn children(p: *mut c_void) -> i32 {
    unsafe { fake(p) }.children.borrow().len() as i32
}
unsafe extern "C" fn child(p: *mut c_void, index: i32) -> *mut c_void {
    unsafe { fake(p) }
        .children
        .borrow()
        .get(index as usize)
        .copied()
        .unwrap_or(ptr::null_mut())
}
unsafe extern "C" fn parameter_count(p: *mut c_void) -> i32 {
    unsafe { fake(p) }.count.get()
}
unsafe extern "C" fn parameter_at(p: *mut c_void, _: i32) -> *mut c_void {
    let f = unsafe { fake(p) };
    f.indexed_calls.set(f.indexed_calls.get() + 1);
    f.parameter.get()
}
unsafe extern "C" fn parameter_find(p: *mut c_void, name: *const PlainCString) -> *mut c_void {
    let f = unsafe { fake(p) };
    f.find_calls.set(f.find_calls.get() + 1);
    if f.materialize.get() {
        f.count.set(1);
    }
    let name = unsafe { std::ffi::CStr::from_ptr((*name).text) }.to_bytes();
    if name == b"gain" {
        f.parameter.get()
    } else {
        ptr::null_mut()
    }
}
unsafe extern "C" fn parameter_owner(p: *mut c_void) -> *mut c_void {
    unsafe { fake(p) }.owner.get()
}
unsafe extern "C" fn unknown_kind(p: *mut c_void) -> i32 {
    unsafe { fake(p) }.kind.get()
}
unsafe extern "C" fn state_flag(_: *mut c_void, _: i32) -> u8 {
    0
}
unsafe extern "C" fn normalized(_: *mut c_void) -> f32 {
    0.5
}
unsafe extern "C" fn absent(_: *mut c_void) -> *mut c_void {
    ptr::null_mut()
}
unsafe extern "C" fn display(_: *mut c_void, _: *mut PlainString) {}
unsafe extern "C" fn zero(_: *mut c_void) -> i32 {
    0
}
unsafe extern "C" fn yes(_: *mut c_void) -> u8 {
    1
}
#[repr(C)]
struct CString {
    bytes: *const c_char,
    owner: *mut c_void,
}
unsafe impl Sync for CString {}
unsafe extern "C" fn parameter_name(_: *mut c_void) -> *const CString {
    static NAME: CString = CString {
        bytes: c"gain".as_ptr(),
        owner: ptr::null_mut(),
    };
    &NAME
}
unsafe extern "C" fn original(p: *mut c_void) -> *mut c_void {
    p
}
struct Graph {
    #[allow(
        clippy::vec_box,
        reason = "Native mock pointers must stay stable while the vector grows"
    )]
    objects: Vec<Box<Fake>>,
    tables: [Box<[*const c_void; 46]>; 5],
    root: *mut c_void,
    nodes: Vec<*mut c_void>,
    controllers: Vec<*mut c_void>,
}
impl Graph {
    fn new() -> Self {
        let mut node = Box::new([ptr::null(); 46]);
        node[0] = query as *const c_void;
        node[1] = retain as *const c_void;
        node[2] = drop_ref as *const c_void;
        node[3] = id as *const c_void;
        node[6] = parent as *const c_void;
        node[8] = children as *const c_void;
        node[9] = child as *const c_void;
        let mut controller = node.clone();
        controller[3] = parameter_count as *const c_void;
        controller[4] = parameter_at as *const c_void;
        controller[5] = parameter_find as *const c_void;
        let mut string = node.clone();
        string[4] = length as *const c_void;
        string[8] = copy as *const c_void;
        let mut parameter = node.clone();
        parameter[3] = unknown_kind as *const c_void;
        parameter[4] = parameter_name as *const c_void;
        parameter[6] = yes as *const c_void;
        parameter[8] = state_flag as *const c_void;
        parameter[13] = zero as *const c_void;
        parameter[14] = parameter_owner as *const c_void;
        parameter[18] = original as *const c_void;
        parameter[35] = normalized as *const c_void;
        parameter[41] = absent as *const c_void;
        parameter[44] = display as *const c_void;
        string[21] = absent as *const c_void;
        let mut object = controller.clone();
        object[4] = property as *const c_void;
        let mut g = Self {
            objects: Vec::new(),
            tables: [node, controller, string, parameter, object],
            root: ptr::null_mut(),
            nodes: Vec::new(),
            controllers: Vec::new(),
        };
        g.root = g.make(0, "root");
        for i in 0..26 {
            let string = g.make(2, &slot(i));
            let node = g.make(0, "");
            let controller = g.make(1, "");
            unsafe {
                (*node.cast::<Fake>()).name.object = string;
                fake(node).controller.set(controller);
                fake(controller).canonical.set(node);
                fake(node).parent.set(g.root);
                fake(g.root).children.borrow_mut().push(node);
            }
            g.nodes.push(node);
            g.controllers.push(controller);
        }
        g
    }
    fn make(&mut self, table: usize, text: &str) -> *mut c_void {
        let mut value = Box::new(Fake {
            table: self.tables[table].as_ptr(),
            refs: Cell::new(1),
            canonical: Cell::new(ptr::null_mut()),
            controller: Cell::new(ptr::null_mut()),
            status: Cell::new(OK),
            node: table == 0,
            name: PlainString {
                object: ptr::null_mut(),
                opaque_debug: ptr::null_mut(),
            },
            units: text.encode_utf16().collect(),
            parent: Cell::new(ptr::null_mut()),
            children: RefCell::new(Vec::new()),
            count: Cell::new(0),
            parameter: Cell::new(ptr::null_mut()),
            owner: Cell::new(ptr::null_mut()),
            object: Cell::new(ptr::null_mut()),
            kind: Cell::new(11),
            indexed_calls: Cell::new(0),
            find_calls: Cell::new(0),
            materialize: Cell::new(false),
        });
        let pointer = ptr::addr_of_mut!(*value).cast();
        value.canonical.set(pointer);
        self.objects.push(value);
        pointer
    }
    fn parameter(&mut self, index: usize) -> *mut c_void {
        let p = self.make(3, "");
        unsafe {
            fake(p).owner.set(self.controllers[index]);
            fake(self.controllers[index]).count.set(1);
            fake(self.controllers[index]).parameter.set(p);
        }
        p
    }
}
impl Drop for Graph {
    fn drop(&mut self) {
        for f in &self.objects {
            assert_eq!(f.refs.get(), 1);
        }
    }
}
fn context(host: &mut state::tests::Fixture) -> Context<'static> {
    let mut ctx = host.context();
    ctx.instance.native_class = "AudioEffect".into();
    ctx.instance.object_id = "Rumble Native".into();
    ctx
}
fn reply(ctx: Context<'static>, value: Value, observation: Observation) -> Reply {
    let mut r = Reply::unbound(value);
    r.context = Some(Box::new(ctx));
    r.provider = Some(Provider::RumbleParameters {
        retrieval: observation.retrieval,
        observation: Some(Box::new(observation)),
    });
    r
}

#[test]
fn fixed26_zero_census_is_present_and_retains_no_write_authority() {
    let _serial = super::super::super::super::tests::TEST_LOCK.lock().unwrap();
    let graph = Graph::new();
    let mut host = state::tests::rumble_parameter_host(graph.root);
    let ctx = context(&mut host);
    let (value, observation) = observe(&ctx).unwrap();
    assert!(valid_receipt(&json!(ctx.instance), &value));
    assert_eq!(value["controllers"].as_array().unwrap().len(), 26);
    assert_eq!(value["reported_parameter_count"], 0);
    assert_eq!(value["census_complete"], true);
    assert_eq!(value["parameter_lists_complete"], true);
    assert_eq!(value["serialized_name_coverage_complete"], false);
    assert!(
        value["controllers"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v["status"] == "present"
                && v["parameter_count"] == 0
                && v["parameters"] == json!([]))
    );
    observation.check(&ctx).unwrap();
    host.parameter_no_writes();
    drop(observation);
    drop(ctx);
}
#[test]
fn positive_census_and_null_index_remain_explicitly_incomplete() {
    let _serial = super::super::super::super::tests::TEST_LOCK.lock().unwrap();
    let graph = Graph::new();
    unsafe {
        fake(graph.controllers[0]).count.set(2);
    }
    let mut host = state::tests::rumble_parameter_host(graph.root);
    let ctx = context(&mut host);
    let (value, observation) = observe(&ctx).unwrap();
    assert_eq!(value["reported_parameter_count"], 2);
    assert_eq!(
        value["controllers"][0]["parameters"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        value["controllers"][0]["parameters"][1]["status"],
        "parameter_absent"
    );
    assert_eq!(value["census_complete"], true);
    assert_eq!(value["parameter_lists_complete"], false);
    host.parameter_no_writes();
    drop(observation);
    drop(ctx);
}
#[test]
fn total_limit_preserves_census_and_returns_no_truncated_descriptors() {
    let _serial = super::super::super::super::tests::TEST_LOCK.lock().unwrap();
    let graph = Graph::new();
    for p in graph.controllers.iter().take(3) {
        unsafe { fake(*p) }.count.set(128);
    }
    let mut host = state::tests::rumble_parameter_host(graph.root);
    let ctx = context(&mut host);
    let (value, observation) = observe(&ctx).unwrap();
    assert_eq!(value["reported_parameter_count"], 384);
    assert_eq!(value["enumerated_parameter_count"], 0);
    assert_eq!(value["descriptor_budget_exceeded"], true);
    assert_eq!(value["census_complete"], true);
    assert_eq!(value["parameter_lists_complete"], false);
    assert!(
        value["controllers"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v["parameters"].is_null()
                && v["enumeration_status"] == "descriptor_total_limit")
    );
    host.parameter_no_writes();
    drop(observation);
    drop(ctx);
}
#[test]
fn qi_status_algebra_distinguishes_zero_absence_failures_and_counts() {
    let graph = Graph::new();
    let node = graph.nodes[0];
    let controller = graph.controllers[0];
    for (status, out, n, expected) in [
        (OK, true, 0, "present"),
        (NO_INTERFACE, false, 0, "controller_absent"),
        (OK, false, 0, "malformed_success"),
        (NO_INTERFACE, true, 0, "malformed_absence"),
        (FAILED, false, 0, "qi_failed"),
        (OK, true, -1, "negative_count"),
        (OK, true, 129, "count_limit"),
    ] {
        unsafe {
            fake(node).status.set(status);
            fake(node)
                .controller
                .set(if out { controller } else { ptr::null_mut() });
            fake(controller).count.set(n);
        }
        let observed = capability(node, node, &mut || Ok(())).unwrap();
        assert_eq!(observed.status, expected);
        assert_eq!(
            observed.count,
            if expected == "present" { Some(n) } else { None }
        );
        drop(observed);
    }
    unsafe {
        fake(node).status.set(OK);
        fake(node).controller.set(controller);
        fake(controller).canonical.set(graph.root);
    }
    assert_eq!(
        capability(node, node, &mut || Ok(())).unwrap().status,
        "identity_mismatch"
    );
}
#[test]
fn direct_membership_duplicates_missing_and_count_drift_are_guarded() {
    let _serial = super::super::super::super::tests::TEST_LOCK.lock().unwrap();
    for change in 0..4 {
        let graph = Graph::new();
        let mut host = state::tests::rumble_parameter_host(graph.root);
        let ctx = context(&mut host);
        let (value, observation) = observe(&ctx).unwrap();
        assert_eq!(value["census_complete"], true);
        match change {
            0 => unsafe { fake(graph.root).children.borrow_mut().swap(0, 1) },
            1 => unsafe { fake(graph.controllers[0]).count.set(1) },
            2 => unsafe { fake(graph.nodes[0]).parent.set(graph.nodes[1]) },
            _ => unsafe { fake(graph.controllers[0]).canonical.set(graph.root) },
        }
        assert!(observation.check(&ctx).is_err());
        drop(observation);
        drop(ctx);
    }
    let graph = Graph::new();
    unsafe {
        fake(graph.root).children.borrow_mut().push(graph.nodes[0]);
    }
    let mut host = state::tests::rumble_parameter_host(graph.root);
    let ctx = context(&mut host);
    assert!(observe(&ctx).is_err());
    drop(ctx);
    drop(host);
    drop(graph);
    let graph = Graph::new();
    unsafe {
        fake(graph.root).children.borrow_mut().pop();
    }
    let mut host = state::tests::rumble_parameter_host(graph.root);
    let ctx = context(&mut host);
    let (value, observation) = observe(&ctx).unwrap();
    assert_eq!(value["controllers"][25]["status"], "node_absent");
    drop(observation);
    drop(ctx);
}
#[test]
fn failed_description_retains_parameter_owner_and_detects_index_and_owner_drift() {
    let _serial = super::super::super::super::tests::TEST_LOCK.lock().unwrap();
    for change in 0..3 {
        let mut graph = Graph::new();
        let p = graph.parameter(0);
        let mut host = state::tests::rumble_parameter_host(graph.root);
        let ctx = context(&mut host);
        let (value, observation) = observe(&ctx).unwrap();
        assert_eq!(
            value["controllers"][0]["parameters"][0]["status"],
            "description_unavailable"
        );
        match change {
            0 => unsafe { fake(graph.controllers[0]).parameter.set(ptr::null_mut()) },
            1 => unsafe { fake(p).owner.set(graph.controllers[1]) },
            _ => unsafe { fake(graph.controllers[0]).count.set(0) },
        }
        assert!(observation.check(&ctx).is_err());
        host.parameter_no_writes();
        drop(observation);
        drop(ctx);
    }
}
#[test]
fn late_document_model_and_forged_operation_erase_owned_provider_evidence() {
    let _serial = super::super::super::super::tests::TEST_LOCK.lock().unwrap();
    for change in 0..3 {
        let graph = Graph::new();
        let mut host = state::tests::rumble_parameter_host(graph.root);
        let ctx = context(&mut host);
        let (value, observation) = observe(&ctx).unwrap();
        let service = ctx.service;
        let mut r = reply(ctx, value, observation);
        match change {
            0 => host.parameter_detach_document(),
            1 => {
                let mut value = state::tests::rumble_parameter_fixture();
                value["parameters"]["amp"]["__classid"] =
                    json!("11111111-1111-4111-8111-111111111111");
                host.parameter_state(value);
            }
            _ => r.value["operation"] = json!("forged"),
        }
        assert!(!r.final_check(service));
        assert_eq!(r.value, failure());
        assert!(r.value.get("controllers").is_none());
        host.parameter_no_writes();
        drop(r);
    }
}
fn description() -> parameters::Description {
    parameters::Description {
        name: "gain".into(),
        tag: 7,
        kind: parameters::Kind::Float,
        enabled: true,
        read_only: false,
        raw: Some(parameters::NativeValue {
            variant_tag: 2,
            value: parameters::Value::Integer(i64::MAX),
        }),
        min: Some(parameters::NativeValue {
            variant_tag: 2,
            value: parameters::Value::Integer(i64::MIN),
        }),
        max: Some(parameters::NativeValue {
            variant_tag: 3,
            value: parameters::Value::Float(1.0),
        }),
        default_value: Some(parameters::NativeValue {
            variant_tag: 3,
            value: parameters::Value::Float(-0.0),
        }),
        normalized: parameters::Normalized::Finite(0.5),
        display: "SYNTHETIC_PRIVATE_DISPLAY".into(),
        display_parser: parameters::DisplayParser::NativeFormatter { flags: 0 },
        support: parameters::Support {
            raw_number: true,
            normalized: true,
            display_text: true,
        },
    }
}
fn wire() -> (Value, Value) {
    let instance = json!({"object_uid":"12345678-1234-1234-1234-123456789ABC","class_uid":"B9D7536C-A010-457D-A182-36E5531164C9","native_class":"AudioEffect","object_id":"Rumble Native"});
    let classes = vec!["00000000-0000-0000-0000-000000000000".to_owned(); 26];
    let mut rows:Vec<Value>=(0..26).map(|i|json!({"slot":slot(i),"serialized_class_uid":classes[i],"status":"present","parameter_count":0,"enumeration_status":"complete","parameters":[],"serialized_name_count":0,"matched_serialized_name_count":0})).collect();
    rows[0]["parameter_count"] = json!(1);
    rows[0]["parameters"] =
        json!([{"index":0,"status":"observed","description":project(&description()).unwrap()}]);
    let value = json!({"ok":true,"outcome":"observed","operation":"rumble_parameters_read","schema":"rumble_controller_parameters_v1","instance":instance,"source":"component_alias_fixed_controllers","scope":"fixed_immediate_controller_parameter_lists","controllers":rows,"models_sha256":state::rumble_model_digest(&classes).unwrap(),"before_after_model_ids_equal":true,"census_complete":true,"parameter_lists_complete":true,"access_metadata_complete":true,"current_values_complete":true,"serialized_name_coverage_complete":true,"reported_parameter_count":1,"enumerated_parameter_count":1,"descriptor_budget_exceeded":false,"value_semantics":"sequential_snapshot","mutation_capability":false,"can_set_via_this_action":false,"selection_authority":false,"atomic_snapshot":false,"audio_verified":false,"persistence_verified":false});
    (instance, value)
}
#[test]
fn numeric_projection_preserves_precision_and_excludes_private_payloads() {
    let mut d = description();
    let v = project(&d).unwrap();
    assert!(valid_description(&v));
    assert_eq!(v["current"]["decimal"], i64::MAX.to_string());
    assert_eq!(v["min"]["decimal"], i64::MIN.to_string());
    assert_eq!(v["default"]["bits_hex"], "8000000000000000");
    assert_eq!(v["normalized"]["bits_hex"], "3f000000");
    assert!(!v.to_string().contains("PRIVATE"));
    for payload in [
        parameters::Value::Text("SYNTHETIC_PRIVATE_RAW".into()),
        parameters::Value::OpaqueObject { present: true },
        parameters::Value::Unsupported { kind: 98 },
        parameters::Value::Empty,
        parameters::Value::NonFiniteFloat { bits: u64::MAX },
        parameters::Value::Float(f64::INFINITY),
    ] {
        d.raw = Some(parameters::NativeValue {
            variant_tag: -17,
            value: payload,
        });
        let v = project(&d).unwrap();
        assert!(valid_description(&v));
        assert_eq!(v["current"]["status"], "unavailable");
        assert_eq!(v["support"], json!({"raw_number":false,"normalized":false}));
        assert!(!v.to_string().contains("PRIVATE"));
    }
    d = description();
    d.kind = parameters::Kind::Unknown(-99);
    let v = project(&d).unwrap();
    assert_eq!(v["current"]["status"], "observed");
    assert_eq!(v["support"]["raw_number"], false);
    for (enabled, read_only) in [(false, false), (true, true)] {
        d = description();
        d.enabled = enabled;
        d.read_only = read_only;
        assert_eq!(
            project(&d).unwrap()["support"],
            json!({"raw_number":false,"normalized":false})
        );
    }
    d = description();
    d.name = "가".repeat(42);
    assert!(project(&d).is_some());
    d.name = "가".repeat(43);
    assert!(project(&d).is_none());
    d.name = "bad\0name".into();
    assert!(project(&d).is_none());
}
#[test]
fn numeric_wire_rejects_unsafe_or_noncanonical_carriers_and_unknown_support() {
    for value in [
        json!({"status":"observed","variant_tag":2,"kind":"integer","decimal":"9223372036854775808"}),
        json!({"status":"observed","variant_tag":2,"kind":"integer","decimal":"-0"}),
        json!({"status":"observed","variant_tag":2,"kind":"integer","decimal":"01"}),
        json!({"status":"observed","variant_tag":3,"kind":"float64","bits_hex":"7ff0000000000000"}),
        json!({"status":"observed","variant_tag":32768,"kind":"integer","decimal":"1"}),
        json!({"status":"unavailable","variant_tag":1,"reason":"missing"}),
    ] {
        assert!(!valid_numeric(&value));
    }
    let mut v = project(&description()).unwrap();
    v["kind"] = json!(99);
    assert!(!valid_description(&v));
    v["support"] = json!({"raw_number":false,"normalized":false});
    assert!(valid_description(&v));
    v["normalized"] = json!({"status":"observed","bits_hex":"7fc00000"});
    assert!(!valid_description(&v));
}
#[test]
fn closed_receipt_derives_completeness_and_independently_recomputes_models() {
    let (instance, base) = wire();
    assert!(valid_receipt(&instance, &base));
    for field in ["operation", "schema", "source", "scope", "models_sha256"] {
        let mut v = base.clone();
        v[field] = json!("forged");
        assert!(!valid_receipt(&instance, &v), "{field}");
    }
    for field in [
        "mutation_capability",
        "can_set_via_this_action",
        "selection_authority",
        "atomic_snapshot",
        "audio_verified",
        "persistence_verified",
    ] {
        let mut v = base.clone();
        v[field] = json!(true);
        assert!(!valid_receipt(&instance, &v), "{field}");
    }
    for field in [
        "census_complete",
        "parameter_lists_complete",
        "access_metadata_complete",
        "current_values_complete",
        "serialized_name_coverage_complete",
    ] {
        let mut v = base.clone();
        v[field] = json!(false);
        assert!(!valid_receipt(&instance, &v), "{field}");
    }
    for pointer in [
        "/controllers/0/extra",
        "/controllers/0/parameters/0/extra",
        "/controllers/0/parameters/0/description/extra",
    ] {
        let mut v = base.clone();
        let parent = pointer.rsplit_once('/').unwrap().0;
        v.pointer_mut(parent).unwrap()["extra"] = json!("private");
        assert!(!valid_receipt(&instance, &v));
    }
    let mut v = base.clone();
    v["controllers"][0]["parameters"][0] =
        json!({"index":0,"status":"parameter_absent","description":null});
    v["enumerated_parameter_count"] = json!(0);
    v["parameter_lists_complete"] = json!(false);
    v["access_metadata_complete"] = json!(false);
    v["current_values_complete"] = json!(false);
    v["serialized_name_coverage_complete"] = json!(false);
    v["controllers"][0]["enumeration_status"] = json!("incomplete");
    assert!(valid_receipt(&instance, &v));
    v["parameter_lists_complete"] = json!(true);
    assert!(!valid_receipt(&instance, &v));
}
#[test]
fn output_budget_is_on_adapted_utf8_and_native_request_refuses_write_inputs() {
    let (instance, mut v) = wire();
    let description = project(&description()).unwrap();
    let records: Vec<Value> = (0..128)
        .map(|i| {
            let mut d = description.clone();
            d["name"] = json!("x".repeat(128));
            json!({"index":i,"status":"observed","description":d})
        })
        .collect();
    for i in 0..2 {
        v["controllers"][i]["parameter_count"] = json!(128);
        v["controllers"][i]["parameters"] = json!(records);
    }
    v["reported_parameter_count"] = json!(256);
    v["enumerated_parameter_count"] = json!(256);
    assert!(valid_receipt(&instance, &v));
    assert!(v.to_string().len() < MAX_BYTES);
    for i in 0..2 {
        for p in v["controllers"][i]["parameters"].as_array_mut().unwrap() {
            p["description"]["name"] = json!("\u{1}".repeat(128));
        }
    }
    assert!(v.to_string().len() > MAX_BYTES);
    assert!(!valid_receipt(&instance, &v));
    let base = json!({"operation":"rumble_parameters_read","expected_instance":"12345678-1234-1234-1234-123456789ABC"});
    assert!(
        serde_json::from_value::<Request>(base.clone())
            .unwrap()
            .validate()
            .is_ok()
    );
    for field in [
        "value", "source", "token", "path", "bytes", "selector", "script",
    ] {
        let mut v = base.clone();
        v[field] = json!("unchecked");
        assert!(serde_json::from_value::<Request>(v).is_err());
    }
}
#[test]
fn transient_context_loss_during_canonical_qi_is_not_published_as_partial_census() {
    let graph = Graph::new();
    let calls = Cell::new(0);
    let mut guard = || {
        calls.set(calls.get() + 1);
        if calls.get() == 3 {
            Err("transient context loss".into())
        } else {
            Ok(())
        }
    };
    assert!(capability(graph.nodes[0], graph.nodes[0], &mut guard).is_err());
}

#[test]
fn trusted_failure_policy_erases_forged_partial_data_without_an_observation_lease() {
    for via_encode in [false, true] {
        let mut reply = Reply::unbound(
            json!({"ok":true,"operation":"forged","schema":"forged","controllers":[{"private":"SYNTHETIC_PRIVATE"}],"census_complete":true}),
        );
        reply.provider = Some(Provider::RumbleParameters {
            retrieval: Retrieval::Indexed,
            observation: None,
        });
        if via_encode {
            assert_eq!(
                serde_json::from_str::<Value>(&reply.encode()).unwrap(),
                failure()
            );
        } else {
            reply.invalidate("SYNTHETIC_PRIVATE_CONTEXT".into());
            assert_eq!(reply.value, failure());
        }
    }
}
#[test]
fn an_initial_null_parameter_is_a_retained_absence_witness() {
    let _serial = super::super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut graph = Graph::new();
    unsafe {
        fake(graph.controllers[0]).count.set(1);
    }
    let mut host = state::tests::rumble_parameter_host(graph.root);
    let ctx = context(&mut host);
    let (_, observation) = observe(&ctx).unwrap();
    let p = graph.make(3, "");
    unsafe {
        fake(graph.controllers[0]).parameter.set(p);
    }
    assert!(observation.check(&ctx).is_err());
    host.parameter_no_writes();
    drop(observation);
    drop(ctx);
}

#[repr(C)]
struct Variant {
    tag: i16,
    padding: [u8; 6],
    payload: u64,
}
unsafe extern "C" fn property(_: *mut c_void, out: *mut Variant, name: *const c_void) -> u8 {
    let name = unsafe { std::ffi::CStr::from_ptr(name.cast()) }.to_bytes();
    let number = match name {
        b"value" => 0.5_f64,
        b"min" => 0.0,
        b"max" => 1.0,
        b"default" => 0.25,
        _ => return 0,
    };
    unsafe {
        *out = Variant {
            tag: 2,
            padding: [0; 6],
            payload: number.to_bits(),
        };
    }
    1
}
thread_local! {static EMPTY_STRING:Cell<*mut c_void>=const{Cell::new(ptr::null_mut())};}
unsafe extern "C" fn empty_string() -> *mut c_void {
    EMPTY_STRING.with(Cell::get)
}
unsafe extern "C" fn constant_cstring(name: *const c_char) -> *const PlainCString {
    name.cast()
}
unsafe extern "C" fn thread_zero() -> i64 {
    0
}
#[test]
fn positive_raw_parameter_description_uses_actual_typed_api_and_retained_native_owner() {
    let _serial = super::super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut graph = Graph::new();
    let p = graph.parameter(0);
    let object = graph.make(4, "");
    unsafe {
        fake(p).kind.set(2);
        fake(p).object.set(object);
    }
    let string = graph.make(2, "SYNTHETIC_PRIVATE_DISPLAY");
    EMPTY_STRING.with(|s| s.set(string));
    let mut host = state::tests::rumble_parameter_host(graph.root);
    let mut ctx = context(&mut host);
    static API: Api = Api {
        gui: empty_string,
        objects: empty_string,
        main_thread: empty_string,
        thread_self: thread_zero,
        cstring: constant_cstring,
        empty: empty_string,
        desktop: empty_string,
        scripting: empty_string,
        plugin_manager: empty_string,
        file_system: empty_string,
    };
    ctx.api = &API;
    let (value, observation) = observe(&ctx).unwrap();
    assert_eq!(value["parameter_lists_complete"], true);
    assert_eq!(value["access_metadata_complete"], true);
    assert_eq!(value["current_values_complete"], true);
    assert_eq!(value["serialized_name_coverage_complete"], true);
    assert_eq!(
        value["controllers"][0]["parameters"][0]["description"]["current"]["bits_hex"],
        "3fe0000000000000"
    );
    assert_eq!(
        value["controllers"][0]["parameters"][0]["description"]["support"],
        json!({"raw_number":true,"normalized":true})
    );
    assert!(!value.to_string().contains("PRIVATE"));
    observation.check(&ctx).unwrap();
    host.parameter_no_writes();
    drop(observation);
    drop(ctx);
    EMPTY_STRING.with(|s| s.set(ptr::null_mut()));
}
#[test]
fn future_nonnumeric_serialized_siblings_do_not_block_native_census_or_leak_values() {
    let _serial = super::super::super::super::tests::TEST_LOCK.lock().unwrap();
    let graph = Graph::new();
    let mut host = state::tests::rumble_parameter_host(graph.root);
    let mut state = state::tests::rumble_parameter_fixture();
    state["parameters"]["amp"]["futureText"] = json!("SYNTHETIC_PRIVATE_STORED_VALUE");
    host.parameter_state(state);
    let ctx = context(&mut host);
    let (value, observation) = observe(&ctx).unwrap();
    assert_eq!(value["census_complete"], true);
    assert_eq!(value["controllers"][25]["serialized_name_count"], 1);
    assert_eq!(value["serialized_name_coverage_complete"], false);
    assert!(!value.to_string().contains("PRIVATE"));
    drop(observation);
    drop(ctx);
}
#[test]
fn absent_parameter_owner_is_rechecked_and_does_not_become_implicit_authority() {
    let _serial = super::super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut graph = Graph::new();
    let p = graph.parameter(0);
    unsafe {
        fake(p).owner.set(ptr::null_mut());
    }
    let mut host = state::tests::rumble_parameter_host(graph.root);
    let ctx = context(&mut host);
    let (_, observation) = observe(&ctx).unwrap();
    unsafe {
        fake(p).owner.set(graph.controllers[0]);
    }
    assert!(observation.check(&ctx).is_err());
    host.parameter_no_writes();
    drop(observation);
    drop(ctx);
}

fn named_context(host: &mut state::tests::Fixture, graph: &mut Graph) -> Context<'static> {
    let object = graph.make(4, "");
    let parameter = unsafe { fake(graph.controllers[0]).parameter.get() };
    if !parameter.is_null() {
        unsafe {
            fake(parameter).kind.set(2);
            fake(parameter).object.set(object);
        }
    }
    let string = graph.make(2, "SYNTHETIC_PRIVATE_DISPLAY");
    EMPTY_STRING.with(|s| s.set(string));
    static API: Api = Api {
        gui: empty_string,
        objects: empty_string,
        main_thread: empty_string,
        thread_self: thread_zero,
        cstring: constant_cstring,
        empty: empty_string,
        desktop: empty_string,
        scripting: empty_string,
        plugin_manager: empty_string,
        file_system: empty_string,
    };
    let mut ctx = context(host);
    ctx.api = &API;
    ctx
}
fn assert_no_indexed_lookup(graph: &Graph) {
    assert!(
        graph
            .controllers
            .iter()
            .all(|p| unsafe { fake(*p).indexed_calls.get() } == 0)
    );
}
#[test]
fn named_count_zero_positive_uses_actual_description_and_never_indexed_lookup() {
    let _serial = super::super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut graph = Graph::new();
    graph.parameter(0);
    unsafe {
        fake(graph.controllers[0]).count.set(0);
    }
    let mut host = state::tests::rumble_parameter_host(graph.root);
    let ctx = named_context(&mut host, &mut graph);
    let (value, observation) = named::observe(&ctx).unwrap();
    assert!(named::valid_receipt(&json!(ctx.instance), &value));
    assert_eq!(value.as_object().unwrap().len(), 25);
    assert_eq!(value["controllers"][0]["parameter_count"], 0);
    assert_eq!(value["expected_name_count"], 1);
    assert_eq!(value["matched_name_count"], 1);
    assert_eq!(value["named_metadata_complete"], true);
    assert_eq!(value["current_values_complete"], true);
    assert_eq!(
        value["controllers"][0]["parameters"][0]["description"]["current"]["bits_hex"],
        "3fe0000000000000"
    );
    assert!(!value.to_string().contains("PRIVATE"));
    observation.check(&ctx).unwrap();
    assert_no_indexed_lookup(&graph);
    host.parameter_no_writes();
    drop(observation);
    drop(ctx);
    EMPTY_STRING.with(|s| s.set(ptr::null_mut()));
}
#[test]
fn named_null_and_unavailable_controllers_keep_separate_completeness() {
    let _serial = super::super::super::super::tests::TEST_LOCK.lock().unwrap();
    for unavailable in [false, true] {
        let mut graph = Graph::new();
        if unavailable {
            unsafe {
                fake(graph.nodes[25]).controller.set(ptr::null_mut());
                fake(graph.nodes[25]).status.set(NO_INTERFACE);
            }
        }
        let mut host = state::tests::rumble_parameter_host(graph.root);
        let ctx = context(&mut host);
        let (value, observation) = named::observe(&ctx).unwrap();
        assert_eq!(
            value["controllers"][0]["parameters"][0]["status"],
            "parameter_absent"
        );
        assert_eq!(value["controller_counts_complete"], !unavailable);
        assert_eq!(value["named_metadata_complete"], false);
        assert_eq!(value["access_metadata_complete"], false);
        assert_eq!(value["current_values_complete"], false);
        let p = graph.make(3, "");
        unsafe {
            fake(graph.controllers[0]).parameter.set(p);
        }
        assert!(observation.check(&ctx).is_err());
        assert_no_indexed_lookup(&graph);
        drop(observation);
        drop(ctx);
    }
}
#[test]
fn named_aggregate_index_count_above_budget_does_not_disable_bounded_names() {
    let _serial = super::super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut graph = Graph::new();
    graph.parameter(0);
    for p in graph.controllers.iter().take(3) {
        unsafe {
            fake(*p).count.set(128);
        }
    }
    let mut host = state::tests::rumble_parameter_host(graph.root);
    let ctx = named_context(&mut host, &mut graph);
    let (value, observation) = named::observe(&ctx).unwrap();
    assert_eq!(value["named_metadata_complete"], true);
    assert_eq!(value["matched_name_count"], 1);
    assert_no_indexed_lookup(&graph);
    drop(observation);
    drop(ctx);
    EMPTY_STRING.with(|s| s.set(ptr::null_mut()));
}
#[test]
fn named_retained_presence_canonical_owner_count_and_attachment_drift_fail_closed() {
    let _serial = super::super::super::super::tests::TEST_LOCK.lock().unwrap();
    for change in 0..8 {
        let mut graph = Graph::new();
        let p = graph.parameter(0);
        unsafe {
            fake(graph.controllers[0]).count.set(0);
        }
        if change == 7 {
            unsafe {
                fake(p).owner.set(ptr::null_mut());
            }
        }
        let mut host = state::tests::rumble_parameter_host(graph.root);
        let ctx = named_context(&mut host, &mut graph);
        let (value, observation) = named::observe(&ctx).unwrap();
        let service = ctx.service;
        let mut receipt = reply(ctx, value, observation);
        match change {
            0 => unsafe {
                fake(graph.controllers[0]).parameter.set(ptr::null_mut());
            },
            1 => unsafe {
                fake(p).canonical.set(graph.root);
            },
            2 => unsafe {
                fake(p).owner.set(graph.controllers[1]);
            },
            3 => unsafe {
                fake(graph.controllers[0]).count.set(1);
            },
            4 => unsafe {
                fake(graph.root).children.borrow_mut().swap(0, 1);
            },
            5 => unsafe {
                fake(graph.nodes[0]).parent.set(graph.nodes[1]);
            },
            6 => unsafe {
                fake(graph.controllers[0]).canonical.set(graph.root);
            },
            _ => unsafe {
                fake(p).owner.set(graph.controllers[0]);
            },
        }
        assert!(!receipt.final_check(service));
        assert_eq!(receipt.value, named::failure());
        assert_no_indexed_lookup(&graph);
        drop(receipt);
        EMPTY_STRING.with(|s| s.set(ptr::null_mut()));
    }
}
#[test]
fn named_model_names_document_and_forged_output_drift_erase_every_positive_field() {
    let _serial = super::super::super::super::tests::TEST_LOCK.lock().unwrap();
    for change in 0..6 {
        let graph = Graph::new();
        let mut host = state::tests::rumble_parameter_host(graph.root);
        let ctx = context(&mut host);
        let (value, observation) = named::observe(&ctx).unwrap();
        let service = ctx.service;
        let mut receipt = reply(ctx, value, observation);
        let mut state = state::tests::rumble_parameter_fixture();
        match change {
            0 => {
                state["parameters"]["amp"]["__classid"] =
                    json!("11111111-1111-4111-8111-111111111111");
                host.parameter_state(state);
            }
            1 => {
                state["parameters"]["amp"]["newName"] = json!("PRIVATE");
                host.parameter_state(state);
            }
            2 => host.parameter_detach_document(),
            3 => receipt.value["operation"] = json!("rumble_parameters_read"),
            4 => receipt.value["schema"] = json!("rumble_controller_parameters_v1"),
            _ => {
                receipt.value["controllers"][0]["expected_names"][0] = json!("forgedName");
                receipt.value["controllers"][0]["parameters"][0]["name"] = json!("forgedName");
                assert!(named::valid_receipt(
                    &json!(receipt.context.as_ref().unwrap().instance),
                    &receipt.value
                ));
            }
        }
        assert!(!receipt.final_check(service));
        assert_eq!(receipt.value, named::failure());
        drop(receipt);
    }
}
#[test]
fn named_lazy_count_materialization_and_wrong_description_name_are_fatal() {
    let _serial = super::super::super::super::tests::TEST_LOCK.lock().unwrap();
    for lazy in [false, true] {
        let mut graph = Graph::new();
        graph.parameter(0);
        unsafe {
            fake(graph.controllers[0]).count.set(0);
            fake(graph.controllers[0]).materialize.set(lazy);
        }
        let mut host = state::tests::rumble_parameter_host(graph.root);
        if !lazy {
            let mut state = state::tests::rumble_parameter_fixture();
            state["parameters"]["effect1"]
                .as_object_mut()
                .unwrap()
                .remove("gain");
            state["parameters"]["effect1"]["GAIN"] = json!(0);
            host.parameter_state(state);
            unsafe extern "C" fn any(p: *mut c_void, _: *const PlainCString) -> *mut c_void {
                unsafe { fake(p).parameter.get() }
            }
            graph.tables[1][5] = any as *const c_void;
        }
        let ctx = named_context(&mut host, &mut graph);
        assert!(named::observe(&ctx).is_err());
        assert_no_indexed_lookup(&graph);
        drop(ctx);
        EMPTY_STRING.with(|s| s.set(ptr::null_mut()));
    }
}
#[test]
fn named_key_only_domain_accepts_future_nonnumeric_values_and_rejects_whole_bad_domains() {
    let _serial = super::super::super::super::tests::TEST_LOCK.lock().unwrap();
    for case in 0..9 {
        let graph = Graph::new();
        let mut host = state::tests::rumble_parameter_host(graph.root);
        let mut state = state::tests::rumble_parameter_fixture();
        match case {
            0 => {
                state["parameters"]["amp"]["futureText"] =
                    json!({"private":"SYNTHETIC_PRIVATE_STORED_VALUE"})
            }
            1 => state["parameters"]["amp"]["bad.path"] = json!(0),
            2 => state["parameters"]["amp"]["1bad"] = json!(0),
            3 => state["parameters"]["amp"]["한글"] = json!(0),
            4 => state["parameters"]["amp"]["x".repeat(65)] = json!(0),
            5 => {
                for n in 0..65 {
                    state["parameters"]["amp"][format!("n{n:02}")] = json!(0);
                }
            }
            6 => {
                for i in 0..5 {
                    for n in 0..64 {
                        state["parameters"][slot(i)][format!("n{n:02}")] = json!(0);
                    }
                }
            }
            7 => state["parameters"]["amp"]["x".repeat(64)] = json!(null),
            _ => {
                state["parameters"]["effect1"]
                    .as_object_mut()
                    .unwrap()
                    .remove("gain");
                for i in 0..4 {
                    for n in 0..64 {
                        state["parameters"][slot(i)][format!("n{n:02}")] = json!(false);
                    }
                }
            }
        }
        host.parameter_state(state);
        let ctx = context(&mut host);
        let result = named::observe(&ctx);
        if matches!(case, 0 | 7 | 8) {
            let (value, observation) = result.unwrap();
            assert!(!value.to_string().contains("PRIVATE"));
            assert_eq!(value["name_set_complete"], true);
            if case == 8 {
                assert_eq!(value["expected_name_count"], 256);
            }
            drop(observation);
        } else {
            assert!(result.is_err(), "case {case}");
        }
        assert_no_indexed_lookup(&graph);
        drop(ctx);
    }
}
#[test]
fn named_trusted_none_policy_and_mode_mismatch_never_follow_output_discriminators() {
    for retrieval in [Retrieval::Indexed, Retrieval::Named] {
        for encode in [false, true] {
            let mut reply = Reply::unbound(
                json!({"ok":true,"operation":"forged","schema":"forged","controllers":[{"private":"PRIVATE"}]}),
            );
            reply.provider = Some(Provider::RumbleParameters {
                retrieval,
                observation: None,
            });
            if encode {
                assert_eq!(
                    serde_json::from_str::<Value>(&reply.encode()).unwrap(),
                    retrieval.failure()
                );
            } else {
                reply.invalidate("PRIVATE".into());
                assert_eq!(reply.value, retrieval.failure());
            }
        }
    }
    let _serial = super::super::super::super::tests::TEST_LOCK.lock().unwrap();
    let graph = Graph::new();
    let mut host = state::tests::rumble_parameter_host(graph.root);
    let ctx = context(&mut host);
    let (value, observation) = named::observe(&ctx).unwrap();
    let service = ctx.service;
    let mut reply = reply(ctx, value, observation);
    if let Some(Provider::RumbleParameters { retrieval, .. }) = &mut reply.provider {
        *retrieval = Retrieval::Indexed;
    }
    assert!(!reply.final_check(service));
    assert_eq!(reply.value, failure());
    drop(reply);
}
#[test]
fn named_wire_validator_rejects_wrong_shapes_totals_names_precision_and_size() {
    let (instance, indexed) = wire();
    let mut rows = indexed["controllers"].as_array().unwrap().clone();
    for (i, row) in rows.iter_mut().enumerate() {
        let parameters = if i == 0 {
            json!([{"name":"gain","status":"observed","description":project(&description()).unwrap()}])
        } else {
            json!([])
        };
        *row = json!({"slot":slot(i),"serialized_class_uid":row["serialized_class_uid"],"status":"present","parameter_count":0,"expected_names":if i==0 { json!(["gain"]) } else { json!([]) },"parameters":parameters});
    }
    let value = json!({"ok":true,"outcome":"observed","operation":"rumble_named_parameters_read","schema":"rumble_named_parameters_v1","instance":instance,"source":"component_alias_fixed_controllers","scope":"fixed_immediate_serialized_sibling_names","controllers":rows,"models_sha256":indexed["models_sha256"],"before_after_model_ids_equal":true,"before_after_name_sets_equal":true,"name_set_complete":true,"controller_counts_complete":true,"named_metadata_complete":true,"access_metadata_complete":true,"current_values_complete":true,"expected_name_count":1,"matched_name_count":1,"value_semantics":"sequential_snapshot","mutation_capability":false,"can_set_via_this_action":false,"selection_authority":false,"atomic_snapshot":false,"audio_verified":false,"persistence_verified":false});
    assert!(named::valid_receipt(&instance, &value));
    let mut at_limit = value.clone();
    let size = at_limit.to_string().len();
    at_limit["instance"]["object_id"] = json!("x".repeat(MAX_BYTES - size - 4));
    let own_instance = at_limit["instance"].clone();
    assert!(at_limit.to_string().len() <= MAX_BYTES);
    let mut adapted = at_limit.clone();
    adapted["plugin"] = json!("Rumble Native");
    adapted["native_outcome"] = json!("observed");
    assert!(adapted.to_string().len() > MAX_BYTES);
    assert!(!named::valid_receipt(&own_instance, &at_limit));
    let mut over_limit = at_limit.clone();
    over_limit["instance"]["object_id"] = json!("é".repeat(MAX_BYTES));
    let own_instance = over_limit["instance"].clone();
    assert!(over_limit.to_string().len() > MAX_BYTES);
    assert!(!named::valid_receipt(&own_instance, &over_limit));
    for case in 0..10 {
        let mut bad = value.clone();
        match case {
            0 => bad["extra"] = json!(true),
            1 => bad["expected_name_count"] = json!(2),
            2 => bad["matched_name_count"] = json!(0),
            3 => bad["controllers"][0]["parameters"][0]["description"]["name"] = json!("wrong"),
            4 => bad["controllers"][0]["expected_names"] = json!(["gain", "gain"]),
            5 => {
                bad["controllers"][0]["parameters"][0]["description"]["current"]["decimal"] =
                    json!("9223372036854775808")
            }
            6 => {
                bad["controllers"][0]["parameters"][0]["description"]["normalized"]["bits_hex"] =
                    json!("7f800000")
            }
            7 => bad["named_metadata_complete"] = json!(false),
            8 => {
                bad["controllers"][0]["parameters"][0]["description"]["display"] = json!("PRIVATE")
            }
            _ => bad["instance"]["object_id"] = json!("x".repeat(MAX_BYTES)),
        }
        assert!(!named::valid_receipt(&instance, &bad), "case {case}");
    }
    let base = json!({"operation":"rumble_named_parameters_read","expected_instance":"12345678-1234-1234-1234-123456789ABC"});
    assert!(
        serde_json::from_value::<Request>(base.clone())
            .unwrap()
            .validate()
            .is_ok()
    );
    for field in ["value", "source", "token", "name", "path", "script"] {
        let mut bad = base.clone();
        bad[field] = json!("PRIVATE");
        assert!(serde_json::from_value::<Request>(bad).is_err());
    }
}

#[test]
fn named_empty_source_is_vacuous_only_with_all_present_controllers() {
    let _serial = super::super::super::super::tests::TEST_LOCK.lock().unwrap();
    for absent in [false, true] {
        let graph = Graph::new();
        if absent {
            unsafe {
                fake(graph.root).children.borrow_mut().pop();
            }
        }
        let mut host = state::tests::rumble_parameter_host(graph.root);
        let mut state = state::tests::rumble_parameter_fixture();
        state["parameters"]["effect1"]
            .as_object_mut()
            .unwrap()
            .remove("gain");
        host.parameter_state(state);
        let ctx = context(&mut host);
        let (value, observation) = named::observe(&ctx).unwrap();
        assert_eq!(value["expected_name_count"], 0);
        assert_eq!(value["matched_name_count"], 0);
        assert_eq!(value["named_metadata_complete"], !absent);
        assert_eq!(value["access_metadata_complete"], !absent);
        assert_eq!(value["current_values_complete"], !absent);
        assert!(
            graph
                .controllers
                .iter()
                .all(|p| unsafe { fake(*p).find_calls.get() } == 0)
        );
        assert_no_indexed_lookup(&graph);
        drop(observation);
        drop(ctx);
    }
}
#[test]
fn named_failed_description_retains_its_named_association_and_owner() {
    let _serial = super::super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut graph = Graph::new();
    graph.parameter(0);
    unsafe {
        fake(graph.controllers[0]).count.set(0);
    }
    let mut host = state::tests::rumble_parameter_host(graph.root);
    let ctx = context(&mut host);
    let (value, observation) = named::observe(&ctx).unwrap();
    assert_eq!(
        value["controllers"][0]["parameters"][0]["status"],
        "description_unavailable"
    );
    assert_eq!(
        value["controllers"][0]["parameters"][0]["description"],
        Value::Null
    );
    assert_eq!(value["named_metadata_complete"], false);
    observation.check(&ctx).unwrap();
    unsafe {
        fake(graph.controllers[0]).parameter.set(ptr::null_mut());
    }
    assert!(observation.check(&ctx).is_err());
    assert_no_indexed_lookup(&graph);
    drop(observation);
    drop(ctx);
}
#[test]
fn named_preflight_and_empty_capture_or_cleanup_leases_keep_trusted_failure_policy() {
    let _serial = super::super::super::super::tests::TEST_LOCK.lock().unwrap();
    let graph = Graph::new();
    let mut host = state::tests::rumble_parameter_host(graph.root);
    let ctx = context(&mut host);
    let mut result = dispatch(
        ctx.service,
        hold(ctx.target.0).unwrap(),
        &json!({"operation":"rumble_named_parameters_read","expected_instance":"invalid"})
            .to_string(),
        None,
        ctx.api,
    );
    assert_eq!(
        serde_json::from_str::<Value>(&result.encode()).unwrap(),
        named::failure()
    );
    let mut result=dispatch(ctx.service,hold(ctx.target.0).unwrap(),&json!({"operation":"rumble_named_parameters_read","expected_instance":ctx.instance.object_uid}).to_string(),Some(hold(graph.root).unwrap()),ctx.api);
    assert_eq!(
        serde_json::from_str::<Value>(&result.encode()).unwrap(),
        named::failure()
    );
    drop(ctx);
    for change in 0..3 {
        let ctx = context(&mut host);
        let service = ctx.service;
        let (valid, observation) = named::observe(&ctx).unwrap();
        drop(observation);
        let mut receipt = Reply::unbound(valid);
        receipt.provider = Some(Provider::RumbleParameters {
            retrieval: Retrieval::Named,
            observation: None,
        });
        receipt.context = Some(Box::new(ctx));
        match change {
            0 => {
                assert!(!receipt.final_check(service));
            }
            1 => {
                assert_eq!(
                    serde_json::from_str::<Value>(&receipt.encode()).unwrap(),
                    named::failure()
                );
            }
            _ => {
                receipt.invalidate("PRIVATE_CLEANUP".into());
            }
        }
        assert_eq!(receipt.value, named::failure());
        drop(receipt);
    }
}

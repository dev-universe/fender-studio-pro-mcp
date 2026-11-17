use super::*;
use std::cell::{Cell, RefCell};

#[repr(C)]
struct Fake {
    table: *const *const c_void,
    refs: Cell<u32>,
    canonical: Cell<*mut c_void>,
    alias: Cell<*mut c_void>,
    parent: Cell<*mut c_void>,
    provider: Cell<*mut c_void>,
    member: Cell<*mut c_void>,
    gets: Cell<u32>,
    queries: RefCell<Vec<Uid>>,
    failed_out: Cell<Option<Uid>>,
}

unsafe fn fake<'a>(object: *mut c_void) -> &'a Fake {
    unsafe { &*object.cast::<Fake>() }
}
unsafe extern "C" fn retain(object: *mut c_void) -> u32 {
    let object = unsafe { fake(object) };
    object.refs.set(object.refs.get() + 1);
    object.refs.get()
}
unsafe extern "C" fn release_fake(object: *mut c_void) -> u32 {
    let object = unsafe { fake(object) };
    object.refs.set(object.refs.get() - 1);
    object.refs.get()
}
unsafe extern "C" fn query_fake(
    object: *mut c_void,
    iid: *const Uid,
    out: *mut *mut c_void,
) -> i32 {
    let f = unsafe { fake(object) };
    let iid = unsafe { *iid };
    f.queries.borrow_mut().push(iid);
    unsafe {
        *out = ptr::null_mut();
    }
    if f.failed_out.get() == Some(iid) {
        unsafe {
            retain(object);
            *out = object;
        }
        return NO_INTERFACE;
    }
    let found = if iid == UNKNOWN {
        f.canonical.get()
    } else if iid == ALIAS {
        f.alias.get()
    } else if iid == CONTROLLER_PROVIDER {
        f.provider.get()
    } else {
        ptr::null_mut()
    };
    if found.is_null() {
        return NO_INTERFACE;
    }
    unsafe {
        retain(found);
        *out = found;
    }
    OK
}
unsafe extern "C" fn member(object: *mut c_void) -> *mut c_void {
    let f = unsafe { fake(object) };
    f.gets.set(f.gets.get() + 1);
    f.member.get()
}
unsafe extern "C" fn parent(object: *mut c_void) -> *mut c_void {
    unsafe { fake(object) }.parent.get()
}
unsafe extern "C" fn constant(_: *const c_char) -> *const c_void {
    ptr::null()
}

struct Fixture {
    objects: Box<[Fake; 6]>,
    _table: Box<[*const c_void; 7]>,
}
impl Fixture {
    fn new(present: bool) -> Self {
        let mut table = Box::new([ptr::null(); 7]);
        table[0] = query_fake as *const c_void;
        table[1] = retain as *const c_void;
        table[2] = release_fake as *const c_void;
        table[3] = member as *const c_void;
        table[6] = parent as *const c_void;
        let objects = Box::new(std::array::from_fn(|_| Fake {
            table: table.as_ptr(),
            refs: Cell::new(1),
            canonical: Cell::new(ptr::null_mut()),
            alias: Cell::new(ptr::null_mut()),
            parent: Cell::new(ptr::null_mut()),
            provider: Cell::new(ptr::null_mut()),
            member: Cell::new(ptr::null_mut()),
            gets: Cell::new(0),
            queries: RefCell::new(Vec::new()),
            failed_out: Cell::new(None),
        }));
        let result = Self {
            objects,
            _table: table,
        };
        for index in 0..6 {
            result.objects[index].canonical.set(result.pointer(index));
        }
        result.objects[0].alias.set(result.pointer(2));
        result.objects[0].parent.set(result.pointer(1));
        result.objects[2].member.set(result.pointer(3));
        result.objects[3].provider.set(result.pointer(3));
        if present {
            result.objects[3].member.set(result.pointer(4));
        }
        result
    }
    fn pointer(&self, index: usize) -> *mut c_void {
        ptr::from_ref(&self.objects[index]).cast_mut().cast()
    }
    fn binding<G: FnMut() -> Result<(), String>>(
        &self,
        guard: &mut G,
    ) -> Result<AliasBinding, String> {
        AliasBinding::capture(self.pointer(0), self.pointer(0), guard)
    }
    fn inventory<G: FnMut() -> Result<(), String>>(
        &self,
        guard: &mut G,
    ) -> Result<Inventory, String> {
        observe(
            self.binding(guard)?,
            TextApi {
                get_constant_string: constant,
            },
            node_inspect::limits(node_inspect::Scope::PluginController),
            guard,
        )
    }
    fn released(&self) {
        assert!(self.objects.iter().all(|object| object.refs.get() == 1));
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.released();
    }
}

#[test]
fn scope_requires_the_fixed_shape_bound_instance_and_observed_classes() {
    let base = json!({"operation":"inspect","include_nodes":true,"node_scope":"plugin_controller",
        "expected_instance":"12345678-1234-1234-1234-123456789ABC"});
    let request: Request = serde_json::from_value(base.clone()).unwrap();
    request.validate().unwrap();
    for extra in [
        json!({"path":"workspaceroot"}),
        json!({"method":"invoke"}),
        json!({"model":"effectListModel"}),
    ] {
        let mut invalid = base.clone();
        invalid
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        assert!(serde_json::from_value::<Request>(invalid).is_err());
    }
    for key in ["include_nodes", "expected_instance"] {
        let mut invalid = base.clone();
        invalid.as_object_mut().unwrap().remove(key);
        assert!(
            serde_json::from_value::<Request>(invalid)
                .unwrap()
                .validate()
                .is_err()
        );
    }
    assert!(supported("11B8AA91-9BED-4A7F-90F4-02C6A6C7AF34"));
    assert!(supported("B9D7536C-A010-457D-A182-36E5531164C9"));
    assert!(!supported("11b8aa91-9bed-4a7f-90f4-02c6a6c7af34"));
    assert!(!supported("831A0046-BD71-45BF-A419-6FC90D6E9F2F"));
}

#[test]
fn an_unobserved_device_class_is_refused_before_alias_resolution() {
    let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut host = guard_tests::Fixture::new();
    let context = host.context();
    let result = capture(
        &context,
        TextApi {
            get_constant_string: constant,
        },
        node_inspect::limits(node_inspect::Scope::PluginController),
    );
    assert!(
        matches!(result, Err(reason) if reason.contains("observed Mustang Native or Rumble Native"))
    );
    drop(context);
}

#[test]
fn present_non_node_controller_is_pinned_and_reports_only_bounded_metadata() {
    let fixture = Fixture::new(true);
    let inventory = fixture
        .inventory(&mut || {
            if fixture.objects[3].gets.get() > 0 {
                assert!(fixture.objects[4].refs.get() > 1);
            }
            Ok(())
        })
        .unwrap();
    assert!(
        fixture
            .objects
            .iter()
            .take(5)
            .all(|object| object.refs.get() > 1)
    );
    let value = inventory.value();
    assert_eq!(value["scope"], "alias_declared_controller_observation");
    assert_eq!(value["mutation_capability"], false);
    assert_eq!(value["atomic_snapshot"], false);
    assert_eq!(value["nodes"].as_array().unwrap().len(), 1);
    assert_eq!(value["nodes"][0]["identity"], Value::Null);
    assert_eq!(value["nodes"][0]["reflection"]["available"], false);
    assert_eq!(value["limits"]["ancestors"], 0);
    assert_eq!(
        value["limits"],
        json!({"nodes":96,"depth":5,"ancestors":0,"children_per_node":32})
    );
    assert!(value.get("catalog").is_none());
    inventory.check(&mut || Ok(())).unwrap();
    drop(inventory);
    fixture.released();
}

#[test]
fn stable_absence_is_not_an_empty_or_complete_model_catalog() {
    let fixture = Fixture::new(false);
    let inventory = fixture.inventory(&mut || Ok(())).unwrap();
    let value = inventory.value();
    assert_eq!(value["controller"], Value::Null);
    assert_eq!(value["complete"], false);
    assert_eq!(value["atomic_snapshot"], false);
    assert_eq!(value["diagnostics"][0]["kind"], "controller_absent");
    assert_eq!(
        value["limits"],
        json!({"nodes":96,"depth":5,"ancestors":0,"children_per_node":32})
    );
    inventory.check(&mut || Ok(())).unwrap();
    fixture.objects[3].member.set(fixture.pointer(4));
    assert!(inventory.check(&mut || Ok(())).is_err());
    drop(inventory);
}

#[test]
fn replacements_at_every_route_identity_or_borrowed_attachment_are_refused() {
    for change in 0..7 {
        let fixture = Fixture::new(true);
        let inventory = fixture.inventory(&mut || Ok(())).unwrap();
        match change {
            0 => fixture.objects[2].member.set(fixture.pointer(5)),
            1 => fixture.objects[0].parent.set(fixture.pointer(5)),
            2 => fixture.objects[3].member.set(fixture.pointer(5)),
            3 => fixture.objects[3].provider.set(fixture.pointer(5)),
            4 => fixture.objects[3].canonical.set(fixture.pointer(5)),
            5 => fixture.objects[4].canonical.set(fixture.pointer(5)),
            6 => fixture.objects[1].canonical.set(fixture.pointer(5)),
            _ => unreachable!(),
        }
        assert!(
            inventory.check(&mut || Ok(())).is_err(),
            "route change {change} passed"
        );
        drop(inventory);
    }
}

#[test]
fn final_guard_cannot_replace_the_alias_parent_or_controller_after_cached_reads() {
    for change in 0..3 {
        let fixture = Fixture::new(true);
        let inventory = fixture.inventory(&mut || Ok(())).unwrap();
        let mut guards = 0;
        inventory
            .check(&mut || {
                guards += 1;
                Ok(())
            })
            .unwrap();
        let last_guard = guards;
        let mut calls = 0;
        let result = inventory.check(&mut || {
            calls += 1;
            if calls == last_guard {
                match change {
                    0 => fixture.objects[2].member.set(fixture.pointer(5)),
                    1 => fixture.objects[0].parent.set(fixture.pointer(5)),
                    2 => fixture.objects[3].member.set(fixture.pointer(5)),
                    _ => unreachable!(),
                }
            }
            Ok(())
        });
        assert!(result.is_err());
        drop(inventory);
    }
}

#[test]
fn missing_provider_and_failed_owned_out_release_every_pin() {
    for bad_out in [false, true] {
        let fixture = Fixture::new(true);
        if bad_out {
            fixture.objects[3].failed_out.set(Some(CONTROLLER_PROVIDER));
        } else {
            fixture.objects[3].provider.set(ptr::null_mut());
        }
        assert!(fixture.binding(&mut || Ok(())).is_err());
        fixture.released();
    }
}

#[test]
fn context_loss_at_every_capture_guard_releases_pins_without_an_inventory() {
    let baseline = Fixture::new(true);
    let mut calls = 0;
    drop(
        baseline
            .inventory(&mut || {
                calls += 1;
                Ok(())
            })
            .unwrap(),
    );
    baseline.released();
    for stop in 1..=calls {
        let fixture = Fixture::new(true);
        let mut observed = 0;
        let result = fixture.inventory(&mut || {
            observed += 1;
            if observed >= stop {
                Err("synthetic document changed".into())
            } else {
                Ok(())
            }
        });
        assert!(result.is_err(), "guard {stop} produced an inventory");
        fixture.released();
    }
}

fn reply(context: Box<Context<'static>>, inventory: Inventory) -> Reply {
    let nodes = node_inspect::Inventory::PluginController(inventory);
    Reply {
        value: json!({"ok":true,"node_report":nodes.value()}),
        menu: None,
        catalog: None,
        edit: None,
        action: None,
        presets: None,
        owner: None,
        gui: None,
        auxiliary: None,
        folder_preflight: None,
        nodes: Some(nodes),
        provider: None,
        context: Some(context),
        mutation: false,
    }
}

#[test]
fn inventory_and_declared_controller_survive_output_until_receipt_release() {
    let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut host = guard_tests::Fixture::new();
    let context = host.context();
    let service = context.service;
    let fixture = Fixture::new(true);
    let inventory = fixture.inventory(&mut || Ok(())).unwrap();
    let reply = reply(context, inventory);
    let mut out = VariantOut {
        kind: 0,
        payload: 0,
    };
    assert!(publish_reply(&mut out, reply, api().unwrap(), service));
    let value: Value =
        serde_json::from_str(&super::super::super::tests::output_text(&out)).unwrap();
    assert_eq!(value["ok"], true);
    assert_eq!(value["node_report"]["mutation_capability"], false);
    assert!(fixture.objects[4].refs.get() > 1);
    release_receipt(service);
    fixture.released();
    super::super::super::tests::clear_output(&mut out);
}

#[test]
fn changed_attachment_before_publication_discards_all_controller_metadata() {
    let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut host = guard_tests::Fixture::new();
    let context = host.context();
    let service = context.service;
    let fixture = Fixture::new(true);
    let inventory = fixture.inventory(&mut || Ok(())).unwrap();
    let mut reply = reply(context, inventory);
    fixture.objects[3].member.set(fixture.pointer(5));
    assert!(!reply.final_check(service));
    assert_eq!(reply.value["ok"], false);
    assert!(reply.value.get("node_report").is_none());
    drop(reply);
    fixture.released();
}

fn multi_instance() -> Instance {
    Instance {
        object_uid: "12345678-1234-1234-1234-123456789ABC".into(),
        class_uid: "B9D410AD-E47D-495E-8AEE-10D2B2987CC7".into(),
        native_class: "AudioSynth".into(),
        object_id: "Multi Instrument".into(),
    }
}
fn multi_capture<G: FnMut() -> Result<(), String>>(
    fixture: &Fixture,
    instance: &Instance,
    guard: &mut G,
) -> Result<Inventory, String> {
    capture_bound(
        instance,
        fixture.pointer(0),
        fixture.pointer(0),
        TextApi {
            get_constant_string: constant,
        },
        node_inspect::limits(node_inspect::Scope::PluginController),
        guard,
    )
}
#[test]
fn exact_multi_readonly_admission_cannot_admit_its_model_catalog_or_other_namespace() {
    let instance = multi_instance();
    assert!(inspection_supported(&instance));
    assert!(!supported(&instance.class_uid)); // catalog keeps its original two classes
    for field in 0..3 {
        let fixture = Fixture::new(true);
        let mut wrong = instance.clone();
        match field {
            0 => wrong.class_uid = "831A0046-BD71-45BF-A419-6FC90D6E9F2F".into(),
            1 => wrong.native_class = "Device".into(),
            _ => wrong.object_id = "Other Multi".into(),
        }
        assert!(multi_capture(&fixture, &wrong, &mut || Ok(())).is_err());
        assert!(
            fixture
                .objects
                .iter()
                .all(|o| o.gets.get() == 0 && o.queries.borrow().is_empty())
        );
        fixture.released();
    }
}
#[test]
fn exact_multi_fake_uses_declared_interfaces_retains_owners_and_keeps_bounds() {
    let fixture = Fixture::new(true);
    let inventory = multi_capture(&fixture, &multi_instance(), &mut || Ok(())).unwrap();
    let value = inventory.value();
    assert_eq!(value["mutation_capability"], false);
    assert_eq!(value["atomic_snapshot"], false);
    assert_eq!(
        value["limits"],
        json!({"nodes":96,"depth":5,"ancestors":0,"children_per_node":32})
    );
    assert!(fixture.objects[4].refs.get() > 1);
    assert!(
        fixture.objects[3]
            .queries
            .borrow()
            .contains(&CONTROLLER_PROVIDER)
    );
    inventory.check(&mut || Ok(())).unwrap();
    drop(inventory);
    fixture.released();
}
#[test]
fn multi_wrong_instance_context_is_refused_before_any_native_dispatch() {
    let fixture = Fixture::new(true);
    assert!(
        multi_capture(&fixture, &multi_instance(), &mut || Err(
            "expected instance changed".into()
        ))
        .is_err()
    );
    assert!(
        fixture
            .objects
            .iter()
            .all(|o| o.gets.get() == 0 && o.queries.borrow().is_empty())
    );
    fixture.released();
}
#[test]
fn multi_missing_declared_provider_never_falls_back_and_null_controller_stays_unavailable() {
    let fixture = Fixture::new(true);
    fixture.objects[3].provider.set(ptr::null_mut());
    assert!(multi_capture(&fixture, &multi_instance(), &mut || Ok(())).is_err());
    fixture.released();
    let fixture = Fixture::new(false);
    let inventory = multi_capture(&fixture, &multi_instance(), &mut || Ok(())).unwrap();
    assert_eq!(inventory.value()["controller"], Value::Null);
    assert_eq!(inventory.value()["complete"], false);
    assert_eq!(
        inventory.value()["diagnostics"][0]["kind"],
        "controller_absent"
    );
    drop(inventory);
    fixture.released();
}

#[test]
fn actual_context_instance_guard_still_precedes_alias_capture() {
    let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut host = guard_tests::Fixture::new();
    let mut context = host.context();
    context.instance.object_uid = "00000000-0000-0000-0000-000000000000".into();
    let result = capture(
        &context,
        TextApi {
            get_constant_string: constant,
        },
        node_inspect::limits(node_inspect::Scope::PluginController),
    );
    assert!(matches!(result,Err(reason) if reason=="native target identity changed"));
    drop(context);
}

fn level_instance() -> Instance {
    Instance {
        object_uid: "12345678-1234-1234-1234-123456789ABC".into(),
        class_uid: "DF958D90-0196-434F-ADC3-1FC2BB7F4AF6".into(),
        native_class: "AudioEffect".into(),
        object_id: "Level Meter".into(),
    }
}
#[test]
fn exact_level_metadata_admission_does_not_admit_catalog_or_other_effects() {
    let exact = level_instance();
    assert!(inspection_supported(&exact));
    assert!(!supported(&exact.class_uid));
    for field in 0..5 {
        let f = Fixture::new(true);
        let mut wrong = exact.clone();
        match field {
            0 => wrong.class_uid = "831A0046-BD71-45BF-A419-6FC90D6E9F2F".into(),
            1 => wrong.native_class = "AudioSynth".into(),
            2 => wrong.object_id = "Other Meter".into(),
            3 => wrong.object_uid = ZERO_UID.into(),
            _ => wrong.object_uid = "not-a-native-uid".into(),
        }
        assert!(multi_capture(&f, &wrong, &mut || Ok(())).is_err());
        assert!(
            f.objects
                .iter()
                .all(|o| o.gets.get() == 0 && o.queries.borrow().is_empty())
        );
        f.released();
    }
}
#[test]
fn level_present_non_node_controller_keeps_unknown_types_limits_and_no_values() {
    let f = Fixture::new(true);
    let inventory = multi_capture(&f, &level_instance(), &mut || Ok(())).unwrap();
    let value = inventory.value();
    assert_eq!(
        value["coverage"],
        "bounded_alias_declared_controller_metadata"
    );
    assert_eq!(value["scope"], "alias_declared_controller_observation");
    assert_eq!(value["mutation_capability"], false);
    assert_eq!(value["atomic_snapshot"], false);
    assert_eq!(value["nodes"].as_array().unwrap().len(), 1);
    assert_eq!(value["nodes"][0]["identity"], Value::Null);
    assert_eq!(value["nodes"][0]["reflection"]["available"], false);
    assert_eq!(
        value["limits"],
        json!({"nodes":96,"depth":5,"ancestors":0,"children_per_node":32})
    );
    for field in ["catalog", "params", "parameter_values", "preset_support"] {
        assert!(value.get(field).is_none());
    }
    assert!(f.objects[4].refs.get() > 1);
    inventory.check(&mut || Ok(())).unwrap();
    drop(inventory);
    f.released();
}
#[test]
fn level_missing_provider_or_null_controller_is_not_positive_support() {
    for bad_out in [false, true] {
        let f = Fixture::new(true);
        if bad_out {
            f.objects[3].failed_out.set(Some(CONTROLLER_PROVIDER));
        } else {
            f.objects[3].provider.set(ptr::null_mut());
        }
        assert!(multi_capture(&f, &level_instance(), &mut || Ok(())).is_err());
        f.released();
    }
    let f = Fixture::new(false);
    let inventory = multi_capture(&f, &level_instance(), &mut || Ok(())).unwrap();
    let value = inventory.value();
    assert_eq!(value["controller"], Value::Null);
    assert_eq!(value["complete"], false);
    assert_eq!(value["diagnostics"][0]["kind"], "controller_absent");
    drop(inventory);
    f.released();
}
#[test]
fn level_parent_alias_provider_controller_and_canonical_replacement_rejects() {
    for change in 0..7 {
        let f = Fixture::new(true);
        let inventory = multi_capture(&f, &level_instance(), &mut || Ok(())).unwrap();
        match change {
            0 => f.objects[2].member.set(f.pointer(5)),
            1 => f.objects[0].parent.set(f.pointer(5)),
            2 => f.objects[3].member.set(f.pointer(5)),
            3 => f.objects[3].provider.set(f.pointer(5)),
            4 => f.objects[3].canonical.set(f.pointer(5)),
            5 => f.objects[4].canonical.set(f.pointer(5)),
            _ => f.objects[1].canonical.set(f.pointer(5)),
        }
        assert!(inventory.check(&mut || Ok(())).is_err(), "route {change}");
        drop(inventory);
        f.released();
    }
}
#[test]
fn level_late_final_guard_cannot_hide_parent_alias_or_controller_replacement() {
    for change in 0..3 {
        let f = Fixture::new(true);
        let inventory = multi_capture(&f, &level_instance(), &mut || Ok(())).unwrap();
        let mut total = 0;
        inventory
            .check(&mut || {
                total += 1;
                Ok(())
            })
            .unwrap();
        let mut seen = 0;
        let result = inventory.check(&mut || {
            seen += 1;
            if seen == total {
                match change {
                    0 => f.objects[2].member.set(f.pointer(5)),
                    1 => f.objects[0].parent.set(f.pointer(5)),
                    _ => f.objects[3].member.set(f.pointer(5)),
                }
            }
            Ok(())
        });
        assert!(result.is_err());
        drop(inventory);
        f.released();
    }
}
#[test]
fn level_context_loss_at_each_capture_guard_releases_all_held_objects() {
    let baseline = Fixture::new(true);
    let mut count = 0;
    drop(
        multi_capture(&baseline, &level_instance(), &mut || {
            count += 1;
            Ok(())
        })
        .unwrap(),
    );
    baseline.released();
    for stop in 1..=count {
        let f = Fixture::new(true);
        let mut calls = 0;
        let result = multi_capture(&f, &level_instance(), &mut || {
            calls += 1;
            if calls >= stop {
                Err("Level document or instance changed".into())
            } else {
                Ok(())
            }
        });
        assert!(result.is_err(), "guard {stop}");
        f.released();
    }
}

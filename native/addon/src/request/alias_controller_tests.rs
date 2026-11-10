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
        catalog: None,
        edit: None,
        action: None,
        presets: None,
        owner: None,
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
        padding: [0; 6],
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

use super::*;

#[test]
fn catalog_tokens_are_unpredictable_canonical_and_service_local() {
    let first = token();
    let second = token();
    assert!(state::valid_token(&first));
    assert!(state::valid_token(&second));
    assert_ne!(first, second);
    let mut first_service = Registry::new();
    let second_service = Registry::<usize>::new();
    first_service.entries.push(Record {
        token: first.clone(),
        lease: Rc::new(7usize),
    });
    assert_eq!(*first_service.get(&first).unwrap(), 7);
    assert!(second_service.get(&first).is_err());
    assert!(first_service.get(&second).is_err());
    let record = first_service.take(&first).unwrap();
    assert!(first_service.get(&first).is_err());
    drop(record);
}
#[test]
fn bounded_registry_moves_owners_and_publication_retains_catalog_after_release() {
    struct Owner(Rc<std::cell::Cell<u32>>);
    impl Drop for Owner {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    let drops = Rc::new(std::cell::Cell::new(0));
    let mut registry = Registry::new();
    for i in 0..MAX_CATALOGS {
        registry.capacity().unwrap();
        registry.entries.push(Record {
            token: format!("{i:064X}"),
            lease: Rc::new(Owner(Rc::clone(&drops))),
        });
    }
    assert!(registry.capacity().is_err());
    let publication = registry.get(&format!("{:064X}", 0)).unwrap();
    let removed = registry.take(&format!("{:064X}", 0)).unwrap();
    assert_eq!(drops.get(), 0);
    drop(removed);
    assert_eq!(drops.get(), 0);
    let drained = std::mem::take(&mut registry.entries);
    assert_eq!(drops.get(), 0);
    drop(drained);
    assert_eq!(drops.get(), 3);
    drop(publication);
    assert_eq!(drops.get(), 4);
}
#[test]
fn pages_expose_only_finite_entries_and_do_not_infer_factory_origin() {
    let page = Page {
        entries: vec![fender_native_preset_api::PresetInfo {
            index: 3,
            name: "Bounded preset".into(),
            class_key: "CLASS".into(),
            read_only: true,
            availability: Availability::UnsupportedProtocol("package".into()),
        }],
        observed_count: 5,
        offset: 3,
        next: Some(4),
        catalog_complete: true,
    };
    let value = page_value(&"A".repeat(64), &page);
    assert_eq!(value["entries"][0]["recallable"], false);
    assert_eq!(value["excluded_on_page"]["unsupported_protocol"], 1);
    assert_eq!(value["factory_origin"], "not_inferred");
    assert!(!value.to_string().contains("package"));
    assert!(value["entries"][0].get("url").is_none());
    assert!(value["entries"][0].get("path").is_none());
    assert_eq!(limits()["max_count"], 4);
    assert_eq!(limits()["max_page_size"], 64);
}

#[test]
fn forged_indices_never_reach_recall_and_acceptance_is_not_verification() {
    let mut issued = vec![false; fender_native_preset_api::MAX_PRESETS];
    issued[2] = true;
    for index in [0, 1, 3, 4096, u32::MAX] {
        assert!(!issued_index(&issued, index));
    }
    assert!(issued_index(&issued, 2));
    let accepted = apply_value(
        "TOKEN",
        2,
        Ok(fender_native_preset_api::Recall {
            accepted: true,
            verified: false,
        }),
    );
    assert_eq!(accepted["accepted"], true);
    assert_eq!(accepted["verified"], false);
    assert_eq!(accepted["outcome"], "applied_unverified");
    let rejected = apply_value(
        "TOKEN",
        2,
        Ok(fender_native_preset_api::Recall {
            accepted: false,
            verified: false,
        }),
    );
    assert_eq!(rejected["outcome"], "not_applied");
    assert_eq!(rejected["verified"], false);
    for dispatched in [false, true] {
        let value = apply_value("TOKEN", 2, Err(("document changed".into(), dispatched)));
        assert_eq!(value["ok"], false);
        assert!(value.get("verified").is_none());
        assert_eq!(
            value["outcome"],
            if dispatched { "unknown" } else { "not_applied" }
        );
    }
}

#[repr(C)]
struct Node {
    table: *const *const c_void,
    refs: AtomicU32,
    parent: *mut c_void,
    child: *mut c_void,
    uid: Uid,
    class: Uid,
}
unsafe extern "C" fn retain(p: *mut c_void) -> u32 {
    unsafe { (*p.cast::<Node>()).refs.fetch_add(1, Ordering::Relaxed) + 1 }
}
unsafe extern "C" fn release(p: *mut c_void) -> u32 {
    unsafe { (*p.cast::<Node>()).refs.fetch_sub(1, Ordering::Relaxed) - 1 }
}
unsafe extern "C" fn query_node(p: *mut c_void, iid: *const Uid, out: *mut *mut c_void) -> i32 {
    unsafe {
        *out = ptr::null_mut();
    }
    if unsafe { *iid } != UNKNOWN {
        return NO_INTERFACE;
    }
    unsafe {
        retain(p);
        *out = p;
    }
    OK
}
unsafe extern "C" fn object_uid(p: *mut c_void) -> *const Uid {
    unsafe { ptr::addr_of!((*p.cast::<Node>()).uid) }
}
unsafe extern "C" fn class_uid(p: *mut c_void) -> *const Uid {
    unsafe { ptr::addr_of!((*p.cast::<Node>()).class) }
}
unsafe extern "C" fn parent(p: *mut c_void) -> *mut c_void {
    unsafe { (*p.cast::<Node>()).parent }
}
unsafe extern "C" fn child(p: *mut c_void, _: *const PlainString) -> *mut c_void {
    unsafe { (*p.cast::<Node>()).child }
}
struct Nodes {
    _table: Box<[*const c_void; 11]>,
    nodes: Box<[Node; 3]>,
}
impl Nodes {
    fn new() -> Self {
        let mut table = Box::new([ptr::null(); 11]);
        table[0] = query_node as *const c_void;
        table[1] = retain as *const c_void;
        table[2] = release as *const c_void;
        table[4] = object_uid as *const c_void;
        table[5] = class_uid as *const c_void;
        table[6] = parent as *const c_void;
        table[10] = child as *const c_void;
        let nodes = Box::new(std::array::from_fn(|_| Node {
            table: table.as_ptr(),
            refs: AtomicU32::new(1),
            parent: ptr::null_mut(),
            child: ptr::null_mut(),
            uid: SERVICE_CID,
            class: API_CLASS,
        }));
        let mut nodes = Self {
            _table: table,
            nodes,
        };
        nodes.nodes[0].parent = nodes.pointer(1);
        nodes.nodes[1].child = nodes.pointer(2);
        nodes
    }
    fn pointer(&mut self, index: usize) -> *mut c_void {
        ptr::addr_of_mut!(self.nodes[index]).cast()
    }
    fn route(&mut self, ctx: &Context<'_>) -> Route {
        let target = query(ctx.target.0, &UNKNOWN).unwrap();
        let slot = self.pointer(1);
        let presets = self.pointer(2);
        Route {
            presets: hold(presets).unwrap(),
            _presets_identity: query(presets, &UNKNOWN).unwrap(),
            slot: hold(slot).unwrap(),
            _slot_identity: query(slot, &UNKNOWN).unwrap(),
            device: hold(self.pointer(0)).unwrap(),
            key: Key {
                document: ctx.document._identity.0,
                model: ctx.document._model_identity.0,
                target: target.0,
                object_uid: ctx.instance.object_uid.clone(),
                class_uid: ctx.instance.class_uid.clone(),
            },
            _target: target,
            _model: hold(ctx.document._model_identity.0).unwrap(),
            _document: hold(ctx.document._identity.0).unwrap(),
            name: ptr::null(),
            slot_identity: node_identity(slot),
            presets_identity: node_identity(presets),
        }
    }
}
impl Drop for Nodes {
    fn drop(&mut self) {
        assert!(
            self.nodes
                .iter()
                .all(|node| node.refs.load(Ordering::Acquire) == 1)
        );
    }
}
#[test]
fn fixed_preset_route_rejects_replacement_and_in_place_identity_changes() {
    let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut fixture = super::super::guard_tests::Fixture::new();
    fixture.canonical_queries();
    let mut ctx = fixture.context();
    let mut nodes = Nodes::new();
    ctx.node = hold(nodes.pointer(0)).unwrap();
    let route = nodes.route(&ctx);
    route.guard(&ctx).unwrap();
    route.matches(&ctx).unwrap();
    nodes.nodes[0].parent = nodes.pointer(2);
    assert!(route.guard(&ctx).is_err());
    nodes.nodes[0].parent = nodes.pointer(1);
    nodes.nodes[1].child = nodes.pointer(0);
    assert!(route.guard(&ctx).is_err());
    nodes.nodes[1].child = nodes.pointer(2);
    nodes.nodes[1].uid = UNKNOWN;
    assert!(route.guard(&ctx).is_err());
    nodes.nodes[1].uid = SERVICE_CID;
    nodes.nodes[2].class = UNKNOWN;
    assert!(route.guard(&ctx).is_err());
    nodes.nodes[2].class = API_CLASS;
    route.guard(&ctx).unwrap();
    let original = std::mem::replace(&mut ctx.target, hold(nodes.pointer(2)).unwrap());
    assert!(route.matches(&ctx).is_err());
    ctx.target = original;
    let original = std::mem::replace(&mut ctx.document._identity, hold(nodes.pointer(2)).unwrap());
    assert!(route.matches(&ctx).is_err());
    ctx.document._identity = original;
    route.matches(&ctx).unwrap();
    fixture.switch_document();
    assert!(route.guard(&ctx).is_err());
    drop(route);
    drop(ctx);
    drop(nodes);
}

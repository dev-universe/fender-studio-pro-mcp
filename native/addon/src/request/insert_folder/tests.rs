use super::*;

#[repr(C)]
struct Node {
    table: *const *const c_void,
    refs: AtomicU32,
    parent: *mut c_void,
    child: *mut c_void,
    object_uid: Uid,
    class_uid: Uid,
    reject_canonical: bool,
    foreign_interface: *mut c_void,
    metadata_hook: Option<unsafe fn(*mut c_void)>,
    hook_kind: u8,
    hook_target: *mut c_void,
    hook_child: bool,
}
unsafe extern "C" fn retain(pointer: *mut c_void) -> u32 {
    unsafe { &*pointer.cast::<Node>() }
        .refs
        .fetch_add(1, Ordering::Relaxed)
        + 1
}
unsafe extern "C" fn release(pointer: *mut c_void) -> u32 {
    unsafe { &*pointer.cast::<Node>() }
        .refs
        .fetch_sub(1, Ordering::Relaxed)
        - 1
}
unsafe extern "C" fn query_node(
    pointer: *mut c_void,
    iid: *const Uid,
    out: *mut *mut c_void,
) -> i32 {
    unsafe { *out = ptr::null_mut() };
    if unsafe { *iid } != UNKNOWN
        && unsafe { *iid } != NODE
        && unsafe { *iid } != REGISTRY
        && unsafe { *iid } != ALLOCATOR
    {
        return NO_INTERFACE;
    }
    if unsafe { &*pointer.cast::<Node>() }.reject_canonical {
        unsafe { *out = ptr::dangling_mut::<c_void>() };
        return NO_INTERFACE;
    }
    let foreign = unsafe { &*pointer.cast::<Node>() }.foreign_interface;
    let result = if unsafe { *iid } != UNKNOWN && !foreign.is_null() {
        foreign
    } else {
        pointer
    };
    unsafe {
        retain(result);
        *out = result;
    }
    OK
}
unsafe extern "C" fn parent(pointer: *mut c_void) -> *mut c_void {
    unsafe { &*pointer.cast::<Node>() }.parent
}
unsafe extern "C" fn child(pointer: *mut c_void, _: *const PlainString) -> *mut c_void {
    unsafe { &*pointer.cast::<Node>() }.child
}
unsafe fn metadata_hook(pointer: *mut c_void, kind: u8) {
    let node = unsafe { &*pointer.cast::<Node>() };
    let callback = if node.hook_kind == kind {
        node.metadata_hook
    } else {
        None
    };
    if let Some(callback) = callback {
        unsafe { callback(pointer) };
    }
}
unsafe fn detach_edge(pointer: *mut c_void) {
    let target = unsafe { (*pointer.cast::<Node>()).hook_target.cast::<Node>() };
    if unsafe { (*pointer.cast::<Node>()).hook_child } {
        unsafe { ptr::addr_of_mut!((*target).child).write(ptr::null_mut()) };
    } else {
        unsafe { ptr::addr_of_mut!((*target).parent).write(ptr::null_mut()) };
    }
}
unsafe fn arm_anchor_uid(pointer: *mut c_void) {
    let target = unsafe { (*pointer.cast::<Node>()).hook_target.cast::<Node>() };
    unsafe {
        (*target).metadata_hook = Some(detach_edge);
        (*target).hook_kind = 2;
        (*target).hook_target = target.cast();
        (*target).hook_child = false;
    }
}
unsafe extern "C" fn object_uid(pointer: *mut c_void) -> *const Uid {
    unsafe { metadata_hook(pointer, 1) };
    unsafe { ptr::addr_of!((*pointer.cast::<Node>()).object_uid) }
}
unsafe extern "C" fn class_uid(pointer: *mut c_void) -> *const Uid {
    unsafe { metadata_hook(pointer, 2) };
    unsafe { ptr::addr_of!((*pointer.cast::<Node>()).class_uid) }
}

struct Fixture {
    _table: Box<[*const c_void; 11]>,
    nodes: [Box<Node>; 7],
}
impl Fixture {
    fn new() -> Self {
        let mut table = Box::new([ptr::null(); 11]);
        table[0] = query_node as *const c_void;
        table[1] = retain as *const c_void;
        table[2] = release as *const c_void;
        table[4] = object_uid as *const c_void;
        table[5] = class_uid as *const c_void;
        table[6] = parent as *const c_void;
        table[10] = child as *const c_void;
        let mut nodes: [Box<Node>; 7] = std::array::from_fn(|_| {
            Box::new(Node {
                table: table.as_ptr(),
                refs: AtomicU32::new(1),
                parent: ptr::null_mut(),
                child: ptr::null_mut(),
                object_uid: SERVICE_CID,
                class_uid: API_CLASS,
                reject_canonical: false,
                foreign_interface: ptr::null_mut(),
                metadata_hook: None,
                hook_kind: 0,
                hook_target: ptr::null_mut(),
                hook_child: false,
            })
        });
        for index in 0..3 {
            nodes[index].parent = ptr::addr_of_mut!(*nodes[index + 1]).cast();
        }
        for index in 1..4 {
            nodes[index].child = ptr::addr_of_mut!(*nodes[index - 1]).cast();
        }
        Self {
            _table: table,
            nodes,
        }
    }
    fn pointer(&mut self, index: usize) -> *mut c_void {
        ptr::addr_of_mut!(*self.nodes[index]).cast()
    }
    fn route<G: FnMut() -> Result<(), String>>(
        &mut self,
        auxiliary: usize,
        guard: &mut G,
    ) -> Result<AnchorFolderRoute, String> {
        let device = Pin::borrowed(self.pointer(0))?;
        let auxiliary = Pin::borrowed(self.pointer(auxiliary))?;
        AnchorFolderRoute::resolve(device, auxiliary, [ptr::null(); 3], guard)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        for node in &self.nodes {
            assert_eq!(node.refs.load(Ordering::Relaxed), 1);
        }
    }
}

#[test]
fn exact_parent_and_child_route_retains_only_the_supplied_anchor_folder() {
    let mut fixture = Fixture::new();
    let route = fixture.route(2, &mut || Ok(())).unwrap();
    assert!(route.check().is_ok());
    assert_eq!(fixture.nodes[2].refs.load(Ordering::Relaxed), 5);
    drop(route);
}

#[test]
fn same_document_environment_is_not_enough_for_a_foreign_folder() {
    let mut fixture = Fixture::new();
    assert!(fixture.route(4, &mut || Ok(())).is_err());
}

#[test]
fn any_borrowed_parent_or_child_replacement_invalidates_the_route() {
    for edge in 0..6 {
        let mut fixture = Fixture::new();
        let route = fixture.route(2, &mut || Ok(())).unwrap();
        let foreign = fixture.pointer(4);
        match edge {
            0 => fixture.nodes[0].parent = foreign,
            1 => fixture.nodes[1].parent = foreign,
            2 => fixture.nodes[2].parent = foreign,
            3 => fixture.nodes[3].child = foreign,
            4 => fixture.nodes[2].child = foreign,
            _ => fixture.nodes[1].child = foreign,
        }
        assert!(route.check().is_err(), "edge {edge}");
        drop(route);
    }
}

#[test]
fn folder_slot_and_channel_uid_drift_invalidates_without_pointer_changes() {
    for index in 1..4 {
        let mut fixture = Fixture::new();
        let route = fixture.route(2, &mut || Ok(())).unwrap();
        fixture.nodes[index].object_uid = UNKNOWN;
        assert!(route.check().is_err());
        drop(route);
    }
}

#[test]
fn missing_parent_and_context_switch_during_capture_return_no_route() {
    let mut fixture = Fixture::new();
    fixture.nodes[1].parent = ptr::null_mut();
    assert!(fixture.route(2, &mut || Ok(())).is_err());
    for failure in 1..=5 {
        let mut fixture = Fixture::new();
        let mut calls = 0;
        let mut guard = || {
            calls += 1;
            if calls >= failure {
                Err("document switched".into())
            } else {
                Ok(())
            }
        };
        assert!(fixture.route(2, &mut guard).is_err());
    }
}

#[test]
fn failed_nonnull_canonical_query_never_owns_or_releases_the_sentinel() {
    let mut fixture = Fixture::new();
    fixture.nodes[0].reject_canonical = true;
    assert!(Pin::borrowed(fixture.pointer(0)).is_err());
    assert_eq!(fixture.nodes[0].refs.load(Ordering::Relaxed), 1);
}

#[test]
fn unavailable_backend_and_binding_failures_stay_distinct() {
    assert!(matches!(
        BackendFailure::from(fender_native_preset_api::Error::Unavailable("no handler")),
        BackendFailure::Unavailable(_)
    ));
    for error in [
        fender_native_preset_api::Error::Changed,
        fender_native_preset_api::Error::WrongThread,
        fender_native_preset_api::Error::Guard("document changed".into()),
    ] {
        assert!(matches!(
            BackendFailure::from(error),
            BackendFailure::Binding(_)
        ));
    }
}

#[test]
fn invalidation_removes_every_preflight_success_field() {
    let mut value = json!({"scope":"insert_folder_preset_preflight_observation", "status":"observed",
        "reason":"old", "folder":{}, "anchor_slot_bound":true,"document_environment_observation":{},"target_flags":3,"metadata":{}});
    invalidate(&mut value);
    assert_eq!(
        value,
        json!({"scope":"insert_folder_preset_preflight_observation"})
    );
}

#[test]
fn host_service_queries_require_the_lookup_objects_canonical_identity() {
    let mut fixture = Fixture::new();
    let source = hold(fixture.pointer(0)).unwrap();
    let typed = interface(&source, &REGISTRY, &mut || Ok(())).unwrap();
    drop(typed);
    fixture.nodes[0].foreign_interface = fixture.pointer(4);
    assert!(interface(&source, &REGISTRY, &mut || Ok(())).is_err());
    drop(source);
}

#[test]
fn host_service_query_context_loss_releases_known_owners_before_return() {
    for failure in 1..=5 {
        let mut fixture = Fixture::new();
        let source = hold(fixture.pointer(0)).unwrap();
        let mut count = 0;
        let result = interface(&source, &ALLOCATOR, &mut || {
            count += 1;
            if count >= failure {
                Err("document switched".into())
            } else {
                Ok(())
            }
        });
        assert!(result.is_err());
        drop(source);
    }
}

static TABLE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ALLOCATOR_OBJECT: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
unsafe extern "C" fn table_export() -> *mut c_void {
    TABLE.load(Ordering::Acquire)
}
unsafe extern "C" fn allocator_export() -> *mut c_void {
    ALLOCATOR_OBJECT.load(Ordering::Acquire)
}
unsafe extern "C" fn no_service() -> *mut c_void {
    ptr::null_mut()
}
unsafe extern "C" fn unused_text(_: *const c_char) -> *const PlainCString {
    ptr::null()
}
unsafe extern "C" fn unused_thread() -> i64 {
    0
}
unsafe extern "C" fn registry_lookup(pointer: *mut c_void, iid: *const Uid) -> *mut c_void {
    assert!(unsafe { *iid } == REGISTRY);
    unsafe { &*pointer.cast::<Node>() }.child
}
fn service_api() -> Api {
    Api {
        gui: no_service,
        objects: table_export,
        main_thread: no_service,
        thread_self: unused_thread,
        cstring: unused_text,
        empty: no_service,
        desktop: no_service,
        scripting: no_service,
        plugin_manager: allocator_export,
        file_system: no_service,
    }
}
fn clear_services() {
    TABLE.store(ptr::null_mut(), Ordering::Release);
    ALLOCATOR_OBJECT.store(ptr::null_mut(), Ordering::Release);
}

#[test]
fn fixed_service_acquisition_handles_absence_and_transfers_only_successful_owners() {
    let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut fixture = Fixture::new();
    fixture._table[8] = registry_lookup as *const c_void;
    let pin = Pin::borrowed(fixture.pointer(2)).unwrap();
    let api = service_api();
    clear_services();
    assert!(inputs(&api, &pin, &mut || Ok(())).is_err());
    TABLE.store(fixture.pointer(4), Ordering::Release);
    assert!(inputs(&api, &pin, &mut || Ok(())).is_err());
    fixture.nodes[4].child = fixture.pointer(5);
    assert!(inputs(&api, &pin, &mut || Ok(())).is_err());
    ALLOCATOR_OBJECT.store(fixture.pointer(6), Ordering::Release);
    let transferred = inputs(&api, &pin, &mut || Ok(())).unwrap();
    drop(Owned(transferred.folder.as_ptr()));
    drop(Owned(transferred.registry.as_ptr()));
    drop(Owned(transferred.allocator.as_ptr()));
    drop(pin);
    clear_services();
}

#[test]
fn service_callbacks_cannot_publish_transferred_inputs_after_attachment_loss() {
    let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
    for failure in 1..=14 {
        let mut fixture = Fixture::new();
        fixture._table[8] = registry_lookup as *const c_void;
        fixture.nodes[4].child = fixture.pointer(5);
        TABLE.store(fixture.pointer(4), Ordering::Release);
        ALLOCATOR_OBJECT.store(fixture.pointer(6), Ordering::Release);
        let pin = Pin::borrowed(fixture.pointer(2)).unwrap();
        let mut calls = 0;
        let result = inputs(&service_api(), &pin, &mut || {
            calls += 1;
            if calls >= failure {
                Err("attachment changed".into())
            } else {
                Ok(())
            }
        });
        assert!(result.is_err(), "guard boundary {failure}");
        drop(pin);
        clear_services();
    }
}

#[test]
fn uid_metadata_callbacks_cannot_publish_detached_parent_or_reverse_child_edges() {
    for trigger in 1..=3 {
        for kind in 1..=2 {
            for edge in 0..6 {
                let mut fixture = Fixture::new();
                let route = fixture.route(2, &mut || Ok(())).unwrap();
                let target = fixture.pointer([0, 1, 2, 3, 2, 1][edge]);
                fixture.nodes[trigger].metadata_hook = Some(detach_edge);
                fixture.nodes[trigger].hook_kind = kind;
                fixture.nodes[trigger].hook_target = target;
                fixture.nodes[trigger].hook_child = edge >= 3;
                assert!(
                    route.check().is_err(),
                    "node {trigger} getter {kind} edge {edge}"
                );
            }
        }
    }
}

fn guarded_preflight() -> (
    super::super::guard_tests::Fixture,
    Fixture,
    InsertFolderPreflight,
    Box<Context<'static>>,
) {
    let mut host = super::super::guard_tests::Fixture::new();
    host.canonical_queries();
    let mut ctx = host.context();
    let ancestry = super::super::guard_tests::inspection_for(&ctx).into_auxiliary();
    let mut fixture = Fixture::new();
    ctx.node = hold(fixture.pointer(0)).unwrap();
    let anchor = fixture.route(2, &mut || Ok(())).unwrap();
    let preflight = InsertFolderPreflight {
        availability: Availability::Unavailable("test backend unavailable".into()),
        ancestry,
        anchor,
    };
    (host, fixture, preflight, ctx)
}

fn anchor_uid_callback_refused(publication: bool) {
    let _serial = super::super::super::tests::TEST_LOCK.lock().unwrap();
    let (host, mut fixture, preflight, ctx) = guarded_preflight();
    let target = fixture.pointer(0);
    fixture.nodes[3].metadata_hook = Some(arm_anchor_uid);
    fixture.nodes[3].hook_kind = 2;
    fixture.nodes[3].hook_target = target;
    if publication {
        let service = ctx.service;
        let mut reply = Reply {
            value: json!({"ok":true,"outcome":"observed","scope":"insert_folder_preset_preflight_observation","status":"observed","anchor_slot_bound":true,"folder":{},"target_flags":3,"metadata":{}}),
            catalog: None,
            action: None,
            presets: None,
            auxiliary: None,
            folder_preflight: Some(preflight),
            owner: None,
            nodes: None,
            provider: None,
            context: Some(ctx),
            mutation: false,
        };
        assert!(!reply.final_check(service));
        assert_eq!(reply.value["ok"], false);
        assert!(reply.value.get("anchor_slot_bound").is_none());
        assert!(reply.value.get("folder").is_none());
        drop(reply);
    } else {
        assert!(preflight.check(&ctx).is_err());
        drop(preflight);
        drop(ctx);
    }
    assert!(fixture.nodes[0].parent.is_null());
    drop(fixture);
    drop(host);
}

#[test]
fn anchor_uid_callback_cannot_publish_preflight_after_final_edges() {
    anchor_uid_callback_refused(false);
}

#[test]
fn anchor_uid_callback_cannot_publish_reply_after_final_preflight() {
    anchor_uid_callback_refused(true);
}

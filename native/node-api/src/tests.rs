use super::*;
use ffi::{MethodDefinition, PropertyDefinition};
use std::{
    cell::{Cell, RefCell},
    ffi::CString,
    rc::Rc,
};

#[repr(C)]
struct Interface {
    table: *const *const c_void,
    owner: *mut Mock,
}
#[repr(C)]
struct Mock {
    interfaces: [Interface; 5],
    refs: Cell<u32>,
    node_supported: bool,
    object_supported: bool,
    text: Vec<u16>,
    text_repr: PlainString,
    uid: Uid,
    class_uid: Uid,
    class_name: CString,
    parent: Cell<*mut Mock>,
    type_parent: Cell<*mut Mock>,
    children: RefCell<Vec<*mut Mock>>,
    delegates: RefCell<HashMap<String, *mut Mock>>,
    count_override: Cell<Option<i32>>,
    methods: Vec<MethodDefinition>,
    properties: Vec<PropertyDefinition>,
    child_reads: Cell<usize>,
    replacement: Cell<*mut Mock>,
    replace_at: Cell<usize>,
    fail_on_release: Cell<bool>,
    lost: Rc<Cell<bool>>,
}
const ZERO: Uid = Uid {
    a: 0,
    b: 0,
    c: 0,
    d: [0; 8],
};
const ID: Uid = Uid {
    a: 0x12345678,
    b: 0x1234,
    c: 0x1234,
    d: [0x12; 8],
};
unsafe fn mock<'a>(p: *mut c_void) -> &'a Mock {
    unsafe { &*(*p.cast::<Interface>()).owner }
}
unsafe extern "C" fn retain(p: *mut c_void) -> u32 {
    let m = unsafe { mock(p) };
    m.refs.set(m.refs.get() + 1);
    m.refs.get()
}
unsafe extern "C" fn release(p: *mut c_void) -> u32 {
    let m = unsafe { mock(p) };
    m.refs.set(m.refs.get() - 1);
    if m.fail_on_release.get() {
        m.lost.set(true);
    }
    m.refs.get()
}
unsafe extern "C" fn query(p: *mut c_void, iid: *const Uid, out: *mut *mut c_void) -> i32 {
    let m = unsafe { mock(p) };
    let iid = unsafe { *iid };
    let slot = if iid == ffi::UNKNOWN {
        Some(0)
    } else if iid == ffi::NODE && m.node_supported {
        Some(1)
    } else if iid == ffi::OBJECT && m.object_supported {
        Some(2)
    } else {
        None
    };
    unsafe { *out = std::ptr::null_mut() };
    if let Some(slot) = slot {
        let raw = (&m.interfaces[slot] as *const Interface).cast_mut().cast();
        unsafe {
            retain(raw);
            *out = raw;
        }
        0
    } else {
        -1
    }
}
unsafe extern "C" fn node_id(p: *mut c_void) -> *const PlainString {
    &unsafe { mock(p) }.text_repr
}
unsafe extern "C" fn node_uid(p: *mut c_void) -> *const Uid {
    &unsafe { mock(p) }.uid
}
unsafe extern "C" fn node_class(p: *mut c_void) -> *const Uid {
    &unsafe { mock(p) }.class_uid
}
unsafe extern "C" fn node_parent(p: *mut c_void) -> *mut c_void {
    let parent = unsafe { mock(p) }.parent.get();
    if parent.is_null() {
        std::ptr::null_mut()
    } else {
        unsafe { &mut (*parent).interfaces[1] as *mut Interface }.cast()
    }
}
unsafe extern "C" fn node_count(p: *mut c_void) -> i32 {
    let m = unsafe { mock(p) };
    m.count_override
        .get()
        .unwrap_or(m.children.borrow().len() as i32)
}
unsafe extern "C" fn node_child(p: *mut c_void, index: i32) -> *mut c_void {
    let m = unsafe { mock(p) };
    let child = m
        .children
        .borrow()
        .get(index as usize)
        .copied()
        .unwrap_or(std::ptr::null_mut());
    m.child_reads.set(m.child_reads.get() + 1);
    if !m.replacement.get().is_null() && m.child_reads.get() == m.replace_at.get() {
        m.children.borrow_mut()[index as usize] = m.replacement.get();
    }
    if child.is_null() {
        std::ptr::null_mut()
    } else {
        unsafe { &mut (*child).interfaces[1] as *mut Interface }.cast()
    }
}
unsafe extern "C" fn node_find(p: *mut c_void, key: *const c_void) -> *mut c_void {
    let key = unsafe { CStr::from_ptr(key.cast()) }.to_str().unwrap();
    let child = unsafe { mock(p) }
        .delegates
        .borrow()
        .get(key)
        .copied()
        .unwrap_or(std::ptr::null_mut());
    if child.is_null() {
        std::ptr::null_mut()
    } else {
        unsafe { &mut (*child).interfaces[1] as *mut Interface }.cast()
    }
}
unsafe extern "C" fn object_type(p: *mut c_void) -> *mut c_void {
    (&unsafe { mock(p) }.interfaces[3] as *const Interface)
        .cast_mut()
        .cast()
}
unsafe extern "C" fn type_parent(p: *mut c_void) -> *mut c_void {
    let parent = unsafe { mock(p) }.type_parent.get();
    if parent.is_null() {
        std::ptr::null_mut()
    } else {
        unsafe { &mut (*parent).interfaces[3] as *mut Interface }.cast()
    }
}
unsafe extern "C" fn type_flags(_: *mut c_void) -> i32 {
    2
}
unsafe extern "C" fn type_name(p: *mut c_void) -> *const c_char {
    unsafe { mock(p) }.class_name.as_ptr()
}
unsafe extern "C" fn type_namespace(_: *mut c_void) -> *const c_char {
    c"Testing".as_ptr()
}
unsafe extern "C" fn type_methods(p: *mut c_void) -> *const MethodDefinition {
    unsafe { mock(p) }.methods.as_ptr()
}
unsafe extern "C" fn type_properties(p: *mut c_void) -> *const PropertyDefinition {
    unsafe { mock(p) }.properties.as_ptr()
}
unsafe extern "C" fn string_length(p: *mut c_void) -> i32 {
    unsafe { mock(p) }.text.len() as i32
}
unsafe extern "C" fn string_copy(p: *mut c_void, out: *mut u16, cap: i32) -> i32 {
    let text = &unsafe { mock(p) }.text;
    if cap as usize <= text.len() {
        return -1;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(text.as_ptr(), out, text.len());
        *out.add(text.len()) = 0;
    }
    0
}
unsafe extern "C" fn constant(text: *const c_char) -> *const c_void {
    text.cast()
}

struct Tables {
    values: [[*const c_void; 11]; 5],
}
impl Tables {
    fn new() -> Self {
        let mut values = [[std::ptr::null(); 11]; 5];
        for table in &mut values {
            table[0] = query as *const c_void;
            table[1] = retain as *const c_void;
            table[2] = release as *const c_void;
        }
        values[1][3] = node_id as *const c_void;
        values[1][4] = node_uid as *const c_void;
        values[1][5] = node_class as *const c_void;
        values[1][6] = node_parent as *const c_void;
        values[1][8] = node_count as *const c_void;
        values[1][9] = node_child as *const c_void;
        values[1][10] = node_find as *const c_void;
        values[2][3] = object_type as *const c_void;
        values[3][3] = type_parent as *const c_void;
        values[3][4] = type_flags as *const c_void;
        values[3][5] = type_name as *const c_void;
        values[3][6] = type_namespace as *const c_void;
        values[3][7] = node_class as *const c_void;
        values[3][8] = type_methods as *const c_void;
        values[3][9] = type_properties as *const c_void;
        values[4][4] = string_length as *const c_void;
        values[4][8] = string_copy as *const c_void;
        Self { values }
    }
    fn object(&self, id: &str) -> Box<Mock> {
        let mut m = Box::new(Mock {
            interfaces: std::array::from_fn(|i| Interface {
                table: self.values[i].as_ptr(),
                owner: std::ptr::null_mut(),
            }),
            refs: Cell::new(1),
            node_supported: true,
            object_supported: true,
            text: id.encode_utf16().collect(),
            text_repr: PlainString {
                object: std::ptr::null_mut(),
                private: std::ptr::null_mut(),
            },
            uid: ID,
            class_uid: ZERO,
            class_name: CString::new(id).unwrap(),
            parent: Cell::new(std::ptr::null_mut()),
            type_parent: Cell::new(std::ptr::null_mut()),
            children: RefCell::new(vec![]),
            delegates: RefCell::new(HashMap::new()),
            count_override: Cell::new(None),
            methods: vec![
                MethodDefinition {
                    name: c"readGraph".as_ptr(),
                    args: c"index: int".as_ptr(),
                    retval: c"object".as_ptr(),
                },
                MethodDefinition {
                    name: std::ptr::null(),
                    args: std::ptr::null(),
                    retval: std::ptr::null(),
                },
            ],
            properties: vec![
                PropertyDefinition {
                    name: c"title".as_ptr(),
                    data_type: 0x10301,
                    type_name: c"string".as_ptr(),
                    type_info: std::ptr::null(),
                },
                PropertyDefinition {
                    name: std::ptr::null(),
                    data_type: 0,
                    type_name: std::ptr::null(),
                    type_info: std::ptr::null(),
                },
            ],
            child_reads: Cell::new(0),
            replacement: Cell::new(std::ptr::null_mut()),
            replace_at: Cell::new(0),
            fail_on_release: Cell::new(false),
            lost: Rc::new(Cell::new(false)),
        });
        let owner = &mut *m as *mut Mock;
        for interface in &mut m.interfaces {
            interface.owner = owner;
        }
        m.text_repr.object = (&mut m.interfaces[4] as *mut Interface).cast();
        m
    }
}
fn inspector(mock: &mut Mock) -> NodeInspector {
    unsafe {
        NodeInspector::from_owned(
            NonNull::new((&mut mock.interfaces[0] as *mut Interface).cast()).unwrap(),
            TextApi {
                get_constant_string: constant,
            },
        )
    }
}
fn minimal() -> Limits {
    Limits {
        nodes: 8,
        depth: 1,
        ancestors: 0,
        children_per_node: 8,
    }
}

#[test]
fn inventories_native_identity_and_reflected_signatures_without_class_uid() {
    let tables = Tables::new();
    let mut root = tables.object("Graph");
    let api = inspector(&mut root);
    let result = api.inspect(minimal(), &mut || Ok(())).unwrap();
    assert!(result.complete);
    assert_eq!(result.nodes.len(), 1);
    let identity = result.nodes[0].identity.as_ref().unwrap();
    assert_eq!(identity.id, "Graph");
    assert!(identity.class_uid.is_none());
    assert!(identity.object_uid.is_some());
    let reflected = &result.nodes[0].reflection.types[0];
    assert_eq!(reflected.name, "Graph");
    assert_eq!(reflected.methods[0].name, "readGraph");
    assert_eq!(
        reflected.methods[0].arguments.as_deref(),
        Some("index: int")
    );
    assert_eq!(reflected.properties[0].name, "title");
    assert_eq!(reflected.properties[0].data_type, 0x10301);
    assert!(root.refs.get() > 1);
    drop(result);
    assert_eq!(root.refs.get(), 1);
    drop(api);
    assert_eq!(root.refs.get(), 0);
}
#[test]
fn children_and_fixed_delegates_share_one_canonical_record() {
    let tables = Tables::new();
    let mut root = tables.object("Graph");
    let mut child = tables.object("Device");
    root.children.borrow_mut().push(&mut *child);
    root.delegates
        .borrow_mut()
        .insert("Device".into(), &mut *child);
    let api = inspector(&mut root);
    let result = api.inspect(minimal(), &mut || Ok(())).unwrap();
    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.edges.len(), 2);
    assert!(
        result
            .edges
            .iter()
            .any(|e| e.relation == Relation::Delegate(Delegate::Device) && e.to == 1)
    );
    assert!(
        result
            .edges
            .iter()
            .any(|e| e.relation == Relation::Child(0) && e.to == 1)
    );
    assert!(result.complete);
    drop(result);
    assert_eq!(child.refs.get(), 1);
    drop(api);
}
#[test]
fn borrowed_child_is_pinned_before_the_next_context_guard() {
    let tables = Tables::new();
    let mut root = tables.object("Graph");
    let mut child = tables.object("Device");
    root.children.borrow_mut().push(&mut *child);
    let checked = Cell::new(false);
    let api = inspector(&mut root);
    api.inspect(minimal(), &mut || {
        if root.child_reads.get() == 1 && !checked.replace(true) {
            assert!(child.refs.get() >= 2);
        }
        Ok(())
    })
    .unwrap();
    assert!(checked.get());
    assert_eq!(child.refs.get(), 1);
    drop(api);
}
#[test]
fn ancestor_records_keep_parent_edges_and_report_parent_cycles() {
    let tables = Tables::new();
    let mut root = tables.object("Device");
    let mut parent = tables.object("Slot");
    root.parent.set(&mut *parent);
    parent.parent.set(&mut *root);
    let api = inspector(&mut root);
    let mut limits = minimal();
    limits.ancestors = 3;
    let result = api.inspect(limits, &mut || Ok(())).unwrap();
    assert!(!result.complete);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|x| matches!(x, Diagnostic::NodeCycle { .. }))
    );
    assert_eq!(result.nodes.len(), 2);
    drop(result);
    assert_eq!(parent.refs.get(), 1);
    drop(api);
}
#[test]
fn child_cycle_is_bounded_and_explicit() {
    let tables = Tables::new();
    let mut root = tables.object("Graph");
    let pointer = &mut *root as *mut Mock;
    root.children.borrow_mut().push(pointer);
    let api = inspector(&mut root);
    let result = api.inspect(minimal(), &mut || Ok(())).unwrap();
    assert_eq!(result.nodes.len(), 1);
    assert!(
        result
            .diagnostics
            .contains(&Diagnostic::NodeCycle { from: 0, to: 0 })
    );
    assert!(!result.complete);
    drop(api);
}
#[test]
fn type_parent_chain_is_owned_and_cycle_is_reported() {
    let tables = Tables::new();
    let mut root = tables.object("Graph");
    let mut parent = tables.object("Component");
    root.type_parent.set(&mut *parent);
    parent.type_parent.set(&mut *root);
    let api = inspector(&mut root);
    let result = api.inspect(minimal(), &mut || Ok(())).unwrap();
    assert_eq!(result.nodes[0].reflection.types.len(), 2);
    assert!(
        result.nodes[0]
            .reflection
            .issues
            .contains(&MetadataIssue::TypeCycle)
    );
    assert!(!result.complete);
    drop(result);
    assert_eq!(parent.refs.get(), 1);
    drop(api);
}
#[test]
fn invalid_child_counts_fail_without_leaking_native_owners() {
    for count in [-1, 1_000_001, 1] {
        let tables = Tables::new();
        let mut root = tables.object("Graph");
        root.count_override.set(Some(count));
        let api = inspector(&mut root);
        assert!(matches!(
            api.inspect(minimal(), &mut || Ok(())),
            Err(Error::InvalidData(_))
        ));
        assert_eq!(root.refs.get(), 1);
        drop(api);
    }
}
#[test]
fn node_depth_and_child_limits_are_explicit_partial_reports() {
    let tables = Tables::new();
    let mut root = tables.object("Graph");
    let mut a = tables.object("A");
    let mut b = tables.object("B");
    root.children
        .borrow_mut()
        .extend([&mut *a as *mut Mock, &mut *b as *mut Mock]);
    let api = inspector(&mut root);
    let result = api
        .inspect(
            Limits {
                nodes: 1,
                ..minimal()
            },
            &mut || Ok(()),
        )
        .unwrap();
    assert!(result.diagnostics.contains(&Diagnostic::NodeLimit));
    assert!(!result.complete);
    drop(result);
    let result = api
        .inspect(
            Limits {
                depth: 0,
                ..minimal()
            },
            &mut || Ok(()),
        )
        .unwrap();
    assert!(result.diagnostics.contains(&Diagnostic::DepthLimit(0)));
    assert!(!result.complete);
    drop(result);
    let result = api
        .inspect(
            Limits {
                children_per_node: 1,
                ..minimal()
            },
            &mut || Ok(()),
        )
        .unwrap();
    assert!(result.diagnostics.contains(&Diagnostic::ChildLimit(0)));
    assert!(!result.complete);
    drop(result);
    assert_eq!(a.refs.get(), 1);
    assert_eq!(b.refs.get(), 1);
    drop(api);
}
#[test]
fn same_count_child_replacement_invalidates_the_inventory() {
    let tables = Tables::new();
    let mut root = tables.object("Graph");
    let mut a = tables.object("A");
    let mut b = tables.object("B");
    root.children.borrow_mut().push(&mut *a);
    root.replacement.set(&mut *b);
    root.replace_at.set(1);
    let api = inspector(&mut root);
    assert_eq!(
        api.inspect(minimal(), &mut || Ok(())).unwrap_err(),
        Error::AttachmentChanged
    );
    assert_eq!(a.refs.get(), 1);
    assert_eq!(b.refs.get(), 1);
    drop(api);
}
#[test]
fn non_node_objects_still_expose_readonly_reflection() {
    let tables = Tables::new();
    let mut root = tables.object("GraphEditFunctions");
    root.node_supported = false;
    let api = inspector(&mut root);
    let result = api.inspect(minimal(), &mut || Ok(())).unwrap();
    assert!(result.complete);
    assert!(result.nodes[0].identity.is_none());
    assert!(result.edges.is_empty());
    assert_eq!(
        result.nodes[0].reflection.types[0].methods[0].name,
        "readGraph"
    );
    drop(api);
}
#[test]
fn metadata_and_string_bounds_do_not_read_beyond_their_prefix() {
    let tables = Tables::new();
    let mut root = tables.object("Graph");
    root.methods = vec![
        MethodDefinition {
            name: c"m".as_ptr(),
            args: std::ptr::null(),
            retval: std::ptr::null()
        };
        MAX_METHODS
    ];
    root.properties = vec![
        PropertyDefinition {
            name: c"p".as_ptr(),
            data_type: 0,
            type_name: std::ptr::null(),
            type_info: std::ptr::null()
        };
        MAX_PROPERTIES
    ];
    let api = inspector(&mut root);
    let result = api.inspect(minimal(), &mut || Ok(())).unwrap();
    assert_eq!(
        result.nodes[0].reflection.types[0].methods.len(),
        MAX_METHODS
    );
    assert_eq!(
        result.nodes[0].reflection.types[0].properties.len(),
        MAX_PROPERTIES
    );
    assert!(
        result.nodes[0]
            .reflection
            .issues
            .contains(&MetadataIssue::MethodLimit)
    );
    assert!(
        result.nodes[0]
            .reflection
            .issues
            .contains(&MetadataIssue::PropertyLimit)
    );
    assert!(!result.complete);
    drop(api);
    let mut root = tables.object("Graph");
    root.class_name = CString::new("x".repeat(MAX_CSTRING + 1)).unwrap();
    let api = inspector(&mut root);
    assert_eq!(
        api.inspect(minimal(), &mut || Ok(())).unwrap_err(),
        Error::TextTooLong
    );
    assert_eq!(root.refs.get(), 1);
    drop(api);
}
#[test]
fn context_loss_during_native_cleanup_cannot_publish_a_report() {
    let tables = Tables::new();
    let mut root = tables.object("Graph");
    root.fail_on_release.set(true);
    let lost = root.lost.clone();
    let api = inspector(&mut root);
    assert!(matches!(
        api.inspect(minimal(), &mut || if lost.get() {
            Err("document changed".into())
        } else {
            Ok(())
        }),
        Err(Error::Guard(_))
    ));
    assert_eq!(root.refs.get(), 1);
    drop(api);
    assert_eq!(root.refs.get(), 0);
}
#[test]
fn invalid_limits_do_not_access_the_native_target() {
    let tables = Tables::new();
    let mut root = tables.object("Graph");
    let api = inspector(&mut root);
    assert!(matches!(
        api.inspect(
            Limits {
                nodes: 0,
                ..minimal()
            },
            &mut || panic!("must not call")
        ),
        Err(Error::InvalidLimits)
    ));
    assert_eq!(root.refs.get(), 1);
    drop(api);
}

#[test]
fn retained_inspection_detects_replacement_before_publication_and_releases_pins() {
    let tables = Tables::new();
    let mut root = tables.object("Graph");
    let mut first = tables.object("Device");
    let mut replacement = tables.object("Device");
    root.children.borrow_mut().push(&mut *first);
    let api = inspector(&mut root);
    let lease = api.inspect(minimal(), &mut || Ok(())).unwrap();
    assert!(first.refs.get() > 1);
    lease.check(&mut || Ok(())).unwrap();
    root.children.borrow_mut()[0] = &mut *replacement;
    assert_eq!(
        lease.check(&mut || Ok(())).unwrap_err(),
        Error::AttachmentChanged
    );
    drop(lease);
    assert_eq!(first.refs.get(), 1);
    assert_eq!(replacement.refs.get(), 1);
    assert_eq!(root.refs.get(), 1);
    drop(api);
}

#[test]
fn inspection_refuses_reentry_from_its_context_guard() {
    let tables = Tables::new();
    let mut root = tables.object("Graph");
    let api = inspector(&mut root);
    let lease = api.inspect(minimal(), &mut || Ok(())).unwrap();
    let attempts = Cell::new(0);
    lease
        .check(&mut || {
            attempts.set(attempts.get() + 1);
            assert_eq!(lease.check(&mut || Ok(())).unwrap_err(), Error::Reentrant);
            Ok(())
        })
        .unwrap();
    assert!(attempts.get() > 0);
    lease.check(&mut || Ok(())).unwrap();
    drop(lease);
    assert_eq!(root.refs.get(), 1);
    drop(api);
}

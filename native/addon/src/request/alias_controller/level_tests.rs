use super::*;
use std::cell::Cell;

#[repr(C)]
struct Fake {
    table: *const *const c_void,
    refs: Cell<u32>,
    canonical: Cell<*mut c_void>,
    status: Cell<i32>,
    output: Cell<bool>,
    count: Cell<i32>,
    count_reads: Cell<usize>,
    parent: Cell<*mut c_void>,
    child: Cell<*mut c_void>,
    lost: Cell<bool>,
    lose_query: Cell<bool>,
    lose_count: Cell<bool>,
    lose_release: Cell<bool>,
}
unsafe fn object<'a>(p: *mut c_void) -> &'a Fake {
    unsafe { &*p.cast::<Fake>() }
}
unsafe extern "C" fn retain(p: *mut c_void) -> u32 {
    let f = unsafe { object(p) };
    f.refs.set(f.refs.get() + 1);
    f.refs.get()
}
unsafe extern "C" fn drop_ref(p: *mut c_void) -> u32 {
    let f = unsafe { object(p) };
    f.refs.set(f.refs.get() - 1);
    if f.lose_release.get() {
        f.lost.set(true)
    }
    f.refs.get()
}
unsafe extern "C" fn query(p: *mut c_void, iid: *const Uid, out: *mut *mut c_void) -> i32 {
    let f = unsafe { object(p) };
    let iid = unsafe { *iid };
    let result = if iid == UNKNOWN {
        OK
    } else if iid == CONTROLLER {
        f.status.get()
    } else {
        NO_INTERFACE
    };
    let ptr = if iid == UNKNOWN {
        f.canonical.get()
    } else if iid == CONTROLLER && f.output.get() {
        p
    } else {
        ptr::null_mut()
    };
    unsafe {
        *out = ptr;
        if !ptr.is_null() {
            retain(ptr);
        }
    }
    if f.lose_query.get() {
        f.lost.set(true)
    }
    result
}
unsafe extern "C" fn count(p: *mut c_void) -> i32 {
    let f = unsafe { object(p) };
    f.count_reads.set(f.count_reads.get() + 1);
    if f.lose_count.get() {
        f.lost.set(true)
    }
    f.count.get()
}
unsafe extern "C" fn children(p: *mut c_void) -> i32 {
    if unsafe { object(p) }.child.get().is_null() {
        0
    } else {
        1
    }
}
unsafe extern "C" fn child0(p: *mut c_void, index: i32) -> *mut c_void {
    assert_eq!(index, 0);
    unsafe { object(p) }.child.get()
}
unsafe extern "C" fn parent(p: *mut c_void) -> *mut c_void {
    unsafe { object(p) }.parent.get()
}

struct Fixture {
    objects: Box<[Fake; 3]>,
    _table: Box<[*const c_void; 10]>,
}
impl Fixture {
    fn new() -> Self {
        let mut table = Box::new([ptr::null(); 10]);
        table[0] = query as *const c_void;
        table[1] = retain as *const c_void;
        table[2] = drop_ref as *const c_void;
        table[3] = count as *const c_void;
        table[6] = parent as *const c_void;
        table[8] = children as *const c_void;
        table[9] = child0 as *const c_void;
        let f = Self {
            objects: Box::new(std::array::from_fn(|_| Fake {
                table: table.as_ptr(),
                refs: Cell::new(1),
                canonical: Cell::new(ptr::null_mut()),
                status: Cell::new(OK),
                output: Cell::new(true),
                count: Cell::new(0),
                count_reads: Cell::new(0),
                parent: Cell::new(ptr::null_mut()),
                child: Cell::new(ptr::null_mut()),
                lost: Cell::new(false),
                lose_query: Cell::new(false),
                lose_count: Cell::new(false),
                lose_release: Cell::new(false),
            })),
            _table: table,
        };
        for i in 0..3 {
            f.objects[i].canonical.set(f.pointer(i));
        }
        f.objects[0].child.set(f.pointer(1));
        f.objects[1].parent.set(f.pointer(0));
        f
    }
    fn pointer(&self, i: usize) -> *mut c_void {
        ptr::from_ref(&self.objects[i]).cast_mut().cast()
    }
    fn guard(&self) -> Result<(), String> {
        if self.objects.iter().any(|f| f.lost.get()) {
            Err("guard changed".into())
        } else {
            Ok(())
        }
    }
    fn pair(&self) -> Observation {
        let mut guard = || self.guard();
        Observation {
            pair: Some(Pair {
                root: pin(self.pointer(0), &mut guard).unwrap(),
                root_identity: pin(self.pointer(0), &mut guard).unwrap(),
                child: pin(self.pointer(1), &mut guard).unwrap(),
                child_identity: pin(self.pointer(1), &mut guard).unwrap(),
                capabilities: std::array::from_fn(|_| Capability {
                    interface: None,
                    value: Value::Null,
                }),
            }),
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        assert!(self.objects.iter().all(|f| f.refs.get() == 1));
    }
}

#[test]
fn declared_qi_outcomes_remain_distinct_without_parameter_calls() {
    for (result, output, expected) in [
        (NO_INTERFACE, false, "absent"),
        (OK, false, "malformed_success"),
        (NO_INTERFACE, true, "malformed_absence"),
        (-1, false, "qi_failed"),
        (-1, true, "qi_failed"),
    ] {
        let f = Fixture::new();
        f.objects[0].status.set(result);
        f.objects[0].output.set(output);
        let observation = capability(f.pointer(0), f.pointer(0), &mut || f.guard()).unwrap();
        assert_eq!(observation.value["status"], expected);
        assert_eq!(f.objects[0].count_reads.get(), 0);
        drop(observation);
    }
}
#[test]
fn canonical_mismatch_never_calls_count() {
    let f = Fixture::new();
    f.objects[0].canonical.set(f.pointer(2));
    let observation = capability(f.pointer(0), f.pointer(0), &mut || f.guard()).unwrap();
    assert_eq!(observation.value["status"], "identity_mismatch");
    assert_eq!(f.objects[0].count_reads.get(), 0);
}
#[test]
fn counts_are_bounded_and_never_enumerated() {
    for (count, status) in [
        (-1, "negative_count"),
        (0, "present"),
        (128, "present"),
        (129, "over_limit_count"),
        (i32::MAX, "over_limit_count"),
    ] {
        let f = Fixture::new();
        f.objects[0].count.set(count);
        let observation = capability(f.pointer(0), f.pointer(0), &mut || f.guard()).unwrap();
        assert_eq!(observation.value["status"], status);
        assert_eq!(f.objects[0].count_reads.get(), 1);
        assert_eq!(
            observation.value["count"],
            if status == "present" {
                json!(count)
            } else {
                Value::Null
            }
        );
    }
}
#[test]
fn query_count_and_release_reentry_fail_closed() {
    for phase in 0..3 {
        let f = Fixture::new();
        [
            &f.objects[0].lose_query,
            &f.objects[0].lose_count,
            &f.objects[0].lose_release,
        ][phase]
            .set(true);
        assert!(capability(f.pointer(0), f.pointer(0), &mut || f.guard()).is_err());
    }
}
#[test]
fn parent_or_child_replacement_invalidates_retained_observation() {
    for edge in 0..2 {
        let f = Fixture::new();
        let observation = f.pair();
        observation.check(&mut || f.guard()).unwrap();
        if edge == 0 {
            f.objects[0].child.set(f.pointer(2))
        } else {
            f.objects[1].parent.set(f.pointer(2))
        }
        assert!(observation.check(&mut || f.guard()).is_err());
    }
}
#[test]
fn guard_failure_at_every_capability_checkpoint_releases_all_owners() {
    let f = Fixture::new();
    let calls = Cell::new(0);
    drop(
        capability(f.pointer(0), f.pointer(0), &mut || {
            calls.set(calls.get() + 1);
            Ok(())
        })
        .unwrap(),
    );
    for fail in 1..=calls.get() {
        let f = Fixture::new();
        let seen = Cell::new(0);
        let result = capability(f.pointer(0), f.pointer(0), &mut || {
            seen.set(seen.get() + 1);
            if seen.get() >= fail {
                Err("changed".into())
            } else {
                Ok(())
            }
        });
        assert!(result.is_err(), "checkpoint {fail}");
        drop(result);
    }
}

fn shape() -> Report {
    use fender_native_node_api::{Edge, NodeIdentity, NodeRecord, TypeInfo, TypeInventory};
    Report {
        complete: true,
        diagnostics: vec![],
        edges: vec![Edge {
            from: 0,
            to: 1,
            relation: Relation::Child(0),
        }],
        nodes: [
            ("AudioEffect", "IsLevelMeterComponent", 1),
            ("OutLevels", "LevelMeterComponent", 0),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (id, family, count))| NodeRecord {
            index,
            identity: Some(NodeIdentity {
                id: id.into(),
                object_uid: None,
                class_uid: None,
            }),
            child_count: Some(count),
            declared_interfaces: None,
            reflection: TypeInventory {
                available: true,
                issues: vec![],
                types: vec![TypeInfo {
                    name: family.into(),
                    namespace: None,
                    class_uid: None,
                    flags: 0,
                    methods: vec![],
                    properties: vec![],
                }],
            },
        })
        .collect(),
    }
}
#[test]
fn fixed_metadata_shape_and_finite_output_are_required() {
    assert!(exact_shape(&shape()));
    for change in 0..6 {
        let mut report = shape();
        match change {
            0 => report.complete = false,
            1 => report.nodes[0].identity.as_mut().unwrap().id = "Other".into(),
            2 => report.nodes[1].reflection.types[0].name = "Other".into(),
            3 => report.nodes[0].child_count = Some(2),
            4 => report.nodes[1].child_count = Some(1),
            _ => report.edges[0].relation = Relation::Child(1),
        }
        assert!(!exact_shape(&report));
    }
    let unresolved = Observation { pair: None }.value();
    assert_eq!(unresolved["api"], API);
    assert_eq!(unresolved["status"], "unsupported_shape");
    assert!(unresolved["nodes"].is_null());
    assert_eq!(unresolved["mutation_capability"], false);
    assert_eq!(unresolved["atomic_snapshot"], false);
}
#[test]
fn retained_pair_rechecks_canonical_identity_and_context() {
    let f = Fixture::new();
    let observation = f.pair();
    f.objects[1].canonical.set(f.pointer(2));
    assert!(observation.check(&mut || f.guard()).is_err());
    f.objects[1].canonical.set(f.pointer(1));
    f.objects[1].lose_query.set(true);
    assert!(observation.check(&mut || f.guard()).is_err());
}

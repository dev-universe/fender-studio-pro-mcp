use super::super::parameters::CvPortParametersInventory;
use super::*;
use fender_native_parameter_api as pa;

#[repr(C)]
struct Control {
    table: *const *const c_void,
    refs: Cell<u32>,
    identity: Cell<*mut c_void>,
    parameters: RefCell<Vec<*mut c_void>>,
    count_override: Cell<Option<i32>>,
    at_calls: Cell<u32>,
    forbidden: Cell<u32>,
}
unsafe fn ctl<'a>(p: *mut c_void) -> &'a Control {
    unsafe { &*p.cast() }
}
unsafe extern "C" fn cr(p: *mut c_void) -> u32 {
    let c = unsafe { ctl(p) };
    c.refs.set(c.refs.get() + 1);
    c.refs.get()
}
unsafe extern "C" fn cd(p: *mut c_void) -> u32 {
    let c = unsafe { ctl(p) };
    c.refs.set(c.refs.get().checked_sub(1).unwrap());
    c.refs.get()
}
unsafe extern "C" fn cq(p: *mut c_void, iid: *const Uid, out: *mut *mut c_void) -> i32 {
    let c = unsafe { ctl(p) };
    unsafe { *out = ptr::null_mut() };
    if unsafe { *iid } == UNKNOWN {
        let raw = c.identity.get();
        unsafe {
            retain(raw);
            *out = raw
        };
        OK
    } else {
        c.forbidden.set(c.forbidden.get() + 1);
        NO_INTERFACE
    }
}
unsafe extern "C" fn cc(p: *mut c_void) -> i32 {
    let c = unsafe { ctl(p) };
    c.count_override
        .get()
        .unwrap_or(c.parameters.borrow().len() as i32)
}
unsafe extern "C" fn ca(p: *mut c_void, index: i32) -> *mut c_void {
    let c = unsafe { ctl(p) };
    c.at_calls.set(c.at_calls.get() + 1);
    c.parameters
        .borrow()
        .get(index as usize)
        .copied()
        .unwrap_or(ptr::null_mut())
}
unsafe extern "C" fn cf(p: *mut c_void, _: *const PlainCString) -> *mut c_void {
    let c = unsafe { ctl(p) };
    c.forbidden.set(c.forbidden.get() + 1);
    ptr::null_mut()
}

#[repr(C)]
struct Text {
    table: *const *const c_void,
    refs: Cell<u32>,
    text: RefCell<Vec<u16>>,
}
unsafe fn tx<'a>(p: *mut c_void) -> &'a Text {
    unsafe { &*p.cast() }
}
unsafe extern "C" fn tr(p: *mut c_void) -> u32 {
    let t = unsafe { tx(p) };
    t.refs.set(t.refs.get() + 1);
    t.refs.get()
}
unsafe extern "C" fn td(p: *mut c_void) -> u32 {
    let t = unsafe { tx(p) };
    t.refs.set(t.refs.get().checked_sub(1).unwrap());
    t.refs.get()
}
unsafe extern "C" fn tl(p: *mut c_void) -> i32 {
    unsafe { tx(p) }.text.borrow().len() as i32
}
unsafe extern "C" fn tc(p: *mut c_void, out: *mut u16, capacity: i32) -> i32 {
    let t = unsafe { tx(p) }.text.borrow();
    assert!(capacity as usize > t.len());
    unsafe {
        ptr::copy_nonoverlapping(t.as_ptr(), out, t.len());
        *out.add(t.len()) = 0
    };
    OK
}
unsafe extern "C" fn tp(_: *mut c_void) -> *mut c_void {
    ptr::null_mut()
}
thread_local! {static EMPTY:Cell<*mut c_void>=const{Cell::new(ptr::null_mut())};}
unsafe extern "C" fn empty() -> *mut c_void {
    EMPTY.with(Cell::get)
}
unsafe extern "C" fn cstring(name: *const c_char) -> *const c_void {
    name.cast()
}
fn text_api() -> pa::TextApi {
    pa::TextApi {
        get_empty_string: empty,
        get_constant_cstring: cstring,
    }
}
#[repr(C)]
struct StringOut {
    object: *mut c_void,
    private: *mut c_void,
}
#[repr(C)]
struct Variant {
    tag: i16,
    padding: [u8; 6],
    payload: u64,
}
#[repr(C)]
struct ObjectView {
    table: *const *const c_void,
    owner: *mut Parameter,
}
#[repr(C)]
struct Parameter {
    table: *const *const c_void,
    refs: Cell<u32>,
    object: ObjectView,
    list: ObjectView,
    precision: Cell<i32>,
    normalized: Cell<f32>,
    state: Cell<i32>,
    list_interface: Cell<bool>,
    list_reply: Cell<Option<(i32, *mut c_void)>>,
    precision_calls: Cell<u32>,
    flags_after_precision: Cell<u32>,
    list_query_calls: Cell<u32>,
    list_release_calls: Cell<u32>,
    precision_action: RefCell<Option<Box<dyn FnOnce()>>>,
    list_release_action: RefCell<Option<Box<dyn FnOnce()>>>,
    name: PlainCString,
    _name: std::ffi::CString,
    kind: Cell<i32>,
    values: [Cell<(i16, u64)>; 4],
    display: RefCell<String>,
    release_action: RefCell<Option<Box<dyn FnOnce()>>>,
    display_action: RefCell<Option<Box<dyn FnOnce()>>>,
    forbidden: Cell<u32>,
    fail_property: Cell<bool>,
    canonical_reply: Cell<Option<(i32, *mut c_void)>>,
    release_observer: RefCell<Option<Box<dyn FnMut()>>>,
}
unsafe fn par<'a>(p: *mut c_void) -> &'a Parameter {
    unsafe { &*p.cast() }
}
unsafe extern "C" fn pr(p: *mut c_void) -> u32 {
    let p = unsafe { par(p) };
    p.refs.set(p.refs.get() + 1);
    p.refs.get()
}
unsafe extern "C" fn pd(p: *mut c_void) -> u32 {
    let p = unsafe { par(p) };
    p.refs.set(p.refs.get().checked_sub(1).unwrap());
    if let Some(observe) = p.release_observer.borrow_mut().as_mut() {
        observe();
    }
    p.refs.get()
}
unsafe extern "C" fn pq(p: *mut c_void, iid: *const Uid, out: *mut *mut c_void) -> i32 {
    let f = unsafe { par(p) };
    unsafe { *out = ptr::null_mut() };
    let q = unsafe { *iid };
    let list_iid = Uid {
        a: 0x143641b5,
        b: 0x92a0,
        c: 0x4a2e,
        d: [0xb7, 0x0c, 0x84, 0xc5, 0x7e, 0x8f, 0xbd, 0xb3],
    };
    if q == list_iid {
        f.list_query_calls.set(f.list_query_calls.get() + 1);
        if let Some((status, pointer)) = f.list_reply.get() {
            unsafe { *out = pointer };
            return status;
        }
        if !f.list_interface.get() {
            return NO_INTERFACE;
        }
        unsafe {
            pr(p);
            *out = ptr::from_ref(&f.list).cast_mut().cast();
        }
        return OK;
    }
    if q == UNKNOWN {
        if let Some((status, pointer)) = f.canonical_reply.get() {
            unsafe { *out = pointer };
            return status;
        }
    }
    let raw = if q == UNKNOWN {
        p
    } else if q == OBJECT {
        ptr::from_ref(&f.object).cast_mut().cast()
    } else {
        f.forbidden.set(f.forbidden.get() + 1);
        return NO_INTERFACE;
    };
    unsafe {
        pr(p);
        *out = raw
    };
    OK
}
unsafe extern "C" fn oq(p: *mut c_void, iid: *const Uid, out: *mut *mut c_void) -> i32 {
    unsafe { pq((*p.cast::<ObjectView>()).owner.cast(), iid, out) }
}
unsafe extern "C" fn or(p: *mut c_void) -> u32 {
    unsafe { pr((*p.cast::<ObjectView>()).owner.cast()) }
}
unsafe extern "C" fn od(p: *mut c_void) -> u32 {
    let p = unsafe { (*p.cast::<ObjectView>()).owner };
    let action = unsafe { &*p }.release_action.borrow_mut().take();
    if let Some(action) = action {
        action()
    };
    unsafe { pd(p.cast()) }
}
unsafe extern "C" fn property(p: *mut c_void, out: *mut Variant, name: *const c_void) -> u8 {
    let p = unsafe { &*(*p.cast::<ObjectView>()).owner };
    if p.fail_property.get() {
        unsafe {
            (*out).tag = 3;
            (*out).payload = 0
        };
        return 1;
    }
    let name = unsafe { CStr::from_ptr(name.cast()) }.to_bytes();
    let index = match name {
        b"value" => 0,
        b"min" => 1,
        b"max" => 2,
        b"default" => 3,
        _ => return 0,
    };
    let (tag, payload) = p.values[index].get();
    unsafe {
        *out = Variant {
            tag,
            padding: [0; 6],
            payload,
        }
    };
    1
}
unsafe extern "C" fn pk(p: *mut c_void) -> i32 {
    unsafe { par(p) }.kind.get()
}
unsafe extern "C" fn pn(p: *mut c_void) -> *const PlainCString {
    &unsafe { par(p) }.name
}
unsafe extern "C" fn pe(_: *mut c_void) -> u8 {
    1
}
unsafe extern "C" fn ps(pointer: *mut c_void, mask: i32) -> u8 {
    let p = unsafe { par(pointer) };
    if p.precision_calls.get() > 0 {
        p.flags_after_precision.set(p.flags_after_precision.get() + 1);
    }
    (p.state.get() & mask != 0) as u8
}
unsafe extern "C" fn precision(pointer: *mut c_void) -> i32 {
    let p = unsafe { par(pointer) };
    p.precision_calls.set(p.precision_calls.get() + 1);
    let action = p.precision_action.borrow_mut().take();
    if let Some(action) = action {
        action();
    }
    p.precision.get()
}
unsafe extern "C" fn list_release(pointer: *mut c_void) -> u32 {
    let p = unsafe { &*(*pointer.cast::<ObjectView>()).owner };
    p.list_release_calls.set(p.list_release_calls.get() + 1);
    let action = p.list_release_action.borrow_mut().take();
    if let Some(action) = action {
        action();
    }
    unsafe { pd(ptr::from_ref(p).cast_mut().cast()) }
}
unsafe extern "C" fn pt(_: *mut c_void) -> i32 {
    17
}
unsafe extern "C" fn norm(pointer: *mut c_void) -> f32 {
    unsafe { par(pointer) }.normalized.get()
}
unsafe extern "C" fn formatter(_: *mut c_void) -> *mut c_void {
    ptr::null_mut()
}
unsafe extern "C" fn display(p: *mut c_void, out: *mut StringOut) {
    let p = unsafe { par(p) };
    let object = unsafe { (*out).object };
    *unsafe { tx(object) }.text.borrow_mut() = p.display.borrow().encode_utf16().collect();
    let action = p.display_action.borrow_mut().take();
    if let Some(action) = action {
        action()
    }
}
unsafe extern "C" fn forbidden(p: *mut c_void) -> *mut c_void {
    let p = unsafe { par(p) };
    p.forbidden.set(p.forbidden.get() + 1);
    ptr::null_mut()
}

struct Parameters {
    f: Fixture,
    controllers: Vec<Box<Control>>,
    parameters: Vec<Box<Parameter>>,
    output: Box<Text>,
    value_text: Box<Text>,
    _controller_table: Box<[*const c_void; 6]>,
    _parameter_table: Box<[*const c_void; 46]>,
    _object_table: Box<[*const c_void; 5]>,
    _list_table: Box<[*const c_void; 3]>,
    _text_table: Box<[*const c_void; 22]>,
}
impl Parameters {
    fn new() -> Self {
        let f = Fixture::new();
        *f.nodes[2].children.borrow_mut() = (3..10).map(|i| f.p(i)).collect();
        for (offset, name) in [
            "Input00", "Output01", "Output02", "Output03", "Output04", "Output05", "Output06",
        ]
        .into_iter()
        .enumerate()
        {
            *f.nodes[offset + 3].text.borrow_mut() = name.encode_utf16().collect();
            f.nodes[offset + 3].family.set(if offset == 0 {
                c"AudioFXEndPointComponent"
            } else {
                c"AudioOutputNode"
            });
        }
        let mut ct = Box::new([ptr::null(); 6]);
        ct[0] = cq as *const c_void;
        ct[1] = cr as *const c_void;
        ct[2] = cd as *const c_void;
        ct[3] = cc as *const c_void;
        ct[4] = ca as *const c_void;
        ct[5] = cf as *const c_void;
        let mut t = Box::new([ptr::null(); 46]);
        t[0] = pq as *const c_void;
        t[1] = pr as *const c_void;
        t[2] = pd as *const c_void;
        t[3] = pk as *const c_void;
        t[4] = pn as *const c_void;
        t[6] = pe as *const c_void;
        t[8] = ps as *const c_void;
        t[13] = pt as *const c_void;
        t[14] = forbidden as *const c_void;
        t[18] = forbidden as *const c_void;
        t[21] = forbidden as *const c_void;
        t[30] = precision as *const c_void;
        t[35] = norm as *const c_void;
        t[36] = forbidden as *const c_void;
        t[41] = formatter as *const c_void;
        t[43] = forbidden as *const c_void;
        t[44] = display as *const c_void;
        t[45] = forbidden as *const c_void;
        let mut o = Box::new([ptr::null(); 5]);
        o[0] = oq as *const c_void;
        o[1] = or as *const c_void;
        o[2] = od as *const c_void;
        o[4] = property as *const c_void;
        let mut list_table = Box::new([ptr::null(); 3]);
        list_table[0] = oq as *const c_void;
        list_table[1] = or as *const c_void;
        list_table[2] = list_release as *const c_void;
        let mut st = Box::new([ptr::null(); 22]);
        st[1] = tr as *const c_void;
        st[2] = td as *const c_void;
        st[4] = tl as *const c_void;
        st[8] = tc as *const c_void;
        st[21] = tp as *const c_void;
        let mut output = Box::new(Text {
            table: st.as_ptr(),
            refs: Cell::new(1),
            text: RefCell::new(vec![]),
        });
        let value_text = Box::new(Text {
            table: st.as_ptr(),
            refs: Cell::new(1),
            text: RefCell::new(vec![]),
        });
        EMPTY.with(|p| p.set(ptr::from_mut(&mut *output).cast()));
        let mut controllers = Vec::new();
        for i in 2..10 {
            let mut c = Box::new(Control {
                table: ct.as_ptr(),
                refs: Cell::new(1),
                identity: Cell::new(f.nodes[i].canonical.get()),
                parameters: RefCell::new(vec![]),
                count_override: Cell::new(None),
                at_calls: Cell::new(0),
                forbidden: Cell::new(0),
            });
            f.nodes[i].controller.set(ptr::from_mut(&mut *c).cast());
            controllers.push(c);
        }
        let mut parameters = Vec::new();
        for i in 0..70 {
            let name = std::ffi::CString::new(format!("observed{i}")).unwrap();
            let mut p = Box::new(Parameter {
                table: t.as_ptr(),
                refs: Cell::new(1),
                object: ObjectView {
                    table: o.as_ptr(),
                    owner: ptr::null_mut(),
                },
                list: ObjectView {
                    table: list_table.as_ptr(),
                    owner: ptr::null_mut(),
                },
                precision: Cell::new(1),
                normalized: Cell::new(0.25),
                state: Cell::new(0),
                list_interface: Cell::new(false),
                list_reply: Cell::new(None),
                precision_calls: Cell::new(0),
                flags_after_precision: Cell::new(0),
                list_query_calls: Cell::new(0),
                list_release_calls: Cell::new(0),
                precision_action: RefCell::new(None),
                list_release_action: RefCell::new(None),
                name: PlainCString {
                    text: name.as_ptr(),
                    owner: ptr::null_mut(),
                },
                _name: name,
                kind: Cell::new(4),
                values: std::array::from_fn(|_| Cell::new((1, 0))),
                display: RefCell::new("None".into()),
                release_action: RefCell::new(None),
                display_action: RefCell::new(None),
                forbidden: Cell::new(0),
                fail_property: Cell::new(false),
                canonical_reply: Cell::new(None),
                release_observer: RefCell::new(None),
            });
            p.object.owner = ptr::from_mut(&mut *p);
            p.list.owner = ptr::from_mut(&mut *p);
            parameters.push(p);
        }
        let result = Self {
            f,
            controllers,
            parameters,
            output,
            value_text,
            _controller_table: ct,
            _parameter_table: t,
            _object_table: o,
            _list_table: list_table,
            _text_table: st,
        };
        result.controllers[0]
            .parameters
            .borrow_mut()
            .push(result.p(0));
        result
    }
    fn p(&self, index: usize) -> *mut c_void {
        ptr::from_ref(&*self.parameters[index]).cast_mut().cast()
    }
    fn capture(&self) -> Result<CvPortParametersInventory, String> {
        CvPortParametersInventory::capture_route(self.f.route()?, text_api(), &mut || Ok(()))
    }
}
impl Drop for Parameters {
    fn drop(&mut self) {
        for c in &self.controllers {
            assert_eq!(c.refs.get(), 1, "controller leaked");
            assert_eq!(c.forbidden.get(), 0, "controller fallback called");
        }
        for p in &self.parameters {
            assert_eq!(p.refs.get(), 1, "parameter interface leaked");
            assert_eq!(p.forbidden.get(), 0, "unadmitted parameter method called");
        }
        assert_eq!(self.output.refs.get(), 1);
        assert_eq!(self.value_text.refs.get(), 1);
        EMPTY.with(|p| p.set(ptr::null_mut()));
    }
}

#[test]
fn direct_indexed_values_keep_scope_and_lifetime_without_edit_authority() {
    let f = Parameters::new();
    let inventory = f.capture().unwrap();
    let v = inventory.value();
    assert_eq!(v["status"], "complete");
    assert_eq!(v["nodes"].as_array().unwrap().len(), 8);
    let row = &v["nodes"][0]["controller"]["parameters"][0];
    assert_eq!(row["name"], "observed0");
    assert_eq!(row["value"]["data"]["kind"], "integer");
    assert!(row.get("writable").is_none());
    assert!(f.parameters[0].refs.get() > 1);
    for key in [
        "assignment_values_observed",
        "routing_verified",
        "atomic_snapshot",
        "mutation_capability",
    ] {
        assert_eq!(v[key], false);
    }
    assert_eq!(v["declared_parameter_controller"], "unobserved");
    assert_eq!(v["original_binding"], "unobserved");
    inventory.check(&mut || Ok(()), &mut || Ok(())).unwrap();
}
#[test]
fn stable_absence_zero_and_lookup_absence_are_distinct() {
    let f = Parameters::new();
    f.f.nodes[2].controller.set(ptr::null_mut());
    let inv = f.capture().unwrap();
    assert_eq!(inv.value()["nodes"][0]["controller"]["status"], "absent");
    assert_eq!(inv.value()["nodes"][1]["controller"]["count"], 0);
    drop(inv);
    f.f.nodes[1].fixed.set(ptr::null_mut());
    let inv = f.capture().unwrap();
    assert_eq!(inv.value()["availability"], "lookup_absent");
    assert_eq!(inv.value()["indexed_inventory_complete"], false);
}
#[test]
fn strict_controller_qi_rejects_success_null_and_failed_poison_without_adoption() {
    for status in [OK, NO_INTERFACE, -7] {
        let f = Parameters::new();
        f.f.nodes[2].controller_status.set(Some(status));
        f.f.nodes[2].controller.set(if status == OK {
            ptr::null_mut()
        } else {
            ptr::dangling_mut()
        });
        assert!(f.capture().is_err());
    }
}
#[test]
fn foreign_controller_and_changed_presence_are_rejected() {
    let f = Parameters::new();
    f.controllers[0].identity.set(f.f.nodes[1].canonical.get());
    assert!(f.capture().is_err());
    let f = Parameters::new();
    let inv = f.capture().unwrap();
    f.f.nodes[3].controller.set(ptr::null_mut());
    assert!(inv.check(&mut || Ok(()), &mut || Ok(())).is_err());
}
#[test]
fn count_limits_read_zero_parameters_and_keep_exact_boundary() {
    for (counts, status) in [
        ([33, 0, 0], "index_limit"),
        ([32, 32, 1], "index_limit"),
        ([32, 32, 0], "complete"),
    ] {
        let f = Parameters::new();
        for (i, count) in counts.into_iter().enumerate() {
            *f.controllers[i].parameters.borrow_mut() =
                (0..count).map(|j| f.p(j as usize)).collect();
        }
        let inv = f.capture().unwrap();
        assert_eq!(inv.value()["status"], status);
        if status == "index_limit" {
            assert!(f.controllers.iter().all(|c| c.at_calls.get() == 0));
            assert!(f.parameters.iter().all(|p| p.precision_calls.get() == 0));
            assert!(f.parameters.iter().all(|p| p.list_query_calls.get() == 0));
        }
    }
}
#[test]
fn negative_null_and_replaced_index_are_errors() {
    let f = Parameters::new();
    f.controllers[0].count_override.set(Some(-1));
    assert!(f.capture().is_err());
    let f = Parameters::new();
    f.controllers[0].parameters.borrow_mut()[0] = ptr::null_mut();
    assert!(f.capture().is_err());
    let f = Parameters::new();
    let inv = f.capture().unwrap();
    f.controllers[0].parameters.borrow_mut()[0] = f.p(1);
    assert!(inv.check(&mut || Ok(()), &mut || Ok(())).is_err());
}
#[test]
fn stable_duplicate_indices_are_preserved_without_distinct_name_claim() {
    let f = Parameters::new();
    f.controllers[0].parameters.borrow_mut().push(f.p(0));
    let inv = f.capture().unwrap();
    let v = inv.value();
    let rows = v["nodes"][0]["controller"]["parameters"]
        .as_array()
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["name"], rows[1]["name"]);
    assert_eq!(rows[1]["index"], 1);
}
#[test]
fn typed_opaque_nonfinite_and_large_integer_values_stay_distinct() {
    let f = Parameters::new();
    for (i, value) in [
        (0, (4, 1)),
        (1, (2, f64::INFINITY.to_bits())),
        (2, (1, i64::MAX as u64)),
        (3, (2, 0x8000000000000000)),
    ] {
        f.parameters[i].values[0].set(value);
    }
    *f.controllers[0].parameters.borrow_mut() = (0..4).map(|i| f.p(i)).collect();
    let inv = f.capture().unwrap();
    let v = inv.value();
    let rows = &v["nodes"][0]["controller"]["parameters"];
    assert_eq!(rows[0]["value"]["data"]["kind"], "opaque_object");
    assert_eq!(rows[1]["value"]["data"]["kind"], "non_finite_float");
    assert_eq!(
        rows[2]["value"]["data"]["exact_decimal"],
        i64::MAX.to_string()
    );
    assert_eq!(rows[3]["value"]["data"]["bits_hex"], "8000000000000000");
}
#[test]
fn retained_text_limit_discards_all_rows_without_truncation() {
    let f = Parameters::new();
    *f.value_text.text.borrow_mut() = "가".repeat(4096).encode_utf16().collect();
    let pointer = ptr::from_ref(&*f.value_text) as usize as u64;
    for v in &f.parameters[0].values {
        v.set((3, pointer));
    }
    *f.parameters[0].display.borrow_mut() = "가".repeat(4096);
    let inv = f.capture().unwrap();
    let value = inv.value();
    assert_eq!(value["status"], "text_limit");
    assert_eq!(value["nodes"][0]["controller"]["parameters"], json!([]));
    assert_eq!(value["retained_parameter_text_bytes"], 0);
    assert!(f.parameters.iter().all(|p| p.precision_calls.get() == 0));
    assert!(f.parameters.iter().all(|p| p.list_query_calls.get() == 0));
}
#[test]
fn description_cleanup_replacement_and_count_change_discard_read() {
    for count in [false, true] {
        let f = Parameters::new();
        let c = ptr::from_ref(&*f.controllers[0]);
        let replacement = f.p(1);
        *f.parameters[0].release_action.borrow_mut() = Some(Box::new(move || unsafe {
            if count {
                (*c).count_override.set(Some(2));
            } else {
                (*c).parameters.borrow_mut()[0] = replacement;
            }
        }));
        assert!(f.capture().is_err());
    }
}
#[test]
fn full_inventory_final_id_copy_replacement_is_detected_without_header_or_uid_drift() {
    let f = Parameters::new();
    let inv = node_inspect::Inventory::CvPortParameters(f.capture().unwrap());
    let before = inv.value();
    let mut baseline = 0;
    inv.check(
        &mut || {
            baseline += 1;
            Ok(())
        },
        &mut || Ok(()),
    )
    .unwrap();
    let c = ptr::from_ref(&*f.controllers[0]);
    let replacement = f.p(1);
    let mut n = 0;
    let result = inv.check(
        &mut || {
            n += 1;
            if n == baseline {
                *f.f.nodes[0].copy_action.borrow_mut() = Some(Box::new(move || unsafe {
                    (*c).parameters.borrow_mut()[0] = replacement;
                }));
            }
            Ok(())
        },
        &mut || Ok(()),
    );
    assert!(matches!(result,Err(reason) if reason.contains("replaced after ID copies")));
    assert_eq!(f.f.nodes[2].family.get(), c"AudioFXPortSetup");
    assert_eq!(inv.value(), before);
    let mut reply = Reply::unbound(json!({"ok":true,"node_report":before}));
    reply.nodes = Some(inv);
    reply.invalidate("late indexed parameter replacement".into());
    assert!(reply.value.get("node_report").is_none());
    assert_eq!(reply.value["ok"], false);
}
#[test]
fn node_types_ids_topology_and_late_context_remain_guarded() {
    for change in 0..3 {
        let f = Parameters::new();
        let inv = f.capture().unwrap();
        match change {
            0 => f.f.nodes[3].family.set(c"Other"),
            1 => *f.f.nodes[3].text.borrow_mut() = "Other".encode_utf16().collect(),
            _ => f.f.nodes[2].children.borrow_mut().swap(0, 1),
        };
        assert!(inv.check(&mut || Ok(()), &mut || Ok(())).is_err());
    }
    let f = Parameters::new();
    let inv = f.capture().unwrap();
    assert!(
        inv.check(&mut || Ok(()), &mut || Err("context lost".into()))
            .is_err()
    );
}

#[test]
fn receipt_final_check_discards_report_when_last_id_copy_replaces_an_index() {
    let _serial = crate::native_script::tests::TEST_LOCK.lock().unwrap();
    let mut context_fixture = crate::native_script::request::guard_tests::Fixture::new();
    let context = context_fixture.context();
    let service = context.service;
    let f = Parameters::new();
    let inv = node_inspect::Inventory::CvPortParameters(f.capture().unwrap());
    let before = f.f.nodes[0].copy_calls.get();
    inv.check(&mut || Ok(()), &mut || Ok(())).unwrap();
    let delta = f.f.nodes[0].copy_calls.get() - before;
    assert!(delta > 1);
    let c = ptr::from_ref(&*f.controllers[0]);
    let replacement = f.p(1);
    f.f.nodes[0]
        .copy_action_at
        .set(Some(f.f.nodes[0].copy_calls.get() + delta));
    *f.f.nodes[0].copy_action.borrow_mut() = Some(Box::new(move || unsafe {
        (*c).parameters.borrow_mut()[0] = replacement;
    }));
    let mut reply = Reply::unbound(json!({"ok":true,"node_report":inv.value()}));
    reply.nodes = Some(inv);
    reply.context = Some(context);
    assert!(!reply.final_check(service));
    assert!(reply.value.get("node_report").is_none());
    assert_eq!(
        reply.value["error"]["code"],
        "context_changed_before_receipt"
    );
    assert!(
        reply.value["error"]["message"]
            .as_str()
            .unwrap()
            .contains("replaced after ID copies")
    );
}
#[test]
fn receipt_context_loss_discards_parameter_report_and_releases_pins() {
    let _serial = crate::native_script::tests::TEST_LOCK.lock().unwrap();
    let mut context_fixture = crate::native_script::request::guard_tests::Fixture::new();
    let context = context_fixture.context();
    let service = context.service;
    let f = Parameters::new();
    let inv = node_inspect::Inventory::CvPortParameters(f.capture().unwrap());
    let mut reply = Reply::unbound(json!({"ok":true,"node_report":inv.value()}));
    reply.nodes = Some(inv);
    reply.context = Some(context);
    context_fixture.switch_document();
    assert!(!reply.final_check(service));
    assert!(reply.value.get("node_report").is_none());
    drop(reply);
    assert_eq!(f.parameters[0].refs.get(), 1);
}
#[test]
fn long_name_and_invalid_native_text_fail_the_whole_read() {
    let mut f = Parameters::new();
    f.parameters[0]._name = std::ffi::CString::new("x".repeat(513)).unwrap();
    f.parameters[0].name.text = f.parameters[0]._name.as_ptr();
    assert!(f.capture().is_err());
    let f = Parameters::new();
    f.parameters[0].fail_property.set(true);
    assert!(f.capture().is_err());
}
#[test]
fn float_normalized_metadata_and_encoded_response_refusal_stay_separate() {
    let f = Parameters::new();
    f.parameters[0].kind.set(2);
    let inv = f.capture().unwrap();
    let value = inv.value();
    assert_eq!(
        value["nodes"][0]["controller"]["parameters"][0]["normalized"]["bits_hex"],
        "3E800000"
    );
    let mut reply =
        Reply::unbound(json!({"ok":true,"node_report":value,"other":"\u{0001}".repeat(48*1024)}));
    assert!(reply.value.to_string().len() > MAX_RESPONSE_BYTES);
    let encoded: Value = serde_json::from_str(&reply.encode()).unwrap();
    assert_eq!(encoded["error"]["code"], "response_too_large");
    assert!(encoded.get("node_report").is_none());
}

#[test]
fn retained_text_budget_accepts_exact_limit_and_refuses_one_more_byte() {
    let f = Parameters::new();
    *f.value_text.text.borrow_mut() = "x".repeat(1000).encode_utf16().collect();
    let pointer = ptr::from_ref(&*f.value_text) as usize as u64;
    *f.controllers[0].parameters.borrow_mut() = (0..8).map(|i| f.p(i)).collect();
    for p in &f.parameters[..8] {
        for v in &p.values {
            v.set((3, pointer));
        }
        *p.display.borrow_mut() = "y".repeat(2000);
    }
    let first = f.capture().unwrap();
    let n = first.value()["retained_parameter_text_bytes"]
        .as_u64()
        .unwrap() as usize;
    drop(first);
    let delta = 48 * 1024 - n;
    assert!(delta + 2000 <= 4096);
    *f.parameters[0].display.borrow_mut() = "y".repeat(2000 + delta);
    let exact = f.capture().unwrap();
    assert_eq!(exact.value()["retained_parameter_text_bytes"], 48 * 1024);
    assert_eq!(exact.value()["status"], "complete");
    drop(exact);
    f.parameters[0].display.borrow_mut().push('y');
    let over = f.capture().unwrap();
    assert_eq!(over.value()["status"], "text_limit");
    assert!(
        over.value()["nodes"][0]["controller"]["parameters"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn parameter_canonical_query_failure_never_adopts_a_poison_output() {
    for (status, pointer) in [
        (OK, ptr::null_mut()),
        (NO_INTERFACE, ptr::dangling_mut()),
        (-3, ptr::dangling_mut()),
    ] {
        let f = Parameters::new();
        f.parameters[0].canonical_reply.set(Some((status, pointer)));
        assert!(f.capture().is_err());
    }
}
#[test]
fn absent_to_present_and_rotating_controller_views_are_not_empty_routes() {
    let f = Parameters::new();
    let original = f.f.nodes[2].controller.replace(ptr::null_mut());
    let inv = f.capture().unwrap();
    f.f.nodes[2].controller.set(original);
    assert!(inv.check(&mut || Ok(()), &mut || Ok(())).is_err());
    drop(inv);
    let inv = f.capture().unwrap();
    f.controllers[1].identity.set(f.f.nodes[2].canonical.get());
    let other = ptr::from_ref(&*f.controllers[1]).cast_mut().cast();
    f.f.nodes[2].controller.set(other);
    assert!(inv.check(&mut || Ok(()), &mut || Ok(())).is_err());
}
#[test]
fn child_id_string_callback_is_before_every_selected_header_check() {
    let f = Parameters::new();
    let inv = f.capture().unwrap();
    let mut baseline = 0;
    inv.check(
        &mut || {
            baseline += 1;
            Ok(())
        },
        &mut || Ok(()),
    )
    .unwrap();
    let mut n = 0;
    let result = inv.check(
        &mut || {
            n += 1;
            if n == baseline {
                f.f.nodes[3]
                    .copy_family
                    .set(ptr::from_ref(&*f.f.nodes[4]).cast_mut());
            }
            Ok(())
        },
        &mut || Ok(()),
    );
    assert!(matches!(result,Err(reason) if reason.contains("first native type changed")));
}

#[test]
fn actual_parameter_interfaces_drop_before_their_retained_controller_view() {
    let f = Parameters::new();
    let inv = f.capture().unwrap();
    let controller = ptr::from_ref(&*f.controllers[0]);
    *f.parameters[0].release_observer.borrow_mut() = Some(Box::new(move || {
        assert!(
            unsafe { (*controller).refs.get() } > 1,
            "controller view released before indexed parameter interfaces"
        )
    }));
    drop(inv);
    assert_eq!(f.controllers[0].refs.get(), 1);
}

#[test]
fn all_seven_rows_keep_distinct_fixed_details_and_negative_zero() {
    let f = Parameters::new();
    f.controllers[0].parameters.borrow_mut().clear();
    let states = [0, 0x80, 0x100, 0x800, 0x1000, 0x10000, 0x11980];
    for (i, state) in states.into_iter().enumerate() {
        f.controllers[i + 1].parameters.borrow_mut().push(f.p(i));
        f.parameters[i].precision.set(i as i32 - 3);
        f.parameters[i].state.set(state);
        f.parameters[i].list_interface.set(i % 2 == 0);
    }
    f.parameters[6].kind.set(2);
    f.parameters[6].normalized.set(-0.0);
    f.parameters[6].values[0].set((2, (-0.0_f64).to_bits()));
    let inventory = f.capture().unwrap();
    let value = inventory.value();
    for (i, state) in states.into_iter().enumerate() {
        let row = &value["nodes"][i + 1]["controller"]["parameters"][0];
        assert_eq!(row["name"], format!("observed{i}"));
        assert_eq!(
            row["details"],
            json!({"precision":i as i32-3,"storable":state&0x80!=0,
                "store_list_value":state&0x100!=0,"mutable":state&0x800!=0,
                "read_only":state&0x1000!=0,"can_undo":state&0x10000!=0,
                "list_interface":i%2==0})
        );
        assert_eq!(f.parameters[i].precision_calls.get(), 1);
        assert_eq!(f.parameters[i].flags_after_precision.get(), 5);
        assert_eq!(f.parameters[i].list_query_calls.get(), 1);
        assert_eq!(f.parameters[i].list_release_calls.get(), u32::from(i % 2 == 0));
    }
    let last = &value["nodes"][7]["controller"]["parameters"][0];
    assert_eq!(last["normalized"]["bits_hex"], "80000000");
    assert_eq!(last["value"]["data"]["bits_hex"], "8000000000000000");
    drop(inventory);
    assert!(f.parameters.iter().all(|p| p.refs.get() == 1));
}

#[test]
fn current_precision_replacement_stops_before_flags() {
    let f = Parameters::new();
    let c = ptr::from_ref(&*f.controllers[0]);
    let replacement = f.p(1);
    *f.parameters[0].precision_action.borrow_mut() = Some(Box::new(move || unsafe {
        (*c).parameters.borrow_mut()[0] = replacement;
    }));
    assert!(f.capture().is_err());
    assert_eq!(f.parameters[0].precision_calls.get(), 1);
    assert_eq!(f.parameters[0].flags_after_precision.get(), 0);
    assert_eq!(f.parameters[0].list_query_calls.get(), 0);
    assert_eq!(f.parameters[0].refs.get(), 1);
}

#[test]
fn successful_list_qi_release_replacement_rejects_and_balances_references() {
    let f = Parameters::new();
    let c = ptr::from_ref(&*f.controllers[0]);
    let replacement = f.p(1);
    f.parameters[0].list_interface.set(true);
    *f.parameters[0].list_release_action.borrow_mut() = Some(Box::new(move || unsafe {
        (*c).parameters.borrow_mut()[0] = replacement;
    }));
    assert!(f.capture().is_err());
    assert_eq!(f.parameters[0].precision_calls.get(), 1);
    assert_eq!(f.parameters[0].flags_after_precision.get(), 5);
    assert_eq!(f.parameters[0].list_query_calls.get(), 1);
    assert_eq!(f.parameters[0].list_release_calls.get(), 1);
    assert!(f.parameters.iter().all(|p| p.refs.get() == 1));
}

#[test]
fn indeterminate_list_qi_rejects_without_adopting_poison_outputs() {
    for (status, pointer) in [
        (OK, ptr::null_mut()),
        (NO_INTERFACE, ptr::dangling_mut()),
        (-7, ptr::dangling_mut()),
        (-7, ptr::null_mut()),
    ] {
        let f = Parameters::new();
        f.parameters[0].list_reply.set(Some((status, pointer)));
        assert!(f.capture().is_err());
        assert_eq!(f.parameters[0].list_query_calls.get(), 1);
        assert_eq!(f.parameters[0].list_release_calls.get(), 0);
        assert!(f.parameters.iter().all(|p| p.refs.get() == 1));
    }
}

#[test]
fn late_text_overflow_skips_details_for_earlier_acceptable_rows_too() {
    let f = Parameters::new();
    f.controllers[0].parameters.borrow_mut().push(f.p(1));
    *f.value_text.text.borrow_mut() = "가".repeat(4096).encode_utf16().collect();
    let pointer = ptr::from_ref(&*f.value_text) as usize as u64;
    for value in &f.parameters[1].values {
        value.set((3, pointer));
    }
    *f.parameters[1].display.borrow_mut() = "가".repeat(4096);
    let inventory = f.capture().unwrap();
    assert_eq!(inventory.value()["status"], "text_limit");
    assert_eq!(inventory.value()["nodes"][0]["controller"]["parameters"], json!([]));
    assert!(f.parameters.iter().all(|p| p.precision_calls.get() == 0));
    assert!(f.parameters.iter().all(|p| p.list_query_calls.get() == 0));
}

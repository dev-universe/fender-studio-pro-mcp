use super::*;
use std::cell::{Cell, RefCell};
use std::ffi::CStr;

const TARGET: usize = 0;
const ALIAS_VIEW: usize = 1;
const PARENT: usize = 2;
const PLUGIN: usize = 3;
const ROOT: usize = 4;
const SELECTOR: usize = 8;
const EFFECTS: usize = 9;
const CATEGORIES: usize = 10;
const OTHER: usize = 11;

#[repr(C)]
struct Fake {
    table: *const *const c_void,
    refs: Cell<u32>,
    canonical: Cell<*mut c_void>,
    node: Cell<*mut c_void>,
    controller: Cell<*mut c_void>,
    model: Cell<*mut c_void>,
    parent: Cell<*mut c_void>,
    member: Cell<*mut c_void>,
    child: Cell<*mut c_void>,
    categories: Cell<*mut c_void>,
    expected_name: &'static CStr,
    count: Cell<i32>,
    rows: RefCell<Vec<(*mut c_void, bool)>>,
    title_calls: Cell<u32>,
    find_calls: Cell<u32>,
    object_calls: RefCell<Vec<String>>,
    failed_out: Cell<Option<Uid>>,
}
#[repr(C)]
struct Text {
    table: *const *const c_void,
    refs: Cell<u32>,
    units: RefCell<Vec<u16>>,
    terminated: Cell<bool>,
    released: Cell<u32>,
}
thread_local! {
    static CONSTANTS: RefCell<Vec<(&'static CStr,*const PlainString)>> = const { RefCell::new(Vec::new()) };
    static CSTRINGS: Cell<[*const PlainCString;2]> = const { Cell::new([ptr::null();2]) };
}
unsafe fn fake<'a>(p: *mut c_void) -> &'a Fake {
    unsafe { &*p.cast::<Fake>() }
}
unsafe fn text<'a>(p: *mut c_void) -> &'a Text {
    unsafe { &*p.cast::<Text>() }
}
unsafe extern "C" fn retain(p: *mut c_void) -> u32 {
    let f = unsafe { fake(p) };
    f.refs.set(f.refs.get() + 1);
    f.refs.get()
}
unsafe extern "C" fn release_fake(p: *mut c_void) -> u32 {
    let f = unsafe { fake(p) };
    f.refs
        .set(f.refs.get().checked_sub(1).expect("double release"));
    f.refs.get()
}
unsafe extern "C" fn query_fake(p: *mut c_void, iid: *const Uid, out: *mut *mut c_void) -> i32 {
    let f = unsafe { fake(p) };
    let iid = unsafe { *iid };
    if f.failed_out.get() == Some(iid) {
        unsafe {
            retain(p);
            *out = p
        };
        return NO_INTERFACE;
    }
    let found = if iid == UNKNOWN {
        f.canonical.get()
    } else if iid == NODE {
        f.node.get()
    } else if iid == CONTROLLER {
        f.controller.get()
    } else if iid == ITEM_MODEL {
        f.model.get()
    } else if iid == ALIAS {
        f.member.get()
    } else if iid == CONTROLLER_PROVIDER {
        p
    } else {
        ptr::null_mut()
    };
    if found.is_null() {
        return NO_INTERFACE;
    }
    unsafe {
        retain(found);
        *out = found
    };
    OK
}
unsafe extern "C" fn member(p: *mut c_void) -> *mut c_void {
    unsafe { fake(p) }.member.get()
}
unsafe extern "C" fn parent(p: *mut c_void) -> *mut c_void {
    unsafe { fake(p) }.parent.get()
}
unsafe extern "C" fn find_fake(p: *mut c_void, name: *const PlainString) -> *mut c_void {
    let f = unsafe { fake(p) };
    let name = unsafe { text((*name).object) };
    let value = String::from_utf16(&name.units.borrow()).unwrap();
    assert_eq!(
        value,
        f.expected_name.to_str().unwrap(),
        "caller-selected path escaped fixed route"
    );
    f.find_calls.set(f.find_calls.get() + 1);
    f.child.get()
}
unsafe extern "C" fn get_object(
    p: *mut c_void,
    name: *const PlainCString,
    iid: *const Uid,
) -> *mut c_void {
    assert!(unsafe { *iid } == ITEM_MODEL);
    let f = unsafe { fake(p) };
    let name = unsafe { CStr::from_ptr((*name).text) };
    f.object_calls
        .borrow_mut()
        .push(name.to_str().unwrap().into());
    match name.to_bytes() {
        b"effectListModel" => f.member.get(),
        b"categoryModel" => f.categories.get(),
        _ => panic!("unregistered model name"),
    }
}
unsafe extern "C" fn count(p: *mut c_void) -> i32 {
    unsafe { fake(p) }.count.get()
}
unsafe extern "C" fn get_title(p: *mut c_void, out: *mut PlainString, row: *const ItemIndex) -> u8 {
    let f = unsafe { fake(p) };
    let row = unsafe { &*row };
    assert_eq!(row.tag, 0);
    assert!((0..i64::from(f.count.get())).contains(&row.index));
    assert!(
        !unsafe { (*out).object }.is_null(),
        "OUT must be a constructed String"
    );
    f.title_calls.set(f.title_calls.get() + 1);
    let rows = f.rows.borrow();
    let (value, available) = rows[row.index as usize % rows.len()];
    // A normal String assignment shares the incoming IString and releases the
    // OUT's previous one. The caller must release only its final current value.
    unsafe {
        text_retain(value);
        (unknown_vtable((*out).object).release)((*out).object);
        (*out).object = value;
        (*out).opaque_debug = ptr::dangling_mut::<c_void>();
    }
    u8::from(available)
}
unsafe extern "C" fn text_retain(p: *mut c_void) -> u32 {
    let f = unsafe { text(p) };
    f.refs.set(f.refs.get() + 1);
    f.refs.get()
}
unsafe extern "C" fn text_release(p: *mut c_void) -> u32 {
    let f = unsafe { text(p) };
    f.refs
        .set(f.refs.get().checked_sub(1).expect("String double release"));
    f.released.set(f.released.get() + 1);
    f.refs.get()
}
unsafe extern "C" fn length(p: *mut c_void) -> i32 {
    unsafe { text(p) }.units.borrow().len() as i32
}
unsafe extern "C" fn copy(p: *mut c_void, out: *mut u16, n: i32) -> i32 {
    let f = unsafe { text(p) };
    let units = f.units.borrow();
    assert_eq!(n as usize, units.len() + 1);
    unsafe {
        ptr::copy_nonoverlapping(units.as_ptr(), out, units.len());
        if f.terminated.get() {
            *out.add(units.len()) = 0;
        }
    }
    OK
}
unsafe extern "C" fn string_constant(name: *const c_char) -> *const PlainString {
    let name = unsafe { CStr::from_ptr(name) };
    CONSTANTS.with(|values| {
        values
            .borrow()
            .iter()
            .find(|(n, _)| *n == name)
            .map_or(ptr::null(), |(_, v)| *v)
    })
}
unsafe extern "C" fn cstring_constant(name: *const c_char) -> *const PlainCString {
    let name = unsafe { CStr::from_ptr(name) };
    CSTRINGS.with(|values| match name.to_bytes() {
        b"categoryModel" => values.get()[0],
        b"effectListModel" => values.get()[1],
        _ => ptr::null(),
    })
}

struct Fixture {
    objects: Box<[Fake; 12]>,
    texts: Box<[Text; 8]>,
    _constants: Box<[PlainString; 5]>,
    _cstrings: Box<[PlainCString; 2]>,
    _table: Box<[*const c_void; 13]>,
    _text_table: Box<[*const c_void; 9]>,
}
impl Fixture {
    fn new() -> Self {
        let mut table = Box::new([ptr::null(); 13]);
        table[0] = query_fake as *const c_void;
        table[1] = retain as *const c_void;
        table[2] = release_fake as *const c_void;
        table[3] = member as *const c_void;
        table[5] = count as *const c_void;
        table[6] = parent as *const c_void;
        table[7] = get_object as *const c_void;
        table[10] = find_fake as *const c_void;
        table[12] = get_title as *const c_void;
        let mut text_table = Box::new([ptr::null(); 9]);
        text_table[1] = text_retain as *const c_void;
        text_table[2] = text_release as *const c_void;
        text_table[4] = length as *const c_void;
        text_table[8] = copy as *const c_void;
        let names = [
            c"workspaceroot",
            c"plugInUnknown",
            c"signalchain",
            c"effectSelector",
            c"",
        ];
        let texts = Box::new(std::array::from_fn(|i| Text {
            table: text_table.as_ptr(),
            refs: Cell::new(1),
            units: RefCell::new(
                if i < 5 {
                    names[i].to_str().unwrap()
                } else {
                    ["Clean", "Delay", "No title"][i - 5]
                }
                .encode_utf16()
                .collect(),
            ),
            terminated: Cell::new(true),
            released: Cell::new(0),
        }));
        let constants = Box::new(std::array::from_fn(|i| PlainString {
            object: ptr::from_ref(&texts[i]).cast_mut().cast(),
            opaque_debug: ptr::dangling_mut::<c_void>(),
        }));
        let cstrings = Box::new([
            PlainCString {
                text: c"categoryModel".as_ptr(),
                owner: ptr::null_mut(),
            },
            PlainCString {
                text: c"effectListModel".as_ptr(),
                owner: ptr::null_mut(),
            },
        ]);
        let objects = Box::new(std::array::from_fn(|i| Fake {
            table: table.as_ptr(),
            refs: Cell::new(1),
            canonical: Cell::new(ptr::null_mut()),
            node: Cell::new(ptr::null_mut()),
            controller: Cell::new(ptr::null_mut()),
            model: Cell::new(ptr::null_mut()),
            parent: Cell::new(ptr::null_mut()),
            member: Cell::new(ptr::null_mut()),
            child: Cell::new(ptr::null_mut()),
            categories: Cell::new(ptr::null_mut()),
            expected_name: if (ROOT..SELECTOR).contains(&i) {
                names[i - ROOT]
            } else {
                c""
            },
            count: Cell::new(3),
            rows: RefCell::new(Vec::new()),
            title_calls: Cell::new(0),
            find_calls: Cell::new(0),
            object_calls: RefCell::new(Vec::new()),
            failed_out: Cell::new(None),
        }));
        let fixture = Self {
            objects,
            texts,
            _constants: constants,
            _cstrings: cstrings,
            _table: table,
            _text_table: text_table,
        };
        for i in 0..12 {
            fixture.objects[i].canonical.set(fixture.pointer(i));
            fixture.objects[i].node.set(fixture.pointer(i));
        }
        fixture.objects[TARGET]
            .member
            .set(fixture.pointer(ALIAS_VIEW));
        fixture.objects[TARGET].parent.set(fixture.pointer(PARENT));
        fixture.objects[ALIAS_VIEW]
            .member
            .set(fixture.pointer(PLUGIN));
        fixture.objects[PLUGIN].member.set(fixture.pointer(ROOT));
        fixture.objects[ROOT].controller.set(fixture.pointer(ROOT));
        for i in ROOT..SELECTOR {
            fixture.objects[i].child.set(fixture.pointer(i + 1));
        }
        fixture.objects[SELECTOR]
            .controller
            .set(fixture.pointer(SELECTOR));
        fixture.objects[SELECTOR]
            .member
            .set(fixture.pointer(EFFECTS));
        fixture.objects[SELECTOR]
            .categories
            .set(fixture.pointer(CATEGORIES));
        for index in [EFFECTS, CATEGORIES, OTHER] {
            fixture.objects[index].model.set(fixture.pointer(index));
            *fixture.objects[index].rows.borrow_mut() =
                (5..8).map(|i| (fixture.text_pointer(i), i != 7)).collect();
        }
        CONSTANTS.with(|values| {
            *values.borrow_mut() = names
                .into_iter()
                .enumerate()
                .map(|(i, n)| (n, ptr::from_ref(&fixture._constants[i])))
                .collect()
        });
        CSTRINGS.with(|values| {
            values.set([
                ptr::from_ref(&fixture._cstrings[0]),
                ptr::from_ref(&fixture._cstrings[1]),
            ])
        });
        fixture
    }
    fn pointer(&self, i: usize) -> *mut c_void {
        ptr::from_ref(&self.objects[i]).cast_mut().cast()
    }
    fn text_pointer(&self, i: usize) -> *mut c_void {
        ptr::from_ref(&self.texts[i]).cast_mut().cast()
    }
    fn catalog<G: FnMut() -> Result<(), String>>(
        &self,
        kind: Kind,
        offset: u32,
        limit: u32,
        guard: &mut G,
    ) -> Result<Catalog, String> {
        let result = AliasBinding::capture(self.pointer(TARGET), self.pointer(TARGET), guard)
            .and_then(|alias| ControllerBinding::capture(alias, guard))
            .and_then(|binding| {
                Catalog::capture(
                    binding,
                    string_constant,
                    cstring_constant,
                    kind,
                    offset,
                    limit,
                    guard,
                )
            });
        if let Err(reason) = guard() {
            release(result, guard)?;
            return Err(reason);
        }
        result
    }
    fn released(&self) {
        assert!(
            self.objects.iter().all(|o| o.refs.get() == 1),
            "native object refs leaked"
        );
        assert!(
            self.texts.iter().all(|s| s.refs.get() == 1),
            "String refs leaked"
        );
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.released();
        CONSTANTS.with(|v| v.borrow_mut().clear());
        CSTRINGS.with(|v| v.set([ptr::null(); 2]));
    }
}

#[test]
fn request_is_strict_finite_and_bound_to_an_explicit_instance() {
    let base = json!({"operation":"model_catalog","catalog":"categories","expected_instance":"12345678-1234-1234-1234-123456789ABC"});
    let request: Request = serde_json::from_value(base.clone()).unwrap();
    request.validate().unwrap();
    assert!(!request.needs_auxiliary());
    for key in ["expected_instance", "catalog"] {
        let mut bad = base.clone();
        bad.as_object_mut().unwrap().remove(key);
        assert!(serde_json::from_value::<Request>(bad).is_err());
    }
    for extra in [
        json!({"path":"signalchain"}),
        json!({"name":"effectListModel"}),
        json!({"iid":"x"}),
        json!({"source":"controller"}),
        json!({"token":"x"}),
        json!({"catalog":"cabinet"}),
        json!({"expected_instance":""}),
    ] {
        let mut bad = base.clone();
        bad.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        assert!(serde_json::from_value::<Request>(bad).map_or(true, |r| r.validate().is_err()));
    }
    for (offset, limit, valid) in [
        (0, 1, true),
        (4096, 32, true),
        (4097, 1, false),
        (0, 0, false),
        (0, 33, false),
    ] {
        assert_eq!(validate_page(offset, limit).is_ok(), valid);
    }
}

#[test]
fn fixed_catalogs_preserve_false_titles_and_bound_current_model_coverage() {
    for kind in [Kind::Categories, Kind::Effects] {
        let f = Fixture::new();
        let catalog = f.catalog(kind, 0, 32, &mut || Ok(())).unwrap();
        let value = catalog.value();
        assert_eq!(value["count"], 3);
        assert_eq!(value["rows"][0]["title"], "Clean");
        assert_eq!(value["rows"][2]["title"], "No title");
        assert_eq!(value["rows"][2]["title_available"], false);
        assert_eq!(value["complete"], false);
        assert_eq!(value["all_flat_rows_observed"], false);
        assert_eq!(value["atomic_snapshot"], false);
        assert_eq!(value["mutation_capability"], false);
        assert_eq!(value["scope"], "current_selector_flat_model");
        assert!(value.get("token").is_none());
        assert!(
            f.objects[SELECTOR]
                .object_calls
                .borrow()
                .iter()
                .all(|n| n == kind.name().to_str().unwrap())
        );
        catalog.check(&mut || Ok(())).unwrap();
        drop(catalog);
        f.released();
    }
}

#[test]
fn empty_model_is_distinct_from_an_absent_named_object_and_pages_are_precise() {
    let f = Fixture::new();
    f.objects[EFFECTS].count.set(0);
    let catalog = f.catalog(Kind::Effects, 0, 32, &mut || Ok(())).unwrap();
    let value = catalog.value();
    assert_eq!(value["rows"], json!([]));
    assert_eq!(value["complete"], true);
    assert_eq!(value["all_flat_rows_observed"], true);
    drop(catalog);
    f.objects[SELECTOR].member.set(ptr::null_mut());
    assert!(
        matches!(f.catalog(Kind::Effects,0,32,&mut ||Ok(())),Err(reason) if reason=="named model object absent")
    );
    f.objects[SELECTOR].member.set(f.pointer(EFFECTS));
    f.objects[EFFECTS].count.set(3);
    let catalog = f.catalog(Kind::Effects, 1, 1, &mut || Ok(())).unwrap();
    let value = catalog.value();
    assert_eq!(
        value["rows"],
        json!([{"index":1,"title":"Delay","title_available":true}])
    );
    assert_eq!(value["next_offset"], 2);
    assert_eq!(value["complete"], true);
    assert_eq!(value["all_flat_rows_observed"], false);
    drop(catalog);
    assert!(f.catalog(Kind::Effects, 4, 1, &mut || Ok(())).is_err());
    f.texts[5].units.borrow_mut().clear();
    let catalog = f.catalog(Kind::Effects, 0, 1, &mut || Ok(())).unwrap();
    assert_eq!(
        catalog.value()["rows"][0],
        json!({"index":0,"title":"","title_available":true})
    );
    assert_eq!(catalog.value()["complete"], true);
    drop(catalog);
}

#[test]
fn hard_count_row_and_text_boundaries_are_enforced() {
    let f = Fixture::new();
    for count in [-1, 4097] {
        f.objects[EFFECTS].count.set(count);
        assert!(f.catalog(Kind::Effects, 0, 1, &mut || Ok(())).is_err());
    }
    assert_eq!(f.objects[EFFECTS].title_calls.get(), 0);
    f.objects[EFFECTS].count.set(4096);
    let catalog = f.catalog(Kind::Effects, 4095, 32, &mut || Ok(())).unwrap();
    assert_eq!(catalog.value()["rows"][0]["index"], 4095);
    assert!(catalog.value()["next_offset"].is_null());
    drop(catalog);
    f.objects[EFFECTS].count.set(32);
    for t in f.texts.iter().skip(5) {
        *t.units.borrow_mut() = vec![65; 512];
    }
    let catalog = f.catalog(Kind::Effects, 0, 32, &mut || Ok(())).unwrap();
    assert_eq!(
        catalog.rows.iter().map(|r| r.title.len()).sum::<usize>(),
        MAX_TEXT
    );
    drop(catalog);
    *f.texts[5].units.borrow_mut() = vec![65; 513];
    assert!(f.catalog(Kind::Effects, 0, 1, &mut || Ok(())).is_err());
    *f.texts[5].units.borrow_mut() = vec![0xd800];
    assert!(f.catalog(Kind::Effects, 0, 1, &mut || Ok(())).is_err());
    *f.texts[5].units.borrow_mut() = vec![65];
    f.texts[5].terminated.set(false);
    assert!(f.catalog(Kind::Effects, 0, 1, &mut || Ok(())).is_err());
}

#[test]
fn canonical_and_named_replacements_at_every_edge_are_rejected() {
    for change in 0..11 {
        let f = Fixture::new();
        let catalog = f.catalog(Kind::Effects, 0, 2, &mut || Ok(())).unwrap();
        match change {
            0..=3 => f.objects[ROOT + change].child.set(f.pointer(OTHER)),
            4 => f.objects[SELECTOR].member.set(f.pointer(OTHER)),
            5 => f.objects[SELECTOR].controller.set(f.pointer(OTHER)),
            6 => f.objects[EFFECTS].model.set(f.pointer(OTHER)),
            7 => f.objects[EFFECTS].canonical.set(f.pointer(OTHER)),
            8 => f.objects[6].canonical.set(f.pointer(OTHER)),
            9 => f.objects[PLUGIN].member.set(f.pointer(OTHER)),
            10 => f.objects[TARGET].parent.set(f.pointer(OTHER)),
            _ => unreachable!(),
        }
        assert!(
            catalog.check(&mut || Ok(())).is_err(),
            "replacement{change} passed"
        );
        drop(catalog);
    }
}

#[test]
fn final_context_guard_precedes_every_final_catalog_attachment_observation() {
    for changed in 0..5 {
        let f = Fixture::new();
        let catalog = f.catalog(Kind::Effects, 0, 1, &mut || Ok(())).unwrap();
        let mut total = 0;
        catalog
            .check(&mut || {
                total += 1;
                Ok(())
            })
            .unwrap();
        let mut calls = 0;
        let result = catalog.check(&mut || {
            calls += 1;
            if calls == total {
                if changed < 4 {
                    f.objects[ROOT + changed].child.set(f.pointer(OTHER));
                } else {
                    f.objects[SELECTOR].member.set(f.pointer(OTHER));
                }
            }
            Ok(())
        });
        assert!(
            result.is_err(),
            "late attachment replacement{changed} passed"
        );
        drop(catalog);
    }
}

#[test]
fn same_model_count_reordering_title_status_and_count_changes_are_rejected() {
    for change in 0..4 {
        let f = Fixture::new();
        let catalog = f.catalog(Kind::Effects, 0, 2, &mut || Ok(())).unwrap();
        match change {
            0 => f.objects[EFFECTS].rows.borrow_mut().swap(0, 1),
            1 => *f.texts[5].units.borrow_mut() = vec![88],
            2 => f.objects[EFFECTS].rows.borrow_mut()[0].1 = false,
            3 => f.objects[EFFECTS].count.set(4),
            _ => unreachable!(),
        }
        assert!(catalog.check(&mut || Ok(())).is_err());
        drop(catalog);
    }
}

#[test]
fn every_borrowed_child_and_model_is_retained_before_guard_reentry() {
    let f = Fixture::new();
    let catalog = f
        .catalog(Kind::Effects, 0, 1, &mut || {
            for i in ROOT..SELECTOR {
                if f.objects[i].find_calls.get() > 0 {
                    assert!(f.objects[i + 1].refs.get() > 1);
                }
            }
            if !f.objects[SELECTOR].object_calls.borrow().is_empty() {
                assert!(f.objects[EFFECTS].refs.get() > 1);
            }
            Ok(())
        })
        .unwrap();
    drop(catalog);
}

#[test]
fn wrong_model_interface_and_failed_owned_out_are_released() {
    for failed_out in [false, true] {
        let f = Fixture::new();
        if failed_out {
            f.objects[EFFECTS].failed_out.set(Some(ITEM_MODEL));
        } else {
            f.objects[EFFECTS].model.set(ptr::null_mut());
        }
        assert!(f.catalog(Kind::Effects, 0, 1, &mut || Ok(())).is_err());
        f.released();
    }
    let f = Fixture::new();
    f.objects[EFFECTS].model.set(f.pointer(OTHER));
    assert!(f.catalog(Kind::Effects, 0, 1, &mut || Ok(())).is_err());
}

#[test]
fn guards_after_string_release_and_at_capture_failure_release_all_owners() {
    let f = Fixture::new();
    let result = f.catalog(Kind::Effects, 0, 1, &mut || {
        if f.texts[5].released.get() > 0 {
            Err("String release invalidated document".into())
        } else {
            Ok(())
        }
    });
    assert!(result.is_err());
    f.released();
    let mut total = 0;
    drop(
        f.catalog(Kind::Effects, 0, 1, &mut || {
            total += 1;
            Ok(())
        })
        .unwrap(),
    );
    for stop in [1, 3, 7, 17, 41, total / 2, total - 1, total] {
        let f = Fixture::new();
        let mut calls = 0;
        assert!(
            f.catalog(Kind::Effects, 0, 1, &mut || {
                calls += 1;
                if calls >= stop {
                    Err("context lost".into())
                } else {
                    Ok(())
                }
            })
            .is_err()
        );
        f.released();
    }
}

fn reply(context: Box<Context<'static>>, catalog: Catalog) -> Reply {
    let mut value = catalog.value();
    value["instance"] = json!(context.instance);
    Reply {
        value,
        catalog: Some(catalog),
        action: None,
        presets: None,
        owner: None,
        auxiliary: None,
        folder_preflight: None,
        nodes: None,
        provider: None,
        context: Some(context),
        mutation: false,
    }
}

#[test]
fn retained_catalog_survives_publication_and_receipt_release() {
    let _serial = super::super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut host = guard_tests::Fixture::new();
    let context = host.context();
    let service = context.service;
    let f = Fixture::new();
    let catalog = f.catalog(Kind::Effects, 0, 2, &mut || Ok(())).unwrap();
    let mut output = VariantOut {
        kind: 0,
        padding: [0; 6],
        payload: 0,
    };
    assert!(publish_reply(
        &mut output,
        reply(context, catalog),
        api().unwrap(),
        service
    ));
    let value: Value =
        serde_json::from_str(&super::super::super::super::tests::output_text(&output)).unwrap();
    assert_eq!(value["ok"], true);
    assert_eq!(value["count"], 3);
    assert!(f.objects[EFFECTS].refs.get() > 1);
    release_receipt(service);
    f.released();
    super::super::super::super::tests::clear_output(&mut output);
}

#[test]
fn publication_change_removes_catalog_rows_and_never_returns_stale_success() {
    let _serial = super::super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut host = guard_tests::Fixture::new();
    let context = host.context();
    let service = context.service;
    let f = Fixture::new();
    let catalog = f.catalog(Kind::Effects, 0, 2, &mut || Ok(())).unwrap();
    let mut reply = reply(context, catalog);
    f.objects[EFFECTS].rows.borrow_mut().swap(0, 1);
    assert!(!reply.final_check(service));
    assert_eq!(reply.value["ok"], false);
    assert_eq!(reply.value["outcome"], "not_applied");
    assert!(reply.value.get("rows").is_none());
    assert!(reply.value.get("count").is_none());
    drop(reply);
    f.released();
}

#[test]
fn output_allocation_reentry_cannot_publish_a_catalog_after_context_loss() {
    let _serial = super::super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut host = guard_tests::Fixture::new();
    let context = host.context();
    let service = context.service;
    let f = Fixture::new();
    let catalog = f.catalog(Kind::Effects, 0, 2, &mut || Ok(())).unwrap();
    super::super::super::super::tests::ASSIGN_REENTER_STOP.store(service, Ordering::Release);
    let mut output = VariantOut {
        kind: 0,
        padding: [0; 6],
        payload: 0,
    };
    assert!(publish_reply(
        &mut output,
        reply(context, catalog),
        api().unwrap(),
        service
    ));
    let value: Value =
        serde_json::from_str(&super::super::super::super::tests::output_text(&output)).unwrap();
    assert_eq!(value["ok"], false);
    assert_eq!(value["outcome"], "not_applied");
    assert!(value.get("rows").is_none());
    assert!(value.get("count").is_none());
    f.released();
    super::super::super::super::tests::clear_output(&mut output);
}

#[test]
fn unsupported_class_is_refused_before_resolving_any_catalog_route() {
    let _serial = super::super::super::super::tests::TEST_LOCK.lock().unwrap();
    let mut host = guard_tests::Fixture::new();
    let context = host.context();
    assert!(
        matches!(capture(&context,Kind::Effects,0,1),Err(reason) if reason.contains("observed Mustang Native or Rumble Native"))
    );
    drop(context);
}

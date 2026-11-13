use super::*;
use crate::menu::{ExternalPort, MenuPhase};
#[derive(Default)]
struct MenuStats {
    trace: RefCell<Vec<&'static str>>,
    stats: Rc<Stats>,
    context: RefCell<Option<Rc<Cell<bool>>>>,
    stop: Cell<u8>,
    retention: Cell<u8>,
    count: Cell<i32>,
    builder: Cell<*mut c_void>,
    menu: Cell<*mut c_void>,
    bad_bool: Cell<bool>,
    depth: Cell<u8>,
    cycle: Cell<bool>,
    long_text: Cell<bool>,
    empty_text: Cell<bool>,
    rebind: Cell<bool>,
    change_default: Cell<bool>,
    replace_during_attr: Cell<bool>,
    replace_submenu_during_attr: Cell<bool>,
    count_calls: Cell<usize>,
    append_during_attr: Cell<bool>,
    swap_during_attr: Cell<bool>,
    shared_empty_submenu: Cell<bool>,
    shared_submenu_rows: Cell<i32>,
    actual_item_count: Cell<bool>,
    text_length: Cell<usize>,
}
#[repr(C)]
struct N {
    vt: *const usize,
    refs: Cell<u32>,
    kind: u8,
    state: Rc<MenuStats>,
    parameter: Cell<*mut c_void>,
    builder: *mut c_void,
    items: Vec<*mut c_void>,
    ordinal: i32,
}
unsafe fn n<'a>(p: *mut c_void) -> &'a N {
    unsafe { &*p.cast() }
}
fn new(kind: u8, state: Rc<MenuStats>, builder: *mut c_void, ordinal: i32) -> *mut c_void {
    static F: OnceLock<[usize; 10]> = OnceLock::new();
    static B: OnceLock<[usize; 10]> = OnceLock::new();
    static M: OnceLock<[usize; 10]> = OnceLock::new();
    static I: OnceLock<[usize; 10]> = OnceLock::new();
    let vt = table(
        match kind {
            0 => &F,
            1 => &B,
            2 => &M,
            _ => &I,
        },
        || {
            let mut v = [0; 10];
            v[0] = query as *const () as usize;
            v[1] = retain as *const () as usize;
            v[2] = release as *const () as usize;
            v[3] = match kind {
                0 => create as *const () as usize,
                1 => construct as *const () as usize,
                2 => class as *const () as usize,
                _ => attribute as *const () as usize,
            };
            v[4] = build as *const () as usize;
            v[6] = sub as *const () as usize;
            v[8] = count as *const () as usize;
            v[9] = item as *const () as usize;
            v
        },
    );
    Box::into_raw(Box::new(N {
        vt,
        refs: Cell::new(1),
        kind,
        state,
        parameter: Cell::new(std::ptr::null_mut()),
        builder,
        items: Vec::new(),
        ordinal,
    }))
    .cast()
}
unsafe extern "C" fn retain(p: *mut c_void) -> u32 {
    let o = unsafe { n(p) };
    let v = o.refs.get() + 1;
    o.refs.set(v);
    v
}
unsafe extern "C" fn release(p: *mut c_void) -> u32 {
    let o = unsafe { n(p) };
    let v = o.refs.get() - 1;
    o.refs.set(v);
    if v == 0 {
        let o = unsafe { Box::from_raw(p.cast::<N>()) };
        if o.kind == 2 {
            o.state.trace.borrow_mut().push("menu-destroy");
            for p in o.items {
                unsafe { release(p) };
            }
            unsafe { release(o.builder) };
        } else if o.kind == 1 {
            o.state.trace.borrow_mut().push("builder-cleanup");
            if o.state.stop.get() == 3 {
                if let Some(c) = o.state.context.borrow().as_ref() {
                    c.set(false)
                }
            }
            let p = o.parameter.get();
            if !p.is_null() && o.state.change_default.get() {
                unsafe { fp(p) }
                    .format
                    .default_carrier
                    .set(Some(DefaultCarrier::FloatBits(1.0f64.to_bits())));
            }
            if !p.is_null() {
                unsafe { p_release(p) };
            }
        } else if o.kind == 3 {
            for child in o.items {
                unsafe { release(child) };
            }
            o.state.trace.borrow_mut().push("item-destroy");
        }
    }
    v
}
unsafe extern "C" fn query(p: *mut c_void, i: *const Uid, out: *mut *mut c_void) -> i32 {
    if unsafe { *i } == ffi::UNKNOWN {
        unsafe {
            retain(p);
            *out = p
        };
        0
    } else {
        ffi::NO_INTERFACE
    }
}
unsafe extern "C" fn create(
    p: *mut c_void,
    a: *const Uid,
    b: *const Uid,
    c: *mut *mut c_void,
) -> i32 {
    let o = unsafe { n(p) };
    let class = unsafe { &*a };
    let iid = unsafe { &*b };
    assert!(
        *class
            == Uid {
                a: 0xb56d5931,
                b: 0x2225,
                c: 0x42bf,
                d: [0x8c, 0x93, 0xe7, 0x61, 0x8e, 0xf1, 0x71, 0x35]
            }
    );
    assert!(
        *iid == Uid {
            a: 0x8e75cc94,
            b: 0x8a53,
            c: 0x4ff3,
            d: [0x87, 0xce, 0x79, 0xab, 0x5f, 0x9c, 0xd3, 0xf1]
        }
    );
    let q = new(1, o.state.clone(), std::ptr::null_mut(), 0);
    if o.state.retention.get() == 1 {
        unsafe { retain(q) };
    }
    o.state.builder.set(q);
    unsafe { *c = q };
    0
}
unsafe extern "C" fn construct(p: *mut c_void, a: *mut c_void) -> i32 {
    let o = unsafe { n(p) };
    o.state.trace.borrow_mut().push("construct");
    o.parameter.set(a);
    unsafe { p_retain(a) };
    if o.state.stop.get() == 1 {
        if let Some(c) = o.state.context.borrow().as_ref() {
            c.set(false)
        }
    }
    0
}
unsafe extern "C" fn class(_: *mut c_void) -> *const Uid {
    static ID: Uid = Uid {
        a: 0xb2af5314,
        b: 0xd86d,
        c: 0x4bbe,
        d: [0x92, 0x90, 0x71, 0x22, 0x88, 0x79, 9, 0x5],
    };
    &ID
}
unsafe extern "C" fn attribute(p: *mut c_void, a: *mut Variant, id: i32) -> u8 {
    let o = unsafe { n(p) };
    let out = unsafe { &mut *a };
    if o.state.replace_during_attr.replace(false) {
        let root = unsafe { &mut *o.state.menu.get().cast::<N>() };
        let replacement = new(3, o.state.clone(), o.state.menu.get(), 2);
        let old = std::mem::replace(&mut root.items[1], replacement);
        unsafe { release(old) };
    }
    if o.builder == o.state.menu.get()
        && o.ordinal == 1
        && o.state.replace_submenu_during_attr.replace(false)
    {
        let root = unsafe { n(o.state.menu.get()) };
        let first = unsafe { &mut *root.items[0].cast::<N>() };
        let replacement = make_menu(o.state.clone(), root.builder, 0);
        let old = std::mem::replace(&mut first.items[0], replacement);
        unsafe { release(old) };
    }
    if o.state.swap_during_attr.replace(false) {
        let root = unsafe { &mut *o.state.menu.get().cast::<N>() };
        root.items.swap(0, 1);
    }
    if o.state.append_during_attr.replace(false) {
        let root = unsafe { &mut *o.state.menu.get().cast::<N>() };
        root.items
            .push(new(3, o.state.clone(), o.state.menu.get(), 2));
        o.state.count.set(3);
    }
    if matches!(id, 0 | 1 | 11) && o.state.text_length.get() > 0 {
        out.tag = 19;
        out.payload = string_new(
            &"a".repeat(o.state.text_length.get()),
            o.state.stats.clone(),
        ) as usize as u64;
        return 1;
    }
    if matches!(id, 0 | 1 | 11) && o.state.empty_text.get() {
        out.tag = 3;
        out.payload = 0;
        return 1;
    }
    if matches!(id, 0 | 1 | 11) && o.state.long_text.get() {
        out.tag = 19;
        out.payload = string_new(&"a".repeat(1025), o.state.stats.clone()) as usize as u64;
        return 1;
    }

    match id {
        0 | 1 | 11 => {
            out.tag = 19;
            out.payload = string_new(
                if id == 11 {
                    "External"
                } else if o.ordinal == 0 {
                    "None"
                } else {
                    "Main"
                },
                o.state.stats.clone(),
            ) as usize as u64;
        }
        4 | 5 => {
            out.tag = if o.state.bad_bool.get() { 2 } else { 1 };
            out.payload = if id == 5 || o.ordinal == 0 { 1 } else { 0 };
        }
        12 => {}
        _ => return 0,
    }
    1
}
fn make_menu(state: Rc<MenuStats>, builder: *mut c_void, depth: u8) -> *mut c_void {
    unsafe { retain(builder) };
    let q = new(2, state.clone(), builder, 0);
    let m = unsafe { &mut *q.cast::<N>() };
    for i in 0..state.count.get().min(256) {
        m.items.push(new(3, state.clone(), q, i));
    }
    if depth > 0 && !m.items.is_empty() {
        let child = make_menu(state, builder, depth - 1);
        unsafe { &mut *m.items[0].cast::<N>() }.items.push(child);
    }
    q
}
unsafe extern "C" fn build(p: *mut c_void, _: *mut c_void) -> *mut c_void {
    let b = unsafe { n(p) };
    b.state.trace.borrow_mut().push("build");
    let q = make_menu(b.state.clone(), p, b.state.depth.get());
    if b.state.shared_empty_submenu.get() {
        b.state.count.set(b.state.shared_submenu_rows.get());
        let child = make_menu(b.state.clone(), p, 0);
        b.state.count.set(2);
        let root = unsafe { &mut *q.cast::<N>() };
        unsafe { retain(child) };
        unsafe { &mut *root.items[0].cast::<N>() }.items.push(child);
        unsafe { &mut *root.items[1].cast::<N>() }.items.push(child);
    }
    if b.state.rebind.get() {
        unsafe { fp(b.parameter.get()) }
            .details
            .original
            .set(Some(std::ptr::null_mut()));
    }
    if b.state.retention.get() == 2 {
        unsafe { retain(q) };
    }
    b.state.menu.set(q);
    if b.state.stop.get() == 2 {
        if let Some(c) = b.state.context.borrow().as_ref() {
            c.set(false)
        }
    }
    q
}
unsafe extern "C" fn count(p: *mut c_void) -> i32 {
    let state = &unsafe { n(p) }.state;
    state.count_calls.set(state.count_calls.get() + 1);
    {
        let o = unsafe { n(p) };
        if o.state.actual_item_count.get() {
            o.items.len() as i32
        } else {
            o.state.count.get()
        }
    }
}
unsafe extern "C" fn item(p: *mut c_void, i: i32) -> *mut c_void {
    unsafe { n(p) }.items[i as usize]
}
unsafe extern "C" fn sub(p: *mut c_void) -> *mut c_void {
    let o = unsafe { n(p) };
    if o.state.cycle.get() {
        o.state.menu.get()
    } else {
        o.items.first().copied().unwrap_or(std::ptr::null_mut())
    }
}
fn fixture(env: &TextEnv) -> (Fixture, RawParameter, *mut c_void, Rc<MenuStats>) {
    let (f, p) = list_fixture(env, -1, -1);
    unsafe { (*f.pointer.cast::<FakeParameter>()).name.text = c"sendPort".as_ptr() };
    f.p().edit.tag_override.set(Some(102));
    let stats = Rc::new(MenuStats {
        stats: env.stats.clone(),
        ..MenuStats::default()
    });
    stats.count.set(2);
    *stats.context.borrow_mut() = Some(f.p().context.clone());
    let allocator = new(0, stats.clone(), std::ptr::null_mut(), 0);
    (f, p, allocator, stats)
}
#[test]
fn menu_fixed_builder_copies_metadata_and_cleans_once_before_publication() {
    let e = TextEnv::new();
    let (f, p, a, s) = fixture(&e);
    let r = unsafe {
        p.external_port_menu(NonNull::new(a).unwrap(), ExternalPort::Send, &mut f.guard())
    };
    assert_eq!(r.phase, MenuPhase::ClosedChecked);
    let x = r.result.as_ref().unwrap();
    assert_eq!(x.rows.len(), 2);
    assert_eq!(x.rows[1].title.as_deref(), Some("Main"));
    assert_eq!(x.rows[0].checked, Some(true));
    assert_eq!(x.builder_final_release_count, 0);
    assert_eq!(
        &*s.trace.borrow(),
        &[
            "construct",
            "build",
            "menu-destroy",
            "item-destroy",
            "item-destroy",
            "builder-cleanup"
        ]
    );
    assert_eq!(f.p().writes.get(), 0);
    r.lease.as_ref().unwrap().check(&mut f.guard()).unwrap();
    drop(r);
    unsafe { release(a) };
    assert_eq!(f.p().refs.get(), 1);
}
#[test]
fn menu_context_loss_cannot_build_or_publish_and_never_retries_cleanup() {
    for stop in [1, 2, 3] {
        let e = TextEnv::new();
        let (f, p, a, s) = fixture(&e);
        s.stop.set(stop);
        let r = unsafe {
            p.external_port_menu(NonNull::new(a).unwrap(), ExternalPort::Send, &mut f.guard())
        };
        assert!(r.result.is_err());
        assert_eq!(r.phase, MenuPhase::ReleaseDispatched);
        assert_eq!(
            s.trace
                .borrow()
                .iter()
                .filter(|&&t| t == "construct")
                .count(),
            1
        );
        assert_eq!(
            s.trace
                .borrow()
                .iter()
                .filter(|&&t| t == "builder-cleanup")
                .count(),
            1
        );
        if stop == 1 {
            assert!(!s.trace.borrow().contains(&"build"));
        }
        drop(r);
        unsafe { release(a) };
    }
}
#[test]
fn menu_external_retention_is_unknown_without_force_release() {
    for retention in [1, 2] {
        let e = TextEnv::new();
        let (f, p, a, s) = fixture(&e);
        s.retention.set(retention);
        let r = unsafe {
            p.external_port_menu(NonNull::new(a).unwrap(), ExternalPort::Send, &mut f.guard())
        };
        assert!(r.result.is_err());
        assert_eq!(r.phase, MenuPhase::ReleaseDispatched);
        assert!(!s.trace.borrow().contains(&"builder-cleanup"));
        drop(r);
        unsafe {
            release(if retention == 1 {
                s.builder.get()
            } else {
                s.menu.get()
            });
            release(a)
        };
        assert_eq!(
            s.trace
                .borrow()
                .iter()
                .filter(|&&t| t == "builder-cleanup")
                .count(),
            1
        );
    }
}
#[test]
fn menu_bad_metadata_and_row_bounds_refuse_without_selection() {
    for bad in [0, 1] {
        let e = TextEnv::new();
        let (f, p, a, s) = fixture(&e);
        if bad == 0 {
            s.count.set(257)
        } else {
            s.bad_bool.set(true)
        }
        let r = unsafe {
            p.external_port_menu(NonNull::new(a).unwrap(), ExternalPort::Send, &mut f.guard())
        };
        assert!(r.result.is_err());
        assert_eq!(r.phase, MenuPhase::ReleaseDispatched);
        assert_eq!(f.p().writes.get(), 0);
        assert_eq!(
            s.trace
                .borrow()
                .iter()
                .filter(|&&t| t == "builder-cleanup")
                .count(),
            1
        );
        drop(r);
        unsafe { release(a) };
    }
}
#[test]
fn menu_wrong_port_profile_refuses_before_builder_factory() {
    let e = TextEnv::new();
    let (f, p, a, s) = fixture(&e);
    let r = unsafe {
        p.external_port_menu(
            NonNull::new(a).unwrap(),
            ExternalPort::Return,
            &mut f.guard(),
        )
    };
    assert!(r.result.is_err());
    assert_eq!(r.phase, MenuPhase::Prepared);
    assert!(s.trace.borrow().is_empty());
    drop(r);
    unsafe { release(a) };
}

#[test]
fn menu_submenus_are_bounded_and_cycles_never_publish() {
    for mode in 0..3 {
        let e = TextEnv::new();
        let (f, p, a, s) = fixture(&e);
        s.count.set(1);
        if mode == 0 {
            s.depth.set(3)
        } else if mode == 1 {
            s.depth.set(4)
        } else {
            s.cycle.set(true)
        }
        let r = unsafe {
            p.external_port_menu(NonNull::new(a).unwrap(), ExternalPort::Send, &mut f.guard())
        };
        if mode == 0 {
            let x = r.result.as_ref().unwrap();
            assert_eq!(x.rows.len(), 4);
            assert_eq!(x.rows.last().unwrap().path.len(), 4);
        } else {
            assert!(r.result.is_err());
            assert_eq!(r.phase, MenuPhase::ReleaseDispatched);
        }
        drop(r);
        unsafe { release(a) };
    }
}
#[test]
fn menu_empty_public_string_is_valid_but_long_text_is_refused() {
    for empty in [true, false] {
        let e = TextEnv::new();
        let (f, p, a, s) = fixture(&e);
        s.empty_text.set(empty);
        s.long_text.set(!empty);
        let r = unsafe {
            p.external_port_menu(NonNull::new(a).unwrap(), ExternalPort::Send, &mut f.guard())
        };
        if empty {
            assert_eq!(r.result.as_ref().unwrap().rows[0].name.as_deref(), Some(""));
        } else {
            assert!(r.result.is_err());
        }
        drop(r);
        unsafe { release(a) };
    }
}

#[test]
fn menu_original_rebinding_or_cleanup_metadata_drift_never_publishes_rows() {
    for rebind in [true, false] {
        let e = TextEnv::new();
        let (f, p, a, s) = fixture(&e);
        s.rebind.set(rebind);
        s.change_default.set(!rebind);
        let r = unsafe {
            p.external_port_menu(NonNull::new(a).unwrap(), ExternalPort::Send, &mut f.guard())
        };
        assert!(r.result.is_err());
        assert_eq!(r.phase, MenuPhase::ReleaseDispatched);
        assert_eq!(
            s.trace
                .borrow()
                .iter()
                .filter(|&&t| t == "builder-cleanup")
                .count(),
            1
        );
        drop(r);
        unsafe { release(a) };
    }
}

#[test]
fn review_menu_count_growth_during_attribute_must_not_publish_complete_rows() {
    let e = TextEnv::new();
    let (f, p, a, s) = fixture(&e);
    s.append_during_attr.set(true);
    let r = unsafe {
        p.external_port_menu(NonNull::new(a).unwrap(), ExternalPort::Send, &mut f.guard())
    };
    assert_eq!(s.count.get(), 3);
    assert!(
        r.result.is_err(),
        "reader published phase={:?} rows={:?} after native menu grew to three",
        r.phase,
        r.result
            .as_ref()
            .map(|x| x.rows.iter().map(|r| r.title.clone()).collect::<Vec<_>>())
    );
    drop(r);
    unsafe { release(a) };
}

#[test]
fn review_stable_menu_retains_positive_control() {
    let e = TextEnv::new();
    let (f, p, a, _) = fixture(&e);
    let r = unsafe {
        p.external_port_menu(NonNull::new(a).unwrap(), ExternalPort::Send, &mut f.guard())
    };
    assert_eq!(r.phase, MenuPhase::ClosedChecked);
    assert_eq!(r.result.as_ref().unwrap().rows.len(), 2);
    drop(r);
    unsafe { release(a) };
}

#[test]
fn review_constant_count_reorder_must_not_publish_duplicate_missing_members() {
    let e = TextEnv::new();
    let (f, p, a, s) = fixture(&e);
    s.swap_during_attr.set(true);
    let r = unsafe {
        p.external_port_menu(NonNull::new(a).unwrap(), ExternalPort::Send, &mut f.guard())
    };
    assert_eq!(s.count.get(), 2);
    assert!(
        r.result.is_err(),
        "reader published phase={:?} rows={:?} after native menu reordered None/Main into Main/None",
        r.phase,
        r.result
            .as_ref()
            .map(|x| x.rows.iter().map(|r| r.title.clone()).collect::<Vec<_>>())
    );
    drop(r);
    unsafe { release(a) };
}
#[test]
fn review_shared_empty_submenus_are_valid_not_cycles() {
    let e = TextEnv::new();
    let (f, p, a, s) = fixture(&e);
    s.shared_empty_submenu.set(true);
    s.actual_item_count.set(true);
    let r = unsafe {
        p.external_port_menu(NonNull::new(a).unwrap(), ExternalPort::Send, &mut f.guard())
    };
    assert_eq!(r.phase, MenuPhase::ClosedChecked);
    let rows = &r.result.as_ref().unwrap().rows;
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|r| r.has_submenu));
    drop(r);
    unsafe { release(a) };
}
#[test]
fn review_exact_total_text_bound_accepted_and_overflow_refused() {
    for n in [10, 11] {
        let e = TextEnv::new();
        let (f, p, a, s) = fixture(&e);
        s.count.set(n);
        s.text_length.set(1024);
        let r = unsafe {
            p.external_port_menu(NonNull::new(a).unwrap(), ExternalPort::Send, &mut f.guard())
        };
        assert_eq!(r.result.is_ok(), n == 10);
        drop(r);
        unsafe { release(a) };
    }
}

#[test]
fn review_shared_nonempty_submenu_is_enumerated_at_both_paths() {
    let e = TextEnv::new();
    let (f, p, a, s) = fixture(&e);
    s.shared_empty_submenu.set(true);
    s.shared_submenu_rows.set(1);
    s.actual_item_count.set(true);
    let r = unsafe {
        p.external_port_menu(NonNull::new(a).unwrap(), ExternalPort::Send, &mut f.guard())
    };
    assert_eq!(r.phase, MenuPhase::ClosedChecked);
    let rows = &r.result.as_ref().unwrap().rows;
    assert_eq!(
        rows.iter().map(|r| r.path.clone()).collect::<Vec<_>>(),
        vec![vec![0], vec![0, 0], vec![1], vec![1, 0]]
    );
    drop(r);
    unsafe { release(a) };
}

#[test]
fn menu_constant_count_replacement_with_retained_original_items_is_refused() {
    let e = TextEnv::new();
    let (f, p, a, s) = fixture(&e);
    s.replace_during_attr.set(true);
    let r = unsafe {
        p.external_port_menu(NonNull::new(a).unwrap(), ExternalPort::Send, &mut f.guard())
    };
    assert!(r.result.is_err());
    assert_eq!(r.phase, MenuPhase::ReleaseDispatched);
    assert_eq!(s.count.get(), 2);
    assert_eq!(
        s.trace
            .borrow()
            .iter()
            .filter(|&&v| v == "builder-cleanup")
            .count(),
        1
    );
    assert_eq!(f.p().writes.get(), 0);
    drop(r);
    unsafe { release(a) };
}
#[test]
fn menu_changed_submenu_association_is_refused_after_child_observation() {
    let e = TextEnv::new();
    let (f, p, a, s) = fixture(&e);
    s.depth.set(1);
    s.replace_submenu_during_attr.set(true);
    let r = unsafe {
        p.external_port_menu(NonNull::new(a).unwrap(), ExternalPort::Send, &mut f.guard())
    };
    assert!(r.result.is_err());
    assert_eq!(r.phase, MenuPhase::ReleaseDispatched);
    assert_eq!(
        s.trace
            .borrow()
            .iter()
            .filter(|&&v| v == "builder-cleanup")
            .count(),
        1
    );
    assert_eq!(f.p().writes.get(), 0);
    drop(r);
    unsafe { release(a) };
}
#[test]
fn menu_revalidation_binding_loss_keeps_one_cleanup_and_no_selection() {
    let e = TextEnv::new();
    let (f, p, a, s) = fixture(&e);
    let mut original_guard = f.guard();
    let r = unsafe {
        p.external_port_menu(NonNull::new(a).unwrap(), ExternalPort::Send, &mut || {
            if s.count_calls.get() >= 3 {
                f.p().context.set(false);
            }
            original_guard()
        })
    };
    assert!(r.result.is_err());
    assert_eq!(r.phase, MenuPhase::ReleaseDispatched);
    assert_eq!(
        s.trace
            .borrow()
            .iter()
            .filter(|&&v| v == "builder-cleanup")
            .count(),
        1
    );
    assert_eq!(f.p().writes.get(), 0);
    drop(r);
    unsafe { release(a) };
}

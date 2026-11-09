//! Fake CCL facets; no native runtime, application or SDK is loaded.
use super::*;
use std::{
    cell::{Cell, RefCell},
    ffi::CString,
    sync::OnceLock,
};

#[repr(C)]
struct Facet {
    table: *const usize,
    object: *const Object,
}
struct Object {
    refs: Cell<u32>,
    class: CString,
    uid: Cell<Uid>,
    class_id: Cell<Uid>,
    unknown: Facet,
    node: Facet,
    object: Facet,
    type_info: Facet,
    view: Facet,
    window: Facet,
    children: Facet,
    controller: Facet,
    is_node: bool,
    is_view: bool,
    is_controller: bool,
    component: Cell<*const Object>,
    slot: Cell<*const Object>,
    device: Cell<*const Object>,
    views: RefCell<Vec<*const Object>>,
}
unsafe fn obj<'a>(p: *mut c_void) -> &'a Object {
    unsafe { &*(*p.cast::<Facet>()).object }
}
fn raw(f: &Facet) -> *mut c_void {
    (f as *const Facet).cast_mut().cast()
}
unsafe extern "C" fn retain(p: *mut c_void) -> u32 {
    let o = unsafe { obj(p) };
    let n = o.refs.get() + 1;
    o.refs.set(n);
    n
}
unsafe extern "C" fn release(p: *mut c_void) -> u32 {
    let o = unsafe { obj(p) };
    let n = o.refs.get() - 1;
    o.refs.set(n);
    if o.class.as_bytes() == b"Window" {
        CLOSE_ON_WINDOW_RELEASE.with(|flag| {
            if flag.replace(false) {
                DESKTOP.with(|d| unsafe { &*d.get() }.windows.borrow_mut().clear());
            }
        });
    }
    n
}
unsafe extern "C" fn query(p: *mut c_void, iid: *const Uid, out: *mut *mut c_void) -> i32 {
    let o = unsafe { obj(p) };
    let iid = unsafe { &*iid };
    let facet = if *iid == UNKNOWN {
        Some(&o.unknown)
    } else if *iid == OBJECT {
        Some(&o.object)
    } else if *iid == NODE && o.is_node {
        Some(&o.node)
    } else if *iid == VIEW && o.is_view {
        Some(&o.view)
    } else if *iid == CONTROLLER && o.is_controller {
        Some(&o.controller)
    } else {
        None
    };
    let Some(facet) = facet else {
        return -1;
    };
    unsafe {
        retain(raw(facet));
        *out = raw(facet);
    }
    0
}
unsafe extern "C" fn type_info(p: *mut c_void) -> *mut c_void {
    raw(&unsafe { obj(p) }.type_info)
}
unsafe extern "C" fn class_name(p: *mut c_void) -> *const c_char {
    let object = unsafe { obj(p) };
    if object.class.as_bytes() == b"ExternalEffectComponent" {
        REPLACE_ON_COMPONENT_CLASS.with(|state| {
            let (remaining, replacement) = state.get();
            if remaining > 0 {
                state.set((remaining - 1, replacement));
                if remaining == 1 {
                    DESKTOP
                        .with(|d| *unsafe { &*d.get() }.windows.borrow_mut() = vec![replacement]);
                }
            }
        });
    }
    object.class.as_ptr()
}
unsafe extern "C" fn object_uid(p: *mut c_void) -> *const Uid {
    unsafe { obj(p) }.uid.as_ptr()
}
unsafe extern "C" fn class_uid(p: *mut c_void) -> *const Uid {
    unsafe { obj(p) }.class_id.as_ptr()
}
#[repr(C)]
struct ConstName {
    ascii: *const c_char,
    opaque: *const c_void,
}
thread_local! {
    static SLOT:ConstName=const{ConstName{ascii:c"DeviceSlot".as_ptr(),opaque:std::ptr::null()}};
    static DEVICE:ConstName=const{ConstName{ascii:c"Device".as_ptr(),opaque:std::ptr::null()}};
    static DESKTOP:Cell<*mut Desktop>=const{Cell::new(std::ptr::null_mut())};
    static ITER_COUNTS:Cell<(u32,u32)>=const{Cell::new((0,0))};
    static CLOSE_ON_ITER_RELEASE:Cell<bool>=const{Cell::new(false)};
    static CLOSE_ON_WINDOW_RELEASE:Cell<bool>=const{Cell::new(false)};
    static REPLACE_ON_COMPONENT_CLASS:Cell<(u32,*const Object)>=const{Cell::new((0,std::ptr::null()))};
}
unsafe extern "C" fn constant(s: *const c_char) -> *const c_void {
    match unsafe { std::ffi::CStr::from_ptr(s) }.to_bytes() {
        b"DeviceSlot" => SLOT.with(|v| (v as *const ConstName).cast()),
        b"Device" => DEVICE.with(|v| (v as *const ConstName).cast()),
        _ => std::ptr::null(),
    }
}
unsafe extern "C" fn find(p: *mut c_void, s: *const c_void) -> *mut c_void {
    let o = unsafe { obj(p) };
    let name = unsafe { std::ffi::CStr::from_ptr((*s.cast::<ConstName>()).ascii) }.to_bytes();
    let target = match name {
        b"DeviceSlot" => o.slot.get(),
        b"Device" => o.device.get(),
        _ => std::ptr::null(),
    };
    if target.is_null() {
        std::ptr::null_mut()
    } else {
        raw(&unsafe { &*target }.node)
    }
}
unsafe extern "C" fn view_children(p: *mut c_void) -> *mut c_void {
    raw(&unsafe { obj(p) }.children)
}
unsafe extern "C" fn controller(p: *mut c_void) -> *mut c_void {
    let target = unsafe { obj(p) }.component.get();
    if target.is_null() {
        std::ptr::null_mut()
    } else {
        raw(&unsafe { &*target }.unknown)
    }
}
unsafe extern "C" fn is_child(p: *mut c_void, child: *mut c_void, deep: u8) -> u8 {
    let parent = unsafe { obj(p) };
    let child = unsafe { obj(child) } as *const Object;
    u8::from(deep == 0 && parent.views.borrow().contains(&child))
}
#[repr(C)]
struct Iterator {
    table: *const usize,
    parent: *const Object,
    index: Cell<usize>,
}
unsafe extern "C" fn iter_release(p: *mut c_void) -> u32 {
    drop(unsafe { Box::from_raw(p.cast::<Iterator>()) });
    ITER_COUNTS.with(|c| {
        let (a, b) = c.get();
        c.set((a, b + 1));
    });
    CLOSE_ON_ITER_RELEASE.with(|flag| {
        if flag.replace(false) {
            DESKTOP.with(|d| unsafe { &*d.get() }.windows.borrow_mut().clear());
        }
    });
    0
}
unsafe extern "C" fn iter_done(p: *mut c_void) -> u8 {
    let i = unsafe { &*p.cast::<Iterator>() };
    u8::from(i.index.get() >= unsafe { &*i.parent }.views.borrow().len())
}
unsafe extern "C" fn iter_next(p: *mut c_void) -> *mut c_void {
    let i = unsafe { &*p.cast::<Iterator>() };
    let parent = unsafe { &*i.parent };
    let index = i.index.get();
    i.index.set(index + 1);
    parent
        .views
        .borrow()
        .get(index)
        .map_or(std::ptr::null_mut(), |p| raw(&unsafe { &**p }.view))
}
unsafe extern "C" fn create_iterator(p: *mut c_void) -> *mut c_void {
    static TABLE: OnceLock<[usize; 5]> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        [
            0,
            0,
            iter_release as *const () as usize,
            iter_done as *const () as usize,
            iter_next as *const () as usize,
        ]
    });
    ITER_COUNTS.with(|c| {
        let (a, b) = c.get();
        c.set((a + 1, b));
    });
    Box::into_raw(Box::new(Iterator {
        table: table.as_ptr(),
        parent: unsafe { obj(p) },
        index: Cell::new(0),
    }))
    .cast()
}
#[repr(C)]
struct Desktop {
    table: *const usize,
    windows: RefCell<Vec<*const Object>>,
}
unsafe extern "C" fn get_desktop() -> *mut c_void {
    DESKTOP.with(|d| d.get().cast())
}
unsafe extern "C" fn window_count(p: *mut c_void) -> i32 {
    unsafe { &*p.cast::<Desktop>() }.windows.borrow().len() as i32
}
unsafe extern "C" fn window_at(p: *mut c_void, index: i32) -> *mut c_void {
    unsafe { &*p.cast::<Desktop>() }
        .windows
        .borrow()
        .get(index as usize)
        .map_or(std::ptr::null_mut(), |o| raw(&unsafe { &**o }.window))
}
fn base() -> [usize; 33] {
    let mut table = [0; 33];
    table[0] = query as *const () as usize;
    table[1] = retain as *const () as usize;
    table[2] = release as *const () as usize;
    table
}
fn tables() -> &'static [[usize; 33]; 8] {
    static TABLES: OnceLock<[[usize; 33]; 8]> = OnceLock::new();
    TABLES.get_or_init(|| {
        let mut tables = [base(); 8];
        tables[1][4] = object_uid as *const () as usize;
        tables[1][5] = class_uid as *const () as usize;
        tables[1][10] = find as *const () as usize;
        tables[1][13] = find as *const () as usize;
        tables[2][3] = type_info as *const () as usize;
        tables[3][5] = class_name as *const () as usize;
        tables[4][20] = view_children as *const () as usize;
        tables[4][32] = controller as *const () as usize;
        tables[5][30] = controller as *const () as usize;
        tables[6][11] = create_iterator as *const () as usize;
        tables[6][12] = is_child as *const () as usize;
        tables
    })
}
fn id(n: u32) -> Uid {
    Uid {
        a: n,
        b: 0,
        c: 0,
        d: [0; 8],
    }
}
// Every fake facet holds its owning object's stable address. Moving objects
// inside a Vec would invalidate those pointers when the arena grows.
#[allow(clippy::vec_box)]
struct World {
    objects: Vec<Box<Object>>,
    desktop: Box<Desktop>,
    expected: *const Object,
    editor: *const Object,
    slot: *const Object,
    component: *const Object,
    terminal: *const Object,
    window: *const Object,
}
impl World {
    fn add(
        &mut self,
        class: &str,
        uid: Uid,
        class_id: Uid,
        node: bool,
        view: bool,
        ctrl: bool,
    ) -> *const Object {
        let blank = || Facet {
            table: std::ptr::null(),
            object: std::ptr::null(),
        };
        let mut o = Box::new(Object {
            refs: Cell::new(1),
            class: CString::new(class).unwrap(),
            uid: Cell::new(uid),
            class_id: Cell::new(class_id),
            unknown: blank(),
            node: blank(),
            object: blank(),
            type_info: blank(),
            view: blank(),
            window: blank(),
            children: blank(),
            controller: blank(),
            is_node: node,
            is_view: view,
            is_controller: ctrl,
            component: Cell::new(std::ptr::null()),
            slot: Cell::new(std::ptr::null()),
            device: Cell::new(std::ptr::null()),
            views: RefCell::new(Vec::new()),
        });
        let p = &*o as *const Object;
        for (i, facet) in [
            &mut o.unknown,
            &mut o.node,
            &mut o.object,
            &mut o.type_info,
            &mut o.view,
            &mut o.window,
            &mut o.children,
            &mut o.controller,
        ]
        .into_iter()
        .enumerate()
        {
            facet.object = p;
            facet.table = tables()[i].as_ptr();
        }
        self.objects.push(o);
        p
    }
    fn new() -> Self {
        static TABLE: OnceLock<[usize; 8]> = OnceLock::new();
        let table = TABLE.get_or_init(|| {
            let mut t = [0; 8];
            t[6] = window_count as *const () as usize;
            t[7] = window_at as *const () as usize;
            t
        });
        let desktop = Box::new(Desktop {
            table: table.as_ptr(),
            windows: RefCell::new(Vec::new()),
        });
        let null = std::ptr::null();
        let mut w = Self {
            objects: Vec::new(),
            desktop,
            expected: null,
            editor: null,
            slot: null,
            component: null,
            terminal: null,
            window: null,
        };
        w.expected = w.add("AudioEffect", id(1), EXTERNAL_MONO, true, false, false);
        w.editor = w.add("DeviceEditor", id(2), id(100), true, false, false);
        w.slot = w.add("AudioInsertSlot", id(3), id(101), true, false, false);
        w.component = w.add("ExternalEffectComponent", id(4), id(0), false, false, true);
        w.window = w.add("Window", id(5), id(102), false, true, false);
        unsafe { &*w.editor }.slot.set(w.slot);
        unsafe { &*w.slot }.device.set(w.expected);
        unsafe { &*w.window }.component.set(w.editor);
        let mut parent = w.window;
        for i in 0..4 {
            let view = w.add("View", id(10 + i), id(0), false, true, false);
            unsafe { &*parent }.views.borrow_mut().push(view);
            parent = view;
        }
        let dummy = w.add("View", id(20), id(0), false, true, false);
        w.terminal = w.add("View", id(21), id(0), false, true, false);
        unsafe { &*parent }
            .views
            .borrow_mut()
            .extend([dummy, w.terminal]);
        unsafe { &*w.terminal }.component.set(w.component);
        w.desktop.windows.borrow_mut().push(w.window);
        DESKTOP.with(|d| {
            assert!(d.get().is_null());
            d.set(&mut *w.desktop);
        });
        ITER_COUNTS.with(|c| c.set((0, 0)));
        w
    }
    fn resolve(&self) -> Result<ExternalEditorLease, Error> {
        let raw = raw(&unsafe { &*self.expected }.unknown);
        unsafe {
            retain(raw);
            resolve_external_owned(
                NonNull::new(raw).unwrap(),
                GuiApi {
                    get_desktop,
                    get_constant_string: constant,
                },
            )
        }
    }
}
impl Drop for World {
    fn drop(&mut self) {
        DESKTOP.with(|d| d.set(std::ptr::null_mut()));
        CLOSE_ON_ITER_RELEASE.with(|f| f.set(false));
        CLOSE_ON_WINDOW_RELEASE.with(|f| f.set(false));
        REPLACE_ON_COMPONENT_CLASS.with(|f| f.set((0, std::ptr::null())));
        for o in &self.objects {
            assert_eq!(
                o.refs.get(),
                1,
                "leaked {} native references",
                o.class.to_string_lossy()
            );
        }
        ITER_COUNTS.with(|c| {
            let (a, b) = c.get();
            assert_eq!(a, b, "iterator references balanced");
        });
    }
}

#[test]
fn canonical_resolution_revalidates_and_releases_every_owned_facet() {
    let w = World::new();
    let lease = w.resolve().unwrap();
    assert_eq!(lease.identity().matched_by, DeviceMatch::CanonicalIUnknown);
    assert_eq!(
        lease.controller().as_ptr(),
        raw(&unsafe { &*w.component }.controller)
    );
    lease.check().unwrap();
    assert!(unsafe { &*w.window }.refs.get() > 1);
    drop(lease);
    assert_eq!(unsafe { &*w.expected }.refs.get(), 1);
}
#[test]
fn same_class_different_device_uid_is_rejected() {
    let mut w = World::new();
    let other = w.add("AudioEffect", id(99), EXTERNAL_MONO, true, false, false);
    unsafe { &*w.slot }.device.set(other);
    assert!(matches!(w.resolve(), Err(Error::MissingEditor)));
}
#[test]
fn stable_nonzero_uid_and_class_can_match_a_distinct_canonical_adapter() {
    let mut w = World::new();
    let alias = w.add("AudioEffect", id(1), EXTERNAL_MONO, true, false, false);
    unsafe { &*w.slot }.device.set(alias);
    let lease = w.resolve().unwrap();
    assert_eq!(lease.identity().matched_by, DeviceMatch::StableNodeUid);
    lease.check().unwrap();
}
#[test]
fn wrong_class_same_uid_and_nonexternal_devices_are_rejected() {
    let mut w = World::new();
    let other = w.add("AudioEffect", id(1), EXTERNAL_STEREO, true, false, false);
    unsafe { &*w.slot }.device.set(other);
    assert!(matches!(w.resolve(), Err(Error::MissingEditor)));
    unsafe { &*w.expected }.class_id.set(id(999));
    assert!(matches!(w.resolve(), Err(Error::UnsupportedDevice)));
}
#[test]
fn missing_and_multiple_matching_editor_windows_fail_closed() {
    let mut w = World::new();
    w.desktop.windows.borrow_mut().clear();
    assert!(matches!(w.resolve(), Err(Error::MissingEditor)));
    w.desktop.windows.borrow_mut().push(w.window);
    let second = w.add("Window", id(80), id(102), false, true, false);
    unsafe { &*second }.component.set(w.editor);
    w.desktop.windows.borrow_mut().push(second);
    assert!(matches!(w.resolve(), Err(Error::AmbiguousEditor)));
}
#[test]
fn missing_view_path_and_nonexternal_component_fail_closed() {
    let mut w = World::new();
    let other = w.add("AnotherComponent", id(100), id(0), false, false, true);
    unsafe { &*w.terminal }.component.set(other);
    assert!(matches!(w.resolve(), Err(Error::MissingExternalController)));
    unsafe { &*w.window }.views.borrow_mut().clear();
    assert!(matches!(w.resolve(), Err(Error::MissingViewPath)));
}
#[test]
fn detached_window_replaced_device_or_controller_invalidates_the_lease() {
    let mut w = World::new();
    let lease = w.resolve().unwrap();
    w.desktop.windows.borrow_mut().clear();
    assert!(lease.check().is_err());
    w.desktop.windows.borrow_mut().push(w.window);
    let other = w.add(
        "ExternalEffectComponent",
        id(100),
        id(0),
        false,
        false,
        true,
    );
    unsafe { &*w.terminal }.component.set(other);
    assert_eq!(lease.check(), Err(Error::Stale));
    unsafe { &*w.terminal }.component.set(w.component);
    let device = w.add("AudioEffect", id(99), EXTERNAL_MONO, true, false, false);
    unsafe { &*w.slot }.device.set(device);
    assert!(lease.check().is_err());
}
#[test]
fn validation_iterator_release_can_close_window_without_stale_success() {
    let w = World::new();
    let lease = w.resolve().unwrap();
    CLOSE_ON_ITER_RELEASE.with(|f| f.set(true));
    assert!(lease.check().is_err());
}

#[test]
fn fresh_frame_cleanup_reentry_is_rechecked_before_check_returns_success() {
    let w = World::new();
    let lease = w.resolve().unwrap();
    CLOSE_ON_WINDOW_RELEASE.with(|flag| flag.set(true));
    assert_eq!(lease.check(), Err(Error::Stale));
    assert!(w.desktop.windows.borrow().is_empty());
}

#[test]
fn late_relationship_getter_same_count_window_replacement_is_stale() {
    let mut w = World::new();
    let lease = w.resolve().unwrap();
    let replacement = w.add("Window", id(600), id(102), false, true, false);
    // The first class read is fresh discovery. The second is the final pinned
    // relationship phase, after its earlier membership observation.
    REPLACE_ON_COMPONENT_CLASS.with(|state| state.set((2, replacement)));
    assert_eq!(lease.check(), Err(Error::Stale));
    assert_eq!(w.desktop.windows.borrow().len(), 1);
    assert_eq!(w.desktop.windows.borrow()[0], replacement);
}
#[test]
fn detached_parent_view_link_and_changed_native_uid_are_detected() {
    let w = World::new();
    let lease = w.resolve().unwrap();
    unsafe { &*w.expected }.uid.set(id(200));
    assert!(lease.check().is_err());
    unsafe { &*w.expected }.uid.set(id(1));
    unsafe { &*w.window }.views.borrow_mut().clear();
    assert!(lease.check().is_err());
}
#[test]
fn window_and_thread_bounds_precede_unbounded_native_work() {
    let w = World::new();
    let mut lease = w.resolve().unwrap();
    lease.thread = std::thread::spawn(|| std::thread::current().id())
        .join()
        .unwrap();
    assert_eq!(lease.check(), Err(Error::WrongThread));
    drop(lease);
    w.desktop.windows.borrow_mut().resize(65, w.window);
    assert!(matches!(w.resolve(), Err(Error::WindowLimit)));
}

use super::*;
use std::sync::atomic::AtomicUsize;

#[repr(C)]
struct Mock {
    table: *const *const c_void,
    refs: AtomicU32,
    controller: *mut c_void,
    parameter: *mut c_void,
    alias_queries: AtomicUsize,
    count_calls: AtomicUsize,
    find_calls: AtomicUsize,
}
unsafe extern "C" fn retain(p: *mut c_void) -> u32 {
    unsafe { &*p.cast::<Mock>() }
        .refs
        .fetch_add(1, Ordering::Relaxed)
        + 1
}
unsafe extern "C" fn release(p: *mut c_void) -> u32 {
    unsafe { &*p.cast::<Mock>() }
        .refs
        .fetch_sub(1, Ordering::Relaxed)
        - 1
}
unsafe extern "C" fn query_mock(p: *mut c_void, iid: *const Uid, out: *mut *mut c_void) -> i32 {
    let mock = unsafe { &*p.cast::<Mock>() };
    if unsafe { *iid } == ALIAS {
        mock.alias_queries.fetch_add(1, Ordering::Relaxed);
    }
    unsafe { *out = ptr::null_mut() };
    if unsafe { *iid } != CONTROLLER || mock.controller.is_null() {
        return NO_INTERFACE;
    }
    unsafe {
        retain(mock.controller);
        *out = mock.controller;
    }
    OK
}
unsafe extern "C" fn count(p: *mut c_void) -> i32 {
    unsafe { &*p.cast::<Mock>() }
        .count_calls
        .fetch_add(1, Ordering::Relaxed);
    0
}
unsafe extern "C" fn find(p: *mut c_void, name: *const PlainCString) -> *mut c_void {
    let mock = unsafe { &*p.cast::<Mock>() };
    mock.find_calls.fetch_add(1, Ordering::Relaxed);
    let name = unsafe { std::ffi::CStr::from_ptr((*name).text) };
    if name == c"module.gain" {
        mock.parameter
    } else {
        ptr::null_mut()
    }
}
unsafe extern "C" fn empty() -> *mut c_void {
    ptr::null_mut()
}
unsafe extern "C" fn text(_: *const c_char) -> *const PlainCString {
    ptr::null()
}
unsafe extern "C" fn thread() -> i64 {
    0
}
fn api_mock() -> Api {
    Api {
        gui: empty,
        objects: empty,
        main_thread: empty,
        thread_self: thread,
        cstring: text,
        empty,
        desktop: empty,
        scripting: empty,
        plugin_manager: empty,
        file_system: empty,
    }
}
fn make(table: *const *const c_void) -> Mock {
    Mock {
        table,
        refs: AtomicU32::new(1),
        controller: ptr::null_mut(),
        parameter: ptr::null_mut(),
        alias_queries: AtomicUsize::new(0),
        count_calls: AtomicUsize::new(0),
        find_calls: AtomicUsize::new(0),
    }
}

#[test]
fn direct_dsp_lookup_accepts_zero_count_without_alias_fallback() {
    let mut table = [ptr::null::<c_void>(); 6];
    table[0] = query_mock as *const c_void;
    table[1] = retain as *const c_void;
    table[2] = release as *const c_void;
    table[3] = count as *const c_void;
    table[5] = find as *const c_void;
    let (mut target, mut controller, mut parameter) = (
        make(table.as_ptr()),
        make(table.as_ptr()),
        make(table.as_ptr()),
    );
    target.controller = ptr::addr_of_mut!(controller).cast();
    controller.parameter = ptr::addr_of_mut!(parameter).cast();
    let found = Controller::direct(ptr::addr_of_mut!(target).cast()).unwrap();
    assert_eq!(controller.count_calls.load(Ordering::Relaxed), 0);
    assert_eq!(found.count().unwrap(), 0);
    let parameter_owner = found.find("module.gain", &api_mock()).unwrap();
    assert_eq!(controller.find_calls.load(Ordering::Relaxed), 1);
    assert_eq!(parameter.refs.load(Ordering::Relaxed), 2);
    drop(parameter_owner);
    let provider = Provider::DspValue(found);
    assert_eq!(provider.source(), "dsp");
    assert!(provider.scalar().is_some());
    drop(provider);
    assert_eq!(target.alias_queries.load(Ordering::Relaxed), 0);
    assert_eq!(controller.refs.load(Ordering::Relaxed), 1);
    assert_eq!(parameter.refs.load(Ordering::Relaxed), 1);
    target.controller = ptr::null_mut();
    assert!(Controller::direct(ptr::addr_of_mut!(target).cast()).is_err());
    assert_eq!(target.alias_queries.load(Ordering::Relaxed), 0);
}

#[test]
fn dsp_access_metadata_preserves_canonical_and_parameter_local_identity() {
    let record = fender_native_dsp_api::Parameter {
        index: 17,
        tag: 42,
        name: "module.gain".into(),
        title: "Gain".into(),
        folder: 3,
        raw_flags: 9,
    };
    let access = parameters::Description {
        name: "gain".into(),
        tag: 7,
        kind: parameters::Kind::Float,
        enabled: true,
        read_only: false,
        raw: Some(parameters::NativeValue {
            variant_tag: 2,
            value: parameters::Value::Float(-0.0),
        }),
        min: None,
        max: None,
        default_value: None,
        normalized: parameters::Normalized::Finite(0.0),
        display: "0".into(),
        display_parser: parameters::DisplayParser::PlainNumeric,
        support: parameters::Support {
            raw_number: true,
            normalized: true,
            display_text: true,
        },
    };
    let value = dsp_record_value(&record, Ok(access));
    assert_eq!(value["index"], 17);
    assert_eq!(value["name"], "module.gain");
    assert_eq!(value["tag"], 42);
    assert_eq!(value["native_description"]["name"], "gain");
    assert_eq!(value["native_description"]["tag"], 7);
    assert_eq!(value["native_description"]["writable"], true);
    assert_eq!(
        value["native_description"]["native_value"]["data"]["bits_hex"],
        "8000000000000000"
    );
    assert!(value.get("access_error").is_none());
    let missing = dsp_record_value(&record, Err("not exposed by direct Device".into()));
    assert_eq!(missing["name"], "module.gain");
    assert_eq!(missing["tag"], 42);
    assert!(missing["native_description"].is_null());
    assert_eq!(missing["access_error"], "not exposed by direct Device");
    assert!(missing.get("writable").is_none());
}

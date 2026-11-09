use super::*;

const STORABLE: Uid = Uid {
    a: 0xb131b242,
    b: 0xbff4,
    c: 0x446f,
    d: [0x96, 0xe4, 0xec, 0x19, 0xdf, 0xad, 0x11, 0xe6],
};
const PRESET_MEDIATOR: Uid = Uid {
    a: 0x25e340af,
    b: 0xd6e6,
    c: 0x49cf,
    d: [0xb2, 0x58, 0x87, 0x47, 0x6e, 0x4a, 0x27, 0x3a],
};
const PRESET_METADATA: Uid = Uid {
    a: 0x2e962128,
    b: 0x1615,
    c: 0x4407,
    d: [0x9e, 0xf2, 0x14, 0x84, 0xa2, 0x37, 0x37, 0x1c],
};
const DATA_TARGET: Uid = Uid {
    a: 0x9967d17c,
    b: 0xa519,
    c: 0x4c00,
    d: [0xa7, 0xcd, 0x92, 0x1e, 0x4d, 0x7c, 0x5e, 0xcb],
};
const PRESET_MANAGER: Uid = Uid {
    a: 0x861a7175,
    b: 0xac7b,
    c: 0x457c,
    d: [0xac, 0xcd, 0x8b, 0x35, 0xdf, 0xcc, 0xea, 0x9d],
};
const CONTROLLER_PROVIDER: Uid = Uid {
    a: 0x16ad4d2a,
    b: 0x9e14,
    c: 0x42e1,
    d: [0x9d, 0xa5, 0x72, 0xde, 0x99, 0x2b, 0xf5, 0x16],
};
const INTERFACES: [(&str, &Uid); 5] = [
    ("storable", &STORABLE),
    ("preset_mediator", &PRESET_MEDIATOR),
    ("preset_metadata", &PRESET_METADATA),
    ("data_target", &DATA_TARGET),
    ("controller_provider", &CONTROLLER_PROVIDER),
];

fn object(object: *mut c_void) -> Value {
    let mut value = json!({"present":true,"native_class":class_name(object)});
    for (name, iid) in INTERFACES {
        value[name] = json!(query(object, iid).is_some());
    }
    match instance(object) {
        Ok(identity) => value["identity"] = json!(identity),
        Err(reason) => {
            value["identity"] = Value::Null;
            value["identity_error"] = json!(reason);
        }
    }
    value
}

fn manager(a: &Api) -> bool {
    let table = unsafe { (a.objects)() };
    if table.is_null() {
        return false;
    }
    let by_id: unsafe extern "C" fn(*mut c_void, *const Uid) -> *mut c_void =
        unsafe { method(table, 8) };
    let Some(service) = hold(unsafe { by_id(table, &PRESET_MANAGER) }) else {
        return false;
    };
    query(service.0, &PRESET_MANAGER).is_some()
}

fn related(ctx: &Context<'_>) -> Result<Value, String> {
    let mut value = json!({"device":object(ctx.target.0),"alias":{"present":false},"parent":{"present":false},"presets_child":{"present":false}});
    ctx.check()?;
    if let Some(alias) = query(ctx.target.0, &ALIAS) {
        let get: Borrowed = unsafe { method(alias.0, 3) };
        if let Some(plugin) = hold(unsafe { get(alias.0) }) {
            value["alias"] = object(plugin.0);
        }
    }
    ctx.check()?;
    let get_parent: Borrowed = unsafe { method(ctx.node.0, 6) };
    if let Some(parent) = hold(unsafe { get_parent(ctx.node.0) }) {
        value["parent"] = object(parent.0);
        ctx.check()?;
        let get_string = constant_string().ok_or("native string constants unavailable")?;
        let name = unsafe { get_string(c"Presets".as_ptr()) };
        if name.is_null() {
            return Err("Presets constant unavailable".into());
        }
        let find: unsafe extern "C" fn(*mut c_void, *const PlainString) -> *mut c_void =
            unsafe { method(parent.0, 10) };
        if let Some(presets) = hold(unsafe { find(parent.0, name) }) {
            value["presets_child"] = object(presets.0);
        }
        if unsafe { get_parent(ctx.node.0) } != parent.0 {
            return Err("native target parent changed during capability inspection".into());
        }
    }
    ctx.check()?;
    value["preset_manager"] = json!(manager(ctx.api));
    Ok(value)
}

pub(super) fn inspect(ctx: &Context<'_>) -> Result<Value, String> {
    ctx.check()?;
    let value = related(ctx);
    ctx.check()?;
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    #[repr(C)]
    struct Mock {
        table: *const *const c_void,
        refs: AtomicU32,
        queries: std::cell::RefCell<Vec<Uid>>,
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
        mock.queries.borrow_mut().push(unsafe { *iid });
        unsafe { *out = ptr::null_mut() };
        if unsafe { *iid } != STORABLE && unsafe { *iid } != PRESET_METADATA {
            return NO_INTERFACE;
        }
        unsafe {
            retain(p);
            *out = p;
        }
        OK
    }
    #[test]
    fn capabilities_are_fixed_queries_and_release_every_temporary_interface() {
        let table = [
            query_mock as *const c_void,
            retain as *const c_void,
            release as *const c_void,
        ];
        let mut mock = Mock {
            table: table.as_ptr(),
            refs: AtomicU32::new(1),
            queries: std::cell::RefCell::new(Vec::new()),
        };
        let value = object(ptr::addr_of_mut!(mock).cast());
        assert_eq!(value["storable"], true);
        assert_eq!(value["preset_metadata"], true);
        assert_eq!(value["preset_mediator"], false);
        assert_eq!(value["data_target"], false);
        assert_eq!(value["controller_provider"], false);
        assert!(value["identity"].is_null());
        assert!(value["identity_error"].is_string());
        assert_eq!(mock.refs.load(Ordering::Relaxed), 1);
        assert!(mock.queries.borrow().iter().all(|iid| {
            *iid == OBJECT || *iid == NODE || INTERFACES.iter().any(|(_, known)| **known == *iid)
        }));
        assert_eq!(mock.queries.borrow().len(), 7);
    }
}

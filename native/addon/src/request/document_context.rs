use super::*;
use std::sync::atomic::AtomicU64;

pub(crate) const API_NAME: &str = "native_document_context_v1";
const MAX_TITLE_UNITS: usize = 4096;
const MAX_URL_UNITS: usize = 16384;
static NEXT_SERVICE_SERIAL: AtomicU64 = AtomicU64::new(1);

pub(crate) fn next_service_serial() -> Option<u64> {
    reserve_serial(&NEXT_SERVICE_SERIAL)
}

fn reserve_serial(counter: &AtomicU64) -> Option<u64> {
    let mut next = counter.load(Ordering::Acquire);
    loop {
        let following = next.checked_add(1).filter(|_| next != 0)?;
        match counter.compare_exchange_weak(next, following, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => return Some(next),
            Err(actual) => next = actual,
        }
    }
}

pub(crate) struct Registry {
    serial: u64,
    generation: u64,
    observed: bool,
    identity: Option<Owned>,
}

impl Registry {
    pub(crate) fn new(serial: u64) -> Self {
        Self {
            serial,
            generation: 0,
            observed: false,
            identity: None,
        }
    }

    fn token(&self) -> String {
        format!("ndc1:{:016x}:{}", self.serial, self.generation)
    }
}

pub(crate) fn drain(service: *mut Service) {
    let state = unsafe { &*service };
    let retired = {
        let registry = unsafe { &mut *state.document_context.get() };
        registry.observed = false;
        registry.identity.take()
    };
    state
        .document_context_pending
        .store(false, Ordering::Release);
    drop(retired);
}

fn manager(a: &Api) -> Result<Owned, String> {
    let table = unsafe { (a.objects)() };
    let name = unsafe { (a.cstring)(c"hostapp".as_ptr()) };
    if table.is_null() || name.is_null() {
        return Err("native host table unavailable".into());
    }
    let get: unsafe extern "C" fn(*mut c_void, *const PlainCString) -> *mut c_void =
        unsafe { method(table, 9) };
    let host = hold(unsafe { get(table, name) }).ok_or("hostapp unavailable")?;
    let node = query(host.0, &NODE).ok_or("hostapp node unavailable")?;
    let name = unsafe {
        constant_string().ok_or("native text API unavailable")?(c"DocumentManager".as_ptr())
    };
    if name.is_null() {
        return Err("document manager identifier unavailable".into());
    }
    let find: unsafe extern "C" fn(*mut c_void, *const PlainString) -> *mut c_void =
        unsafe { method(node.0, 10) };
    let found = hold(unsafe { find(node.0, name) }).ok_or("document manager unavailable")?;
    query(found.0, &DOC_MANAGER).ok_or_else(|| "document manager interface unavailable".into())
}

struct NativeText {
    repr: PlainString,
}

impl NativeText {
    fn empty(a: &Api) -> Result<Self, String> {
        let owner = hold(unsafe { (a.empty)() }).ok_or("native empty string unavailable")?;
        let private: unsafe extern "C" fn(*mut c_void) -> *mut c_void =
            unsafe { method(owner.0, 21) };
        let repr = PlainString {
            object: owner.0,
            opaque_debug: unsafe { private(owner.0) },
        };
        std::mem::forget(owner);
        Ok(Self { repr })
    }
}

impl Drop for NativeText {
    fn drop(&mut self) {
        if !self.repr.object.is_null() {
            unsafe { (unknown_vtable(self.repr.object).release)(self.repr.object) };
        }
    }
}

fn text(object: *mut c_void, limit: usize) -> Result<String, String> {
    let copied = copy_string(object, limit)?;
    if copied.contains('\0') {
        return Err("native document text contains NUL".into());
    }
    Ok(copied)
}

struct Snapshot {
    document: Option<Owned>,
    identity: Option<Owned>,
    manager: Owned,
    service: *mut Service,
    started: Instant,
    api: &'static Api,
}

impl Snapshot {
    fn capture(service: *mut Service, manager: Owned, api: &'static Api) -> Result<Self, String> {
        check_service_active(service)?;
        let active: Borrowed = unsafe { method(manager.0, 9) };
        let document = hold(unsafe { active(manager.0) });
        let identity = document
            .as_ref()
            .map(|document| query(document.0, &UNKNOWN).ok_or("document identity unavailable"))
            .transpose()?;
        let snapshot = Self {
            document,
            identity,
            manager,
            service,
            started: Instant::now(),
            api,
        };
        snapshot.check()?;
        Ok(snapshot)
    }

    fn document(&self) -> *mut c_void {
        self.document
            .as_ref()
            .map_or(ptr::null_mut(), |owner| owner.0)
    }

    fn identity(&self) -> *mut c_void {
        self.identity
            .as_ref()
            .map_or(ptr::null_mut(), |owner| owner.0)
    }

    fn check(&self) -> Result<(), String> {
        check_service_active(self.service)?;
        if !on_main(self.api) {
            return Err("document context left the host GUI thread".into());
        }
        if self.started.elapsed() > Duration::from_secs(2) {
            return Err("document context exceeded its two second budget".into());
        }
        let active: Borrowed = unsafe { method(self.manager.0, 9) };
        if unsafe { active(self.manager.0) } != self.document() {
            return Err("active document changed".into());
        }
        check_service_active(self.service)
    }

    fn metadata(&self) -> Result<(Option<String>, Option<String>), String> {
        self.check()?;
        if self.document.is_none() {
            return Ok((None, None));
        }
        let title: unsafe extern "C" fn(*mut c_void) -> *const PlainString =
            unsafe { method(self.document(), 3) };
        let title = unsafe { title(self.document()) };
        if title.is_null() {
            return Err("document title unavailable".into());
        }
        let title_owner =
            hold(unsafe { (*title).object }).ok_or("document title string unavailable")?;
        let title = text(title_owner.0, MAX_TITLE_UNITS)?;
        drop(title_owner);
        self.check()?;
        let path: Borrowed = unsafe { method(self.document(), 4) };
        let path = hold(unsafe { path(self.document()) }).ok_or("document path unavailable")?;
        self.check()?;
        let mut output = NativeText::empty(self.api)?;
        let url: unsafe extern "C" fn(*mut c_void, *mut PlainString, u8) =
            unsafe { method(path.0, 8) };
        unsafe { url(path.0, &mut output.repr, 0) };
        let value = text(output.repr.object, MAX_URL_UNITS)?;
        drop(output);
        drop(path);
        self.check()?;
        Ok((Some(title), (!value.is_empty()).then_some(value)))
    }

    fn check_registry(&self, generation: u64, serial: u64) -> Result<(), String> {
        let registry = unsafe { &*(*self.service).document_context.get() };
        let current = registry
            .identity
            .as_ref()
            .map_or(ptr::null_mut(), |owner| owner.0);
        if !registry.observed
            || registry.serial != serial
            || registry.generation != generation
            || current != self.identity()
        {
            return Err("native document context identity changed".into());
        }
        self.check()
    }
}

fn transition(snapshot: &Snapshot) -> Result<(u64, u64, String), String> {
    snapshot.check()?;
    let service = unsafe { &*snapshot.service };
    let (changed, generation) = {
        let registry = unsafe { &*service.document_context.get() };
        if registry.serial == 0 {
            return Err("native service serial unavailable".into());
        }
        let changed = !registry.observed
            || registry
                .identity
                .as_ref()
                .map_or(ptr::null_mut(), |owner| owner.0)
                != snapshot.identity();
        let generation = if changed {
            registry
                .generation
                .checked_add(1)
                .ok_or("document generation exhausted")?
        } else {
            registry.generation
        };
        (changed, generation)
    };
    if changed {
        let identity = hold(snapshot.identity());
        snapshot.check()?;
        let retired = {
            let registry = unsafe { &mut *service.document_context.get() };
            registry.generation = generation;
            registry.observed = true;
            std::mem::replace(&mut registry.identity, identity)
        };
        service
            .document_context_pending
            .store(snapshot.identity.is_some(), Ordering::Release);
        drop(retired);
        snapshot.check()?;
    }
    let registry = unsafe { &*service.document_context.get() };
    Ok((registry.generation, registry.serial, registry.token()))
}

pub(crate) struct Receipt {
    _snapshot: Snapshot,
}

pub(in crate::native_script) struct Reply {
    value: Value,
    snapshot: Option<Snapshot>,
    generation: u64,
    serial: u64,
    metadata: (Option<String>, Option<String>),
}

impl Reply {
    pub(super) fn encode(&mut self) -> String {
        self.value.to_string()
    }

    pub(super) fn final_check(&mut self, service: *mut Service) -> bool {
        let result = self
            .snapshot
            .as_ref()
            .ok_or_else(|| "document context receipt unavailable".to_string())
            .and_then(|snapshot| {
                if snapshot.service != service {
                    return Err("native document context service changed".into());
                }
                snapshot.check_registry(self.generation, self.serial)?;
                if snapshot.metadata()? != self.metadata {
                    return Err("document metadata changed before publication".into());
                }
                snapshot.check_registry(self.generation, self.serial)
            });
        match result {
            Ok(()) => true,
            Err(message) => {
                self.value = error(
                    "native_document_context_unavailable",
                    &message,
                    "not_applied",
                );
                false
            }
        }
    }

    pub(super) fn take_receipt(&mut self) -> Option<Receipt> {
        self.snapshot.take().map(|snapshot| Receipt {
            _snapshot: snapshot,
        })
    }

    pub(super) fn release_context(&mut self) {
        drop(self.snapshot.take());
    }
}

fn observe(service: *mut Service, manager: Owned, api: &'static Api) -> Result<Reply, String> {
    let snapshot = Snapshot::capture(service, manager, api)?;
    release_receipt(service);
    snapshot.check()?;
    let metadata = snapshot.metadata()?;
    let (generation, serial, token) = transition(&snapshot)?;
    snapshot.check_registry(generation, serial)?;
    Ok(Reply {
        value: json!({"ok":true,"api":"fender-mcp-native-v1","context_api":API_NAME,
            "token":token,"open":snapshot.document.is_some(),"title":metadata.0,"fileUrl":metadata.1}),
        snapshot: Some(snapshot),
        generation,
        serial,
        metadata,
    })
}

pub(in crate::native_script) fn dispatch(service: *mut Service, api: &'static Api) -> NativeReply {
    let result = check_service_active(service)
        .and_then(|_| manager(api))
        .and_then(|manager| observe(service, manager, api));
    match result {
        Ok(reply) => NativeReply::DocumentContext(reply),
        Err(message) => NativeReply::error(
            "native_document_context_unavailable",
            &message,
            "not_applied",
        ),
    }
}

#[cfg(test)]
#[path = "document_context_tests.rs"]
mod tests;

//! Finite JSON requests. A native object is supplied by the guarded DAW bridge.
use super::*;
use fender_native_dsp_api::{DspParameters, TextApi as DspText};
use fender_native_parameter_api::{self as parameters, RawParameter};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    ffi::CString,
    ptr::NonNull,
    time::{Duration, Instant},
};

pub(super) const MAX_REQUEST_BYTES: usize = 16 * 1024;
pub(super) const MAX_RESPONSE_BYTES: usize = 256 * 1024;
const MAX_COUNT: u32 = 1_000_000;
const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;
const ZERO_UID: &str = "00000000-0000-0000-0000-000000000000";
const DOC_MANAGER: Uid = Uid {
    a: 0x1e03d0d4,
    b: 0x9fc9,
    c: 0x41f7,
    d: [0xb9, 0x3e, 0x66, 0x74, 0x1a, 0x3f, 0x1b, 0x3d],
};

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Source {
    #[default]
    Auto,
    Dsp,
    Controller,
}
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Mode {
    Raw,
    Normalized,
    Text,
}
fn default_limit() -> u32 {
    64
}
#[derive(Debug, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
enum Request {
    Inspect {
        #[serde(default)]
        source: Source,
        expected_instance: Option<String>,
    },
    Parameters {
        #[serde(default)]
        offset: u32,
        #[serde(default = "default_limit")]
        limit: u32,
        #[serde(default)]
        source: Source,
        expected_instance: Option<String>,
    },
    Choices {
        param: String,
        #[serde(default)]
        offset: u32,
        #[serde(default = "default_limit")]
        limit: u32,
        #[serde(default)]
        source: Source,
        expected_instance: Option<String>,
    },
    Parameter {
        param: String,
        #[serde(default)]
        source: Source,
        expected_instance: Option<String>,
    },
    SetParameter {
        param: String,
        mode: Mode,
        value: Value,
        #[serde(default)]
        source: Source,
        expected_instance: Option<String>,
    },
}
impl Request {
    fn source(&self) -> Source {
        match self {
            Self::Inspect { source, .. }
            | Self::Parameters { source, .. }
            | Self::Choices { source, .. }
            | Self::Parameter { source, .. }
            | Self::SetParameter { source, .. } => *source,
        }
    }
    fn expected(&self) -> Option<&str> {
        match self {
            Self::Inspect {
                expected_instance, ..
            }
            | Self::Parameters {
                expected_instance, ..
            }
            | Self::Choices {
                expected_instance, ..
            }
            | Self::Parameter {
                expected_instance, ..
            }
            | Self::SetParameter {
                expected_instance, ..
            } => expected_instance.as_deref(),
        }
    }
    fn validate(&self) -> Result<(), String> {
        if let Some(uid) = self.expected() {
            if !valid_uid(uid) {
                return Err("expected_instance must be a nonzero canonical object UID".into());
            }
        }
        match self {
            Self::Parameters { limit, .. } | Self::Choices { limit, .. }
                if !(1..=128).contains(limit) =>
            {
                Err("limit must be between 1 and 128".into())
            }
            Self::Parameter { param, .. }
            | Self::Choices { param, .. }
            | Self::SetParameter { param, .. }
                if param.is_empty() || param.len() > 512 || param.contains('\0') =>
            {
                Err("param must contain 1..512 UTF-8 bytes without NUL".into())
            }
            Self::SetParameter { mode, value, .. } => match (mode, value) {
                (Mode::Raw | Mode::Normalized, Value::Number(n))
                    if n.as_f64()
                        .is_some_and(|n| n.is_finite() && n.abs() <= MAX_SAFE_INTEGER as f64) =>
                {
                    Ok(())
                }
                (Mode::Text, Value::String(s))
                    if s.encode_utf16().count() <= 4096 && !s.contains('\0') =>
                {
                    Ok(())
                }
                _ => Err(
                    "value does not match mode or exceeds the supported finite/text bounds".into(),
                ),
            },
            _ => Ok(()),
        }
    }
}
fn valid_uid(s: &str) -> bool {
    s.len() == 36
        && s != ZERO_UID
        && s.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit() && !b.is_ascii_lowercase()
            }
        })
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct Instance {
    object_uid: String,
    class_uid: String,
    native_class: String,
    object_id: String,
}

pub(super) fn copy_string(object: *mut c_void, cap: usize) -> Result<String, String> {
    if object.is_null() {
        return Err("missing native string".into());
    }
    let length: unsafe extern "C" fn(*mut c_void) -> i32 = unsafe { method(object, 4) };
    let before = unsafe { length(object) };
    if before < 0 || before as usize > cap {
        return Err("native string exceeds the allowed length".into());
    }
    let mut units = vec![0xffff; before as usize + 1];
    let copy: unsafe extern "C" fn(*mut c_void, *mut u16, i32) -> i32 =
        unsafe { method(object, 8) };
    if unsafe { copy(object, units.as_mut_ptr(), units.len() as i32) } != OK {
        return Err("native string copy failed".into());
    }
    if unsafe { length(object) } != before || units[before as usize] != 0 {
        return Err("native string changed or was not terminated".into());
    }
    String::from_utf16(&units[..before as usize]).map_err(|_| "invalid native UTF-16".into())
}
fn instance(object: *mut c_void) -> Result<Instance, String> {
    let node = query(object, &NODE).ok_or("native target has no node identity")?;
    let get_uid: unsafe extern "C" fn(*mut c_void) -> *const Uid = unsafe { method(node.0, 4) };
    let get_class: unsafe extern "C" fn(*mut c_void) -> *const Uid = unsafe { method(node.0, 5) };
    let get_id: unsafe extern "C" fn(*mut c_void) -> *const PlainString =
        unsafe { method(node.0, 3) };
    let object_uid = uid(unsafe { get_uid(node.0) });
    let class_uid = uid(unsafe { get_class(node.0) });
    if !valid_uid(&object_uid) || !valid_uid(&class_uid) {
        return Err("native target has no stable object/class UID".into());
    }
    let id = unsafe { get_id(node.0) };
    if id.is_null() {
        return Err("native target identifier is unavailable".into());
    }
    let object_id = copy_string(unsafe { (*id).object }, 512)?;
    Ok(Instance {
        object_uid,
        class_uid,
        native_class: class_name(object),
        object_id,
    })
}
struct Document {
    manager: Owned,
    document: Owned,
    _identity: Owned,
    model: Owned,
    _model_identity: Owned,
}
fn active_document(a: &Api) -> Result<Document, String> {
    let table = unsafe { (a.objects)() };
    let key = unsafe { (a.cstring)(c"hostapp".as_ptr()) };
    if table.is_null() || key.is_null() {
        return Err("native host table unavailable".into());
    }
    let get: unsafe extern "C" fn(*mut c_void, *const PlainCString) -> *mut c_void =
        unsafe { method(table, 9) };
    let root = hold(unsafe { get(table, key) }).ok_or("hostapp unavailable")?;
    let node = query(root.0, &NODE).ok_or("hostapp node unavailable")?;
    let key = unsafe {
        constant_string().ok_or("native text API unavailable")?(c"DocumentManager".as_ptr())
    };
    if key.is_null() {
        return Err("document manager identifier unavailable".into());
    }
    let find: unsafe extern "C" fn(*mut c_void, *const PlainString) -> *mut c_void =
        unsafe { method(node.0, 10) };
    let manager = hold(unsafe { find(node.0, key) }).ok_or("document manager unavailable")?;
    let manager = query(manager.0, &DOC_MANAGER).ok_or("document manager interface unavailable")?;
    let active: Borrowed = unsafe { method(manager.0, 9) };
    let document = hold(unsafe { active(manager.0) }).ok_or("no active document")?;
    let identity = query(document.0, &UNKNOWN).ok_or("document identity unavailable")?;
    let get_model: Borrowed = unsafe { method(document.0, 6) };
    let model = hold(unsafe { get_model(document.0) }).ok_or("document model unavailable")?;
    let model_identity = query(model.0, &UNKNOWN).ok_or("document model identity unavailable")?;
    Ok(Document {
        manager,
        document,
        _identity: identity,
        model,
        _model_identity: model_identity,
    })
}
pub(super) struct Context<'a> {
    service: *mut Service,
    target: Owned,
    node: Owned,
    instance: Instance,
    document: Document,
    started: Instant,
    api: &'a Api,
}
impl<'a> Context<'a> {
    fn capture(
        service: *mut Service,
        target: Owned,
        expected: Option<&str>,
        a: &'a Api,
    ) -> Result<Self, String> {
        let document = active_document(a)?;
        let found = instance(target.0)?;
        let node = query(target.0, &NODE).ok_or("native target node unavailable")?;
        if expected.is_some_and(|s| s != found.object_uid) {
            return Err("expected_instance does not match resolved native target".into());
        }
        let ctx = Self {
            service,
            target,
            node,
            instance: found,
            document,
            started: Instant::now(),
            api: a,
        };
        ctx.check()?;
        Ok(ctx)
    }
    fn service_active(&self) -> Result<(), String> {
        let service = unsafe { &*self.service };
        if !READY.load(Ordering::Acquire)
            || service.terminated.load(Ordering::Acquire)
            || !service.initialized.load(Ordering::Acquire)
            || !service.registered.load(Ordering::Acquire)
        {
            Err("native service is no longer active".into())
        } else {
            Ok(())
        }
    }
    fn check(&self) -> Result<(), String> {
        self.service_active()?;
        if !on_main(self.api) {
            return Err("request left the host GUI thread".into());
        }
        if self.started.elapsed() > Duration::from_secs(2) {
            return Err("native request exceeded its two second budget".into());
        }
        // All interfaces are already pinned. These final reads create no
        // temporary native owner whose release could reenter after validation.
        let active: Borrowed = unsafe { method(self.document.manager.0, 9) };
        let model: Borrowed = unsafe { method(self.document.document.0, 6) };
        let uid_at: unsafe extern "C" fn(*mut c_void) -> *const Uid =
            unsafe { method(self.node.0, 4) };
        let class_at: unsafe extern "C" fn(*mut c_void) -> *const Uid =
            unsafe { method(self.node.0, 5) };
        if unsafe { active(self.document.manager.0) } != self.document.document.0
            || unsafe { model(self.document.document.0) } != self.document.model.0
        {
            return Err("active document/model changed".into());
        }
        if uid(unsafe { uid_at(self.node.0) }) != self.instance.object_uid
            || uid(unsafe { class_at(self.node.0) }) != self.instance.class_uid
        {
            return Err("native target identity changed".into());
        }
        if unsafe { active(self.document.manager.0) } != self.document.document.0
            || unsafe { model(self.document.document.0) } != self.document.model.0
        {
            return Err("active document/model changed".into());
        }
        self.service_active()
    }
}
fn raw_text(a: &Api) -> parameters::TextApi {
    parameters::TextApi {
        get_empty_string: a.empty,
        get_constant_cstring: unsafe {
            std::mem::transmute::<CStringLookup, unsafe extern "C" fn(*const c_char) -> *const c_void>(
                a.cstring,
            )
        },
    }
}
fn dsp(ctx: &Context<'_>) -> Result<DspParameters, String> {
    ctx.check()?;
    let target = hold(ctx.target.0).ok_or("target unavailable")?;
    let text = DspText {
        get_constant_string: unsafe {
            std::mem::transmute::<
                ConstantString,
                unsafe extern "C" fn(*const c_char) -> *const c_void,
            >(constant_string().ok_or("text API unavailable")?)
        },
        get_constant_cstring: unsafe {
            std::mem::transmute::<CStringLookup, unsafe extern "C" fn(*const c_char) -> *const c_void>(
                ctx.api.cstring,
            )
        },
    };
    let ptr = NonNull::new(target.0).expect("held pointer");
    std::mem::forget(target);
    unsafe { DspParameters::from_owned_target(ptr, text) }.map_err(|e| e.to_string())
}
struct Controller {
    handle: Owned,
    lease: Option<Box<fender_native_editor_api::ExternalEditorLease>>,
}
impl Controller {
    fn count(&self) -> Result<u32, String> {
        let get: Count = unsafe { method(self.handle.0, 3) };
        let count = unsafe { get(self.handle.0) };
        if !(0..=MAX_COUNT as i32).contains(&count) {
            Err("invalid native controller count".into())
        } else {
            Ok(count as u32)
        }
    }
    fn at(&self, index: u32, a: &Api) -> Result<RawParameter, String> {
        let get: unsafe extern "C" fn(*mut c_void, i32) -> *mut c_void =
            unsafe { method(self.handle.0, 4) };
        self.parameter(unsafe { get(self.handle.0, index as i32) }, a)
    }
    fn find(&self, name: &str, a: &Api) -> Result<RawParameter, String> {
        let text = CString::new(name).map_err(|_| "parameter contains NUL")?;
        // Borrowed CString representation, valid for this synchronous call;
        // dynamic parameter names are never interned in a process-wide cache.
        let name = PlainCString {
            text: text.as_ptr(),
            owner: ptr::null_mut(),
        };
        let get: unsafe extern "C" fn(*mut c_void, *const PlainCString) -> *mut c_void =
            unsafe { method(self.handle.0, 5) };
        self.parameter(unsafe { get(self.handle.0, &name) }, a)
    }
    fn parameter(&self, p: *mut c_void, a: &Api) -> Result<RawParameter, String> {
        let owned = hold(p).ok_or("native controller parameter unavailable")?;
        let raw = NonNull::new(owned.0).expect("held pointer");
        std::mem::forget(owned);
        Ok(unsafe { RawParameter::from_owned(raw, raw_text(a)) })
    }
    fn check(&self) -> Result<(), String> {
        self.lease
            .as_ref()
            .map_or(Ok(()), |lease| lease.check().map_err(|e| e.to_string()))
    }
    fn coverage(&self) -> &'static str {
        if self.lease.is_some() {
            "exact_device_external_editor_controller"
        } else {
            "resolved_controller_provider"
        }
    }
}
fn controller(ctx: &Context<'_>) -> Result<Controller, String> {
    ctx.check()?;
    let mut empty = None;
    if let Some(handle) = query(ctx.target.0, &CONTROLLER) {
        let c = Controller {
            handle,
            lease: None,
        };
        if c.count()? > 0 {
            return Ok(c);
        }
        empty = Some(c);
    }
    if let Some(alias) = query(ctx.target.0, &ALIAS) {
        let get: Borrowed = unsafe { method(alias.0, 3) };
        if let Some(plugin) = hold(unsafe { get(alias.0) }) {
            if let Some(handle) = query(plugin.0, &CONTROLLER) {
                let c = Controller {
                    handle,
                    lease: None,
                };
                if c.count()? > 0 {
                    return Ok(c);
                }
                if empty.is_none() {
                    empty = Some(c);
                }
            }
        }
    }
    let target = hold(ctx.target.0).ok_or("native target unavailable")?;
    let pointer = NonNull::new(target.0).expect("held pointer");
    std::mem::forget(target);
    let gui = fender_native_editor_api::GuiApi {
        get_desktop: ctx.api.desktop,
        get_constant_string: unsafe {
            std::mem::transmute::<
                ConstantString,
                unsafe extern "C" fn(*const c_char) -> *const c_void,
            >(constant_string().ok_or("text API unavailable")?)
        },
    };
    match unsafe { fender_native_editor_api::resolve_external_owned(pointer, gui) } {
        Ok(lease) => {
            ctx.check()?;
            lease.check().map_err(|e| e.to_string())?;
            let handle =
                hold(lease.controller().as_ptr()).ok_or("external controller unavailable")?;
            Ok(Controller {
                handle,
                lease: Some(Box::new(lease)),
            })
        }
        Err(fender_native_editor_api::Error::UnsupportedDevice) => {
            empty.ok_or("native target has no controller provider".into())
        }
        Err(e) => Err(format!(
            "exact-device External Effect editor unavailable: {e}"
        )),
    }
}
enum Provider {
    Dsp(DspParameters),
    Controller(Controller),
}
impl Provider {
    fn source(&self) -> &'static str {
        match self {
            Self::Dsp(_) => "dsp",
            Self::Controller(_) => "controller",
        }
    }
}
fn provider(ctx: &Context<'_>, source: Source, scalar: bool) -> Result<Provider, String> {
    if scalar {
        return if source == Source::Dsp {
            Err("DSP interface supplies metadata only; native setters require a controller".into())
        } else {
            controller(ctx).map(Provider::Controller)
        };
    }
    match source {
        Source::Dsp => dsp(ctx).map(Provider::Dsp),
        Source::Controller => controller(ctx).map(Provider::Controller),
        Source::Auto => {
            let d = dsp(ctx);
            if let Ok(ref p) = d {
                if p.count().map_err(|e| e.to_string())? > 0 {
                    return Ok(Provider::Dsp(d.expect("checked")));
                }
            }
            match controller(ctx) {
                Ok(c) if c.count()? > 0 => Ok(Provider::Controller(c)),
                Ok(c) => d.map(Provider::Dsp).or(Ok(Provider::Controller(c))),
                Err(e) => d.map(Provider::Dsp).map_err(|_| e),
            }
        }
    }
}
fn scalar(value: Option<&parameters::NativeValue>) -> Value {
    match value.map(|v| &v.value) {
        Some(parameters::Value::Integer(v)) if v.abs_diff(0) <= MAX_SAFE_INTEGER as u64 => json!(v),
        Some(parameters::Value::Float(v)) => json!(v),
        Some(parameters::Value::Text(v)) => json!(v),
        _ => Value::Null,
    }
}
fn typed(value: Option<&parameters::NativeValue>) -> Value {
    match value {
        None => Value::Null,
        Some(v) => {
            let data = match &v.value {
                parameters::Value::Empty => json!({"kind":"empty"}),
                parameters::Value::Integer(n) if n.abs_diff(0) <= MAX_SAFE_INTEGER as u64 => {
                    json!({"kind":"integer","value":n})
                }
                parameters::Value::Integer(n) => {
                    json!({"kind":"integer","exact_decimal":n.to_string()})
                }
                parameters::Value::Float(n) => json!({"kind":"float","value":n}),
                parameters::Value::Text(s) => json!({"kind":"text","value":s}),
                parameters::Value::NonFiniteFloat { bits } => {
                    json!({"kind":"non_finite_float","bits_hex":format!("{bits:016X}")})
                }
                parameters::Value::OpaqueObject { present } => {
                    json!({"kind":"opaque_object","present":present})
                }
                parameters::Value::Unsupported { kind } => json!({"kind":"unsupported","tag":kind}),
            };
            json!({"variant_tag":v.variant_tag,"data":data})
        }
    }
}
fn safe_integer_metadata(d: &parameters::Description) -> bool {
    [d.raw.as_ref(),d.min.as_ref(),d.max.as_ref(),d.default_value.as_ref()].into_iter().flatten().all(|v| !matches!(v.value,parameters::Value::Integer(n) if n.abs_diff(0)>MAX_SAFE_INTEGER as u64))
}
fn description(d: &parameters::Description) -> Value {
    let safe_integers = safe_integer_metadata(d);
    let supported =
        safe_integers && (d.support.raw_number || d.support.normalized || d.support.display_text);
    let opaque = matches!(
        d.raw.as_ref().map(|v| &v.value),
        Some(
            parameters::Value::OpaqueObject { .. }
                | parameters::Value::Unsupported { .. }
                | parameters::Value::NonFiniteFloat { .. }
        )
    );
    json!({"name":d.name,"title":d.name,"tag":d.tag,"value":scalar(d.raw.as_ref()),"text":d.display,"min":scalar(d.min.as_ref()),"max":scalar(d.max.as_ref()),"default":scalar(d.default_value.as_ref()),"normalized":match d.normalized {parameters::Normalized::Finite(v)=>Some(v),_=>None},"enabled":d.enabled,"readOnly":d.read_only,"writable":supported&&!opaque,"native_kind":format!("{:?}",d.kind),"native_value":typed(d.raw.as_ref()),"support":{"raw":safe_integers&&d.support.raw_number,"normalized":safe_integers&&d.support.normalized,"text":safe_integers&&d.support.display_text},"display_parser":format!("{:?}",d.display_parser)})
}
fn page(ctx: &Context<'_>, p: &Provider, offset: u32, limit: u32) -> Result<Value, String> {
    ctx.check()?;
    match p {
        Provider::Dsp(p) => {
            let result = p.page(offset, limit).map_err(|e| e.to_string())?;
            ctx.check()?;
            Ok(
                json!({"parameters":result.records.iter().map(|r|json!({"index":r.index,"tag":r.tag,"name":r.name,"title":r.title,"folder":r.folder,"raw_flags":r.raw_flags})).collect::<Vec<_>>(),"reported_count":result.reported_count,"offset":offset,"next_offset":result.next,"page_complete":result.failed_indices.is_empty(),"all_indices_read":result.complete,"failed_indices":result.failed_indices.iter().map(|r|json!({"index":r.index,"reason":format!("{:?}",r.reason)})).collect::<Vec<_>>(),"coverage":"direct_device_dsp_provider"}),
            )
        }
        Provider::Controller(c) => {
            c.check()?;
            let count = c.count()?;
            if offset > count {
                return Err("offset exceeds reported controller count".into());
            }
            let end = offset.saturating_add(limit).min(count);
            let mut records = Vec::new();
            let mut failed = Vec::new();
            for index in offset..end {
                ctx.check()?;
                c.check()?;
                match c
                    .at(index, ctx.api)
                    .and_then(|p| p.describe().map_err(|e| e.to_string()))
                {
                    Ok(d) => {
                        let mut d = description(&d);
                        d["index"] = json!(index);
                        records.push(d)
                    }
                    Err(e) => failed.push(json!({"index":index,"reason":e})),
                }
            }
            ctx.check()?;
            c.check()?;
            if c.count()? != count {
                return Err("controller parameter count changed".into());
            }
            Ok(
                json!({"parameters":records,"reported_count":count,"offset":offset,"next_offset":if end<count{Some(end)}else{None},"page_complete":failed.is_empty(),"all_indices_read":offset==0&&end==count&&failed.is_empty(),"failed_indices":failed,"coverage":c.coverage()}),
            )
        }
    }
}
fn set_parameter(
    ctx: &Context<'_>,
    controller: &Controller,
    name: &str,
    mode: Mode,
    value: Value,
) -> Value {
    let resolved = ctx
        .check()
        .and_then(|_| controller.check())
        .and_then(|_| controller.find(name, ctx.api));
    let parameter = match resolved {
        Ok(p) => p,
        Err(e) => return error("parameter_unavailable", &e, "not_applied"),
    };
    let mut guard = || {
        ctx.check()?;
        let current = parameter.describe().map_err(|e| e.to_string())?;
        if !safe_integer_metadata(&current) {
            return Err(
                "native integer state/range exceeds exact JavaScript JSON transport".into(),
            );
        }
        controller.check()?;
        // Last foreign operations are pinned borrowed identity reads. No native
        // owner is released between this guard return and the helper setter.
        ctx.check()
    };
    let result = match mode {
        Mode::Raw => {
            let number = value.as_number().expect("validated number");
            let number = match number.as_i64() {
                Some(integer) => parameters::Number::Integer(integer),
                None => {
                    parameters::Number::Float(number.as_f64().expect("validated finite number"))
                }
            };
            parameter.set_raw_number(number, &mut guard)
        }
        Mode::Normalized => parameter.set_normalized(
            value.as_f64().expect("validated finite number") as f32,
            &mut guard,
        ),
        Mode::Text => {
            parameter.set_display_text(value.as_str().expect("validated string"), &mut guard)
        }
    };
    match result {
        Ok(r) => {
            json!({"ok":true,"before":description(&r.before),"after":description(&r.after),"verified":r.verified,"outcome":"observed"})
        }
        Err(e) => {
            let outcome = match e.outcome {
                parameters::Outcome::NotApplied => "not_applied",
                parameters::Outcome::Unknown => "unknown",
            };
            let mut value = error("native_parameter_error", &e.error.to_string(), outcome);
            if let Some(d) = e.before {
                value["before"] = description(&d);
            }
            if let Some(d) = e.after {
                value["after"] = description(&d);
            }
            value
        }
    }
}
fn choices(
    ctx: &Context<'_>,
    controller: &Controller,
    name: &str,
    offset: u32,
    limit: u32,
) -> Result<Value, String> {
    ctx.check()?;
    controller.check()?;
    let parameter = controller.find(name, ctx.api)?;
    let mut guard = || {
        controller.check()?;
        ctx.check()
    };
    let page = parameter
        .list_choices(offset, limit, &mut guard)
        .map_err(|e| e.to_string())?;
    drop(parameter);
    controller.check()?;
    ctx.check()?;
    Ok(
        json!({"param":name,"choices":page.choices.iter().map(|choice|json!({"index":choice.index,"text":choice.label})).collect::<Vec<_>>(),"reported_count":page.count,"offset":page.offset,"next_offset":page.next,"page_complete":true,"all_indices_read":page.offset==0&&page.next.is_none(),"coverage":controller.coverage()}),
    )
}
fn execute(ctx: &Context<'_>, request: Request, provider: &Provider) -> Value {
    let result: Result<Value, String> = match request {
        Request::Inspect { .. } => match provider {
            Provider::Dsp(p) => p.count().map(|count| json!({"reported_count":count,"coverage":"direct_device_dsp_provider","native_setter":false})).map_err(|e| e.to_string()),
            Provider::Controller(c) => c.count().map(|count| json!({"reported_count":count,"coverage":c.coverage(),"native_setter":true})),
        },
        Request::Parameters { offset, limit, .. } => page(ctx, provider, offset, limit),
        Request::Choices {param,offset,limit,..}=>match provider {
            Provider::Controller(controller)=>choices(ctx,controller,&param,offset,limit),
            _=>Err("DSP metadata does not expose native list labels".into()),
        },
        Request::Parameter { param, .. } => match provider {
            Provider::Controller(c) => ctx.check().and_then(|_| c.check()).and_then(|_| c.find(&param, ctx.api)).and_then(|parameter| {
                ctx.check()?;
                parameter.describe().map_err(|e| e.to_string())
            }).map(|d| json!({"parameter":description(&d)})),
            _ => Err("DSP metadata has no value getter".into()),
        },
        Request::SetParameter { param, mode, value, .. } => return match provider {
            Provider::Controller(c) => set_parameter(ctx, c, &param, mode, value),
            _ => error("unsupported_source", "DSP metadata has no native setter", "not_applied"),
        },
    };
    match result.and_then(|value| {
        if let Provider::Controller(c) = provider {
            c.check()?;
        }
        ctx.check()?;
        Ok(value)
    }) {
        Ok(mut value) => {
            value["ok"] = json!(true);
            value
        }
        Err(e) => error("native_query_failed", &e, "not_applied"),
    }
}
fn error(code: &str, message: &str, outcome: &str) -> Value {
    json!({"ok":false,"error":{"code":code,"message":message},"outcome":outcome})
}
pub(crate) struct ReceiptLease {
    // Provider drops first while document/target pins remain valid.
    _provider: Option<Provider>,
    _context: Box<Context<'static>>,
}
pub(super) struct Reply {
    value: Value,
    provider: Option<Provider>,
    context: Option<Box<Context<'static>>>,
    mutation: bool,
}
impl Reply {
    pub(super) fn unbound(value: Value) -> Self {
        Self {
            value,
            provider: None,
            context: None,
            mutation: false,
        }
    }
    pub(super) fn error(code: &str, message: &str, outcome: &str) -> Self {
        Self::unbound(error(code, message, outcome))
    }
    fn invalidate(&mut self, message: String) {
        let outcome = if self.mutation && self.value["outcome"] != "not_applied" {
            "unknown"
        } else {
            "not_applied"
        };
        self.value["ok"] = json!(false);
        self.value["error"] = json!({"code":"context_changed_before_receipt","message":message});
        self.value["outcome"] = json!(outcome);
        self.value
            .as_object_mut()
            .expect("object response")
            .remove("verified");
    }
    pub(super) fn final_check(&mut self, service: *mut Service) -> bool {
        let result = match self.context.as_ref() {
            Some(ctx) => self
                .provider
                .as_ref()
                .map_or(Ok(()), |provider| match provider {
                    Provider::Controller(c) => c.check(),
                    _ => Ok(()),
                })
                .and_then(|_| ctx.check()),
            None => {
                let service = unsafe { &*service };
                if READY.load(Ordering::Acquire)
                    && !service.terminated.load(Ordering::Acquire)
                    && service.initialized.load(Ordering::Acquire)
                    && service.registered.load(Ordering::Acquire)
                {
                    Ok(())
                } else {
                    Err("native service is no longer active".into())
                }
            }
        };
        match result {
            Ok(()) => true,
            Err(e) => {
                self.invalidate(e);
                false
            }
        }
    }
    pub(super) fn take_receipt(&mut self) -> Option<ReceiptLease> {
        self.context.take().map(|context| ReceiptLease {
            _provider: self.provider.take(),
            _context: context,
        })
    }
    pub(super) fn release_context(&mut self) {
        drop(self.provider.take());
        drop(self.context.take());
    }
    pub(super) fn encode(&mut self) -> String {
        let encoded = self.value.to_string();
        if encoded.len() <= MAX_RESPONSE_BYTES {
            return encoded;
        }
        let mut value = error(
            "response_too_large",
            "native response exceeded256KiB",
            if self.mutation && self.value["outcome"] != "not_applied" {
                "unknown"
            } else {
                "not_applied"
            },
        );
        if self.value.get("instance").is_some() {
            value["instance"] = self.value["instance"].clone();
        }
        if self.value.get("source").is_some() {
            value["source"] = self.value["source"].clone();
        }
        self.value = value;
        self.value.to_string()
    }
}
fn release_previous(ctx: &Context<'_>) -> Result<(), String> {
    // Capture this request's document/model/Device pins before releasing the
    // old receipt. A destructor may switch documents or terminate the service.
    release_receipt(ctx.service);
    ctx.check()
}
pub(super) fn dispatch(service: *mut Service, target: Owned, text: &str, a: &'static Api) -> Reply {
    let request: Request = match serde_json::from_str(text) {
        Ok(r) => r,
        Err(_) => {
            return Reply::error(
                "invalid_request",
                "unsupported operation, field or JSON shape",
                "not_applied",
            );
        }
    };
    if let Err(e) = request.validate() {
        return Reply::error("invalid_request", &e, "not_applied");
    }
    let ctx = match Context::capture(service, target, request.expected(), a) {
        Ok(ctx) => Box::new(ctx),
        Err(e) => return Reply::error("context_guard_failed", &e, "not_applied"),
    };
    let source = request.source();
    let mutation = matches!(request, Request::SetParameter { .. });
    let scalar = matches!(
        request,
        Request::Parameter { .. } | Request::Choices { .. } | Request::SetParameter { .. }
    );
    let (mut value, provider) =
        match release_previous(&ctx).and_then(|_| provider(&ctx, source, scalar)) {
            Ok(provider) => {
                let mut value = execute(&ctx, request, &provider);
                value["source"] = json!(provider.source());
                (value, Some(provider))
            }
            Err(e) => {
                let mut value = error("provider_unavailable", &e, "not_applied");
                value["source"] = match source {
                    Source::Dsp => json!("dsp"),
                    Source::Controller => json!("controller"),
                    Source::Auto => Value::Null,
                };
                (value, None)
            }
        };
    value["instance"] = json!(ctx.instance);
    Reply {
        value,
        provider,
        context: Some(ctx),
        mutation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(value: Value) -> Result<Request, String> {
        let r: Request = serde_json::from_value(value).map_err(|e| e.to_string())?;
        r.validate()?;
        Ok(r)
    }
    #[test]
    fn request_schema_is_finite_and_operation_specific() {
        for operation in ["inspect", "parameters"] {
            assert!(parse(json!({"operation":operation})).is_ok());
        }
        for value in [
            json!({"operation":"eval","code":"x"}),
            json!({"operation":"inspect","param":"x"}),
            json!({"operation":"parameters","limit":129}),
            json!({"operation":"parameters","offset":-1}),
            json!({"operation":"parameter","param":"x","source":"alias"}),
            json!({"operation":"set_parameter","param":"x","mode":"raw","value":{}}),
            json!({"operation":"set_parameter","param":"x","mode":"text","value":1}),
            json!({"operation":"parameter","param":""}),
        ] {
            assert!(parse(value).is_err());
        }
        assert!(
            serde_json::from_str::<Request>(
                r#"{"operation":"inspect","source":"auto","source":"dsp"}"#
            )
            .is_err()
        );
        assert!(
            parse(json!({"operation":"parameter","param":"x","expected_instance":ZERO_UID}))
                .is_err()
        );
        assert!(parse(json!({"operation":"parameter","param":"x","expected_instance":"61D64D66-2A46-4BEE-A623-EC5CEB898313"})).is_ok());
    }
    #[test]
    fn choice_requests_are_read_only_finite_and_bounded() {
        let request =
            parse(json!({"operation":"choices","param":"sendPort","source":"controller"})).unwrap();
        assert!(matches!(
            request,
            Request::Choices {
                offset: 0,
                limit: 64,
                ..
            }
        ));
        assert!(parse(json!({"operation":"choices","param":"x","offset":128,"limit":128,"expected_instance":"61D64D66-2A46-4BEE-A623-EC5CEB898313"})).is_ok());
        for request in [
            json!({"operation":"choices"}),
            json!({"operation":"choices","param":""}),
            json!({"operation":"choices","param":"x","limit":0}),
            json!({"operation":"choices","param":"x","limit":129}),
            json!({"operation":"choices","param":"x","offset":-1}),
            json!({"operation":"choices","param":"x","mode":"text"}),
            json!({"operation":"choices","param":"x","value":1}),
            json!({"operation":"choices","param":"x","source":"anything"}),
            json!({"operation":"choices","param":"x\u{0}"}),
        ] {
            assert!(parse(request).is_err());
        }
        assert!(parse(json!({"operation":"choices","param":"한".repeat(171)})).is_err());
        assert!(
            serde_json::from_str::<Request>(
                r#"{"operation":"choices","param":"x","limit":1,"limit":2}"#
            )
            .is_err()
        );
    }
    #[test]
    fn request_text_and_names_are_bounded_in_the_right_units() {
        assert!(parse(json!({"operation":"parameter","param":"a".repeat(512)})).is_ok());
        assert!(parse(json!({"operation":"parameter","param":"한".repeat(171)})).is_err());
        assert!(parse(json!({"operation":"parameter","param":"bad\u{0}name"})).is_err());
        assert!(parse(json!({"operation":"set_parameter","param":"x","mode":"text","value":"🎵".repeat(2048)})).is_ok());
        assert!(parse(json!({"operation":"set_parameter","param":"x","mode":"text","value":"🎵".repeat(2049)})).is_err());
        let r = parse(json!({"operation":"parameters","limit":128,"offset":199,"source":"dsp"}))
            .unwrap();
        assert_eq!(r.source(), Source::Dsp);
    }
    #[test]
    fn unsupported_values_are_explicit_and_never_writable() {
        let d = parameters::Description {
            name: "opaque".into(),
            tag: 1,
            kind: parameters::Kind::Image,
            enabled: true,
            read_only: false,
            raw: Some(parameters::NativeValue {
                variant_tag: 20,
                value: parameters::Value::OpaqueObject { present: true },
            }),
            min: None,
            max: None,
            default_value: None,
            normalized: parameters::Normalized::NotApplicable,
            display: "".into(),
            display_parser: parameters::DisplayParser::Unsupported,
            support: parameters::Support {
                raw_number: false,
                normalized: false,
                display_text: false,
            },
        };
        let v = description(&d);
        assert_eq!(v["value"], Value::Null);
        assert_eq!(v["writable"], false);
        assert_eq!(v["native_value"]["data"]["kind"], "opaque_object");
    }
    #[test]
    fn json_integer_transport_refuses_rounded_inputs_and_preserves_exact_outputs() {
        for n in [
            json!(9_007_199_254_740_992_i64),
            json!(-9_007_199_254_740_992_i64),
            json!(u64::MAX),
            json!(1e100),
        ] {
            assert!(
                parse(json!({"operation":"set_parameter","param":"x","mode":"raw","value":n}))
                    .is_err()
            );
        }
        assert!(parse(json!({"operation":"set_parameter","param":"x","mode":"raw","value":MAX_SAFE_INTEGER})).is_ok());
        let exact = parameters::NativeValue {
            variant_tag: 1,
            value: parameters::Value::Integer(9_007_199_254_740_993),
        };
        assert_eq!(scalar(Some(&exact)), Value::Null);
        assert_eq!(
            typed(Some(&exact))["data"]["exact_decimal"],
            "9007199254740993"
        );
        let nan = parameters::NativeValue {
            variant_tag: 2,
            value: parameters::Value::NonFiniteFloat {
                bits: 0x7ff8_0000_0000_0001,
            },
        };
        assert_eq!(typed(Some(&nan))["data"]["bits_hex"], "7FF8000000000001");
        let d = parameters::Description {
            name: "integer".into(),
            tag: 1,
            kind: parameters::Kind::Integer,
            enabled: true,
            read_only: false,
            raw: Some(exact),
            min: None,
            max: None,
            default_value: None,
            normalized: parameters::Normalized::Finite(0.5),
            display: "9007199254740993".into(),
            display_parser: parameters::DisplayParser::PlainNumeric,
            support: parameters::Support {
                raw_number: true,
                normalized: true,
                display_text: true,
            },
        };
        let v = description(&d);
        assert_eq!(v["writable"], false);
        for key in ["raw", "normalized", "text"] {
            assert_eq!(v["support"][key], false);
        }
        let mut d = d;
        d.raw = None;
        d.max = Some(parameters::NativeValue {
            variant_tag: 1,
            value: parameters::Value::Integer(i64::MAX),
        });
        assert!(!safe_integer_metadata(&d));
    }
}

#[cfg(test)]
mod guard_tests {
    use super::*;
    #[repr(C)]
    struct Mock {
        table: *const *const c_void,
        refs: AtomicU32,
        active: *mut c_void,
        model: *mut c_void,
        uid: Uid,
        class: Uid,
    }
    unsafe extern "C" fn retain(p: *mut c_void) -> u32 {
        unsafe { &*p.cast::<Mock>() }
            .refs
            .fetch_add(1, Ordering::Relaxed)
            + 1
    }
    static RELEASE_HOOK_TARGET: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
    static SWITCH_MANAGER: AtomicPtr<Mock> = AtomicPtr::new(ptr::null_mut());
    unsafe extern "C" fn release(p: *mut c_void) -> u32 {
        let refs = unsafe { &*p.cast::<Mock>() }
            .refs
            .fetch_sub(1, Ordering::Relaxed)
            - 1;
        if RELEASE_HOOK_TARGET
            .compare_exchange(p, ptr::null_mut(), Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            let manager = SWITCH_MANAGER.swap(ptr::null_mut(), Ordering::AcqRel);
            if !manager.is_null() {
                unsafe {
                    (*manager).active = ptr::null_mut();
                }
            }
        }
        refs
    }
    unsafe extern "C" fn active(p: *mut c_void) -> *mut c_void {
        unsafe { (*p.cast::<Mock>()).active }
    }
    unsafe extern "C" fn model(p: *mut c_void) -> *mut c_void {
        unsafe { (*p.cast::<Mock>()).model }
    }
    unsafe extern "C" fn object_uid(p: *mut c_void) -> *const Uid {
        unsafe { ptr::addr_of!((*p.cast::<Mock>()).uid) }
    }
    unsafe extern "C" fn class_uid(p: *mut c_void) -> *const Uid {
        unsafe { ptr::addr_of!((*p.cast::<Mock>()).class) }
    }
    unsafe extern "C" fn empty() -> *mut c_void {
        ptr::null_mut()
    }
    unsafe extern "C" fn thread() -> i64 {
        0
    }
    unsafe extern "C" fn text(_: *const c_char) -> *const PlainCString {
        ptr::null()
    }
    #[test]
    fn pinned_guard_rejects_document_model_target_service_and_budget_changes() {
        let _serial = super::super::tests::TEST_LOCK.lock().unwrap();
        let mut table = [ptr::null::<c_void>(); 10];
        table[1] = retain as *const c_void;
        table[2] = release as *const c_void;
        table[4] = object_uid as *const c_void;
        table[5] = class_uid as *const c_void;
        table[6] = model as *const c_void;
        table[9] = active as *const c_void;
        let make = || Mock {
            table: table.as_ptr(),
            refs: AtomicU32::new(1),
            active: ptr::null_mut(),
            model: ptr::null_mut(),
            uid: SERVICE_CID,
            class: API_CLASS,
        };
        let (mut manager, mut document, mut model, mut device) = (make(), make(), make(), make());
        let manager_ptr = ptr::addr_of_mut!(manager).cast();
        let doc_ptr = ptr::addr_of_mut!(document).cast();
        let model_ptr = ptr::addr_of_mut!(model).cast();
        let dev_ptr = ptr::addr_of_mut!(device).cast();
        manager.active = doc_ptr;
        document.model = model_ptr;
        let service = Service {
            vtable: &SERVICE_VTABLE,
            refs: AtomicU32::new(1),
            initialized: AtomicBool::new(true),
            script: ScriptFacet::new(),
            registered: AtomicBool::new(true),
            terminated: AtomicBool::new(false),
            initializing: AtomicBool::new(false),
            stopping: AtomicBool::new(false),
            module_attached: AtomicBool::new(false),
            calls: AtomicU32::new(0),
            busy: AtomicBool::new(true),
            receipt: std::cell::UnsafeCell::new(None),
            receipt_pending: AtomicBool::new(false),
        };
        let a = Api {
            gui: empty,
            objects: empty,
            main_thread: empty,
            thread_self: thread,
            cstring: text,
            empty,
            desktop: empty,
            scripting: empty,
        };
        READY.store(true, Ordering::Release);
        let mut ctx = Context {
            service: ptr::addr_of!(service).cast_mut(),
            target: hold(dev_ptr).unwrap(),
            node: hold(dev_ptr).unwrap(),
            instance: Instance {
                object_uid: uid(&device.uid),
                class_uid: uid(&device.class),
                native_class: "mock".into(),
                object_id: "mock".into(),
            },
            document: Document {
                manager: hold(manager_ptr).unwrap(),
                document: hold(doc_ptr).unwrap(),
                _identity: hold(doc_ptr).unwrap(),
                model: hold(model_ptr).unwrap(),
                _model_identity: hold(model_ptr).unwrap(),
            },
            started: Instant::now(),
            api: &a,
        };
        assert!(ctx.check().is_ok());
        manager.active = ptr::null_mut();
        assert!(ctx.check().unwrap_err().contains("document/model"));
        manager.active = doc_ptr;
        document.model = ptr::null_mut();
        assert!(ctx.check().unwrap_err().contains("document/model"));
        document.model = model_ptr;
        device.uid = UNKNOWN;
        assert!(ctx.check().unwrap_err().contains("target identity"));
        device.uid = SERVICE_CID;
        device.class = UNKNOWN;
        assert!(ctx.check().unwrap_err().contains("target identity"));
        device.class = API_CLASS;
        service.terminated.store(true, Ordering::Release);
        assert!(ctx.check().unwrap_err().contains("service"));
        service.terminated.store(false, Ordering::Release);
        READY.store(false, Ordering::Release);
        assert!(ctx.check().is_err());
        READY.store(true, Ordering::Release);
        ctx.started = Instant::now() - Duration::from_secs(3);
        assert!(ctx.check().unwrap_err().contains("budget"));
        drop(ctx);
        for value in [&manager, &document, &model, &device] {
            assert_eq!(value.refs.load(Ordering::Acquire), 1);
        }
        READY.store(false, Ordering::Release);
    }
    struct Fixture {
        _table: Box<[*const c_void; 10]>,
        manager: Box<Mock>,
        document: Box<Mock>,
        model: Box<Mock>,
        device: Box<Mock>,
        service: *mut Service,
    }
    impl Fixture {
        fn new() -> Self {
            super::super::tests::install_mock_api();
            READY.store(true, Ordering::Release);
            let mut service = ptr::null_mut();
            assert_eq!(
                unsafe { factory_create(ptr::null_mut(), &SERVICE_CID, &COMPONENT, &mut service) },
                OK
            );
            assert_eq!(unsafe { service_initialize(service, ptr::null_mut()) }, OK);
            let mut table = Box::new([ptr::null::<c_void>(); 10]);
            table[1] = retain as *const c_void;
            table[2] = release as *const c_void;
            table[4] = object_uid as *const c_void;
            table[5] = class_uid as *const c_void;
            table[6] = model as *const c_void;
            table[9] = active as *const c_void;
            let make = || {
                Box::new(Mock {
                    table: table.as_ptr(),
                    refs: AtomicU32::new(1),
                    active: ptr::null_mut(),
                    model: ptr::null_mut(),
                    uid: SERVICE_CID,
                    class: API_CLASS,
                })
            };
            let (mut manager, mut document, mut model, device) = (make(), make(), make(), make());
            manager.active = ptr::addr_of_mut!(*document).cast();
            document.model = ptr::addr_of_mut!(*model).cast();
            Self {
                _table: table,
                manager,
                document,
                model,
                device,
                service: service.cast(),
            }
        }
        fn context(&mut self) -> Box<Context<'static>> {
            let device = ptr::addr_of_mut!(*self.device).cast();
            let document = ptr::addr_of_mut!(*self.document).cast();
            let model = ptr::addr_of_mut!(*self.model).cast();
            let manager = ptr::addr_of_mut!(*self.manager).cast();
            Box::new(Context {
                service: self.service,
                target: hold(device).unwrap(),
                node: hold(device).unwrap(),
                instance: Instance {
                    object_uid: uid(&self.device.uid),
                    class_uid: uid(&self.device.class),
                    native_class: "mock".into(),
                    object_id: "mock".into(),
                },
                document: Document {
                    manager: hold(manager).unwrap(),
                    document: hold(document).unwrap(),
                    _identity: hold(document).unwrap(),
                    model: hold(model).unwrap(),
                    _model_identity: hold(model).unwrap(),
                },
                started: Instant::now(),
                api: api().unwrap(),
            })
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            RELEASE_HOOK_TARGET.store(ptr::null_mut(), Ordering::Release);
            SWITCH_MANAGER.store(ptr::null_mut(), Ordering::Release);
            assert_eq!(unsafe { service_terminate(self.service.cast()) }, OK);
            assert_eq!(unsafe { service_release(self.service.cast()) }, 0);
            for object in [&*self.manager, &*self.document, &*self.model, &*self.device] {
                assert_eq!(object.refs.load(Ordering::Acquire), 1);
            }
            assert_eq!(LIVE_RECEIPTS.load(Ordering::Acquire), 0);
            assert!(module_exit_state());
            READY.store(false, Ordering::Release);
        }
    }
    #[test]
    fn previous_receipt_cleanup_is_checked_after_new_context_capture() {
        let _serial = super::super::tests::TEST_LOCK.lock().unwrap();
        let mut fixture = Fixture::new();
        let old = fixture.context();
        retain_receipt(
            fixture.service,
            ReceiptLease {
                _provider: None,
                _context: old,
            },
        );
        assert_eq!(LIVE_RECEIPTS.load(Ordering::Acquire), 1);
        // Capture the NEW context first; releasing the old device then switches
        // the active document. The production handoff rejects before mutation.
        let current = fixture.context();
        RELEASE_HOOK_TARGET.store(ptr::addr_of_mut!(*fixture.device).cast(), Ordering::Release);
        SWITCH_MANAGER.store(ptr::addr_of_mut!(*fixture.manager), Ordering::Release);
        let mut writes = 0;
        if release_previous(&current).is_ok() {
            writes += 1;
        }
        assert_eq!(writes, 0);
        assert_eq!(LIVE_RECEIPTS.load(Ordering::Acquire), 0);
        assert!(!unsafe { (*fixture.service).receipt_pending.load(Ordering::Acquire) });
        drop(current);
    }
    #[test]
    fn output_assignment_reentry_cannot_publish_a_verified_success() {
        let _serial = super::super::tests::TEST_LOCK.lock().unwrap();
        let mut fixture = Fixture::new();
        let context = fixture.context();
        let reply = Reply {
            value: json!({"ok":true,"verified":true,"outcome":"observed","instance":context.instance,"source":"controller"}),
            provider: None,
            context: Some(context),
            mutation: true,
        };
        super::super::tests::ASSIGN_REENTER_STOP.store(fixture.service, Ordering::Release);
        let mut out = VariantOut {
            kind: 0,
            padding: [0; 6],
            payload: 0,
        };
        assert!(publish_reply(
            &mut out,
            reply,
            api().unwrap(),
            fixture.service
        ));
        let value: Value = serde_json::from_str(&super::super::tests::output_text(&out)).unwrap();
        assert_eq!(value["ok"], false);
        assert_eq!(value["outcome"], "unknown");
        assert!(value.get("verified").is_none());
        assert_eq!(LIVE_RECEIPTS.load(Ordering::Acquire), 0);
        super::super::tests::clear_output(&mut out);
    }
}

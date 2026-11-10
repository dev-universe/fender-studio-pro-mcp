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
#[path = "request/action.rs"]
mod action;
#[path = "request/alias_controller.rs"]
mod alias_controller;
use alias_controller::catalog;
#[path = "request/capabilities.rs"]
mod capabilities;
#[path = "request/details.rs"]
mod details;
#[path = "request/digest.rs"]
mod digest;
#[cfg(test)]
#[path = "request/direct_tests.rs"]
mod direct_tests;
#[path = "request/document_context.rs"]
pub(super) mod document_context;
#[path = "request/folder_context.rs"]
mod folder_context;
#[path = "request/format.rs"]
mod format;
#[path = "request/gui_model.rs"]
mod gui_model;
#[path = "request/gui_parameter.rs"]
mod gui_parameter;
#[path = "request/insert_chain.rs"]
pub(super) mod insert_chain;
#[path = "request/insert_folder.rs"]
mod insert_folder;
#[path = "request/node_inspect.rs"]
mod node_inspect;
#[path = "request/numeric_edit.rs"]
mod numeric_edit;
#[path = "request/owner_inspect.rs"]
mod owner_inspect;
#[path = "request/presets.rs"]
pub(super) mod presets;
#[path = "request/state.rs"]
pub(super) mod state;

pub(super) const MAX_REQUEST_BYTES: usize = 16 * 1024;
pub(super) fn gui_model_api() -> &'static str {
    gui_model::API
}
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
    FloatBits,
}
fn default_limit() -> u32 {
    64
}
fn supplied_folder<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    String::deserialize(deserializer).map(Some)
}
#[derive(Debug, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
enum Request {
    Inspect {
        #[serde(default)]
        source: Source,
        #[serde(default)]
        include_nodes: bool,
        #[serde(default)]
        node_scope: node_inspect::Scope,
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
    ParameterDetails {
        param: String,
        source: Source,
        #[serde(default)]
        include_owner: bool,
        #[serde(default)]
        compare_gui: bool,
        expected_instance: Option<String>,
    },
    ParameterDigest {
        source: Source,
        expected_instance: Option<String>,
    },
    FormatValues {
        param: String,
        source: Source,
        values: Vec<f64>,
        expected_instance: Option<String>,
    },
    ParameterAction {
        action: action::Action,
        expected_instance: String,
    },
    ModelCatalog {
        catalog: catalog::Kind,
        #[serde(default)]
        offset: u32,
        #[serde(default = "catalog::default_limit")]
        limit: u32,
        expected_instance: String,
    },
    EditNumericParameter {
        param: String,
        value: f64,
        expected_instance: String,
    },
    EditChorderAutofill {
        enabled: bool,
        expected_instance: String,
    },
    EditChorderModel {
        change: gui_model::Change,
        expected_instance: String,
    },
    ChorderLearningGet {
        expected_instance: String,
    },
    SetParameter {
        param: String,
        mode: Mode,
        value: Value,
        #[serde(default)]
        source: Source,
        expected_instance: Option<String>,
    },
    StateCapture {
        expected_instance: Option<String>,
    },
    StateRestore {
        token: String,
        expected_instance: Option<String>,
    },
    StateRelease {
        token: String,
        expected_instance: Option<String>,
    },
    Presets {
        #[serde(default)]
        offset: u32,
        #[serde(default = "presets::default_limit")]
        limit: u32,
        catalog_token: Option<String>,
        expected_instance: Option<String>,
    },
    PresetApply {
        catalog_token: String,
        index: u32,
        expected_instance: Option<String>,
    },
    PresetRelease {
        catalog_token: String,
        expected_instance: Option<String>,
    },
    InspectAux {
        expected_instance: Option<String>,
    },
    InspectInsertFolderAncestry {
        expected_instance: Option<String>,
    },
    InspectInsertFolderPresetPreflight {
        expected_instance: Option<String>,
    },
    InsertChainCapture {
        bridge_session: String,
        document_epoch: String,
        #[serde(default, deserialize_with = "supplied_folder")]
        expected_folder: Option<String>,
    },
    InsertChainRestore {
        bridge_session: String,
        document_epoch: String,
        #[serde(default, deserialize_with = "supplied_folder")]
        expected_folder: Option<String>,
        token: String,
    },
    InsertChainRelease {
        bridge_session: String,
        document_epoch: String,
        #[serde(default, deserialize_with = "supplied_folder")]
        expected_folder: Option<String>,
        token: String,
    },
}
impl Request {
    fn source(&self) -> Source {
        match self {
            Self::ParameterAction { .. }
            | Self::EditNumericParameter { .. }
            | Self::EditChorderAutofill { .. }
            | Self::EditChorderModel { .. }
            | Self::ChorderLearningGet { .. } => Source::Dsp,
            Self::ModelCatalog { .. } => Source::Auto,
            Self::Inspect { source, .. }
            | Self::Parameters { source, .. }
            | Self::Choices { source, .. }
            | Self::Parameter { source, .. }
            | Self::ParameterDetails { source, .. }
            | Self::ParameterDigest { source, .. }
            | Self::FormatValues { source, .. }
            | Self::SetParameter { source, .. } => *source,
            Self::StateCapture { .. }
            | Self::StateRestore { .. }
            | Self::StateRelease { .. }
            | Self::Presets { .. }
            | Self::PresetApply { .. }
            | Self::PresetRelease { .. }
            | Self::InspectAux { .. }
            | Self::InspectInsertFolderAncestry { .. }
            | Self::InspectInsertFolderPresetPreflight { .. }
            | Self::InsertChainCapture { .. }
            | Self::InsertChainRestore { .. }
            | Self::InsertChainRelease { .. } => Source::Auto,
        }
    }
    fn expected(&self) -> Option<&str> {
        match self {
            Self::InsertChainCapture { .. }
            | Self::InsertChainRestore { .. }
            | Self::InsertChainRelease { .. } => None,
            Self::ParameterAction {
                expected_instance, ..
            }
            | Self::EditNumericParameter {
                expected_instance, ..
            }
            | Self::EditChorderAutofill {
                expected_instance, ..
            }
            | Self::EditChorderModel {
                expected_instance, ..
            }
            | Self::ChorderLearningGet {
                expected_instance, ..
            }
            | Self::ModelCatalog {
                expected_instance, ..
            } => Some(expected_instance.as_str()),
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
            | Self::ParameterDetails {
                expected_instance, ..
            }
            | Self::ParameterDigest {
                expected_instance, ..
            }
            | Self::FormatValues {
                expected_instance, ..
            }
            | Self::SetParameter {
                expected_instance, ..
            }
            | Self::StateCapture { expected_instance }
            | Self::StateRestore {
                expected_instance, ..
            }
            | Self::StateRelease {
                expected_instance, ..
            }
            | Self::Presets {
                expected_instance, ..
            }
            | Self::PresetApply {
                expected_instance, ..
            }
            | Self::PresetRelease {
                expected_instance, ..
            }
            | Self::InspectAux { expected_instance }
            | Self::InspectInsertFolderAncestry { expected_instance }
            | Self::InspectInsertFolderPresetPreflight { expected_instance } => {
                expected_instance.as_deref()
            }
        }
    }
    fn validate(&self) -> Result<(), String> {
        if let Some(uid) = self.expected() {
            if !valid_uid(uid) {
                return Err("expected_instance must be a nonzero canonical object UID".into());
            }
        }
        match self {
            Self::InsertChainCapture {
                bridge_session,
                document_epoch,
                expected_folder,
            }
            | Self::InsertChainRestore {
                bridge_session,
                document_epoch,
                expected_folder,
                ..
            }
            | Self::InsertChainRelease {
                bridge_session,
                document_epoch,
                expected_folder,
                ..
            } => {
                for value in [bridge_session, document_epoch] {
                    if value.is_empty()
                        || value.encode_utf16().count() > 128
                        || value.chars().any(char::is_control)
                    {
                        return Err(
                            "insert-chain bridge generation requires bounded nonempty text".into(),
                        );
                    }
                }
                if expected_folder.as_ref().is_some_and(|uid| !valid_uid(uid)) {
                    return Err("expected_folder must be a nonzero canonical object UID".into());
                }
                if let Self::InsertChainRestore { token, .. }
                | Self::InsertChainRelease { token, .. } = self
                {
                    if !state::valid_token(token) {
                        return Err(
                            "insert-chain token requires64 uppercase hexadecimal characters".into(),
                        );
                    }
                }
                Ok(())
            }
            Self::EditNumericParameter { param, value, .. } => {
                numeric_edit::validate(param, *value)
            }
            Self::ModelCatalog { offset, limit, .. } => catalog::validate_page(*offset, *limit),
            Self::ParameterDigest {
                expected_instance: None,
                ..
            } => Err("parameter_digest requires expected_instance".into()),
            Self::ParameterDigest {
                source: Source::Auto,
                ..
            } => Err("parameter_digest requires explicit dsp or controller source".into()),
            Self::FormatValues {
                expected_instance: None,
                ..
            } => Err("format_values requires expected_instance".into()),
            Self::FormatValues {
                source: Source::Auto,
                ..
            } => Err("format_values requires explicit dsp or controller source".into()),
            Self::FormatValues { values, .. }
                if !(1..=parameters::MAX_FORMAT_VALUES).contains(&values.len()) =>
            {
                Err("format_values requires 1..32 explicit values".into())
            }
            Self::FormatValues { values, .. }
                if values
                    .iter()
                    .any(|value| !value.is_finite() || value.abs() > MAX_SAFE_INTEGER as f64) =>
            {
                Err(
                    "format_values requires finite numbers within the safe JSON numeric range"
                        .into(),
                )
            }
            Self::ParameterDetails {
                expected_instance: None,
                ..
            } => Err("parameter_details requires expected_instance".into()),
            Self::ParameterDetails {
                compare_gui: true,
                source,
                param,
                ..
            } if *source != Source::Dsp || param != gui_parameter::PARAMETER => {
                Err("GUI comparison requires chorder.autofill and explicit dsp source".into())
            }
            Self::ParameterDetails {
                source: Source::Auto,
                ..
            } => Err("parameter_details requires explicit dsp or controller source".into()),
            Self::Inspect {
                include_nodes: false,
                node_scope: node_inspect::Scope::Graph | node_inspect::Scope::PluginController,
                ..
            } => Err("this node_scope requires include_nodes=true".into()),
            Self::Inspect {
                node_scope: node_inspect::Scope::PluginController,
                expected_instance: None,
                ..
            } => Err("plugin_controller node_scope requires expected_instance".into()),
            Self::StateCapture {
                expected_instance: None,
            }
            | Self::StateRestore {
                expected_instance: None,
                ..
            }
            | Self::StateRelease {
                expected_instance: None,
                ..
            }
            | Self::Presets {
                expected_instance: None,
                ..
            }
            | Self::PresetApply {
                expected_instance: None,
                ..
            }
            | Self::PresetRelease {
                expected_instance: None,
                ..
            }
            | Self::InspectAux {
                expected_instance: None,
            }
            | Self::InspectInsertFolderAncestry {
                expected_instance: None,
            }
            | Self::InspectInsertFolderPresetPreflight {
                expected_instance: None,
            } => Err("state, preset and auxiliary operations require expected_instance".into()),
            Self::Presets { limit, .. } if !(1..=presets::MAX_PAGE).contains(limit) => {
                Err("preset limit must be between 1 and 64".into())
            }
            Self::Presets {
                catalog_token: Some(token),
                ..
            }
            | Self::PresetApply {
                catalog_token: token,
                ..
            }
            | Self::PresetRelease {
                catalog_token: token,
                ..
            } if !state::valid_token(token) => {
                Err("catalog_token must contain64 uppercase hexadecimal characters".into())
            }
            Self::StateRestore { token, .. } | Self::StateRelease { token, .. }
                if !state::valid_token(token) =>
            {
                Err("state token must contain64 uppercase hexadecimal characters".into())
            }
            Self::Parameters { limit, .. } | Self::Choices { limit, .. }
                if !(1..=128).contains(limit) =>
            {
                Err("limit must be between 1 and 128".into())
            }
            Self::Parameter { param, .. }
            | Self::ParameterDetails { param, .. }
            | Self::FormatValues { param, .. }
            | Self::Choices { param, .. }
            | Self::SetParameter { param, .. }
                if param.is_empty() || param.len() > 512 || param.contains('\0') =>
            {
                Err("param must contain 1..512 UTF-8 bytes without NUL".into())
            }
            Self::SetParameter { mode, value, .. } => match (mode, value) {
                (Mode::FloatBits, _) => decode_float_bits(value).map(|_| ()),
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
    fn needs_auxiliary(&self) -> bool {
        matches!(
            self,
            Self::Presets {
                catalog_token: None,
                ..
            } | Self::InspectAux { .. }
                | Self::InspectInsertFolderAncestry { .. }
                | Self::InspectInsertFolderPresetPreflight { .. }
                | Self::InsertChainCapture { .. }
        )
    }
}
fn decode_float_bits(value: &Value) -> Result<f64, String> {
    let Some(text) = value
        .as_str()
        .filter(|s| s.len() == 16 && s.bytes().all(|c| c.is_ascii_hexdigit()))
    else {
        return Err("float_bits requires exactly16 hexadecimal characters".into());
    };
    let bits = u64::from_str_radix(text, 16).map_err(|_| "invalid float bits")?;
    let number = f64::from_bits(bits);
    if number.is_finite() {
        Ok(number)
    } else {
        Err("float_bits requires a finite native float".into())
    }
}
fn check_float_bits_type(description: &parameters::Description) -> Result<(), String> {
    if matches!(
        description.raw.as_ref().map(|v| &v.value),
        Some(parameters::Value::Float(_))
    ) {
        Ok(())
    } else {
        Err("float_bits requires a native Float value, never an integer".into())
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
fn check_service_active(pointer: *mut Service) -> Result<(), String> {
    let service = unsafe { &*pointer };
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

// Shared document-only final witness. No target UID/class getter or temporary
// native owner follows this witness in the folder publication path.
fn check_document_only(
    service: *mut Service,
    document: &Document,
    started: Instant,
    api: &Api,
) -> Result<(), String> {
    check_service_active(service)?;
    if !on_main(api) {
        return Err("request left the host GUI thread".into());
    }
    if started.elapsed() > Duration::from_secs(2) {
        return Err("native request exceeded its two second budget".into());
    }
    let active: Borrowed = unsafe { method(document.manager.0, 9) };
    let model: Borrowed = unsafe { method(document.document.0, 6) };
    if unsafe { active(document.manager.0) } != document.document.0
        || unsafe { model(document.document.0) } != document.model.0
    {
        return Err("active document/model changed".into());
    }
    check_service_active(service)
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
        check_service_active(self.service)
    }
    fn check_document(&self) -> Result<(), String> {
        check_document_only(self.service, &self.document, self.started, self.api)
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
    fn direct(target: *mut c_void) -> Result<Self, String> {
        let handle =
            query(target, &CONTROLLER).ok_or("direct Device has no controller interface")?;
        Ok(Self {
            handle,
            lease: None,
        })
    }
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
fn direct_controller(ctx: &Context<'_>) -> Result<Controller, String> {
    ctx.check()?;
    let controller = Controller::direct(ctx.target.0)?;
    ctx.check()?;
    Ok(controller)
}
struct DspProvider {
    metadata: DspParameters,
    direct: Result<Controller, String>,
}
fn dsp_provider(ctx: &Context<'_>, metadata: DspParameters) -> Result<Provider, String> {
    let direct = direct_controller(ctx);
    ctx.check()?;
    Ok(Provider::Dsp(DspProvider { metadata, direct }))
}
enum Provider {
    Dsp(DspProvider),
    DspValue(Controller),
    Controller(Controller),
    State(state::AliasLease),
}
impl Provider {
    fn source(&self) -> &'static str {
        match self {
            Self::Dsp(_) | Self::DspValue(_) => "dsp",
            Self::Controller(_) => "controller",
            Self::State(_) => "component_alias_storable",
        }
    }
    fn scalar(&self) -> Option<&Controller> {
        match self {
            Self::DspValue(controller) | Self::Controller(controller) => Some(controller),
            Self::Dsp(_) | Self::State(_) => None,
        }
    }
    fn check(&self) -> Result<(), String> {
        match self {
            Self::Dsp(provider) => provider.direct.as_ref().map_or(Ok(()), |c| c.check()),
            Self::DspValue(controller) | Self::Controller(controller) => controller.check(),
            Self::State(alias) => alias.check(),
        }
    }
}
fn provider(ctx: &Context<'_>, source: Source, scalar: bool) -> Result<Provider, String> {
    if scalar {
        return if source == Source::Dsp {
            direct_controller(ctx).map(Provider::DspValue)
        } else {
            controller(ctx).map(Provider::Controller)
        };
    }
    match source {
        Source::Dsp => dsp(ctx).and_then(|p| dsp_provider(ctx, p)),
        Source::Controller => controller(ctx).map(Provider::Controller),
        Source::Auto => {
            let d = dsp(ctx);
            if let Ok(ref p) = d {
                if p.count().map_err(|e| e.to_string())? > 0 {
                    return dsp_provider(ctx, d.expect("checked"));
                }
            }
            match controller(ctx) {
                Ok(c) if c.count()? > 0 => Ok(Provider::Controller(c)),
                Ok(c) => match d {
                    Ok(p) => dsp_provider(ctx, p),
                    Err(_) => Ok(Provider::Controller(c)),
                },
                Err(e) => d.and_then(|p| dsp_provider(ctx, p)).map_err(|_| e),
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
                parameters::Value::Float(n) => {
                    json!({"kind":"float","value":n,"bits_hex":format!("{:016X}", n.to_bits())})
                }
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
    json!({"name":d.name,"title":d.name,"tag":d.tag,"value":scalar(d.raw.as_ref()),"text":d.display,"min":scalar(d.min.as_ref()),"max":scalar(d.max.as_ref()),"default":scalar(d.default_value.as_ref()),"normalized":match d.normalized {parameters::Normalized::Finite(v)=>Some(v),_=>None},"enabled":d.enabled,"readOnly":d.read_only,"writable":supported&&!opaque,"native_kind":format!("{:?}",d.kind),"native_value":typed(d.raw.as_ref()),"native_min":typed(d.min.as_ref()),"native_max":typed(d.max.as_ref()),"native_default":typed(d.default_value.as_ref()),"native_normalized":match d.normalized {parameters::Normalized::NotApplicable=>json!({"kind":"not_applicable"}),parameters::Normalized::Finite(v) if v.is_finite()=>json!({"kind":"float32","bits_hex":format!("{:08X}",v.to_bits())}),parameters::Normalized::Finite(v)=>json!({"kind":"invalid_float32","bits_hex":format!("{:08X}",v.to_bits())}),parameters::Normalized::NonFinite{bits}=>json!({"kind":"non_finite_float32","bits_hex":format!("{bits:08X}")})},"native_support":{"raw_number":d.support.raw_number,"normalized":d.support.normalized,"display_text":d.support.display_text},"support":{"raw":safe_integers&&d.support.raw_number,"normalized":safe_integers&&d.support.normalized,"text":safe_integers&&d.support.display_text},"display_parser":format!("{:?}",d.display_parser)})
}
fn dsp_record(
    ctx: &Context<'_>,
    provider: &DspProvider,
    record: &fender_native_dsp_api::Parameter,
) -> Result<Value, String> {
    ctx.check()?;
    let access = match &provider.direct {
        Ok(controller) => controller
            .find(&record.name, ctx.api)
            .and_then(|parameter| parameter.describe().map_err(|e| e.to_string())),
        Err(error) => Err(error.clone()),
    };
    ctx.check()?;
    Ok(dsp_record_value(record, access))
}
fn dsp_record_value(
    record: &fender_native_dsp_api::Parameter,
    access: Result<parameters::Description, String>,
) -> Value {
    let mut value = json!({"index":record.index,"tag":record.tag,"name":record.name,"title":record.title,"folder":record.folder,"raw_flags":record.raw_flags});
    match access {
        Ok(access) => value["native_description"] = description(&access),
        Err(error) => {
            value["native_description"] = Value::Null;
            value["access_error"] = json!(error);
        }
    }
    value
}
fn page(ctx: &Context<'_>, p: &Provider, offset: u32, limit: u32) -> Result<Value, String> {
    ctx.check()?;
    match p {
        Provider::Dsp(p) => {
            let result = p.metadata.page(offset, limit).map_err(|e| e.to_string())?;
            ctx.check()?;
            let records = result
                .records
                .iter()
                .map(|record| dsp_record(ctx, p, record))
                .collect::<Result<Vec<_>, _>>()?;
            let failed_access: Vec<u32> = records
                .iter()
                .filter(|record| record.get("access_error").is_some())
                .map(|record| record["index"].as_u64().expect("DSP index") as u32)
                .collect();
            if p.metadata.count().map_err(|e| e.to_string())? != result.reported_count {
                return Err("DSP parameter count changed during access description".into());
            }
            ctx.check()?;
            Ok(
                json!({"parameters":records,"reported_count":result.reported_count,"offset":offset,"next_offset":result.next,"page_complete":result.failed_indices.is_empty(),"all_indices_read":result.complete,"failed_indices":result.failed_indices.iter().map(|r|json!({"index":r.index,"reason":format!("{:?}",r.reason)})).collect::<Vec<_>>(),"access_metadata_complete":failed_access.is_empty(),"failed_access_indices":failed_access,"coverage":"direct_device_dsp_provider"}),
            )
        }
        Provider::DspValue(_) => {
            Err("DSP scalar provider cannot enumerate controller indices".into())
        }
        Provider::State(_) => Err("state provider cannot enumerate parameters".into()),
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
        if matches!(mode, Mode::FloatBits) {
            check_float_bits_type(&current)?;
        }
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
        Mode::FloatBits => parameter.set_raw_number(
            parameters::Number::Float(
                decode_float_bits(&value).expect("validated finite float bits"),
            ),
            &mut guard,
        ),
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
        Request::Inspect { .. } => (|| {
            let mut value = match provider {
                Provider::Dsp(p) => p.metadata.count().map(|count| json!({"reported_count":count,"coverage":"direct_device_dsp_provider","native_setter":p.direct.is_ok(),"native_setter_scope":"direct_device_named_parameter"})).map_err(|e| e.to_string())?,
                Provider::Controller(c) => c.count().map(|count| json!({"reported_count":count,"coverage":c.coverage(),"native_setter":true}))?,
                Provider::DspValue(_) => return Err("inspect requires an indexed provider".into()),
                Provider::State(_) => return Err("inspect requires an indexed provider".into()),
            };
            value["capabilities"] = capabilities::inspect(ctx)?;
            Ok(value)
        })(),
        Request::Parameters { offset, limit, .. } => page(ctx, provider, offset, limit),
        Request::Choices {
            param,
            offset,
            limit,
            ..
        } => match provider.scalar() {
            Some(controller) => choices(ctx, controller, &param, offset, limit),
            None => Err("parameter provider has no scalar controller".into()),
        },
        Request::Parameter { param, .. } => match provider.scalar() {
            Some(c) => ctx
                .check()
                .and_then(|_| c.check())
                .and_then(|_| c.find(&param, ctx.api))
                .and_then(|parameter| {
                    ctx.check()?;
                    parameter.describe().map_err(|e| e.to_string())
                })
                .map(|d| json!({"parameter":description(&d)})),
            None => Err("parameter provider has no scalar controller".into()),
        },
        Request::ParameterDetails {
            param,
            include_owner: false,
            compare_gui: false,
            ..
        } => details::inspect(ctx, provider, &param),
        Request::ParameterDetails { .. } => {
            Err("parameter inspection requires its publication dispatcher".into())
        }
        Request::ParameterDigest { .. } => digest::inspect(ctx, provider),
        Request::FormatValues { param, values, .. } => {
            format::inspect(ctx, provider, &param, &values)
        }
        Request::SetParameter {
            param, mode, value, ..
        } => {
            return match provider.scalar() {
                Some(c) => set_parameter(ctx, c, &param, mode, value),
                None => error(
                    "unsupported_source",
                    "parameter provider has no scalar controller",
                    "not_applied",
                ),
            };
        }
        Request::ParameterAction { .. }
        | Request::EditNumericParameter { .. }
        | Request::EditChorderAutofill { .. }
        | Request::EditChorderModel { .. }
        | Request::ChorderLearningGet { .. }
        | Request::ModelCatalog { .. }
        | Request::StateCapture { .. }
        | Request::StateRestore { .. }
        | Request::StateRelease { .. }
        | Request::Presets { .. }
        | Request::PresetApply { .. }
        | Request::PresetRelease { .. }
        | Request::InspectAux { .. }
        | Request::InspectInsertFolderAncestry { .. }
        | Request::InspectInsertFolderPresetPreflight { .. }
        | Request::InsertChainCapture { .. }
        | Request::InsertChainRestore { .. }
        | Request::InsertChainRelease { .. } => Err("request requires its own dispatcher".into()),
    };
    match result.and_then(|value| {
        provider.check()?;
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
#[allow(
    clippy::large_enum_variant,
    reason = "Receipt transfer after the final host guard must not allocate"
)]
pub(crate) enum NativeReceiptLease {
    Device { _lease: ReceiptLease },
    InsertChain { _lease: insert_chain::ReceiptLease },
    DocumentContext { _lease: document_context::Receipt },
}
impl From<ReceiptLease> for NativeReceiptLease {
    fn from(lease: ReceiptLease) -> Self {
        Self::Device { _lease: lease }
    }
}
pub(super) enum NativeReply {
    Device(Box<Reply>),
    InsertChain(insert_chain::Reply),
    DocumentContext(document_context::Reply),
}
impl From<Reply> for NativeReply {
    fn from(reply: Reply) -> Self {
        Self::Device(Box::new(reply))
    }
}
impl NativeReply {
    pub(super) fn unbound(value: Value) -> Self {
        Reply::unbound(value).into()
    }
    pub(super) fn error(code: &str, message: &str, outcome: &str) -> Self {
        Reply::error(code, message, outcome).into()
    }
    pub(super) fn encode(&mut self) -> String {
        match self {
            Self::Device(reply) => reply.encode(),
            Self::InsertChain(reply) => reply.encode(),
            Self::DocumentContext(reply) => reply.encode(),
        }
    }
    pub(super) fn final_check(&mut self, service: *mut Service) -> bool {
        match self {
            Self::Device(reply) => reply.final_check(service),
            Self::InsertChain(reply) => reply.final_check(service),
            Self::DocumentContext(reply) => reply.final_check(service),
        }
    }
    pub(super) fn take_receipt(&mut self) -> Result<Option<NativeReceiptLease>, ()> {
        match self {
            Self::Device(reply) => Ok(reply.take_receipt().map(NativeReceiptLease::from)),
            Self::InsertChain(reply) => reply.take_receipt().map(|receipt| {
                receipt.map(|lease| NativeReceiptLease::InsertChain { _lease: lease })
            }),
            Self::DocumentContext(reply) => Ok(reply
                .take_receipt()
                .map(|lease| NativeReceiptLease::DocumentContext { _lease: lease })),
        }
    }
    pub(super) fn release_context(&mut self) {
        match self {
            Self::Device(reply) => reply.release_context(),
            Self::InsertChain(reply) => reply.release_context(),
            Self::DocumentContext(reply) => reply.release_context(),
        }
    }
}
enum ParameterEditLease {
    Direct(Box<numeric_edit::EditLease>),
    ChorderGui(Box<gui_parameter::AutofillEditLease>),
    ChorderModel(Box<gui_model::EditLease>),
}
impl ParameterEditLease {
    fn phase(&self) -> &'static str {
        match self {
            Self::Direct(lease) => lease.phase(),
            Self::ChorderGui(lease) => lease.phase(),
            Self::ChorderModel(lease) => lease.phase(),
        }
    }
    fn began(&self) -> bool {
        match self {
            Self::Direct(lease) => lease.began(),
            Self::ChorderGui(lease) => lease.began(),
            Self::ChorderModel(lease) => lease.began(),
        }
    }
    fn check(&self, ctx: &Context<'_>) -> Result<(), String> {
        match self {
            Self::Direct(lease) => lease.check(ctx),
            Self::ChorderGui(lease) => lease.check(ctx),
            Self::ChorderModel(lease) => lease.check(ctx),
        }
    }
}
pub(crate) struct ReceiptLease {
    _catalog: Option<catalog::Catalog>,
    _edit: Option<ParameterEditLease>,
    _action: Option<action::ActionLease>,
    _presets: Option<presets::Receipt>,
    _owner: Option<owner_inspect::OwnerInspection>,
    _gui: Option<gui_parameter::ComparisonRead>,
    _auxiliary: Option<node_inspect::Auxiliary>,
    _folder_preflight: Option<insert_folder::InsertFolderPreflight>,
    _nodes: Option<node_inspect::Inventory>,
    _provider: Option<Provider>,
    _context: Box<Context<'static>>,
}
pub(super) struct Reply {
    value: Value,
    catalog: Option<catalog::Catalog>,
    edit: Option<ParameterEditLease>,
    action: Option<action::ActionLease>,
    presets: Option<presets::Receipt>,
    owner: Option<owner_inspect::OwnerInspection>,
    gui: Option<gui_parameter::ComparisonRead>,
    auxiliary: Option<node_inspect::Auxiliary>,
    folder_preflight: Option<insert_folder::InsertFolderPreflight>,
    nodes: Option<node_inspect::Inventory>,
    provider: Option<Provider>,
    context: Option<Box<Context<'static>>>,
    mutation: bool,
}
impl Reply {
    pub(super) fn unbound(value: Value) -> Self {
        Self {
            value,
            catalog: None,
            edit: None,
            action: None,
            presets: None,
            owner: None,
            gui: None,
            auxiliary: None,
            folder_preflight: None,
            nodes: None,
            provider: None,
            context: None,
            mutation: false,
        }
    }
    pub(super) fn error(code: &str, message: &str, outcome: &str) -> Self {
        Self::unbound(error(code, message, outcome))
    }
    fn invalidate(&mut self, message: String) {
        let outcome = if self.edit.as_ref().is_some_and(ParameterEditLease::began)
            || self.mutation && self.value["outcome"] != "not_applied"
        {
            "unknown"
        } else {
            "not_applied"
        };
        if let Some(edit) = self.edit.as_ref() {
            self.value["edit_phase"] = json!(edit.phase());
        }
        self.value["ok"] = json!(false);
        self.value["error"] = json!({"code":"context_changed_before_receipt","message":message});
        self.value["outcome"] = json!(outcome);
        self.value
            .as_object_mut()
            .expect("object response")
            .remove("verified");
        digest::invalidate(&mut self.value);
        owner_inspect::invalidate(&mut self.value);
        gui_parameter::invalidate(&mut self.value);
        catalog::invalidate(&mut self.value);
        presets::invalidate(&mut self.value);
        insert_folder::invalidate(&mut self.value);
        if self.value["scope"] == "insert_folder_ancestry_observation" {
            let object = self.value.as_object_mut().expect("object response");
            object.remove("insert_folder_ancestry_report");
            object.remove("ancestor_matches_active_model");
            object.remove("document_environment_observation");
        }
        if self
            .nodes
            .as_ref()
            .is_some_and(node_inspect::Inventory::is_plugin_controller)
        {
            self.value
                .as_object_mut()
                .expect("object response")
                .remove("node_report");
        }
    }
    pub(super) fn final_check(&mut self, service: *mut Service) -> bool {
        let result = match self.context.as_ref() {
            Some(ctx) => self
                .presets
                .as_ref()
                .map_or(Ok(()), |presets| presets.check(ctx))
                .and_then(|_| {
                    self.owner.as_ref().map_or(Ok(()), |owner| {
                        owner.check(&mut || {
                            self.provider.as_ref().map_or(Ok(()), Provider::check)?;
                            ctx.check()
                        })
                    })
                })
                .and_then(|_| {
                    self.auxiliary
                        .as_ref()
                        .map_or(Ok(()), |auxiliary| auxiliary.check(ctx))
                })
                .and_then(|_| self.provider.as_ref().map_or(Ok(()), Provider::check))
                .and_then(|_| ctx.check())
                .and_then(|_| {
                    self.nodes.as_ref().map_or(Ok(()), |nodes| {
                        nodes.check(&mut || {
                            self.provider.as_ref().map_or(Ok(()), Provider::check)?;
                            ctx.check()
                        })
                    })
                })
                .and_then(|_| {
                    self.action.as_ref().map_or(Ok(()), |action| {
                        let provider = self
                            .provider
                            .as_ref()
                            .and_then(Provider::scalar)
                            .ok_or("action scalar provider unavailable")?;
                        action.check(provider, &mut || {
                            provider.check()?;
                            ctx.check()
                        })
                    })
                })
                .and_then(|_| {
                    self.catalog.as_ref().map_or(Ok(()), |catalog| {
                        catalog.check(&mut || {
                            self.provider.as_ref().map_or(Ok(()), Provider::check)?;
                            ctx.check()
                        })
                    })
                })
                .and_then(|_| {
                    self.folder_preflight
                        .as_ref()
                        .map_or(Ok(()), |preflight| preflight.check(ctx))
                })
                .and_then(|_| self.edit.as_ref().map_or(Ok(()), |edit| edit.check(ctx)))
                .and_then(|_| {
                    self.gui.as_ref().map_or(Ok(()), |gui| {
                        let controller = self
                            .provider
                            .as_ref()
                            .and_then(Provider::scalar)
                            .ok_or("GUI comparison scalar provider unavailable")?;
                        gui.check(ctx, controller)
                    })
                }),
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
            _catalog: self.catalog.take(),
            _edit: self.edit.take(),
            _action: self.action.take(),
            _presets: self.presets.take(),
            _owner: self.owner.take(),
            _gui: self.gui.take(),
            _auxiliary: self.auxiliary.take(),
            _folder_preflight: self.folder_preflight.take(),
            _nodes: self.nodes.take(),
            _provider: self.provider.take(),
            _context: context,
        })
    }
    pub(super) fn release_context(&mut self) {
        drop(self.edit.take());
        drop(self.catalog.take());
        drop(self.action.take());
        drop(self.presets.take());
        drop(self.owner.take());
        drop(self.gui.take());
        drop(self.folder_preflight.take());
        drop(self.auxiliary.take());
        drop(self.nodes.take());
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
            if self.edit.as_ref().is_some_and(ParameterEditLease::began)
                || self.mutation && self.value["outcome"] != "not_applied"
            {
                "unknown"
            } else {
                "not_applied"
            },
        );
        if let Some(phase) = self.value.get("edit_phase") {
            value["edit_phase"] = phase.clone();
        }
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
pub(super) fn dispatch(
    service: *mut Service,
    target: Owned,
    text: &str,
    auxiliary: Option<Owned>,
    a: &'static Api,
) -> NativeReply {
    let request: Request = match serde_json::from_str(text) {
        Ok(r) => r,
        Err(_) => {
            return NativeReply::error(
                "invalid_request",
                "unsupported operation, field or JSON shape",
                "not_applied",
            );
        }
    };
    if let Err(e) = request.validate() {
        return NativeReply::error("invalid_request", &e, "not_applied");
    }
    if request.needs_auxiliary() != auxiliary.is_some() {
        return NativeReply::error(
            "invalid_arguments",
            "initial presets and fixed auxiliary observations require exactly one auxiliary native object",
            "not_applied",
        );
    }
    if matches!(
        request,
        Request::InsertChainCapture { .. }
            | Request::InsertChainRestore { .. }
            | Request::InsertChainRelease { .. }
    ) {
        return NativeReply::InsertChain(insert_chain::dispatch(
            service, target, request, auxiliary, a,
        ));
    }
    dispatch_device(service, target, request, auxiliary, a).into()
}
fn dispatch_device(
    service: *mut Service,
    target: Owned,
    request: Request,
    auxiliary: Option<Owned>,
    a: &'static Api,
) -> Reply {
    let ctx = match Context::capture(service, target, request.expected(), a) {
        Ok(ctx) => Box::new(ctx),
        Err(e) => return Reply::error("context_guard_failed", &e, "not_applied"),
    };
    let source = request.source();
    let mutation = matches!(
        request,
        Request::SetParameter { .. }
            | Request::EditNumericParameter { .. }
            | Request::EditChorderAutofill { .. }
            | Request::EditChorderModel { .. }
            | Request::ParameterAction { .. }
            | Request::StateRestore { .. }
            | Request::PresetApply { .. }
    );
    if let Err(reason) = release_previous(&ctx)
        .and_then(|_| state::expire(&ctx))
        .and_then(|_| presets::expire(&ctx))
        .and_then(|_| insert_chain::expire(&ctx))
    {
        return Reply {
            value: error("context_guard_failed", &reason, "not_applied"),
            catalog: None,
            edit: None,
            action: None,
            presets: None,
            auxiliary: None,
            folder_preflight: None,
            owner: None,
            gui: None,
            nodes: None,
            provider: None,
            context: Some(ctx),
            mutation,
        };
    }
    if let Request::EditNumericParameter { param, value, .. } = request {
        let (mut value, edit) = numeric_edit::execute(&ctx, &param, value);
        value["instance"] = json!(ctx.instance);
        value["source"] = json!("dsp");
        return Reply {
            value,
            catalog: None,
            edit: edit.map(|lease| ParameterEditLease::Direct(Box::new(lease))),
            action: None,
            presets: None,
            auxiliary: None,
            folder_preflight: None,
            owner: None,
            gui: None,
            nodes: None,
            provider: None,
            context: Some(ctx),
            mutation: true,
        };
    }
    if let Request::EditChorderModel { change, .. } = request {
        let (mut value, edit) = gui_model::edit(&ctx, change);
        if value["ok"] == true {
            value["instance"] = json!(ctx.instance);
        }
        return Reply {
            value,
            catalog: None,
            edit: edit.map(|lease| ParameterEditLease::ChorderModel(Box::new(lease))),
            action: None,
            presets: None,
            auxiliary: None,
            folder_preflight: None,
            owner: None,
            gui: None,
            nodes: None,
            provider: None,
            context: Some(ctx),
            mutation: true,
        };
    }
    if let Request::ChorderLearningGet { .. } = request {
        let (mut value, gui, provider) = match gui_model::get(&ctx) {
            Ok((value, gui, direct)) => (value, Some(gui), Some(Provider::DspValue(direct))),
            Err(reason) => (
                error("chorder_learning_unavailable", &reason, "not_applied"),
                None,
                None,
            ),
        };
        if value["ok"] == true {
            value["instance"] = json!(ctx.instance);
        }
        return Reply {
            value,
            catalog: None,
            edit: None,
            action: None,
            presets: None,
            auxiliary: None,
            folder_preflight: None,
            owner: None,
            gui,
            nodes: None,
            provider,
            context: Some(ctx),
            mutation: false,
        };
    }
    if let Request::EditChorderAutofill { enabled, .. } = request {
        let (mut value, edit) = gui_parameter::edit_autofill(&ctx, enabled);
        value["instance"] = json!(ctx.instance);
        return Reply {
            value,
            catalog: None,
            edit: edit.map(|lease| ParameterEditLease::ChorderGui(Box::new(lease))),
            action: None,
            presets: None,
            auxiliary: None,
            folder_preflight: None,
            owner: None,
            gui: None,
            nodes: None,
            provider: None,
            context: Some(ctx),
            mutation: true,
        };
    }
    if let Request::InspectAux { .. } = request {
        let (value, auxiliary) = match node_inspect::capture_auxiliary(
            &ctx,
            auxiliary.expect("validated auxiliary"),
        ) {
            Ok(auxiliary) => (
                json!({"ok":true,"outcome":"observed","scope":"auxiliary_type_observation","mutation_capability":false,"auxiliary_report":auxiliary.value()}),
                Some(auxiliary),
            ),
            Err(reason) => (
                error("auxiliary_inspection_failed", &reason, "not_applied"),
                None,
            ),
        };
        let mut reply = Reply {
            value,
            catalog: None,
            edit: None,
            action: None,
            presets: None,
            auxiliary,
            folder_preflight: None,
            owner: None,
            gui: None,
            nodes: None,
            provider: None,
            context: Some(ctx),
            mutation: false,
        };
        reply.value["instance"] = json!(reply.context.as_ref().expect("bound context").instance);
        return reply;
    }
    if let Request::InspectInsertFolderPresetPreflight { .. } = request {
        let (value, folder_preflight) = match insert_folder::InsertFolderPreflight::capture(
            &ctx,
            auxiliary.expect("validated Inserts auxiliary"),
        ) {
            Ok(preflight) => (preflight.value(), Some(preflight)),
            Err(reason) => (
                error("insert_folder_preflight_failed", &reason, "not_applied"),
                None,
            ),
        };
        let mut reply = Reply {
            value,
            catalog: None,
            edit: None,
            action: None,
            presets: None,
            auxiliary: None,
            folder_preflight,
            owner: None,
            gui: None,
            nodes: None,
            provider: None,
            context: Some(ctx),
            mutation: false,
        };
        reply.value["instance"] = json!(reply.context.as_ref().expect("bound context").instance);
        return reply;
    }
    if let Request::InspectInsertFolderAncestry { .. } = request {
        let (value, auxiliary) = match node_inspect::capture_insert_folder_ancestry(
            &ctx,
            auxiliary.expect("validated Inserts auxiliary"),
        ) {
            Ok(auxiliary) => {
                let report = auxiliary.value();
                (
                    json!({"ok":true,"outcome":"observed","scope":"insert_folder_ancestry_observation",
                        "mutation_capability":false,"atomic_snapshot":false,
                        "ancestor_matches_active_model":report["ancestor_matches_active_model"],
                        "document_environment_observation":report["document_environment_observation"],
                        "insert_folder_ancestry_report":report}),
                    Some(auxiliary),
                )
            }
            Err(reason) => (
                error("insert_folder_ancestry_failed", &reason, "not_applied"),
                None,
            ),
        };
        let mut reply = Reply {
            value,
            catalog: None,
            edit: None,
            action: None,
            presets: None,
            auxiliary,
            folder_preflight: None,
            owner: None,
            gui: None,
            nodes: None,
            provider: None,
            context: Some(ctx),
            mutation: false,
        };
        reply.value["instance"] = json!(reply.context.as_ref().expect("bound context").instance);
        return reply;
    }
    if matches!(
        request,
        Request::Presets { .. } | Request::PresetApply { .. } | Request::PresetRelease { .. }
    ) {
        let (mut value, catalog) = presets::execute(&ctx, &request, auxiliary);
        value["instance"] = json!(ctx.instance);
        value["source"] = json!("native_slot_preset_catalog");
        return Reply {
            value,
            catalog: None,
            edit: None,
            action: None,
            presets: catalog,
            auxiliary: None,
            folder_preflight: None,
            owner: None,
            gui: None,
            nodes: None,
            provider: None,
            context: Some(ctx),
            mutation,
        };
    }
    if let Request::ModelCatalog {
        catalog: kind,
        offset,
        limit,
        ..
    } = request
    {
        let (mut value, catalog) = match catalog::capture(&ctx, kind, offset, limit) {
            Ok(catalog) => (catalog.value(), Some(catalog)),
            Err(reason) => (error("model_catalog_failed", &reason, "not_applied"), None),
        };
        value["instance"] = json!(ctx.instance);
        return Reply {
            value,
            catalog,
            edit: None,
            action: None,
            presets: None,
            owner: None,
            gui: None,
            auxiliary: None,
            folder_preflight: None,
            nodes: None,
            provider: None,
            context: Some(ctx),
            mutation: false,
        };
    }
    let state_result = match &request {
        Request::StateCapture { .. } => Some(match state::capture(&ctx) {
            Ok((value, alias)) => (value, Some(Provider::State(alias))),
            Err(reason) => (error("state_capture_failed", &reason, "not_applied"), None),
        }),
        Request::StateRestore { token, .. } => {
            let (value, alias) = state::restore(&ctx, token);
            Some((value, alias.map(Provider::State)))
        }
        Request::StateRelease { token, .. } => Some((
            state::release(&ctx, token)
                .unwrap_or_else(|reason| error("state_release_failed", &reason, "not_applied")),
            None,
        )),
        _ => None,
    };
    if let Some((mut value, provider)) = state_result {
        value["instance"] = json!(ctx.instance);
        value["source"] = json!("component_alias_storable");
        return Reply {
            value,
            catalog: None,
            edit: None,
            action: None,
            presets: None,
            auxiliary: None,
            folder_preflight: None,
            owner: None,
            gui: None,
            nodes: None,
            provider,
            context: Some(ctx),
            mutation,
        };
    }
    if let Request::ParameterDetails {
        compare_gui: true,
        include_owner,
        ..
    } = &request
    {
        let (mut value, provider, owner, gui) = match provider(&ctx, Source::Dsp, true) {
            Ok(provider) => match details::inspect_gui(&ctx, &provider, *include_owner) {
                Ok(read) => (read.value, Some(provider), read.owner, Some(read.gui)),
                Err(reason) => (
                    error("gui_parameter_inspection_failed", &reason, "not_applied"),
                    Some(provider),
                    None,
                    None,
                ),
            },
            Err(reason) => (
                error("provider_unavailable", &reason, "not_applied"),
                None,
                None,
                None,
            ),
        };
        value["source"] = json!("dsp");
        value["instance"] = json!(ctx.instance);
        return Reply {
            value,
            catalog: None,
            edit: None,
            action: None,
            presets: None,
            auxiliary: None,
            folder_preflight: None,
            owner,
            gui,
            nodes: None,
            provider,
            context: Some(ctx),
            mutation: false,
        };
    }
    let scalar = matches!(
        request,
        Request::Parameter { .. }
            | Request::Choices { .. }
            | Request::SetParameter { .. }
            | Request::ParameterAction { .. }
            | Request::ParameterDetails { .. }
            | Request::FormatValues { .. }
    );
    let node_scope = match &request {
        Request::Inspect {
            include_nodes: true,
            node_scope,
            ..
        } => Some(*node_scope),
        _ => None,
    };
    let (mut value, provider, owner, action) = match provider(&ctx, source, scalar) {
        Ok(provider) => {
            let (mut value, owner, action) = match request {
                Request::ParameterDetails {
                    param,
                    include_owner: true,
                    ..
                } => match details::inspect_owner(&ctx, &provider, &param) {
                    Ok(read) => (read.value, Some(read.owner), None),
                    Err(reason) => (
                        error("parameter_owner_inspection_failed", &reason, "not_applied"),
                        None,
                        None,
                    ),
                },
                Request::ParameterAction { action, .. } => {
                    let (value, lease) = action::execute(&ctx, &provider, action);
                    (value, None, lease)
                }
                request => (execute(&ctx, request, &provider), None, None),
            };
            value["source"] = json!(provider.source());
            (value, Some(provider), owner, action)
        }
        Err(e) => {
            let mut value = error("provider_unavailable", &e, "not_applied");
            value["source"] = match source {
                Source::Dsp => json!("dsp"),
                Source::Controller => json!("controller"),
                Source::Auto => Value::Null,
            };
            (value, None, None, None)
        }
    };
    let nodes = if let Some(scope) = node_scope.filter(|_| value["ok"] == true) {
        match node_inspect::capture(&ctx, scope) {
            Ok(nodes) => {
                value["node_report"] = nodes.value();
                Some(nodes)
            }
            Err(reason) => {
                value = error("node_inspection_failed", &reason, "not_applied");
                if let Some(provider) = provider.as_ref() {
                    value["source"] = json!(provider.source());
                }
                None
            }
        }
    } else {
        None
    };
    value["instance"] = json!(ctx.instance);
    Reply {
        value,
        catalog: None,
        edit: None,
        action,
        presets: None,
        auxiliary: None,
        folder_preflight: None,
        owner,
        gui: None,
        nodes,
        provider,
        context: Some(ctx),
        mutation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn description_fixture() -> parameters::Description {
        parameters::Description {
            name: "synthetic".into(),
            tag: 7,
            kind: parameters::Kind::Float,
            enabled: true,
            read_only: false,
            raw: Some(parameters::NativeValue {
                variant_tag: 2,
                value: parameters::Value::Float(1.0),
            }),
            min: None,
            max: None,
            default_value: None,
            normalized: parameters::Normalized::Finite(0.5),
            display: "synthetic".into(),
            display_parser: parameters::DisplayParser::PlainNumeric,
            support: parameters::Support {
                raw_number: true,
                normalized: true,
                display_text: true,
            },
        }
    }
    #[test]
    fn native_normalized_preserves_applicability_signed_zero_and_exact_float32_bits() {
        for (normalized, expected) in [
            (
                parameters::Normalized::NotApplicable,
                json!({"kind":"not_applicable"}),
            ),
            (
                parameters::Normalized::Finite(-0.0),
                json!({"kind":"float32","bits_hex":"80000000"}),
            ),
            (
                parameters::Normalized::Finite(f32::from_bits(0x3F80_0001)),
                json!({"kind":"float32","bits_hex":"3F800001"}),
            ),
            (
                parameters::Normalized::NonFinite { bits: 0x7FC0_0042 },
                json!({"kind":"non_finite_float32","bits_hex":"7FC00042"}),
            ),
            (
                parameters::Normalized::NonFinite { bits: 0xFF80_0000 },
                json!({"kind":"non_finite_float32","bits_hex":"FF800000"}),
            ),
        ] {
            let mut d = description_fixture();
            d.normalized = normalized;
            let value = description(&d);
            assert_eq!(value["native_normalized"], expected);
            match normalized {
                parameters::Normalized::Finite(v) => assert_eq!(
                    (value["normalized"].as_f64().unwrap() as f32).to_bits(),
                    v.to_bits()
                ),
                _ => assert_eq!(value["normalized"], Value::Null),
            }
            let transported: Value = serde_json::from_str(&value.to_string()).unwrap();
            assert_eq!(transported["native_normalized"], expected);
        }
    }
    #[test]
    fn description_json_transport_preserves_promoted_normalized_and_exact_f64_bits() {
        let normalized = f32::from_bits(0x307D_AB0C);
        for bits in [
            f64::from(normalized).to_bits(), // The Unit13 normalized shadow lost two f64 ULPs.
            0x3FB9_9999_9999_999A,
            0x3FB9_9999_9999_999B,
            0x3FEF_FFFF_FFFF_FFFF,
            0x3FF0_0000_0000_0001,
            0x8000_0000_0000_0000,
            0x0000_0000_0000_0001,
            0x000F_FFFF_FFFF_FFFF,
            0x0010_0000_0000_0000,
            0x7FEF_FFFF_FFFF_FFFF,
            0xFFEF_FFFF_FFFF_FFFF,
        ] {
            let mut d = description_fixture();
            d.raw = Some(parameters::NativeValue {
                variant_tag: 2,
                value: parameters::Value::Float(f64::from_bits(bits)),
            });
            d.normalized = parameters::Normalized::Finite(normalized);
            let mut wire = serde_json::to_string(&description(&d)).unwrap();
            for _ in 0..2 {
                let parsed: Value = serde_json::from_str(&wire).unwrap();
                assert_eq!(parsed["value"].as_f64().unwrap().to_bits(), bits);
                assert_eq!(
                    parsed["native_value"]["data"]["bits_hex"],
                    format!("{bits:016X}")
                );
                assert_eq!(
                    parsed["normalized"].as_f64().unwrap().to_bits(),
                    f64::from(normalized).to_bits()
                );
                assert_eq!(parsed["native_normalized"]["bits_hex"], "307DAB0C");
                wire = serde_json::to_string(&parsed).unwrap();
            }
        }
    }
    #[test]
    fn invalid_finite_normalized_values_have_an_explicit_non_numeric_marker() {
        for bits in [0x7FC0_1234, 0x7F80_0000, 0xFF80_0000] {
            let mut d = description_fixture();
            d.normalized = parameters::Normalized::Finite(f32::from_bits(bits));
            let value = description(&d);
            assert_eq!(
                value["native_normalized"],
                json!({"kind":"invalid_float32","bits_hex":format!("{bits:08X}")})
            );
            assert_eq!(value["normalized"], Value::Null);
            assert!(value["native_normalized"].get("value").is_none());
            assert!(serde_json::from_str::<Value>(&value.to_string()).is_ok());
        }
    }
    #[test]
    fn descriptions_expose_exact_float_extrema_and_preserve_scalar_fields() {
        let mut d = description_fixture();
        d.min = Some(parameters::NativeValue {
            variant_tag: 2,
            value: parameters::Value::Float(-0.0),
        });
        d.max = Some(parameters::NativeValue {
            variant_tag: 2,
            value: parameters::Value::Float(f64::from_bits(0x3FF0_0000_0000_0001)),
        });
        d.default_value = Some(parameters::NativeValue {
            variant_tag: 2,
            value: parameters::Value::Float(0.0),
        });
        let value = description(&d);
        assert_eq!(
            value["min"].as_f64().unwrap().to_bits(),
            (-0.0_f64).to_bits()
        );
        assert_eq!(value["max"], scalar(d.max.as_ref()));
        assert_eq!(value["default"], json!(0.0));
        assert_eq!(value["native_min"]["data"]["bits_hex"], "8000000000000000");
        assert_eq!(value["native_max"]["data"]["bits_hex"], "3FF0000000000001");
        assert_eq!(
            value["native_default"]["data"]["bits_hex"],
            "0000000000000000"
        );
        for key in ["native_min", "native_max", "native_default"] {
            assert_eq!(value[key]["variant_tag"], 2);
            assert_eq!(value[key]["data"]["kind"], "float");
        }
        let transported: Value = serde_json::from_str(&value.to_string()).unwrap();
        assert_eq!(
            transported["native_max"]["data"]["bits_hex"],
            "3FF0000000000001"
        );
        assert_eq!(
            value["native_support"],
            json!({"raw_number":true,"normalized":true,"display_text":true})
        );
        assert_eq!(
            value["support"],
            json!({"raw":true,"normalized":true,"text":true})
        );
        d.min = None;
        d.max = None;
        d.default_value = None;
        let value = description(&d);
        for key in [
            "min",
            "max",
            "default",
            "native_min",
            "native_max",
            "native_default",
        ] {
            assert_eq!(value[key], Value::Null);
        }
    }
    #[test]
    fn exact_integer_extrema_keep_native_support_when_public_safety_filters_it() {
        for n in [
            1,
            MAX_SAFE_INTEGER,
            -MAX_SAFE_INTEGER,
            MAX_SAFE_INTEGER + 2,
            -MAX_SAFE_INTEGER - 2,
            i64::MIN,
            i64::MAX,
        ] {
            let native = parameters::NativeValue {
                variant_tag: 1,
                value: parameters::Value::Integer(n),
            };
            let mut d = description_fixture();
            d.kind = parameters::Kind::Integer;
            d.raw = Some(native.clone());
            d.min = Some(native.clone());
            d.max = Some(native.clone());
            d.default_value = Some(native);
            let value = description(&d);
            let safe = n.abs_diff(0) <= MAX_SAFE_INTEGER as u64;
            for key in ["native_min", "native_max", "native_default"] {
                assert_eq!(value[key]["variant_tag"], 1);
                assert_eq!(value[key]["data"]["kind"], "integer");
                if safe {
                    assert_eq!(value[key]["data"]["value"], json!(n));
                    assert!(value[key]["data"].get("exact_decimal").is_none());
                } else {
                    assert_eq!(value[key]["data"]["exact_decimal"], n.to_string());
                    assert!(value[key]["data"].get("value").is_none());
                }
            }
            for key in ["value", "min", "max", "default"] {
                assert_eq!(value[key], if safe { json!(n) } else { Value::Null });
            }
            assert_eq!(value["writable"], safe);
            assert_eq!(
                value["support"],
                json!({"raw":safe,"normalized":safe,"text":safe})
            );
            assert_eq!(
                value["native_support"],
                json!({"raw_number":true,"normalized":true,"display_text":true})
            );
        }
    }
    #[test]
    fn preset_and_auxiliary_requests_require_exact_target_and_finite_fields() {
        let expected = uid(&SERVICE_CID);
        for (operation, extra, needs_auxiliary) in [
            ("presets", json!({}), true),
            ("presets", json!({"catalog_token":"A".repeat(64)}), false),
            (
                "preset_apply",
                json!({"catalog_token":"A".repeat(64),"index":0}),
                false,
            ),
            (
                "preset_release",
                json!({"catalog_token":"A".repeat(64)}),
                false,
            ),
            ("inspect_aux", json!({}), true),
            ("inspect_insert_folder_ancestry", json!({}), true),
            ("inspect_insert_folder_preset_preflight", json!({}), true),
        ] {
            let mut value = json!({"operation":operation,"expected_instance":expected});
            value
                .as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            let request = parse(value.clone()).unwrap();
            assert_eq!(request.needs_auxiliary(), needs_auxiliary);
            let mut missing = value.clone();
            missing.as_object_mut().unwrap().remove("expected_instance");
            assert!(parse(missing).is_err());
            for field in [
                "source",
                "url",
                "path",
                "bytes",
                "script",
                "method",
                "limits",
                "ancestors",
                "iid",
            ] {
                let mut unchecked = value.clone();
                unchecked[field] = json!("rejected");
                assert!(parse(unchecked).is_err());
            }
        }
        let request = parse(json!({"operation":"presets","expected_instance":expected})).unwrap();
        assert!(matches!(request, Request::Presets { limit: 32, .. }));
        for limit in [0, 65, 128] {
            assert!(
                parse(json!({"operation":"presets","limit":limit,"expected_instance":expected}))
                    .is_err()
            );
        }
        for token in ["a".repeat(64), "F".repeat(63), "G".repeat(64)] {
            assert!(parse(json!({"operation":"presets","catalog_token":token,"expected_instance":expected})).is_err());
        }
        assert!(
            !parse(json!({"operation":"inspect"}))
                .unwrap()
                .needs_auxiliary()
        );
    }

    #[test]
    fn reply_context_invalidation_strips_the_optional_preset_display_observation() {
        let mut reply = Reply::unbound(json!({"ok":true,"outcome":"observed",
            "current_preset_observation":{"status":"observed","display_text":"stale"}}));
        reply.invalidate("preset attachment changed".into());
        assert_eq!(reply.value["ok"], false);
        assert_eq!(reply.value["outcome"], "not_applied");
        assert!(reply.value.get("current_preset_observation").is_none());
    }
    #[test]
    fn reply_context_invalidation_strips_insert_folder_ancestry_claims() {
        let mut reply = Reply::unbound(json!({"ok":true,"outcome":"observed",
            "scope":"insert_folder_ancestry_observation","ancestor_matches_active_model":true,
            "document_environment_observation":{"status":"observed","ancestor_matches":true},
            "insert_folder_ancestry_report":{"nodes":[],"ancestor_matches_active_model":true}}));
        reply.invalidate("document or parent chain changed".into());
        assert_eq!(reply.value["ok"], false);
        assert_eq!(reply.value["outcome"], "not_applied");
        assert!(reply.value.get("ancestor_matches_active_model").is_none());
        assert!(reply.value.get("insert_folder_ancestry_report").is_none());
        assert!(
            reply
                .value
                .get("document_environment_observation")
                .is_none()
        );
    }
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
    fn node_inspection_is_an_optional_boolean_for_inspect_only() {
        let default = parse(json!({"operation":"inspect"})).unwrap();
        assert!(matches!(
            default,
            Request::Inspect {
                include_nodes: false,
                ..
            }
        ));
        let explicit =
            parse(json!({"operation":"inspect","source":"dsp","include_nodes":true})).unwrap();
        assert!(matches!(
            explicit,
            Request::Inspect {
                include_nodes: true,
                ..
            }
        ));
        for request in [
            json!({"operation":"inspect","include_nodes":"all"}),
            json!({"operation":"inspect","include_nodes":1}),
            json!({"operation":"parameters","include_nodes":true}),
            json!({"operation":"inspect","include_nodes":true,"depth":1000}),
            json!({"operation":"inspect","include_nodes":true,"method":"invoke"}),
            json!({"operation":"inspect","node_scope":"graph"}),
            json!({"operation":"inspect","include_nodes":true,"node_scope":"arbitrary/path"}),
            json!({"operation":"inspect","include_nodes":true,"node_scope":false}),
            json!({"operation":"parameters","include_nodes":true,"node_scope":"graph"}),
        ] {
            assert!(parse(request).is_err());
        }
        let graph = parse(json!({"operation":"inspect","include_nodes":true,"node_scope":"graph"}))
            .unwrap();
        assert!(matches!(
            graph,
            Request::Inspect {
                node_scope: node_inspect::Scope::Graph,
                ..
            }
        ));
        assert!(matches!(
            default,
            Request::Inspect {
                node_scope: node_inspect::Scope::Target,
                ..
            }
        ));
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
    fn float_bits_preserve_exact_finite_values_and_reject_nonfinite_or_numeric_inputs() {
        for bits in [0_u64, 0x8000_0000_0000_0000, 0x7fef_ffff_ffff_ffff]
            .into_iter()
            .chain((0..32).map(|n| 0x3fe0_0000_0000_0001_u64 + n))
        {
            let value = json!(format!("{bits:016X}"));
            assert_eq!(decode_float_bits(&value).unwrap().to_bits(), bits);
            assert!(parse(json!({"operation":"set_parameter","param":"gain","source":"dsp","mode":"float_bits","value":value})).is_ok());
        }
        for value in [
            json!(0),
            json!("0"),
            json!("10000000000000000"),
            json!("GG00000000000000"),
            json!("7FF0000000000000"),
            json!("FFF0000000000000"),
            json!("7FF8000000000001"),
        ] {
            assert!(parse(json!({"operation":"set_parameter","param":"gain","mode":"float_bits","value":value})).is_err());
        }
    }
    #[test]
    fn state_request_tokens_are_target_bound_and_cannot_import_bytes_or_paths() {
        let instance = "61D64D66-2A46-4BEE-A623-EC5CEB898313";
        for operation in ["state_capture", "state_restore", "state_release"] {
            let mut request = json!({"operation":operation,"expected_instance":instance});
            if operation != "state_capture" {
                request["token"] = json!("A".repeat(64));
            }
            assert!(parse(request.clone()).is_ok());
            request.as_object_mut().unwrap().remove("expected_instance");
            assert!(parse(request.clone()).is_err());
            request["expected_instance"] = json!(instance);
            request["source"] = json!("alias");
            assert!(parse(request).is_err());
        }
        for request in [
            json!({"operation":"state_capture","expected_instance":instance,"bytes":"AABB"}),
            json!({"operation":"state_restore","expected_instance":instance,"token":"a".repeat(64)}),
            json!({"operation":"state_restore","expected_instance":instance,"token":"A".repeat(65)}),
            json!({"operation":"state_restore","expected_instance":instance,"token":"A".repeat(64),"path":"state.bin"}),
        ] {
            assert!(parse(request).is_err());
        }
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
        assert!(check_float_bits_type(&d).is_err());
        let mut d = d;
        d.kind = parameters::Kind::Integer;
        d.raw = Some(parameters::NativeValue {
            variant_tag: 1,
            value: parameters::Value::Integer(1),
        });
        assert!(check_float_bits_type(&d).is_err());
        d.kind = parameters::Kind::Float;
        d.raw = Some(parameters::NativeValue {
            variant_tag: 2,
            value: parameters::Value::Float(-0.0),
        });
        assert!(check_float_bits_type(&d).is_ok());
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
    unsafe extern "C" fn query_switch_document(
        p: *mut c_void,
        iid: *const Uid,
        out: *mut *mut c_void,
    ) -> i32 {
        unsafe { *out = ptr::null_mut() };
        if unsafe { *iid } != CONTROLLER {
            return NO_INTERFACE;
        }
        unsafe {
            retain(p);
            *out = p;
        }
        let manager = SWITCH_MANAGER.swap(ptr::null_mut(), Ordering::AcqRel);
        if !manager.is_null() {
            unsafe { (*manager).active = ptr::null_mut() };
        }
        OK
    }
    unsafe extern "C" fn query_inspection_identity(
        p: *mut c_void,
        iid: *const Uid,
        out: *mut *mut c_void,
    ) -> i32 {
        unsafe { *out = ptr::null_mut() };
        if unsafe { *iid } != UNKNOWN {
            return NO_INTERFACE;
        }
        unsafe {
            retain(p);
            *out = p;
        }
        OK
    }
    unsafe extern "C" fn inspection_text(_: *const c_char) -> *const c_void {
        ptr::null()
    }
    pub(super) fn inspection_for(context: &Context<'_>) -> node_inspect::Inventory {
        let target = hold(context.target.0).unwrap();
        let pointer = NonNull::new(target.0).unwrap();
        std::mem::forget(target);
        let inspector = unsafe {
            fender_native_node_api::NodeInspector::from_owned(
                pointer,
                fender_native_node_api::TextApi {
                    get_constant_string: inspection_text,
                },
            )
        };
        let result = inspector
            .inspect(fender_native_node_api::Limits::default(), &mut || {
                context.check()
            })
            .unwrap();
        drop(inspector);
        context.check().unwrap();
        node_inspect::Inventory::target(result)
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
            document_context: std::cell::UnsafeCell::new(document_context::Registry::new(1)),
            document_context_pending: AtomicBool::new(false),
            states: std::cell::UnsafeCell::new(state::Registry::new()),
            states_pending: AtomicBool::new(false),
            presets: std::cell::UnsafeCell::new(presets::Registry::new()),
            presets_pending: AtomicBool::new(false),
            insert_chains: std::cell::UnsafeCell::new(insert_chain::Registry::new()),
            insert_chains_pending: AtomicBool::new(false),
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
            plugin_manager: empty,
            file_system: empty,
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
        assert!(ctx.check_document().is_ok());
        manager.active = ptr::null_mut();
        assert!(ctx.check().unwrap_err().contains("document/model"));
        assert!(ctx.check_document().unwrap_err().contains("document/model"));
        manager.active = doc_ptr;
        document.model = ptr::null_mut();
        assert!(ctx.check().unwrap_err().contains("document/model"));
        assert!(ctx.check_document().unwrap_err().contains("document/model"));
        document.model = model_ptr;
        device.uid = UNKNOWN;
        assert!(ctx.check().unwrap_err().contains("target identity"));
        assert!(ctx.check_document().is_ok());
        device.uid = SERVICE_CID;
        device.class = UNKNOWN;
        assert!(ctx.check().unwrap_err().contains("target identity"));
        assert!(ctx.check_document().is_ok());
        device.class = API_CLASS;
        service.terminated.store(true, Ordering::Release);
        assert!(ctx.check().unwrap_err().contains("service"));
        assert!(ctx.check_document().unwrap_err().contains("service"));
        service.terminated.store(false, Ordering::Release);
        READY.store(false, Ordering::Release);
        assert!(ctx.check().is_err());
        assert!(ctx.check_document().is_err());
        READY.store(true, Ordering::Release);
        ctx.started = Instant::now() - Duration::from_secs(3);
        assert!(ctx.check().unwrap_err().contains("budget"));
        assert!(ctx.check_document().unwrap_err().contains("budget"));
        drop(ctx);
        for value in [&manager, &document, &model, &device] {
            assert_eq!(value.refs.load(Ordering::Acquire), 1);
        }
        READY.store(false, Ordering::Release);
    }
    pub(super) struct Fixture {
        _table: Box<[*const c_void; 10]>,
        manager: Box<Mock>,
        document: Box<Mock>,
        model: Box<Mock>,
        device: Box<Mock>,
        service: *mut Service,
    }
    impl Fixture {
        pub(super) fn new() -> Self {
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
        pub(super) fn context(&mut self) -> Box<Context<'static>> {
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
        pub(super) fn canonical_queries(&mut self) {
            self._table[0] = query_inspection_identity as *const c_void;
        }
        pub(super) fn switch_document(&mut self) {
            self.manager.active = ptr::null_mut();
        }
        pub(super) fn switch_model(&mut self) {
            self.document.model = ptr::null_mut();
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
    fn direct_controller_query_cannot_cross_a_document_switch() {
        let _serial = super::super::tests::TEST_LOCK.lock().unwrap();
        let mut fixture = Fixture::new();
        fixture._table[0] = query_switch_document as *const c_void;
        let context = fixture.context();
        let refs = fixture.device.refs.load(Ordering::Acquire);
        SWITCH_MANAGER.store(ptr::addr_of_mut!(*fixture.manager), Ordering::Release);
        let result = direct_controller(&context);
        assert!(matches!(result, Err(ref reason) if reason.contains("document/model")));
        assert_eq!(fixture.device.refs.load(Ordering::Acquire), refs);
        assert!(context.check().is_err());
        drop(context);
    }
    #[test]
    fn previous_receipt_cleanup_is_checked_after_new_context_capture() {
        let _serial = super::super::tests::TEST_LOCK.lock().unwrap();
        let mut fixture = Fixture::new();
        let old = fixture.context();
        retain_receipt(
            fixture.service,
            ReceiptLease {
                _catalog: None,
                _edit: None,
                _action: None,
                _presets: None,
                _owner: None,
                _gui: None,
                _auxiliary: None,
                _folder_preflight: None,
                _nodes: None,
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
    fn numeric_edit_refuses_an_unregistered_class_before_route_discovery() {
        let _serial = super::super::tests::TEST_LOCK.lock().unwrap();
        let mut fixture = Fixture::new();
        let context = fixture.context();
        let (value, lease) = numeric_edit::execute(&context, "chorder.autofill", 0.0);
        assert_eq!(value["ok"], false);
        assert_eq!(value["outcome"], "not_applied");
        assert_eq!(value["error"]["code"], "unregistered_numeric_edit");
        assert!(lease.is_none());
    }
    #[test]
    fn output_assignment_reentry_cannot_publish_a_verified_success() {
        let _serial = super::super::tests::TEST_LOCK.lock().unwrap();
        let mut fixture = Fixture::new();
        let context = fixture.context();
        let reply = Reply {
            value: json!({"ok":true,"verified":true,"outcome":"observed","instance":context.instance,"source":"controller"}),
            catalog: None,
            edit: None,
            action: None,
            presets: None,
            auxiliary: None,
            folder_preflight: None,
            owner: None,
            gui: None,
            nodes: None,
            provider: None,
            context: Some(context),
            mutation: true,
        };
        super::super::tests::ASSIGN_REENTER_STOP.store(fixture.service, Ordering::Release);
        let mut out = VariantOut {
            kind: 0,
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
    #[test]
    fn node_inspection_owners_survive_publication_until_receipt_release() {
        let _serial = super::super::tests::TEST_LOCK.lock().unwrap();
        let mut fixture = Fixture::new();
        fixture._table[0] = query_inspection_identity as *const c_void;
        let context = fixture.context();
        let inspection = inspection_for(&context);
        assert_eq!(inspection.report().nodes.len(), 1);
        assert!(fixture.device.refs.load(Ordering::Acquire) > 3);
        let report = inspection.value();
        let reply = Reply {
            value: json!({"ok":true,"node_report":report,"instance":context.instance,"source":"dsp"}),
            catalog: None,
            edit: None,
            action: None,
            presets: None,
            auxiliary: None,
            folder_preflight: None,
            owner: None,
            gui: None,
            nodes: Some(inspection),
            provider: None,
            context: Some(context),
            mutation: false,
        };
        let mut out = VariantOut {
            kind: 0,
            payload: 0,
        };
        assert!(publish_reply(
            &mut out,
            reply,
            api().unwrap(),
            fixture.service
        ));
        let value: Value = serde_json::from_str(&super::super::tests::output_text(&out)).unwrap();
        assert_eq!(value["ok"], true);
        assert_eq!(LIVE_RECEIPTS.load(Ordering::Acquire), 1);
        assert!(fixture.device.refs.load(Ordering::Acquire) > 3);
        release_receipt(fixture.service);
        assert_eq!(fixture.device.refs.load(Ordering::Acquire), 1);
        assert_eq!(LIVE_RECEIPTS.load(Ordering::Acquire), 0);
        super::super::tests::clear_output(&mut out);
    }
    #[test]
    fn node_inspection_rejects_a_document_switch_before_publication() {
        let _serial = super::super::tests::TEST_LOCK.lock().unwrap();
        let mut fixture = Fixture::new();
        fixture._table[0] = query_inspection_identity as *const c_void;
        let context = fixture.context();
        let inspection = inspection_for(&context);
        let mut reply = Reply {
            value: json!({"ok":true,"instance":context.instance,"source":"dsp"}),
            catalog: None,
            edit: None,
            action: None,
            presets: None,
            auxiliary: None,
            folder_preflight: None,
            owner: None,
            gui: None,
            nodes: Some(inspection),
            provider: None,
            context: Some(context),
            mutation: false,
        };
        fixture.manager.active = ptr::null_mut();
        assert!(!reply.final_check(fixture.service));
        assert_eq!(reply.value["ok"], false);
        assert_eq!(reply.value["outcome"], "not_applied");
        drop(reply);
        assert_eq!(fixture.device.refs.load(Ordering::Acquire), 1);
    }
    #[test]
    fn auxiliary_metadata_is_readonly_and_retained_through_receipt_publication() {
        let _serial = super::super::tests::TEST_LOCK.lock().unwrap();
        let mut fixture = Fixture::new();
        fixture.canonical_queries();
        let context = fixture.context();
        let auxiliary = inspection_for(&context).into_auxiliary();
        let report = auxiliary.value();
        assert_eq!(report["scope"], "auxiliary_type_observation");
        assert_eq!(report["mutation_capability"], false);
        assert_eq!(report["limits"]["nodes"], 1);
        assert_eq!(report["limits"]["ancestors"], 0);
        assert!(report["nodes"][0]["identity"].is_null());
        assert!(report.get("instance").is_none());
        let reply = Reply {
            value: json!({"ok":true,"outcome":"observed","auxiliary_report":report}),
            catalog: None,
            edit: None,
            action: None,
            presets: None,
            auxiliary: Some(auxiliary),
            folder_preflight: None,
            owner: None,
            gui: None,
            nodes: None,
            provider: None,
            context: Some(context),
            mutation: false,
        };
        let mut out = VariantOut {
            kind: 0,
            payload: 0,
        };
        assert!(publish_reply(
            &mut out,
            reply,
            api().unwrap(),
            fixture.service
        ));
        assert_eq!(LIVE_RECEIPTS.load(Ordering::Acquire), 1);
        assert!(fixture.device.refs.load(Ordering::Acquire) > 3);
        release_receipt(fixture.service);
        assert_eq!(fixture.device.refs.load(Ordering::Acquire), 1);
        super::super::tests::clear_output(&mut out);
        let context = fixture.context();
        let auxiliary = inspection_for(&context).into_auxiliary();
        let mut reply = Reply {
            value: json!({"ok":true,"outcome":"observed"}),
            catalog: None,
            edit: None,
            action: None,
            presets: None,
            auxiliary: Some(auxiliary),
            folder_preflight: None,
            owner: None,
            gui: None,
            nodes: None,
            provider: None,
            context: Some(context),
            mutation: false,
        };
        fixture.switch_document();
        assert!(!reply.final_check(fixture.service));
        assert_eq!(reply.value["outcome"], "not_applied");
        drop(reply);
        assert_eq!(fixture.device.refs.load(Ordering::Acquire), 1);
    }
    #[test]
    fn ancestry_observation_keeps_its_inspection_through_publication_and_rejects_document_switch() {
        let _serial = super::super::tests::TEST_LOCK.lock().unwrap();
        let mut fixture = Fixture::new();
        fixture.canonical_queries();
        let context = fixture.context();
        let node_inspect::Auxiliary::TypeMetadata(inspection) =
            inspection_for(&context).into_auxiliary()
        else {
            unreachable!()
        };
        let auxiliary = node_inspect::Auxiliary::InsertFolderAncestry {
            inspection,
            ancestor_matches_active_model: false,
            document_environment: node_inspect::DocumentEnvironmentObservation::Unavailable(
                "synthetic legacy ancestry".into(),
            ),
        };
        let report = auxiliary.value();
        assert_eq!(report["scope"], "insert_folder_ancestry_observation");
        assert_eq!(report["ancestor_matches_active_model"], false);
        assert_eq!(report["atomic_snapshot"], false);
        assert_eq!(report["mutation_capability"], false);
        assert_eq!(report["traversal"], "parent_chain_only");
        assert_eq!(report["descendants_observed"], false);
        assert_eq!(
            report["limits"],
            json!({"nodes":9,"depth":0,"ancestors":8,"children_per_node":1})
        );
        let reply = Reply {
            value: json!({"ok":true,"outcome":"observed","scope":"insert_folder_ancestry_observation",
                "ancestor_matches_active_model":false,"insert_folder_ancestry_report":report}),
            catalog: None,
            edit: None,
            action: None,
            presets: None,
            auxiliary: Some(auxiliary),
            folder_preflight: None,
            owner: None,
            gui: None,
            nodes: None,
            provider: None,
            context: Some(context),
            mutation: false,
        };
        let mut out = VariantOut {
            kind: 0,
            payload: 0,
        };
        assert!(publish_reply(
            &mut out,
            reply,
            api().unwrap(),
            fixture.service
        ));
        let value: Value = serde_json::from_str(&super::super::tests::output_text(&out)).unwrap();
        assert_eq!(value["ok"], true);
        assert_eq!(value["ancestor_matches_active_model"], false);
        assert!(fixture.device.refs.load(Ordering::Acquire) > 3);
        assert_eq!(LIVE_RECEIPTS.load(Ordering::Acquire), 1);
        release_receipt(fixture.service);
        assert_eq!(fixture.device.refs.load(Ordering::Acquire), 1);
        super::super::tests::clear_output(&mut out);

        let context = fixture.context();
        let node_inspect::Auxiliary::TypeMetadata(inspection) =
            inspection_for(&context).into_auxiliary()
        else {
            unreachable!()
        };
        let auxiliary = node_inspect::Auxiliary::InsertFolderAncestry {
            inspection,
            ancestor_matches_active_model: false,
            document_environment: node_inspect::DocumentEnvironmentObservation::Unavailable(
                "synthetic legacy ancestry".into(),
            ),
        };
        // The node lease still rejects a host-context switch after all retained
        // observations exist, before a predicate can be published.
        fixture.switch_document();
        assert!(auxiliary.check(&context).is_err());
        drop(auxiliary);
        drop(context);
        assert_eq!(fixture.device.refs.load(Ordering::Acquire), 1);
    }
    #[test]
    fn digest_publication_reentry_strips_every_hash() {
        let _serial = super::super::tests::TEST_LOCK.lock().unwrap();
        let mut fixture = Fixture::new();
        let reply = Reply {
            value: json!({"ok":true,"outcome":"observed","hash_format_version":"fsp-parameter-digest-v1",
                "schema_sha256":"a".repeat(64),"all_values_sha256":"b".repeat(64),"non_read_only_values_sha256":"c".repeat(64),"access_complete":true}),
            catalog: None,
            edit: None,
            action: None,
            presets: None,
            auxiliary: None,
            folder_preflight: None,
            owner: None,
            gui: None,
            nodes: None,
            provider: None,
            context: Some(fixture.context()),
            mutation: false,
        };
        super::super::tests::ASSIGN_REENTER_STOP.store(fixture.service, Ordering::Release);
        let mut out = VariantOut {
            kind: 0,
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
        assert_eq!(value["outcome"], "not_applied");
        assert_eq!(value["access_complete"], false);
        for key in [
            "schema_sha256",
            "all_values_sha256",
            "non_read_only_values_sha256",
        ] {
            assert!(value.get(key).is_none());
        }
        assert_eq!(LIVE_RECEIPTS.load(Ordering::Acquire), 0);
        super::super::tests::clear_output(&mut out);
    }
}

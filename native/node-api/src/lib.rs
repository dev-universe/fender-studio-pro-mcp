//! Read-only CCL 5.1.2 node and reflected type inventory. No invocation API.
#![deny(unsafe_op_in_unsafe_fn)]
mod ffi;
#[cfg(test)]
mod tests;

use ffi::{Owned, PlainString, Uid};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    ffi::{CStr, c_char, c_void},
    ptr::NonNull,
    thread::{self, ThreadId},
};

pub const CCL_ABI_VERSION: &str = "5.1.2";
const MAX_CSTRING: usize = 512;
const MAX_STRING_UNITS: usize = 512;
const MAX_STRING_BYTES: usize = 48 * 1024;
const MAX_TYPES: usize = 64;
const MAX_TYPE_DEPTH: usize = 16;
const MAX_METHODS: usize = 256;
const MAX_PROPERTIES: usize = 256;

#[derive(Clone, Copy)]
pub struct TextApi {
    pub get_constant_string: unsafe extern "C" fn(*const c_char) -> *const c_void,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    WrongThread,
    InvalidLimits,
    Guard(String),
    MissingIdentity,
    InvalidData(&'static str),
    TextTooLong,
    InvalidText,
    AttachmentChanged,
    Reentrant,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub nodes: usize,
    pub depth: u8,
    pub ancestors: u8,
    pub children_per_node: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            nodes: 64,
            depth: 2,
            ancestors: 3,
            children_per_node: 32,
        }
    }
}
impl Limits {
    fn valid(self) -> bool {
        (1..=128).contains(&self.nodes)
            && self.depth <= 8
            && self.ancestors <= 8
            && (1..=128).contains(&self.children_per_node)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Delegate {
    NodeController,
    Slot,
    Device,
    Graph,
    Combinator,
    Presets,
    RoutingModel,
    Inserts,
}
impl Delegate {
    pub const ALL: [Self; 8] = [
        Self::NodeController,
        Self::Slot,
        Self::Device,
        Self::Graph,
        Self::Combinator,
        Self::Presets,
        Self::RoutingModel,
        Self::Inserts,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::NodeController => "NodeController",
            Self::Slot => "Slot",
            Self::Device => "Device",
            Self::Graph => "Graph",
            Self::Combinator => "Combinator",
            Self::Presets => "Presets",
            Self::RoutingModel => "RoutingModel",
            Self::Inserts => "Inserts",
        }
    }
    fn c_name(self) -> &'static CStr {
        match self {
            Self::NodeController => c"NodeController",
            Self::Slot => c"Slot",
            Self::Device => c"Device",
            Self::Graph => c"Graph",
            Self::Combinator => c"Combinator",
            Self::Presets => c"Presets",
            Self::RoutingModel => c"RoutingModel",
            Self::Inserts => c"Inserts",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Relation {
    Parent,
    Child(u32),
    Delegate(Delegate),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub relation: Relation,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeIdentity {
    pub id: String,
    pub object_uid: Option<String>,
    pub class_uid: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Method {
    pub name: String,
    pub arguments: Option<String>,
    pub return_type: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Property {
    pub name: String,
    pub data_type: i32,
    pub type_name: Option<String>,
    pub has_type_info: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeInfo {
    pub name: String,
    pub namespace: Option<String>,
    pub class_uid: Option<String>,
    pub flags: i32,
    pub methods: Vec<Method>,
    pub properties: Vec<Property>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetadataIssue {
    NoObjectInterface,
    TypeCycle,
    TypeDepth,
    TypeLimit,
    MethodLimit,
    PropertyLimit,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeInventory {
    pub available: bool,
    pub types: Vec<TypeInfo>,
    pub issues: Vec<MetadataIssue>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeRecord {
    pub index: usize,
    pub identity: Option<NodeIdentity>,
    pub child_count: Option<u32>,
    pub reflection: TypeInventory,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Diagnostic {
    NodeLimit,
    DepthLimit(usize),
    ChildLimit(usize),
    AncestorLimit,
    NodeCycle { from: usize, to: usize },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub nodes: Vec<NodeRecord>,
    pub edges: Vec<Edge>,
    pub diagnostics: Vec<Diagnostic>,
    pub complete: bool,
}

/// Retains the native attachment witnesses until the caller publishes a report.
///
/// ```compile_fail
/// use fender_native_node_api::Inspection;
/// fn send(value: Inspection) { std::thread::spawn(move || drop(value)); }
/// ```
/// ```compile_fail
/// use fender_native_node_api::Inspection;
/// fn needs_sync<T: Sync>() {}
/// needs_sync::<Inspection>();
/// ```
pub struct Inspection {
    report: Report,
    pins: RefCell<Option<Vec<Pin>>>,
    text: TextApi,
    thread: ThreadId,
}
impl std::fmt::Debug for Inspection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.report.fmt(f)
    }
}
impl std::ops::Deref for Inspection {
    type Target = Report;
    fn deref(&self) -> &Report {
        &self.report
    }
}
impl Inspection {
    pub fn report(&self) -> &Report {
        &self.report
    }

    /// Revalidate the retained witnesses immediately before publication.
    pub fn check<G: FnMut() -> Result<(), String>>(&self, guard: &mut G) -> Result<(), Error> {
        if thread::current().id() != self.thread {
            return Err(Error::WrongThread);
        }
        let pins = self
            .pins
            .try_borrow_mut()
            .map_err(|_| Error::Reentrant)?
            .take()
            .ok_or(Error::Reentrant)?;
        let mut session = Session {
            text: self.text,
            limits: Limits::default(),
            guard,
            pins,
            seen: HashMap::new(),
            expanded: HashSet::new(),
            budget: Budget::default(),
            report: self.report.clone(),
        };
        let result = session.validate();
        let pins = std::mem::take(&mut session.pins);
        drop(session);
        let final_guard = guard().map_err(Error::Guard);
        *self.pins.borrow_mut() = Some(pins);
        final_guard?;
        result
    }
}

/// Owns one caller-resolved object on its validated GUI thread.
///
/// ```compile_fail
/// use fender_native_node_api::NodeInspector;
/// fn send(value: NodeInspector) { std::thread::spawn(move || drop(value)); }
/// ```
/// ```compile_fail
/// use fender_native_node_api::NodeInspector;
/// fn needs_sync<T: Sync>() {}
/// needs_sync::<NodeInspector>();
/// ```
pub struct NodeInspector {
    root: Owned,
    text: TextApi,
    thread: ThreadId,
}
impl NodeInspector {
    /// Takes one owned IUnknown-compatible reference.
    ///
    /// # Safety
    /// The caller must verify the CCL5.1.2 macOS arm64 ABI, GUI thread, live
    /// object and text export lifetime. Metadata getters must honor their public
    /// pointer, ownership and null-termination contracts. The caller retains its
    /// document/model/target lease until this wrapper is dropped, then checks it.
    pub unsafe fn from_owned(root: NonNull<c_void>, text: TextApi) -> Self {
        Self {
            root: unsafe { Owned::take(root) },
            text,
            thread: thread::current().id(),
        }
    }

    pub fn inspect<G: FnMut() -> Result<(), String>>(
        &self,
        limits: Limits,
        guard: &mut G,
    ) -> Result<Inspection, Error> {
        if thread::current().id() != self.thread {
            return Err(Error::WrongThread);
        }
        if !limits.valid() {
            return Err(Error::InvalidLimits);
        }
        guard().map_err(Error::Guard)?;
        let mut session = Session {
            text: self.text,
            limits,
            guard,
            pins: Vec::new(),
            seen: HashMap::new(),
            expanded: HashSet::new(),
            budget: Budget::default(),
            report: Report {
                nodes: vec![],
                edges: vec![],
                diagnostics: vec![],
                complete: false,
            },
        };
        let result = session.run(self.root.raw()).map(|report| Inspection {
            report,
            pins: RefCell::new(Some(std::mem::take(&mut session.pins))),
            text: self.text,
            thread: self.thread,
        });
        // Temporary reflection/String releases finish before the final guard.
        // Discovered node owners remain in the returned receipt lease.
        drop(session);
        if let Err(error) = guard() {
            drop(result);
            guard().map_err(Error::Guard)?;
            return Err(Error::Guard(error));
        }
        result
    }
}

#[derive(Default)]
struct Budget {
    bytes: usize,
    types: usize,
    methods: usize,
    properties: usize,
}
struct Pin {
    _object: Owned,
    canonical: Owned,
    node: Option<Owned>,
}
struct Session<'a, G> {
    text: TextApi,
    limits: Limits,
    guard: &'a mut G,
    pins: Vec<Pin>,
    seen: HashMap<usize, usize>,
    expanded: HashSet<usize>,
    budget: Budget,
    report: Report,
}
impl<G: FnMut() -> Result<(), String>> Session<'_, G> {
    fn check(&mut self) -> Result<(), Error> {
        (self.guard)().map_err(Error::Guard)
    }
    fn hold(&mut self, raw: *mut c_void) -> Result<Option<Owned>, Error> {
        // A getter's borrowed result must be pinned before a guard can reenter.
        let result = unsafe { Owned::hold(raw) };
        if let Err(error) = self.check() {
            drop(result);
            self.check()?;
            return Err(error);
        }
        Ok(result)
    }
    fn query(&mut self, raw: *mut c_void, iid: &Uid) -> Result<Option<Owned>, Error> {
        self.check()?;
        let result = unsafe { ffi::query(raw, iid) };
        if let Err(error) = self.check() {
            drop(result);
            self.check()?;
            return Err(error);
        }
        Ok(result)
    }
    fn charge(&mut self, text: &str) -> Result<(), Error> {
        self.budget.bytes = self
            .budget
            .bytes
            .checked_add(text.len())
            .ok_or(Error::TextTooLong)?;
        if self.budget.bytes > MAX_STRING_BYTES {
            Err(Error::TextTooLong)
        } else {
            Ok(())
        }
    }
    fn cstring(&mut self, pointer: *const c_char) -> Result<Option<String>, Error> {
        if pointer.is_null() {
            return Ok(None);
        }
        let mut bytes = Vec::new();
        for i in 0..=MAX_CSTRING {
            let byte = unsafe { *pointer.add(i) } as u8;
            if byte == 0 {
                let text = String::from_utf8(bytes).map_err(|_| Error::InvalidText)?;
                self.charge(&text)?;
                return Ok(Some(text));
            }
            if i == MAX_CSTRING {
                return Err(Error::TextTooLong);
            }
            bytes.push(byte);
        }
        Err(Error::TextTooLong)
    }
    fn string(&mut self, reference: *const PlainString) -> Result<String, Error> {
        if reference.is_null() {
            return Err(Error::InvalidData("null String reference"));
        }
        let raw = unsafe { (*reference).object };
        let owner = self
            .hold(raw)?
            .ok_or(Error::InvalidData("null String owner"))?;
        let result = (|| {
            self.check()?;
            let length: unsafe extern "C" fn(*mut c_void) -> i32 =
                unsafe { ffi::method(owner.raw(), 4) };
            let n = unsafe { length(owner.raw()) };
            self.check()?;
            if !(0..=MAX_STRING_UNITS as i32).contains(&n) {
                return Err(Error::TextTooLong);
            }
            let mut units = vec![0xffff; n as usize + 1];
            let copy: unsafe extern "C" fn(*mut c_void, *mut u16, i32) -> i32 =
                unsafe { ffi::method(owner.raw(), 8) };
            self.check()?;
            let status = unsafe { copy(owner.raw(), units.as_mut_ptr(), n + 1) };
            self.check()?;
            if status != 0 || units[n as usize] != 0 {
                return Err(Error::InvalidData("String copy failed"));
            }
            self.check()?;
            let after = unsafe { length(owner.raw()) };
            self.check()?;
            if after != n {
                return Err(Error::InvalidData("String changed during copy"));
            }
            let text = String::from_utf16(&units[..n as usize]).map_err(|_| Error::InvalidText)?;
            self.charge(&text)?;
            Ok(text)
        })();
        drop(owner);
        self.check()?;
        result
    }
    fn uid_at(&mut self, raw: *mut c_void, slot: usize) -> Result<Option<String>, Error> {
        self.check()?;
        let get: unsafe extern "C" fn(*mut c_void) -> *const Uid =
            unsafe { ffi::method(raw, slot) };
        let pointer = unsafe { get(raw) };
        if pointer.is_null() {
            return Err(Error::InvalidData("null UID reference"));
        }
        let value = unsafe { ffi::uid(pointer) };
        self.check()?;
        Ok(value)
    }
    fn identity(&mut self, raw: *mut c_void) -> Result<NodeIdentity, Error> {
        self.check()?;
        let get: unsafe extern "C" fn(*mut c_void) -> *const PlainString =
            unsafe { ffi::method(raw, 3) };
        let reference = unsafe { get(raw) };
        let id = self.string(reference)?;
        Ok(NodeIdentity {
            id,
            object_uid: self.uid_at(raw, 4)?,
            class_uid: self.uid_at(raw, 5)?,
        })
    }
    fn child_count(&mut self, raw: *mut c_void) -> Result<u32, Error> {
        self.check()?;
        let get: unsafe extern "C" fn(*mut c_void) -> i32 = unsafe { ffi::method(raw, 8) };
        let count = unsafe { get(raw) };
        self.check()?;
        if !(0..=1_000_000).contains(&count) {
            return Err(Error::InvalidData("invalid node child count"));
        }
        Ok(count as u32)
    }
    fn reflection(&mut self, raw: *mut c_void) -> Result<TypeInventory, Error> {
        let Some(object) = self.query(raw, &ffi::OBJECT)? else {
            return Ok(TypeInventory {
                available: false,
                types: vec![],
                issues: vec![MetadataIssue::NoObjectInterface],
            });
        };
        let result = (|| {
            self.check()?;
            let get: unsafe extern "C" fn(*mut c_void) -> *mut c_void =
                unsafe { ffi::method(object.raw(), 3) };
            let first = unsafe { get(object.raw()) };
            let mut current = self
                .hold(first)?
                .ok_or(Error::InvalidData("null type reference"))?;
            let mut seen = HashSet::new();
            let mut inventory = TypeInventory {
                available: true,
                types: vec![],
                issues: vec![],
            };
            let result = (|| {
                loop {
                    if !seen.insert(current.raw() as usize) {
                        inventory.issues.push(MetadataIssue::TypeCycle);
                        break;
                    }
                    if inventory.types.len() >= MAX_TYPE_DEPTH {
                        inventory.issues.push(MetadataIssue::TypeDepth);
                        break;
                    }
                    if self.budget.types >= MAX_TYPES {
                        inventory.issues.push(MetadataIssue::TypeLimit);
                        break;
                    }
                    self.budget.types += 1;
                    inventory
                        .types
                        .push(self.type_record(current.raw(), &mut inventory.issues)?);
                    self.check()?;
                    let parent: unsafe extern "C" fn(*mut c_void) -> *mut c_void =
                        unsafe { ffi::method(current.raw(), 3) };
                    let pointer = unsafe { parent(current.raw()) };
                    let next = self.hold(pointer)?;
                    let old = std::mem::replace(
                        &mut current,
                        match next {
                            Some(next) => next,
                            None => break,
                        },
                    );
                    drop(old);
                    self.check()?;
                }
                Ok(inventory)
            })();
            drop(current);
            self.check()?;
            result
        })();
        drop(object);
        self.check()?;
        result
    }
    fn type_record(
        &mut self,
        raw: *mut c_void,
        issues: &mut Vec<MetadataIssue>,
    ) -> Result<TypeInfo, Error> {
        self.check()?;
        let flags: unsafe extern "C" fn(*mut c_void) -> i32 = unsafe { ffi::method(raw, 4) };
        let flags = unsafe { flags(raw) };
        self.check()?;
        let name: unsafe extern "C" fn(*mut c_void) -> *const c_char =
            unsafe { ffi::method(raw, 5) };
        let pointer = unsafe { name(raw) };
        let name = self
            .cstring(pointer)?
            .ok_or(Error::InvalidData("null type name"))?;
        self.check()?;
        let namespace: unsafe extern "C" fn(*mut c_void) -> *const c_char =
            unsafe { ffi::method(raw, 6) };
        let pointer = unsafe { namespace(raw) };
        let namespace = self.cstring(pointer)?;
        let class_uid = self.uid_at(raw, 7)?;
        self.check()?;
        let methods: unsafe extern "C" fn(*mut c_void) -> *const ffi::MethodDefinition =
            unsafe { ffi::method(raw, 8) };
        let pointer = unsafe { methods(raw) };
        let mut methods = Vec::new();
        if !pointer.is_null() {
            let available = MAX_METHODS - self.budget.methods;
            let mut terminated = false;
            for i in 0..available {
                let record = unsafe { *pointer.add(i) };
                if record.name.is_null() {
                    terminated = true;
                    break;
                }
                methods.push(Method {
                    name: self
                        .cstring(record.name)?
                        .ok_or(Error::InvalidData("null method name"))?,
                    arguments: self.cstring(record.args)?,
                    return_type: self.cstring(record.retval)?,
                });
                self.budget.methods += 1;
            }
            if !terminated {
                issues.push(MetadataIssue::MethodLimit);
            }
        }
        self.check()?;
        let properties: unsafe extern "C" fn(*mut c_void) -> *const ffi::PropertyDefinition =
            unsafe { ffi::method(raw, 9) };
        let pointer = unsafe { properties(raw) };
        let mut properties = Vec::new();
        if !pointer.is_null() {
            let available = MAX_PROPERTIES - self.budget.properties;
            let mut terminated = false;
            for i in 0..available {
                let record = unsafe { *pointer.add(i) };
                if record.name.is_null() {
                    terminated = true;
                    break;
                }
                properties.push(Property {
                    name: self
                        .cstring(record.name)?
                        .ok_or(Error::InvalidData("null property name"))?,
                    data_type: record.data_type,
                    type_name: self.cstring(record.type_name)?,
                    has_type_info: !record.type_info.is_null(),
                });
                self.budget.properties += 1;
            }
            if !terminated {
                issues.push(MetadataIssue::PropertyLimit);
            }
        }
        self.check()?;
        Ok(TypeInfo {
            name,
            namespace,
            class_uid,
            flags,
            methods,
            properties,
        })
    }
    fn intern(&mut self, object: Owned) -> Result<Option<usize>, Error> {
        let canonical = self
            .query(object.raw(), &ffi::UNKNOWN)?
            .ok_or(Error::MissingIdentity)?;
        if let Some(index) = self.seen.get(&(canonical.raw() as usize)).copied() {
            drop(canonical);
            drop(object);
            self.check()?;
            return Ok(Some(index));
        }
        if self.pins.len() >= self.limits.nodes {
            drop(canonical);
            drop(object);
            self.check()?;
            if !self.report.diagnostics.contains(&Diagnostic::NodeLimit) {
                self.report.diagnostics.push(Diagnostic::NodeLimit);
            }
            return Ok(None);
        }
        let node = self.query(object.raw(), &ffi::NODE)?;
        let identity = node
            .as_ref()
            .map(|node| node.raw())
            .map(|raw| self.identity(raw))
            .transpose()?;
        let child_count = node
            .as_ref()
            .map(|node| node.raw())
            .map(|raw| self.child_count(raw))
            .transpose()?;
        let reflection = self.reflection(object.raw())?;
        let index = self.pins.len();
        self.seen.insert(canonical.raw() as usize, index);
        self.pins.push(Pin {
            _object: object,
            canonical,
            node,
        });
        self.report.nodes.push(NodeRecord {
            index,
            identity,
            child_count,
            reflection,
        });
        Ok(Some(index))
    }
    fn related(&mut self, from: usize, relation: Relation) -> Result<Option<Owned>, Error> {
        let Some(node) = self.pins[from].node.as_ref() else {
            return Ok(None);
        };
        let raw = node.raw();
        self.check()?;
        let result = match relation {
            Relation::Parent => {
                let get: unsafe extern "C" fn(*mut c_void) -> *mut c_void =
                    unsafe { ffi::method(raw, 6) };
                unsafe { get(raw) }
            }
            Relation::Child(index) => {
                let get: unsafe extern "C" fn(*mut c_void, i32) -> *mut c_void =
                    unsafe { ffi::method(raw, 9) };
                unsafe { get(raw, index as i32) }
            }
            Relation::Delegate(delegate) => {
                let name = unsafe { (self.text.get_constant_string)(delegate.c_name().as_ptr()) };
                self.check()?;
                if name.is_null() {
                    return Err(Error::InvalidData("native String constant unavailable"));
                }
                let find: unsafe extern "C" fn(*mut c_void, *const c_void) -> *mut c_void =
                    unsafe { ffi::method(raw, 10) };
                unsafe { find(raw, name) }
            }
        };
        self.hold(result)
    }
    fn connect(&mut self, from: usize, relation: Relation) -> Result<Option<usize>, Error> {
        let Some(object) = self.related(from, relation)? else {
            if matches!(relation, Relation::Child(_)) {
                return Err(Error::InvalidData("reported child is unavailable"));
            }
            return Ok(None);
        };
        let Some(to) = self.intern(object)? else {
            return Ok(None);
        };
        let edge = Edge { from, to, relation };
        if !self.report.edges.contains(&edge) {
            self.report.edges.push(edge);
        }
        Ok(Some(to))
    }
    fn walk(&mut self, from: usize, depth: u8, path: &mut Vec<usize>) -> Result<(), Error> {
        if !self.expanded.insert(from) {
            return Ok(());
        }
        let Some(count) = self.report.nodes[from].child_count else {
            return Ok(());
        };
        let mut relations: Vec<Relation> = (0..count.min(self.limits.children_per_node as u32))
            .map(Relation::Child)
            .collect();
        if count > self.limits.children_per_node as u32 {
            self.report.diagnostics.push(Diagnostic::ChildLimit(from));
        }
        relations.extend(Delegate::ALL.into_iter().map(Relation::Delegate));
        path.push(from);
        for relation in relations {
            if depth == 0 {
                let related = self.related(from, relation)?;
                let present = related.is_some();
                drop(related);
                self.check()?;
                if !present && matches!(relation, Relation::Child(_)) {
                    return Err(Error::InvalidData("reported child is unavailable"));
                }
                if present
                    && !self
                        .report
                        .diagnostics
                        .contains(&Diagnostic::DepthLimit(from))
                {
                    self.report.diagnostics.push(Diagnostic::DepthLimit(from));
                }
                continue;
            }
            if let Some(to) = self.connect(from, relation)? {
                if path.contains(&to) {
                    self.report
                        .diagnostics
                        .push(Diagnostic::NodeCycle { from, to });
                } else {
                    self.walk(to, depth - 1, path)?;
                }
            }
        }
        path.pop();
        Ok(())
    }
    fn validate(&mut self) -> Result<(), Error> {
        for index in 0..self.pins.len() {
            if let Some(node) = self.pins[index].node.as_ref() {
                let raw = node.raw();
                if Some(self.child_count(raw)?) != self.report.nodes[index].child_count
                    || Some(self.identity(raw)?) != self.report.nodes[index].identity
                {
                    return Err(Error::AttachmentChanged);
                }
            }
        }
        for edge in self.report.edges.clone() {
            let owner = self
                .related(edge.from, edge.relation)?
                .ok_or(Error::AttachmentChanged)?;
            let identity = self
                .query(owner.raw(), &ffi::UNKNOWN)?
                .ok_or(Error::MissingIdentity)?;
            let same = identity.raw() == self.pins[edge.to].canonical.raw();
            drop(identity);
            drop(owner);
            self.check()?;
            if !same {
                return Err(Error::AttachmentChanged);
            }
        }
        self.check()
    }
    fn run(&mut self, root: *mut c_void) -> Result<Report, Error> {
        let owner = self.hold(root)?.ok_or(Error::MissingIdentity)?;
        let root = self.intern(owner)?.ok_or(Error::InvalidLimits)?;
        let mut ancestors = vec![root];
        let mut from = root;
        for _ in 0..self.limits.ancestors {
            let Some(to) = self.connect(from, Relation::Parent)? else {
                break;
            };
            if ancestors.contains(&to) {
                self.report
                    .diagnostics
                    .push(Diagnostic::NodeCycle { from, to });
                break;
            }
            ancestors.push(to);
            from = to;
        }
        if ancestors.len() == usize::from(self.limits.ancestors) + 1 {
            let related = self.related(from, Relation::Parent)?;
            let present = related.is_some();
            drop(related);
            self.check()?;
            if present {
                self.report.diagnostics.push(Diagnostic::AncestorLimit);
            }
        }
        for index in ancestors {
            self.walk(index, self.limits.depth, &mut vec![])?;
        }
        self.validate()?;
        self.report.complete = self.report.diagnostics.is_empty()
            && self
                .report
                .nodes
                .iter()
                .all(|n| n.reflection.issues.is_empty());
        Ok(self.report.clone())
    }
}

//! Fixed current-selector flat models. No item interaction or model mutation.
use super::*;

const ITEM_MODEL: Uid = Uid {
    a: 0x5a8661f7,
    b: 0xcc02,
    c: 0x4432,
    d: [0xbb, 0xcd, 0x98, 0xf9, 0x92, 0x9d, 0x9c, 0xfa],
};
pub(crate) const MAX_ITEMS: u32 = 4096;
pub(crate) const MAX_PAGE: u32 = 32;
const MAX_TITLE: usize = 512;
const MAX_TEXT: usize = 16384;
pub(crate) fn default_limit() -> u32 {
    MAX_PAGE
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Kind {
    Categories,
    Effects,
}
impl Kind {
    fn name(self) -> &'static std::ffi::CStr {
        match self {
            Self::Categories => c"categoryModel",
            Self::Effects => c"effectListModel",
        }
    }
}

// Independently declared interoperability carriers, compiler-pinned to CCL5.1.2.
#[repr(C)]
struct ItemIndex {
    tag: i16,
    padding: [u8; 6],
    index: i64,
}
impl ItemIndex {
    fn row(index: u32) -> Self {
        Self {
            tag: 0,
            padding: [0; 6],
            index: i64::from(index),
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<ItemIndex>() == 16 && std::mem::align_of::<ItemIndex>() == 8);
    assert!(
        std::mem::offset_of!(ItemIndex, tag) == 0 && std::mem::offset_of!(ItemIndex, index) == 8
    );
};

struct Title(PlainString);
impl Title {
    fn new<G: FnMut() -> Result<(), String>>(
        text: ConstantString,
        guard: &mut G,
    ) -> Result<Self, String> {
        guard()?;
        let source =
            unsafe { text(c"".as_ptr()).as_ref() }.ok_or("model title empty String unavailable")?;
        let value = *source;
        // This is a constructed String copy: own its current IString. The OUT
        // assignment may release/replace it. Opaque debugger bytes stay opaque.
        let retained = hold(value.object).ok_or("model title empty IString unavailable")?;
        std::mem::forget(retained);
        let title = Self(value);
        if let Err(reason) = guard() {
            release(title, guard)?;
            return Err(reason);
        }
        Ok(title)
    }
    fn copy<G: FnMut() -> Result<(), String>>(
        &self,
        guard: &mut G,
    ) -> Result<(String, usize), String> {
        guard()?;
        let object = self.0.object;
        if object.is_null() {
            return Err("model title returned an invalid String".into());
        }
        let length: unsafe extern "C" fn(*mut c_void) -> i32 = unsafe { method(object, 4) };
        let before = unsafe { length(object) };
        guard()?;
        if before < 0 || before as usize > MAX_TITLE {
            return Err("model title exceeds512 UTF-16 units".into());
        }
        let mut units = vec![0xffff; before as usize + 1];
        let copy: unsafe extern "C" fn(*mut c_void, *mut u16, i32) -> i32 =
            unsafe { method(object, 8) };
        let status = unsafe { copy(object, units.as_mut_ptr(), units.len() as i32) };
        guard()?;
        let after = unsafe { length(object) };
        guard()?;
        if status != OK || before != after || units[before as usize] != 0 {
            return Err("model title changed or was not terminated".into());
        }
        let value = String::from_utf16(&units[..before as usize])
            .map_err(|_| "invalid model title UTF-16")?;
        Ok((value, before as usize))
    }
}
impl Drop for Title {
    fn drop(&mut self) {
        if !self.0.object.is_null() {
            unsafe { (unknown_vtable(self.0.object).release)(self.0.object) };
        }
    }
}

struct NodePin {
    node: Owned,
    identity: Owned,
}
impl NodePin {
    fn capture<G: FnMut() -> Result<(), String>>(
        node: Owned,
        guard: &mut G,
    ) -> Result<Self, String> {
        let identity = interface(node.0, &UNKNOWN, guard)?;
        Ok(Self { node, identity })
    }
}
struct RoutePins(Vec<NodePin>);
impl std::ops::Deref for RoutePins {
    type Target = [NodePin];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Drop for RoutePins {
    fn drop(&mut self) {
        while self.0.pop().is_some() {}
    }
}
type Find = unsafe extern "C" fn(*mut c_void, *const PlainString) -> *mut c_void;
type GetObject = unsafe extern "C" fn(*mut c_void, *const PlainCString, *const Uid) -> *mut c_void;
fn find(parent: *mut c_void, name: *const PlainString) -> *mut c_void {
    let get: Find = unsafe { method(parent, 10) };
    unsafe { get(parent, name) }
}
fn object(controller: *mut c_void, name: *const PlainCString) -> *mut c_void {
    let get: GetObject = unsafe { method(controller, 7) };
    unsafe { get(controller, name, &ITEM_MODEL) }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct Row {
    index: u32,
    title: String,
    title_available: bool,
}

pub(crate) struct Catalog {
    // Children are dropped before the route and its owning alias/document.
    model: Owned,
    model_identity: Owned,
    object: Owned,
    controller: Owned,
    nodes: RoutePins,
    binding: ControllerBinding,
    names: [*const PlainString; 4],
    model_name: *const PlainCString,
    text: ConstantString,
    kind: Kind,
    count: u32,
    offset: u32,
    rows: Vec<Row>,
}
impl Catalog {
    fn capture<G: FnMut() -> Result<(), String>>(
        binding: ControllerBinding,
        text: ConstantString,
        cstring: CStringLookup,
        kind: Kind,
        offset: u32,
        limit: u32,
        guard: &mut G,
    ) -> Result<Self, String> {
        validate_page(offset, limit)?;
        binding.check(guard)?;
        let mut names = [ptr::null(); 4];
        let mut nodes = vec![NodePin::capture(
            interface(binding.controller.0, &NODE, guard)?,
            guard,
        )?];
        if nodes[0].identity.0 != binding.identity.0 {
            return Err("model root Node identity differs from controller".into());
        }
        for (index, name) in [
            c"workspaceroot",
            c"plugInUnknown",
            c"signalchain",
            c"effectSelector",
        ]
        .into_iter()
        .enumerate()
        {
            guard()?;
            names[index] = unsafe { text(name.as_ptr()) };
            guard()?;
            if names[index].is_null() {
                return Err("model route constant unavailable".into());
            }
            let child = find(nodes[index].node.0, names[index]);
            let child = pin(child, guard)
                .map_err(|reason| format!("model route {}: {reason}", name.to_string_lossy()))?;
            nodes.push(NodePin::capture(child, guard)?);
        }
        let controller = interface(nodes[4].node.0, &CONTROLLER, guard)?;
        let identity = interface(controller.0, &UNKNOWN, guard)?;
        let matches = identity.0 == nodes[4].identity.0;
        release(identity, guard)?;
        if !matches {
            return Err("model selector controller identity differs from Node".into());
        }
        let model_name = unsafe { cstring(kind.name().as_ptr()) };
        guard()?;
        if model_name.is_null() {
            return Err("model lookup constant unavailable".into());
        }
        let pointer = object(controller.0, model_name);
        let object = pin(pointer, guard).map_err(|_| "named model object absent".to_owned())?;
        // getObject may ignore the requested IID. Never treat its result as a
        // model until the declared interface and canonical identity both match.
        let model = interface(object.0, &ITEM_MODEL, guard)?;
        let model_identity = interface(model.0, &UNKNOWN, guard)?;
        let object_identity = interface(object.0, &UNKNOWN, guard)?;
        let matches = model_identity.0 == object_identity.0;
        release(object_identity, guard)?;
        if !matches {
            return Err("named model has a different canonical interface identity".into());
        }
        let mut catalog = Self {
            model,
            model_identity,
            object,
            controller,
            nodes: RoutePins(nodes),
            binding,
            names,
            model_name,
            text,
            kind,
            count: 0,
            offset,
            rows: Vec::new(),
        };
        catalog.count = catalog.read_count(guard)?;
        if offset > catalog.count {
            return Err("model offset exceeds observed count".into());
        }
        let end = offset.saturating_add(limit).min(catalog.count);
        catalog.rows = catalog.read_rows(offset..end, guard)?;
        catalog.check(guard)?;
        Ok(catalog)
    }
    fn read_count<G: FnMut() -> Result<(), String>>(&self, guard: &mut G) -> Result<u32, String> {
        self.binding.check(guard)?;
        let get: Count = unsafe { method(self.model.0, 5) };
        let count = unsafe { get(self.model.0) };
        self.binding.check(guard)?;
        if !(0..=MAX_ITEMS as i32).contains(&count) {
            return Err("model count outside0..4096".into());
        }
        Ok(count as u32)
    }
    fn read_rows<G: FnMut() -> Result<(), String>>(
        &self,
        rows: std::ops::Range<u32>,
        guard: &mut G,
    ) -> Result<Vec<Row>, String> {
        let mut observed = Vec::new();
        let mut units = 0;
        for index in rows {
            let mut checked = || self.binding.check(guard);
            let mut title = Title::new(self.text, &mut checked)?;
            let row = ItemIndex::row(index);
            let get: unsafe extern "C" fn(*mut c_void, *mut PlainString, *const ItemIndex) -> u8 =
                unsafe { method(self.model.0, 12) };
            let available = unsafe { get(self.model.0, &mut title.0, &row) } != 0;
            let result = checked().and_then(|_| title.copy(&mut checked));
            release(title, &mut checked)?;
            let (title, copied) = result?;
            units += copied;
            if units > MAX_TEXT {
                return Err("model titles exceed aggregate16384 UTF-16 units".into());
            }
            observed.push(Row {
                index,
                title,
                title_available: available,
            });
        }
        Ok(observed)
    }
    pub(crate) fn check<G: FnMut() -> Result<(), String>>(
        &self,
        guard: &mut G,
    ) -> Result<(), String> {
        self.binding.check(guard)?;
        let observations = (|| {
            let mut owners = Vec::new();
            for node in self.nodes.iter() {
                let identity = interface(node.node.0, &UNKNOWN, guard)?;
                if identity.0 != node.identity.0 {
                    return Err("model route canonical identity changed".into());
                }
                owners.push(identity);
            }
            for (base, iid, expected) in [
                (self.binding.controller.0, &NODE, self.nodes[0].node.0),
                (self.nodes[4].node.0, &CONTROLLER, self.controller.0),
                (self.object.0, &ITEM_MODEL, self.model.0),
                (self.model.0, &UNKNOWN, self.model_identity.0),
                (self.object.0, &UNKNOWN, self.model_identity.0),
                (self.controller.0, &UNKNOWN, self.nodes[4].identity.0),
            ] {
                let view = interface(base, iid, guard)?;
                if view.0 != expected {
                    return Err("model interface view or identity changed".into());
                }
                owners.push(view);
            }
            Ok::<_, String>(owners)
        })();
        let reason = observations.as_ref().err().cloned();
        release(observations, guard)?;
        if let Some(reason) = reason {
            return Err(reason);
        }
        if self.read_count(guard)? != self.count {
            return Err("model count changed".into());
        }
        let end = self.offset + self.rows.len() as u32;
        if self.read_rows(self.offset..end, guard)? != self.rows {
            return Err("model row content changed".into());
        }
        if self.read_count(guard)? != self.count {
            return Err("model count changed after rows".into());
        }
        // All temporary owners, row carriers and context/provider/instance
        // guards precede the final borrowed attachment observations.
        self.binding.check(guard)?;
        for index in 0..4 {
            if find(self.nodes[index].node.0, self.names[index]) != self.nodes[index + 1].node.0 {
                return Err("model route attachment changed".into());
            }
        }
        if object(self.controller.0, self.model_name) != self.object.0 {
            return Err("named model attachment changed".into());
        }
        Ok(())
    }
    pub(crate) fn value(&self) -> Value {
        let end = self.offset + self.rows.len() as u32;
        let complete = self.rows.iter().all(|row| row.title_available);
        json!({"ok":true,"outcome":"observed","catalog":self.kind,
            "model_name":self.kind.name().to_str().expect("fixed ASCII name"),
            "count":self.count,"offset":self.offset,"rows":self.rows,
            "next_offset":(end < self.count).then_some(end),"complete":complete,
            "all_flat_rows_observed":complete && self.offset==0 && end==self.count,
            "atomic_snapshot":false,"mutation_capability":false,"scope":"current_selector_flat_model"})
    }
}

pub(crate) fn validate_page(offset: u32, limit: u32) -> Result<(), String> {
    if offset > MAX_ITEMS || !(1..=MAX_PAGE).contains(&limit) {
        Err("model catalog requires offset0..4096 and limit1..32".into())
    } else {
        Ok(())
    }
}
pub(crate) fn capture(
    ctx: &Context<'_>,
    kind: Kind,
    offset: u32,
    limit: u32,
) -> Result<Catalog, String> {
    ctx.check()?;
    if !supported(&ctx.instance.class_uid) {
        return Err(
            "model_catalog requires the observed Mustang Native or Rumble Native class".into(),
        );
    }
    let text = constant_string().ok_or("native String constants unavailable")?;
    let mut guard = || ctx.check();
    let result = AliasBinding::capture(ctx.target.0, ctx.node.0, &mut guard)
        .and_then(|alias| ControllerBinding::capture(alias, &mut guard))
        .and_then(|binding| {
            Catalog::capture(
                binding,
                text,
                ctx.api.cstring,
                kind,
                offset,
                limit,
                &mut guard,
            )
        });
    if let Err(reason) = guard() {
        release(result, &mut guard)?;
        return Err(reason);
    }
    result
}
pub(crate) fn invalidate(value: &mut Value) {
    if value["scope"] == "current_selector_flat_model" {
        if let Some(object) = value.as_object_mut() {
            for key in [
                "count",
                "offset",
                "rows",
                "next_offset",
                "complete",
                "all_flat_rows_observed",
            ] {
                object.remove(key);
            }
        }
    }
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod tests;

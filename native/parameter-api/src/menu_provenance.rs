//! Fixed public read-only provenance. No methods are selected by request data.
use super::*;
pub(crate) const PARAMETER: Uid = Uid {
    a: 0x0a9a3dd8,
    b: 0x1262,
    c: 0x4152,
    d: [0xbe, 0xfa, 0xcd, 0xcb, 0x8d, 0x1a, 0x1e, 0xf6],
};
pub(crate) const COMMAND: Uid = Uid {
    a: 0xcb8108a9,
    b: 0xdc88,
    c: 0x4152,
    d: [0xad, 0xe0, 0xc3, 0xae, 0x48, 0x9d, 0x75, 0xb5],
};
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProvenanceChainEnd {
    SelfReference,
    MissingOriginal,
    Cycle { index: usize },
    Limit,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeafProvenance {
    pub path: Vec<u32>,
    pub handler_matches_builder: bool,
    pub data_handler_matches_builder: bool,
    pub data_builder_matches_builder: bool,
    pub data_parameter_is_selected: bool,
    pub original_is_selected: Vec<bool>,
    pub original_end: ProvenanceChainEnd,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuProvenance {
    pub leaves: Vec<LeafProvenance>,
    pub menu_data_typed_values_observed: bool,
    pub selection_authority: bool,
    pub selection_calls: u32,
}
// Canonical addresses are internal comparison witnesses only, never output.
#[derive(PartialEq, Eq)]
pub(super) struct Capture {
    pub observation: MenuProvenance,
    identities: Vec<usize>,
}
const MAX_PROVENANCE_BORROWED: usize = 256;
fn keep(raw: *mut c_void, r: &mut Resources) -> Result<*mut c_void, Error> {
    if r.borrowed.len() >= MAX_PROVENANCE_BORROWED {
        return Err(Error::InvalidData("fixed provenance reference ceiling"));
    }
    let held = unsafe { Owned::hold(raw) }?;
    let p = held.raw();
    r.borrowed.push(held);
    Ok(p)
}
fn qi<G: FnMut() -> Result<(), String>>(
    raw: *mut c_void,
    iid: &Uid,
    r: &mut Resources,
    g: &mut G,
) -> Result<*mut c_void, Error> {
    g().map_err(Error::Guard)?;
    if r.borrowed.len() + 2 > MAX_PROVENANCE_BORROWED {
        return Err(Error::InvalidData("fixed provenance reference ceiling"));
    }
    let owned = unsafe { Owned::hold(raw) }?;
    // Retain first; no guard may invalidate a borrowed view before retain.
    r.borrowed.push(owned);
    g().map_err(Error::Guard)?;
    let view = r.borrowed.last().unwrap().query(iid);
    let result = match view {
        Ok(v) => {
            let p = v.raw();
            r.borrowed.push(v);
            Ok(p)
        }
        Err(e) => Err(e),
    };
    g().map_err(Error::Guard)?;
    result
}
fn canonical<G: FnMut() -> Result<(), String>>(
    raw: *mut c_void,
    r: &mut Resources,
    g: &mut G,
) -> Result<usize, Error> {
    Ok(qi(raw, &ffi::UNKNOWN, r, g)? as usize)
}
fn parent<G: FnMut() -> Result<(), String>>(
    item: *mut c_void,
    r: &mut Resources,
    g: &mut G,
) -> Result<*mut c_void, Error> {
    g().map_err(Error::Guard)?;
    let f: unsafe extern "C" fn(*mut c_void) -> *mut c_void = unsafe { ffi::method(item, 5) };
    let held = keep(unsafe { f(item) }, r);
    g().map_err(Error::Guard)?;
    held
}
fn object_attribute<G: FnMut() -> Result<(), String>>(
    raw: *mut c_void,
    menu: bool,
    r: &mut Resources,
    g: &mut G,
) -> Result<*mut c_void, Error> {
    g().map_err(Error::Guard)?;
    let mut v = Variant::empty();
    let f: unsafe extern "C" fn(*mut c_void, *mut Variant, i32) -> u8 =
        unsafe { ffi::method(raw, if menu { 6 } else { 3 }) };
    let found = unsafe { f(raw, &mut v, if menu { 3 } else { 12 }) };
    let held = if found == 1 && matches!(v.tag, 4 | 20) && v.payload != 0 {
        keep(v.payload as usize as *mut c_void, r)
    } else {
        Err(Error::InvalidData(
            "fixed public object attribute unavailable",
        ))
    };
    // All successful objects are retained before either guard or Variant drop.
    let after = g().map_err(Error::Guard);
    drop(v);
    let final_guard = g().map_err(Error::Guard);
    after?;
    final_guard?;
    held
}
fn chain<G: FnMut() -> Result<(), String>>(
    start: *mut c_void,
    selected: usize,
    r: &mut Resources,
    g: &mut G,
) -> Result<(Vec<usize>, Vec<bool>, ProvenanceChainEnd), Error> {
    let mut current = start;
    let mut ids = Vec::new();
    let mut matches = Vec::new();
    loop {
        let id = canonical(current, r, g)?;
        if let Some(index) = ids.iter().position(|x| *x == id) {
            let end = if index + 1 == ids.len() {
                ProvenanceChainEnd::SelfReference
            } else {
                ProvenanceChainEnd::Cycle { index }
            };
            return Ok((ids, matches, end));
        }
        ids.push(id);
        matches.push(id == selected);
        if ids.len() == 8 {
            return Ok((ids, matches, ProvenanceChainEnd::Limit));
        }
        g().map_err(Error::Guard)?;
        let original: unsafe extern "C" fn(*mut c_void) -> *mut c_void =
            unsafe { ffi::method(current, 18) };
        let raw = unsafe { original(current) };
        if raw.is_null() {
            g().map_err(Error::Guard)?;
            return Ok((ids, matches, ProvenanceChainEnd::MissingOriginal));
        }
        current = keep(raw, r)?;
        g().map_err(Error::Guard)?;
    }
}
pub(super) fn capture<G: FnMut() -> Result<(), String>>(
    menu: *mut c_void,
    builder: *mut c_void,
    selected: *mut c_void,
    rows: &[MenuRow],
    r: &mut Resources,
    g: &mut G,
) -> Result<Capture, Error> {
    if rows.len() != 4 {
        return Err(Error::InvalidData(
            "exact Stereo Send four-row menu required",
        ));
    }
    for (i, title) in ["없음", "메인 - 1 + 2"].iter().enumerate() {
        let row = &rows[i];
        if row.path != [i as u32]
            || row.name.as_deref() != Some(if i == 0 { "0" } else { "1" })
            || row.title.as_deref() != Some(title)
            || row.category.as_deref() != Some("Param")
            || row.checked != Some(i == 0)
            || row.enabled != Some(true)
            || row.handler_present != Some(true)
            || row.has_submenu
        {
            return Err(Error::InvalidData("fixed Param leaf differs"));
        }
    }
    for (index, name, title, category, handler) in [
        (2, "", "-", "", false),
        (3, "Setup", "오디오 I/O 설정", "Console", true),
    ] {
        let row = &rows[index];
        if row.path != [index as u32]
            || row.name.as_deref() != Some(name)
            || row.title.as_deref() != Some(title)
            || row.category.as_deref() != Some(category)
            || row.checked != Some(false)
            || row.enabled != Some(true)
            || row.handler_present != Some(handler)
            || row.has_submenu
        {
            return Err(Error::InvalidData("fixed four-row membership differs"));
        }
    }
    // These are members pinned by the original bounded walk, never caller paths.
    let items = {
        let root = r
            .snapshots
            .iter()
            .find(|s| s.raw == menu)
            .ok_or(Error::InvalidData("missing root membership"))?;
        [root.items[0].raw, root.items[1].raw]
    };
    let menu_id = canonical(menu, r, g)?;
    let builder_id = canonical(builder, r, g)?;
    let selected_id = canonical(selected, r, g)?;
    let mut identities = vec![menu_id, builder_id, selected_id];
    let mut leaves = Vec::new();
    for (index, item) in items.into_iter().enumerate() {
        // Reread each leaf metadata around provenance callbacks.
        let mut total = 0;
        let now = MenuRow {
            path: vec![index as u32],
            name: text(item, 0, &mut total, g)?,
            title: text(item, 1, &mut total, g)?,
            category: text(item, 11, &mut total, g)?,
            checked: boolean(item, 4, g)?,
            enabled: boolean(item, 5, g)?,
            handler_present: handler(item, g)?,
            has_submenu: submenu(item, r, g)?.is_some(),
        };
        if now != rows[index] {
            return Err(Error::MetadataChanged);
        }
        let parent = parent(item, r, g)?;
        let parent_id = canonical(parent, r, g)?;
        if parent_id != menu_id {
            return Err(Error::MetadataChanged);
        }
        let handler = object_attribute(item, false, r, g)?;
        let command = qi(handler, &COMMAND, r, g)?;
        let handler_id = canonical(command, r, g)?;
        let data = object_attribute(parent, true, r, g)?;
        let data_id = canonical(data, r, g)?;
        let data_builder = qi(data, &BUILDER_IID, r, g)?;
        let data_builder_id = canonical(data_builder, r, g)?;
        let data_command = qi(data, &COMMAND, r, g)?;
        let data_command_id = canonical(data_command, r, g)?;
        let parameter = qi(data, &PARAMETER, r, g)?;
        let parameter_id = canonical(parameter, r, g)?;
        let (original_ids, original_is_selected, original_end) =
            chain(parameter, selected_id, r, g)?;
        identities.extend([
            parent_id,
            handler_id,
            data_id,
            data_builder_id,
            data_command_id,
            parameter_id,
        ]);
        identities.extend(original_ids);
        leaves.push(LeafProvenance {
            path: vec![index as u32],
            handler_matches_builder: handler_id == builder_id,
            data_handler_matches_builder: data_command_id == builder_id,
            data_builder_matches_builder: data_builder_id == builder_id,
            data_parameter_is_selected: parameter_id == selected_id,
            original_is_selected,
            original_end,
        });
    }
    g().map_err(Error::Guard)?;
    Ok(Capture {
        observation: MenuProvenance {
            leaves,
            menu_data_typed_values_observed: false,
            selection_authority: false,
            selection_calls: 0,
        },
        identities,
    })
}

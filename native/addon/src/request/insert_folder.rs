//! Read-only host preset preflight for one bound audio Inserts folder.
use super::*;
use fender_native_preset_api::folder::{FolderInputs, FolderObservation, FolderPresetRoute};

const REGISTRY: Uid = Uid {
    a: 0x6c19f466,
    b: 0xc0c3,
    c: 0x4c91,
    d: [0x8b, 0x1e, 0x5e, 0x00, 0x29, 0xc6, 0x74, 0xa8],
};

struct Pin {
    node: Owned,
    canonical: Owned,
}
impl Pin {
    fn borrowed(node: *mut c_void) -> Result<Self, String> {
        let node = hold(node).ok_or("Inserts attachment node unavailable")?;
        let canonical = query(node.0, &UNKNOWN).ok_or("Inserts attachment identity unavailable")?;
        Ok(Self { node, canonical })
    }
    fn parent(&self) -> Result<Self, String> {
        let get: Borrowed = unsafe { method(self.node.0, 6) };
        Self::borrowed(unsafe { get(self.node.0) })
    }
    fn uids(&self) -> (String, String) {
        let object: unsafe extern "C" fn(*mut c_void) -> *const Uid =
            unsafe { method(self.node.0, 4) };
        let class: unsafe extern "C" fn(*mut c_void) -> *const Uid =
            unsafe { method(self.node.0, 5) };
        (
            uid(unsafe { object(self.node.0) }),
            uid(unsafe { class(self.node.0) }),
        )
    }
}

struct AnchorFolderRoute {
    folder: Pin,
    channel: Pin,
    slot: Pin,
    device: Pin,
    auxiliary: Pin,
    inserts_name: *const PlainString,
    slot_name: *const PlainString,
    device_name: *const PlainString,
    folder_uids: (String, String),
    channel_uids: (String, String),
    slot_uids: (String, String),
}
impl AnchorFolderRoute {
    fn resolve<G: FnMut() -> Result<(), String>>(
        device: Pin,
        auxiliary: Pin,
        names: [*const PlainString; 3],
        guard: &mut G,
    ) -> Result<Self, String> {
        guard()?;
        let slot = device.parent()?;
        guard()?;
        let folder = slot.parent()?;
        guard()?;
        if folder.canonical.0 != auxiliary.canonical.0 {
            return Err("auxiliary Inserts is not the anchor Device's folder".into());
        }
        let channel = folder.parent()?;
        guard()?;
        let route = Self {
            folder_uids: folder.uids(),
            channel_uids: channel.uids(),
            slot_uids: slot.uids(),
            folder,
            channel,
            slot,
            device,
            auxiliary,
            inserts_name: names[0],
            slot_name: names[1],
            device_name: names[2],
        };
        guard()?;
        route.check()?;
        Ok(route)
    }
    fn capture(ctx: &Context<'_>, auxiliary: &Owned) -> Result<Self, String> {
        ctx.check()?;
        let string = constant_string().ok_or("native string constants unavailable")?;
        let names = [c"Inserts", c"FX01", c"Device"].map(|name| unsafe { string(name.as_ptr()) });
        if names.iter().any(|name| name.is_null()) {
            return Err("fixed Inserts identifiers unavailable".into());
        }
        let auxiliary_node =
            query(auxiliary.0, &NODE).ok_or("Inserts node interface unavailable")?;
        let auxiliary_identity =
            query(auxiliary.0, &UNKNOWN).ok_or("Inserts canonical identity unavailable")?;
        let auxiliary_pin = Pin::borrowed(auxiliary_node.0)?;
        let same = auxiliary_pin.canonical.0 == auxiliary_identity.0;
        drop(auxiliary_identity);
        drop(auxiliary_node);
        ctx.check()?;
        if !same {
            return Err("Inserts node canonical identity mismatch".into());
        }
        let device = Pin::borrowed(ctx.node.0)?;
        let target_identity =
            query(ctx.target.0, &UNKNOWN).ok_or("anchor canonical identity unavailable")?;
        let same = target_identity.0 == device.canonical.0;
        drop(target_identity);
        ctx.check()?;
        if !same {
            return Err("anchor node canonical identity mismatch".into());
        }
        let route = Self::resolve(device, auxiliary_pin, names, &mut || ctx.check())?;
        let family = (
            class_name(route.slot.node.0),
            class_name(route.folder.node.0),
            class_name(route.channel.node.0),
        );
        ctx.check()?;
        route.check()?;
        ctx.check_document()?;
        if family
            != (
                "AudioInsertSlot".into(),
                "AudioInsertFolder".into(),
                "AudioTrackChannel".into(),
            )
            || !valid_uid(&route.folder_uids.0)
            || !valid_uid(&route.channel_uids.0)
            || !valid_uid(&route.slot_uids.0)
        {
            return Err(
                "preflight requires an AudioTrackChannel AudioInsertFolder and AudioInsertSlot"
                    .into(),
            );
        }
        Ok(route)
    }
    fn check_metadata(&self) -> Result<(), String> {
        if self.folder.canonical.0 != self.auxiliary.canonical.0
            || self.folder.uids() != self.folder_uids
            || self.slot.uids() != self.slot_uids
            || self.channel.uids() != self.channel_uids
        {
            Err("anchor Device, Inserts or channel identity changed".into())
        } else {
            Ok(())
        }
    }
    fn check_attachments(&self) -> Result<(), String> {
        let parent: Borrowed = unsafe { method(self.device.node.0, 6) };
        let slot_parent: Borrowed = unsafe { method(self.slot.node.0, 6) };
        let folder_parent: Borrowed = unsafe { method(self.folder.node.0, 6) };
        let find: unsafe extern "C" fn(*mut c_void, *const PlainString) -> *mut c_void =
            unsafe { method(self.channel.node.0, 10) };
        let find_slot: unsafe extern "C" fn(*mut c_void, *const PlainString) -> *mut c_void =
            unsafe { method(self.folder.node.0, 10) };
        let find_device: unsafe extern "C" fn(*mut c_void, *const PlainString) -> *mut c_void =
            unsafe { method(self.slot.node.0, 10) };
        if unsafe { parent(self.device.node.0) } != self.slot.node.0
            || unsafe { slot_parent(self.slot.node.0) } != self.folder.node.0
            || unsafe { folder_parent(self.folder.node.0) } != self.channel.node.0
            || unsafe { find(self.channel.node.0, self.inserts_name) } != self.folder.node.0
            || unsafe { find_slot(self.folder.node.0, self.slot_name) } != self.slot.node.0
            || unsafe { find_device(self.slot.node.0, self.device_name) } != self.device.node.0
        {
            Err("anchor Device, Inserts or channel attachment changed".into())
        } else {
            Ok(())
        }
    }
    fn check(&self) -> Result<(), String> {
        self.check_metadata()?;
        self.check_attachments()
    }
    fn guard(&self, ancestry: &node_inspect::Auxiliary, ctx: &Context<'_>) -> Result<(), String> {
        ancestry.check(ctx)?;
        self.check_metadata()?;
        ctx.check()?;
        self.check_attachments()?;
        ctx.check_document()
    }
}

enum BackendFailure {
    Unavailable(String),
    Binding(String),
}
impl From<fender_native_preset_api::Error> for BackendFailure {
    fn from(error: fender_native_preset_api::Error) -> Self {
        match error {
            fender_native_preset_api::Error::Unavailable(_)
            | fender_native_preset_api::Error::InvalidData(_) => {
                Self::Unavailable(error.to_string())
            }
            _ => Self::Binding(error.to_string()),
        }
    }
}

enum Availability {
    Observed {
        backend: FolderPresetRoute,
        observation: FolderObservation,
    },
    Unavailable(String),
}

pub(super) struct InsertFolderPreflight {
    availability: Availability,
    ancestry: node_inspect::Auxiliary,
    anchor: AnchorFolderRoute,
}

fn transfer(owner: Owned) -> NonNull<c_void> {
    let pointer = NonNull::new(owner.0).expect("owned native interface");
    std::mem::forget(owner);
    pointer
}

fn interface<G: FnMut() -> Result<(), String>>(
    source: &Owned,
    iid: &Uid,
    guard: &mut G,
) -> Result<Owned, String> {
    let result = (|| {
        guard()?;
        let original = query(source.0, &UNKNOWN).ok_or("service canonical identity unavailable")?;
        guard()?;
        let typed = query(source.0, iid).ok_or("required host service interface unavailable")?;
        guard()?;
        let identity = query(typed.0, &UNKNOWN).ok_or("typed service identity unavailable")?;
        guard()?;
        if original.0 != identity.0 {
            return Err("host service interface canonical mismatch".into());
        }
        Ok(typed)
    })();
    if let Err(reason) = guard() {
        drop(result);
        guard()?;
        return Err(reason);
    }
    result
}

fn inputs<G: FnMut() -> Result<(), String>>(
    api: &Api,
    folder: &Pin,
    guard: &mut G,
) -> Result<FolderInputs, String> {
    let result: Result<_, String> = (|| {
        guard()?;
        let table = hold(unsafe { (api.objects)() }).ok_or("object table unavailable")?;
        guard()?;
        let by_id: unsafe extern "C" fn(*mut c_void, *const Uid) -> *mut c_void =
            unsafe { method(table.0, 8) };
        let registry =
            hold(unsafe { by_id(table.0, &REGISTRY) }).ok_or("preset file registry unavailable")?;
        guard()?;
        let registry_interface = interface(&registry, &REGISTRY, guard)?;
        drop(registry);
        drop(table);
        guard()?;
        let plugin_manager =
            hold(unsafe { (api.plugin_manager)() }).ok_or("plug-in manager unavailable")?;
        guard()?;
        let allocator = interface(&plugin_manager, &ALLOCATOR, guard)?;
        drop(plugin_manager);
        guard()?;
        let folder = hold(folder.node.0).expect("retained folder");
        guard()?;
        Ok((folder, registry_interface, allocator))
    })();
    if let Err(reason) = guard() {
        drop(result);
        guard()?;
        return Err(reason);
    }
    let (folder, registry, allocator) = result?;
    Ok(FolderInputs {
        folder: transfer(folder),
        registry: transfer(registry),
        allocator: transfer(allocator),
    })
}

impl InsertFolderPreflight {
    pub(super) fn capture(ctx: &Context<'_>, auxiliary: Owned) -> Result<Self, String> {
        let anchor = AnchorFolderRoute::capture(ctx, &auxiliary)?;
        let ancestry = node_inspect::capture_insert_folder_ancestry(ctx, auxiliary)?;
        if !ancestry.document_environment_matched() {
            return Err(
                "Inserts ancestry does not establish the current document Environment".into(),
            );
        }
        anchor.guard(&ancestry, ctx)?;
        let backend = (|| {
            let inputs = inputs(ctx.api, &anchor.folder, &mut || {
                anchor.guard(&ancestry, ctx)
            })
            .map_err(BackendFailure::Unavailable)?;
            let backend = unsafe {
                FolderPresetRoute::discover_owned(inputs, &mut || anchor.guard(&ancestry, ctx))
            }
            .map_err(BackendFailure::from)?;
            let observation = backend
                .observe(&mut || anchor.guard(&ancestry, ctx))
                .map_err(BackendFailure::from)?;
            Ok::<_, BackendFailure>(Availability::Observed {
                backend,
                observation,
            })
        })();
        anchor.guard(&ancestry, ctx)?;
        let preflight = Self {
            availability: match backend {
                Ok(availability) => availability,
                Err(BackendFailure::Unavailable(reason)) => Availability::Unavailable(reason),
                Err(BackendFailure::Binding(reason)) => return Err(reason),
            },
            ancestry,
            anchor,
        };
        preflight.check(ctx)?;
        Ok(preflight)
    }
    pub(super) fn check(&self, ctx: &Context<'_>) -> Result<(), String> {
        if let Availability::Observed { backend, .. } = &self.availability {
            backend
                .check(&mut || self.anchor.guard(&self.ancestry, ctx))
                .map_err(|error| error.to_string())?;
        }
        self.anchor.guard(&self.ancestry, ctx)
    }
    pub(super) fn value(&self) -> Value {
        let mut value = json!({"ok":true,"outcome":"observed","scope":"insert_folder_preset_preflight_observation",
            "mutation_capability":false,"atomic_snapshot":false});
        match &self.availability {
            Availability::Observed { observation, .. } => {
                value["status"] = json!("observed");
                value["anchor_slot_bound"] = json!(true);
                value["folder"] = json!({"id":"Inserts","native_class":"AudioInsertFolder",
                    "object_uid":self.anchor.folder_uids.0,"class_uid":if self.anchor.folder_uids.1 == ZERO_UID { None } else { Some(&self.anchor.folder_uids.1) }});
                value["document_environment_observation"] =
                    self.ancestry.value()["document_environment_observation"].clone();
                value["target_flags"] = json!(observation.target_flags);
                value["metadata"] = json!({"attribute_count":observation.metadata.attribute_count,
                    "class_name":observation.metadata.class_name,"class_vendor":observation.metadata.class_vendor,
                    "class_category":observation.metadata.class_category});
            }
            Availability::Unavailable(reason) => {
                value["status"] = json!("unavailable");
                value["reason"] = json!(reason.chars().take(512).collect::<String>());
            }
        }
        value
    }
}

pub(super) fn invalidate(value: &mut Value) {
    if value["scope"] == "insert_folder_preset_preflight_observation" {
        let object = value.as_object_mut().expect("preflight response");
        for key in [
            "status",
            "reason",
            "anchor_slot_bound",
            "folder",
            "document_environment_observation",
            "target_flags",
            "metadata",
        ] {
            object.remove(key);
        }
    }
}

#[cfg(test)]
#[path = "insert_folder/tests.rs"]
mod tests;

use super::*;
use sha2::{Digest, Sha256};
use std::sync::Arc;

#[path = "state_stream.rs"]
mod stream;
use stream::MemoryStream;

#[path = "state/pedalboard.rs"]
mod pedalboard;
#[path = "state/rumble_model_manifest.rs"]
mod rumble_model_manifest;
#[path = "state/rumble_processor_state.rs"]
mod rumble_processor_state;
#[path = "state/rumble_serialization.rs"]
mod rumble_serialization;

pub(super) use super::ObservationKind;

pub(crate) const MAX_CAPSULE_BYTES: usize = 16 * 1024 * 1024;
const MAX_TOTAL_BYTES: usize = 64 * 1024 * 1024;
const MAX_CAPSULES: usize = 8;
static LIVE_CAPSULES: AtomicU32 = AtomicU32::new(0);
pub(super) const STORABLE: Uid = Uid {
    a: 0xb131b242,
    b: 0xbff4,
    c: 0x446f,
    d: [0x96, 0xe4, 0xec, 0x19, 0xdf, 0xad, 0x11, 0xe6],
};
#[link(name = "System")]
unsafe extern "C" {
    fn arc4random_buf(buffer: *mut c_void, bytes: usize);
}

/// The normal Device component alias, not an editor-selected or title-matched
/// provider. This lease lives through native output publication.
pub(super) struct AliasLease {
    pub(super) observation: ObservationKind,
    storable: Owned,
    canonical: Owned,
    plugin: Owned,
    node: Owned,
    alias: Owned,
    parent: Owned,
}
impl AliasLease {
    fn capture(ctx: &Context<'_>) -> Result<Self, String> {
        ctx.check()?;
        let alias = query(ctx.target.0, &ALIAS).ok_or("Device component alias unavailable")?;
        let get: Borrowed = unsafe { method(alias.0, 3) };
        let plugin = hold(unsafe { get(alias.0) }).ok_or("Device component alias is empty")?;
        let canonical = query(plugin.0, &UNKNOWN).ok_or("component alias identity unavailable")?;
        let storable = query(plugin.0, &STORABLE).ok_or("component alias has no IStorable")?;
        let node = hold(ctx.node.0).ok_or("Device node unavailable")?;
        let parent_at: Borrowed = unsafe { method(node.0, 6) };
        let parent = hold(unsafe { parent_at(node.0) }).ok_or("Device parent unavailable")?;
        let lease = Self {
            observation: ObservationKind::Ordinary,
            storable,
            canonical,
            plugin,
            alias,
            parent,
            node,
        };
        lease.check()?;
        ctx.check()?;
        Ok(lease)
    }
    pub(super) fn check(&self) -> Result<(), String> {
        let get: Borrowed = unsafe { method(self.alias.0, 3) };
        let parent_at: Borrowed = unsafe { method(self.node.0, 6) };
        if unsafe { get(self.alias.0) } != self.plugin.0
            || unsafe { parent_at(self.node.0) } != self.parent.0
        {
            Err("Device component alias or parent attachment changed".into())
        } else {
            Ok(())
        }
    }
    pub(super) fn component_identity_matches(&self, canonical: *mut c_void) -> bool {
        self.canonical.0 == canonical
    }
    fn guard(&self, ctx: &Context<'_>) -> Result<(), String> {
        self.check()?;
        ctx.check()
    }
}

struct Binding {
    // Child/interface owners drop while ancestor/document pins are still live.
    alias: Owned,
    target: Owned,
    parent: Owned,
    model: Owned,
    document: Owned,
    instance: Instance,
}
impl Binding {
    fn capture(ctx: &Context<'_>, alias: &AliasLease) -> Result<Self, String> {
        let binding = Self {
            document: hold(ctx.document._identity.0).ok_or("document identity unavailable")?,
            model: hold(ctx.document._model_identity.0).ok_or("model identity unavailable")?,
            target: query(ctx.target.0, &UNKNOWN).ok_or("target identity unavailable")?,
            alias: hold(alias.canonical.0).ok_or("alias identity unavailable")?,
            parent: query(alias.parent.0, &UNKNOWN).ok_or("parent identity unavailable")?,
            instance: ctx.instance.clone(),
        };
        alias.guard(ctx)?;
        Ok(binding)
    }
    fn current_document(&self, ctx: &Context<'_>) -> bool {
        self.document.0 == ctx.document._identity.0
            && self.model.0 == ctx.document._model_identity.0
    }
    fn matches(&self, ctx: &Context<'_>, alias: &AliasLease) -> Result<(), String> {
        let target = query(ctx.target.0, &UNKNOWN).ok_or("target identity unavailable")?;
        let parent = query(alias.parent.0, &UNKNOWN).ok_or("parent identity unavailable")?;
        let matches = self.current_document(ctx)
            && self.target.0 == target.0
            && self.alias.0 == alias.canonical.0
            && self.parent.0 == parent.0
            && same_instance(&self.instance, &ctx.instance);
        drop(target);
        drop(parent);
        alias.guard(ctx)?;
        if matches {
            Ok(())
        } else {
            Err("state token belongs to a different document/model/Device/alias".into())
        }
    }
}
struct Capsule {
    token: String,
    bytes: Arc<[u8]>,
    sha256: String,
    binding: Binding,
}
impl Drop for Capsule {
    fn drop(&mut self) {
        LIVE_CAPSULES.fetch_sub(1, Ordering::Relaxed);
    }
}
pub(crate) struct Registry {
    entries: Vec<Capsule>,
}
impl Registry {
    pub(crate) fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
    fn bytes(&self) -> usize {
        self.entries.iter().map(|c| c.bytes.len()).sum()
    }
    fn capacity(&self, bytes: usize) -> Result<(), String> {
        if bytes > MAX_CAPSULE_BYTES
            || self.entries.len() >= MAX_CAPSULES
            || self
                .bytes()
                .checked_add(bytes)
                .is_none_or(|total| total > MAX_TOTAL_BYTES)
        {
            Err("native state capsule count or byte limit reached".into())
        } else {
            Ok(())
        }
    }
    fn take(&mut self, token: &str) -> Result<Capsule, String> {
        self.entries
            .iter()
            .position(|c| c.token == token)
            .map(|index| self.entries.swap_remove(index))
            .ok_or("state token is expired, released or belongs to another service".into())
    }
}
pub(super) fn valid_token(token: &str) -> bool {
    token.len() == 64
        && token
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'A'..=b'F').contains(&c))
}
fn same_instance(a: &Instance, b: &Instance) -> bool {
    a.object_uid == b.object_uid && a.class_uid == b.class_uid
}
fn token() -> String {
    let mut bytes = [0_u8; 32];
    unsafe { arc4random_buf(bytes.as_mut_ptr().cast(), bytes.len()) };
    bytes.iter().map(|b| format!("{b:02X}")).collect()
}
fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn with_registry<T>(service: *mut Service, f: impl FnOnce(&mut Registry) -> T) -> T {
    // Only request dispatch/stop on the validated GUI thread enters this slot.
    // The closure never calls native code or drops an owned native object.
    let service = unsafe { &*service };
    let registry = unsafe { &mut *service.states.get() };
    let result = f(registry);
    service
        .states_pending
        .store(!registry.entries.is_empty(), Ordering::Release);
    result
}
pub(super) fn expire(ctx: &Context<'_>) -> Result<(), String> {
    let old = with_registry(ctx.service, |registry| {
        let mut old = Vec::new();
        let mut index = 0;
        while index < registry.entries.len() {
            if registry.entries[index].binding.current_document(ctx) {
                index += 1;
            } else {
                old.push(registry.entries.swap_remove(index));
            }
        }
        old
    });
    drop(old);
    ctx.check()
}
pub(crate) fn drain(service: *mut Service) {
    let old = with_registry(service, |registry| std::mem::take(&mut registry.entries));
    drop(old);
}
pub(crate) fn live_count() -> u32 {
    LIVE_CAPSULES.load(Ordering::Acquire)
}
pub(crate) fn limits() -> Value {
    json!({"max_count":MAX_CAPSULES,"max_bytes_per_state":MAX_CAPSULE_BYTES,"max_total_bytes":MAX_TOTAL_BYTES})
}

fn save(ctx: &Context<'_>, lease: &AliasLease, limit: usize) -> Result<Arc<[u8]>, String> {
    let stream = MemoryStream::capture(limit);
    let save: unsafe extern "C" fn(*mut c_void, *mut c_void) -> u8 =
        unsafe { method(lease.storable.0, 4) };
    lease.guard(ctx)?;
    let accepted = unsafe { save(lease.storable.0, stream.raw()) } != 0;
    let bytes = stream.finish_capture();
    drop(stream);
    lease.guard(ctx)?;
    if !accepted {
        return Err("IStorable.save rejected state capture".into());
    }
    Ok(Arc::from(bytes?))
}

pub(super) struct PedalboardExchange {
    lease: AliasLease,
    expected: Vec<u8>,
}

impl PedalboardExchange {
    pub(super) fn prepare(
        ctx: &Context<'_>,
        slots: [u8; 2],
        vacant: [bool; 2],
        fingerprint: &str,
    ) -> Result<Self, String> {
        let lease = AliasLease::capture(ctx)?;
        let bytes = save(ctx, &lease, pedalboard::MAX_RACK_BYTES)?;
        if sha256(&bytes) != fingerprint {
            return Err("Pedalboard component no longer matches the expected fingerprint".into());
        }
        let expected = pedalboard::expected_exchange(&bytes, slots, vacant)
            .ok_or("Pedalboard serialization cannot establish the expected exchange")?;
        lease.guard(ctx)?;
        Ok(Self { lease, expected })
    }

    pub(super) fn verify(&self, ctx: &Context<'_>) -> Result<(), String> {
        let bytes = save(ctx, &self.lease, pedalboard::MAX_RACK_BYTES)?;
        let value =
            pedalboard::decode(&bytes).ok_or("Post-exchange Pedalboard state is unavailable")?;
        if serde_json::to_vec(&value)
            .map_err(|_| "Post-exchange Pedalboard state is unavailable")?
            != self.expected
        {
            return Err(
                "Serialized Pedalboard component differs from the expected exchange".into(),
            );
        }
        self.lease.guard(ctx)?;
        ctx.check_document()
    }
}

pub(super) fn capture(ctx: &Context<'_>) -> Result<(Value, AliasLease), String> {
    with_registry(ctx.service, |registry| registry.capacity(0))?;
    let lease = AliasLease::capture(ctx)?;
    let bytes = save(ctx, &lease, MAX_CAPSULE_BYTES)?;
    let binding = Binding::capture(ctx, &lease)?;
    let sha256 = sha256(&bytes);
    let token = token();
    let mut summary = json!({"ok":true,"token":token,"byte_count":bytes.len(),"sha256":sha256,"serialization":"component_alias_IStorable","verified":false,"outcome":"observed"});
    if ctx.instance.native_class == "AudioEffect"
        && ctx.instance.object_id == "Pedalboard"
        && ctx.instance.class_uid == pedalboard::CLASS
        && let Some(rack) = pedalboard::rack(&bytes)
    {
        summary["pedalboard_rack"] = rack;
    }
    lease.guard(ctx)?;
    with_registry(ctx.service, |registry| registry.capacity(bytes.len()))?;
    let capsule = Capsule {
        token,
        bytes,
        sha256,
        binding,
    };
    LIVE_CAPSULES.fetch_add(1, Ordering::Relaxed);
    with_registry(ctx.service, |registry| registry.entries.push(capsule));
    lease.guard(ctx)?;
    Ok((summary, lease))
}
pub(super) fn read_chorder(ctx: &Context<'_>) -> Result<(Arc<[u8]>, AliasLease), String> {
    let lease = AliasLease::capture(ctx)?;
    let bytes = save(ctx, &lease, super::chorder_chords::MAX_CAPTURE_BYTES)?;
    lease.guard(ctx)?;
    Ok((bytes, lease))
}
pub(super) fn inspect_rumble_serialization(
    ctx: &Context<'_>,
) -> Result<(Value, AliasLease), String> {
    ctx.check()?;
    if ctx.instance.class_uid != "B9D7536C-A010-457D-A182-36E5531164C9"
        || ctx.instance.native_class != "AudioEffect"
        || ctx.instance.object_id != "Rumble Native"
    {
        return Err("The exact Rumble Native component is required".into());
    }
    let mut lease = AliasLease::capture(ctx)?;
    lease.observation = ObservationKind::RumbleStructure;
    let bytes = save(ctx, &lease, MAX_CAPSULE_BYTES)?;
    let mut value = rumble_serialization::summarize(&bytes).fields();
    let fields = json!({"ok":true,"outcome":"observed","operation":"rumble_serialization_inspect",
        "schema":"rumble_alias_serialization_summary_v1","serialization":"component_alias_IStorable",
        "instance":ctx.instance,"source":"component_alias","capture_sha256":sha256(&bytes),
        "capture_byte_count":bytes.len(),"value_semantics":"sequential_snapshot","mutation_capability":false,
        "model_identity_verified":false,"audio_verified":false,"persistence_verified":false});
    value
        .as_object_mut()
        .expect("summary")
        .extend(fields.as_object().expect("receipt").clone());
    lease.guard(ctx)?;
    ctx.check_document()?;
    // Reserve the exact adapter fields before native publication.
    let mut adapted = value.clone();
    adapted["plugin"] = json!("Rumble Native");
    adapted["native_outcome"] = json!("observed");
    if adapted.to_string().len() > 4096 {
        return Err("Rumble structural receipt exceeds the fixed output limit".into());
    }
    Ok((value, lease))
}
pub(super) struct ModelSnapshot {
    pub(super) classes: Vec<String>,
    pub(super) names: Vec<std::collections::BTreeSet<String>>,
    pub(super) sha256: String,
}
pub(super) fn rumble_model_digest(classes: &[String]) -> Option<String> {
    if classes.len() != 26 {
        return None;
    }
    let mut hash = rumble_model_manifest::model_hasher();
    for (i, class) in classes.iter().enumerate() {
        let (canonical, uid) = rumble_model_manifest::guid(class)?;
        if canonical != *class {
            return None;
        }
        rumble_model_manifest::hash_record(&mut hash, &rumble_model_manifest::slot(i), &uid);
    }
    Some(format!("{:x}", hash.finalize()))
}
pub(super) fn capture_rumble_models(
    ctx: &Context<'_>,
) -> Result<(ModelSnapshot, AliasLease), String> {
    if ctx.instance.class_uid != rumble_model_manifest::CLASS
        || ctx.instance.native_class != "AudioEffect"
        || ctx.instance.object_id != "Rumble Native"
    {
        return Err("The exact Rumble Native component is required".into());
    }
    let lease = AliasLease::capture(ctx)?;
    let snapshot = rumble_models(ctx, &lease)?;
    Ok((snapshot, lease))
}
pub(super) fn rumble_models(
    ctx: &Context<'_>,
    lease: &AliasLease,
) -> Result<ModelSnapshot, String> {
    lease.guard(ctx)?;
    let bytes = save(ctx, lease, MAX_CAPSULE_BYTES)?;
    let document = rumble_model_manifest::Document::decode(&bytes)?;
    let snapshot = ModelSnapshot {
        classes: document.models.iter().map(|m| m.class.clone()).collect(),
        names: (0..26)
            .map(|i| {
                document
                    .processor(i)
                    .keys()
                    .filter(|k| k.as_str() != "__classid")
                    .cloned()
                    .collect()
            })
            .collect(),
        sha256: document.models_sha256(),
    };
    lease.guard(ctx)?;
    ctx.check_document()?;
    Ok(snapshot)
}
#[cfg(test)]
pub(super) fn read_rumble_state(ctx: &Context<'_>) -> Result<(Value, AliasLease), String> {
    read_processor_state(ctx, FenderProcessor::Rumble)
}

pub(super) fn read_processor_state(
    ctx: &Context<'_>,
    plugin: FenderProcessor,
) -> Result<(Value, AliasLease), String> {
    ctx.check()?;
    let profile = plugin.profile();
    // Studio Pro can append a number to the object label for duplicate inserts.
    if ctx.instance.class_uid != profile.class_uid
        || ctx.instance.native_class != "AudioEffect"
        || ctx.instance.object_id.is_empty()
    {
        return Err(format!("The exact {} component is required", profile.name));
    }
    let mut lease = AliasLease::capture(ctx)?;
    lease.observation = ObservationKind::ProcessorState(plugin);
    let bytes = save(ctx, &lease, MAX_CAPSULE_BYTES)?;
    let mut value = rumble_processor_state::read_for(&bytes, plugin)?;
    let fields = json!({"ok":true,"outcome":"observed","operation":profile.receipt_operation,"schema":profile.schema,"serialization":"component_alias_IStorable","instance":ctx.instance,"source":"component_alias","capture_sha256":sha256(&bytes),"capture_byte_count":bytes.len(),"value_semantics":"sequential_snapshot","native_access_metadata_verified":false,"mutation_capability":false,"audio_verified":false,"persistence_verified":false});
    value
        .as_object_mut()
        .expect("projection")
        .extend(fields.as_object().expect("fields").clone());
    lease.guard(ctx)?;
    ctx.check_document()?;
    let mut adapted = value.clone();
    adapted["plugin"] = json!(profile.name);
    adapted["native_outcome"] = json!("observed");
    if adapted.to_string().len() > rumble_processor_state::MAX_RESULT_BYTES
        || !rumble_processor_state::valid_receipt(&json!(ctx.instance), &value, plugin)
    {
        return Err(format!("{} processor receipt unavailable", profile.product));
    }
    Ok((value, lease))
}
pub(super) fn observation_failure(kind: ObservationKind) -> Value {
    match kind {
        ObservationKind::ProcessorState(plugin) => {
            let profile = plugin.profile();
            json!({"ok":false,"outcome":"not_applied","native_outcome":"not_applied","native_access_metadata_verified":false,"mutation_capability":false,"audio_verified":false,"persistence_verified":false,"error":{"code":profile.error_code,"message":format!("The bounded {} processor-state observation was not established", profile.product)}})
        }
        _ => rumble_failure(),
    }
}
pub(super) fn valid_observation(kind: ObservationKind, instance: &Instance, value: &Value) -> bool {
    match kind {
        ObservationKind::Ordinary => true,
        ObservationKind::RumbleStructure => valid_rumble_receipt(instance, value),
        ObservationKind::ProcessorState(plugin) => {
            rumble_processor_state::valid_receipt(&json!(instance), value, plugin)
        }
        ObservationKind::RumbleParameters | ObservationKind::RumbleNamedParameters => false,
    }
}
pub(super) fn valid_rumble_receipt(instance: &Instance, value: &Value) -> bool {
    rumble_serialization::valid_receipt(&json!(instance), value)
}
pub(super) fn rumble_failure() -> Value {
    json!({"ok":false,"outcome":"not_applied","mutation_capability":false,"model_identity_verified":false,
        "audio_verified":false,"persistence_verified":false,"error":{"code":"rumble_serialization_unavailable",
        "message":"The bounded Rumble serialization observation was not established"}})
}
pub(super) fn invalidate_serialized_observation(value: &mut Value) {
    for plugin in [FenderProcessor::Rumble, FenderProcessor::Mustang] {
        if value["operation"] == plugin.profile().receipt_operation {
            *value = observation_failure(ObservationKind::ProcessorState(plugin));
            return;
        }
    }
    if value["operation"] == "rumble_serialization_inspect" {
        *value = rumble_failure();
    }
}
#[derive(Debug)]
pub(super) struct LoadFailure {
    pub(super) reason: String,
    pub(super) dispatched: bool,
}

pub(super) fn load_once<G: FnMut() -> Result<(), String>>(
    storable: &Owned,
    bytes: Arc<[u8]>,
    guard: &mut G,
) -> Result<(), LoadFailure> {
    let stream = MemoryStream::restore(bytes);
    let load: unsafe extern "C" fn(*mut c_void, *mut c_void) -> u8 =
        unsafe { method(storable.0, 5) };
    guard().map_err(|reason| LoadFailure {
        reason,
        dispatched: false,
    })?;
    let accepted = unsafe { load(storable.0, stream.raw()) } != 0;
    let stream_result = stream.finish_restore();
    drop(stream);
    guard().map_err(|reason| LoadFailure {
        reason,
        dispatched: true,
    })?;
    stream_result.map_err(|reason| LoadFailure {
        reason,
        dispatched: true,
    })?;
    if !accepted {
        return Err(LoadFailure {
            reason: "IStorable.load returned failure after possible state changes".into(),
            dispatched: true,
        });
    }
    Ok(())
}

pub(super) fn restore(ctx: &Context<'_>, token: &str) -> (Value, Option<AliasLease>) {
    let lease = match AliasLease::capture(ctx) {
        Ok(lease) => lease,
        Err(reason) => {
            return (
                error("state_target_unavailable", &reason, "not_applied"),
                None,
            );
        }
    };
    let capsule = match with_registry(ctx.service, |registry| registry.take(token)) {
        Ok(capsule) => capsule,
        Err(reason) => {
            return (
                error("invalid_state_token", &reason, "not_applied"),
                Some(lease),
            );
        }
    };
    let result = capsule.binding.matches(ctx, &lease)
        .map_err(|reason| LoadFailure { reason, dispatched: false })
        .and_then(|()| load_once(&lease.storable, capsule.bytes.clone(), &mut || lease.guard(ctx)))
        .map(|()| json!({"ok":true,"token":token,"byte_count":capsule.bytes.len(),
            "sha256":capsule.sha256,"load_accepted":true,"verified":false,"outcome":"applied_unverified"}));
    // A removed token remains reusable only while the original context and
    // alias are still current. No native owner is dropped inside the registry.
    if lease.guard(ctx).is_ok() {
        with_registry(ctx.service, |registry| registry.entries.push(capsule));
    } else {
        drop(capsule);
    }
    let value = match result {
        Ok(value) => value,
        Err(failure) => error(
            "state_restore_failed",
            &failure.reason,
            if failure.dispatched {
                "unknown"
            } else {
                "not_applied"
            },
        ),
    };
    (value, Some(lease))
}
pub(super) fn release(ctx: &Context<'_>, token: &str) -> Result<Value, String> {
    let capsule = with_registry(ctx.service, |registry| registry.take(token))?;
    let same_document = capsule.binding.current_document(ctx)
        && same_instance(&capsule.binding.instance, &ctx.instance);
    if !same_document {
        with_registry(ctx.service, |registry| registry.entries.push(capsule));
        return Err("state token does not belong to this target".into());
    }
    drop(capsule);
    ctx.check()?;
    Ok(
        json!({"ok":true,"token":token,"released":true,"verified":true,"outcome":"applied_verified"}),
    )
}

#[cfg(test)]
#[path = "state_tests.rs"]
pub(super) mod tests;

use super::*;
use sha2::{Digest, Sha256};
use std::sync::Arc;

#[path = "state_stream.rs"]
mod stream;
use stream::MemoryStream;

pub(crate) const MAX_CAPSULE_BYTES: usize = 16 * 1024 * 1024;
const MAX_TOTAL_BYTES: usize = 64 * 1024 * 1024;
const MAX_CAPSULES: usize = 8;
static LIVE_CAPSULES: AtomicU32 = AtomicU32::new(0);
const STORABLE: Uid = Uid {
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
pub(super) fn capture(ctx: &Context<'_>) -> Result<(Value, AliasLease), String> {
    with_registry(ctx.service, |registry| registry.capacity(0))?;
    let lease = AliasLease::capture(ctx)?;
    let bytes = save(ctx, &lease, MAX_CAPSULE_BYTES)?;
    let binding = Binding::capture(ctx, &lease)?;
    let sha256 = sha256(&bytes);
    let token = token();
    let summary = json!({"ok":true,"token":token,"byte_count":bytes.len(),"sha256":sha256,"serialization":"component_alias_IStorable","verified":false,"outcome":"observed"});
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
    let mut dispatched = false;
    let result: Result<Value, String> = (|| {
        capsule.binding.matches(ctx, &lease)?;
        let stream = MemoryStream::restore(capsule.bytes.clone());
        let load: unsafe extern "C" fn(*mut c_void, *mut c_void) -> u8 =
            unsafe { method(lease.storable.0, 5) };
        lease.guard(ctx)?;
        dispatched = true;
        let accepted = unsafe { load(lease.storable.0, stream.raw()) } != 0;
        let stream_result = stream.finish_restore();
        drop(stream);
        lease.guard(ctx)?;
        stream_result?;
        if !accepted {
            return Err("IStorable.load returned failure after possible state changes".into());
        }
        Ok(
            json!({"ok":true,"token":token,"byte_count":capsule.bytes.len(),"sha256":capsule.sha256,"load_accepted":true,"verified":false,"outcome":"applied_unverified"}),
        )
    })();
    // A removed token remains reusable only while the original context and
    // alias are still current. No native owner is dropped inside the registry.
    if lease.guard(ctx).is_ok() {
        with_registry(ctx.service, |registry| registry.entries.push(capsule));
    } else {
        drop(capsule);
    }
    let value = match result {
        Ok(value) => value,
        Err(reason) => error(
            "state_restore_failed",
            &reason,
            if dispatched { "unknown" } else { "not_applied" },
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
mod tests;

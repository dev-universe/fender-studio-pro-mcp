//! Private host-authored whole-Inserts capsules. No caller file authority.
use super::folder_context::{FolderBindingKey, FolderContext};
use super::*;
use fender_native_preset_api::folder::collection::{CollectionFile, PrivateDestination};
use fender_native_preset_api::folder::{FolderInputs, FolderPresetRoute};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, DirBuilder, Metadata, OpenOptions},
    io::Read,
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    rc::Rc,
};

const MAX_ENTRIES: usize = 8;
const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 64 * 1024 * 1024;
const TTL: Duration = Duration::from_secs(600);
const O_NOFOLLOW: i32 = 0x100;
static LIVE_CAPSULES: AtomicU32 = AtomicU32::new(0);

#[link(name = "System")]
unsafe extern "C" {
    fn arc4random_buf(buffer: *mut c_void, bytes: usize);
    fn geteuid() -> u32;
}
fn random_hex() -> String {
    let mut bytes = [0_u8; 32];
    unsafe { arc4random_buf(bytes.as_mut_ptr().cast(), bytes.len()) };
    bytes.iter().map(|byte| format!("{byte:02X}")).collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Reserved,
    StoredUnpublished,
    Issued,
    InFlight,
    Quarantined,
}
struct Entry<T> {
    token: String,
    created: Instant,
    phase: Phase,
    bytes: u64,
    owner: Option<Rc<T>>,
}
pub(crate) struct Registry<T = Capsule> {
    entries: Vec<Entry<T>>,
}
impl<T> Registry<T> {
    pub(crate) fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
    fn reserve(&mut self, token: String, now: Instant) -> Result<(), &'static str> {
        if self.entries.len() >= MAX_ENTRIES {
            return Err("insert-chain capsule count limit reached");
        }
        if self.entries.iter().any(|entry| entry.token == token) {
            return Err("insert-chain capsule token collision");
        }
        self.entries.push(Entry {
            token,
            created: now,
            phase: Phase::Reserved,
            bytes: 0,
            owner: None,
        });
        Ok(())
    }
    fn total_bytes(&self) -> u64 {
        self.entries.iter().map(|entry| entry.bytes).sum()
    }
    fn store(
        &mut self,
        token: &str,
        bytes: u64,
        owner: Rc<T>,
    ) -> Result<(), (Rc<T>, &'static str)> {
        let total = self.total_bytes().checked_add(bytes);
        let Some(entry) = self.entries.iter_mut().find(|entry| entry.token == token) else {
            return Err((owner, "insert-chain reservation is no longer available"));
        };
        if entry.phase != Phase::Reserved || entry.owner.is_some() {
            return Err((owner, "insert-chain reservation is not empty"));
        }
        if bytes == 0 || bytes > MAX_FILE_BYTES || total.is_none_or(|sum| sum > MAX_TOTAL_BYTES) {
            return Err((owner, "insert-chain capsule byte limit reached"));
        }
        entry.bytes = bytes;
        entry.owner = Some(owner);
        entry.phase = Phase::StoredUnpublished;
        Ok(())
    }
    fn issue(&mut self, token: &str) -> bool {
        let Some(entry) = self.entries.iter_mut().find(|entry| entry.token == token) else {
            return false;
        };
        if entry.phase != Phase::StoredUnpublished
            || entry.owner.is_none()
            || Instant::now().saturating_duration_since(entry.created) >= TTL
        {
            return false;
        }
        entry.phase = Phase::Issued;
        true
    }
    fn can_issue(&self, token: &str) -> bool {
        self.entries.iter().any(|entry| {
            entry.token == token
                && entry.phase == Phase::StoredUnpublished
                && entry.owner.is_some()
                && Instant::now().saturating_duration_since(entry.created) < TTL
        })
    }
    fn in_flight(&self, token: &str, now: Instant) -> bool {
        self.entries.iter().any(|entry| {
            entry.token == token
                && entry.phase == Phase::InFlight
                && now.saturating_duration_since(entry.created) < TTL
        })
    }
    fn peek(&self, token: &str, now: Instant) -> Result<Rc<T>, &'static str> {
        self.entries
            .iter()
            .find(|entry| {
                entry.token == token
                    && matches!(entry.phase, Phase::Issued | Phase::Quarantined)
                    && now.saturating_duration_since(entry.created) < TTL
            })
            .and_then(|entry| entry.owner.as_ref().map(Rc::clone))
            .ok_or("insert-chain token is unavailable")
    }
    fn begin(&mut self, token: &str, now: Instant) -> Result<Rc<T>, &'static str> {
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.token == token)
            .ok_or("insert-chain token is unavailable")?;
        if entry.phase != Phase::Issued || now.saturating_duration_since(entry.created) >= TTL {
            return Err("insert-chain token is expired or not callable");
        }
        let owner = entry
            .owner
            .take()
            .ok_or("insert-chain capsule is unavailable")?;
        entry.phase = Phase::InFlight;
        Ok(owner)
    }
    fn finish(&mut self, token: &str, owner: Rc<T>, phase: Phase) -> Result<(), Rc<T>> {
        let Some(entry) = self.entries.iter_mut().find(|entry| entry.token == token) else {
            return Err(owner);
        };
        if entry.phase != Phase::InFlight
            || entry.owner.is_some()
            || !matches!(
                phase,
                Phase::StoredUnpublished | Phase::Issued | Phase::Quarantined
            )
        {
            return Err(owner);
        }
        entry.owner = Some(owner);
        entry.phase = phase;
        Ok(())
    }
    fn remove(&mut self, token: &str) -> Option<Entry<T>> {
        self.entries
            .iter()
            .position(|entry| entry.token == token)
            .map(|index| self.entries.swap_remove(index))
    }
    fn reap(&mut self, now: Instant, keep: impl Fn(&T) -> bool) -> Vec<Entry<T>> {
        let mut removed = Vec::new();
        let mut index = 0;
        while index < self.entries.len() {
            let entry = &self.entries[index];
            if now.saturating_duration_since(entry.created) >= TTL
                || entry.owner.as_ref().is_some_and(|owner| !keep(owner))
            {
                removed.push(self.entries.swap_remove(index));
            } else {
                index += 1;
            }
        }
        removed
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileIdentity {
    device: u64,
    inode: u64,
    user: u32,
}
impl FileIdentity {
    fn of(metadata: &Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            user: metadata.uid(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct FileProof {
    identity: FileIdentity,
    byte_count: u64,
    sha256: String,
}
struct PrivateFile {
    directory: PathBuf,
    path: PathBuf,
    directory_identity: FileIdentity,
    file_identity: Option<FileIdentity>,
}
impl PrivateFile {
    fn create() -> Result<Self, &'static str> {
        let parent = fs::canonicalize(std::env::temp_dir())
            .map_err(|_| "private temporary directory is unavailable")?;
        Self::create_under(&parent)
    }
    fn create_under(parent: &Path) -> Result<Self, &'static str> {
        let directory = parent.join(format!("fsp-inserts-{}", random_hex()));
        DirBuilder::new()
            .mode(0o700)
            .create(&directory)
            .map_err(|_| "private capsule directory could not be created exclusively")?;
        let metadata = fs::symlink_metadata(&directory)
            .map_err(|_| "private capsule directory could not be inspected")?;
        let path = directory.join(format!("{}.multipreset", random_hex()));
        let file = Self {
            directory,
            path,
            directory_identity: FileIdentity::of(&metadata),
            file_identity: None,
        };
        file.check_directory()?;
        match fs::symlink_metadata(&file.path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(file),
            _ => Err("private capsule destination already exists"),
        }
    }
    fn check_directory(&self) -> Result<(), &'static str> {
        let metadata = fs::symlink_metadata(&self.directory)
            .map_err(|_| "private capsule directory is unavailable")?;
        if !metadata.is_dir()
            || metadata.file_type().is_symlink()
            || FileIdentity::of(&metadata) != self.directory_identity
            || metadata.uid() != unsafe { geteuid() }
            || metadata.mode() & 0o777 != 0o700
        {
            return Err("private capsule directory identity or permissions changed");
        }
        Ok(())
    }
    fn path_text(&self) -> Result<&str, &'static str> {
        self.path
            .to_str()
            .ok_or("private capsule path is unavailable")
    }
    fn fingerprint(&self) -> Result<FileProof, &'static str> {
        self.check_directory()?;
        let before =
            fs::symlink_metadata(&self.path).map_err(|_| "private capsule file is unavailable")?;
        if !before.is_file()
            || before.file_type().is_symlink()
            || before.uid() != self.directory_identity.user
            || before.nlink() != 1
            || before.len() == 0
            || before.len() > MAX_FILE_BYTES
            || self
                .file_identity
                .is_some_and(|identity| identity != FileIdentity::of(&before))
        {
            return Err("private capsule file identity, type or size is invalid");
        }
        let mut file = OpenOptions::new()
            .read(true)
            .custom_flags(O_NOFOLLOW)
            .open(&self.path)
            .map_err(|_| "private capsule file could not be opened without following links")?;
        let opened = file
            .metadata()
            .map_err(|_| "private capsule file could not be inspected")?;
        if !same_file(&before, &opened) || !opened.is_file() {
            return Err("private capsule file changed while opening");
        }
        let mut digest = Sha256::new();
        let mut count = 0_u64;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let bytes = file
                .read(&mut buffer)
                .map_err(|_| "private capsule file could not be read")?;
            if bytes == 0 {
                break;
            }
            count += bytes as u64;
            if count > MAX_FILE_BYTES {
                return Err("private capsule file exceeds its post-store byte limit");
            }
            digest.update(&buffer[..bytes]);
        }
        let after = file
            .metadata()
            .map_err(|_| "private capsule file could not be rechecked")?;
        let named = fs::symlink_metadata(&self.path)
            .map_err(|_| "private capsule path changed while reading")?;
        if count != before.len() || !same_file(&before, &after) || !same_file(&after, &named) {
            return Err("private capsule file changed while reading");
        }
        drop(file);
        self.check_directory()?;
        Ok(FileProof {
            identity: FileIdentity::of(&after),
            byte_count: count,
            sha256: format!("{:x}", digest.finalize()),
        })
    }
    fn seal(&mut self) -> Result<FileProof, &'static str> {
        let proof = self.fingerprint()?;
        self.file_identity = Some(proof.identity);
        Ok(proof)
    }
    fn matches(&self, expected: &FileProof) -> Result<(), &'static str> {
        if self.fingerprint()? == *expected {
            Ok(())
        } else {
            Err("private capsule file no longer matches its protected hash and identity")
        }
    }
    fn cleanup(&self) -> Result<(), &'static str> {
        match fs::symlink_metadata(&self.directory) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(_) => return Err("private capsule directory cleanup could not be checked"),
            Ok(_) => {}
        }
        self.check_directory()?;
        match fs::symlink_metadata(&self.path) {
            Ok(metadata) => {
                if !metadata.is_file()
                    || metadata.file_type().is_symlink()
                    || metadata.uid() != self.directory_identity.user
                    || metadata.nlink() != 1
                    || self
                        .file_identity
                        .is_some_and(|identity| identity != FileIdentity::of(&metadata))
                {
                    return Err("private capsule cleanup refused a replaced file");
                }
                fs::remove_file(&self.path).map_err(|_| "private capsule file cleanup failed")?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("private capsule file cleanup could not be checked"),
        }
        fs::remove_dir(&self.directory).map_err(|_| "private capsule directory cleanup failed")
    }
}
impl Drop for PrivateFile {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}
fn same_file(a: &Metadata, b: &Metadata) -> bool {
    FileIdentity::of(a) == FileIdentity::of(b)
        && a.len() == b.len()
        && a.mtime() == b.mtime()
        && a.mtime_nsec() == b.mtime_nsec()
        && a.ctime() == b.ctime()
        && a.ctime_nsec() == b.ctime_nsec()
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Generation {
    session: String,
    document: String,
}
struct LiveCapsule;
impl LiveCapsule {
    fn new() -> Self {
        LIVE_CAPSULES.fetch_add(1, Ordering::Relaxed);
        Self
    }
}
impl Drop for LiveCapsule {
    fn drop(&mut self) {
        LIVE_CAPSULES.fetch_sub(1, Ordering::Relaxed);
    }
}
pub(crate) struct Capsule {
    backend: CollectionFile,
    context: Rc<FolderContext<'static>>,
    private: PrivateFile,
    proof: FileProof,
    key: FolderBindingKey,
    generation: Generation,
    _live: LiveCapsule,
}
impl Capsule {
    fn matches_binding(&self, ctx: &FolderContext<'_>, generation: &Generation) -> bool {
        self.key == ctx.binding_key() && self.generation == *generation
    }
    fn check(&self, ctx: &FolderContext<'_>, generation: &Generation) -> Result<(), &'static str> {
        if !self.matches_binding(ctx, generation) {
            return Err("insert-chain binding changed");
        }
        self.private.matches(&self.proof)?;
        self.backend
            .check(&mut || ctx.check())
            .map_err(|_| "insert-chain backend binding changed")?;
        self.private.matches(&self.proof)?;
        ctx.check().map_err(|_| "insert-chain context changed")
    }
    fn dispose(self) -> Result<(), &'static str> {
        let Self {
            backend,
            context,
            private,
            _live,
            ..
        } = self;
        drop(backend);
        drop(context);
        let result = private.cleanup();
        drop(private);
        drop(_live);
        result
    }
}
fn with_registry<T>(service: *mut Service, f: impl FnOnce(&mut Registry) -> T) -> T {
    let service = unsafe { &*service };
    let registry = unsafe { &mut *service.insert_chains.get() };
    let result = f(registry);
    service
        .insert_chains_pending
        .store(!registry.entries.is_empty(), Ordering::Release);
    result
}
pub(super) fn expire(ctx: &Context<'_>) -> Result<(), String> {
    let old = with_registry(ctx.service, |registry| {
        registry.reap(Instant::now(), |capsule| capsule.key.current_document(ctx))
    });
    drop(old);
    ctx.check()
}
fn expire_folder(
    service: *mut Service,
    ctx: &FolderContext<'_>,
    generation: &Generation,
) -> Result<(), String> {
    let key = ctx.binding_key();
    let old = with_registry(service, |registry| {
        registry.reap(Instant::now(), |capsule| {
            capsule.key.same_document(&key) && capsule.generation == *generation
        })
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
    json!({"max_count":MAX_ENTRIES,"max_bytes_per_file":MAX_FILE_BYTES,"max_total_bytes":MAX_TOTAL_BYTES,
        "ttl_seconds":TTL.as_secs(),"max_collection_parts":128,"post_store_byte_limits":true})
}
fn transfer(owner: Owned) -> NonNull<c_void> {
    let pointer = NonNull::new(owner.0).expect("owned native interface");
    std::mem::forget(owner);
    pointer
}
const REGISTRY: Uid = Uid {
    a: 0x6c19f466,
    b: 0xc0c3,
    c: 0x4c91,
    d: [0x8b, 0x1e, 0x5e, 0x00, 0x29, 0xc6, 0x74, 0xa8],
};
const URL: Uid = Uid {
    a: 0xbfca729d,
    b: 0x5097,
    c: 0x4b38,
    d: [0x9f, 0x15, 0x52, 0x13, 0xf9, 0x79, 0, 0xc4],
};
fn same_interface(
    source: &Owned,
    iid: &Uid,
    ctx: &FolderContext<'_>,
) -> Result<Owned, &'static str> {
    let result = (|| {
        ctx.check().map_err(|_| "insert-chain context changed")?;
        let original = query(source.0, &UNKNOWN).ok_or("native interface identity unavailable")?;
        ctx.check().map_err(|_| "insert-chain context changed")?;
        let typed = query(source.0, iid).ok_or("native interface unavailable")?;
        ctx.check().map_err(|_| "insert-chain context changed")?;
        let identity =
            query(typed.0, &UNKNOWN).ok_or("typed native interface identity unavailable")?;
        ctx.check().map_err(|_| "insert-chain context changed")?;
        if original.0 != identity.0 {
            return Err("native interface canonical identity changed");
        }
        Ok(typed)
    })();
    if ctx.check().is_err() {
        drop(result);
        let _ = ctx.check();
        return Err("insert-chain context changed");
    }
    result
}
fn backend_route(
    ctx: &FolderContext<'_>,
    api: &Api,
) -> Result<Rc<FolderPresetRoute>, &'static str> {
    let inputs = (|| {
        ctx.check().map_err(|_| "insert-chain context changed")?;
        let table = hold(unsafe { (api.objects)() }).ok_or("native object table unavailable")?;
        ctx.check().map_err(|_| "insert-chain context changed")?;
        let by_id: unsafe extern "C" fn(*mut c_void, *const Uid) -> *mut c_void =
            unsafe { method(table.0, 8) };
        let source =
            hold(unsafe { by_id(table.0, &REGISTRY) }).ok_or("preset file registry unavailable")?;
        ctx.check().map_err(|_| "insert-chain context changed")?;
        let registry = same_interface(&source, &REGISTRY, ctx)?;
        drop(source);
        drop(table);
        ctx.check().map_err(|_| "insert-chain context changed")?;
        let manager =
            hold(unsafe { (api.plugin_manager)() }).ok_or("native class allocator unavailable")?;
        ctx.check().map_err(|_| "insert-chain context changed")?;
        let allocator = same_interface(&manager, &ALLOCATOR, ctx)?;
        drop(manager);
        ctx.check().map_err(|_| "insert-chain context changed")?;
        let folder = ctx
            .owned_folder()
            .map_err(|_| "retained Inserts unavailable")?;
        Ok((folder, registry, allocator))
    })();
    if ctx.check().is_err() {
        drop(inputs);
        let _ = ctx.check();
        return Err("insert-chain context changed");
    }
    let (folder, registry, allocator) = inputs?;
    let result = unsafe {
        FolderPresetRoute::discover_owned(
            FolderInputs {
                folder: transfer(folder),
                registry: transfer(registry),
                allocator: transfer(allocator),
            },
            &mut || ctx.check(),
        )
    }
    .map(Rc::new)
    .map_err(|_| "folder preset route unavailable");
    if ctx.check().is_err() {
        drop(result);
        let _ = ctx.check();
        return Err("insert-chain context changed");
    }
    result
}
fn destination(
    ctx: &FolderContext<'_>,
    auxiliary: Owned,
    private: &PrivateFile,
) -> Result<PrivateDestination, &'static str> {
    let url = same_interface(&auxiliary, &URL, ctx);
    drop(auxiliary);
    if ctx.check().is_err() {
        drop(url);
        let _ = ctx.check();
        return Err("insert-chain context changed");
    }
    let url = url?;
    let path = private.path_text()?;
    unsafe { PrivateDestination::from_owned_empty_url(transfer(url), path, &mut || ctx.check()) }
        .map_err(|_| "private collection destination unavailable")
}

pub(crate) struct ReceiptLease {
    _capsule: Option<Rc<Capsule>>,
    _context: Rc<FolderContext<'static>>,
}
pub(in crate::native_script) struct Reply {
    value: Value,
    service: *mut Service,
    capsule: Option<Rc<Capsule>>,
    context: Option<Rc<FolderContext<'static>>>,
    generation: Generation,
    pending: Option<String>,
    dispatched: bool,
}
impl Reply {
    fn error(service: *mut Service, code: &str, message: &str) -> Self {
        Self {
            value: super::error(code, message, "not_applied"),
            service,
            capsule: None,
            context: None,
            generation: Generation {
                session: String::new(),
                document: String::new(),
            },
            pending: None,
            dispatched: false,
        }
    }
    fn revoke(&mut self) {
        if let Some(token) = self.pending.take() {
            let removed = with_registry(self.service, |registry| registry.remove(&token));
            drop(removed);
        }
    }
    fn invalidate(&mut self, code: &str, message: &str) {
        self.value = super::error(
            code,
            message,
            if self.dispatched {
                "unknown"
            } else {
                "not_applied"
            },
        );
        self.revoke();
    }
    pub(super) fn final_check(&mut self, service: *mut Service) -> bool {
        let result = (|| {
            if service != self.service {
                return Err(());
            }
            if let Some(ctx) = self.context.as_ref() {
                if self.value["ok"] == true {
                    if let Some(capsule) = self.capsule.as_ref() {
                        capsule.check(ctx, &self.generation).map_err(|_| ())?;
                    }
                    if self.pending.as_ref().is_some_and(|token| {
                        !with_registry(service, |registry| registry.can_issue(token))
                    }) {
                        return Err(());
                    }
                }
                ctx.check().map_err(|_| ())
            } else {
                check_service_active(service).map_err(|_| ())
            }
        })();
        if result.is_ok() {
            true
        } else {
            self.invalidate(
                "context_changed_before_receipt",
                "insert-chain context changed before publication",
            );
            false
        }
    }
    pub(super) fn take_receipt(&mut self) -> Result<Option<ReceiptLease>, ()> {
        if let Some(token) = self.pending.take() {
            if !with_registry(self.service, |registry| registry.issue(&token)) {
                self.pending = Some(token);
                self.invalidate(
                    "capsule_publication_failed",
                    "insert-chain capsule could not be published",
                );
                return Err(());
            }
        }
        Ok(self.context.take().map(|context| ReceiptLease {
            _capsule: self.capsule.take(),
            _context: context,
        }))
    }
    pub(super) fn release_context(&mut self) {
        self.revoke();
        drop(self.capsule.take());
        drop(self.context.take());
    }
    pub(super) fn encode(&mut self) -> String {
        let value = self.value.to_string();
        if value.len() <= MAX_RESPONSE_BYTES {
            value
        } else {
            self.invalidate(
                "response_too_large",
                "insert-chain response exceeded its bounded size",
            );
            self.value.to_string()
        }
    }
}
impl Drop for Reply {
    fn drop(&mut self) {
        self.revoke();
    }
}
fn common(ctx: &FolderContext<'_>, outcome: &str) -> Value {
    let folder = ctx.identity();
    json!({"ok":true,"outcome":outcome,"scope":"insert_chain_state","format":"host_multipreset_v1","version":1,
        "folder":{"object_uid":folder.object_uid,"class_uid":folder.class_uid,"native_class":folder.native_class,"object_id":folder.id},
        "verified":false,"effects_verified":false,"atomic_snapshot":false})
}
fn metadata(value: &mut Value, token: &str, capsule: &Capsule) {
    value["token"] = json!(token);
    value["byte_count"] = json!(capsule.proof.byte_count);
    value["sha256"] = json!(capsule.proof.sha256);
    value["part_count"] = json!(capsule.backend.part_count());
}
enum Action {
    Capture,
    Restore(String),
    Release(String),
}
pub(super) fn dispatch(
    service: *mut Service,
    target: Owned,
    request: Request,
    auxiliary: Option<Owned>,
    api: &'static Api,
) -> Reply {
    let (generation, expected, action) = match request {
        Request::InsertChainCapture {
            bridge_session,
            document_epoch,
            expected_folder,
        } => (
            Generation {
                session: bridge_session,
                document: document_epoch,
            },
            expected_folder,
            Action::Capture,
        ),
        Request::InsertChainRestore {
            bridge_session,
            document_epoch,
            expected_folder,
            token,
        } => (
            Generation {
                session: bridge_session,
                document: document_epoch,
            },
            expected_folder,
            Action::Restore(token),
        ),
        Request::InsertChainRelease {
            bridge_session,
            document_epoch,
            expected_folder,
            token,
        } => (
            Generation {
                session: bridge_session,
                document: document_epoch,
            },
            expected_folder,
            Action::Release(token),
        ),
        _ => {
            return Reply::error(
                service,
                "invalid_request",
                "unsupported insert-chain operation",
            );
        }
    };
    let ctx = match FolderContext::capture(service, target, expected.as_deref(), api) {
        Ok(context) => Rc::new(context),
        Err(_) => {
            return Reply::error(
                service,
                "context_guard_failed",
                "native Inserts context could not be established",
            );
        }
    };
    let mut reply = Reply::error(
        service,
        "insert_chain_failed",
        "insert-chain lifecycle failed",
    );
    reply.context = Some(Rc::clone(&ctx));
    reply.generation = generation;
    release_receipt(service);
    if ctx
        .check()
        .and_then(|_| expire_folder(service, &ctx, &reply.generation))
        .is_err()
    {
        return reply;
    }
    match action {
        Action::Capture => {
            let token = random_hex();
            if with_registry(service, |registry| {
                registry.reserve(token.clone(), Instant::now())
            })
            .is_err()
            {
                reply.value = super::error(
                    "capsule_capacity",
                    "insert-chain capsule capacity is unavailable",
                    "not_applied",
                );
                return reply;
            }
            let result = (|| {
                let mut private = PrivateFile::create()?;
                let destination =
                    destination(&ctx, auxiliary.ok_or("capture URL unavailable")?, &private)?;
                let route = backend_route(&ctx, api)?;
                let mut proof = None;
                let backend = match CollectionFile::capture_with_validation(
                    route,
                    destination,
                    &mut || ctx.check(),
                    &mut || {
                        let sealed = private.seal().map_err(str::to_owned)?;
                        ctx.check()?;
                        proof = Some(sealed);
                        Ok(())
                    },
                ) {
                    Ok(file) => {
                        reply.dispatched = true;
                        file
                    }
                    Err(error) => {
                        reply.dispatched = error.dispatched;
                        return Err("host collection capture failed");
                    }
                };
                let proof = proof.ok_or("private file was not sealed before reopen")?;
                private.matches(&proof)?;
                ctx.check().map_err(|_| "insert-chain context changed")?;
                let capsule = Rc::new(Capsule {
                    backend,
                    context: Rc::clone(&ctx),
                    private,
                    proof,
                    key: ctx.binding_key(),
                    generation: reply.generation.clone(),
                    _live: LiveCapsule::new(),
                });
                let stored = with_registry(service, |registry| {
                    registry.store(&token, capsule.proof.byte_count, Rc::clone(&capsule))
                });
                if let Err((owner, reason)) = stored {
                    drop(owner);
                    return Err(reason);
                }
                Ok(capsule)
            })();
            match result {
                Ok(capsule) => {
                    let mut value = common(&ctx, "observed");
                    metadata(&mut value, &token, &capsule);
                    reply.value = value;
                    reply.capsule = Some(capsule);
                    reply.pending = Some(token);
                }
                Err(_) => {
                    let old = with_registry(service, |registry| registry.remove(&token));
                    drop(old);
                    reply.invalidate(
                        "capture_failed",
                        "host collection capture or private-file validation failed",
                    );
                }
            }
        }
        Action::Restore(token) => {
            let capsule =
                match with_registry(service, |registry| registry.begin(&token, Instant::now())) {
                    Ok(owner) => owner,
                    Err(_) => {
                        reply.value = super::error(
                            "invalid_capsule_token",
                            "insert-chain token is unavailable",
                            "not_applied",
                        );
                        return reply;
                    }
                };
            if !capsule.matches_binding(&ctx, &reply.generation) {
                let returned = with_registry(service, |registry| {
                    registry.finish(&token, capsule, Phase::Issued)
                });
                drop(returned);
                reply.value = super::error(
                    "capsule_target_mismatch",
                    "insert-chain token does not match this folder and generation",
                    "not_applied",
                );
                return reply;
            }
            let result = (|| {
                capsule.check(&ctx, &reply.generation)?;
                match capsule.backend.restore(&mut || {
                    if !with_registry(service, |registry| {
                        registry.in_flight(&token, Instant::now())
                    }) {
                        return Err("insert-chain token expired during load".into());
                    }
                    ctx.check()
                }) {
                    Ok(result) if result.accepted => {
                        reply.dispatched = true;
                    }
                    Ok(_) => {
                        reply.dispatched = true;
                        return Err("collection load refused");
                    }
                    Err(error) => {
                        reply.dispatched = error.dispatched;
                        return Err("collection load failed");
                    }
                }
                capsule.check(&ctx, &reply.generation)
            })();
            let phase = if result.is_ok() {
                Phase::StoredUnpublished
            } else {
                Phase::Quarantined
            };
            let returned = with_registry(service, |registry| {
                registry.finish(&token, Rc::clone(&capsule), phase)
            });
            let retained = returned.is_ok();
            drop(returned);
            if result.is_ok() && retained {
                let mut value = common(&ctx, "applied_unverified");
                metadata(&mut value, &token, &capsule);
                value["load_accepted"] = json!(true);
                reply.value = value;
                reply.capsule = Some(capsule);
                reply.pending = Some(token);
            } else {
                reply.invalidate(
                    "restore_failed",
                    "host collection load or post-load validation failed",
                );
            }
        }
        Action::Release(token) => {
            let capsule =
                match with_registry(service, |registry| registry.peek(&token, Instant::now())) {
                    Ok(owner) => owner,
                    Err(_) => {
                        reply.value = super::error(
                            "invalid_capsule_token",
                            "insert-chain token is unavailable",
                            "not_applied",
                        );
                        return reply;
                    }
                };
            if !capsule.matches_binding(&ctx, &reply.generation) {
                reply.value = super::error(
                    "capsule_target_mismatch",
                    "insert-chain token does not match this folder and generation",
                    "not_applied",
                );
                return reply;
            }
            reply.dispatched = true;
            let removed = with_registry(service, |registry| registry.remove(&token));
            drop(removed);
            let clean = match Rc::try_unwrap(capsule) {
                Ok(capsule) => capsule.dispose().is_ok(),
                Err(owner) => {
                    drop(owner);
                    false
                }
            };
            if clean && ctx.check().is_ok() {
                reply.value = common(&ctx, "applied_unverified");
                reply.value["token"] = json!(token);
                reply.value["released"] = json!(true);
            } else {
                reply.invalidate("release_failed","insert-chain token was revoked but private cleanup or binding was not confirmed");
            }
        }
    }
    reply
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        cell::Cell,
        os::unix::fs::{PermissionsExt, symlink},
    };

    fn stored(registry: &mut Registry<u32>, token: &str, bytes: u64) {
        registry.reserve(token.into(), Instant::now()).unwrap();
        assert!(registry.store(token, bytes, Rc::new(7)).is_ok());
        assert!(registry.issue(token));
    }
    #[test]
    fn reservations_inflight_and_quarantine_all_count_against_capacity() {
        let mut registry = Registry::<u32>::new();
        stored(&mut registry, "issued", 1);
        stored(&mut registry, "inflight", 1);
        let inflight = registry.begin("inflight", Instant::now()).unwrap();
        stored(&mut registry, "quarantine", 1);
        let owner = registry.begin("quarantine", Instant::now()).unwrap();
        assert!(
            registry
                .finish("quarantine", owner, Phase::Quarantined)
                .is_ok()
        );
        for index in 3..MAX_ENTRIES {
            registry.reserve(index.to_string(), Instant::now()).unwrap();
        }
        assert!(registry.reserve("ninth".into(), Instant::now()).is_err());
        assert_eq!(registry.total_bytes(), 3);
        assert!(registry.begin("quarantine", Instant::now()).is_err());
        assert!(registry.peek("quarantine", Instant::now()).is_ok());
        assert!(
            registry
                .finish("inflight", inflight, Phase::StoredUnpublished)
                .is_ok()
        );
        assert!(registry.issue("inflight"));
    }
    #[test]
    fn aggregate_budget_includes_inflight_and_quarantined_files() {
        let mut registry = Registry::<u32>::new();
        for index in 0..4 {
            stored(&mut registry, &index.to_string(), MAX_FILE_BYTES);
        }
        let first = registry.begin("0", Instant::now()).unwrap();
        let second = registry.begin("1", Instant::now()).unwrap();
        assert!(registry.finish("1", second, Phase::Quarantined).is_ok());
        registry.reserve("fifth".into(), Instant::now()).unwrap();
        assert!(registry.store("fifth", 1, Rc::new(0)).is_err());
        assert_eq!(registry.total_bytes(), MAX_TOTAL_BYTES);
        drop(first);
    }
    #[test]
    fn unissued_or_expired_tokens_never_dispatch_and_collisions_do_not_replace() {
        let mut registry = Registry::<u32>::new();
        let start = Instant::now();
        registry.reserve("token".into(), start).unwrap();
        assert!(registry.reserve("token".into(), start).is_err());
        assert!(registry.begin("token", start).is_err());
        assert!(registry.store("token", 1, Rc::new(1)).is_ok());
        assert!(registry.begin("token", start).is_err());
        assert!(registry.issue("token"));
        assert!(registry.begin("token", start + TTL).is_err());
        assert!(registry.peek("token", start + TTL).is_err());
        assert_eq!(registry.reap(start + TTL, |_| true).len(), 1);
        assert!(registry.entries.is_empty());
    }
    #[test]
    fn expired_unpublished_reservations_cannot_be_issued() {
        let mut registry = Registry::<u32>::new();
        registry
            .reserve("token".into(), Instant::now() - TTL)
            .unwrap();
        assert!(registry.store("token", 1, Rc::new(1)).is_ok());
        assert!(!registry.can_issue("token"));
        assert!(!registry.issue("token"));
    }
    #[test]
    fn removed_owners_drop_only_after_the_registry_borrow_ends() {
        struct Probe(Rc<Cell<bool>>, Rc<Cell<bool>>);
        impl Drop for Probe {
            fn drop(&mut self) {
                assert!(!self.0.get());
                self.1.set(true);
            }
        }
        let borrowing = Rc::new(Cell::new(false));
        let dropped = Rc::new(Cell::new(false));
        let mut registry = Registry::<Probe>::new();
        registry.reserve("token".into(), Instant::now()).unwrap();
        assert!(
            registry
                .store(
                    "token",
                    1,
                    Rc::new(Probe(Rc::clone(&borrowing), Rc::clone(&dropped)))
                )
                .is_ok()
        );
        borrowing.set(true);
        let old = registry.reap(Instant::now(), |_| false);
        assert!(!dropped.get());
        borrowing.set(false);
        drop(old);
        assert!(dropped.get());
    }
    #[test]
    fn store_rejects_empty_oversize_and_unreserved_artifacts_without_consuming_owners() {
        for bytes in [0, MAX_FILE_BYTES + 1] {
            let mut registry = Registry::<u32>::new();
            registry.reserve("token".into(), Instant::now()).unwrap();
            let owner = Rc::new(9);
            let returned = registry
                .store("token", bytes, Rc::clone(&owner))
                .unwrap_err()
                .0;
            assert!(Rc::ptr_eq(&owner, &returned));
            assert_eq!(registry.entries[0].phase, Phase::Reserved);
        }
        let mut registry = Registry::<u32>::new();
        assert!(registry.store("missing", 1, Rc::new(1)).is_err());
    }
    #[test]
    fn private_file_seals_exact_bytes_and_cleans_only_its_owned_file() {
        let mut private = PrivateFile::create().unwrap();
        assert_eq!(
            fs::symlink_metadata(&private.directory).unwrap().mode() & 0o777,
            0o700
        );
        fs::write(&private.path, b"host-authored-fixture").unwrap();
        let proof = private.seal().unwrap();
        assert_eq!(proof.byte_count, 21);
        private.matches(&proof).unwrap();
        let directory = private.directory.clone();
        private.cleanup().unwrap();
        drop(private);
        assert!(!directory.exists());
    }
    #[test]
    fn missing_empty_and_oversized_files_never_become_capsules() {
        let private = PrivateFile::create().unwrap();
        assert!(private.fingerprint().is_err());
        fs::write(&private.path, []).unwrap();
        assert!(private.fingerprint().is_err());
        OpenOptions::new()
            .write(true)
            .open(&private.path)
            .unwrap()
            .set_len(MAX_FILE_BYTES + 1)
            .unwrap();
        assert!(private.fingerprint().is_err());
    }
    #[test]
    fn replacement_and_hash_drift_are_refused_and_replaced_files_are_not_deleted() {
        let mut private = PrivateFile::create().unwrap();
        fs::write(&private.path, b"original").unwrap();
        let proof = private.seal().unwrap();
        fs::write(&private.path, b"modified").unwrap();
        assert!(private.matches(&proof).is_err());
        let replacement = private.directory.join("replacement");
        fs::write(&replacement, b"original").unwrap();
        fs::rename(&replacement, &private.path).unwrap();
        assert!(private.matches(&proof).is_err());
        assert!(private.cleanup().is_err());
        assert_eq!(fs::read(&private.path).unwrap(), b"original");
        fs::remove_file(&private.path).unwrap();
    }
    #[test]
    fn symlinks_nonregular_files_and_extra_entries_are_not_followed_or_recursively_removed() {
        let private = PrivateFile::create().unwrap();
        let foreign = private.directory.join("foreign");
        fs::write(&foreign, b"untouched").unwrap();
        symlink(&foreign, &private.path).unwrap();
        assert!(private.fingerprint().is_err());
        assert!(private.cleanup().is_err());
        assert_eq!(fs::read(&foreign).unwrap(), b"untouched");
        fs::remove_file(&private.path).unwrap();
        fs::create_dir(&private.path).unwrap();
        assert!(private.fingerprint().is_err());
        assert!(private.cleanup().is_err());
        fs::remove_dir(&private.path).unwrap();
        assert!(private.cleanup().is_err());
        assert!(foreign.exists());
        fs::remove_file(&foreign).unwrap();
        private.cleanup().unwrap();
    }
    #[test]
    fn changed_directory_permissions_or_identity_refuse_reads_and_cleanup() {
        let private = PrivateFile::create().unwrap();
        fs::write(&private.path, b"fixture").unwrap();
        fs::set_permissions(&private.directory, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(private.fingerprint().is_err());
        assert!(private.cleanup().is_err());
        fs::set_permissions(&private.directory, fs::Permissions::from_mode(0o700)).unwrap();
        let displaced = private.directory.with_extension("moved");
        fs::rename(&private.directory, &displaced).unwrap();
        DirBuilder::new()
            .mode(0o700)
            .create(&private.directory)
            .unwrap();
        assert!(private.fingerprint().is_err());
        assert!(private.cleanup().is_err());
        fs::remove_dir(&private.directory).unwrap();
        fs::rename(displaced, &private.directory).unwrap();
    }
    fn request(action: &str) -> Value {
        let mut value = json!({"operation":format!("insert_chain_{action}"),"bridge_session":"session","document_epoch":"session:1"});
        if action != "capture" {
            value["token"] = json!("A".repeat(64));
        }
        value
    }
    fn parses(value: Value) -> bool {
        serde_json::from_value::<Request>(value).is_ok_and(|request| request.validate().is_ok())
    }
    #[test]
    fn closed_lifecycle_parser_accepts_only_finite_generation_and_folder_inputs() {
        for action in ["capture", "restore", "release"] {
            let value = request(action);
            assert!(parses(value.clone()));
            let parsed: Request = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(parsed.needs_auxiliary(), action == "capture");
            for name in [
                "path",
                "url",
                "filename",
                "bytes",
                "xml",
                "channel",
                "source",
                "expected_instance",
            ] {
                let mut wrong = value.clone();
                wrong[name] = json!("caller");
                assert!(!parses(wrong), "{name}");
            }
            for name in ["bridge_session", "document_epoch"] {
                for wrong_value in [
                    json!(null),
                    json!(""),
                    json!("x".repeat(129)),
                    json!("bad\nvalue"),
                ] {
                    let mut wrong = value.clone();
                    wrong[name] = wrong_value;
                    assert!(!parses(wrong));
                }
            }
            for wrong_value in [json!(null), json!(ZERO_UID), json!("lowercase"), json!(17)] {
                let mut wrong = value.clone();
                wrong["expected_folder"] = wrong_value;
                assert!(!parses(wrong));
            }
            let mut valid = value;
            valid["expected_folder"] = json!("D2D9B002-12FC-4F1D-AFDA-70336D1DC108");
            assert!(parses(valid));
        }
        for token in [json!(null), json!("A".repeat(64))] {
            let mut wrong = request("capture");
            wrong["token"] = token;
            assert!(!parses(wrong));
        }
        for action in ["restore", "release"] {
            for token in [json!(null), json!("a".repeat(64)), json!("A".repeat(63))] {
                let mut wrong = request(action);
                wrong["token"] = token;
                assert!(!parses(wrong));
            }
            let mut wrong = request(action);
            wrong.as_object_mut().unwrap().remove("token");
            assert!(!parses(wrong));
        }
    }
    #[test]
    fn failed_folder_publication_strips_every_success_claim_and_revokes_reservation() {
        let _lock = super::super::super::tests::TEST_LOCK.lock().unwrap();
        let fixture = folder_context::tests::Fixture::new();
        let service = fixture.service();
        let ctx = Rc::new(fixture.capture().unwrap());
        let token = "A".repeat(64);
        with_registry(service, |registry| {
            registry.reserve(token.clone(), Instant::now())
        })
        .unwrap();
        let mut reply = Reply::error(service, "placeholder", "placeholder");
        reply.context = Some(Rc::clone(&ctx));
        reply.value = common(&ctx, "observed");
        reply.value["token"] = json!(token);
        reply.value["sha256"] = json!("b".repeat(64));
        reply.value["byte_count"] = json!(1);
        reply.value["part_count"] = json!(0);
        reply.pending = Some(token);
        reply.dispatched = true;
        assert!(!reply.final_check(service));
        assert_eq!(reply.value.as_object().unwrap().len(), 3);
        assert_eq!(reply.value["outcome"], "unknown");
        assert!(with_registry(service, |registry| registry
            .entries
            .is_empty()));
        drop(reply);
        drop(ctx);
    }
    #[test]
    fn dropped_unpublished_reply_revokes_its_slot_before_native_receipt_transfer() {
        let _lock = super::super::super::tests::TEST_LOCK.lock().unwrap();
        let fixture = folder_context::tests::Fixture::new();
        let service = fixture.service();
        let token = "A".repeat(64);
        with_registry(service, |registry| {
            registry.reserve(token.clone(), Instant::now())
        })
        .unwrap();
        let mut reply = Reply::error(service, "placeholder", "placeholder");
        reply.pending = Some(token);
        drop(reply);
        assert!(with_registry(service, |registry| registry
            .entries
            .is_empty()));
        assert!(
            !unsafe { &*service }
                .insert_chains_pending
                .load(Ordering::Acquire)
        );
    }
    #[test]
    fn drain_releases_reserved_capacity_and_clears_the_service_pending_flag() {
        let _lock = super::super::super::tests::TEST_LOCK.lock().unwrap();
        let fixture = folder_context::tests::Fixture::new();
        let service = fixture.service();
        with_registry(service, |registry| {
            registry.reserve("A".repeat(64), Instant::now())
        })
        .unwrap();
        assert!(
            unsafe { &*service }
                .insert_chains_pending
                .load(Ordering::Acquire)
        );
        drain(service);
        assert!(
            !unsafe { &*service }
                .insert_chains_pending
                .load(Ordering::Acquire)
        );
        assert!(with_registry(service, |registry| registry
            .entries
            .is_empty()));
    }
    #[test]
    fn quarantined_owners_remain_live_until_expired_owners_are_dropped_outside_the_registry() {
        let _lock = super::super::super::tests::TEST_LOCK.lock().unwrap();
        let baseline = live_count();
        let mut registry = Registry::<LiveCapsule>::new();
        registry.reserve("token".into(), Instant::now()).unwrap();
        assert!(
            registry
                .store("token", 1, Rc::new(LiveCapsule::new()))
                .is_ok()
        );
        assert!(registry.issue("token"));
        let owner = registry.begin("token", Instant::now()).unwrap();
        assert!(registry.finish("token", owner, Phase::Quarantined).is_ok());
        assert_eq!(live_count(), baseline + 1);
        let old = registry.reap(Instant::now() + TTL, |_| true);
        assert_eq!(live_count(), baseline + 1);
        drop(old);
        assert_eq!(live_count(), baseline);
    }
    fn released_reply(fixture: &folder_context::tests::Fixture) -> Reply {
        let ctx = Rc::new(fixture.capture().unwrap());
        let mut reply = Reply::error(fixture.service(), "placeholder", "placeholder");
        reply.value = common(&ctx, "applied_unverified");
        reply.value["token"] = json!("A".repeat(64));
        reply.value["released"] = json!(true);
        reply.context = Some(ctx);
        reply.dispatched = true;
        reply
    }
    #[test]
    fn folder_receipt_pins_survive_publication_until_explicit_receipt_release() {
        let _lock = super::super::super::tests::TEST_LOCK.lock().unwrap();
        let fixture = folder_context::tests::Fixture::new();
        let service = fixture.service();
        let mut out = VariantOut {
            kind: 0,
            payload: 0,
        };
        assert!(publish_reply(
            &mut out,
            NativeReply::InsertChain(released_reply(&fixture)),
            fixture.api(),
            service
        ));
        let value: Value =
            serde_json::from_str(&super::super::super::tests::output_text(&out)).unwrap();
        assert_eq!(value["ok"], true);
        assert_eq!(value["released"], true);
        assert_eq!(value["verified"], false);
        assert!(value.get("instance").is_none());
        assert_eq!(LIVE_RECEIPTS.load(Ordering::Acquire), 1);
        release_receipt(service);
        assert_eq!(LIVE_RECEIPTS.load(Ordering::Acquire), 0);
        super::super::super::tests::clear_output(&mut out);
    }
    unsafe extern "C" fn missing_output_string() -> *mut c_void {
        ptr::null_mut()
    }
    #[test]
    fn failed_output_preparation_revokes_the_unpublished_reservation_and_keeps_output_empty() {
        let _lock = super::super::super::tests::TEST_LOCK.lock().unwrap();
        let fixture = folder_context::tests::Fixture::new();
        let service = fixture.service();
        let a = fixture.api();
        let failed = Api {
            gui: a.gui,
            objects: a.objects,
            main_thread: a.main_thread,
            thread_self: a.thread_self,
            cstring: a.cstring,
            empty: missing_output_string,
            desktop: a.desktop,
            scripting: a.scripting,
            plugin_manager: a.plugin_manager,
            file_system: a.file_system,
        };
        let token = "A".repeat(64);
        with_registry(service, |registry| {
            registry.reserve(token.clone(), Instant::now())
        })
        .unwrap();
        let mut reply = released_reply(&fixture);
        reply.pending = Some(token);
        let mut out = VariantOut {
            kind: 0,
            payload: 0,
        };
        assert!(!publish_reply(
            &mut out,
            NativeReply::InsertChain(reply),
            &failed,
            service
        ));
        assert_eq!(out.kind, 0);
        assert_eq!(out.payload, 0);
        assert!(with_registry(service, |registry| registry
            .entries
            .is_empty()));
        assert_eq!(LIVE_RECEIPTS.load(Ordering::Acquire), 0);
    }
    #[test]
    fn output_allocation_stop_reentry_strips_every_folder_success_claim() {
        let _lock = super::super::super::tests::TEST_LOCK.lock().unwrap();
        let fixture = folder_context::tests::Fixture::new();
        let service = fixture.service();
        let reply = released_reply(&fixture);
        super::super::super::tests::ASSIGN_REENTER_STOP.store(service, Ordering::Release);
        let mut out = VariantOut {
            kind: 0,
            payload: 0,
        };
        assert!(publish_reply(
            &mut out,
            NativeReply::InsertChain(reply),
            fixture.api(),
            service
        ));
        let value: Value =
            serde_json::from_str(&super::super::super::tests::output_text(&out)).unwrap();
        assert_eq!(value.as_object().unwrap().len(), 3);
        assert_eq!(value["ok"], false);
        assert_eq!(value["outcome"], "unknown");
        assert_eq!(LIVE_RECEIPTS.load(Ordering::Acquire), 0);
        super::super::super::tests::clear_output(&mut out);
    }
    static OUTPUT_FIXTURE: std::sync::atomic::AtomicPtr<folder_context::tests::Fixture> =
        std::sync::atomic::AtomicPtr::new(ptr::null_mut());
    unsafe extern "C" fn old_output_release(_: *mut c_void) -> u32 {
        let fixture = OUTPUT_FIXTURE.load(Ordering::Acquire);
        assert!(!fixture.is_null());
        unsafe { &*fixture }.switch_document();
        0
    }
    #[test]
    fn old_output_cleanup_reentry_is_checked_before_folder_success_publication() {
        let _lock = super::super::super::tests::TEST_LOCK.lock().unwrap();
        let fixture = folder_context::tests::Fixture::new();
        let service = fixture.service();
        let reply = released_reply(&fixture);
        let table = [
            ptr::null(),
            ptr::null(),
            old_output_release as *const c_void,
        ];
        let mut object = table.as_ptr();
        OUTPUT_FIXTURE.store(ptr::from_ref(&fixture).cast_mut(), Ordering::Release);
        let mut out = VariantOut {
            kind: 4 | 16,
            payload: ptr::addr_of_mut!(object) as u64,
        };
        assert!(publish_reply(
            &mut out,
            NativeReply::InsertChain(reply),
            fixture.api(),
            service
        ));
        OUTPUT_FIXTURE.store(ptr::null_mut(), Ordering::Release);
        let value: Value =
            serde_json::from_str(&super::super::super::tests::output_text(&out)).unwrap();
        assert_eq!(value.as_object().unwrap().len(), 3);
        assert_eq!(value["outcome"], "unknown");
        assert_eq!(LIVE_RECEIPTS.load(Ordering::Acquire), 0);
        super::super::super::tests::clear_output(&mut out);
    }
}

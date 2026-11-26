use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

use fs2::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{config::Config, midi::MidiWakeup, model::OperationError};

pub const PROTOCOL: u32 = 2;
const MAX_RESPONSE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_REQUEST_BYTES: usize = 1024 * 1024;

pub struct OwnerLock {
    _profile: File,
    _mailbox: File,
}

impl OwnerLock {
    pub fn acquire(config: &Config) -> Result<Self, OperationError> {
        if !config.data_dir().is_absolute() || !config.profile().is_absolute() {
            return Err(OperationError::new(
                "invalid_config",
                "Use absolute data/profile paths and ensure HOME is set",
            ));
        }
        private_dir(&config.data_dir())?;
        if !config.profile().is_dir() {
            return Err(OperationError::new(
                "profile_not_found",
                "Studio Pro profile does not exist; pass --profile",
            ));
        }
        let profile = lock_file(&config.profile().join(".fender-studio-pro-mcp.lock"))?;
        let mailbox = lock_file(&config.data_dir().join("controller.lock"))?;
        Ok(Self {
            _profile: profile,
            _mailbox: mailbox,
        })
    }
}

fn lock_file(path: &Path) -> Result<File, OperationError> {
    if fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(OperationError::new(
            "unsafe_path",
            "Refusing a symlink lock file",
        ));
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(io_error)?;
    file.try_lock_exclusive().map_err(|_| {
        OperationError::new(
            "controller_busy",
            "Another process owns this Studio Pro profile or mailbox",
        )
    })?;
    Ok(file)
}

pub fn private_dir(path: &Path) -> Result<(), OperationError> {
    if fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(OperationError::new(
            "unsafe_path",
            "Refusing a symlink data directory",
        ));
    }
    fs::create_dir_all(path).map_err(io_error)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(io_error)?;
    }
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WireRequest<'a> {
    protocol: u32,
    id: &'a str,
    expected_session: Option<&'a str>,
    expected_document: Option<&'a str>,
    op: &'a str,
    args: &'a Value,
}

#[derive(Debug, Deserialize)]
pub struct WireResponse {
    pub protocol: u32,
    pub id: String,
    pub session: String,
    pub document: String,
    pub ok: bool,
    #[serde(default)]
    pub result: Value,
    pub error: Option<Value>,
    pub code: Option<String>,
}

pub struct Bridge {
    directory: PathBuf,
    timeout: Duration,
}

#[derive(Debug)]
pub struct ExchangeFailure {
    pub dispatched: bool,
    pub error: OperationError,
}

pub struct StagedFixture {
    path: PathBuf,
    pub basename: String,
}

impl StagedFixture {
    pub fn remove(self) {
        if let Err(error) = fs::remove_file(self.path) {
            tracing::warn!(%error, "Could not remove the private bridge fixture");
        }
    }
}

impl Bridge {
    pub fn new(config: &Config) -> Result<Self, OperationError> {
        private_dir(&config.mailbox())?;
        Ok(Self {
            directory: config.mailbox(),
            timeout: config.timeout(),
        })
    }

    pub fn transact(
        &mut self,
        op: &str,
        args: &Value,
        expected_session: Option<&str>,
        expected_document: Option<&str>,
        midi: &mut MidiWakeup,
    ) -> Result<WireResponse, ExchangeFailure> {
        self.exchange_with(op, args, expected_session, expected_document, || {
            midi.nudge()
        })
    }

    pub(crate) fn transact_bounded(
        &self,
        op: &str,
        args: &Value,
        expected_session: Option<&str>,
        expected_document: Option<&str>,
        midi: &mut MidiWakeup,
        limit: Duration,
    ) -> Result<WireResponse, ExchangeFailure> {
        self.exchange_until(
            op,
            args,
            expected_session,
            expected_document,
            self.timeout.min(limit),
            || midi.nudge(),
        )
    }

    pub fn stage_midi_program(&self, contents: &[u8; 46]) -> Result<StagedFixture, OperationError> {
        let basename = format!("mcp-midi-program-{}.mid", Uuid::new_v4());
        self.stage_fixture(basename, contents)
    }

    pub(crate) fn stage_serum_preset(
        &self,
        contents: &[u8],
    ) -> Result<StagedFixture, OperationError> {
        let basename = format!("mcp-serum2-preset-{}.vstpreset", Uuid::new_v4());
        self.stage_fixture(basename, contents)
    }

    fn stage_fixture(
        &self,
        basename: String,
        contents: &[u8],
    ) -> Result<StagedFixture, OperationError> {
        let path = self.directory.join(&basename);
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&path).map_err(io_error)?;
        if let Err(error) = file.write_all(contents) {
            drop(file);
            let _ = fs::remove_file(&path);
            return Err(io_error(error));
        }
        Ok(StagedFixture { path, basename })
    }

    fn exchange_with(
        &mut self,
        op: &str,
        args: &Value,
        expected_session: Option<&str>,
        expected_document: Option<&str>,
        wake: impl FnMut() -> Result<(), OperationError>,
    ) -> Result<WireResponse, ExchangeFailure> {
        self.exchange_until(
            op,
            args,
            expected_session,
            expected_document,
            self.timeout,
            wake,
        )
    }

    fn exchange_until(
        &self,
        op: &str,
        args: &Value,
        expected_session: Option<&str>,
        expected_document: Option<&str>,
        timeout: Duration,
        mut wake: impl FnMut() -> Result<(), OperationError>,
    ) -> Result<WireResponse, ExchangeFailure> {
        let id = Uuid::new_v4().to_string();
        let request = WireRequest {
            protocol: PROTOCOL,
            id: &id,
            expected_session,
            expected_document,
            op,
            args,
        };
        let mut bytes = serde_json::to_vec(&request).map_err(|e| ExchangeFailure {
            dispatched: false,
            error: OperationError::new("invalid_request", e.to_string()),
        })?;
        if bytes.len() > MAX_REQUEST_BYTES {
            return Err(ExchangeFailure {
                dispatched: false,
                error: OperationError::new("request_too_large", "Request exceeds 1 MiB"),
            });
        }
        bytes.push(b'\n');
        let temporary = self.directory.join(format!("request.{id}.tmp"));
        let published = self.directory.join("request.json");
        atomic_write(&temporary, &published, &bytes).map_err(|error| ExchangeFailure {
            dispatched: false,
            error,
        })?;
        // The bridge can read the published request on any tick, including
        // before this process sends its first wakeup.
        let dispatched = true;
        let deadline = Instant::now() + timeout;
        let mut last_wake = None;
        while Instant::now() < deadline {
            if last_wake.is_none_or(|t: Instant| t.elapsed() >= Duration::from_millis(150)) {
                wake().map_err(|error| ExchangeFailure { dispatched, error })?;
                last_wake = Some(Instant::now());
            }
            if let Some(response) = read_response(&self.directory.join("response.json")) {
                if response.id != id {
                    thread::sleep(Duration::from_millis(20));
                    continue;
                }
                if response.protocol != PROTOCOL {
                    return Err(ExchangeFailure {
                        dispatched,
                        error: OperationError::new(
                            "protocol_mismatch",
                            "Bridge protocol does not match this server; reinstall bridge",
                        ),
                    });
                }
                if expected_session.is_some_and(|session| response.session != session) {
                    return Err(ExchangeFailure {
                        dispatched,
                        error: OperationError::new(
                            "session_changed",
                            "Bridge session changed during operation",
                        ),
                    });
                }
                return Ok(response);
            }
            thread::sleep(Duration::from_millis(20));
        }
        Err(ExchangeFailure {
            dispatched,
            error: OperationError::new(
                "bridge_timeout",
                "No matching response. Open a Session, then check the bridge device and its Receive From MIDI port",
            ),
        })
    }
}

pub fn read_json(path: &Path) -> Result<Value, OperationError> {
    let metadata = fs::symlink_metadata(path).map_err(io_error)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > MAX_RESPONSE_BYTES
    {
        return Err(OperationError::new(
            "unsafe_response",
            "Response is not a bounded regular file",
        ));
    }
    let mut text = String::new();
    File::open(path)
        .map_err(io_error)?
        .take(MAX_RESPONSE_BYTES + 1)
        .read_to_string(&mut text)
        .map_err(io_error)?;
    serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .map_err(|e| OperationError::new("invalid_json", e.to_string()))
}

fn read_response(path: &Path) -> Option<WireResponse> {
    serde_json::from_value(read_json(path).ok()?).ok()
}

pub(crate) fn atomic_write(
    temporary: &Path,
    published: &Path,
    bytes: &[u8],
) -> Result<(), OperationError> {
    if fs::symlink_metadata(published).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(OperationError::new(
            "unsafe_path",
            "Refusing a symlink request file",
        ));
    }
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let result = (|| {
        let mut file = options.open(temporary).map_err(io_error)?;
        file.write_all(bytes).map_err(io_error)?;
        file.sync_all().map_err(io_error)?;
        fs::rename(temporary, published).map_err(io_error)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

pub fn io_error(error: std::io::Error) -> OperationError {
    OperationError::new("io_error", error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn staged_midi_is_private_unique_and_removable() {
        let temp = tempfile::tempdir().unwrap();
        let config = Config {
            data_dir: Some(temp.path().to_owned()),
            profile: None,
            timeout_ms: 100,
        };
        let bridge = Bridge::new(&config).unwrap();
        let first = bridge.stage_midi_program(&[1; 46]).unwrap();
        let second = bridge.stage_midi_program(&[2; 46]).unwrap();
        assert_ne!(first.basename, second.basename);
        assert_eq!(
            Uuid::parse_str(
                first
                    .basename
                    .strip_prefix("mcp-midi-program-")
                    .unwrap()
                    .strip_suffix(".mid")
                    .unwrap()
            )
            .unwrap()
            .get_version_num(),
            4
        );
        assert_eq!(fs::read(&first.path).unwrap(), [1; 46]);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&first.path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        let removed = first.path.clone();
        first.remove();
        assert!(!removed.exists());
        assert_eq!(fs::read(&second.path).unwrap(), [2; 46]);
    }

    #[test]
    fn staged_serum_preset_is_a_private_mailbox_fixture() {
        let temp = tempfile::tempdir().unwrap();
        let config = Config {
            data_dir: Some(temp.path().to_owned()),
            profile: None,
            timeout_ms: 100,
        };
        let bridge = Bridge::new(&config).unwrap();
        let fixture = bridge.stage_serum_preset(b"fixture bytes").unwrap();
        assert_eq!(fixture.path, config.mailbox().join(&fixture.basename));
        let id = fixture
            .basename
            .strip_prefix("mcp-serum2-preset-")
            .unwrap()
            .strip_suffix(".vstpreset")
            .unwrap();
        assert_eq!(Uuid::parse_str(id).unwrap().get_version_num(), 4);
        assert_eq!(fs::read(&fixture.path).unwrap(), b"fixture bytes");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&fixture.path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        let removed = fixture.path.clone();
        fixture.remove();
        assert!(!removed.exists());
    }

    #[test]
    fn second_owner_cannot_use_same_profile_with_another_mailbox() {
        let temp = tempfile::tempdir().unwrap();
        let profile = temp.path().join("profile");
        fs::create_dir(&profile).unwrap();
        let a = Config {
            data_dir: Some(temp.path().join("a")),
            profile: Some(profile.clone()),
            timeout_ms: 100,
        };
        let b = Config {
            data_dir: Some(temp.path().join("b")),
            profile: Some(profile),
            timeout_ms: 100,
        };
        let _owner = OwnerLock::acquire(&a).unwrap();
        assert_eq!(
            OwnerLock::acquire(&b).err().unwrap().code,
            "controller_busy"
        );
    }

    #[test]
    fn partial_and_stale_responses_are_not_results() {
        let temp = tempfile::tempdir().unwrap();
        let config = Config {
            data_dir: Some(temp.path().to_owned()),
            profile: None,
            timeout_ms: 300,
        };
        let mut bridge = Bridge::new(&config).unwrap();
        fs::write(config.mailbox().join("response.json"), "{").unwrap();
        let result = bridge.exchange_with("ping", &json!({}), None, None, || Ok(()));
        assert_eq!(result.err().unwrap().error.code, "bridge_timeout");
    }

    #[test]
    fn response_must_match_the_request_and_protocol() {
        let temp = tempfile::tempdir().unwrap();
        let config = Config {
            data_dir: Some(temp.path().to_owned()),
            profile: None,
            timeout_ms: 500,
        };
        let mut bridge = Bridge::new(&config).unwrap();
        let mailbox = config.mailbox();
        let result = bridge.exchange_with("ping", &json!({}), None, None, || {
            let req = read_json(&mailbox.join("request.json")).unwrap();
            fs::write(mailbox.join("response.json"), serde_json::to_vec(&json!({
                "protocol":2,"id":req["id"],"session":"s","document":"d","ok":true,"result":{"pong":true}
            })).unwrap()).unwrap();
            Ok(())
        }).unwrap();
        assert_eq!(result.result["pong"], true);
    }
}

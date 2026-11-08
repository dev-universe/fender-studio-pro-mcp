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

    fn exchange_with(
        &mut self,
        op: &str,
        args: &Value,
        expected_session: Option<&str>,
        expected_document: Option<&str>,
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
        let deadline = Instant::now() + self.timeout;
        let mut last_wake = None;
        let mut dispatched = false;
        while Instant::now() < deadline {
            if last_wake.is_none_or(|t: Instant| t.elapsed() >= Duration::from_millis(150)) {
                dispatched = true;
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

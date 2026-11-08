use std::{
    collections::BTreeMap,
    fs,
    io::{Cursor, Write},
    path::{Path, PathBuf},
};

use include_dir::{Dir, include_dir};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use zip::{ZipWriter, write::SimpleFileOptions};

use crate::{
    bridge::{OwnerLock, io_error, private_dir, read_json},
    config::Config,
    model::OperationError,
};

static ASSETS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets");

#[derive(Debug, Serialize, Deserialize)]
struct Installation {
    version: String,
    files: BTreeMap<PathBuf, String>,
}

pub fn install(config: &Config, force: bool) -> Result<Value, OperationError> {
    let _lock = OwnerLock::acquire(config)?;
    private_dir(&config.mailbox())?;
    let device = config.profile().join("User Devices/fender-studio-pro-mcp");
    let extension = config
        .profile()
        .join("Extensions/fender-studio-pro-mcp.edittasks");
    let manifest_path = config.data_dir().join("installation.json");
    let old: Option<Installation> = fs::read(&manifest_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    let files = planned_files(config)?;
    for (path, bytes) in &files {
        if path.is_symlink() {
            return Err(OperationError::new(
                "unsafe_path",
                "Refusing a symlink installation target",
            ));
        }
        if path.exists() {
            let current = fs::read(path).map_err(io_error)?;
            let known = old.as_ref().and_then(|m| m.files.get(path));
            if known.is_none() && current != *bytes {
                return Err(OperationError::new(
                    "unknown_installation",
                    "A target file is not owned by this installation; preserve it and choose another profile",
                ));
            }
            if known.is_some_and(|hash| *hash != digest(&current)) && current != *bytes && !force {
                return Err(OperationError::new(
                    "modified_installation",
                    "An installed file was modified. Back it up or explicitly use install --force",
                ));
            }
        }
    }
    let backup_root = config
        .data_dir()
        .join("backups")
        .join(uuid::Uuid::new_v4().to_string());
    let mut backed_up = Vec::new();
    for (path, bytes) in &files {
        if path.exists() && fs::read(path).map_err(io_error)? != *bytes {
            private_dir(&backup_root)?;
            let relative = path
                .strip_prefix(config.profile())
                .map_err(|_| {
                    OperationError::new(
                        "unsafe_path",
                        "Installation target is outside the selected profile",
                    )
                })?
                .to_owned();
            let backup = backup_root.join(relative);
            fs::create_dir_all(backup.parent().unwrap()).map_err(io_error)?;
            fs::copy(path, &backup).map_err(io_error)?;
            backed_up.push(backup);
        }
    }
    let mut manifest = Installation {
        version: env!("CARGO_PKG_VERSION").into(),
        files: BTreeMap::new(),
    };
    for (path, bytes) in files {
        let parent = path.parent().unwrap();
        if parent.is_symlink() {
            return Err(OperationError::new(
                "unsafe_path",
                "Refusing a symlink installation directory",
            ));
        }
        fs::create_dir_all(parent).map_err(io_error)?;
        write_complete_file(&path, &bytes)?;
        manifest.files.insert(path, digest(&bytes));
    }
    write_complete_file(
        &manifest_path,
        &serde_json::to_vec_pretty(&manifest).unwrap(),
    )?;
    Ok(json!({
        "installed":true, "version":env!("CARGO_PKG_VERSION"), "device":device,
        "extension":extension, "midi_port":crate::midi::PORT_NAME, "backups":backed_up,
        "next":"Restart Studio Pro, add fender-studio-pro-mcp / Fender MCP Bridge in External Devices, and set Receive From to the published MIDI source."
    }))
}

pub fn uninstall(config: &Config) -> Result<Value, OperationError> {
    let _lock = OwnerLock::acquire(config)?;
    let manifest_path = config.data_dir().join("installation.json");
    let manifest: Installation = serde_json::from_value(read_json(&manifest_path)?)
        .map_err(|e| OperationError::new("invalid_manifest", e.to_string()))?;
    let profile = config.profile();
    let allowed = [
        profile.join("User Devices/fender-studio-pro-mcp"),
        profile.join("Extensions/fender-studio-pro-mcp.edittasks"),
    ];
    let mut removed = Vec::new();
    let mut preserved = Vec::new();
    for (path, expected) in manifest.files {
        if !allowed.iter().any(|root| path.starts_with(root))
            || path
                .components()
                .any(|c| c == std::path::Component::ParentDir)
        {
            return Err(OperationError::new(
                "unsafe_manifest",
                "Manifest points outside the managed installation",
            ));
        }
        if !path.exists() {
            continue;
        }
        if path.is_symlink() || digest(&fs::read(&path).map_err(io_error)?) != expected {
            preserved.push(path);
            continue;
        }
        fs::remove_file(&path).map_err(io_error)?;
        removed.push(path);
    }
    if preserved.is_empty() {
        fs::remove_file(manifest_path).map_err(io_error)?;
    }
    Ok(json!({"removed":removed,"preserved_modified":preserved,"settings_changed":false}))
}

pub fn doctor(config: &Config) -> Value {
    let device = config.profile().join("User Devices/fender-studio-pro-mcp");
    let extension = config
        .profile()
        .join("Extensions/fender-studio-pro-mcp.edittasks/scripts/fender-studio-pro-mcp.package");
    let status = read_json(&config.mailbox().join("status.json")).ok();
    let comparison = planned_files(config).map(|files| {
        files
            .into_iter()
            .filter_map(|(path, expected)| {
                if path.is_symlink() || !fs::read(&path).is_ok_and(|actual| actual == expected) {
                    Some(path)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    });
    let installation_current = comparison.as_ref().is_ok_and(Vec::is_empty);
    let stale_files = comparison.unwrap_or_default();
    json!({
        "server_version":env!("CARGO_PKG_VERSION"),"bridge_protocol":crate::bridge::PROTOCOL,
        "platform":std::env::consts::OS,"architecture":std::env::consts::ARCH,
        "profile_exists":config.profile().is_dir(),
        "device_installed":device.join("StudioOneMCP.device").is_file(),
        "edit_tasks_installed":extension.is_file(),"status_file":status,
        "installation_current":installation_current,"stale_or_missing_files":stale_files,
        "live_verified":false,
        "note":"status.json reports a previous load only. Use doctor --live for a current request/response check."
    })
}

fn planned_files(config: &Config) -> Result<BTreeMap<PathBuf, Vec<u8>>, OperationError> {
    let device = config.profile().join("User Devices/fender-studio-pro-mcp");
    let extension = config
        .profile()
        .join("Extensions/fender-studio-pro-mcp.edittasks");
    let mut files = BTreeMap::new();
    let bridge = ASSETS
        .get_dir("bridge")
        .ok_or_else(|| OperationError::new("missing_asset", "Embedded bridge is missing"))?;
    for file in bridge.files() {
        files.insert(
            device.join(file.path().file_name().unwrap()),
            file.contents().to_vec(),
        );
    }
    let host_config = json!({"mailbox": file_url(&config.mailbox()), "allowEval": false});
    files.insert(
        device.join("BridgeConfig.js"),
        format!("var BridgeConfig = {};\n", host_config).into_bytes(),
    );
    let extension_assets = ASSETS.get_dir("edit-tasks/extension").ok_or_else(|| {
        OperationError::new("missing_asset", "Embedded EditTask extension is missing")
    })?;
    for file in extension_assets.files() {
        files.insert(
            extension.join(file.path().file_name().unwrap()),
            file.contents().to_vec(),
        );
    }
    files.insert(
        extension.join("scripts/fender-studio-pro-mcp.package"),
        edit_package(&host_config)?,
    );
    Ok(files)
}

fn file_url(path: &Path) -> String {
    format!("file://{}/", path.to_string_lossy().trim_end_matches('/'))
}

fn edit_package(config: &Value) -> Result<Vec<u8>, OperationError> {
    let package = ASSETS
        .get_dir("edit-tasks/package")
        .ok_or_else(|| OperationError::new("missing_asset", "Embedded edit package is missing"))?;
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for file in package.files() {
        writer
            .start_file(file.path().file_name().unwrap().to_string_lossy(), options)
            .map_err(|e| OperationError::new("package_error", e.to_string()))?;
        writer.write_all(file.contents()).map_err(io_error)?;
    }
    writer
        .start_file("McpEditConfig.js", options)
        .map_err(|e| OperationError::new("package_error", e.to_string()))?;
    writer
        .write_all(format!("var McpEditConfig = {};\n", config).as_bytes())
        .map_err(io_error)?;
    writer
        .finish()
        .map(|cursor| cursor.into_inner())
        .map_err(|e| OperationError::new("package_error", e.to_string()))
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn write_complete_file(path: &Path, bytes: &[u8]) -> Result<(), OperationError> {
    let temporary = path.with_file_name(format!(
        ".{}.{}.tmp",
        path.file_name().unwrap().to_string_lossy(),
        uuid::Uuid::new_v4()
    ));
    crate::bridge::atomic_write(&temporary, path, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installer_preserves_unknown_and_modified_files() {
        let temp = tempfile::tempdir().unwrap();
        let profile = temp.path().join("profile");
        fs::create_dir(&profile).unwrap();
        let config = Config {
            data_dir: Some(temp.path().join("data")),
            profile: Some(profile.clone()),
            timeout_ms: 100,
        };
        install(&config, false).unwrap();
        let path = profile.join("User Devices/fender-studio-pro-mcp/BridgeDevice.js");
        fs::write(&path, "user changes").unwrap();
        assert_eq!(
            install(&config, false).unwrap_err().code,
            "modified_installation"
        );
        let result = uninstall(&config).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "user changes");
        assert_eq!(result["preserved_modified"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn force_update_keeps_a_backup_and_doctor_detects_drift() {
        let temp = tempfile::tempdir().unwrap();
        let profile = temp.path().join("profile");
        fs::create_dir(&profile).unwrap();
        let config = Config {
            data_dir: Some(temp.path().join("data")),
            profile: Some(profile.clone()),
            timeout_ms: 100,
        };
        install(&config, false).unwrap();
        assert_eq!(doctor(&config)["installation_current"], true);
        let path = profile.join("User Devices/fender-studio-pro-mcp/BridgeDevice.js");
        fs::write(&path, "custom bridge edit").unwrap();
        assert_eq!(doctor(&config)["installation_current"], false);
        let updated = install(&config, true).unwrap();
        let backups = updated["backups"].as_array().unwrap();
        assert_eq!(backups.len(), 1);
        assert_eq!(
            fs::read_to_string(backups[0].as_str().unwrap()).unwrap(),
            "custom bridge edit"
        );
        assert_eq!(doctor(&config)["installation_current"], true);
    }
}

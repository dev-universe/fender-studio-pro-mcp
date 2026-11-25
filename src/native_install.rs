use std::{collections::BTreeMap, fs, io::Read, path::Path};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    bridge::{OwnerLock, io_error, private_dir},
    config::Config,
    install::{digest, write_complete_file},
    model::OperationError,
};

const EXTENSION: &str = "fender-studio-pro-mcp.native.mac";
const FILES: [&str; 4] = [
    "metainfo.xml",
    "installdata.xml",
    "plugins/mac/fender_mcp_native.bundle/Contents/Info.plist",
    "plugins/mac/fender_mcp_native.bundle/Contents/MacOS/fender_mcp_native",
];
const SIGNATURE_RESOURCES: &str =
    "plugins/mac/fender_mcp_native.bundle/Contents/_CodeSignature/CodeResources";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: u32,
    root: std::path::PathBuf,
    files: BTreeMap<String, String>,
}

fn error(code: &str, message: &str) -> OperationError {
    OperationError::new(code, message)
}

fn manifest(config: &Config) -> Result<Option<Manifest>, OperationError> {
    let path = config.data_dir().join("native-installation.json");
    reject_symlinks(&path)?;
    if !path.exists() && !path.is_symlink() {
        return Ok(None);
    }
    let value: Manifest = serde_json::from_slice(&read_regular(&path, 16_384)?)
        .map_err(|e| OperationError::new("invalid_manifest", e.to_string()))?;
    if value.schema != 1
        || value.root != config.profile().join("Extensions").join(EXTENSION)
        || value.files.len()
            != FILES.len() + usize::from(value.files.contains_key(SIGNATURE_RESOURCES))
        || !FILES.iter().all(|name| value.files.contains_key(*name))
    {
        return Err(error(
            "unsafe_manifest",
            "Unexpected native installation manifest",
        ));
    }
    Ok(Some(value))
}

fn maximum(name: &str) -> u64 {
    match name {
        name if name == FILES[3] => 32_000_000,
        SIGNATURE_RESOURCES => 262_144,
        _ => 32_768,
    }
}

fn read_regular(path: &Path, maximum: u64) -> Result<Vec<u8>, OperationError> {
    reject_symlinks(path)?;
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(path).map_err(io_error)?;
    let metadata = file.metadata().map_err(io_error)?;
    if !metadata.is_file() || metadata.len() > maximum {
        return Err(error(
            "invalid_native_file",
            "Expected a bounded regular native extension file",
        ));
    }
    let mut bytes = Vec::new();
    file.take(maximum + 1)
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    if bytes.len() as u64 > maximum {
        return Err(error(
            "invalid_native_file",
            "Native extension file exceeds the supported size",
        ));
    }
    Ok(bytes)
}

fn reject_symlinks(path: &Path) -> Result<(), OperationError> {
    if path.ancestors().any(Path::is_symlink) {
        return Err(error(
            "unsafe_path",
            "Refusing a symlink in the native extension path",
        ));
    }
    Ok(())
}

pub fn install(config: &Config, source: &Path, force: bool) -> Result<Value, OperationError> {
    if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return Err(error(
            "unsupported_platform",
            "The native extension supports macOS arm64 only",
        ));
    }
    let _owner = OwnerLock::acquire(config)?;
    let root = config.profile().join("Extensions").join(EXTENSION);
    let previous = manifest(config)?;
    let mut contents = BTreeMap::new();
    let mut originals = BTreeMap::new();
    for name in FILES.into_iter().chain([SIGNATURE_RESOURCES]) {
        let input = source.join(name);
        let output = root.join(name);
        reject_symlinks(&input)?;
        reject_symlinks(&output)?;
        if name != SIGNATURE_RESOURCES || input.try_exists().map_err(io_error)? {
            let bytes = read_regular(&input, maximum(name))?;
            if bytes.is_empty() {
                return Err(error(
                    "invalid_native_package",
                    "Native payload has an unsupported size",
                ));
            }
            if name == FILES[3] && !bytes.starts_with(&[0xcf, 0xfa, 0xed, 0xfe, 0x0c, 0, 0, 1]) {
                return Err(error(
                    "invalid_native_package",
                    "Expected a native arm64 Mach-O payload",
                ));
            }
            contents.insert(name, bytes);
        }
        if output.try_exists().map_err(io_error)? {
            let current = read_regular(&output, maximum(name))?;
            let known = previous.as_ref().and_then(|m| m.files.get(name));
            let changed = contents.get(name) != Some(&current);
            if changed && known.is_none() {
                return Err(error(
                    "unknown_installation",
                    "Native target is not owned by this installation",
                ));
            }
            if changed && known.is_some_and(|hash| *hash != digest(&current)) && !force {
                return Err(error(
                    "modified_installation",
                    "Native target was modified; use --force to back it up and replace it",
                ));
            }
            let permissions = fs::metadata(&output).map_err(io_error)?.permissions();
            originals.insert(name, (current, permissions));
        }
    }
    // Preflight all payloads before replacing any managed file.
    let backup = config
        .data_dir()
        .join("backups")
        .join(uuid::Uuid::new_v4().to_string());
    let mut backups = Vec::new();
    for (name, (original, _)) in &originals {
        if contents.get(name) != Some(original) {
            let target = backup.join(EXTENSION).join(name);
            reject_symlinks(&target)?;
            fs::create_dir_all(target.parent().unwrap()).map_err(io_error)?;
            write_complete_file(&target, original)?;
            backups.push(target);
        }
    }
    let installed = Manifest {
        schema: 1,
        root: root.clone(),
        files: contents
            .iter()
            .map(|(name, bytes)| ((*name).into(), digest(bytes)))
            .collect(),
    };
    private_dir(&config.data_dir())?;
    let mut touched = Vec::new();
    let result = (|| {
        for (name, bytes) in contents {
            let output = root.join(name);
            fs::create_dir_all(output.parent().unwrap()).map_err(io_error)?;
            write_complete_file(&output, &bytes)?;
            touched.push(name);
            #[cfg(unix)]
            if name == FILES[3] {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&output, fs::Permissions::from_mode(0o755))
                    .map_err(io_error)?;
            }
        }
        if !installed.files.contains_key(SIGNATURE_RESOURCES)
            && originals.contains_key(SIGNATURE_RESOURCES)
        {
            fs::remove_file(root.join(SIGNATURE_RESOURCES)).map_err(io_error)?;
            touched.push(SIGNATURE_RESOURCES);
        }
        write_complete_file(
            &config.data_dir().join("native-installation.json"),
            &serde_json::to_vec_pretty(&installed).unwrap(),
        )
    })();
    if let Err(install_error) = result {
        let mut rollback_failed = false;
        for name in touched.into_iter().rev() {
            let target = root.join(name);
            let restored = if let Some((bytes, permissions)) = originals.get(name) {
                write_complete_file(&target, bytes).and_then(|()| {
                    fs::set_permissions(&target, permissions.clone()).map_err(io_error)
                })
            } else {
                fs::remove_file(&target).map_err(io_error)
            };
            rollback_failed |= restored.is_err();
        }
        if rollback_failed {
            return Err(error(
                "native_install_rollback_failed",
                &format!(
                    "Native install failed and some assets could not be restored: {}",
                    install_error.message
                ),
            ));
        }
        return Err(install_error);
    }
    Ok(json!({"installed":true,"extension":root,"backups":backups,
        "restart_required":true,"compatibility":"Studio Pro 8.1.1 Build 113103, macOS arm64; native runtime verifies image identities"}))
}

pub fn uninstall(config: &Config) -> Result<Value, OperationError> {
    let _owner = OwnerLock::acquire(config)?;
    let Some(manifest) = manifest(config)? else {
        return Ok(json!({"removed":[],"preserved_modified":[],"restart_required":true}));
    };
    let root = config.profile().join("Extensions").join(EXTENSION);
    let mut removed = Vec::new();
    let mut preserved = Vec::new();
    for (name, expected) in manifest.files {
        let path = root.join(&name);
        if !path.exists() && !path.is_symlink() {
            continue;
        }
        if reject_symlinks(&path).is_err()
            || !read_regular(&path, maximum(&name)).is_ok_and(|bytes| digest(&bytes) == expected)
        {
            preserved.push(path);
        } else {
            fs::remove_file(&path).map_err(io_error)?;
            removed.push(path);
        }
    }
    if preserved.is_empty() {
        fs::remove_file(config.data_dir().join("native-installation.json")).map_err(io_error)?;
    }
    Ok(json!({"removed":removed,"preserved_modified":preserved,"restart_required":true}))
}

pub fn doctor(config: &Config) -> Value {
    let root = config.profile().join("Extensions").join(EXTENSION);
    match manifest(config) {
        Ok(Some(manifest)) => {
            let stale: Vec<_> = manifest
                .files
                .into_iter()
                .filter_map(|(name, hash)| {
                    let path = root.join(&name);
                    (reject_symlinks(&path).is_err()
                        || !read_regular(&path, maximum(&name))
                            .is_ok_and(|bytes| digest(&bytes) == hash))
                    .then_some(path)
                })
                .collect();
            json!({"managed":true,"files_match_manifest":stale.is_empty(),"stale_or_missing_files":stale,"live_verified":false})
        }
        Ok(None) => json!({"managed":false,"live_verified":false}),
        Err(err) => json!({"managed":false,"error":err,"live_verified":false}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(temp: &Path) -> (Config, std::path::PathBuf) {
        let temp = temp.canonicalize().unwrap();
        let config = Config {
            data_dir: Some(temp.join("data")),
            profile: Some(temp.join("profile")),
            timeout_ms: 100,
        };
        let source = temp.join("source");
        fs::create_dir_all(config.profile()).unwrap();
        for name in FILES {
            let path = source.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(
                path,
                if name == FILES[3] {
                    &[0xcf, 0xfa, 0xed, 0xfe, 0x0c, 0, 0, 1][..]
                } else {
                    b"fixture"
                },
            )
            .unwrap();
        }
        (config, source)
    }

    #[test]
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    fn preflight_preserves_unknown_files_and_update_backs_up_modified_files() {
        let temp = tempfile::tempdir().unwrap();
        let (config, source) = fixture(temp.path());
        let root = config.profile().join("Extensions").join(EXTENSION);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join(FILES[0]), b"unowned").unwrap();
        assert_eq!(
            install(&config, &source, true).unwrap_err().code,
            "unknown_installation"
        );
        assert!(!root.join(FILES[3]).exists());
        fs::write(root.join(FILES[0]), b"fixture").unwrap();
        install(&config, &source, false).unwrap();
        fs::write(root.join(FILES[0]), b"user edit").unwrap();
        assert_eq!(doctor(&config)["files_match_manifest"], false);
        assert_eq!(
            install(&config, &source, false).unwrap_err().code,
            "modified_installation"
        );
        let updated = install(&config, &source, true).unwrap();
        let backups = updated["backups"].as_array().unwrap();
        assert_eq!(backups.len(), 1);
        assert_eq!(
            fs::read(backups[0].as_str().unwrap()).unwrap(),
            b"user edit"
        );
        assert_eq!(doctor(&config)["files_match_manifest"], true);
        fs::write(root.join(FILES[2]), b"preserve").unwrap();
        assert_eq!(
            uninstall(&config).unwrap()["preserved_modified"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(fs::read(root.join(FILES[2])).unwrap(), b"preserve");
    }

    #[test]
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    fn owns_signature_resources_when_updating_and_restoring_a_legacy_bundle() {
        let temp = tempfile::tempdir().unwrap();
        let (config, source) = fixture(temp.path());
        let root = config.profile().join("Extensions").join(EXTENSION);
        let source_signature = source.join(SIGNATURE_RESOURCES);
        let installed_signature = root.join(SIGNATURE_RESOURCES);

        install(&config, &source, false).unwrap();
        assert_eq!(manifest(&config).unwrap().unwrap().files.len(), 4);
        fs::create_dir_all(source_signature.parent().unwrap()).unwrap();
        fs::write(&source_signature, b"package signature").unwrap();
        install(&config, &source, false).unwrap();
        assert_eq!(manifest(&config).unwrap().unwrap().files.len(), 5);
        assert_eq!(
            fs::read(&installed_signature).unwrap(),
            b"package signature"
        );
        assert_eq!(doctor(&config)["files_match_manifest"], true);

        fs::remove_file(&source_signature).unwrap();
        fs::write(&installed_signature, b"modified signature").unwrap();
        assert_eq!(
            install(&config, &source, false).unwrap_err().code,
            "modified_installation"
        );
        assert_eq!(
            fs::read(&installed_signature).unwrap(),
            b"modified signature"
        );
        let restore = install(&config, &source, true).unwrap();
        let backups = restore["backups"].as_array().unwrap();
        assert_eq!(backups.len(), 1);
        assert_eq!(
            fs::read(backups[0].as_str().unwrap()).unwrap(),
            b"modified signature"
        );
        assert!(!installed_signature.exists());
        assert_eq!(doctor(&config)["files_match_manifest"], true);

        fs::write(&source_signature, b"package signature").unwrap();
        install(&config, &source, false).unwrap();
        assert_eq!(
            uninstall(&config).unwrap()["removed"]
                .as_array()
                .unwrap()
                .len(),
            5
        );
        assert!(!installed_signature.exists());
    }

    #[test]
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    fn a_signature_destination_failure_rolls_back_files_and_keeps_the_manifest() {
        use std::os::unix::fs::PermissionsExt;
        if unsafe { libc::geteuid() } == 0 {
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let (config, source) = fixture(temp.path());
        install(&config, &source, false).unwrap();
        let root = config.profile().join("Extensions").join(EXTENSION);
        let manifest_path = config.data_dir().join("native-installation.json");
        let before = fs::read(&manifest_path).unwrap();

        fs::write(source.join(FILES[0]), b"new metadata").unwrap();
        let source_signature = source.join(SIGNATURE_RESOURCES);
        fs::create_dir_all(source_signature.parent().unwrap()).unwrap();
        fs::write(&source_signature, b"signature").unwrap();
        let installed_signature = root.join(SIGNATURE_RESOURCES);
        let signature_directory = installed_signature.parent().unwrap();
        fs::create_dir_all(signature_directory).unwrap();
        fs::set_permissions(signature_directory, fs::Permissions::from_mode(0o500)).unwrap();

        let result = install(&config, &source, false);
        fs::set_permissions(signature_directory, fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(result.unwrap_err().code, "io_error");
        assert_eq!(fs::read(root.join(FILES[0])).unwrap(), b"fixture");
        assert_eq!(fs::read(&manifest_path).unwrap(), before);
        assert!(!installed_signature.exists());
        assert_eq!(doctor(&config)["files_match_manifest"], true);
    }

    #[test]
    #[cfg(unix)]
    fn uninstall_cannot_follow_symlinks_or_manifest_traversal() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        let (config, _) = fixture(temp.path());
        private_dir(&config.data_dir()).unwrap();
        let root = config.profile().join("Extensions").join(EXTENSION);
        fs::create_dir_all(&root).unwrap();
        let outside = temp.path().join("outside");
        fs::write(&outside, b"preserve").unwrap();
        symlink(&outside, root.join(FILES[0])).unwrap();
        let mut files: BTreeMap<String, String> = FILES
            .into_iter()
            .map(|name| (name.into(), digest(b"preserve")))
            .collect();
        let path = config.data_dir().join("native-installation.json");
        fs::write(
            &path,
            serde_json::to_vec(&Manifest {
                schema: 1,
                root: root.clone(),
                files: files.clone(),
            })
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            uninstall(&config).unwrap()["preserved_modified"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(fs::read(&outside).unwrap(), b"preserve");
        files.remove(FILES[0]);
        files.insert("../../../outside".into(), digest(b"preserve"));
        fs::write(
            path,
            serde_json::to_vec(&Manifest {
                schema: 1,
                root: root.clone(),
                files,
            })
            .unwrap(),
        )
        .unwrap();
        assert_eq!(uninstall(&config).unwrap_err().code, "unsafe_manifest");
        assert_eq!(fs::read(&outside).unwrap(), b"preserve");
    }

    #[test]
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    fn ownership_is_bound_to_the_selected_profile() {
        let temp = tempfile::tempdir().unwrap();
        let (config, source) = fixture(temp.path());
        install(&config, &source, false).unwrap();
        let other = Config {
            profile: Some(temp.path().canonicalize().unwrap().join("profile-b")),
            ..config.clone()
        };
        let other_root = other.profile().join("Extensions").join(EXTENSION);
        for name in FILES {
            let path = other_root.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::copy(source.join(name), path).unwrap();
        }
        let original_manifest =
            fs::read(config.data_dir().join("native-installation.json")).unwrap();
        assert_eq!(uninstall(&other).unwrap_err().code, "unsafe_manifest");
        assert_eq!(
            install(&other, &source, true).unwrap_err().code,
            "unsafe_manifest"
        );
        assert_eq!(fs::read(other_root.join(FILES[0])).unwrap(), b"fixture");
        assert_eq!(
            fs::read(config.data_dir().join("native-installation.json")).unwrap(),
            original_manifest
        );
        assert_eq!(doctor(&config)["files_match_manifest"], true);
    }

    #[test]
    #[cfg(unix)]
    fn reads_reject_fifos_oversize_and_dangling_manifest_without_blocking() {
        use std::{ffi::CString, os::unix::ffi::OsStrExt, os::unix::fs::symlink};
        let temp = tempfile::tempdir().unwrap();
        let (config, _) = fixture(temp.path());
        let fifo = temp.path().canonicalize().unwrap().join("fifo");
        let path = CString::new(fifo.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
        assert_eq!(
            read_regular(&fifo, 16_384).unwrap_err().code,
            "invalid_native_file"
        );
        let large = temp.path().canonicalize().unwrap().join("large");
        fs::File::create(&large).unwrap().set_len(16_385).unwrap();
        assert_eq!(
            read_regular(&large, 16_384).unwrap_err().code,
            "invalid_native_file"
        );
        private_dir(&config.data_dir()).unwrap();
        symlink(
            config.data_dir().join("absent"),
            config.data_dir().join("native-installation.json"),
        )
        .unwrap();
        assert_eq!(manifest(&config).err().unwrap().code, "unsafe_path");
    }
}

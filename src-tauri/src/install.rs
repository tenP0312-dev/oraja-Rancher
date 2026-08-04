use crate::manifest::{verify_file, ManifestError, ReleaseManifest};
use serde::Serialize;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;
use thiserror::Error;

const MINIMUM_JAVA_MAJOR: u32 = 21;
const UPDATE_STAGING_DIRECTORY: &str = ".bmsir-update-staging";
const UPDATE_BACKUP_DIRECTORY: &str = ".bmsir-launcher-backup";
const UPDATE_HELPER: &str = if cfg!(windows) {
    ".bmsir-launcher-update-helper.exe"
} else {
    ".bmsir-launcher-update-helper"
};

#[derive(Debug, Error)]
pub enum InstallError {
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("installation root is invalid")]
    InvalidRoot,
    #[error("more than one BMS-IR plugin jar exists in ir/")]
    DuplicatePlugins,
    #[error("the selected Java runtime must be Java 21 or newer")]
    UnsupportedJava,
    #[error("an update path is a symlink or leaves its selected root")]
    UnsafeFilesystemPath,
    #[error("the signed update does not contain this launcher executable")]
    LauncherArtifactMissing,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstallationInfo {
    pub root: String,
    pub game_jar: Option<String>,
    pub java_runtime: Option<String>,
    pub java_source: Option<String>,
    pub java_version: Option<u32>,
    pub plugin_jars: Vec<String>,
}

pub fn is_ready(installation: &InstallationInfo) -> bool {
    installation.game_jar.is_some()
        && installation.java_runtime.is_some()
        && installation.plugin_jars.len() == 1
}

pub fn inspect(root: &Path) -> Result<InstallationInfo, InstallError> {
    if !root.is_dir() {
        return Err(InstallError::InvalidRoot);
    }
    let mut game_jars = fs::read_dir(root)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().and_then(|value| value.to_str()) == Some("jar")
                && path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .is_some_and(|name| {
                        let lower = name.to_ascii_lowercase();
                        lower.contains("lr2oraja")
                            || lower.contains("beatoraja")
                            || lower.contains("bms-ir-arena")
                    })
        })
        .collect::<Vec<_>>();
    game_jars.sort_by_key(|path| game_jar_priority(path));
    let game_jar = game_jars.into_iter().next();
    let plugin_jars = plugin_jars(root)?;
    if plugin_jars.len() > 1 {
        return Err(InstallError::DuplicatePlugins);
    }
    let bundled_java = bundled_java_candidates(root)
        .into_iter()
        .find(|path| path.is_file());
    let (java_runtime, java_source) = if let Some(path) = bundled_java {
        (Some(path), Some("bundled".to_string()))
    } else if let Some(path) = system_java() {
        (Some(path), Some("system".to_string()))
    } else {
        (None, None)
    };
    let java_version = java_runtime.as_deref().and_then(java_major_version);
    let (java_runtime, java_source) = if java_version.is_some_and(is_supported_java_major) {
        (java_runtime, java_source)
    } else {
        (None, None)
    };
    Ok(InstallationInfo {
        root: root.to_string_lossy().into_owned(),
        game_jar: game_jar.map(|path| path.to_string_lossy().into_owned()),
        java_runtime: java_runtime.map(|path| path.to_string_lossy().into_owned()),
        java_source,
        java_version,
        plugin_jars: plugin_jars
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect(),
    })
}

fn game_jar_priority(path: &Path) -> (u8, String) {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let priority = if name == "beatoraja.jar" {
        0
    } else if name.contains("bms-ir-arena") {
        1
    } else if name.contains("lr2oraja") {
        2
    } else {
        3
    };
    (priority, name)
}

pub fn inspect_java(path: &Path) -> Result<u32, InstallError> {
    let version = java_major_version(path).ok_or(InstallError::UnsupportedJava)?;
    if !is_supported_java_major(version) {
        return Err(InstallError::UnsupportedJava);
    }
    Ok(version)
}

fn is_supported_java_major(version: u32) -> bool {
    version >= MINIMUM_JAVA_MAJOR
}

fn bundled_java_candidates(root: &Path) -> [PathBuf; 4] {
    [
        root.join("runtime/bin/java.exe"),
        root.join("runtime/bin/java"),
        root.join("jre/bin/java.exe"),
        root.join("jre/bin/java"),
    ]
}

fn java_major_version(path: &Path) -> Option<u32> {
    if !path.is_file() {
        return None;
    }
    let output = Command::new(path).arg("-version").output().ok()?;
    parse_java_major(&format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

fn parse_java_major(value: &str) -> Option<u32> {
    let marker = value.find("version")?;
    let tail = &value[marker + "version".len()..];
    let quoted = tail.split('"').nth(1).unwrap_or(tail).trim();
    let first = quoted.split('.').next()?.trim();
    let major = first.parse::<u32>().ok()?;
    if major == 1 {
        quoted.split('.').nth(1)?.parse().ok()
    } else {
        Some(major)
    }
}

fn system_java() -> Option<PathBuf> {
    if let Some(home) = std::env::var_os("JAVA_HOME") {
        for executable in ["bin/java", "bin/java.exe"] {
            let candidate = PathBuf::from(&home).join(executable);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    let path = std::env::var_os("PATH")?;
    for directory in std::env::split_paths(&path) {
        for executable in ["java", "java.exe"] {
            let candidate = directory.join(executable);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn plugin_jars(root: &Path) -> Result<Vec<PathBuf>, io::Error> {
    let directory = root.join("ir");
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    let mut result: Vec<PathBuf> = fs::read_dir(directory)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().and_then(|value| value.to_str()) == Some("jar")
                && path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .is_some_and(|name| name.to_ascii_lowercase().starts_with("bms_ir"))
        })
        .collect();
    result.sort();
    Ok(result)
}

fn has_bmsir_plugin_filename(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| {
            name.to_ascii_lowercase().starts_with("bms_ir")
                && path
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| value.eq_ignore_ascii_case("jar"))
        })
}

fn is_bmsir_plugin_path(path: &Path) -> bool {
    path.parent()
        .and_then(Path::to_str)
        .is_some_and(|parent| parent.eq_ignore_ascii_case("ir"))
        && has_bmsir_plugin_filename(path)
}

fn normalized_relative_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase()
}

fn paths_refer_to_same_install_target(left: &Path, right: &Path) -> bool {
    if cfg!(windows) {
        normalized_relative_path(left) == normalized_relative_path(right)
    } else {
        left == right
    }
}

fn plugin_path_from_artifacts(
    artifacts: &[crate::manifest::ReleaseArtifact],
) -> Result<Option<&Path>, InstallError> {
    let mut plugins = Vec::new();
    for artifact in artifacts {
        let path = Path::new(&artifact.path);
        if !has_bmsir_plugin_filename(path) {
            continue;
        }
        if !is_bmsir_plugin_path(path) {
            return Err(InstallError::UnsafeFilesystemPath);
        }
        plugins.push(path);
    }
    let mut plugins = plugins.into_iter();
    let plugin = plugins.next();
    if plugins.next().is_some() {
        return Err(InstallError::DuplicatePlugins);
    }
    Ok(plugin)
}

fn effective_artifacts(manifest: &ReleaseManifest) -> Vec<crate::manifest::ReleaseArtifact> {
    let mut artifacts = manifest
        .bootstrap
        .as_ref()
        .map(|bootstrap| bootstrap.artifacts.clone())
        .unwrap_or_default();
    for replacement in &manifest.artifacts {
        if let Some(index) = artifacts
            .iter()
            .position(|artifact| artifact.path.eq_ignore_ascii_case(&replacement.path))
        {
            artifacts[index] = replacement.clone();
        } else {
            artifacts.push(replacement.clone());
        }
    }
    artifacts
}

fn verified_staged_path(
    staging: &Path,
    artifact: &crate::manifest::ReleaseArtifact,
) -> Result<Option<PathBuf>, InstallError> {
    let source = staging.join(&artifact.path);
    if !source.exists() {
        return Ok(None);
    }
    let canonical = source.canonicalize()?;
    if !canonical.starts_with(staging)
        || !canonical.is_file()
        || source.symlink_metadata()?.file_type().is_symlink()
    {
        return Err(InstallError::UnsafeFilesystemPath);
    }
    verify_file(&canonical, artifact)?;
    Ok(Some(canonical))
}

fn verify_installed_artifact(
    root: &Path,
    artifact: &crate::manifest::ReleaseArtifact,
) -> Result<(), InstallError> {
    let destination = root.join(&artifact.path);
    ensure_destination_is_safe(root, &destination)?;
    if !destination.is_file() || destination.symlink_metadata()?.file_type().is_symlink() {
        return Err(InstallError::UnsafeFilesystemPath);
    }
    verify_file(&destination, artifact)?;
    Ok(())
}

fn verified_update_artifacts(
    root: &Path,
    staging: &Path,
    manifest: &ReleaseManifest,
    bootstrap_install: bool,
) -> Result<Vec<crate::manifest::ReleaseArtifact>, InstallError> {
    let expected = if bootstrap_install {
        effective_artifacts(manifest)
    } else {
        manifest.artifacts.clone()
    };
    let mut staged = Vec::new();
    for artifact in expected {
        if verified_staged_path(staging, &artifact)?.is_some() {
            staged.push(artifact);
        } else if bootstrap_install {
            return Err(InstallError::InvalidRoot);
        } else {
            verify_installed_artifact(root, &artifact)?;
        }
    }
    Ok(staged)
}

#[cfg(test)]
pub fn apply_staged(
    root: &Path,
    staging: &Path,
    manifest: &ReleaseManifest,
) -> Result<(), InstallError> {
    apply_staged_mode(root, staging, manifest, false)
}

pub fn apply_staged_mode(
    root: &Path,
    staging: &Path,
    manifest: &ReleaseManifest,
    bootstrap_install: bool,
) -> Result<(), InstallError> {
    if !root.is_dir() || !staging.is_dir() {
        return Err(InstallError::InvalidRoot);
    }
    if plugin_jars(root)?.len() > 1 {
        return Err(InstallError::DuplicatePlugins);
    }
    let root = root.canonicalize()?;
    let staging = staging.canonicalize()?;
    let staged_artifacts = verified_update_artifacts(&root, &staging, manifest, bootstrap_install)?;
    let replacement_plugin = plugin_path_from_artifacts(&staged_artifacts)?.map(Path::to_path_buf);

    let backup = root.join(UPDATE_BACKUP_DIRECTORY);
    if backup.exists() {
        fs::remove_dir_all(&backup)?;
    }
    fs::create_dir_all(&backup)?;
    let mut installed: Vec<(PathBuf, PathBuf, bool)> = Vec::new();
    let mut displaced_plugins: Vec<(PathBuf, PathBuf)> = Vec::new();
    let result = (|| -> Result<(), InstallError> {
        if let (Some(replacement), Some(existing)) = (
            replacement_plugin.as_deref(),
            plugin_jars(&root)?.into_iter().next(),
        ) {
            let existing_relative = existing
                .strip_prefix(&root)
                .map_err(|_| InstallError::UnsafeFilesystemPath)?;
            if !paths_refer_to_same_install_target(existing_relative, replacement) {
                ensure_destination_is_safe(&root, &existing)?;
                let backup_path = backup.join(existing_relative);
                if let Some(parent) = backup_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::rename(&existing, &backup_path)?;
                displaced_plugins.push((existing, backup_path));
            }
        }
        for artifact in &staged_artifacts {
            let source = staging.join(&artifact.path);
            let destination = root.join(&artifact.path);
            ensure_destination_is_safe(&root, &destination)?;
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            let backup_path = backup.join(&artifact.path);
            let existed = destination.exists();
            if existed {
                if let Some(parent) = backup_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::rename(&destination, &backup_path)?;
            }
            installed.push((destination.clone(), backup_path.clone(), existed));
            let temporary = destination.with_extension("bmsir-new");
            if temporary.exists() {
                fs::remove_file(&temporary)?;
            }
            fs::copy(&source, &temporary)?;
            set_executable_if_requested(&temporary, artifact.executable)?;
            fs::rename(&temporary, &destination)?;
        }
        if plugin_jars(&root)?.len() > 1 {
            return Err(InstallError::DuplicatePlugins);
        }
        Ok(())
    })();
    if let Err(error) = result {
        for (destination, backup_path, existed) in installed.into_iter().rev() {
            let _ = fs::remove_file(&destination);
            let temporary = destination.with_extension("bmsir-new");
            let _ = fs::remove_file(temporary);
            if existed && backup_path.exists() {
                if let Some(parent) = destination.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let _ = fs::rename(&backup_path, &destination);
            }
        }
        for (destination, backup_path) in displaced_plugins.into_iter().rev() {
            let _ = fs::remove_file(&destination);
            if backup_path.exists() {
                if let Some(parent) = destination.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let _ = fs::rename(&backup_path, &destination);
            }
        }
        return Err(error);
    }
    Ok(())
}

fn ensure_destination_is_safe(root: &Path, destination: &Path) -> Result<(), InstallError> {
    if !destination.starts_with(root) {
        return Err(InstallError::UnsafeFilesystemPath);
    }
    let relative = destination
        .strip_prefix(root)
        .map_err(|_| InstallError::UnsafeFilesystemPath)?;
    let mut cursor = root.to_path_buf();
    for component in relative.components() {
        cursor.push(component);
        if cursor.exists() && cursor.symlink_metadata()?.file_type().is_symlink() {
            return Err(InstallError::UnsafeFilesystemPath);
        }
    }
    Ok(())
}

#[cfg(unix)]
pub(crate) fn set_executable_if_requested(path: &Path, executable: bool) -> Result<(), io::Error> {
    use std::os::unix::fs::PermissionsExt;
    if executable {
        let mut permissions = path.metadata()?.permissions();
        permissions.set_mode(permissions.mode() | 0o755);
        fs::set_permissions(path, permissions)?;
    }
    Ok(())
}

#[cfg(not(unix))]
pub(crate) fn set_executable_if_requested(
    _path: &Path,
    _executable: bool,
) -> Result<(), io::Error> {
    Ok(())
}

pub fn launcher_install_root() -> Result<PathBuf, InstallError> {
    let executable = std::env::current_exe()?.canonicalize()?;
    for ancestor in executable.ancestors() {
        if ancestor
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("app"))
        {
            return ancestor
                .parent()
                .map(Path::to_path_buf)
                .ok_or(InstallError::InvalidRoot);
        }
    }
    executable
        .parent()
        .map(Path::to_path_buf)
        .ok_or(InstallError::InvalidRoot)
}

pub fn staged_launcher_artifact_path(
    root: &Path,
    staging: &Path,
    manifest: &ReleaseManifest,
) -> Result<String, InstallError> {
    let executable = std::env::current_exe()?.canonicalize()?;
    let root = root.canonicalize()?;
    let relative = executable
        .strip_prefix(&root)
        .map_err(|_| InstallError::LauncherArtifactMissing)?;
    let normalized = relative.to_string_lossy().replace('\\', "/");
    let artifact = manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.path.eq_ignore_ascii_case(&normalized))
        .ok_or(InstallError::LauncherArtifactMissing)?;
    if verified_staged_path(&staging.canonicalize()?, artifact)?.is_none() {
        return Err(InstallError::LauncherArtifactMissing);
    }
    Ok(artifact.path.clone())
}

pub fn spawn_self_update(
    staging: &Path,
    manifest_path: &Path,
    manifest: &ReleaseManifest,
    bootstrap_install: bool,
    launch_after: bool,
) -> Result<(), InstallError> {
    let root = launcher_install_root()?;
    let launcher_path = staged_launcher_artifact_path(&root, staging, manifest)?;
    verified_update_artifacts(
        &root.canonicalize()?,
        &staging.canonicalize()?,
        manifest,
        bootstrap_install,
    )?;
    let staged_launcher = staging.join(&launcher_path).canonicalize()?;
    let helper = root.join(UPDATE_HELPER);
    match helper.symlink_metadata() {
        Ok(metadata) => {
            if metadata.is_dir() && !metadata.file_type().is_symlink() {
                return Err(InstallError::UnsafeFilesystemPath);
            }
            fs::remove_file(&helper)?;
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    fs::copy(&staged_launcher, &helper)?;
    set_executable_if_requested(&helper, true)?;
    Command::new(helper)
        .arg("--apply-self-update")
        .arg(&root)
        .arg(staging)
        .arg(manifest_path)
        .arg(launcher_path)
        .arg(if bootstrap_install { "1" } else { "0" })
        .arg(if launch_after { "1" } else { "0" })
        .spawn()?;
    Ok(())
}

pub fn run_self_update_helper(
    root: &Path,
    staging: &Path,
    manifest: &ReleaseManifest,
    launcher_path: &str,
    bootstrap_install: bool,
    launch_after: bool,
) -> Result<(), InstallError> {
    let mut last_error = None;
    for _ in 0..40 {
        thread::sleep(Duration::from_millis(250));
        match apply_staged_mode(root, staging, manifest, bootstrap_install) {
            Ok(()) => {
                let installation = inspect(root)?;
                if !is_ready(&installation) {
                    return Err(InstallError::InvalidRoot);
                }
                write_version_marker(root, &manifest.version)?;
                cleanup_completed_update(root, staging);
                if launch_after {
                    let java = installation
                        .java_runtime
                        .as_deref()
                        .map(Path::new)
                        .ok_or(InstallError::UnsupportedJava)?;
                    let game = installation
                        .game_jar
                        .as_deref()
                        .map(Path::new)
                        .ok_or(InstallError::InvalidRoot)?;
                    launch(root, java, game, false)?;
                } else {
                    Command::new(root.join(launcher_path)).spawn()?;
                }
                return Ok(());
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or(InstallError::InvalidRoot))
}

fn remove_launcher_owned_path(path: &Path) -> Result<(), io::Error> {
    if !path.exists() {
        return Ok(());
    }
    let metadata = path.symlink_metadata()?;
    if metadata.file_type().is_symlink() || metadata.is_file() {
        fs::remove_file(path)
    } else if metadata.is_dir() {
        fs::remove_dir_all(path)
    } else {
        Ok(())
    }
}

fn cleanup_completed_update(root: &Path, staging: &Path) {
    let Ok(root) = root.canonicalize() else {
        return;
    };
    let staging_parent = root.join(UPDATE_STAGING_DIRECTORY);
    let staging_is_owned = staging
        .canonicalize()
        .is_ok_and(|path| path.starts_with(&staging_parent));
    if !staging_is_owned {
        return;
    }
    let _ = remove_launcher_owned_path(&root.join(UPDATE_BACKUP_DIRECTORY));
    let _ = remove_launcher_owned_path(&staging_parent);
}

pub fn cleanup_stale_update_state(root: &Path) {
    let root = root.to_path_buf();
    thread::spawn(move || {
        for _ in 0..40 {
            let staging = remove_launcher_owned_path(&root.join(UPDATE_STAGING_DIRECTORY));
            let backup = remove_launcher_owned_path(&root.join(UPDATE_BACKUP_DIRECTORY));
            let helper = remove_launcher_owned_path(&root.join(UPDATE_HELPER));
            if staging.is_ok() && backup.is_ok() && helper.is_ok() {
                return;
            }
            thread::sleep(Duration::from_millis(250));
        }
    });
}

pub fn write_version_marker(root: &Path, version: &str) -> Result<(), InstallError> {
    let destination = root.join("bmsir-arena-version.txt");
    fs::write(destination, format!("{}\n", version.trim()))?;
    Ok(())
}

fn game_arguments(root: &Path, game_jar: &Path, configuration: bool) -> Vec<OsString> {
    let mut arguments = vec![
        OsString::from(format!(
            "-DcustomIRDirectory={}",
            root.join("ir").to_string_lossy()
        )),
        OsString::from("-Xms1g"),
        OsString::from("-Xmx4g"),
        OsString::from("-jar"),
        game_jar.as_os_str().to_os_string(),
    ];
    if configuration {
        arguments.push(OsString::from("-c"));
    } else {
        arguments.push(OsString::from("-s"));
    }
    arguments
}

pub fn launch(
    root: &Path,
    java: &Path,
    game_jar: &Path,
    configuration: bool,
) -> Result<(), InstallError> {
    let root = root.canonicalize()?;
    let java = java.canonicalize()?;
    let game_jar = game_jar.canonicalize()?;
    if !game_jar.starts_with(&root) {
        return Err(InstallError::InvalidRoot);
    }
    inspect_java(&java)?;
    let mut command = Command::new(java);
    command
        .current_dir(&root)
        .args(game_arguments(&root, &game_jar, configuration));
    command.spawn()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{ReleaseArtifact, ReleaseBootstrap};
    use sha2::{Digest, Sha256};

    #[test]
    fn staged_install_replaces_only_verified_files_and_keeps_backup() {
        let root = tempfile::tempdir().unwrap();
        let staging = tempfile::tempdir().unwrap();
        fs::write(root.path().join("game.jar"), b"old").unwrap();
        fs::write(staging.path().join("game.jar"), b"new").unwrap();
        let manifest = ReleaseManifest {
            schema_version: 1,
            channel: "stable".into(),
            platform: "windows-x64".into(),
            version: "1".into(),
            published_at: "now".into(),
            release_notes_markdown: String::new(),
            release_notes_markdown_ja: String::new(),
            release_notes_markdown_en: String::new(),
            announcements: vec![],
            mandatory: false,
            minimum_launcher_version: "0.1.0".into(),
            revoked_versions: vec![],
            bootstrap: None,
            artifacts: vec![ReleaseArtifact {
                path: "game.jar".into(),
                sha256: format!("{:x}", Sha256::digest(b"new")),
                size: 3,
                executable: false,
            }],
            signature: String::new(),
        };
        apply_staged(root.path(), staging.path(), &manifest).unwrap();
        assert_eq!(fs::read(root.path().join("game.jar")).unwrap(), b"new");
        assert_eq!(
            fs::read(root.path().join(".bmsir-launcher-backup/game.jar")).unwrap(),
            b"old"
        );
    }

    #[test]
    fn delta_install_skips_matching_files_that_were_not_staged() {
        let root = tempfile::tempdir().unwrap();
        let staging = tempfile::tempdir().unwrap();
        fs::write(root.path().join("same.dat"), b"same").unwrap();
        fs::write(root.path().join("changed.dat"), b"old").unwrap();
        fs::write(staging.path().join("changed.dat"), b"new").unwrap();
        let artifact = |path: &str, bytes: &[u8]| ReleaseArtifact {
            path: path.into(),
            sha256: format!("{:x}", Sha256::digest(bytes)),
            size: bytes.len() as u64,
            executable: false,
        };
        let manifest = ReleaseManifest {
            schema_version: 1,
            channel: "test".into(),
            platform: "windows-x64".into(),
            version: "2".into(),
            published_at: "now".into(),
            release_notes_markdown: String::new(),
            release_notes_markdown_ja: String::new(),
            release_notes_markdown_en: String::new(),
            announcements: vec![],
            mandatory: false,
            minimum_launcher_version: "0.2.8".into(),
            revoked_versions: vec![],
            bootstrap: None,
            artifacts: vec![
                artifact("same.dat", b"same"),
                artifact("changed.dat", b"new"),
            ],
            signature: String::new(),
        };

        apply_staged_mode(root.path(), staging.path(), &manifest, false).unwrap();

        assert_eq!(fs::read(root.path().join("same.dat")).unwrap(), b"same");
        assert_eq!(fs::read(root.path().join("changed.dat")).unwrap(), b"new");
        assert!(!root.path().join(".bmsir-launcher-backup/same.dat").exists());
    }

    #[test]
    fn bootstrap_install_requires_full_effective_inventory() {
        let root = tempfile::tempdir().unwrap();
        let staging = tempfile::tempdir().unwrap();
        fs::create_dir_all(staging.path().join("runtime/bin")).unwrap();
        fs::create_dir_all(staging.path().join("ir")).unwrap();
        fs::write(staging.path().join("Arena.jar"), b"new-body").unwrap();
        fs::write(staging.path().join("runtime/bin/java.exe"), b"java").unwrap();
        fs::write(staging.path().join("ir/bms_ir_arena.jar"), b"plugin").unwrap();
        let artifact = |path: &str, bytes: &[u8], executable: bool| ReleaseArtifact {
            path: path.into(),
            sha256: format!("{:x}", Sha256::digest(bytes)),
            size: bytes.len() as u64,
            executable,
        };
        let manifest = ReleaseManifest {
            schema_version: 1,
            channel: "test".into(),
            platform: "windows-x64".into(),
            version: "2".into(),
            published_at: "now".into(),
            release_notes_markdown: String::new(),
            release_notes_markdown_ja: String::new(),
            release_notes_markdown_en: String::new(),
            announcements: vec![],
            mandatory: false,
            minimum_launcher_version: "0.2.8".into(),
            revoked_versions: vec![],
            bootstrap: Some(ReleaseBootstrap {
                url: "https://example.test/bootstrap.zip".into(),
                sha256: "00".repeat(32),
                size: 1,
                artifacts: vec![
                    artifact("Arena.jar", b"old-body", false),
                    artifact("runtime/bin/java.exe", b"java", true),
                    artifact("ir/bms_ir_arena.jar", b"plugin", false),
                ],
            }),
            artifacts: vec![artifact("Arena.jar", b"new-body", false)],
            signature: String::new(),
        };

        apply_staged_mode(root.path(), staging.path(), &manifest, true).unwrap();
        assert_eq!(
            fs::read(root.path().join("Arena.jar")).unwrap(),
            b"new-body"
        );

        fs::remove_file(root.path().join("runtime/bin/java.exe")).unwrap();
        fs::remove_file(staging.path().join("runtime/bin/java.exe")).unwrap();
        assert!(apply_staged_mode(root.path(), staging.path(), &manifest, true).is_err());
    }

    #[test]
    fn duplicate_plugin_jars_block_installation() {
        let root = tempfile::tempdir().unwrap();
        let staging = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("ir")).unwrap();
        fs::write(root.path().join("ir/bms_ir_a.jar"), b"a").unwrap();
        fs::write(root.path().join("ir/bms_ir_b.jar"), b"b").unwrap();
        let manifest = ReleaseManifest {
            schema_version: 1,
            channel: "stable".into(),
            platform: "windows-x64".into(),
            version: "1".into(),
            published_at: "now".into(),
            release_notes_markdown: String::new(),
            release_notes_markdown_ja: String::new(),
            release_notes_markdown_en: String::new(),
            announcements: vec![],
            mandatory: false,
            minimum_launcher_version: "0.1.0".into(),
            revoked_versions: vec![],
            bootstrap: None,
            artifacts: vec![],
            signature: String::new(),
        };
        assert!(matches!(
            apply_staged(root.path(), staging.path(), &manifest),
            Err(InstallError::DuplicatePlugins)
        ));
    }

    #[test]
    fn parses_modern_and_legacy_java_versions() {
        assert_eq!(parse_java_major("openjdk version \"17.0.12\""), Some(17));
        assert_eq!(parse_java_major("java version \"1.8.0_412\""), Some(8));
        assert_eq!(parse_java_major("openjdk version \"21\""), Some(21));
        assert!(!is_supported_java_major(17));
        assert!(is_supported_java_major(21));
        assert!(is_supported_java_major(25));
    }

    #[test]
    fn bundled_java_candidates_include_current_windows_layout_first() {
        let candidates = bundled_java_candidates(Path::new("arena"));
        assert_eq!(candidates[0], Path::new("arena/runtime/bin/java.exe"));
        assert!(candidates.contains(&Path::new("arena/runtime/bin/java").to_path_buf()));
        assert!(candidates.contains(&Path::new("arena/jre/bin/java.exe").to_path_buf()));
    }

    #[test]
    fn installation_prefers_the_canonical_updated_game_jar() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("LR2oraja-EndlessDream.jar"), b"legacy").unwrap();
        fs::write(
            root.path()
                .join("BMS-IR-Arena-oraja-0.4.14-macos-aarch64.jar"),
            b"versioned",
        )
        .unwrap();
        let canonical = root.path().join("beatoraja.jar");
        fs::write(&canonical, b"updated").unwrap();

        let installation = inspect(root.path()).unwrap();

        assert_eq!(installation.game_jar.as_deref(), canonical.to_str());
    }

    #[test]
    fn portable_launch_keeps_bat_memory_and_plugin_arguments() {
        let arguments = game_arguments(Path::new("arena root"), Path::new("beatoraja.jar"), true);
        assert_eq!(
            arguments[0],
            OsString::from(format!(
                "-DcustomIRDirectory={}",
                Path::new("arena root").join("ir").to_string_lossy()
            ))
        );
        assert_eq!(arguments[1], "-Xms1g");
        assert_eq!(arguments[2], "-Xmx4g");
        assert_eq!(arguments[3], "-jar");
        assert_eq!(arguments[4], "beatoraja.jar");
        assert_eq!(arguments[5], "-c");
    }

    #[test]
    fn normal_launch_enters_music_select_directly() {
        let arguments = game_arguments(Path::new("arena root"), Path::new("beatoraja.jar"), false);
        assert_eq!(arguments[4], "beatoraja.jar");
        assert_eq!(arguments[5], "-s");
    }

    #[test]
    fn version_marker_replaces_an_existing_version() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("bmsir-arena-version.txt"), "0.4.13\n").unwrap();
        write_version_marker(root.path(), "0.4.14").unwrap();
        assert_eq!(
            fs::read_to_string(root.path().join("bmsir-arena-version.txt")).unwrap(),
            "0.4.14\n"
        );
    }

    #[test]
    fn completed_update_cleanup_removes_staging_and_backup() {
        let root = tempfile::tempdir().unwrap();
        let staging = root.path().join(UPDATE_STAGING_DIRECTORY).join("2");
        fs::create_dir_all(&staging).unwrap();
        fs::write(staging.join("artifact"), b"staged").unwrap();
        let backup = root.path().join(UPDATE_BACKUP_DIRECTORY);
        fs::create_dir_all(&backup).unwrap();
        fs::write(backup.join("artifact"), b"backup").unwrap();

        cleanup_completed_update(root.path(), &staging);

        assert!(!root.path().join(UPDATE_STAGING_DIRECTORY).exists());
        assert!(!backup.exists());
    }

    #[test]
    fn active_plugin_artifact_must_be_directly_under_ir() {
        assert!(is_bmsir_plugin_path(Path::new(
            "IR/bms_ir_arena_0.0.70.jar"
        )));
        assert!(!is_bmsir_plugin_path(Path::new(
            "nested/ir/bms_ir_arena_0.0.70.jar"
        )));
    }

    #[test]
    fn versioned_plugin_update_replaces_old_plugin_and_keeps_backup() {
        let root = tempfile::tempdir().unwrap();
        let staging = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("ir")).unwrap();
        fs::create_dir(staging.path().join("ir")).unwrap();
        fs::write(root.path().join("ir/bms_ir_arena_0.0.69.jar"), b"old").unwrap();
        fs::write(staging.path().join("ir/bms_ir_arena_0.0.70.jar"), b"new").unwrap();
        let manifest = ReleaseManifest {
            schema_version: 1,
            channel: "stable".into(),
            platform: "windows-x64".into(),
            version: "1".into(),
            published_at: "now".into(),
            release_notes_markdown: String::new(),
            release_notes_markdown_ja: String::new(),
            release_notes_markdown_en: String::new(),
            announcements: vec![],
            mandatory: false,
            minimum_launcher_version: "0.1.0".into(),
            revoked_versions: vec![],
            bootstrap: None,
            artifacts: vec![ReleaseArtifact {
                path: "ir/bms_ir_arena_0.0.70.jar".into(),
                sha256: format!("{:x}", Sha256::digest(b"new")),
                size: 3,
                executable: false,
            }],
            signature: String::new(),
        };

        apply_staged(root.path(), staging.path(), &manifest).unwrap();

        assert!(!root.path().join("ir/bms_ir_arena_0.0.69.jar").exists());
        assert_eq!(
            fs::read(root.path().join("ir/bms_ir_arena_0.0.70.jar")).unwrap(),
            b"new"
        );
        assert_eq!(
            fs::read(
                root.path()
                    .join(".bmsir-launcher-backup/ir/bms_ir_arena_0.0.69.jar")
            )
            .unwrap(),
            b"old"
        );
    }

    #[test]
    fn failed_versioned_plugin_update_restores_old_plugin() {
        let root = tempfile::tempdir().unwrap();
        let staging = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("ir")).unwrap();
        fs::create_dir(staging.path().join("ir")).unwrap();
        fs::write(root.path().join("ir/bms_ir_arena_0.0.69.jar"), b"old").unwrap();
        fs::write(staging.path().join("ir/bms_ir_arena_0.0.70.jar"), b"new").unwrap();
        fs::create_dir(staging.path().join("blocked")).unwrap();
        fs::write(staging.path().join("blocked/file.bin"), b"data").unwrap();
        fs::write(root.path().join("blocked"), b"not a directory").unwrap();
        let manifest = ReleaseManifest {
            schema_version: 1,
            channel: "stable".into(),
            platform: "windows-x64".into(),
            version: "1".into(),
            published_at: "now".into(),
            release_notes_markdown: String::new(),
            release_notes_markdown_ja: String::new(),
            release_notes_markdown_en: String::new(),
            announcements: vec![],
            mandatory: false,
            minimum_launcher_version: "0.1.0".into(),
            revoked_versions: vec![],
            bootstrap: None,
            artifacts: vec![
                ReleaseArtifact {
                    path: "ir/bms_ir_arena_0.0.70.jar".into(),
                    sha256: format!("{:x}", Sha256::digest(b"new")),
                    size: 3,
                    executable: false,
                },
                ReleaseArtifact {
                    path: "blocked/file.bin".into(),
                    sha256: format!("{:x}", Sha256::digest(b"data")),
                    size: 4,
                    executable: false,
                },
            ],
            signature: String::new(),
        };

        assert!(apply_staged(root.path(), staging.path(), &manifest).is_err());

        assert_eq!(
            fs::read(root.path().join("ir/bms_ir_arena_0.0.69.jar")).unwrap(),
            b"old"
        );
        assert!(!root.path().join("ir/bms_ir_arena_0.0.70.jar").exists());
    }

    #[test]
    fn update_with_multiple_plugin_artifacts_is_rejected_without_changes() {
        let root = tempfile::tempdir().unwrap();
        let staging = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("ir")).unwrap();
        fs::create_dir(staging.path().join("ir")).unwrap();
        fs::write(root.path().join("ir/bms_ir_arena_0.0.69.jar"), b"old").unwrap();
        fs::write(staging.path().join("ir/bms_ir_arena_0.0.70.jar"), b"one").unwrap();
        fs::write(staging.path().join("ir/bms_ir_arena_0.0.71.jar"), b"two").unwrap();
        let manifest = ReleaseManifest {
            schema_version: 1,
            channel: "stable".into(),
            platform: "windows-x64".into(),
            version: "1".into(),
            published_at: "now".into(),
            release_notes_markdown: String::new(),
            release_notes_markdown_ja: String::new(),
            release_notes_markdown_en: String::new(),
            announcements: vec![],
            mandatory: false,
            minimum_launcher_version: "0.1.0".into(),
            revoked_versions: vec![],
            bootstrap: None,
            artifacts: vec![
                ReleaseArtifact {
                    path: "ir/bms_ir_arena_0.0.70.jar".into(),
                    sha256: format!("{:x}", Sha256::digest(b"one")),
                    size: 3,
                    executable: false,
                },
                ReleaseArtifact {
                    path: "ir/bms_ir_arena_0.0.71.jar".into(),
                    sha256: format!("{:x}", Sha256::digest(b"two")),
                    size: 3,
                    executable: false,
                },
            ],
            signature: String::new(),
        };

        assert!(matches!(
            apply_staged(root.path(), staging.path(), &manifest),
            Err(InstallError::DuplicatePlugins)
        ));
        assert_eq!(
            fs::read(root.path().join("ir/bms_ir_arena_0.0.69.jar")).unwrap(),
            b"old"
        );
        assert!(!root.path().join(".bmsir-launcher-backup").exists());
    }

    #[test]
    fn nested_plugin_artifact_is_rejected_without_displacing_current_plugin() {
        let root = tempfile::tempdir().unwrap();
        let staging = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("ir")).unwrap();
        fs::create_dir_all(staging.path().join("nested/ir")).unwrap();
        fs::write(root.path().join("ir/bms_ir_arena_0.0.69.jar"), b"old").unwrap();
        fs::write(
            staging.path().join("nested/ir/bms_ir_arena_0.0.70.jar"),
            b"new",
        )
        .unwrap();
        let manifest = ReleaseManifest {
            schema_version: 1,
            channel: "stable".into(),
            platform: "windows-x64".into(),
            version: "1".into(),
            published_at: "now".into(),
            release_notes_markdown: String::new(),
            release_notes_markdown_ja: String::new(),
            release_notes_markdown_en: String::new(),
            announcements: vec![],
            mandatory: false,
            minimum_launcher_version: "0.1.0".into(),
            revoked_versions: vec![],
            bootstrap: None,
            artifacts: vec![ReleaseArtifact {
                path: "nested/ir/bms_ir_arena_0.0.70.jar".into(),
                sha256: format!("{:x}", Sha256::digest(b"new")),
                size: 3,
                executable: false,
            }],
            signature: String::new(),
        };

        assert!(matches!(
            apply_staged(root.path(), staging.path(), &manifest),
            Err(InstallError::UnsafeFilesystemPath)
        ));
        assert_eq!(
            fs::read(root.path().join("ir/bms_ir_arena_0.0.69.jar")).unwrap(),
            b"old"
        );
        assert!(!root.path().join(".bmsir-launcher-backup").exists());
    }

    #[cfg(unix)]
    #[test]
    fn staged_symlink_is_rejected_before_installation() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let staging = tempfile::tempdir().unwrap();
        let outside = tempfile::NamedTempFile::new().unwrap();
        symlink(outside.path(), staging.path().join("game.jar")).unwrap();
        let manifest = ReleaseManifest {
            schema_version: 1,
            channel: "stable".into(),
            platform: "windows-x64".into(),
            version: "1".into(),
            published_at: "now".into(),
            release_notes_markdown: String::new(),
            release_notes_markdown_ja: String::new(),
            release_notes_markdown_en: String::new(),
            announcements: vec![],
            mandatory: false,
            minimum_launcher_version: "0.1.0".into(),
            revoked_versions: vec![],
            bootstrap: None,
            artifacts: vec![ReleaseArtifact {
                path: "game.jar".into(),
                sha256: "00".repeat(32),
                size: 0,
                executable: false,
            }],
            signature: String::new(),
        };
        assert!(matches!(
            apply_staged(root.path(), staging.path(), &manifest),
            Err(InstallError::UnsafeFilesystemPath)
        ));
    }
}

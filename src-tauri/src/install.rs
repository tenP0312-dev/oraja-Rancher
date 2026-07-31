use crate::manifest::{verify_file, ManifestError, ReleaseManifest};
use serde::Serialize;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;
use thiserror::Error;

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
    #[error("the selected Java runtime is not Java 17")]
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

pub fn inspect(root: &Path) -> Result<InstallationInfo, InstallError> {
    if !root.is_dir() {
        return Err(InstallError::InvalidRoot);
    }
    let game_jar = fs::read_dir(root)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
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
        });
    let plugin_jars = plugin_jars(root)?;
    if plugin_jars.len() > 1 {
        return Err(InstallError::DuplicatePlugins);
    }
    let bundled_java = ["jre/bin/java", "runtime/bin/java", "jre/bin/java.exe"]
        .iter()
        .map(|relative| root.join(relative))
        .find(|path| path.is_file());
    let (java_runtime, java_source) = if let Some(path) = bundled_java {
        (Some(path), Some("bundled".to_string()))
    } else if let Some(path) = system_java() {
        (Some(path), Some("system".to_string()))
    } else {
        (None, None)
    };
    let java_version = java_runtime.as_deref().and_then(java_major_version);
    let (java_runtime, java_source) = if java_version == Some(17) {
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

pub fn inspect_java(path: &Path) -> Result<u32, InstallError> {
    let version = java_major_version(path).ok_or(InstallError::UnsupportedJava)?;
    if version != 17 {
        return Err(InstallError::UnsupportedJava);
    }
    Ok(version)
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

pub fn apply_staged(
    root: &Path,
    staging: &Path,
    manifest: &ReleaseManifest,
) -> Result<(), InstallError> {
    if !root.is_dir() || !staging.is_dir() {
        return Err(InstallError::InvalidRoot);
    }
    if plugin_jars(root)?.len() > 1 {
        return Err(InstallError::DuplicatePlugins);
    }
    let root = root.canonicalize()?;
    let staging = staging.canonicalize()?;
    verify_staged(&staging, manifest)?;

    let backup = root.join(".bmsir-launcher-backup");
    if backup.exists() {
        fs::remove_dir_all(&backup)?;
    }
    fs::create_dir_all(&backup)?;
    let mut installed: Vec<(PathBuf, PathBuf, bool)> = Vec::new();
    let result = (|| -> Result<(), InstallError> {
        for artifact in &manifest.artifacts {
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
        return Err(error);
    }
    Ok(())
}

pub fn verify_staged(staging: &Path, manifest: &ReleaseManifest) -> Result<(), InstallError> {
    let staging = staging.canonicalize()?;
    for artifact in &manifest.artifacts {
        let source = staging.join(&artifact.path);
        let canonical = source.canonicalize()?;
        if !canonical.starts_with(&staging)
            || !canonical.is_file()
            || source.symlink_metadata()?.file_type().is_symlink()
        {
            return Err(InstallError::UnsafeFilesystemPath);
        }
        verify_file(&canonical, artifact)?;
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
fn set_executable_if_requested(path: &Path, executable: bool) -> Result<(), io::Error> {
    use std::os::unix::fs::PermissionsExt;
    if executable {
        let mut permissions = path.metadata()?.permissions();
        permissions.set_mode(permissions.mode() | 0o755);
        fs::set_permissions(path, permissions)?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn set_executable_if_requested(_path: &Path, _executable: bool) -> Result<(), io::Error> {
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

pub fn launcher_artifact_path(
    root: &Path,
    manifest: &ReleaseManifest,
) -> Result<String, InstallError> {
    let executable = std::env::current_exe()?.canonicalize()?;
    let root = root.canonicalize()?;
    let relative = executable
        .strip_prefix(&root)
        .map_err(|_| InstallError::LauncherArtifactMissing)?;
    let normalized = relative.to_string_lossy().replace('\\', "/");
    manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.path.eq_ignore_ascii_case(&normalized))
        .map(|artifact| artifact.path.clone())
        .ok_or(InstallError::LauncherArtifactMissing)
}

pub fn spawn_self_update(
    staging: &Path,
    manifest_path: &Path,
    manifest: &ReleaseManifest,
) -> Result<(), InstallError> {
    let root = launcher_install_root()?;
    let launcher_path = launcher_artifact_path(&root, manifest)?;
    verify_staged(staging, manifest)?;
    let staged_launcher = staging.join(&launcher_path).canonicalize()?;
    Command::new(staged_launcher)
        .arg("--apply-self-update")
        .arg(&root)
        .arg(staging)
        .arg(manifest_path)
        .arg(launcher_path)
        .spawn()?;
    Ok(())
}

pub fn run_self_update_helper(
    root: &Path,
    staging: &Path,
    manifest: &ReleaseManifest,
    launcher_path: &str,
) -> Result<(), InstallError> {
    let mut last_error = None;
    for _ in 0..40 {
        thread::sleep(Duration::from_millis(250));
        match apply_staged(root, staging, manifest) {
            Ok(()) => {
                Command::new(root.join(launcher_path)).spawn()?;
                return Ok(());
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or(InstallError::InvalidRoot))
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
    command.current_dir(&root).arg("-jar").arg(game_jar);
    if configuration {
        command.arg("-c");
    }
    command.spawn()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::ReleaseArtifact;
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
            version: "1".into(),
            published_at: "now".into(),
            release_notes_markdown: String::new(),
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
    fn duplicate_plugin_jars_block_installation() {
        let root = tempfile::tempdir().unwrap();
        let staging = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("ir")).unwrap();
        fs::write(root.path().join("ir/bms_ir_a.jar"), b"a").unwrap();
        fs::write(root.path().join("ir/bms_ir_b.jar"), b"b").unwrap();
        let manifest = ReleaseManifest {
            schema_version: 1,
            channel: "stable".into(),
            version: "1".into(),
            published_at: "now".into(),
            release_notes_markdown: String::new(),
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
            version: "1".into(),
            published_at: "now".into(),
            release_notes_markdown: String::new(),
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

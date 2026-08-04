use crate::install::{set_executable_if_requested, CANONICAL_GAME_JAR};
use crate::manifest::{
    verify_file, verify_manifest, ReleaseAnnouncement, ReleaseArtifact, ReleaseBootstrap,
    ReleaseManifest,
};
use serde::Serialize;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;
use thiserror::Error;
use url::Url;

const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
const VERSION_FILE: &str = "bmsir-arena-version.txt";
const STAGING_DIRECTORY: &str = ".bmsir-update-staging";
const STAGED_MANIFEST: &str = ".bmsir-update-manifest.json";
const CACHED_POLICY: &str = ".bmsir-launcher-policy.json";
const CACHED_POLICY_TEMPORARY: &str = ".bmsir-launcher-policy.tmp";
const BOOTSTRAP_ARCHIVE: &str = ".bmsir-bootstrap.zip";
const DOWNLOAD_ATTEMPTS: usize = 4;
const PROGRESS_BYTES_STEP: u64 = 1024 * 1024;
const fn configured_value(value: Option<&str>) -> bool {
    match value {
        Some(value) => !value.is_empty(),
        None => false,
    }
}

pub const CONFIGURATION_MARKER: &str =
    if configured_value(option_env!("BMSIR_ARENA_UPDATE_BASE_URL"))
        && configured_value(option_env!("BMSIR_ARENA_RELEASE_PUBLIC_KEY"))
    {
        "BMSIR_ARENA_UPDATE_CONFIGURED_V1"
    } else {
        "BMSIR_ARENA_UPDATE_UNCONFIGURED_V1"
    };

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("update endpoint is not configured")]
    Endpoint,
    #[error("release verification key is not configured")]
    PublicKey,
    #[error("update URL is invalid")]
    Url,
    #[error("update request failed: {0}")]
    Request(String),
    #[error("update response is too large")]
    TooLarge,
    #[error("update response is incomplete")]
    Incomplete,
    #[error("the signed release does not contain a complete initial installation")]
    IncompleteBootstrap,
    #[error("manifest channel or platform does not match this launcher")]
    WrongTarget,
    #[error("update staging path is unsafe")]
    UnsafeStaging,
    #[error("bootstrap archive is invalid: {0}")]
    Archive(#[from] zip::result::ZipError),
    #[error("BMS-IR Arena {0} is a required update")]
    RequiredUpdate(String),
    #[error(transparent)]
    Manifest(#[from] crate::manifest::ManifestError),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    pub channel: String,
    pub platform: String,
    pub installed_version: String,
    pub available_version: String,
    pub status: String,
    pub mandatory: bool,
    pub release_notes_markdown: String,
    pub release_notes_markdown_ja: String,
    pub release_notes_markdown_en: String,
    pub announcements: Vec<ReleaseAnnouncement>,
}

#[derive(Debug, Clone)]
pub struct PreparedUpdate {
    pub manifest: ReleaseManifest,
    pub staging: PathBuf,
    pub manifest_path: PathBuf,
    pub bootstrap_install: bool,
    pub transfer_bytes_total: u64,
    pub verified_files_total: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct UpdateProgress {
    pub phase: String,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub files_done: u64,
    pub files_total: u64,
}

impl UpdateProgress {
    pub fn completed(phase: &str, bytes_total: u64, files_total: u64) -> Self {
        Self {
            phase: phase.to_string(),
            bytes_done: bytes_total,
            bytes_total,
            files_done: files_total,
            files_total,
        }
    }
}

pub fn channel() -> String {
    std::env::current_exe()
        .ok()
        .map(|path| channel_from_executable_path(&path))
        .unwrap_or("stable")
        .to_string()
}

fn channel_from_name(name: &str) -> &'static str {
    if name.to_ascii_lowercase().trim_end().ends_with(" test") {
        "test"
    } else {
        "stable"
    }
}

fn channel_from_executable_path(path: &Path) -> &'static str {
    for ancestor in path.ancestors() {
        if ancestor
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("app"))
        {
            let app_name = ancestor
                .file_stem()
                .map(|value| value.to_string_lossy())
                .unwrap_or_default();
            return channel_from_name(&app_name);
        }
    }
    let executable_name = path
        .file_stem()
        .map(|value| value.to_string_lossy())
        .unwrap_or_default();
    channel_from_name(&executable_name)
}

pub fn platform() -> &'static str {
    if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        "windows-x64"
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "macos-arm64"
    } else {
        "unsupported"
    }
}

pub fn installed_version(root: &Path) -> String {
    fs::read_to_string(root.join(VERSION_FILE))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            option_env!("BMSIR_ARENA_CLIENT_VERSION")
                .unwrap_or("0.4.14")
                .to_string()
        })
}

fn update_base_url() -> Result<&'static str, UpdateError> {
    option_env!("BMSIR_ARENA_UPDATE_BASE_URL")
        .filter(|value| !value.trim().is_empty())
        .ok_or(UpdateError::Endpoint)
}

fn release_public_key() -> Result<&'static str, UpdateError> {
    option_env!("BMSIR_ARENA_RELEASE_PUBLIC_KEY")
        .filter(|value| !value.trim().is_empty())
        .ok_or(UpdateError::PublicKey)
}

fn append_url(base: &str, segments: &[&str]) -> Result<Url, UpdateError> {
    let mut url = Url::parse(base).map_err(|_| UpdateError::Url)?;
    {
        let mut path = url.path_segments_mut().map_err(|_| UpdateError::Url)?;
        path.pop_if_empty();
        for segment in segments {
            path.push(segment);
        }
    }
    Ok(url)
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(5))
        .timeout_read(Duration::from_secs(20))
        .timeout_write(Duration::from_secs(20))
        .redirects(3)
        .build()
}

fn fetch_response(url: &Url) -> Result<ureq::Response, UpdateError> {
    let agent = agent();
    for attempt in 0..DOWNLOAD_ATTEMPTS {
        match agent.get(url.as_str()).call() {
            Ok(response) => return Ok(response),
            Err(error) => {
                let retryable = matches!(
                    error,
                    ureq::Error::Transport(_)
                        | ureq::Error::Status(429, _)
                        | ureq::Error::Status(500..=599, _)
                );
                if !retryable || attempt + 1 == DOWNLOAD_ATTEMPTS {
                    return Err(UpdateError::Request(error.to_string()));
                }
                std::thread::sleep(Duration::from_millis(250 * (1 << attempt)));
            }
        }
    }
    unreachable!("download retry loop always returns")
}

fn fetch_bytes(url: &Url, maximum: u64) -> Result<Vec<u8>, UpdateError> {
    let response = fetch_response(url)?;
    let mut reader = response.into_reader().take(maximum + 1);
    let mut output = Vec::new();
    reader.read_to_end(&mut output)?;
    if output.len() as u64 > maximum {
        return Err(UpdateError::TooLarge);
    }
    Ok(output)
}

fn fetch_release() -> Result<(String, ReleaseManifest), UpdateError> {
    let selected_channel = channel();
    let selected_platform = platform();
    fetch_release_from(
        update_base_url()?,
        release_public_key()?,
        &selected_channel,
        selected_platform,
    )
}

fn fetch_release_from(
    base_url: &str,
    public_key: &str,
    selected_channel: &str,
    selected_platform: &str,
) -> Result<(String, ReleaseManifest), UpdateError> {
    if selected_platform == "unsupported" {
        return Err(UpdateError::WrongTarget);
    }
    let url = append_url(
        base_url,
        &[
            "channels",
            &selected_channel,
            selected_platform,
            "manifest.json",
        ],
    )?;
    let bytes = fetch_bytes(&url, MAX_MANIFEST_BYTES)?;
    let input = String::from_utf8(bytes).map_err(|_| UpdateError::Incomplete)?;
    let release = verify_manifest(&input, public_key)?;
    if release.channel != selected_channel || release.platform != selected_platform {
        return Err(UpdateError::WrongTarget);
    }
    Ok((input, release))
}

fn bootstrap_artifacts_present(release: &ReleaseManifest) -> bool {
    let expected_java = match release.platform.as_str() {
        "windows-x64" => "runtime/bin/java.exe",
        "macos-arm64" => "runtime/bin/java",
        _ => return false,
    };
    let mut body = false;
    let mut java = false;
    let mut plugin = false;
    let artifacts = release
        .bootstrap
        .as_ref()
        .map(|bootstrap| bootstrap.artifacts.as_slice())
        .unwrap_or(release.artifacts.as_slice());
    for artifact in artifacts {
        let normalized = artifact.path.replace('\\', "/").to_ascii_lowercase();
        let path = Path::new(&normalized);
        if path
            .parent()
            .is_some_and(|parent| parent.as_os_str().is_empty())
            && path.extension().and_then(|value| value.to_str()) == Some("jar")
            && path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|name| {
                    name == CANONICAL_GAME_JAR.to_ascii_lowercase()
                        || name.contains("beatoraja")
                        || name.contains("lr2oraja")
                        || name.contains("bms-ir-arena")
                })
        {
            body = true;
        }
        java |= normalized == expected_java;
        plugin |= path
            .parent()
            .and_then(Path::to_str)
            .is_some_and(|parent| parent == "ir")
            && path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|name| name.starts_with("bms_ir") && name.ends_with(".jar"));
    }
    body && java && plugin
}

fn status_for_versions(
    installed: &str,
    available: &str,
    installation_ready: bool,
    revoked: bool,
    launcher_old: bool,
) -> Result<&'static str, UpdateError> {
    let version_order = compare_versions(installed, available);
    if revoked {
        return Ok("revoked");
    }
    if launcher_old {
        return Ok("launcher_too_old");
    }
    if !installation_ready {
        if version_order == Ordering::Greater {
            return Err(UpdateError::Request(
                "the installed version is newer than the available bootstrap release".to_string(),
            ));
        }
        return Ok("install_required");
    }
    Ok(if version_order == Ordering::Less {
        "available"
    } else {
        "current"
    })
}

fn bootstrap_allowed_for_versions(
    installed: &str,
    available: &str,
    installation_ready: bool,
    revoked: bool,
) -> bool {
    revoked || (!installation_ready && compare_versions(installed, available) != Ordering::Greater)
}

fn update_info_from_release(
    installed: String,
    installation_ready: bool,
    launcher_version: &str,
    release: &ReleaseManifest,
) -> Result<UpdateInfo, UpdateError> {
    if !installation_ready && !bootstrap_artifacts_present(release) {
        return Err(UpdateError::IncompleteBootstrap);
    }
    let revoked = release
        .revoked_versions
        .iter()
        .any(|value| value == &installed);
    let launcher_old =
        compare_versions(launcher_version, &release.minimum_launcher_version) == Ordering::Less;
    let status = status_for_versions(
        &installed,
        &release.version,
        installation_ready,
        revoked,
        launcher_old,
    )?;
    let release_notes_markdown_ja = if release.release_notes_markdown_ja.is_empty() {
        release.release_notes_markdown.clone()
    } else {
        release.release_notes_markdown_ja.clone()
    };
    let release_notes_markdown_en = if release.release_notes_markdown_en.is_empty() {
        release.release_notes_markdown.clone()
    } else {
        release.release_notes_markdown_en.clone()
    };
    Ok(UpdateInfo {
        channel: release.channel.clone(),
        platform: release.platform.clone(),
        installed_version: installed,
        available_version: release.version.clone(),
        status: status.to_string(),
        mandatory: release.mandatory || revoked || launcher_old,
        release_notes_markdown: release.release_notes_markdown.clone(),
        release_notes_markdown_ja,
        release_notes_markdown_en,
        announcements: release.announcements.clone(),
    })
}

fn update_blocks_launch(update: &UpdateInfo) -> bool {
    matches!(
        update.status.as_str(),
        "install_required" | "revoked" | "launcher_too_old"
    ) || (update.status == "available" && update.mandatory)
}

fn cache_verified_release(root: &Path, manifest_json: &str) -> Result<(), UpdateError> {
    let target = root.join(CACHED_POLICY);
    let temporary = root.join(CACHED_POLICY_TEMPORARY);
    for path in [&target, &temporary] {
        if path.exists() && path.symlink_metadata()?.file_type().is_symlink() {
            return Err(UpdateError::UnsafeStaging);
        }
    }
    fs::write(&temporary, manifest_json)?;
    if target.exists() {
        fs::remove_file(&target)?;
    }
    fs::rename(temporary, target)?;
    Ok(())
}

fn cached_update_from(
    root: &Path,
    installation_ready: bool,
    public_key: &str,
    selected_channel: &str,
    selected_platform: &str,
    launcher_version: &str,
) -> Result<Option<UpdateInfo>, UpdateError> {
    let path = root.join(CACHED_POLICY);
    if !path.exists() {
        return Ok(None);
    }
    if path.symlink_metadata()?.file_type().is_symlink() {
        return Err(UpdateError::UnsafeStaging);
    }
    if path.metadata()?.len() > MAX_MANIFEST_BYTES {
        return Err(UpdateError::TooLarge);
    }
    let input = fs::read_to_string(path)?;
    let release = verify_manifest(&input, public_key)?;
    if release.channel != selected_channel || release.platform != selected_platform {
        return Ok(None);
    }
    update_info_from_release(
        installed_version(root),
        installation_ready,
        launcher_version,
        &release,
    )
    .map(Some)
}

pub fn cached_update(
    root: &Path,
    installation_ready: bool,
) -> Result<Option<UpdateInfo>, UpdateError> {
    let Ok(public_key) = release_public_key() else {
        return Ok(None);
    };
    cached_update_from(
        root,
        installation_ready,
        public_key,
        &channel(),
        platform(),
        env!("CARGO_PKG_VERSION"),
    )
}

pub fn enforce_cached_launch_policy(
    root: &Path,
    installation_ready: bool,
) -> Result<(), UpdateError> {
    if let Some(update) = cached_update(root, installation_ready)? {
        if update_blocks_launch(&update) {
            return Err(UpdateError::RequiredUpdate(update.available_version));
        }
    }
    Ok(())
}

pub fn check_installation(
    root: &Path,
    installation_ready: bool,
) -> Result<UpdateInfo, UpdateError> {
    let installed = installed_version(root);
    let (input, release) = fetch_release()?;
    let update = update_info_from_release(
        installed,
        installation_ready,
        env!("CARGO_PKG_VERSION"),
        &release,
    )?;
    cache_verified_release(root, &input)?;
    Ok(update)
}

fn artifact_matches(root: &Path, artifact: &ReleaseArtifact) -> bool {
    let path = root.join(&artifact.path);
    if !path.is_file()
        || path
            .symlink_metadata()
            .is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        return false;
    }
    if cfg!(unix) && artifact.executable {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if path
                .metadata()
                .is_ok_and(|metadata| metadata.permissions().mode() & 0o111 == 0)
            {
                return false;
            }
        }
    }
    verify_file(&path, artifact).is_ok()
}

fn artifacts_match(left: &ReleaseArtifact, right: &ReleaseArtifact) -> bool {
    left.path.eq_ignore_ascii_case(&right.path)
        && left.sha256.eq_ignore_ascii_case(&right.sha256)
        && left.size == right.size
        && left.executable == right.executable
}

fn bootstrap_delta_artifacts(release: &ReleaseManifest) -> Vec<ReleaseArtifact> {
    let Some(bootstrap) = &release.bootstrap else {
        return release.artifacts.clone();
    };
    release
        .artifacts
        .iter()
        .filter(|artifact| {
            !bootstrap
                .artifacts
                .iter()
                .any(|base| artifacts_match(base, artifact))
        })
        .cloned()
        .collect()
}

fn effective_file_count(release: &ReleaseManifest, bootstrap_install: bool) -> u64 {
    if !bootstrap_install {
        return release.artifacts.len() as u64;
    }
    let mut paths = release
        .bootstrap
        .as_ref()
        .map(|bootstrap| {
            bootstrap
                .artifacts
                .iter()
                .map(|artifact| artifact.path.to_ascii_lowercase())
                .collect::<std::collections::BTreeSet<_>>()
        })
        .unwrap_or_default();
    paths.extend(
        release
            .artifacts
            .iter()
            .map(|artifact| artifact.path.to_ascii_lowercase()),
    );
    paths.len() as u64
}

fn download_to_path<F>(
    url: &Url,
    destination: &Path,
    expected_size: u64,
    bytes_done: &mut u64,
    bytes_total: u64,
    files_done: u64,
    files_total: u64,
    progress: &mut F,
) -> Result<(), UpdateError>
where
    F: FnMut(UpdateProgress),
{
    let response = fetch_response(url)?;
    let mut reader = response.into_reader().take(expected_size + 1);
    let mut target = fs::File::create(destination)?;
    let mut copied = 0_u64;
    let mut last_reported_bytes = *bytes_done;
    let mut buffer = [0_u8; 256 * 1024];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        target.write_all(&buffer[..count])?;
        copied = copied.saturating_add(count as u64);
        *bytes_done = (*bytes_done).saturating_add(count as u64);
        if (*bytes_done).saturating_sub(last_reported_bytes) >= PROGRESS_BYTES_STEP {
            progress(UpdateProgress {
                phase: "downloading".to_string(),
                bytes_done: (*bytes_done).min(bytes_total),
                bytes_total,
                files_done,
                files_total,
            });
            last_reported_bytes = *bytes_done;
        }
    }
    target.flush()?;
    if copied != expected_size {
        return Err(if copied > expected_size {
            UpdateError::TooLarge
        } else {
            UpdateError::Incomplete
        });
    }
    Ok(())
}

fn bootstrap_archive_artifact(bootstrap: &ReleaseBootstrap) -> ReleaseArtifact {
    ReleaseArtifact {
        path: BOOTSTRAP_ARCHIVE.to_string(),
        sha256: bootstrap.sha256.clone(),
        size: bootstrap.size,
        executable: false,
    }
}

fn extract_bootstrap_archive<F>(
    archive_path: &Path,
    staging: &Path,
    bootstrap: &ReleaseBootstrap,
    bytes_done: u64,
    bytes_total: u64,
    progress: &mut F,
) -> Result<(), UpdateError>
where
    F: FnMut(UpdateProgress),
{
    let mut expected = bootstrap
        .artifacts
        .iter()
        .map(|artifact| (artifact.path.to_ascii_lowercase(), artifact))
        .collect::<HashMap<_, _>>();
    let mut archive = zip::ZipArchive::new(fs::File::open(archive_path)?)?;
    let files_total = expected.len() as u64;
    let mut files_done = 0_u64;
    progress(UpdateProgress {
        phase: "extracting".to_string(),
        bytes_done,
        bytes_total,
        files_done,
        files_total,
    });
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let enclosed = entry
            .enclosed_name()
            .ok_or(UpdateError::IncompleteBootstrap)?;
        if entry.name().contains('\\')
            || entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err(UpdateError::IncompleteBootstrap);
        }
        if entry.is_dir() {
            continue;
        }
        let relative = enclosed.to_string_lossy().replace('\\', "/");
        if relative.eq_ignore_ascii_case(VERSION_FILE) {
            continue;
        }
        let artifact = expected
            .remove(&relative.to_ascii_lowercase())
            .ok_or(UpdateError::IncompleteBootstrap)?;
        if entry.size() != artifact.size {
            return Err(UpdateError::IncompleteBootstrap);
        }
        let destination = staging.join(&artifact.path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut target = fs::File::create(&destination)?;
        let copied = io::copy(&mut entry, &mut target)?;
        target.flush()?;
        if copied != artifact.size {
            return Err(UpdateError::IncompleteBootstrap);
        }
        verify_file(&destination, artifact)?;
        set_executable_if_requested(&destination, artifact.executable)?;
        files_done += 1;
        progress(UpdateProgress {
            phase: "extracting".to_string(),
            bytes_done,
            bytes_total,
            files_done,
            files_total,
        });
    }
    if !expected.is_empty() {
        return Err(UpdateError::IncompleteBootstrap);
    }
    Ok(())
}

pub fn prepare_with_progress<F>(
    root: &Path,
    installation_ready: bool,
    progress: F,
) -> Result<PreparedUpdate, UpdateError>
where
    F: FnMut(UpdateProgress),
{
    let selected_channel = channel();
    prepare_from_with_progress(
        root,
        installation_ready,
        update_base_url()?,
        release_public_key()?,
        &selected_channel,
        platform(),
        progress,
    )
}

fn prepare_from_with_progress<F>(
    root: &Path,
    installation_ready: bool,
    base_url: &str,
    public_key: &str,
    selected_channel: &str,
    selected_platform: &str,
    mut progress: F,
) -> Result<PreparedUpdate, UpdateError>
where
    F: FnMut(UpdateProgress),
{
    let root = root.canonicalize()?;
    let (manifest_json, release) =
        fetch_release_from(base_url, public_key, selected_channel, selected_platform)?;
    if !installation_ready && !bootstrap_artifacts_present(&release) {
        return Err(UpdateError::IncompleteBootstrap);
    }
    let installed = installed_version(&root);
    let revoked = release
        .revoked_versions
        .iter()
        .any(|value| value == &installed);
    let version_order = compare_versions(&installed, &release.version);
    let bootstrap_allowed =
        bootstrap_allowed_for_versions(&installed, &release.version, installation_ready, revoked);
    if version_order != Ordering::Less && !bootstrap_allowed {
        return Err(UpdateError::Request(
            "no newer release is available".to_string(),
        ));
    }
    let staging_parent = root.join(STAGING_DIRECTORY);
    if staging_parent.exists() && staging_parent.symlink_metadata()?.file_type().is_symlink() {
        return Err(UpdateError::UnsafeStaging);
    }
    fs::create_dir_all(&staging_parent)?;
    let staging = staging_parent.join(&release.version);
    if staging.exists() {
        if staging.symlink_metadata()?.file_type().is_symlink() {
            return Err(UpdateError::UnsafeStaging);
        }
        fs::remove_dir_all(&staging)?;
    }
    fs::create_dir_all(&staging)?;

    let bootstrap_install = !installation_ready;
    let bootstrap = if bootstrap_install {
        release.bootstrap.as_ref()
    } else {
        None
    };
    let download_artifacts = if bootstrap.is_some() {
        bootstrap_delta_artifacts(&release)
            .into_iter()
            .collect::<Vec<_>>()
    } else if bootstrap_install {
        release.artifacts.clone()
    } else {
        release
            .artifacts
            .iter()
            .filter(|artifact| !artifact_matches(&root, artifact))
            .cloned()
            .collect::<Vec<_>>()
    };
    let bytes_total = bootstrap.map_or(0, |value| value.size).saturating_add(
        download_artifacts
            .iter()
            .fold(0_u64, |total, artifact| total.saturating_add(artifact.size)),
    );
    let files_total = download_artifacts.len() as u64 + u64::from(bootstrap.is_some());
    let mut bytes_done = 0_u64;
    let mut files_done = 0_u64;
    progress(UpdateProgress {
        phase: "downloading".to_string(),
        bytes_done,
        bytes_total,
        files_done,
        files_total,
    });

    if let Some(bootstrap) = bootstrap {
        let archive_path = staging.join(BOOTSTRAP_ARCHIVE);
        let url = Url::parse(&bootstrap.url).map_err(|_| UpdateError::Url)?;
        download_to_path(
            &url,
            &archive_path,
            bootstrap.size,
            &mut bytes_done,
            bytes_total,
            files_done,
            files_total,
            &mut progress,
        )?;
        verify_file(&archive_path, &bootstrap_archive_artifact(bootstrap))?;
        files_done += 1;
        progress(UpdateProgress {
            phase: "downloading".to_string(),
            bytes_done: bytes_done.min(bytes_total),
            bytes_total,
            files_done,
            files_total,
        });
        extract_bootstrap_archive(
            &archive_path,
            &staging,
            bootstrap,
            bytes_done,
            bytes_total,
            &mut progress,
        )?;
        fs::remove_file(archive_path)?;
    }

    for artifact in &download_artifacts {
        let destination = staging.join(&artifact.path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let segments: Vec<&str> = artifact.path.split('/').collect();
        let mut url_segments = vec![
            "channels",
            release.channel.as_str(),
            release.platform.as_str(),
            "releases",
            release.version.as_str(),
        ];
        url_segments.extend(segments);
        let url = append_url(base_url, &url_segments)?;
        download_to_path(
            &url,
            &destination,
            artifact.size,
            &mut bytes_done,
            bytes_total,
            files_done,
            files_total,
            &mut progress,
        )?;
        verify_file(&destination, artifact)?;
        set_executable_if_requested(&destination, artifact.executable)?;
        files_done += 1;
        progress(UpdateProgress {
            phase: "downloading".to_string(),
            bytes_done: bytes_done.min(bytes_total),
            bytes_total,
            files_done,
            files_total,
        });
    }
    let manifest_path = staging.join(STAGED_MANIFEST);
    fs::write(&manifest_path, manifest_json)?;
    let verified_files_total = effective_file_count(&release, bootstrap_install);
    Ok(PreparedUpdate {
        manifest: release,
        staging,
        manifest_path,
        bootstrap_install,
        transfer_bytes_total: bytes_total,
        verified_files_total,
    })
}

fn compare_versions(left: &str, right: &str) -> Ordering {
    let parse = |value: &str| {
        let (main, suffix) = value.split_once('-').unwrap_or((value, ""));
        let numbers = main
            .split('.')
            .map(|part| part.parse::<u64>().unwrap_or(0))
            .collect::<Vec<_>>();
        (numbers, suffix.to_string())
    };
    let (left_numbers, left_suffix) = parse(left.trim());
    let (right_numbers, right_suffix) = parse(right.trim());
    let length = left_numbers.len().max(right_numbers.len());
    for index in 0..length {
        match left_numbers
            .get(index)
            .copied()
            .unwrap_or(0)
            .cmp(&right_numbers.get(index).copied().unwrap_or(0))
        {
            Ordering::Equal => {}
            ordering => return ordering,
        }
    }
    match (left_suffix.is_empty(), right_suffix.is_empty()) {
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        _ => left_suffix.cmp(&right_suffix),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use ed25519_dalek::{Signer, SigningKey};
    use serde_json::{json, Value};
    use sha2::{Digest, Sha256};
    use std::collections::HashMap;
    use std::net::TcpListener;
    use std::thread;
    use zip::write::SimpleFileOptions;

    #[test]
    fn executable_name_selects_test_channel_only_when_explicit() {
        assert_eq!(channel_from_name("BMS-IR Arena"), "stable");
        assert_eq!(channel_from_name("BMS-IR Arena Test"), "test");
        assert_eq!(channel_from_name("contest"), "stable");
    }

    #[test]
    fn macos_app_name_selects_channel_instead_of_inner_binary_name() {
        assert_eq!(
            channel_from_executable_path(Path::new(
                "/Applications/BMS-IR Arena Test.app/Contents/MacOS/bmsir-arena-launcher"
            )),
            "test"
        );
        assert_eq!(
            channel_from_executable_path(Path::new(
                "/Applications/BMS-IR Arena.app/Contents/MacOS/bmsir-arena-launcher"
            )),
            "stable"
        );
        assert_eq!(
            channel_from_executable_path(Path::new("C:/Arena/BMS-IR Arena Test.exe")),
            "test"
        );
    }

    #[test]
    fn versions_order_release_after_prerelease() {
        assert_eq!(compare_versions("0.4.14", "0.4.13"), Ordering::Greater);
        assert_eq!(compare_versions("0.4.14-test", "0.4.14"), Ordering::Less);
        assert_eq!(compare_versions("0.4.14", "0.4.14.0"), Ordering::Equal);
    }

    #[test]
    fn missing_installation_can_bootstrap_the_same_version() {
        assert_eq!(
            status_for_versions("0.4.14", "0.4.14", false, false, false).unwrap(),
            "install_required"
        );
        assert!(bootstrap_allowed_for_versions(
            "0.4.14", "0.4.14", false, false
        ));
    }

    #[test]
    fn initial_install_requires_body_java_and_plugin_artifacts() {
        let artifact = |path: &str| crate::manifest::ReleaseArtifact {
            path: path.into(),
            sha256: "00".repeat(32),
            size: 1,
            executable: false,
        };
        let mut release = ReleaseManifest {
            schema_version: 1,
            channel: "test".into(),
            platform: "windows-x64".into(),
            version: "0.4.14".into(),
            published_at: "now".into(),
            release_notes_markdown: String::new(),
            release_notes_markdown_ja: String::new(),
            release_notes_markdown_en: String::new(),
            announcements: vec![],
            mandatory: false,
            minimum_launcher_version: "0.2.1".into(),
            revoked_versions: vec![],
            bootstrap: None,
            artifacts: vec![
                artifact("Arena-oraja.jar"),
                artifact("runtime/bin/java.exe"),
            ],
            signature: String::new(),
        };
        assert!(!bootstrap_artifacts_present(&release));
        release
            .artifacts
            .push(artifact("ir/bms_ir_arena_oraja_0.0.69.jar"));
        assert!(bootstrap_artifacts_present(&release));
    }

    #[test]
    fn complete_installation_keeps_normal_version_rules() {
        assert_eq!(
            status_for_versions("0.4.14", "0.4.14", true, false, false).unwrap(),
            "current"
        );
        assert!(!bootstrap_allowed_for_versions(
            "0.4.14", "0.4.14", true, false
        ));
    }

    #[test]
    fn incomplete_newer_installation_does_not_silently_downgrade() {
        assert!(status_for_versions("0.4.15", "0.4.14", false, false, false).is_err());
        assert!(!bootstrap_allowed_for_versions(
            "0.4.15", "0.4.14", false, false
        ));
    }

    #[test]
    fn cached_signed_mandatory_update_blocks_offline_launch() {
        let signing = SigningKey::from_bytes(&[23_u8; 32]);
        let mut manifest = json!({
            "schema_version": 1,
            "channel": "test",
            "platform": "windows-x64",
            "version": "0.4.14",
            "published_at": "2026-08-03T00:00:00Z",
            "release_notes_markdown": "",
            "release_notes_markdown_ja": "## 必須更新",
            "release_notes_markdown_en": "## Required update",
            "announcements": [{
                "date": "2026-08-03",
                "title_ja": "更新のお知らせ",
                "title_en": "Update notice"
            }],
            "mandatory": true,
            "minimum_launcher_version": "0.2.0",
            "revoked_versions": [],
            "artifacts": []
        });
        let signature = signing.sign(&serde_jcs::to_vec(&manifest).unwrap());
        manifest["signature"] = Value::String(STANDARD.encode(signature.to_bytes()));

        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join(VERSION_FILE), "0.4.13\n").unwrap();
        cache_verified_release(root.path(), &serde_json::to_string(&manifest).unwrap()).unwrap();
        let update = cached_update_from(
            root.path(),
            true,
            &STANDARD.encode(signing.verifying_key().to_bytes()),
            "test",
            "windows-x64",
            "0.2.3",
        )
        .unwrap()
        .unwrap();
        assert_eq!(update.release_notes_markdown_ja, "## 必須更新");
        assert_eq!(update.announcements[0].title_en, "Update notice");
        assert!(update_blocks_launch(&update));

        fs::write(root.path().join(VERSION_FILE), "0.4.14\n").unwrap();
        let current = cached_update_from(
            root.path(),
            true,
            &STANDARD.encode(signing.verifying_key().to_bytes()),
            "test",
            "windows-x64",
            "0.2.3",
        )
        .unwrap()
        .unwrap();
        assert_eq!(current.status, "current");
        assert!(!update_blocks_launch(&current));

        fs::write(root.path().join(CACHED_POLICY), "{}\n").unwrap();
        assert!(cached_update_from(
            root.path(),
            true,
            &STANDARD.encode(signing.verifying_key().to_bytes()),
            "test",
            "windows-x64",
            "0.2.3",
        )
        .is_err());
    }

    #[test]
    fn legacy_release_notes_fill_both_languages() {
        let release = ReleaseManifest {
            schema_version: 1,
            channel: "test".into(),
            platform: "windows-x64".into(),
            version: "0.4.14".into(),
            published_at: "now".into(),
            release_notes_markdown: "Legacy notes".into(),
            release_notes_markdown_ja: String::new(),
            release_notes_markdown_en: String::new(),
            announcements: vec![],
            mandatory: false,
            minimum_launcher_version: "0.2.0".into(),
            revoked_versions: vec![],
            bootstrap: None,
            artifacts: vec![],
            signature: String::new(),
        };
        let update = update_info_from_release("0.4.13".into(), true, "0.2.3", &release).unwrap();
        assert_eq!(update.release_notes_markdown_ja, "Legacy notes");
        assert_eq!(update.release_notes_markdown_en, "Legacy notes");
    }

    #[test]
    fn installed_update_downloads_only_changed_artifacts() {
        let signing = SigningKey::from_bytes(&[29_u8; 32]);
        let artifact = |path: &str, bytes: &[u8]| {
            json!({
                "path": path,
                "sha256": format!("{:x}", Sha256::digest(bytes)),
                "size": bytes.len(),
                "executable": false
            })
        };
        let mut manifest = json!({
            "schema_version": 1,
            "channel": "test",
            "platform": "windows-x64",
            "version": "0.4.14.9",
            "published_at": "2026-08-04T00:00:00Z",
            "release_notes_markdown": "delta",
            "mandatory": false,
            "minimum_launcher_version": "0.2.7",
            "revoked_versions": [],
            "artifacts": [artifact("same.dat", b"same"), artifact("changed.dat", b"new")]
        });
        let signature = signing.sign(&serde_jcs::to_vec(&manifest).unwrap());
        manifest["signature"] = Value::String(STANDARD.encode(signature.to_bytes()));
        let manifest_bytes = serde_json::to_vec(&manifest).unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let responses = [
                (
                    "/channels/test/windows-x64/manifest.json",
                    manifest_bytes.as_slice(),
                ),
                (
                    "/channels/test/windows-x64/releases/0.4.14.9/changed.dat",
                    b"new".as_slice(),
                ),
            ];
            for (expected_path, body) in responses {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0_u8; 4096];
                let length = stream.read(&mut request).unwrap();
                let line = String::from_utf8_lossy(&request[..length]);
                let path = line
                    .lines()
                    .next()
                    .and_then(|value| value.split_whitespace().nth(1))
                    .unwrap();
                assert_eq!(path, expected_path);
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .unwrap();
                stream.write_all(body).unwrap();
            }
        });

        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join(VERSION_FILE), "0.4.14.8\n").unwrap();
        fs::write(root.path().join("same.dat"), b"same").unwrap();
        fs::write(root.path().join("changed.dat"), b"old").unwrap();
        let prepared = prepare_from_with_progress(
            root.path(),
            true,
            &format!("http://{address}"),
            &STANDARD.encode(signing.verifying_key().to_bytes()),
            "test",
            "windows-x64",
            |_| {},
        )
        .unwrap();
        server.join().unwrap();

        assert_eq!(prepared.transfer_bytes_total, 3);
        assert!(!prepared.staging.join("same.dat").exists());
        assert_eq!(
            fs::read(prepared.staging.join("changed.dat")).unwrap(),
            b"new"
        );
    }

    #[test]
    fn bootstrap_archive_extracts_only_the_signed_inventory() {
        let root = tempfile::tempdir().unwrap();
        let archive_path = root.path().join("bootstrap.zip");
        let file = fs::File::create(&archive_path).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        for (path, bytes, executable) in [
            ("Arena.jar", b"body".as_slice(), false),
            ("runtime/bin/java.exe", b"java".as_slice(), true),
            ("ir/bms_ir_arena.jar", b"plugin".as_slice(), false),
        ] {
            let mode = if executable { 0o755 } else { 0o644 };
            archive
                .start_file(path, SimpleFileOptions::default().unix_permissions(mode))
                .unwrap();
            archive.write_all(bytes).unwrap();
        }
        archive.finish().unwrap();
        let artifacts = [
            ("Arena.jar", b"body".as_slice(), false),
            ("runtime/bin/java.exe", b"java".as_slice(), true),
            ("ir/bms_ir_arena.jar", b"plugin".as_slice(), false),
        ]
        .into_iter()
        .map(|(path, bytes, executable)| ReleaseArtifact {
            path: path.into(),
            sha256: format!("{:x}", Sha256::digest(bytes)),
            size: bytes.len() as u64,
            executable,
        })
        .collect::<Vec<_>>();
        let bootstrap = ReleaseBootstrap {
            url: "https://example.test/bootstrap.zip".into(),
            sha256: format!("{:x}", Sha256::digest(fs::read(&archive_path).unwrap())),
            size: fs::metadata(&archive_path).unwrap().len(),
            artifacts,
        };
        let staging = tempfile::tempdir().unwrap();
        let mut progress = Vec::new();
        extract_bootstrap_archive(
            &archive_path,
            staging.path(),
            &bootstrap,
            bootstrap.size,
            bootstrap.size,
            &mut |event| progress.push(event),
        )
        .unwrap();

        assert_eq!(fs::read(staging.path().join("Arena.jar")).unwrap(), b"body");
        assert_eq!(progress.last().unwrap().files_done, 3);
        assert!(progress.iter().all(|event| event.phase == "extracting"));
    }

    #[test]
    fn empty_root_downloads_a_complete_same_version_release() {
        let files = [
            ("Arena-oraja.jar", b"body".as_slice()),
            ("runtime/bin/java.exe", b"java".as_slice()),
            ("ir/bms_ir_arena_oraja_0.0.69.jar", b"plugin".as_slice()),
        ];
        let artifacts = files
            .iter()
            .map(|(path, bytes)| {
                json!({
                    "path": path,
                    "sha256": format!("{:x}", Sha256::digest(bytes)),
                    "size": bytes.len(),
                    "executable": path.ends_with(".exe")
                })
            })
            .collect::<Vec<_>>();
        let signing = SigningKey::from_bytes(&[19_u8; 32]);
        let mut manifest = json!({
            "schema_version": 1,
            "channel": "test",
            "platform": "windows-x64",
            "version": "0.4.14",
            "published_at": "2026-08-03T00:00:00Z",
            "release_notes_markdown": "internal bootstrap",
            "mandatory": false,
            "minimum_launcher_version": "0.2.1",
            "revoked_versions": [],
            "artifacts": artifacts
        });
        let signature = signing.sign(&serde_jcs::to_vec(&manifest).unwrap());
        manifest["signature"] = Value::String(STANDARD.encode(signature.to_bytes()));
        let manifest_bytes = serde_json::to_vec(&manifest).unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let mut responses = HashMap::new();
        responses.insert(
            "/channels/test/windows-x64/manifest.json".to_string(),
            manifest_bytes,
        );
        for (path, bytes) in files {
            responses.insert(
                format!("/channels/test/windows-x64/releases/0.4.14/{path}"),
                bytes.to_vec(),
            );
        }
        let server = thread::spawn(move || {
            for _ in 0..responses.len() {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0_u8; 4096];
                let length = stream.read(&mut request).unwrap();
                let first_line = String::from_utf8_lossy(&request[..length]);
                let path = first_line
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap();
                let body = responses.get(path).unwrap();
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .unwrap();
                stream.write_all(body).unwrap();
            }
        });

        let root = tempfile::tempdir().unwrap();
        let mut progress = Vec::new();
        let prepared = prepare_from_with_progress(
            root.path(),
            false,
            &format!("http://{address}"),
            &STANDARD.encode(signing.verifying_key().to_bytes()),
            "test",
            "windows-x64",
            |event| progress.push(event),
        )
        .unwrap();
        server.join().unwrap();
        assert_eq!(prepared.manifest.version, "0.4.14");
        let expected_bytes = files
            .iter()
            .map(|(_, bytes)| bytes.len() as u64)
            .sum::<u64>();
        assert_eq!(
            progress.first(),
            Some(&UpdateProgress {
                phase: "downloading".into(),
                bytes_done: 0,
                bytes_total: expected_bytes,
                files_done: 0,
                files_total: files.len() as u64,
            })
        );
        assert_eq!(
            progress.last(),
            Some(&UpdateProgress {
                phase: "downloading".into(),
                bytes_done: expected_bytes,
                bytes_total: expected_bytes,
                files_done: files.len() as u64,
                files_total: files.len() as u64,
            })
        );
        assert!(progress.windows(2).all(|events| {
            events[0].bytes_done <= events[1].bytes_done
                && events[0].files_done <= events[1].files_done
        }));
        for (path, bytes) in files {
            assert_eq!(fs::read(prepared.staging.join(path)).unwrap(), bytes);
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(prepared.staging.join("runtime/bin/java.exe"))
                .unwrap()
                .permissions()
                .mode();
            assert_ne!(mode & 0o111, 0);
        }
    }

    #[test]
    fn transient_server_errors_are_retried() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            for attempt in 0..2 {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0_u8; 1024];
                stream.read(&mut request).unwrap();
                if attempt == 0 {
                    write!(
                        stream,
                        "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    )
                    .unwrap();
                } else {
                    write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK"
                    )
                    .unwrap();
                }
            }
        });

        let bytes = fetch_bytes(
            &Url::parse(&format!("http://{address}/artifact")).unwrap(),
            2,
        )
        .unwrap();
        server.join().unwrap();
        assert_eq!(bytes, b"OK");
    }

    #[test]
    fn artifact_urls_encode_spaces_without_losing_base_path() {
        let url = append_url(
            "https://example.test/arena-patches/",
            &["channels", "test", "windows-x64", "BMS-IR Arena.exe"],
        )
        .unwrap();
        assert_eq!(
            url.as_str(),
            "https://example.test/arena-patches/channels/test/windows-x64/BMS-IR%20Arena.exe"
        );
    }
}

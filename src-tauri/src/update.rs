use crate::install::{set_executable_if_requested, CANONICAL_GAME_JAR};
use crate::manifest::{
    verify_artifact_locations, verify_file, verify_history, verify_manifest, ArtifactLocations,
    HistoryEntry, ReleaseAnnouncement, ReleaseArtifact, ReleaseBootstrap, ReleaseHistory,
    ReleaseManifest,
};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;
use thiserror::Error;
use url::Url;

const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
const VERSION_FILE: &str = "bmsir-arena-version.txt";
const STAGING_DIRECTORY: &str = ".bmsir-update-staging";
const STAGED_MANIFEST: &str = ".bmsir-update-manifest.json";
const STAGED_LAUNCHER_MANIFEST: &str = ".bmsir-launcher-manifest.json";
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
        && configured_value(option_env!("BMSIR_ARENA_UPDATE_CHANNEL"))
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
    #[error("this launcher is too old to downgrade to that release")]
    DowngradeLauncherTooOld,
    #[error("the current installation must be complete before downgrading")]
    DowngradeInstallationNotReady,
    #[error("the selected release does not contain the game JAR")]
    DowngradeArtifactMissing,
    #[error("signed release history does not contain an Arena plugin")]
    PluginReleaseMissing,
    #[error("the selected component has no available update")]
    SelectedComponentCurrent,
    #[error(transparent)]
    Manifest(#[from] crate::manifest::ManifestError),
    #[error(transparent)]
    Install(#[from] crate::install::InstallError),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    pub channel: String,
    pub platform: String,
    pub installed_version: String,
    pub available_version: String,
    pub installed_launcher_version: String,
    pub available_launcher_version: String,
    pub body_update_available: bool,
    pub launcher_update_available: bool,
    pub available_published_at: String,
    pub status: String,
    pub mandatory: bool,
    pub release_notes_markdown: String,
    pub release_notes_markdown_ja: String,
    pub release_notes_markdown_en: String,
    pub announcements: Vec<ReleaseAnnouncement>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpdateTarget {
    All,
    Body,
    Launcher,
}

impl UpdateTarget {
    pub fn from_argument(value: &str) -> Option<Self> {
        match value {
            "all" => Some(Self::All),
            "body" => Some(Self::Body),
            "launcher" => Some(Self::Launcher),
            _ => None,
        }
    }

    pub fn includes_body(self) -> bool {
        matches!(self, Self::All | Self::Body)
    }

    pub fn as_argument(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Body => "body",
            Self::Launcher => "launcher",
        }
    }

    fn as_directory_name(self) -> &'static str {
        self.as_argument()
    }
}

#[derive(Debug, Clone)]
pub struct PreparedUpdate {
    pub manifest: ReleaseManifest,
    pub staging: PathBuf,
    pub manifest_path: PathBuf,
    pub launcher_manifest_path: Option<PathBuf>,
    pub bootstrap_install: bool,
    pub transfer_bytes_total: u64,
    pub verified_files_total: u64,
    pub target: UpdateTarget,
    pub writes_body_version: bool,
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
        .map(|path| {
            channel_from_configuration_or_path(option_env!("BMSIR_ARENA_UPDATE_CHANNEL"), &path)
        })
        .unwrap_or_else(|| {
            configured_channel(option_env!("BMSIR_ARENA_UPDATE_CHANNEL")).unwrap_or("stable")
        })
        .to_string()
}

fn configured_channel(value: Option<&str>) -> Option<&'static str> {
    match value {
        Some("stable") => Some("stable"),
        Some("test") => Some("test"),
        _ => None,
    }
}

fn channel_from_configuration_or_path(configured: Option<&str>, path: &Path) -> &'static str {
    configured_channel(configured).unwrap_or_else(|| channel_from_executable_path(path))
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
                .filter(|value| !value.trim().is_empty())
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

fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(5))
            .timeout_read(Duration::from_secs(20))
            .timeout_write(Duration::from_secs(20))
            .redirects(3)
            .build()
    })
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

fn fetch_history_index_from(
    base_url: &str,
    public_key: &str,
    selected_channel: &str,
    selected_platform: &str,
) -> Result<ReleaseHistory, UpdateError> {
    if selected_platform == "unsupported" {
        return Err(UpdateError::WrongTarget);
    }
    let url = append_url(
        base_url,
        &[
            "channels",
            selected_channel,
            selected_platform,
            "history.json",
        ],
    )?;
    let bytes = fetch_bytes(&url, MAX_MANIFEST_BYTES)?;
    let input = String::from_utf8(bytes).map_err(|_| UpdateError::Incomplete)?;
    verify_history(&input, public_key, selected_channel, selected_platform).map_err(Into::into)
}

fn fetch_history_from(
    base_url: &str,
    public_key: &str,
    selected_channel: &str,
    selected_platform: &str,
) -> Result<Vec<HistoryEntry>, UpdateError> {
    Ok(
        fetch_history_index_from(base_url, public_key, selected_channel, selected_platform)?
            .versions,
    )
}

fn fetch_artifact_locations_from(
    base_url: &str,
    public_key: &str,
    selected_channel: &str,
    selected_platform: &str,
    history: &ReleaseHistory,
) -> Result<Option<ArtifactLocations>, UpdateError> {
    let Some(reference) = &history.artifact_locations else {
        return Ok(None);
    };
    let url = append_url(
        base_url,
        &[
            "channels",
            selected_channel,
            selected_platform,
            &reference.path,
        ],
    )?;
    let bytes = fetch_bytes(&url, MAX_MANIFEST_BYTES)?;
    let input = String::from_utf8(bytes).map_err(|_| UpdateError::Incomplete)?;
    verify_artifact_locations(&input, public_key, selected_channel, selected_platform)
        .map(Some)
        .map_err(Into::into)
}

fn artifact_download_url(
    base_url: &str,
    selected_channel: &str,
    selected_platform: &str,
    version: &str,
    artifact: &ReleaseArtifact,
    locations: Option<&ArtifactLocations>,
) -> Result<Url, UpdateError> {
    if let Some(url) = locations
        .map(|index| index.url_for(version, artifact))
        .transpose()?
        .flatten()
    {
        return Ok(url);
    }
    let mut segments = vec![
        "channels",
        selected_channel,
        selected_platform,
        "releases",
        version,
    ];
    segments.extend(artifact.path.split('/'));
    append_url(base_url, &segments)
}

/// Lists every version in the signed history index other than
/// `current_version` and the channel's current published version, for
/// presentation as a selectable "deprecated" release the operator can
/// knowingly downgrade to. The published version is excluded in addition to
/// `current_version` because they can differ (a newer release is available
/// but not yet installed) — without also excluding it, the very release
/// being offered as "the update" would confusingly also appear as
/// "deprecated".
pub fn list_deprecated_versions(current_version: &str) -> Result<Vec<HistoryEntry>, UpdateError> {
    list_deprecated_versions_from(
        update_base_url()?,
        release_public_key()?,
        &channel(),
        platform(),
        current_version,
    )
}

fn list_deprecated_versions_from(
    base_url: &str,
    public_key: &str,
    selected_channel: &str,
    selected_platform: &str,
    current_version: &str,
) -> Result<Vec<HistoryEntry>, UpdateError> {
    let versions = fetch_history_from(base_url, public_key, selected_channel, selected_platform)?;
    let (_, published_release) =
        fetch_release_from(base_url, public_key, selected_channel, selected_platform)?;
    let candidates = versions
        .into_iter()
        .filter(|entry| {
            entry.version != current_version && entry.version != published_release.version
        })
        .collect::<Vec<_>>();
    let mut body_versions = Vec::with_capacity(candidates.len());
    for entry in candidates {
        let manifest = fetch_versioned_manifest_from(
            base_url,
            public_key,
            selected_channel,
            selected_platform,
            &entry.version,
        )?;
        if release_contains_game_jar(&manifest) {
            body_versions.push(entry);
        }
    }
    Ok(body_versions)
}

#[derive(Debug, Clone, Serialize)]
pub struct VersionNotes {
    pub release_notes_markdown: String,
    pub release_notes_markdown_ja: String,
    pub release_notes_markdown_en: String,
}

/// Fetches and verifies one historical release's notes on demand. Historical
/// manifests remain the signed source of truth; the WebView never fetches or
/// trusts release text directly.
pub fn fetch_version_notes(version: &str) -> Result<VersionNotes, UpdateError> {
    fetch_version_notes_from(
        update_base_url()?,
        release_public_key()?,
        &channel(),
        platform(),
        version,
    )
}

fn fetch_version_notes_from(
    base_url: &str,
    public_key: &str,
    selected_channel: &str,
    selected_platform: &str,
    version: &str,
) -> Result<VersionNotes, UpdateError> {
    let manifest = fetch_versioned_manifest_from(
        base_url,
        public_key,
        selected_channel,
        selected_platform,
        version,
    )?;
    Ok(VersionNotes {
        release_notes_markdown: manifest.release_notes_markdown,
        release_notes_markdown_ja: manifest.release_notes_markdown_ja,
        release_notes_markdown_en: manifest.release_notes_markdown_en,
    })
}

fn fetch_versioned_manifest_from(
    base_url: &str,
    public_key: &str,
    selected_channel: &str,
    selected_platform: &str,
    version: &str,
) -> Result<ReleaseManifest, UpdateError> {
    fetch_versioned_release_from(
        base_url,
        public_key,
        selected_channel,
        selected_platform,
        version,
    )
    .map(|(_, manifest)| manifest)
}

fn fetch_versioned_release_from(
    base_url: &str,
    public_key: &str,
    selected_channel: &str,
    selected_platform: &str,
    version: &str,
) -> Result<(String, ReleaseManifest), UpdateError> {
    let url = append_url(
        base_url,
        &[
            "channels",
            selected_channel,
            selected_platform,
            "manifests",
            &format!("{version}.json"),
        ],
    )?;
    let bytes = fetch_bytes(&url, MAX_MANIFEST_BYTES)?;
    let input = String::from_utf8(bytes).map_err(|_| UpdateError::Incomplete)?;
    let manifest = verify_manifest(&input, public_key)?;
    if manifest.channel != selected_channel
        || manifest.platform != selected_platform
        || manifest.version != version
    {
        return Err(UpdateError::WrongTarget);
    }
    Ok((input, manifest))
}

fn release_contains_game_jar(manifest: &ReleaseManifest) -> bool {
    manifest
        .artifacts
        .iter()
        .any(|artifact| artifact.path.eq_ignore_ascii_case(CANONICAL_GAME_JAR))
}

fn is_launcher_artifact(manifest: &ReleaseManifest, artifact: &ReleaseArtifact) -> bool {
    let path = artifact.path.to_ascii_lowercase();
    match (manifest.platform.as_str(), manifest.channel.as_str()) {
        ("windows-x64", "stable") => !path.contains('/') && path == "bms-ir arena.exe",
        ("windows-x64", "test") => !path.contains('/') && path == "bms-ir arena test.exe",
        ("macos-arm64", "stable") => path.starts_with("bms-ir arena.app/"),
        ("macos-arm64", "test") => path.starts_with("bms-ir arena test.app/"),
        _ => false,
    }
}

fn release_contains_launcher(manifest: &ReleaseManifest) -> bool {
    manifest.artifacts.iter().any(|artifact| {
        let path = artifact.path.to_ascii_lowercase();
        is_launcher_artifact(manifest, artifact)
            && if manifest.platform == "windows-x64" {
                path.ends_with(".exe")
            } else {
                path.ends_with("/contents/macos/bmsir-arena-launcher")
            }
    })
}

/// Produces a trusted component view only after the complete signed manifest
/// has been verified. The WebView chooses a component name, never file paths.
pub fn release_for_target(
    manifest: &ReleaseManifest,
    target: UpdateTarget,
) -> Result<ReleaseManifest, UpdateError> {
    if target == UpdateTarget::All {
        return Ok(manifest.clone());
    }
    let mut selected = manifest.clone();
    selected.bootstrap = None;
    selected.artifacts = manifest
        .artifacts
        .iter()
        .filter(|artifact| {
            let launcher = is_launcher_artifact(manifest, artifact);
            match target {
                UpdateTarget::All => true,
                UpdateTarget::Body => !launcher,
                UpdateTarget::Launcher => launcher,
            }
        })
        .cloned()
        .collect();
    if selected.artifacts.is_empty() {
        return Err(UpdateError::SelectedComponentCurrent);
    }
    Ok(selected)
}

/// Derives the component transaction from independently verified manifests.
/// Sparse current releases remain authoritative for body/plugin artifacts;
/// only launcher-owned paths may come from the selected history release.
pub(crate) fn release_for_target_with_launcher(
    current: &ReleaseManifest,
    launcher_release: Option<&ReleaseManifest>,
    target: UpdateTarget,
) -> Result<ReleaseManifest, UpdateError> {
    let Some(launcher_release) = launcher_release else {
        return release_for_target(current, target);
    };
    if launcher_release.channel != current.channel || launcher_release.platform != current.platform
    {
        return Err(UpdateError::WrongTarget);
    }
    if target == UpdateTarget::Body {
        return release_for_target(current, target);
    }
    let launcher = release_for_target(launcher_release, UpdateTarget::Launcher)?;
    if target == UpdateTarget::Launcher {
        return Ok(launcher);
    }

    let mut selected = current.clone();
    selected
        .artifacts
        .retain(|artifact| !is_launcher_artifact(current, artifact));
    selected.artifacts.extend(launcher.artifacts);
    selected.launcher_version = launcher_release.launcher_version.clone();
    if selected.artifacts.is_empty() {
        return Err(UpdateError::SelectedComponentCurrent);
    }
    Ok(selected)
}

fn release_plugin_artifact(manifest: &ReleaseManifest) -> Option<&ReleaseArtifact> {
    let mut plugins = manifest.artifacts.iter().filter(|artifact| {
        let path = Path::new(&artifact.path);
        path.parent()
            .and_then(Path::to_str)
            .is_some_and(|parent| parent.eq_ignore_ascii_case("ir"))
            && path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("jar"))
            && path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.to_ascii_lowercase().starts_with("bms_ir"))
    });
    let plugin = plugins.next()?;
    if plugins.next().is_some() {
        None
    } else {
        Some(plugin)
    }
}

/// A plugin release is identified by its signed channel release, rather than
/// by an untrusted filename.  The artifact's hash and exact path remain part
/// of the manifest signature.
#[derive(Debug, Clone, Serialize)]
pub struct PluginRelease {
    pub version: String,
    pub published_at: String,
    pub artifact_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PluginStatus {
    pub installed_artifact_path: Option<String>,
    pub available: PluginRelease,
    pub update_available: bool,
}

fn plugin_release(manifest: &ReleaseManifest) -> Option<(PluginRelease, ReleaseArtifact)> {
    let artifact = release_plugin_artifact(manifest)?.clone();
    Some((
        PluginRelease {
            version: manifest.version.clone(),
            published_at: manifest.published_at.clone(),
            artifact_path: artifact.path.clone(),
        },
        artifact,
    ))
}

fn latest_plugin_release_from(
    base_url: &str,
    public_key: &str,
    selected_channel: &str,
    selected_platform: &str,
) -> Result<(PluginRelease, ReleaseArtifact), UpdateError> {
    let (_, current) =
        fetch_release_from(base_url, public_key, selected_channel, selected_platform)?;
    if let Some(release) = plugin_release(&current) {
        return Ok(release);
    }
    for entry in fetch_history_from(base_url, public_key, selected_channel, selected_platform)? {
        if entry.version == current.version {
            continue;
        }
        let manifest = fetch_versioned_manifest_from(
            base_url,
            public_key,
            selected_channel,
            selected_platform,
            &entry.version,
        )?;
        if let Some(release) = plugin_release(&manifest) {
            return Ok(release);
        }
    }
    Err(UpdateError::PluginReleaseMissing)
}

/// Resolves launcher updates independently from the sparse current release.
/// History order and body-version spelling are not launcher-version order, so
/// every signed launcher-bearing candidate is compared by `launcher_version`.
#[cfg(test)]
fn latest_launcher_release_from(
    base_url: &str,
    public_key: &str,
    selected_channel: &str,
    selected_platform: &str,
    current_json: &str,
    current: &ReleaseManifest,
) -> Result<Option<(String, ReleaseManifest)>, UpdateError> {
    let history =
        fetch_history_index_from(base_url, public_key, selected_channel, selected_platform)?;
    latest_launcher_release_from_history(
        base_url,
        public_key,
        selected_channel,
        selected_platform,
        current_json,
        current,
        &history,
    )
}

fn latest_launcher_and_history_from(
    base_url: &str,
    public_key: &str,
    selected_channel: &str,
    selected_platform: &str,
    current_json: &str,
    current: &ReleaseManifest,
) -> Result<(Option<(String, ReleaseManifest)>, ReleaseHistory), UpdateError> {
    let history =
        fetch_history_index_from(base_url, public_key, selected_channel, selected_platform)?;
    let launcher = latest_launcher_release_from_history(
        base_url,
        public_key,
        selected_channel,
        selected_platform,
        current_json,
        current,
        &history,
    )?;
    Ok((launcher, history))
}

#[allow(clippy::too_many_arguments)]
fn latest_launcher_release_from_history(
    base_url: &str,
    public_key: &str,
    selected_channel: &str,
    selected_platform: &str,
    current_json: &str,
    current: &ReleaseManifest,
    history: &ReleaseHistory,
) -> Result<Option<(String, ReleaseManifest)>, UpdateError> {
    if let Some(reference) = &history.latest_launcher {
        let candidate = if reference.release_version == current.version {
            (current_json.to_string(), current.clone())
        } else {
            fetch_versioned_release_from(
                base_url,
                public_key,
                selected_channel,
                selected_platform,
                &reference.release_version,
            )?
        };
        if candidate.1.launcher_version != reference.launcher_version
            || !release_contains_launcher(&candidate.1)
            || (!current.launcher_version.trim().is_empty()
                && compare_versions(
                    current.launcher_version.trim(),
                    reference.launcher_version.trim(),
                ) == Ordering::Greater)
        {
            return Err(UpdateError::Manifest(
                crate::manifest::ManifestError::Schema,
            ));
        }
        return Ok(Some(candidate));
    }

    let mut latest =
        if !current.launcher_version.trim().is_empty() && release_contains_launcher(current) {
            Some((current_json.to_string(), current.clone()))
        } else {
            None
        };
    for entry in &history.versions {
        if entry.version == current.version {
            continue;
        }
        let candidate = fetch_versioned_release_from(
            base_url,
            public_key,
            selected_channel,
            selected_platform,
            &entry.version,
        )?;
        if candidate.1.launcher_version.trim().is_empty()
            || !release_contains_launcher(&candidate.1)
        {
            continue;
        }
        let replace = latest.as_ref().is_none_or(|(_, selected)| {
            compare_versions(
                candidate.1.launcher_version.trim(),
                selected.launcher_version.trim(),
            ) == Ordering::Greater
        });
        if replace {
            latest = Some(candidate);
        }
    }
    Ok(latest)
}

pub fn list_deprecated_plugin_versions() -> Result<Vec<PluginRelease>, UpdateError> {
    list_deprecated_plugin_versions_from(
        update_base_url()?,
        release_public_key()?,
        &channel(),
        platform(),
    )
}

fn list_deprecated_plugin_versions_from(
    base_url: &str,
    public_key: &str,
    selected_channel: &str,
    selected_platform: &str,
) -> Result<Vec<PluginRelease>, UpdateError> {
    let (_, current) =
        fetch_release_from(base_url, public_key, selected_channel, selected_platform)?;
    let mut result = Vec::new();
    let mut seen_plugin_hashes = HashSet::new();
    let mut newest_plugin_found = false;
    if let Some((_, artifact)) = plugin_release(&current) {
        seen_plugin_hashes.insert(artifact.sha256.to_ascii_lowercase());
        newest_plugin_found = true;
    }
    for entry in fetch_history_from(base_url, public_key, selected_channel, selected_platform)? {
        if entry.version == current.version {
            continue;
        }
        let manifest = fetch_versioned_manifest_from(
            base_url,
            public_key,
            selected_channel,
            selected_platform,
            &entry.version,
        )?;
        if let Some((release, artifact)) = plugin_release(&manifest) {
            if !seen_plugin_hashes.insert(artifact.sha256.to_ascii_lowercase()) {
                continue;
            }
            if !newest_plugin_found {
                newest_plugin_found = true;
                continue;
            }
            result.push(release);
        }
    }
    Ok(result)
}

pub fn plugin_status(root: &Path) -> Result<PluginStatus, UpdateError> {
    plugin_status_from(
        update_base_url()?,
        release_public_key()?,
        &channel(),
        platform(),
        root,
    )
}

fn plugin_status_from(
    base_url: &str,
    public_key: &str,
    selected_channel: &str,
    selected_platform: &str,
    root: &Path,
) -> Result<PluginStatus, UpdateError> {
    let (available, artifact) =
        latest_plugin_release_from(base_url, public_key, selected_channel, selected_platform)?;
    let installation = crate::install::inspect(root)?;
    let installed_artifact_path = installation.plugin_jars.first().cloned();
    let update_available = match installed_artifact_path.as_deref() {
        Some(path) => !path_matches_artifact(Path::new(path), &artifact),
        None => true,
    };
    Ok(PluginStatus {
        installed_artifact_path,
        available,
        update_available,
    })
}

pub fn install_plugin_version<F>(
    root: &Path,
    version: &str,
    launcher_version: &str,
    progress: F,
) -> Result<PluginRelease, UpdateError>
where
    F: FnMut(UpdateProgress),
{
    install_plugin_version_from(
        update_base_url()?,
        release_public_key()?,
        &channel(),
        platform(),
        root,
        version,
        launcher_version,
        progress,
    )
}

#[allow(clippy::too_many_arguments)]
fn install_plugin_version_from<F>(
    base_url: &str,
    public_key: &str,
    selected_channel: &str,
    selected_platform: &str,
    root: &Path,
    version: &str,
    launcher_version: &str,
    mut progress: F,
) -> Result<PluginRelease, UpdateError>
where
    F: FnMut(UpdateProgress),
{
    let history =
        fetch_history_index_from(base_url, public_key, selected_channel, selected_platform)?;
    if !history
        .versions
        .iter()
        .any(|entry| entry.version == version)
    {
        return Err(UpdateError::Manifest(
            crate::manifest::ManifestError::HistoryVersionMissing,
        ));
    }
    let manifest = fetch_versioned_manifest_from(
        base_url,
        public_key,
        selected_channel,
        selected_platform,
        version,
    )?;
    if compare_versions(launcher_version, &manifest.minimum_launcher_version) == Ordering::Less {
        return Err(UpdateError::DowngradeLauncherTooOld);
    }
    let installation = crate::install::inspect(root)?;
    if installation.plugin_jars.len() != 1 {
        return Err(UpdateError::DowngradeInstallationNotReady);
    }
    let artifact = release_plugin_artifact(&manifest)
        .ok_or(UpdateError::DowngradeArtifactMissing)?
        .clone();
    let staging = root.join(STAGING_DIRECTORY);
    let staged_path = staging.join(&artifact.path);
    fs::create_dir_all(staged_path.parent().ok_or(UpdateError::UnsafeStaging)?)?;
    let locations = fetch_artifact_locations_from(
        base_url,
        public_key,
        selected_channel,
        selected_platform,
        &history,
    )?;
    let url = artifact_download_url(
        base_url,
        selected_channel,
        selected_platform,
        version,
        &artifact,
        locations.as_ref(),
    )?;
    let mut done = 0;
    download_to_path(
        &url,
        &staged_path,
        artifact.size,
        &mut done,
        artifact.size,
        0,
        1,
        &mut progress,
    )?;
    verify_file(&staged_path, &artifact)?;
    // Reuse the install transaction: it moves the old single plugin to its
    // backup, swaps the verified staged JAR, and restores it on any failure.
    let mut plugin_manifest = manifest.clone();
    plugin_manifest.artifacts = vec![artifact.clone()];
    crate::install::apply_staged_mode(root, &staging, &plugin_manifest, false)?;
    progress(UpdateProgress::completed("applying", artifact.size, 1));
    Ok(PluginRelease {
        version: version.to_owned(),
        published_at: manifest.published_at,
        artifact_path: artifact.path,
    })
}

/// Replaces only the canonical game JAR with the one from an older, still
/// signed release. Java, the BMS-IR plugin, launcher settings, skins, and
/// every player database are left untouched, matching item #17 of the
/// checklist: a downgrade or unpatch must not touch anything else.
///
/// The target version must (1) still be listed in the signed history index,
/// (2) have its own independently signed manifest matching that exact
/// version, channel and platform, (3) declare a `minimum_launcher_version`
/// this launcher build still satisfies, and (4) only proceed once the
/// current installation is already complete (Java + plugin + JAR), so a
/// downgrade never has to guess about anything beyond the JAR itself.
pub fn downgrade_to_version<F>(
    root: &Path,
    target_version: &str,
    launcher_version: &str,
    progress: F,
) -> Result<ReleaseManifest, UpdateError>
where
    F: FnMut(UpdateProgress),
{
    downgrade_to_version_from(
        update_base_url()?,
        release_public_key()?,
        &channel(),
        platform(),
        root,
        target_version,
        launcher_version,
        progress,
    )
}

#[allow(clippy::too_many_arguments)]
fn downgrade_to_version_from<F>(
    base_url: &str,
    public_key: &str,
    selected_channel: &str,
    selected_platform: &str,
    root: &Path,
    target_version: &str,
    launcher_version: &str,
    mut progress: F,
) -> Result<ReleaseManifest, UpdateError>
where
    F: FnMut(UpdateProgress),
{
    let history =
        fetch_history_index_from(base_url, public_key, selected_channel, selected_platform)?;
    if !history
        .versions
        .iter()
        .any(|entry| entry.version == target_version)
    {
        return Err(UpdateError::Manifest(
            crate::manifest::ManifestError::HistoryVersionMissing,
        ));
    }

    let manifest = fetch_versioned_manifest_from(
        base_url,
        public_key,
        selected_channel,
        selected_platform,
        target_version,
    )?;

    if compare_versions(launcher_version, &manifest.minimum_launcher_version) == Ordering::Less {
        return Err(UpdateError::DowngradeLauncherTooOld);
    }
    let installation = crate::install::inspect(root)?;
    if !crate::install::is_ready(&installation) {
        return Err(UpdateError::DowngradeInstallationNotReady);
    }

    let jar_artifact = manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.path.eq_ignore_ascii_case(CANONICAL_GAME_JAR))
        .ok_or(UpdateError::DowngradeArtifactMissing)?
        .clone();

    let staging_directory = root.join(STAGING_DIRECTORY);
    fs::create_dir_all(&staging_directory)?;
    let staging_path = staging_directory.join("downgrade.jar");
    let locations = fetch_artifact_locations_from(
        base_url,
        public_key,
        selected_channel,
        selected_platform,
        &history,
    )?;
    let artifact_url = artifact_download_url(
        base_url,
        selected_channel,
        selected_platform,
        target_version,
        &jar_artifact,
        locations.as_ref(),
    )?;
    let mut bytes_done = 0_u64;
    download_to_path(
        &artifact_url,
        &staging_path,
        jar_artifact.size,
        &mut bytes_done,
        jar_artifact.size,
        0,
        1,
        &mut progress,
    )?;
    verify_file(&staging_path, &jar_artifact)?;
    set_executable_if_requested(&staging_path, jar_artifact.executable)?;

    let destination = root.join(CANONICAL_GAME_JAR);
    let backup = staging_directory.join("downgrade.jar.previous");
    replace_game_jar_with_rollback(
        &staging_path,
        &destination,
        &backup,
        |from, to| fs::rename(from, to),
        |path| fs::remove_file(path),
    )?;
    progress(UpdateProgress::completed("applying", jar_artifact.size, 1));
    Ok(manifest)
}

fn replace_game_jar_with_rollback<R, D>(
    staging_path: &Path,
    destination: &Path,
    backup: &Path,
    mut rename: R,
    mut remove_file: D,
) -> io::Result<()>
where
    R: FnMut(&Path, &Path) -> io::Result<()>,
    D: FnMut(&Path) -> io::Result<()>,
{
    let had_previous = destination.exists();
    if had_previous {
        rename(destination, backup)?;
    }
    if let Err(swap_error) = rename(staging_path, destination) {
        if had_previous {
            if let Err(restore_error) = rename(backup, destination) {
                return Err(io::Error::new(
                    restore_error.kind(),
                    format!(
                        "game JAR replacement failed ({swap_error}); restoring the previous JAR also failed ({restore_error})"
                    ),
                ));
            }
        }
        return Err(swap_error);
    }
    if had_previous {
        // The replacement already succeeded. Retaining a stale backup is safer
        // than reporting the downgrade as failed after the new JAR is active.
        let _ = remove_file(backup);
    }
    Ok(())
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
    launcher_update_available: bool,
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
    } else if launcher_update_available {
        "launcher_available"
    } else {
        "current"
    })
}

#[cfg(test)]
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
    let launcher_release = (!release.launcher_version.trim().is_empty()
        && release_contains_launcher(release))
    .then_some(release);
    update_info_from_releases(
        installed,
        installation_ready,
        launcher_version,
        release,
        launcher_release,
    )
}

fn update_info_from_releases(
    installed: String,
    installation_ready: bool,
    launcher_version: &str,
    release: &ReleaseManifest,
    launcher_release: Option<&ReleaseManifest>,
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
    let available_launcher_version = launcher_release
        .map(|candidate| candidate.launcher_version.trim())
        .filter(|version| !version.is_empty())
        .unwrap_or(launcher_version)
        .to_string();
    let launcher_update_available = launcher_release.is_some()
        && compare_versions(launcher_version, &available_launcher_version) == Ordering::Less;
    let body_update_available = !installation_ready
        || revoked
        || compare_versions(&installed, &release.version) == Ordering::Less;
    let status = status_for_versions(
        &installed,
        &release.version,
        installation_ready,
        revoked,
        launcher_old,
        launcher_update_available,
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
        installed_launcher_version: launcher_version.to_string(),
        available_launcher_version,
        body_update_available,
        launcher_update_available,
        available_published_at: release.published_at.clone(),
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
    ) || (update.mandatory && (update.body_update_available || update.launcher_update_available))
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
    let selected_channel = channel();
    let selected_platform = platform();
    let base_url = update_base_url()?;
    let public_key = release_public_key()?;
    let (input, release) =
        fetch_release_from(base_url, public_key, &selected_channel, selected_platform)?;
    let (launcher_release, _history) = latest_launcher_and_history_from(
        base_url,
        public_key,
        &selected_channel,
        selected_platform,
        &input,
        &release,
    )?;
    let update = update_info_from_releases(
        installed,
        installation_ready,
        env!("CARGO_PKG_VERSION"),
        &release,
        launcher_release.as_ref().map(|(_, manifest)| manifest),
    )?;
    cache_verified_release(root, &input)?;
    Ok(update)
}

fn path_matches_artifact(path: &Path, artifact: &ReleaseArtifact) -> bool {
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
    verify_file(path, artifact).is_ok()
}

fn artifact_matches(root: &Path, artifact: &ReleaseArtifact) -> bool {
    path_matches_artifact(&root.join(&artifact.path), artifact)
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
    target: UpdateTarget,
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
        target,
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
    target: UpdateTarget,
    mut progress: F,
) -> Result<PreparedUpdate, UpdateError>
where
    F: FnMut(UpdateProgress),
{
    let root = root.canonicalize()?;
    let (manifest_json, release) =
        fetch_release_from(base_url, public_key, selected_channel, selected_platform)?;
    let (latest_launcher, history) = latest_launcher_and_history_from(
        base_url,
        public_key,
        selected_channel,
        selected_platform,
        &manifest_json,
        &release,
    )?;
    if !installation_ready && !bootstrap_artifacts_present(&release) {
        return Err(UpdateError::IncompleteBootstrap);
    }
    if !installation_ready && target != UpdateTarget::All {
        return Err(UpdateError::IncompleteBootstrap);
    }
    let installed = installed_version(&root);
    let revoked = release
        .revoked_versions
        .iter()
        .any(|value| value == &installed);
    let launcher_update_available = latest_launcher.as_ref().is_some_and(|(_, candidate)| {
        compare_versions(env!("CARGO_PKG_VERSION"), candidate.launcher_version.trim())
            == Ordering::Less
    });
    let body_update_available = !installation_ready
        || revoked
        || compare_versions(&installed, &release.version) == Ordering::Less;
    let target_available = match target {
        UpdateTarget::All => body_update_available || launcher_update_available,
        UpdateTarget::Body => body_update_available,
        UpdateTarget::Launcher => launcher_update_available,
    };
    if !target_available {
        return Err(UpdateError::SelectedComponentCurrent);
    }
    let effective_target = if target == UpdateTarget::All && installation_ready {
        match (body_update_available, launcher_update_available) {
            (true, true) => UpdateTarget::All,
            (true, false) => UpdateTarget::Body,
            (false, true) => UpdateTarget::Launcher,
            (false, false) => return Err(UpdateError::SelectedComponentCurrent),
        }
    } else {
        target
    };
    let launcher_for_selection = if effective_target != UpdateTarget::Body
        && (launcher_update_available || !installation_ready)
    {
        latest_launcher.as_ref().map(|(_, manifest)| manifest)
    } else {
        None
    };
    let selected_release =
        release_for_target_with_launcher(&release, launcher_for_selection, effective_target)?;
    let staging_parent = root.join(STAGING_DIRECTORY);
    if staging_parent.exists() && staging_parent.symlink_metadata()?.file_type().is_symlink() {
        return Err(UpdateError::UnsafeStaging);
    }
    fs::create_dir_all(&staging_parent)?;
    let staging = staging_parent.join(format!(
        "{}-{}",
        release.version,
        effective_target.as_directory_name()
    ));
    if staging.exists() {
        if staging.symlink_metadata()?.file_type().is_symlink() {
            return Err(UpdateError::UnsafeStaging);
        }
        fs::remove_dir_all(&staging)?;
    }
    fs::create_dir_all(&staging)?;

    let bootstrap_install = !installation_ready;
    let bootstrap = if bootstrap_install {
        selected_release.bootstrap.as_ref()
    } else {
        None
    };
    let artifact_source_version = |artifact: &ReleaseArtifact| {
        launcher_for_selection
            .filter(|launcher| launcher.version != release.version)
            .filter(|launcher| {
                launcher.artifacts.iter().any(|candidate| {
                    is_launcher_artifact(launcher, candidate)
                        && artifacts_match(candidate, artifact)
                })
            })
            .map(|launcher| launcher.version.clone())
            .unwrap_or_else(|| release.version.clone())
    };
    let download_artifacts = if bootstrap.is_some() {
        bootstrap_delta_artifacts(&selected_release)
            .into_iter()
            .map(|artifact| {
                let source_version = artifact_source_version(&artifact);
                (artifact, source_version)
            })
            .collect::<Vec<_>>()
    } else if bootstrap_install {
        selected_release
            .artifacts
            .iter()
            .cloned()
            .map(|artifact| {
                let source_version = artifact_source_version(&artifact);
                (artifact, source_version)
            })
            .collect::<Vec<_>>()
    } else {
        selected_release
            .artifacts
            .iter()
            .filter(|artifact| !artifact_matches(&root, artifact))
            .cloned()
            .map(|artifact| {
                let source_version = artifact_source_version(&artifact);
                (artifact, source_version)
            })
            .collect::<Vec<_>>()
    };
    let artifact_locations = if download_artifacts.is_empty() {
        None
    } else {
        fetch_artifact_locations_from(
            base_url,
            public_key,
            selected_channel,
            selected_platform,
            &history,
        )?
    };
    let bytes_total = bootstrap.map_or(0, |value| value.size).saturating_add(
        download_artifacts
            .iter()
            .fold(0_u64, |total, (artifact, _)| {
                total.saturating_add(artifact.size)
            }),
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

    for (artifact, source_version) in &download_artifacts {
        let destination = staging.join(&artifact.path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let url = artifact_download_url(
            base_url,
            selected_release.channel.as_str(),
            selected_release.platform.as_str(),
            source_version.as_str(),
            artifact,
            artifact_locations.as_ref(),
        )?;
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
    let launcher_manifest_path = latest_launcher
        .as_ref()
        .filter(|(_, manifest)| {
            launcher_for_selection.is_some() && manifest.version != release.version
        })
        .map(|(launcher_json, _)| {
            let path = staging.join(STAGED_LAUNCHER_MANIFEST);
            fs::write(&path, launcher_json)?;
            Ok::<PathBuf, UpdateError>(path)
        })
        .transpose()?;
    let verified_files_total = effective_file_count(&selected_release, bootstrap_install);
    Ok(PreparedUpdate {
        manifest: selected_release,
        staging,
        manifest_path,
        launcher_manifest_path,
        bootstrap_install,
        transfer_bytes_total: bytes_total,
        verified_files_total,
        target: effective_target,
        writes_body_version: effective_target.includes_body() && body_update_available,
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
    fn configured_channel_does_not_depend_on_distributed_executable_name() {
        let github_asset =
            Path::new("C:/Arena/BMS-IR-Arena-Test-Launcher-0.2.22-windows-x86-64.exe");
        assert_eq!(configured_channel(Some("test")), Some("test"));
        assert_eq!(configured_channel(Some("stable")), Some("stable"));
        assert_eq!(configured_channel(None), None);
        assert_eq!(configured_channel(Some("preview")), None);
        assert_eq!(channel_from_executable_path(github_asset), "stable");
        assert_eq!(
            channel_from_configuration_or_path(Some("test"), github_asset),
            "test"
        );
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
        assert_eq!(
            compare_versions("0.4.14.37", "0.4.14.00037"),
            Ordering::Equal
        );
    }

    #[test]
    fn missing_installation_can_bootstrap_the_same_version() {
        assert_eq!(
            status_for_versions("0.4.14", "0.4.14", false, false, false, false).unwrap(),
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
            launcher_version: String::new(),
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
    fn update_info_carries_the_release_published_at_timestamp() {
        let artifact = |path: &str| crate::manifest::ReleaseArtifact {
            path: path.into(),
            sha256: "00".repeat(32),
            size: 1,
            executable: false,
        };
        let release = ReleaseManifest {
            schema_version: 1,
            channel: "test".into(),
            platform: "windows-x64".into(),
            version: "0.4.15".into(),
            published_at: "2026-08-06T12:00:00Z".into(),
            release_notes_markdown: String::new(),
            release_notes_markdown_ja: String::new(),
            release_notes_markdown_en: String::new(),
            announcements: vec![],
            mandatory: false,
            minimum_launcher_version: "0.2.1".into(),
            launcher_version: String::new(),
            revoked_versions: vec![],
            bootstrap: None,
            artifacts: vec![
                artifact("Arena-oraja.jar"),
                artifact("runtime/bin/java.exe"),
                artifact("ir/bms_ir_arena_oraja_0.0.69.jar"),
            ],
            signature: String::new(),
        };
        let update = update_info_from_release("0.4.14".into(), true, "0.2.11", &release).unwrap();
        assert_eq!(update.available_version, "0.4.15");
        assert_eq!(update.available_published_at, "2026-08-06T12:00:00Z");
    }

    #[test]
    fn launcher_update_is_independent_and_component_paths_are_selected_in_rust() {
        let artifact = |path: &str| ReleaseArtifact {
            path: path.into(),
            sha256: "00".repeat(32),
            size: 1,
            executable: path.ends_with("bmsir-arena-launcher"),
        };
        let release = ReleaseManifest {
            schema_version: 1,
            channel: "test".into(),
            platform: "macos-arm64".into(),
            version: "0.4.14.25".into(),
            published_at: "2026-08-10T00:00:00Z".into(),
            release_notes_markdown: String::new(),
            release_notes_markdown_ja: String::new(),
            release_notes_markdown_en: String::new(),
            announcements: vec![],
            mandatory: false,
            minimum_launcher_version: "0.2.17".into(),
            launcher_version: "0.2.20".into(),
            revoked_versions: vec![],
            bootstrap: None,
            artifacts: vec![
                artifact("Arena-oraja.jar"),
                artifact("ir/bms_ir_arena.jar"),
                artifact("BMS-IR Arena Test.app/Contents/Info.plist"),
                artifact("BMS-IR Arena Test.app/Contents/MacOS/bmsir-arena-launcher"),
            ],
            signature: String::new(),
        };
        let update =
            update_info_from_release("0.4.14.25".into(), true, "0.2.17", &release).unwrap();
        assert!(!update.body_update_available);
        assert!(update.launcher_update_available);
        assert_eq!(update.status, "launcher_available");

        let launcher = release_for_target(&release, UpdateTarget::Launcher).unwrap();
        assert_eq!(launcher.artifacts.len(), 2);
        assert!(launcher
            .artifacts
            .iter()
            .all(|item| item.path.contains(".app/")));
        let body = release_for_target(&release, UpdateTarget::Body).unwrap();
        assert_eq!(
            body.artifacts
                .iter()
                .map(|item| item.path.as_str())
                .collect::<Vec<_>>(),
            vec!["Arena-oraja.jar", "ir/bms_ir_arena.jar"]
        );
    }

    #[test]
    fn complete_installation_keeps_normal_version_rules() {
        assert_eq!(
            status_for_versions("0.4.14", "0.4.14", true, false, false, false).unwrap(),
            "current"
        );
        assert!(!bootstrap_allowed_for_versions(
            "0.4.14", "0.4.14", true, false
        ));
    }

    #[test]
    fn incomplete_newer_installation_does_not_silently_downgrade() {
        assert!(status_for_versions("0.4.15", "0.4.14", false, false, false, false).is_err());
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
            launcher_version: String::new(),
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
        let history_bytes = signed_history_bytes(
            &signing,
            "test",
            "windows-x64",
            &[("0.4.14.9", "2026-08-04T00:00:00Z")],
        );

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let responses = [
                (
                    "/channels/test/windows-x64/manifest.json",
                    manifest_bytes.as_slice(),
                ),
                (
                    "/channels/test/windows-x64/history.json",
                    history_bytes.as_slice(),
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
            UpdateTarget::All,
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
        let history_bytes = signed_history_bytes(
            &signing,
            "test",
            "windows-x64",
            &[("0.4.14", "2026-08-03T00:00:00Z")],
        );

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let mut responses = HashMap::new();
        responses.insert(
            "/channels/test/windows-x64/manifest.json".to_string(),
            manifest_bytes,
        );
        responses.insert(
            "/channels/test/windows-x64/history.json".to_string(),
            history_bytes,
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
            UpdateTarget::All,
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

    #[test]
    fn signed_external_location_overrides_pages_and_mismatch_fails_closed() {
        let artifact = ReleaseArtifact {
            path: "Arena-oraja.jar".into(),
            sha256: "12".repeat(32),
            size: 123,
            executable: false,
        };
        let locations = ArtifactLocations {
            schema_version: 1,
            channel: "test".into(),
            platform: "windows-x64".into(),
            locations: vec![crate::manifest::ArtifactLocation {
                version: "0.4.14.49".into(),
                path: artifact.path.clone(),
                sha256: artifact.sha256.clone(),
                size: artifact.size,
                url: "https://github.com/tenP0312-dev/bms-ir-arena-patch-server/releases/download/test-0.4.14.49/windows-x64-Arena-oraja.jar".into(),
                retain_on_pages: false,
            }],
            signature: String::new(),
        };
        let external = artifact_download_url(
            "https://example.test/patches",
            "test",
            "windows-x64",
            "0.4.14.49",
            &artifact,
            Some(&locations),
        )
        .unwrap();
        assert_eq!(external.host_str(), Some("github.com"));
        assert!(external.path().ends_with("/windows-x64-Arena-oraja.jar"));

        let legacy = artifact_download_url(
            "https://example.test/patches",
            "test",
            "windows-x64",
            "0.4.14.49",
            &artifact,
            None,
        )
        .unwrap();
        assert_eq!(
            legacy.as_str(),
            "https://example.test/patches/channels/test/windows-x64/releases/0.4.14.49/Arena-oraja.jar"
        );

        let mismatched = ReleaseArtifact {
            size: 124,
            ..artifact
        };
        assert!(matches!(
            artifact_download_url(
                "https://example.test/patches",
                "test",
                "windows-x64",
                "0.4.14.49",
                &mismatched,
                Some(&locations),
            ),
            Err(UpdateError::Manifest(
                crate::manifest::ManifestError::ArtifactLocationMismatch(_, _)
            ))
        ));
    }

    fn signed_history_bytes(
        signing: &SigningKey,
        channel: &str,
        platform: &str,
        versions: &[(&str, &str)],
    ) -> Vec<u8> {
        let mut history = json!({
            "schema_version": 1,
            "channel": channel,
            "platform": platform,
            "versions": versions
                .iter()
                .map(|(version, published_at)| json!({
                    "version": version,
                    "published_at": published_at,
                }))
                .collect::<Vec<_>>(),
        });
        let signature = signing.sign(&serde_jcs::to_vec(&history).unwrap());
        history["signature"] = Value::String(STANDARD.encode(signature.to_bytes()));
        serde_json::to_vec(&history).unwrap()
    }

    fn signed_history_with_latest_launcher_bytes(
        signing: &SigningKey,
        channel: &str,
        platform: &str,
        versions: &[(&str, &str)],
        release_version: &str,
        launcher_version: &str,
    ) -> Vec<u8> {
        let mut history: Value =
            serde_json::from_slice(&signed_history_bytes(signing, channel, platform, versions))
                .unwrap();
        history.as_object_mut().unwrap().remove("signature");
        history["latest_launcher"] = json!({
            "release_version": release_version,
            "launcher_version": launcher_version,
        });
        let signature = signing.sign(&serde_jcs::to_vec(&history).unwrap());
        history["signature"] = Value::String(STANDARD.encode(signature.to_bytes()));
        serde_json::to_vec(&history).unwrap()
    }

    fn signed_downgrade_manifest_bytes(
        signing: &SigningKey,
        channel: &str,
        platform: &str,
        version: &str,
        minimum_launcher_version: &str,
        artifacts: Vec<Value>,
    ) -> Vec<u8> {
        let mut manifest = json!({
            "schema_version": 1,
            "channel": channel,
            "platform": platform,
            "version": version,
            "published_at": "2026-08-01T00:00:00Z",
            "release_notes_markdown": "",
            "mandatory": false,
            "minimum_launcher_version": minimum_launcher_version,
            "revoked_versions": [],
            "artifacts": artifacts,
        });
        let signature = signing.sign(&serde_jcs::to_vec(&manifest).unwrap());
        manifest["signature"] = Value::String(STANDARD.encode(signature.to_bytes()));
        serde_json::to_vec(&manifest).unwrap()
    }

    fn signed_launcher_manifest_bytes(
        signing: &SigningKey,
        version: &str,
        launcher_version: &str,
    ) -> Vec<u8> {
        let launcher = format!("launcher-{launcher_version}");
        let mut manifest = json!({
            "schema_version": 1,
            "channel": "test",
            "platform": "windows-x64",
            "version": version,
            "published_at": "2026-08-13T00:00:00Z",
            "release_notes_markdown": "",
            "mandatory": false,
            "minimum_launcher_version": "0.2.20",
            "launcher_version": launcher_version,
            "revoked_versions": [],
            "artifacts": [{
                "path": "BMS-IR Arena Test.exe",
                "sha256": format!("{:x}", Sha256::digest(launcher.as_bytes())),
                "size": launcher.len(),
                "executable": true
            }]
        });
        let signature = signing.sign(&serde_jcs::to_vec(&manifest).unwrap());
        manifest["signature"] = Value::String(STANDARD.encode(signature.to_bytes()));
        serde_json::to_vec(&manifest).unwrap()
    }

    fn game_jar_artifact(bytes: &[u8]) -> Value {
        json!({
            "path": CANONICAL_GAME_JAR,
            "sha256": format!("{:x}", Sha256::digest(bytes)),
            "size": bytes.len(),
            "executable": false
        })
    }

    fn plugin_artifact(path: &str, bytes: &[u8]) -> Value {
        json!({
            "path": path,
            "sha256": format!("{:x}", Sha256::digest(bytes)),
            "size": bytes.len(),
            "executable": false
        })
    }

    /// Spawns a thread that replies to `responses.len()` sequential requests
    /// on one connection each, asserting the exact path requested for every
    /// reply in order.
    fn serve_sequential_responses(
        listener: TcpListener,
        responses: Vec<(&'static str, Vec<u8>)>,
    ) -> thread::JoinHandle<()> {
        thread::spawn(move || {
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
                stream.write_all(&body).unwrap();
            }
        })
    }

    #[test]
    fn latest_launcher_uses_the_maximum_signed_launcher_version_independent_of_history_order() {
        let resolve = |newest_first: bool| {
            let signing = SigningKey::from_bytes(&[55_u8; 32]);
            let ordered_versions = if newest_first {
                vec![
                    ("0.4.14.036", "2026-08-13T01:00:00Z"),
                    ("0.4.14.037", "2026-08-13T00:00:00Z"),
                ]
            } else {
                vec![
                    ("0.4.14.037", "2026-08-13T00:00:00Z"),
                    ("0.4.14.036", "2026-08-13T01:00:00Z"),
                ]
            };
            let history = signed_history_bytes(&signing, "test", "windows-x64", &ordered_versions);
            let launcher_022 = signed_launcher_manifest_bytes(&signing, "0.4.14.036", "0.2.22");
            let launcher_023 = signed_launcher_manifest_bytes(&signing, "0.4.14.037", "0.2.23");
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let responses = if newest_first {
                vec![
                    ("/channels/test/windows-x64/history.json", history),
                    (
                        "/channels/test/windows-x64/manifests/0.4.14.036.json",
                        launcher_022,
                    ),
                    (
                        "/channels/test/windows-x64/manifests/0.4.14.037.json",
                        launcher_023,
                    ),
                ]
            } else {
                vec![
                    ("/channels/test/windows-x64/history.json", history),
                    (
                        "/channels/test/windows-x64/manifests/0.4.14.037.json",
                        launcher_023,
                    ),
                    (
                        "/channels/test/windows-x64/manifests/0.4.14.036.json",
                        launcher_022,
                    ),
                ]
            };
            let server = serve_sequential_responses(listener, responses);
            let current = ReleaseManifest {
                schema_version: 1,
                channel: "test".into(),
                platform: "windows-x64".into(),
                version: "0.4.14.00037".into(),
                published_at: "2026-08-14T00:00:00Z".into(),
                release_notes_markdown: String::new(),
                release_notes_markdown_ja: String::new(),
                release_notes_markdown_en: String::new(),
                announcements: vec![],
                mandatory: false,
                minimum_launcher_version: "0.2.20".into(),
                launcher_version: String::new(),
                revoked_versions: vec![],
                bootstrap: None,
                artifacts: vec![ReleaseArtifact {
                    path: "ir/bms_ir_arena_oraja_0.0.70.jar".into(),
                    sha256: "00".repeat(32),
                    size: 1,
                    executable: false,
                }],
                signature: String::new(),
            };
            let latest = latest_launcher_release_from(
                &format!("http://{address}"),
                &STANDARD.encode(signing.verifying_key().to_bytes()),
                "test",
                "windows-x64",
                "current",
                &current,
            )
            .unwrap()
            .unwrap()
            .1;
            server.join().unwrap();
            (current, latest)
        };

        for newest_first in [true, false] {
            let (current, latest) = resolve(newest_first);
            assert_eq!(latest.launcher_version, "0.2.23");
            let older = update_info_from_releases(
                "0.4.14.37".into(),
                true,
                "0.2.20",
                &current,
                Some(&latest),
            )
            .unwrap();
            assert!(older.launcher_update_available);
            assert_eq!(older.available_launcher_version, "0.2.23");
            assert_eq!(older.status, "launcher_available");

            let current_launcher = update_info_from_releases(
                "0.4.14.37".into(),
                true,
                "0.2.23",
                &current,
                Some(&latest),
            )
            .unwrap();
            assert!(!current_launcher.launcher_update_available);
            assert_eq!(current_launcher.status, "current");

            let combined =
                release_for_target_with_launcher(&current, Some(&latest), UpdateTarget::All)
                    .unwrap();
            assert_eq!(combined.launcher_version, "0.2.23");
            assert!(combined
                .artifacts
                .iter()
                .any(|artifact| artifact.path == "ir/bms_ir_arena_oraja_0.0.70.jar"));
            assert!(combined
                .artifacts
                .iter()
                .any(|artifact| artifact.path == "BMS-IR Arena Test.exe"));
        }
    }

    #[test]
    fn signed_latest_launcher_pointer_fetches_only_the_selected_manifest() {
        let signing = SigningKey::from_bytes(&[56_u8; 32]);
        let history = signed_history_with_latest_launcher_bytes(
            &signing,
            "test",
            "windows-x64",
            &[
                ("0.4.14.46", "2026-08-15T00:00:00Z"),
                ("0.4.14.44", "2026-08-14T00:00:00Z"),
                ("0.4.14.43", "2026-08-13T00:00:00Z"),
            ],
            "0.4.14.44",
            "0.2.25",
        );
        let launcher = signed_launcher_manifest_bytes(&signing, "0.4.14.44", "0.2.25");
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = serve_sequential_responses(
            listener,
            vec![
                ("/channels/test/windows-x64/history.json", history),
                (
                    "/channels/test/windows-x64/manifests/0.4.14.44.json",
                    launcher,
                ),
            ],
        );
        let current = ReleaseManifest {
            schema_version: 1,
            channel: "test".into(),
            platform: "windows-x64".into(),
            version: "0.4.14.46".into(),
            published_at: "2026-08-15T00:00:00Z".into(),
            release_notes_markdown: String::new(),
            release_notes_markdown_ja: String::new(),
            release_notes_markdown_en: String::new(),
            announcements: vec![],
            mandatory: false,
            minimum_launcher_version: "0.2.20".into(),
            launcher_version: String::new(),
            revoked_versions: vec![],
            bootstrap: None,
            artifacts: vec![ReleaseArtifact {
                path: CANONICAL_GAME_JAR.into(),
                sha256: "00".repeat(32),
                size: 1,
                executable: false,
            }],
            signature: String::new(),
        };
        let latest = latest_launcher_release_from(
            &format!("http://{address}"),
            &STANDARD.encode(signing.verifying_key().to_bytes()),
            "test",
            "windows-x64",
            "current",
            &current,
        )
        .unwrap()
        .unwrap()
        .1;
        server.join().unwrap();
        assert_eq!(latest.version, "0.4.14.44");
        assert_eq!(latest.launcher_version, "0.2.25");
    }

    #[cfg(unix)]
    fn write_fake_java(path: &Path) {
        use std::os::unix::fs::PermissionsExt;
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            path,
            "#!/bin/sh\necho 'openjdk version \"21.0.1\" 2026-01-01' 1>&2\n",
        )
        .unwrap();
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).unwrap();
    }

    /// Builds a root that `install::is_ready` reports as complete: a game
    /// JAR, exactly one plugin JAR, and a bundled Java that reports a
    /// supported major version, so downgrade tests can reach the
    /// installation-readiness gate deterministically instead of depending on
    /// whatever Java (if any) happens to be on the host running the tests.
    #[cfg(unix)]
    fn make_ready_installation(root: &Path, game_jar_bytes: &[u8]) {
        fs::write(root.join(CANONICAL_GAME_JAR), game_jar_bytes).unwrap();
        write_fake_java(&root.join("runtime/bin/java"));
        fs::create_dir_all(root.join("ir")).unwrap();
        fs::write(root.join("ir/bms_ir_arena_oraja_0.0.69.jar"), b"plugin").unwrap();
    }

    #[test]
    fn list_deprecated_versions_from_excludes_the_current_version() {
        let signing = SigningKey::from_bytes(&[41_u8; 32]);
        let history_bytes = signed_history_bytes(
            &signing,
            "test",
            "windows-x64",
            &[
                ("0.4.14", "2026-08-01T00:00:00Z"),
                ("0.4.13", "2026-07-01T00:00:00Z"),
                ("0.4.12", "2026-06-01T00:00:00Z"),
            ],
        );
        let manifest_bytes = signed_downgrade_manifest_bytes(
            &signing,
            "test",
            "windows-x64",
            "0.4.14",
            "0.2.0",
            vec![],
        );
        let body_013_manifest = signed_downgrade_manifest_bytes(
            &signing,
            "test",
            "windows-x64",
            "0.4.13",
            "0.2.0",
            vec![game_jar_artifact(b"0.4.13")],
        );
        let body_012_manifest = signed_downgrade_manifest_bytes(
            &signing,
            "test",
            "windows-x64",
            "0.4.12",
            "0.2.0",
            vec![game_jar_artifact(b"0.4.12")],
        );

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = serve_sequential_responses(
            listener,
            vec![
                ("/channels/test/windows-x64/history.json", history_bytes),
                ("/channels/test/windows-x64/manifest.json", manifest_bytes),
                (
                    "/channels/test/windows-x64/manifests/0.4.13.json",
                    body_013_manifest,
                ),
                (
                    "/channels/test/windows-x64/manifests/0.4.12.json",
                    body_012_manifest,
                ),
            ],
        );

        let versions = list_deprecated_versions_from(
            &format!("http://{address}"),
            &STANDARD.encode(signing.verifying_key().to_bytes()),
            "test",
            "windows-x64",
            "0.4.14",
        )
        .unwrap();
        server.join().unwrap();

        assert_eq!(
            versions
                .iter()
                .map(|entry| entry.version.as_str())
                .collect::<Vec<_>>(),
            vec!["0.4.13", "0.4.12"]
        );
        assert_eq!(versions[0].published_at, "2026-07-01T00:00:00Z");
    }

    #[test]
    fn deprecated_plugin_versions_are_signed_distinct_and_plugin_bearing() {
        let signing = SigningKey::from_bytes(&[55_u8; 32]);
        let history = signed_history_bytes(
            &signing,
            "test",
            "windows-x64",
            &[
                ("0.4.14.24", "2026-08-09T00:00:00Z"),
                ("0.4.14.23", "2026-08-08T00:00:00Z"),
                ("0.4.14.22", "2026-08-07T00:00:00Z"),
                ("0.2.16", "2026-08-06T00:00:00Z"),
            ],
        );
        let current = signed_downgrade_manifest_bytes(
            &signing,
            "test",
            "windows-x64",
            "0.4.14.24",
            "0.2.0",
            vec![plugin_artifact("ir/bms_ir_arena_0.0.70.jar", b"current")],
        );
        let old = signed_downgrade_manifest_bytes(
            &signing,
            "test",
            "windows-x64",
            "0.4.14.23",
            "0.2.0",
            vec![plugin_artifact("IR/bms_ir_arena_0.0.69.jar", b"old")],
        );
        let duplicate = signed_downgrade_manifest_bytes(
            &signing,
            "test",
            "windows-x64",
            "0.4.14.22",
            "0.2.0",
            vec![plugin_artifact("ir/bms_ir_arena_0.0.69.jar", b"old")],
        );
        let launcher_only = signed_downgrade_manifest_bytes(
            &signing,
            "test",
            "windows-x64",
            "0.2.16",
            "0.2.0",
            vec![],
        );

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = serve_sequential_responses(
            listener,
            vec![
                ("/channels/test/windows-x64/manifest.json", current),
                ("/channels/test/windows-x64/history.json", history),
                ("/channels/test/windows-x64/manifests/0.4.14.23.json", old),
                (
                    "/channels/test/windows-x64/manifests/0.4.14.22.json",
                    duplicate,
                ),
                (
                    "/channels/test/windows-x64/manifests/0.2.16.json",
                    launcher_only,
                ),
            ],
        );

        let releases = list_deprecated_plugin_versions_from(
            &format!("http://{address}"),
            &STANDARD.encode(signing.verifying_key().to_bytes()),
            "test",
            "windows-x64",
        )
        .unwrap();
        server.join().unwrap();

        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].version, "0.4.14.23");
        assert_eq!(releases[0].artifact_path, "IR/bms_ir_arena_0.0.69.jar");
    }

    #[test]
    fn plugin_status_uses_newest_plugin_bearing_history_release_and_compares_bytes() {
        let signing = SigningKey::from_bytes(&[57_u8; 32]);
        let history = signed_history_bytes(
            &signing,
            "test",
            "windows-x64",
            &[
                ("0.4.14.37", "2026-08-12T20:00:00Z"),
                ("0.4.14.36", "2026-08-12T14:00:00Z"),
                ("0.4.14.25", "2026-08-09T02:00:00Z"),
            ],
        );
        let current = signed_downgrade_manifest_bytes(
            &signing,
            "test",
            "windows-x64",
            "0.4.14.37",
            "0.2.0",
            vec![game_jar_artifact(b"current-body")],
        );
        let body_only = signed_downgrade_manifest_bytes(
            &signing,
            "test",
            "windows-x64",
            "0.4.14.36",
            "0.2.0",
            vec![game_jar_artifact(b"older-body")],
        );
        let plugin_bytes = b"newest-signed-plugin";
        let plugin_release = signed_downgrade_manifest_bytes(
            &signing,
            "test",
            "windows-x64",
            "0.4.14.25",
            "0.2.0",
            vec![plugin_artifact(
                "ir/bms_ir_arena_oraja_0.0.69.jar",
                plugin_bytes,
            )],
        );

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = serve_sequential_responses(
            listener,
            vec![
                ("/channels/test/windows-x64/manifest.json", current),
                ("/channels/test/windows-x64/history.json", history),
                (
                    "/channels/test/windows-x64/manifests/0.4.14.36.json",
                    body_only,
                ),
                (
                    "/channels/test/windows-x64/manifests/0.4.14.25.json",
                    plugin_release,
                ),
            ],
        );

        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("ir")).unwrap();
        let renamed = root.path().join("ir/bms_ir_arena_oraja_renamed.jar");
        fs::write(&renamed, plugin_bytes).unwrap();
        let status = plugin_status_from(
            &format!("http://{address}"),
            &STANDARD.encode(signing.verifying_key().to_bytes()),
            "test",
            "windows-x64",
            root.path(),
        )
        .unwrap();
        server.join().unwrap();

        assert_eq!(status.available.version, "0.4.14.25");
        assert_eq!(
            status.available.artifact_path,
            "ir/bms_ir_arena_oraja_0.0.69.jar"
        );
        assert_eq!(
            status.installed_artifact_path.as_deref().map(Path::new),
            Some(renamed.as_path())
        );
        assert!(!status.update_available);
    }

    #[test]
    fn plugin_status_marks_different_installed_bytes_as_update_available() {
        let signing = SigningKey::from_bytes(&[58_u8; 32]);
        let current = signed_downgrade_manifest_bytes(
            &signing,
            "test",
            "windows-x64",
            "0.4.14.38",
            "0.2.0",
            vec![plugin_artifact(
                "ir/bms_ir_arena_oraja_0.0.70.jar",
                b"new-plugin",
            )],
        );
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = serve_sequential_responses(
            listener,
            vec![("/channels/test/windows-x64/manifest.json", current)],
        );
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("ir")).unwrap();
        fs::write(
            root.path().join("ir/bms_ir_arena_oraja_0.0.69.jar"),
            b"old-plugin",
        )
        .unwrap();

        let status = plugin_status_from(
            &format!("http://{address}"),
            &STANDARD.encode(signing.verifying_key().to_bytes()),
            "test",
            "windows-x64",
            root.path(),
        )
        .unwrap();
        server.join().unwrap();

        assert_eq!(status.available.version, "0.4.14.38");
        assert!(status.update_available);
    }

    #[test]
    fn historical_plugin_install_replaces_only_the_plugin() {
        let signing = SigningKey::from_bytes(&[56_u8; 32]);
        let version = "0.4.14.23";
        let plugin_path = "ir/bms_ir_arena_0.0.69.jar";
        let plugin_bytes = b"historical-plugin";
        let history = signed_history_bytes(
            &signing,
            "test",
            "windows-x64",
            &[(version, "2026-08-08T00:00:00Z")],
        );
        let manifest = signed_downgrade_manifest_bytes(
            &signing,
            "test",
            "windows-x64",
            version,
            "0.2.0",
            vec![plugin_artifact(plugin_path, plugin_bytes)],
        );
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = serve_sequential_responses(
            listener,
            vec![
                ("/channels/test/windows-x64/history.json", history),
                (
                    "/channels/test/windows-x64/manifests/0.4.14.23.json",
                    manifest,
                ),
                (
                    "/channels/test/windows-x64/releases/0.4.14.23/ir/bms_ir_arena_0.0.69.jar",
                    plugin_bytes.to_vec(),
                ),
            ],
        );

        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("ir")).unwrap();
        fs::create_dir_all(root.path().join("skin")).unwrap();
        fs::write(root.path().join("ir/bms_ir_arena_0.0.70.jar"), b"current").unwrap();
        fs::write(root.path().join("playerconfig.json"), b"settings").unwrap();
        fs::write(root.path().join("score.db"), b"scores").unwrap();
        fs::write(root.path().join("skin/selected.json"), b"skin").unwrap();

        let mut progress = Vec::new();
        let installed = install_plugin_version_from(
            &format!("http://{address}"),
            &STANDARD.encode(signing.verifying_key().to_bytes()),
            "test",
            "windows-x64",
            root.path(),
            version,
            "0.2.17",
            |event| progress.push(event),
        )
        .unwrap();
        server.join().unwrap();

        assert_eq!(installed.version, version);
        assert_eq!(
            fs::read(root.path().join(plugin_path)).unwrap(),
            plugin_bytes
        );
        assert!(!root.path().join("ir/bms_ir_arena_0.0.70.jar").exists());
        assert_eq!(
            fs::read(root.path().join("playerconfig.json")).unwrap(),
            b"settings"
        );
        assert_eq!(fs::read(root.path().join("score.db")).unwrap(), b"scores");
        assert_eq!(
            fs::read(root.path().join("skin/selected.json")).unwrap(),
            b"skin"
        );
        assert_eq!(
            progress.last().map(|event| event.phase.as_str()),
            Some("applying")
        );
    }

    #[test]
    fn list_deprecated_versions_from_also_excludes_the_published_version() {
        // The installed version can differ from the channel's current
        // published version (an update is available but not yet installed).
        // The published release must still never appear as "deprecated" —
        // that is the exact release being offered as the update.
        let signing = SigningKey::from_bytes(&[49_u8; 32]);
        let history_bytes = signed_history_bytes(
            &signing,
            "test",
            "windows-x64",
            &[
                ("0.4.14.19", "2026-08-06T00:00:00Z"),
                ("0.4.13", "2026-07-01T00:00:00Z"),
            ],
        );
        let manifest_bytes = signed_downgrade_manifest_bytes(
            &signing,
            "test",
            "windows-x64",
            "0.4.14.19",
            "0.2.0",
            vec![],
        );
        let body_013_manifest = signed_downgrade_manifest_bytes(
            &signing,
            "test",
            "windows-x64",
            "0.4.13",
            "0.2.0",
            vec![game_jar_artifact(b"0.4.13")],
        );

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = serve_sequential_responses(
            listener,
            vec![
                ("/channels/test/windows-x64/history.json", history_bytes),
                ("/channels/test/windows-x64/manifest.json", manifest_bytes),
                (
                    "/channels/test/windows-x64/manifests/0.4.13.json",
                    body_013_manifest,
                ),
            ],
        );

        // The installed version (0.4.14.18) is neither history entry, so
        // without excluding the published version too, 0.4.14.19 would
        // incorrectly show up as "deprecated" even though it is the exact
        // release currently being offered as the update.
        let versions = list_deprecated_versions_from(
            &format!("http://{address}"),
            &STANDARD.encode(signing.verifying_key().to_bytes()),
            "test",
            "windows-x64",
            "0.4.14.18",
        )
        .unwrap();
        server.join().unwrap();

        assert_eq!(
            versions
                .iter()
                .map(|entry| entry.version.as_str())
                .collect::<Vec<_>>(),
            vec!["0.4.13"]
        );
    }

    #[test]
    fn list_deprecated_versions_from_omits_launcher_only_releases() {
        let signing = SigningKey::from_bytes(&[50_u8; 32]);
        let history_bytes = signed_history_bytes(
            &signing,
            "test",
            "windows-x64",
            &[
                ("0.4.14", "2026-08-01T00:00:00Z"),
                ("0.4.13", "2026-07-01T00:00:00Z"),
                ("0.2.11", "2026-06-01T00:00:00Z"),
            ],
        );
        let published_manifest = signed_downgrade_manifest_bytes(
            &signing,
            "test",
            "windows-x64",
            "0.4.14",
            "0.2.0",
            vec![],
        );
        let body_manifest = signed_downgrade_manifest_bytes(
            &signing,
            "test",
            "windows-x64",
            "0.4.13",
            "0.2.0",
            vec![game_jar_artifact(b"0.4.13")],
        );
        let launcher_only_manifest = signed_downgrade_manifest_bytes(
            &signing,
            "test",
            "windows-x64",
            "0.2.11",
            "0.2.0",
            vec![json!({
                "path": "BMS-IR Arena.exe",
                "sha256": format!("{:x}", Sha256::digest(b"launcher")),
                "size": 8,
                "executable": true
            })],
        );

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = serve_sequential_responses(
            listener,
            vec![
                ("/channels/test/windows-x64/history.json", history_bytes),
                (
                    "/channels/test/windows-x64/manifest.json",
                    published_manifest,
                ),
                (
                    "/channels/test/windows-x64/manifests/0.4.13.json",
                    body_manifest,
                ),
                (
                    "/channels/test/windows-x64/manifests/0.2.11.json",
                    launcher_only_manifest,
                ),
            ],
        );

        let versions = list_deprecated_versions_from(
            &format!("http://{address}"),
            &STANDARD.encode(signing.verifying_key().to_bytes()),
            "test",
            "windows-x64",
            "0.4.14",
        )
        .unwrap();
        server.join().unwrap();

        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].version, "0.4.13");
    }

    #[test]
    fn list_deprecated_versions_from_rejects_tampered_history() {
        let signing = SigningKey::from_bytes(&[42_u8; 32]);
        let mut history: Value = serde_json::from_slice(&signed_history_bytes(
            &signing,
            "test",
            "windows-x64",
            &[("0.4.14", "2026-08-01T00:00:00Z")],
        ))
        .unwrap();
        // Tamper with the payload after signing so the signature no longer
        // matches the content the launcher would verify.
        history["versions"][0]["version"] = Value::String("0.4.99".into());
        let tampered_bytes = serde_json::to_vec(&history).unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = serve_sequential_responses(
            listener,
            vec![("/channels/test/windows-x64/history.json", tampered_bytes)],
        );

        let result = list_deprecated_versions_from(
            &format!("http://{address}"),
            &STANDARD.encode(signing.verifying_key().to_bytes()),
            "test",
            "windows-x64",
            "0.4.14",
        );
        server.join().unwrap();

        assert!(matches!(
            result,
            Err(UpdateError::Manifest(
                crate::manifest::ManifestError::Signature
            ))
        ));
    }

    #[test]
    fn list_deprecated_versions_from_rejects_wrong_target_history() {
        let signing = SigningKey::from_bytes(&[43_u8; 32]);
        // Signed for the "stable" channel while the launcher requests "test".
        let history_bytes = signed_history_bytes(
            &signing,
            "stable",
            "windows-x64",
            &[("0.4.14", "2026-08-01T00:00:00Z")],
        );

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = serve_sequential_responses(
            listener,
            vec![("/channels/test/windows-x64/history.json", history_bytes)],
        );

        let result = list_deprecated_versions_from(
            &format!("http://{address}"),
            &STANDARD.encode(signing.verifying_key().to_bytes()),
            "test",
            "windows-x64",
            "0.4.14",
        );
        server.join().unwrap();

        assert!(matches!(
            result,
            Err(UpdateError::Manifest(
                crate::manifest::ManifestError::HistoryTarget
            ))
        ));
    }

    #[test]
    fn downgrade_to_version_from_rejects_a_target_missing_from_history() {
        let signing = SigningKey::from_bytes(&[44_u8; 32]);
        let history_bytes = signed_history_bytes(
            &signing,
            "test",
            "windows-x64",
            &[("0.4.14", "2026-08-01T00:00:00Z")],
        );

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = serve_sequential_responses(
            listener,
            vec![("/channels/test/windows-x64/history.json", history_bytes)],
        );

        let root = tempfile::tempdir().unwrap();
        let result = downgrade_to_version_from(
            &format!("http://{address}"),
            &STANDARD.encode(signing.verifying_key().to_bytes()),
            "test",
            "windows-x64",
            root.path(),
            "0.4.13",
            "0.2.11",
            |_| {},
        );
        server.join().unwrap();

        assert!(matches!(
            result,
            Err(UpdateError::Manifest(
                crate::manifest::ManifestError::HistoryVersionMissing
            ))
        ));
    }

    #[test]
    fn downgrade_to_version_from_rejects_a_launcher_that_is_too_old() {
        let signing = SigningKey::from_bytes(&[45_u8; 32]);
        let history_bytes = signed_history_bytes(
            &signing,
            "test",
            "windows-x64",
            &[
                ("0.4.14", "2026-08-01T00:00:00Z"),
                ("0.4.13", "2026-07-01T00:00:00Z"),
            ],
        );
        let manifest_bytes = signed_downgrade_manifest_bytes(
            &signing,
            "test",
            "windows-x64",
            "0.4.13",
            "0.3.0",
            vec![],
        );

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = serve_sequential_responses(
            listener,
            vec![
                ("/channels/test/windows-x64/history.json", history_bytes),
                (
                    "/channels/test/windows-x64/manifests/0.4.13.json",
                    manifest_bytes,
                ),
            ],
        );

        let root = tempfile::tempdir().unwrap();
        let result = downgrade_to_version_from(
            &format!("http://{address}"),
            &STANDARD.encode(signing.verifying_key().to_bytes()),
            "test",
            "windows-x64",
            root.path(),
            "0.4.13",
            "0.2.11",
            |_| {},
        );
        server.join().unwrap();

        assert!(matches!(result, Err(UpdateError::DowngradeLauncherTooOld)));
    }

    #[test]
    fn downgrade_to_version_from_rejects_an_incomplete_installation() {
        let signing = SigningKey::from_bytes(&[46_u8; 32]);
        let history_bytes = signed_history_bytes(
            &signing,
            "test",
            "windows-x64",
            &[
                ("0.4.14", "2026-08-01T00:00:00Z"),
                ("0.4.13", "2026-07-01T00:00:00Z"),
            ],
        );
        let manifest_bytes = signed_downgrade_manifest_bytes(
            &signing,
            "test",
            "windows-x64",
            "0.4.13",
            "0.2.0",
            vec![],
        );

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = serve_sequential_responses(
            listener,
            vec![
                ("/channels/test/windows-x64/history.json", history_bytes),
                (
                    "/channels/test/windows-x64/manifests/0.4.13.json",
                    manifest_bytes,
                ),
            ],
        );

        // An empty root has no game JAR, Java, or plugin, so it can never be
        // reported ready regardless of what the host running the tests has
        // installed.
        let root = tempfile::tempdir().unwrap();
        let result = downgrade_to_version_from(
            &format!("http://{address}"),
            &STANDARD.encode(signing.verifying_key().to_bytes()),
            "test",
            "windows-x64",
            root.path(),
            "0.4.13",
            "0.2.11",
            |_| {},
        );
        server.join().unwrap();

        assert!(matches!(
            result,
            Err(UpdateError::DowngradeInstallationNotReady)
        ));
    }

    #[test]
    #[cfg(unix)]
    fn downgrade_to_version_from_rejects_a_target_without_the_game_jar() {
        let signing = SigningKey::from_bytes(&[47_u8; 32]);
        let history_bytes = signed_history_bytes(
            &signing,
            "test",
            "windows-x64",
            &[
                ("0.4.14", "2026-08-01T00:00:00Z"),
                ("0.4.13", "2026-07-01T00:00:00Z"),
            ],
        );
        let manifest_bytes = signed_downgrade_manifest_bytes(
            &signing,
            "test",
            "windows-x64",
            "0.4.13",
            "0.2.0",
            vec![json!({
                "path": "runtime/bin/java.exe",
                "sha256": format!("{:x}", Sha256::digest(b"java")),
                "size": 4,
                "executable": true
            })],
        );

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = serve_sequential_responses(
            listener,
            vec![
                ("/channels/test/windows-x64/history.json", history_bytes),
                (
                    "/channels/test/windows-x64/manifests/0.4.13.json",
                    manifest_bytes,
                ),
            ],
        );

        let root = tempfile::tempdir().unwrap();
        make_ready_installation(root.path(), b"old-jar-body");
        let result = downgrade_to_version_from(
            &format!("http://{address}"),
            &STANDARD.encode(signing.verifying_key().to_bytes()),
            "test",
            "windows-x64",
            root.path(),
            "0.4.13",
            "0.2.11",
            |_| {},
        );
        server.join().unwrap();

        assert!(matches!(result, Err(UpdateError::DowngradeArtifactMissing)));
    }

    #[test]
    #[cfg(unix)]
    fn downgrade_to_version_from_replaces_only_the_game_jar() {
        let signing = SigningKey::from_bytes(&[48_u8; 32]);
        let history_bytes = signed_history_bytes(
            &signing,
            "test",
            "windows-x64",
            &[
                ("0.4.14", "2026-08-01T00:00:00Z"),
                ("0.4.13", "2026-07-01T00:00:00Z"),
            ],
        );
        let new_jar_bytes = b"new-jar-body".to_vec();
        let manifest_bytes = signed_downgrade_manifest_bytes(
            &signing,
            "test",
            "windows-x64",
            "0.4.13",
            "0.2.0",
            vec![json!({
                "path": CANONICAL_GAME_JAR,
                "sha256": format!("{:x}", Sha256::digest(&new_jar_bytes)),
                "size": new_jar_bytes.len(),
                "executable": false
            })],
        );

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = serve_sequential_responses(
            listener,
            vec![
                ("/channels/test/windows-x64/history.json", history_bytes),
                (
                    "/channels/test/windows-x64/manifests/0.4.13.json",
                    manifest_bytes,
                ),
                (
                    "/channels/test/windows-x64/releases/0.4.13/Arena-oraja.jar",
                    new_jar_bytes.clone(),
                ),
            ],
        );

        let root = tempfile::tempdir().unwrap();
        make_ready_installation(root.path(), b"old-jar-body");
        let sentinels = [
            ("config_player.json", b"settings".as_slice()),
            ("skin/custom/theme.json", b"skin".as_slice()),
            ("replay/sample.brp", b"replay".as_slice()),
            ("player/default/score.db", b"score-db".as_slice()),
        ];
        for (path, bytes) in sentinels {
            let destination = root.path().join(path);
            fs::create_dir_all(destination.parent().unwrap()).unwrap();
            fs::write(destination, bytes).unwrap();
        }
        let plugin_before = fs::read(root.path().join("ir/bms_ir_arena_oraja_0.0.69.jar")).unwrap();
        let java_before = fs::read(root.path().join("runtime/bin/java")).unwrap();

        let manifest = downgrade_to_version_from(
            &format!("http://{address}"),
            &STANDARD.encode(signing.verifying_key().to_bytes()),
            "test",
            "windows-x64",
            root.path(),
            "0.4.13",
            "0.2.11",
            |_| {},
        )
        .unwrap();
        server.join().unwrap();

        assert_eq!(manifest.version, "0.4.13");
        assert_eq!(
            fs::read(root.path().join(CANONICAL_GAME_JAR)).unwrap(),
            new_jar_bytes
        );
        assert_eq!(
            fs::read(root.path().join("ir/bms_ir_arena_oraja_0.0.69.jar")).unwrap(),
            plugin_before
        );
        assert_eq!(
            fs::read(root.path().join("runtime/bin/java")).unwrap(),
            java_before
        );
        for (path, bytes) in sentinels {
            assert_eq!(fs::read(root.path().join(path)).unwrap(), bytes);
        }
        assert!(!root
            .path()
            .join(STAGING_DIRECTORY)
            .join("downgrade.jar.previous")
            .exists());
    }

    #[test]
    fn fetch_version_notes_from_returns_that_versions_own_localized_notes() {
        let signing = SigningKey::from_bytes(&[52_u8; 32]);
        let mut manifest = json!({
            "schema_version": 1,
            "channel": "test",
            "platform": "windows-x64",
            "version": "0.4.14.18",
            "published_at": "2026-08-01T00:00:00Z",
            "release_notes_markdown": "",
            "release_notes_markdown_ja": "## 修正\n- 旧バージョンの説明",
            "release_notes_markdown_en": "## Fixes\n- Notes for the old version",
            "mandatory": false,
            "minimum_launcher_version": "0.2.0",
            "revoked_versions": [],
            "artifacts": [],
        });
        let signature = signing.sign(&serde_jcs::to_vec(&manifest).unwrap());
        manifest["signature"] = Value::String(STANDARD.encode(signature.to_bytes()));
        let manifest_bytes = serde_json::to_vec(&manifest).unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = serve_sequential_responses(
            listener,
            vec![(
                "/channels/test/windows-x64/manifests/0.4.14.18.json",
                manifest_bytes,
            )],
        );

        let notes = fetch_version_notes_from(
            &format!("http://{address}"),
            &STANDARD.encode(signing.verifying_key().to_bytes()),
            "test",
            "windows-x64",
            "0.4.14.18",
        )
        .unwrap();
        server.join().unwrap();

        assert_eq!(
            notes.release_notes_markdown_ja,
            "## 修正\n- 旧バージョンの説明"
        );
        assert_eq!(
            notes.release_notes_markdown_en,
            "## Fixes\n- Notes for the old version"
        );
    }

    #[test]
    fn game_jar_swap_restores_the_previous_jar_after_replacement_failure() {
        let root = tempfile::tempdir().unwrap();
        let staging_directory = root.path().join(STAGING_DIRECTORY);
        fs::create_dir_all(&staging_directory).unwrap();
        let staging = staging_directory.join("downgrade.jar");
        let destination = root.path().join(CANONICAL_GAME_JAR);
        let backup = staging_directory.join("downgrade.jar.previous");
        fs::write(&destination, b"old-jar-body").unwrap();
        fs::write(&staging, b"new-jar-body").unwrap();

        let mut rename_count = 0;
        let result = replace_game_jar_with_rollback(
            &staging,
            &destination,
            &backup,
            |from, to| {
                rename_count += 1;
                if rename_count == 2 {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "injected replacement failure",
                    ));
                }
                fs::rename(from, to)
            },
            |path| fs::remove_file(path),
        );

        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(fs::read(&destination).unwrap(), b"old-jar-body");
        assert_eq!(fs::read(&staging).unwrap(), b"new-jar-body");
        assert!(!backup.exists());
        assert_eq!(rename_count, 3);
    }

    #[test]
    fn plugin_release_requires_exactly_one_direct_ir_plugin_jar() {
        let artifact = |path: &str| ReleaseArtifact {
            path: path.into(),
            sha256: "00".repeat(32),
            size: 1,
            executable: false,
        };
        let manifest = ReleaseManifest {
            schema_version: 1,
            channel: "test".into(),
            platform: "windows-x64".into(),
            version: "0.4.14".into(),
            published_at: "2026-08-09T00:00:00Z".into(),
            release_notes_markdown: String::new(),
            release_notes_markdown_ja: String::new(),
            release_notes_markdown_en: String::new(),
            announcements: vec![],
            mandatory: false,
            minimum_launcher_version: "0.2.0".into(),
            launcher_version: String::new(),
            revoked_versions: vec![],
            bootstrap: None,
            artifacts: vec![artifact("ir/bms_ir_arena.jar")],
            signature: String::new(),
        };
        assert_eq!(
            release_plugin_artifact(&manifest).unwrap().path,
            "ir/bms_ir_arena.jar"
        );
        let mut invalid = manifest.clone();
        invalid.artifacts = vec![artifact("plugins/bms_ir_arena.jar")];
        assert!(release_plugin_artifact(&invalid).is_none());
        let mut ambiguous = manifest;
        ambiguous.artifacts.push(artifact("ir/bms_ir_second.jar"));
        assert!(release_plugin_artifact(&ambiguous).is_none());
    }
}

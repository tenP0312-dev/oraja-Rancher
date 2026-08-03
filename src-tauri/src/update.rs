use crate::manifest::{verify_file, verify_manifest, ReleaseManifest};
use serde::Serialize;
use std::cmp::Ordering;
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
    #[error("manifest channel or platform does not match this launcher")]
    WrongTarget,
    #[error("update staging path is unsafe")]
    UnsafeStaging,
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
}

#[derive(Debug, Clone)]
pub struct PreparedUpdate {
    pub manifest: ReleaseManifest,
    pub staging: PathBuf,
    pub manifest_path: PathBuf,
}

pub fn channel() -> String {
    let executable = std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.file_stem()
                .map(|value| value.to_string_lossy().to_string())
        })
        .unwrap_or_default();
    channel_from_name(&executable).to_string()
}

fn channel_from_name(name: &str) -> &'static str {
    if name.to_ascii_lowercase().trim_end().ends_with(" test") {
        "test"
    } else {
        "stable"
    }
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

fn fetch_bytes(url: &Url, maximum: u64) -> Result<Vec<u8>, UpdateError> {
    let response = agent()
        .get(url.as_str())
        .call()
        .map_err(|error| UpdateError::Request(error.to_string()))?;
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
    if selected_platform == "unsupported" {
        return Err(UpdateError::WrongTarget);
    }
    let url = append_url(
        update_base_url()?,
        &[
            "channels",
            &selected_channel,
            selected_platform,
            "manifest.json",
        ],
    )?;
    let bytes = fetch_bytes(&url, MAX_MANIFEST_BYTES)?;
    let input = String::from_utf8(bytes).map_err(|_| UpdateError::Incomplete)?;
    let release = verify_manifest(&input, release_public_key()?)?;
    if release.channel != selected_channel || release.platform != selected_platform {
        return Err(UpdateError::WrongTarget);
    }
    Ok((input, release))
}

pub fn check(root: &Path) -> Result<UpdateInfo, UpdateError> {
    let installed = installed_version(root);
    let (_input, release) = fetch_release()?;
    let revoked = release
        .revoked_versions
        .iter()
        .any(|value| value == &installed);
    let launcher_old =
        compare_versions(env!("CARGO_PKG_VERSION"), &release.minimum_launcher_version)
            == Ordering::Less;
    let available = compare_versions(&installed, &release.version) == Ordering::Less;
    let status = if revoked {
        "revoked"
    } else if launcher_old {
        "launcher_too_old"
    } else if available {
        "available"
    } else {
        "current"
    };
    Ok(UpdateInfo {
        channel: release.channel.clone(),
        platform: release.platform.clone(),
        installed_version: installed,
        available_version: release.version.clone(),
        status: status.to_string(),
        mandatory: release.mandatory || revoked || launcher_old,
        release_notes_markdown: release.release_notes_markdown.clone(),
    })
}

pub fn prepare(root: &Path) -> Result<PreparedUpdate, UpdateError> {
    let root = root.canonicalize()?;
    let (manifest_json, release) = fetch_release()?;
    let installed = installed_version(&root);
    if compare_versions(&installed, &release.version) != Ordering::Less
        && !release
            .revoked_versions
            .iter()
            .any(|value| value == &installed)
    {
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

    for artifact in &release.artifacts {
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
        let url = append_url(update_base_url()?, &url_segments)?;
        let response = agent()
            .get(url.as_str())
            .call()
            .map_err(|error| UpdateError::Request(error.to_string()))?;
        let mut reader = response.into_reader().take(artifact.size + 1);
        let mut target = fs::File::create(&destination)?;
        let copied = io::copy(&mut reader, &mut target)?;
        target.flush()?;
        if copied != artifact.size {
            return Err(if copied > artifact.size {
                UpdateError::TooLarge
            } else {
                UpdateError::Incomplete
            });
        }
        verify_file(&destination, artifact)?;
    }
    let manifest_path = staging.join(STAGED_MANIFEST);
    fs::write(&manifest_path, manifest_json)?;
    Ok(PreparedUpdate {
        manifest: release,
        staging,
        manifest_path,
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

    #[test]
    fn executable_name_selects_test_channel_only_when_explicit() {
        assert_eq!(channel_from_name("BMS-IR Arena"), "stable");
        assert_eq!(channel_from_name("BMS-IR Arena Test"), "test");
        assert_eq!(channel_from_name("contest"), "stable");
    }

    #[test]
    fn versions_order_release_after_prerelease() {
        assert_eq!(compare_versions("0.4.14", "0.4.13"), Ordering::Greater);
        assert_eq!(compare_versions("0.4.14-test", "0.4.14"), Ordering::Less);
        assert_eq!(compare_versions("0.4.14", "0.4.14.0"), Ordering::Equal);
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

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use thiserror::Error;
use url::Url;

const MAX_RELEASE_NOTES_BYTES: usize = 64 * 1024;
const MAX_ANNOUNCEMENTS: usize = 20;
const MAX_ANNOUNCEMENT_TITLE: usize = 200;
const MAX_BOOTSTRAP_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_ARTIFACT_LOCATIONS: usize = 10_000;
const PLUGIN_MANDATORY_MINIMUM_LAUNCHER_VERSION: &str = "0.2.27";
pub const ARTIFACT_LOCATIONS_NAME: &str = "artifact-locations.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseArtifact {
    pub path: String,
    pub sha256: String,
    pub size: u64,
    #[serde(default)]
    pub executable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseAnnouncement {
    pub date: String,
    pub title_ja: String,
    pub title_en: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseBootstrap {
    pub url: String,
    pub sha256: String,
    pub size: u64,
    pub artifacts: Vec<ReleaseArtifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseManifest {
    pub schema_version: u32,
    pub channel: String,
    #[serde(default)]
    pub platform: String,
    pub version: String,
    pub published_at: String,
    #[serde(default)]
    pub release_notes_markdown: String,
    #[serde(default)]
    pub release_notes_markdown_ja: String,
    #[serde(default)]
    pub release_notes_markdown_en: String,
    #[serde(default)]
    pub announcements: Vec<ReleaseAnnouncement>,
    #[serde(default)]
    pub mandatory: bool,
    #[serde(default)]
    pub plugin_mandatory: bool,
    #[serde(default)]
    pub minimum_launcher_version: String,
    #[serde(default)]
    pub launcher_version: String,
    #[serde(default)]
    pub revoked_versions: Vec<String>,
    #[serde(default)]
    pub bootstrap: Option<ReleaseBootstrap>,
    pub artifacts: Vec<ReleaseArtifact>,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HistoryEntry {
    pub version: String,
    pub published_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LatestLauncherReference {
    pub release_version: String,
    pub launcher_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactLocationsReference {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseHistory {
    pub schema_version: u32,
    pub channel: String,
    pub platform: String,
    pub versions: Vec<HistoryEntry>,
    #[serde(default)]
    pub latest_launcher: Option<LatestLauncherReference>,
    #[serde(default)]
    pub artifact_locations: Option<ArtifactLocationsReference>,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactLocation {
    pub version: String,
    pub path: String,
    pub sha256: String,
    pub size: u64,
    pub url: String,
    pub retain_on_pages: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactLocations {
    pub schema_version: u32,
    pub channel: String,
    pub platform: String,
    pub locations: Vec<ArtifactLocation>,
    pub signature: String,
}

const MAX_HISTORY_VERSIONS: usize = 2000;

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("invalid manifest JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("manifest schema is unsupported")]
    Schema,
    #[error("manifest signature or public key is malformed")]
    SignatureEncoding,
    #[error("manifest signature verification failed")]
    Signature,
    #[error("artifact path is unsafe: {0}")]
    UnsafePath(String),
    #[error("artifact SHA-256 is malformed")]
    InvalidDigest,
    #[error("artifact list contains duplicate path: {0}")]
    DuplicatePath(String),
    #[error("artifact hash mismatch: {0}")]
    HashMismatch(String),
    #[error("history channel or platform does not match the request")]
    HistoryTarget,
    #[error("history does not list the requested version")]
    HistoryVersionMissing,
    #[error("artifact-location channel or platform does not match the request")]
    ArtifactLocationTarget,
    #[error("artifact location does not match the signed manifest: {0}/{1}")]
    ArtifactLocationMismatch(String, String),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

pub fn verify_manifest(
    input: &str,
    public_key_base64: &str,
) -> Result<ReleaseManifest, ManifestError> {
    let manifest: ReleaseManifest = serde_json::from_str(input)?;
    if manifest.schema_version != 1 {
        return Err(ManifestError::Schema);
    }
    if !matches!(manifest.channel.as_str(), "stable" | "test")
        || !matches!(manifest.platform.as_str(), "windows-x64" | "macos-arm64")
        || manifest.version.trim().is_empty()
        || manifest.published_at.trim().is_empty()
    {
        return Err(ManifestError::Schema);
    }
    validate_artifacts(&manifest.artifacts)?;
    if manifest.plugin_mandatory {
        let plugin_count = manifest
            .artifacts
            .iter()
            .filter(|artifact| is_bmsir_plugin_artifact(&artifact.path))
            .count();
        if plugin_count != 1
            || !valid_version(&manifest.minimum_launcher_version)
            || version_is_less(
                &manifest.minimum_launcher_version,
                PLUGIN_MANDATORY_MINIMUM_LAUNCHER_VERSION,
            )
        {
            return Err(ManifestError::Schema);
        }
    }
    if !manifest.launcher_version.is_empty()
        && (!valid_version(&manifest.launcher_version) || !contains_platform_launcher(&manifest))
    {
        return Err(ManifestError::Schema);
    }
    if let Some(bootstrap) = &manifest.bootstrap {
        let url = Url::parse(&bootstrap.url).map_err(|_| ManifestError::Schema)?;
        if url.scheme() != "https"
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || bootstrap.size == 0
            || bootstrap.size > MAX_BOOTSTRAP_BYTES
            || bootstrap.sha256.len() != 64
            || !bootstrap
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            || bootstrap.artifacts.is_empty()
        {
            return Err(ManifestError::Schema);
        }
        validate_artifacts(&bootstrap.artifacts)?;
    }
    validate_localized_content(&manifest)?;

    verify_canonical_signature(input, public_key_base64, &manifest.signature)?;
    Ok(manifest)
}

fn valid_version(value: &str) -> bool {
    let (main, suffix) = value.split_once('-').unwrap_or((value, ""));
    !main.is_empty()
        && main.split('.').count() >= 2
        && main
            .split('.')
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
        && (suffix.is_empty()
            || (suffix
                .bytes()
                .next()
                .is_some_and(|byte| byte.is_ascii_alphanumeric())
                && suffix
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))))
}

fn is_bmsir_plugin_artifact(value: &str) -> bool {
    let path = Path::new(value);
    path.parent()
        .and_then(Path::to_str)
        .is_some_and(|parent| parent.eq_ignore_ascii_case("ir"))
        && path
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|name| {
                name.to_ascii_lowercase().starts_with("bms_ir")
                    && name.to_ascii_lowercase().ends_with(".jar")
            })
}

fn version_is_less(left: &str, right: &str) -> bool {
    let parse = |value: &str| {
        value
            .split_once('-')
            .map_or(value, |(main, _)| main)
            .split('.')
            .map(|part| part.parse::<u64>().unwrap_or(0))
            .collect::<Vec<_>>()
    };
    let left = parse(left);
    let right = parse(right);
    let length = left.len().max(right.len());
    (0..length).find_map(|index| {
        let ordering = left
            .get(index)
            .copied()
            .unwrap_or(0)
            .cmp(&right.get(index).copied().unwrap_or(0));
        (ordering != std::cmp::Ordering::Equal).then_some(ordering)
    }) == Some(std::cmp::Ordering::Less)
}

fn contains_platform_launcher(manifest: &ReleaseManifest) -> bool {
    let expected = match (manifest.platform.as_str(), manifest.channel.as_str()) {
        ("windows-x64", "stable") => "bms-ir arena.exe",
        ("windows-x64", "test") => "bms-ir arena test.exe",
        ("macos-arm64", "stable") => "bms-ir arena.app/contents/macos/bmsir-arena-launcher",
        ("macos-arm64", "test") => "bms-ir arena test.app/contents/macos/bmsir-arena-launcher",
        _ => return false,
    };
    manifest.artifacts.iter().any(|artifact| {
        let path = artifact.path.to_ascii_lowercase();
        match manifest.platform.as_str() {
            "windows-x64" => !path.contains('/') && path == expected,
            "macos-arm64" => path == expected,
            _ => false,
        }
    })
}

/// Verifies an Ed25519 signature over the RFC 8785 canonical form of `input`
/// with its `signature` field removed. Shared by manifests and the history
/// index, which use the exact same signing scheme.
fn verify_canonical_signature(
    input: &str,
    public_key_base64: &str,
    signature_base64: &str,
) -> Result<(), ManifestError> {
    let mut unsigned: Value = serde_json::from_str(input)?;
    unsigned
        .as_object_mut()
        .ok_or(ManifestError::Schema)?
        .remove("signature");
    let canonical = serde_jcs::to_vec(&unsigned).map_err(|_| ManifestError::SignatureEncoding)?;
    let key_bytes = STANDARD
        .decode(public_key_base64)
        .map_err(|_| ManifestError::SignatureEncoding)?;
    let signature_bytes = STANDARD
        .decode(signature_base64)
        .map_err(|_| ManifestError::SignatureEncoding)?;
    let key_array: [u8; 32] = key_bytes
        .try_into()
        .map_err(|_| ManifestError::SignatureEncoding)?;
    let verifying_key =
        VerifyingKey::from_bytes(&key_array).map_err(|_| ManifestError::SignatureEncoding)?;
    let signature =
        Signature::from_slice(&signature_bytes).map_err(|_| ManifestError::SignatureEncoding)?;
    verifying_key
        .verify(&canonical, &signature)
        .map_err(|_| ManifestError::Signature)?;
    Ok(())
}

/// Verifies a signed `history.json` index for the given channel/platform and
/// returns it. Every entry must have a non-empty version and published_at,
/// versions must be unique (case-insensitive), and the list is capped to
/// guard against a pathologically large response.
pub fn verify_history(
    input: &str,
    public_key_base64: &str,
    expected_channel: &str,
    expected_platform: &str,
) -> Result<ReleaseHistory, ManifestError> {
    let history: ReleaseHistory = serde_json::from_str(input)?;
    if history.schema_version != 1 {
        return Err(ManifestError::Schema);
    }
    if !matches!(history.channel.as_str(), "stable" | "test")
        || !matches!(history.platform.as_str(), "windows-x64" | "macos-arm64")
        || history.versions.is_empty()
        || history.versions.len() > MAX_HISTORY_VERSIONS
    {
        return Err(ManifestError::Schema);
    }
    if history.channel != expected_channel || history.platform != expected_platform {
        return Err(ManifestError::HistoryTarget);
    }
    let mut seen = std::collections::HashSet::with_capacity(history.versions.len());
    for entry in &history.versions {
        if entry.version.trim().is_empty() || entry.published_at.trim().is_empty() {
            return Err(ManifestError::Schema);
        }
        if !seen.insert(entry.version.to_ascii_lowercase()) {
            return Err(ManifestError::Schema);
        }
    }
    if let Some(latest) = &history.latest_launcher {
        if !valid_version(&latest.release_version)
            || !valid_version(&latest.launcher_version)
            || !seen.contains(&latest.release_version.to_ascii_lowercase())
        {
            return Err(ManifestError::Schema);
        }
    }
    if history
        .artifact_locations
        .as_ref()
        .is_some_and(|reference| reference.path != ARTIFACT_LOCATIONS_NAME)
    {
        return Err(ManifestError::Schema);
    }
    verify_canonical_signature(input, public_key_base64, &history.signature)?;
    Ok(history)
}

fn valid_github_release_url(value: &str) -> bool {
    let Ok(url) = Url::parse(value) else {
        return false;
    };
    if url.scheme() != "https"
        || url.host_str() != Some("github.com")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !matches!(url.port(), None | Some(443))
    {
        return false;
    }
    let Some(segments) = url.path_segments() else {
        return false;
    };
    let segments = segments.collect::<Vec<_>>();
    if segments.len() != 6
        || segments[2] != "releases"
        || segments[3] != "download"
        || segments.iter().any(|segment| {
            segment.is_empty() || {
                let folded = segment.to_ascii_lowercase();
                folded.contains("%2f") || folded.contains("%5c")
            }
        })
    {
        return false;
    }
    segments[0]
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
        && segments[1]
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
}

pub fn verify_artifact_locations(
    input: &str,
    public_key_base64: &str,
    expected_channel: &str,
    expected_platform: &str,
) -> Result<ArtifactLocations, ManifestError> {
    let locations: ArtifactLocations = serde_json::from_str(input)?;
    if locations.schema_version != 1
        || !matches!(locations.channel.as_str(), "stable" | "test")
        || !matches!(locations.platform.as_str(), "windows-x64" | "macos-arm64")
        || locations.locations.len() > MAX_ARTIFACT_LOCATIONS
    {
        return Err(ManifestError::Schema);
    }
    if locations.channel != expected_channel || locations.platform != expected_platform {
        return Err(ManifestError::ArtifactLocationTarget);
    }
    let mut seen = std::collections::HashSet::with_capacity(locations.locations.len());
    for location in &locations.locations {
        let artifact = ReleaseArtifact {
            path: location.path.clone(),
            sha256: location.sha256.clone(),
            size: location.size,
            executable: false,
        };
        if !valid_version(&location.version)
            || !valid_github_release_url(&location.url)
            || !seen.insert((
                location.version.to_ascii_lowercase(),
                location.path.to_ascii_lowercase(),
            ))
        {
            return Err(ManifestError::Schema);
        }
        validate_artifacts(&[artifact])?;
    }
    verify_canonical_signature(input, public_key_base64, &locations.signature)?;
    Ok(locations)
}

impl ArtifactLocations {
    pub fn url_for(
        &self,
        version: &str,
        artifact: &ReleaseArtifact,
    ) -> Result<Option<Url>, ManifestError> {
        let Some(location) = self.locations.iter().find(|location| {
            location.version.eq_ignore_ascii_case(version)
                && location.path.eq_ignore_ascii_case(&artifact.path)
        }) else {
            return Ok(None);
        };
        if !location.sha256.eq_ignore_ascii_case(&artifact.sha256) || location.size != artifact.size
        {
            return Err(ManifestError::ArtifactLocationMismatch(
                version.to_string(),
                artifact.path.clone(),
            ));
        }
        Url::parse(&location.url)
            .map(Some)
            .map_err(|_| ManifestError::Schema)
    }
}

fn valid_announcement_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if !(bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit()))
    {
        return false;
    }
    let year = value[0..4].parse::<u32>().unwrap_or(0);
    let month = value[5..7].parse::<usize>().unwrap_or(0);
    let day = value[8..10].parse::<u32>().unwrap_or(0);
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    month > 0 && month <= days.len() && day > 0 && day <= days[month - 1]
}

fn valid_announcement_title(value: &str) -> bool {
    !value.trim().is_empty()
        && value.chars().count() <= MAX_ANNOUNCEMENT_TITLE
        && !value.chars().any(char::is_control)
}

fn validate_localized_content(manifest: &ReleaseManifest) -> Result<(), ManifestError> {
    for notes in [
        &manifest.release_notes_markdown,
        &manifest.release_notes_markdown_ja,
        &manifest.release_notes_markdown_en,
    ] {
        if notes.len() > MAX_RELEASE_NOTES_BYTES {
            return Err(ManifestError::Schema);
        }
    }
    if manifest.announcements.len() > MAX_ANNOUNCEMENTS
        || manifest.announcements.iter().any(|announcement| {
            !valid_announcement_date(&announcement.date)
                || !valid_announcement_title(&announcement.title_ja)
                || !valid_announcement_title(&announcement.title_en)
        })
    {
        return Err(ManifestError::Schema);
    }
    Ok(())
}

pub fn validate_artifacts(artifacts: &[ReleaseArtifact]) -> Result<(), ManifestError> {
    let mut seen = std::collections::BTreeSet::new();
    for artifact in artifacts {
        let path = Path::new(&artifact.path);
        let normalized = artifact.path.to_ascii_lowercase();
        let first = normalized.split('/').next().unwrap_or("");
        let file_name = normalized.rsplit('/').next().unwrap_or("");
        let mutable_path = matches!(first, "player" | "bms" | "replay" | ".bmsir-update-staging")
            || matches!(
                file_name,
                "config_sys.json"
                    | "config_player.json"
                    | "score.db"
                    | "songdata.db"
                    | "bmsir_maniac.db"
                    | "bmsir_arena.json"
                    | "bmsir-arena-version.txt"
                    | ".bmsir-launcher-policy.json"
                    | ".bmsir-launcher-policy.tmp"
                    | ".bmsir-launcher-settings.json"
            );
        if artifact.path.is_empty()
            || artifact.path.contains('\\')
            || artifact.path.contains(':')
            || artifact.path.chars().any(char::is_control)
            || path.is_absolute()
            || path.components().any(|part| {
                matches!(
                    part,
                    std::path::Component::ParentDir | std::path::Component::CurDir
                )
            })
            || artifact.path == ".bmsir-launcher-backup"
            || artifact.path.starts_with(".bmsir-launcher-backup/")
            || mutable_path
        {
            return Err(ManifestError::UnsafePath(artifact.path.clone()));
        }
        if !seen.insert(artifact.path.to_lowercase()) {
            return Err(ManifestError::DuplicatePath(artifact.path.clone()));
        }
        if artifact.sha256.len() != 64
            || !artifact.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(ManifestError::InvalidDigest);
        }
    }
    Ok(())
}

pub fn verify_file(path: &Path, artifact: &ReleaseArtifact) -> Result<(), ManifestError> {
    let metadata = path.metadata()?;
    if metadata.len() != artifact.size {
        return Err(ManifestError::HashMismatch(artifact.path.clone()));
    }
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    let actual = format!("{:x}", digest.finalize());
    if !actual.eq_ignore_ascii_case(&artifact.sha256) {
        return Err(ManifestError::HashMismatch(artifact.path.clone()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use serde_json::json;

    fn signed_manifest() -> (String, String) {
        let signing = SigningKey::from_bytes(&[7_u8; 32]);
        let mut value = json!({
            "schema_version": 1,
            "channel": "stable",
            "platform": "windows-x64",
            "version": "0.4.0",
            "published_at": "2026-07-31T00:00:00Z",
            "release_notes_markdown": "## Arena 0.4.0\n- safe",
            "mandatory": false,
            "minimum_launcher_version": "0.1.0",
            "revoked_versions": [],
            "bootstrap": null,
            "artifacts": [{
                "path": "BMS-IR-Arena-oraja.jar",
                "sha256": "00".repeat(32),
                "size": 0,
                "executable": false
            }]
        });
        let canonical = serde_jcs::to_vec(&value).unwrap();
        let signature = signing.sign(&canonical);
        value["signature"] = Value::String(STANDARD.encode(signature.to_bytes()));
        (
            serde_json::to_string(&value).unwrap(),
            STANDARD.encode(signing.verifying_key().to_bytes()),
        )
    }

    #[test]
    fn verifies_canonical_manifest_and_rejects_tampering() {
        let (input, key) = signed_manifest();
        let manifest = verify_manifest(&input, &key).unwrap();
        assert_eq!(manifest.version, "0.4.0");
        assert!(!manifest.plugin_mandatory);
        let tampered = input.replace("0.4.0", "9.9.9");
        assert!(matches!(
            verify_manifest(&tampered, &key),
            Err(ManifestError::Signature)
        ));
    }

    #[test]
    fn plugin_mandatory_requires_one_plugin_and_launcher_0_2_27() {
        let signing = SigningKey::from_bytes(&[12_u8; 32]);
        let plugin = json!({
            "path": "ir/bms_ir_arena_0.0.73.jar",
            "sha256": "11".repeat(32),
            "size": 1,
            "executable": false
        });
        let signed = |minimum_launcher_version: &str, artifacts: Value| {
            let mut value = json!({
                "schema_version": 1,
                "channel": "test",
                "platform": "windows-x64",
                "version": "0.4.14.54",
                "published_at": "2026-08-18T00:00:00Z",
                "plugin_mandatory": true,
                "minimum_launcher_version": minimum_launcher_version,
                "revoked_versions": [],
                "artifacts": artifacts
            });
            let signature = signing.sign(&serde_jcs::to_vec(&value).unwrap());
            value["signature"] = Value::String(STANDARD.encode(signature.to_bytes()));
            serde_json::to_string(&value).unwrap()
        };
        let key = STANDARD.encode(signing.verifying_key().to_bytes());
        let manifest = verify_manifest(&signed("0.2.27", json!([plugin.clone()])), &key).unwrap();
        assert!(manifest.plugin_mandatory);
        assert!(verify_manifest(&signed("0.2.26", json!([plugin])), &key).is_err());
        assert!(verify_manifest(&signed("0.2.27", json!([])), &key).is_err());
    }

    #[test]
    fn launcher_version_is_signed_and_requires_platform_launcher() {
        let signing = SigningKey::from_bytes(&[9_u8; 32]);
        let mut value = json!({
            "schema_version": 1,
            "channel": "test",
            "platform": "windows-x64",
            "version": "0.4.14.26",
            "launcher_version": "0.2.20",
            "published_at": "2026-08-10T00:00:00Z",
            "mandatory": false,
            "minimum_launcher_version": "0.2.17",
            "revoked_versions": [],
            "bootstrap": null,
            "artifacts": [{
                "path": "BMS-IR Arena Test.exe",
                "sha256": "00".repeat(32),
                "size": 1,
                "executable": true
            }]
        });
        let signature = signing.sign(&serde_jcs::to_vec(&value).unwrap());
        value["signature"] = Value::String(STANDARD.encode(signature.to_bytes()));
        let input = serde_json::to_string(&value).unwrap();
        let key = STANDARD.encode(signing.verifying_key().to_bytes());
        assert_eq!(
            verify_manifest(&input, &key).unwrap().launcher_version,
            "0.2.20"
        );

        value.as_object_mut().unwrap().remove("signature");
        value["artifacts"][0]["path"] = Value::String("Arena-oraja.jar".into());
        let signature = signing.sign(&serde_jcs::to_vec(&value).unwrap());
        value["signature"] = Value::String(STANDARD.encode(signature.to_bytes()));
        assert!(matches!(
            verify_manifest(&serde_json::to_string(&value).unwrap(), &key),
            Err(ManifestError::Schema)
        ));
    }

    #[test]
    fn validates_localized_notes_and_announcements() {
        let signing = SigningKey::from_bytes(&[7_u8; 32]);
        let mut value = json!({
            "schema_version": 1,
            "channel": "test",
            "platform": "windows-x64",
            "version": "0.4.14.4",
            "published_at": "2026-08-03T00:00:00Z",
            "release_notes_markdown": "legacy",
            "release_notes_markdown_ja": "## 更新",
            "release_notes_markdown_en": "## Update",
            "announcements": [{
                "date": "2026-08-03",
                "title_ja": "更新のお知らせ",
                "title_en": "Update notice"
            }],
            "mandatory": true,
            "minimum_launcher_version": "0.2.4",
            "revoked_versions": [],
            "bootstrap": null,
            "artifacts": []
        });
        let signature = signing.sign(&serde_jcs::to_vec(&value).unwrap());
        value["signature"] = Value::String(STANDARD.encode(signature.to_bytes()));
        let release = verify_manifest(
            &serde_json::to_string(&value).unwrap(),
            &STANDARD.encode(signing.verifying_key().to_bytes()),
        )
        .unwrap();
        assert_eq!(release.announcements[0].title_ja, "更新のお知らせ");

        value.as_object_mut().unwrap().remove("signature");
        value["announcements"][0]["date"] = Value::String("2026-13-40".into());
        let signature = signing.sign(&serde_jcs::to_vec(&value).unwrap());
        value["signature"] = Value::String(STANDARD.encode(signature.to_bytes()));
        assert!(matches!(
            verify_manifest(
                &serde_json::to_string(&value).unwrap(),
                &STANDARD.encode(signing.verifying_key().to_bytes()),
            ),
            Err(ManifestError::Schema)
        ));
    }

    #[test]
    fn rejects_parent_and_duplicate_artifact_paths() {
        let artifact = ReleaseArtifact {
            path: "../outside".into(),
            sha256: "00".repeat(32),
            size: 0,
            executable: false,
        };
        assert!(matches!(
            validate_artifacts(&[artifact]),
            Err(ManifestError::UnsafePath(_))
        ));
        let case_duplicates = vec![
            ReleaseArtifact {
                path: "Game.jar".into(),
                sha256: "00".repeat(32),
                size: 0,
                executable: false,
            },
            ReleaseArtifact {
                path: "game.jar".into(),
                sha256: "11".repeat(32),
                size: 0,
                executable: false,
            },
        ];
        assert!(matches!(
            validate_artifacts(&case_duplicates),
            Err(ManifestError::DuplicatePath(_))
        ));
        let mutable = ReleaseArtifact {
            path: "player/player1/score.db".into(),
            sha256: "00".repeat(32),
            size: 0,
            executable: false,
        };
        assert!(matches!(
            validate_artifacts(&[mutable]),
            Err(ManifestError::UnsafePath(_))
        ));
        let settings = ReleaseArtifact {
            path: ".bmsir-launcher-settings.json".into(),
            sha256: "00".repeat(32),
            size: 0,
            executable: false,
        };
        assert!(matches!(
            validate_artifacts(&[settings]),
            Err(ManifestError::UnsafePath(_))
        ));
    }

    #[test]
    fn verifies_python_generated_canonical_manifest() {
        let input = r###"{"artifacts":[{"executable":true,"path":"BMS-IR Arena Test.exe","sha256":"0000000000000000000000000000000000000000000000000000000000000000","size":123}],"channel":"test","mandatory":false,"minimum_launcher_version":"0.2.0","platform":"windows-x64","published_at":"2026-08-03T00:00:00Z","release_notes_markdown":"## テスト\n- portable","revoked_versions":[],"schema_version":1,"signature":"MLd6VQXHO8mRw2/ohlj0cnsFrowkLBElzOGFUiCbUvEvMefV63TkOopaXyb7nTbJQdpgCTniOhNAnK/Yj+TBBg==","version":"0.4.14"}"###;
        let key = "A6EHv/POEL4dcN0Y50vAmWfk1jCbpQ1fHdyGZBJVMbg=";
        let manifest = verify_manifest(input, key).unwrap();
        assert_eq!(manifest.version, "0.4.14");
    }

    fn signed_history(versions: &[(&str, &str)]) -> (String, String) {
        let signing = SigningKey::from_bytes(&[7_u8; 32]);
        let mut value = json!({
            "schema_version": 1,
            "channel": "test",
            "platform": "windows-x64",
            "versions": versions.iter().map(|(version, published_at)| json!({
                "version": version,
                "published_at": published_at,
            })).collect::<Vec<_>>(),
        });
        let canonical = serde_jcs::to_vec(&value).unwrap();
        let signature = signing.sign(&canonical);
        value["signature"] = Value::String(STANDARD.encode(signature.to_bytes()));
        (
            serde_json::to_string(&value).unwrap(),
            STANDARD.encode(signing.verifying_key().to_bytes()),
        )
    }

    #[test]
    fn verifies_canonical_history_and_rejects_tampering() {
        let (input, key) = signed_history(&[
            ("0.4.15", "2026-08-06T00:00:00Z"),
            ("0.4.14", "2026-08-03T00:00:00Z"),
        ]);
        let history = verify_history(&input, &key, "test", "windows-x64").unwrap();
        assert_eq!(history.versions.len(), 2);
        assert_eq!(history.versions[0].version, "0.4.15");
        assert!(history.latest_launcher.is_none());

        let tampered = input.replace("0.4.14", "9.9.9");
        assert!(matches!(
            verify_history(&tampered, &key, "test", "windows-x64"),
            Err(ManifestError::Signature)
        ));
    }

    #[test]
    fn verifies_signed_latest_launcher_reference() {
        let signing = SigningKey::from_bytes(&[8_u8; 32]);
        let mut value = json!({
            "schema_version": 1,
            "channel": "test",
            "platform": "windows-x64",
            "versions": [
                {"version": "0.4.14.44", "published_at": "2026-08-14T00:00:00Z"},
                {"version": "0.4.14.43", "published_at": "2026-08-13T00:00:00Z"}
            ],
            "latest_launcher": {
                "release_version": "0.4.14.44",
                "launcher_version": "0.2.25"
            },
            "artifact_locations": {
                "path": "artifact-locations.json"
            }
        });
        let signature = signing.sign(&serde_jcs::to_vec(&value).unwrap());
        value["signature"] = Value::String(STANDARD.encode(signature.to_bytes()));
        let key = STANDARD.encode(signing.verifying_key().to_bytes());
        let history = verify_history(
            &serde_json::to_string(&value).unwrap(),
            &key,
            "test",
            "windows-x64",
        )
        .unwrap();
        assert_eq!(history.latest_launcher.unwrap().launcher_version, "0.2.25");
        assert_eq!(
            history.artifact_locations.unwrap().path,
            ARTIFACT_LOCATIONS_NAME
        );

        value.as_object_mut().unwrap().remove("signature");
        value["latest_launcher"]["release_version"] = Value::String("0.4.14.99".into());
        let signature = signing.sign(&serde_jcs::to_vec(&value).unwrap());
        value["signature"] = Value::String(STANDARD.encode(signature.to_bytes()));
        assert!(matches!(
            verify_history(
                &serde_json::to_string(&value).unwrap(),
                &key,
                "test",
                "windows-x64"
            ),
            Err(ManifestError::Schema)
        ));
    }

    #[test]
    fn history_rejects_wrong_target_and_duplicate_versions() {
        let (input, key) = signed_history(&[("0.4.14", "2026-08-03T00:00:00Z")]);
        assert!(matches!(
            verify_history(&input, &key, "test", "macos-arm64"),
            Err(ManifestError::HistoryTarget)
        ));
        assert!(matches!(
            verify_history(&input, &key, "stable", "windows-x64"),
            Err(ManifestError::HistoryTarget)
        ));

        let (duplicate_input, duplicate_key) = signed_history(&[
            ("0.4.14", "2026-08-03T00:00:00Z"),
            ("0.4.14", "2026-08-04T00:00:00Z"),
        ]);
        assert!(matches!(
            verify_history(&duplicate_input, &duplicate_key, "test", "windows-x64"),
            Err(ManifestError::Schema)
        ));
    }

    #[test]
    fn history_rejects_empty_version_list() {
        let (input, key) = signed_history(&[]);
        assert!(matches!(
            verify_history(&input, &key, "test", "windows-x64"),
            Err(ManifestError::Schema)
        ));
    }

    #[test]
    fn verifies_signed_artifact_locations_and_rejects_mismatch() {
        let signing = SigningKey::from_bytes(&[31_u8; 32]);
        let mut value = json!({
            "schema_version": 1,
            "channel": "test",
            "platform": "windows-x64",
            "locations": [{
                "version": "0.4.14.49",
                "path": "Arena-oraja.jar",
                "sha256": "12".repeat(32),
                "size": 123,
                "url": "https://github.com/tenP0312-dev/bms-ir-arena-patch-server/releases/download/test-0.4.14.49/windows-x64-Arena-oraja.jar",
                "retain_on_pages": false
            }]
        });
        let signature = signing.sign(&serde_jcs::to_vec(&value).unwrap());
        value["signature"] = Value::String(STANDARD.encode(signature.to_bytes()));
        let input = serde_json::to_string(&value).unwrap();
        let locations = verify_artifact_locations(
            &input,
            &STANDARD.encode(signing.verifying_key().to_bytes()),
            "test",
            "windows-x64",
        )
        .unwrap();
        let artifact = ReleaseArtifact {
            path: "Arena-oraja.jar".into(),
            sha256: "12".repeat(32),
            size: 123,
            executable: false,
        };
        assert_eq!(
            locations
                .url_for("0.4.14.49", &artifact)
                .unwrap()
                .unwrap()
                .host_str(),
            Some("github.com")
        );

        let mismatched = ReleaseArtifact {
            size: 124,
            ..artifact
        };
        assert!(matches!(
            locations.url_for("0.4.14.49", &mismatched),
            Err(ManifestError::ArtifactLocationMismatch(_, _))
        ));

        value["locations"][0]["url"] = Value::String("https://example.test/not-a-release".into());
        assert!(matches!(
            verify_artifact_locations(
                &serde_json::to_string(&value).unwrap(),
                &STANDARD.encode(signing.verifying_key().to_bytes()),
                "test",
                "windows-x64",
            ),
            Err(ManifestError::Schema)
        ));
    }

    #[test]
    fn verifies_python_generated_artifact_locations() {
        let input = r###"{"channel":"test","locations":[{"path":"Arena-oraja.jar","retain_on_pages":false,"sha256":"1212121212121212121212121212121212121212121212121212121212121212","size":123,"url":"https://github.com/tenP0312-dev/bms-ir-arena-patch-server/releases/download/test-0.4.14.49/windows-x64-Arena-oraja.jar","version":"0.4.14.49"}],"platform":"windows-x64","schema_version":1,"signature":"h9A08Cv/Xi4nNoktHo0QQGvEAf7uGUY4LgaLhn+ZjmRPrBMjQTk4QGCgJ+Qbt4npY4u5L0pjJ4H6GyrEzsmvDg=="}"###;
        let key = "A6EHv/POEL4dcN0Y50vAmWfk1jCbpQ1fHdyGZBJVMbg=";
        let locations = verify_artifact_locations(input, key, "test", "windows-x64").unwrap();
        assert_eq!(locations.locations[0].version, "0.4.14.49");
    }
}

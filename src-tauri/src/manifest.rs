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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseArtifact {
    pub path: String,
    pub sha256: String,
    pub size: u64,
    #[serde(default)]
    pub executable: bool,
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
    pub mandatory: bool,
    #[serde(default)]
    pub minimum_launcher_version: String,
    #[serde(default)]
    pub revoked_versions: Vec<String>,
    pub artifacts: Vec<ReleaseArtifact>,
    pub signature: String,
}

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
        .decode(&manifest.signature)
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
    Ok(manifest)
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
        assert_eq!(verify_manifest(&input, &key).unwrap().version, "0.4.0");
        let tampered = input.replace("0.4.0", "9.9.9");
        assert!(matches!(
            verify_manifest(&tampered, &key),
            Err(ManifestError::Signature)
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
    }

    #[test]
    fn verifies_python_generated_canonical_manifest() {
        let input = r###"{"artifacts":[{"executable":true,"path":"BMS-IR Arena Test.exe","sha256":"0000000000000000000000000000000000000000000000000000000000000000","size":123}],"channel":"test","mandatory":false,"minimum_launcher_version":"0.2.0","platform":"windows-x64","published_at":"2026-08-03T00:00:00Z","release_notes_markdown":"## テスト\n- portable","revoked_versions":[],"schema_version":1,"signature":"MLd6VQXHO8mRw2/ohlj0cnsFrowkLBElzOGFUiCbUvEvMefV63TkOopaXyb7nTbJQdpgCTniOhNAnK/Yj+TBBg==","version":"0.4.14"}"###;
        let key = "A6EHv/POEL4dcN0Y50vAmWfk1jCbpQ1fHdyGZBJVMbg=";
        let manifest = verify_manifest(input, key).unwrap();
        assert_eq!(manifest.version, "0.4.14");
    }
}

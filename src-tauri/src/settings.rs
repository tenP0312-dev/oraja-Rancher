use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const SETTINGS_FILE: &str = ".bmsir-launcher-settings.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LauncherSettings {
    pub resident: bool,
    pub autostart: bool,
    pub background_check: bool,
    pub last_background_check_millis: u64,
    pub last_notified_version: String,
}

impl Default for LauncherSettings {
    fn default() -> Self {
        Self {
            resident: false,
            autostart: false,
            background_check: false,
            last_background_check_millis: 0,
            last_notified_version: String::new(),
        }
    }
}

fn settings_path(root: &Path) -> PathBuf {
    root.join(SETTINGS_FILE)
}

pub fn load_settings(root: &Path) -> LauncherSettings {
    let Ok(text) = fs::read_to_string(settings_path(root)) else {
        return LauncherSettings::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

pub fn save_settings(root: &Path, settings: &LauncherSettings) -> Result<(), std::io::Error> {
    let path = settings_path(root);
    let temporary = path.with_extension("json.tmp");
    let text = serde_json::to_string_pretty(settings)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    match temporary.symlink_metadata() {
        Ok(metadata) if metadata.is_file() || metadata.file_type().is_symlink() => {
            fs::remove_file(&temporary)?;
        }
        Ok(_) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "launcher settings temporary path is not a file",
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    fs::write(&temporary, text)?;
    if path.exists() {
        fs::remove_file(&path)?;
    }
    fs::rename(temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_settings_file_loads_opt_in_defaults() {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(load_settings(root.path()), LauncherSettings::default());
    }

    #[test]
    fn settings_round_trip_through_atomic_save() {
        let root = tempfile::tempdir().unwrap();
        let settings = LauncherSettings {
            resident: true,
            autostart: true,
            background_check: true,
            last_background_check_millis: 1_700_000_000_000,
            last_notified_version: "0.4.14.25/0.2.20".to_string(),
        };
        save_settings(root.path(), &settings).unwrap();
        assert_eq!(load_settings(root.path()), settings);
    }

    #[test]
    fn corrupt_or_partial_settings_are_safe() {
        let root = tempfile::tempdir().unwrap();
        fs::write(settings_path(root.path()), "{ not json").unwrap();
        assert_eq!(load_settings(root.path()), LauncherSettings::default());
        fs::write(settings_path(root.path()), r#"{"resident": true}"#).unwrap();
        assert!(load_settings(root.path()).resident);
    }
}

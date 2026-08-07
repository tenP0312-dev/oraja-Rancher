use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const SETTINGS_FILE: &str = ".bmsir-launcher-settings.json";

/// Persisted, opt-in launcher preferences. Every field defaults to `false`/
/// empty so a fresh install never starts resident, autostarting, or
/// background-checking without the operator explicitly turning it on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LauncherSettings {
    pub resident: bool,
    pub autostart: bool,
    pub background_check: bool,
    /// Unix millis of the last completed background check, or 0 if none.
    pub last_background_check_millis: u64,
    /// The available_version last surfaced as a desktop notification, so a
    /// still-available release is not re-notified on every daily check.
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

/// Loads persisted settings, falling back to defaults if the file is
/// missing or unreadable/corrupt. Never fails the caller: a broken
/// preferences file should not block using the launcher.
pub fn load_settings(root: &Path) -> LauncherSettings {
    let path = settings_path(root);
    let Ok(text) = fs::read_to_string(&path) else {
        return LauncherSettings::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

pub fn save_settings(root: &Path, settings: &LauncherSettings) -> Result<(), std::io::Error> {
    let path = settings_path(root);
    let text = serde_json::to_string_pretty(settings)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    fs::write(path, text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_settings_file_loads_all_opt_in_defaults() {
        let root = tempfile::tempdir().unwrap();
        let settings = load_settings(root.path());
        assert_eq!(settings, LauncherSettings::default());
        assert!(!settings.resident);
        assert!(!settings.autostart);
        assert!(!settings.background_check);
    }

    #[test]
    fn settings_round_trip_through_save_and_load() {
        let root = tempfile::tempdir().unwrap();
        let settings = LauncherSettings {
            resident: true,
            autostart: true,
            background_check: true,
            last_background_check_millis: 1_700_000_000_000,
            last_notified_version: "0.4.14.21".to_string(),
        };
        save_settings(root.path(), &settings).unwrap();
        assert_eq!(load_settings(root.path()), settings);
    }

    #[test]
    fn corrupt_settings_file_falls_back_to_defaults_instead_of_failing() {
        let root = tempfile::tempdir().unwrap();
        fs::write(settings_path(root.path()), "{ not json").unwrap();
        assert_eq!(load_settings(root.path()), LauncherSettings::default());
    }

    #[test]
    fn partial_settings_file_fills_missing_fields_with_defaults() {
        let root = tempfile::tempdir().unwrap();
        fs::write(settings_path(root.path()), r#"{"resident": true}"#).unwrap();
        let settings = load_settings(root.path());
        assert!(settings.resident);
        assert!(!settings.autostart);
        assert!(!settings.background_check);
    }
}

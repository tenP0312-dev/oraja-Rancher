mod ini;
mod install;
mod manifest;

use install::InstallationInfo;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::Path;
use tauri::AppHandle;

#[tauri::command]
fn inspect_installation(path: String) -> Result<InstallationInfo, String> {
    install::inspect(Path::new(&path)).map_err(|error| error.to_string())
}

#[tauri::command]
fn inspect_java(path: String) -> Result<u32, String> {
    install::inspect_java(Path::new(&path)).map_err(|error| error.to_string())
}

#[tauri::command]
fn update_ini(path: String, updates: BTreeMap<String, String>) -> Result<(), String> {
    let source = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let updated = ini::update_preserving_layout(&source, &updates);
    let target = Path::new(&path);
    let temporary = target.with_extension("bmsir-new");
    fs::write(&temporary, updated).map_err(|error| error.to_string())?;
    fs::rename(temporary, target).map_err(|error| error.to_string())
}

#[tauri::command]
fn apply_offline_update(
    root: String,
    staging: String,
    manifest_path: String,
) -> Result<manifest::ReleaseManifest, String> {
    let release = load_verified_manifest(&manifest_path)?;
    install::apply_staged(Path::new(&root), Path::new(&staging), &release)
        .map_err(|error| error.to_string())?;
    Ok(release)
}

#[tauri::command]
fn inspect_update_manifest(manifest_path: String) -> Result<manifest::ReleaseManifest, String> {
    load_verified_manifest(&manifest_path)
}

#[tauri::command]
fn begin_self_update(app: AppHandle, staging: String, manifest_path: String) -> Result<(), String> {
    let release = load_verified_manifest(&manifest_path)?;
    install::spawn_self_update(Path::new(&staging), Path::new(&manifest_path), &release)
        .map_err(|error| error.to_string())?;
    app.exit(0);
    Ok(())
}

#[tauri::command]
fn launch_game(
    root: String,
    java: String,
    game_jar: String,
    configuration: bool,
) -> Result<(), String> {
    install::launch(
        Path::new(&root),
        Path::new(&java),
        Path::new(&game_jar),
        configuration,
    )
    .map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            inspect_installation,
            inspect_java,
            update_ini,
            inspect_update_manifest,
            apply_offline_update,
            begin_self_update,
            launch_game
        ])
        .plugin(tauri_plugin_dialog::init())
        .run(tauri::generate_context!())
        .expect("BMS-IR Arena Launcher failed");
}

fn release_public_key() -> Result<&'static str, String> {
    option_env!("BMSIR_ARENA_RELEASE_PUBLIC_KEY")
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "release verification key is not configured".to_string())
}

fn load_verified_manifest(manifest_path: &str) -> Result<manifest::ReleaseManifest, String> {
    let manifest_json = fs::read_to_string(manifest_path).map_err(|error| error.to_string())?;
    manifest::verify_manifest(&manifest_json, release_public_key()?)
        .map_err(|error| error.to_string())
}

pub fn run_self_update_helper_if_requested() -> bool {
    let mut arguments = env::args_os();
    let _program = arguments.next();
    if arguments.next().as_deref() != Some(std::ffi::OsStr::new("--apply-self-update")) {
        return false;
    }
    let result = (|| -> Result<(), String> {
        let root = arguments
            .next()
            .ok_or_else(|| "self-update root is missing".to_string())?;
        let staging = arguments
            .next()
            .ok_or_else(|| "self-update staging path is missing".to_string())?;
        let manifest_path = arguments
            .next()
            .ok_or_else(|| "self-update manifest path is missing".to_string())?;
        let launcher_path = arguments
            .next()
            .ok_or_else(|| "self-update launcher path is missing".to_string())?;
        if arguments.next().is_some() {
            return Err("unexpected self-update arguments".to_string());
        }
        let manifest_path_text = manifest_path.to_string_lossy();
        let release = load_verified_manifest(&manifest_path_text)?;
        let launcher_path_text = launcher_path.to_string_lossy();
        if !release
            .artifacts
            .iter()
            .any(|artifact| artifact.path.eq_ignore_ascii_case(&launcher_path_text))
        {
            return Err("self-update launcher is not in the signed manifest".to_string());
        }
        install::run_self_update_helper(
            Path::new(&root),
            Path::new(&staging),
            &release,
            &launcher_path_text,
        )
        .map_err(|error| error.to_string())
    })();
    if let Err(error) = result {
        eprintln!("BMS-IR Arena Launcher self-update failed: {error}");
    }
    true
}

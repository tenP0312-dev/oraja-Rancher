mod install;
mod manifest;
mod update;

use install::InstallationInfo;
use serde::Serialize;
use std::env;
use std::fs;
use std::path::Path;
use tauri::AppHandle;

#[derive(Debug, Serialize)]
struct LauncherState {
    installation: InstallationInfo,
    installation_ready: bool,
    update_configuration: &'static str,
    channel: String,
    platform: String,
    installed_version: String,
    launcher_version: String,
}

#[tauri::command]
fn launcher_state() -> Result<LauncherState, String> {
    let root = install::launcher_install_root().map_err(|error| error.to_string())?;
    let installation = install::inspect(&root).map_err(|error| error.to_string())?;
    let installation_ready = install::is_ready(&installation);
    Ok(LauncherState {
        channel: update::channel(),
        platform: update::platform().to_string(),
        installed_version: update::installed_version(&root),
        launcher_version: env!("CARGO_PKG_VERSION").to_string(),
        installation,
        installation_ready,
        update_configuration: update::CONFIGURATION_MARKER,
    })
}

#[tauri::command]
async fn check_online_update() -> Result<update::UpdateInfo, String> {
    let root = install::launcher_install_root().map_err(|error| error.to_string())?;
    let installation = install::inspect(&root).map_err(|error| error.to_string())?;
    let installation_ready = install::is_ready(&installation);
    tauri::async_runtime::spawn_blocking(move || {
        update::check_installation(&root, installation_ready)
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| error.to_string())
}

#[tauri::command]
async fn install_online_update(app: AppHandle, launch_after: bool) -> Result<(), String> {
    let root = install::launcher_install_root().map_err(|error| error.to_string())?;
    let installation = install::inspect(&root).map_err(|error| error.to_string())?;
    let installation_ready = install::is_ready(&installation);
    let prepare_root = root.clone();
    let prepared = tauri::async_runtime::spawn_blocking(move || {
        update::prepare(&prepare_root, installation_ready)
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| error.to_string())?;

    if install::launcher_artifact_path(&root, &prepared.manifest).is_ok() {
        install::spawn_self_update(
            &prepared.staging,
            &prepared.manifest_path,
            &prepared.manifest,
            launch_after,
        )
        .map_err(|error| error.to_string())?;
        app.exit(0);
        return Ok(());
    }

    install::apply_staged(&root, &prepared.staging, &prepared.manifest)
        .map_err(|error| error.to_string())?;
    let installed = install::inspect(&root).map_err(|error| error.to_string())?;
    if !install::is_ready(&installed) {
        return Err(
            "the signed release did not install Arena oraja, Java, and its plugin".to_string(),
        );
    }
    install::write_version_marker(&root, &prepared.manifest.version)
        .map_err(|error| error.to_string())?;
    if launch_after {
        launch_detected(&root, false)?;
        app.exit(0);
    }
    Ok(())
}

#[tauri::command]
fn launch_game(app: AppHandle, configuration: bool) -> Result<(), String> {
    let root = install::launcher_install_root().map_err(|error| error.to_string())?;
    launch_detected(&root, configuration)?;
    app.exit(0);
    Ok(())
}

fn launch_detected(root: &Path, configuration: bool) -> Result<(), String> {
    let installation = install::inspect(root).map_err(|error| error.to_string())?;
    let java = installation
        .java_runtime
        .as_deref()
        .ok_or_else(|| "Java 21 or newer was not found".to_string())?;
    let game = installation
        .game_jar
        .as_deref()
        .ok_or_else(|| "Arena oraja JAR was not found".to_string())?;
    install::launch(root, Path::new(java), Path::new(game), configuration)
        .map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            launcher_state,
            check_online_update,
            install_online_update,
            launch_game
        ])
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
        let launch_after = arguments
            .next()
            .is_some_and(|value| value == std::ffi::OsStr::new("1"));
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
            launch_after,
        )
        .map_err(|error| error.to_string())
    })();
    if let Err(error) = result {
        eprintln!("BMS-IR Arena Launcher self-update failed: {error}");
    }
    true
}

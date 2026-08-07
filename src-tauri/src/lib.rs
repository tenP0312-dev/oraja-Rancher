mod install;
mod manifest;
mod settings;
mod update;

use install::InstallationInfo;
use serde::Serialize;
use std::env;
use std::fs;
use std::path::Path;
use std::time::Duration;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, WindowEvent};
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;
use tauri_plugin_notification::NotificationExt;

const UPDATE_PROGRESS_EVENT: &str = "arena-update-progress";
const MAIN_WINDOW_LABEL: &str = "main";
const BACKGROUND_CHECK_INTERVAL_MILLIS: u64 = 24 * 60 * 60 * 1000;
const BACKGROUND_POLL_INTERVAL: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, Serialize)]
struct LauncherState {
    installation: InstallationInfo,
    installation_ready: bool,
    update_configuration: &'static str,
    channel: String,
    platform: String,
    installed_version: String,
    launcher_version: String,
    cached_update: Option<update::UpdateInfo>,
    cached_policy_invalid: bool,
}

#[tauri::command]
fn launcher_state() -> Result<LauncherState, String> {
    let root = install::launcher_install_root().map_err(|error| error.to_string())?;
    let installation = install::inspect(&root).map_err(|error| error.to_string())?;
    let installation_ready = install::is_ready(&installation);
    let (cached_update, cached_policy_invalid) =
        match update::cached_update(&root, installation_ready) {
            Ok(update) => (update, false),
            Err(_) => (None, true),
        };
    Ok(LauncherState {
        channel: update::channel(),
        platform: update::platform().to_string(),
        installed_version: update::installed_version(&root),
        launcher_version: env!("CARGO_PKG_VERSION").to_string(),
        cached_update,
        cached_policy_invalid,
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
    let progress_app = app.clone();
    let prepared = tauri::async_runtime::spawn_blocking(move || {
        update::prepare_with_progress(&prepare_root, installation_ready, |progress| {
            let _ = progress_app.emit(UPDATE_PROGRESS_EVENT, progress);
        })
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| error.to_string())?;

    let _ = app.emit(
        UPDATE_PROGRESS_EVENT,
        update::UpdateProgress::completed(
            "verifying",
            prepared.transfer_bytes_total,
            prepared.verified_files_total,
        ),
    );
    let staged_launcher =
        install::staged_launcher_artifact_path(&root, &prepared.staging, &prepared.manifest);
    if staged_launcher.is_ok() {
        install::spawn_self_update(
            &prepared.staging,
            &prepared.manifest_path,
            &prepared.manifest,
            prepared.bootstrap_install,
            launch_after,
        )
        .map_err(|error| error.to_string())?;
        let _ = app.emit(
            UPDATE_PROGRESS_EVENT,
            update::UpdateProgress::completed(
                "restarting",
                prepared.transfer_bytes_total,
                prepared.verified_files_total,
            ),
        );
        app.exit(0);
        return Ok(());
    }
    if prepared.bootstrap_install {
        return Err(
            "the signed bootstrap release does not contain the current launcher".to_string(),
        );
    }

    let _ = app.emit(
        UPDATE_PROGRESS_EVENT,
        update::UpdateProgress::completed(
            "applying",
            prepared.transfer_bytes_total,
            prepared.verified_files_total,
        ),
    );
    install::apply_staged_mode(
        &root,
        &prepared.staging,
        &prepared.manifest,
        prepared.bootstrap_install,
    )
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
async fn list_deprecated_versions() -> Result<Vec<manifest::HistoryEntry>, String> {
    let root = install::launcher_install_root().map_err(|error| error.to_string())?;
    let current_version = update::installed_version(&root);
    tauri::async_runtime::spawn_blocking(move || update::list_deprecated_versions(&current_version))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn downgrade_to_version(app: AppHandle, version: String) -> Result<String, String> {
    let root = install::launcher_install_root().map_err(|error| error.to_string())?;
    let launcher_version = env!("CARGO_PKG_VERSION").to_string();
    let progress_app = app.clone();
    let manifest = tauri::async_runtime::spawn_blocking(move || {
        update::downgrade_to_version(&root, &version, &launcher_version, |progress| {
            let _ = progress_app.emit(UPDATE_PROGRESS_EVENT, progress);
        })
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| error.to_string())?;
    let root = install::launcher_install_root().map_err(|error| error.to_string())?;
    install::write_version_marker(&root, &manifest.version).map_err(|error| error.to_string())?;
    Ok(manifest.version)
}

#[tauri::command]
fn get_launcher_settings() -> Result<settings::LauncherSettings, String> {
    let root = install::launcher_install_root().map_err(|error| error.to_string())?;
    Ok(settings::load_settings(&root))
}

#[tauri::command]
fn set_launcher_settings(
    app: AppHandle,
    settings: settings::LauncherSettings,
) -> Result<(), String> {
    let root = install::launcher_install_root().map_err(|error| error.to_string())?;
    settings::save_settings(&root, &settings).map_err(|error| error.to_string())?;
    apply_autostart_setting(&app, settings.autostart)
}

fn apply_autostart_setting(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let manager = app.autolaunch();
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    result.map_err(|error| error.to_string())
}

fn current_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

/// Runs on its own OS thread for the life of the app. Sleeps in short
/// increments and only performs an actual check once the background-check
/// setting is on and 24 hours have passed, so toggling the setting takes
/// effect on the next poll without needing to start/stop the thread.
fn spawn_background_check_loop(app: AppHandle) {
    std::thread::spawn(move || loop {
        run_background_check_once(&app);
        std::thread::sleep(BACKGROUND_POLL_INTERVAL);
    });
}

fn run_background_check_once(app: &AppHandle) {
    let Ok(root) = install::launcher_install_root() else {
        return;
    };
    let mut current = settings::load_settings(&root);
    if !current.background_check {
        return;
    }
    let now = current_millis();
    if now.saturating_sub(current.last_background_check_millis) < BACKGROUND_CHECK_INTERVAL_MILLIS {
        return;
    }
    let Ok(installation) = install::inspect(&root) else {
        return;
    };
    let ready = install::is_ready(&installation);
    current.last_background_check_millis = now;
    if let Ok(info) = update::check_installation(&root, ready) {
        let should_notify = matches!(info.status.as_str(), "available" | "install_required")
            && info.available_version != current.last_notified_version;
        if should_notify {
            send_update_notification(app, &info.available_version);
            current.last_notified_version = info.available_version.clone();
        }
    }
    let _ = settings::save_settings(&root, &current);
}

fn send_update_notification(app: &AppHandle, version: &str) {
    let _ = app
        .notification()
        .builder()
        .title(format!("BMS-IR Arena {version} が利用できます"))
        .body("ランチャーを開いて更新してください")
        .show();
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let open_item = MenuItem::with_id(app, "open", "開く / Open", true, None::<&str>)?;
    let launch_item = MenuItem::with_id(
        app,
        "launch",
        "Arenaを起動 / Launch Arena",
        true,
        None::<&str>,
    )?;
    let check_item = MenuItem::with_id(
        app,
        "check",
        "更新を確認 / Check for updates",
        true,
        None::<&str>,
    )?;
    let quit_item = MenuItem::with_id(app, "quit", "終了 / Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_item, &launch_item, &check_item, &quit_item])?;

    let mut builder = TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(true)
        .tooltip("BMS-IR Arena");
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main_window(app),
            "launch" => {
                let _ = launch_game(app.clone(), false);
            }
            "check" => {
                show_main_window(app);
                let _ = app.emit("arena-tray-check-requested", ());
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

#[tauri::command]
fn launch_game(app: AppHandle, configuration: bool) -> Result<(), String> {
    let root = install::launcher_install_root().map_err(|error| error.to_string())?;
    let installation = install::inspect(&root).map_err(|error| error.to_string())?;
    update::enforce_cached_launch_policy(&root, install::is_ready(&installation))
        .map_err(|error| error.to_string())?;
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
    if let Ok(root) = install::launcher_install_root() {
        install::cleanup_stale_update_state(&root);
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .invoke_handler(tauri::generate_handler![
            launcher_state,
            check_online_update,
            install_online_update,
            list_deprecated_versions,
            downgrade_to_version,
            get_launcher_settings,
            set_launcher_settings,
            launch_game
        ])
        .on_window_event(|window, event| {
            if window.label() != MAIN_WINDOW_LABEL {
                return;
            }
            if let WindowEvent::CloseRequested { api, .. } = event {
                let resident = install::launcher_install_root()
                    .map(|root| settings::load_settings(&root).resident)
                    .unwrap_or(false);
                if resident {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(|app| {
            build_tray(app)?;
            spawn_background_check_loop(app.handle().clone());
            Ok(())
        })
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
        let remaining = arguments.collect::<Vec<_>>();
        let (bootstrap_install, launch_after) = match remaining.as_slice() {
            [launch_after] => (false, launch_after == std::ffi::OsStr::new("1")),
            [bootstrap, launch_after] => (
                bootstrap == std::ffi::OsStr::new("1"),
                launch_after == std::ffi::OsStr::new("1"),
            ),
            _ => return Err("unexpected self-update arguments".to_string()),
        };
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
            bootstrap_install,
            launch_after,
        )
        .map_err(|error| error.to_string())
    })();
    if let Err(error) = result {
        eprintln!("BMS-IR Arena Launcher self-update failed: {error}");
    }
    true
}

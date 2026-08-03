#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

fn main() {
    if bmsir_arena_launcher::run_self_update_helper_if_requested() {
        return;
    }
    bmsir_arena_launcher::run();
}

fn main() {
    if bmsir_arena_launcher::run_self_update_helper_if_requested() {
        return;
    }
    bmsir_arena_launcher::run();
}

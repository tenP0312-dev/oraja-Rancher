fn main() {
    // `option_env!` alone is not an input that Cargo tracks for incremental
    // compilation. Forward every launcher configuration value through this
    // build script so a changed test-channel version, endpoint, or key forces
    // a fresh launcher binary instead of reusing an older configured build.
    for variable in [
        "BMSIR_ARENA_CLIENT_VERSION",
        "BMSIR_ARENA_UPDATE_BASE_URL",
        "BMSIR_ARENA_UPDATE_CHANNEL",
        "BMSIR_ARENA_RELEASE_PUBLIC_KEY",
    ] {
        println!("cargo:rerun-if-env-changed={variable}");
        let value = std::env::var(variable).unwrap_or_default();
        assert!(
            !value.contains('\n') && !value.contains('\r'),
            "{variable} must not contain a line break"
        );
        if variable == "BMSIR_ARENA_UPDATE_CHANNEL" {
            assert!(
                value.is_empty() || matches!(value.as_str(), "stable" | "test"),
                "BMSIR_ARENA_UPDATE_CHANNEL must be stable or test"
            );
        }
        println!("cargo:rustc-env={variable}={value}");
    }

    tauri_build::build()
}

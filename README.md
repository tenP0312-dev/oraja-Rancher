# BMS-IR Arena Launcher

Tauri 2 based launcher for the unified BMS-IR Arena oraja distribution.
The current source version is `0.1.1`.

The launcher treats a release manifest as untrusted until its canonical JSON
payload passes Ed25519 verification. Every staged artifact is then checked
against its declared SHA-256 before an atomic install. Existing files are
backed up and restored if any step fails.

Development builds are intentionally not production releases. Windows
Authenticode and macOS Developer ID + notarization are required before a
launcher artifact can be published as the official download.

The GUI detects the game body, rejects ambiguous duplicate BMS-IR plugin jars,
accepts Java 21 or newer, launches configuration or play without shell
interpolation, and can update INI values without rewriting unrelated comments
or keys. A verified versioned plugin update moves the prior single plugin into
the transaction backup before installing its replacement. Offline
updates use the public key compiled as `BMSIR_ARENA_RELEASE_PUBLIC_KEY`; builds
without that reviewed key fail closed when update is requested. A verified
staged launcher can restart in helper mode, wait for the old process to exit,
atomically replace itself, roll back on failure, and relaunch. Signed release
notes are rendered with DOM text nodes and a small heading/list subset; release
HTML is never executed.

`arena-launcher-ci.yml` creates short-lived unsigned validation bundles on
Windows x64 and macOS arm64. Those artifacts are explicitly not releases.
Official publication remains gated on platform signing/notarization and the
compiled release-verification key.

## Local checks

```sh
cd src-tauri
cargo test
cargo check
```

The static frontend lives in `web/`; no remote script or HTML release note is
loaded.

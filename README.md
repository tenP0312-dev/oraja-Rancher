# BMS-IR Arena portable launcher

`BMS-IR Arena.exe` is a portable Tauri 2 launcher. It uses the directory that
contains the executable as the Arena oraja root. There is no installer, MSI,
registry write, administrator requirement, Start Menu registration, or folder
picker. Existing BAT launch remains valid.

The window exposes three normal actions:

- launch Arena
- open the existing pre-launch configuration with `-c`
- check for updates

Update checks use the channel selected from the executable name. `BMS-IR
Arena.exe` reads `stable`; `BMS-IR Arena Test.exe` reads `test`, so separate
folders can coexist. Network failure never blocks a valid installed body.
When the executable is placed in an otherwise empty directory, the launcher
checks the selected channel immediately and offers the signed current release
as an initial download even when its version matches the launcher's body
version. A missing or incomplete body is never treated as already installed.
Optional updates retain a launch-current action; mandatory or revoked versions
do not.

Rust downloads the platform manifest and artifacts. The WebView never chooses
paths or verifies security metadata. The update is accepted only after the
canonical manifest passes its compiled Ed25519 key and every file matches the
signed path, size, and SHA-256. Files are staged under the portable root,
backed up, replaced, and rolled back on failure. A launcher update starts the
verified staged executable as a helper, waits for the old process, applies the
same transaction, and then starts the game when requested.

The static patch publication tools and manifest layout are maintained in
`tenP0312-dev/bms-ir-arena-patch-server`. Build variables are:

- `BMSIR_ARENA_RELEASE_PUBLIC_KEY`: raw Ed25519 public key in Base64
- `BMSIR_ARENA_UPDATE_BASE_URL`: HTTPS root containing `channels/`
- `BMSIR_ARENA_CLIENT_VERSION`: initial body version when no local marker exists

The release packager accepts only a launcher compiled with both the endpoint
and verification key. This prevents an offline CI validation binary from being
renamed and shipped as a working updater.

## Validation build

```sh
cd src-tauri
cargo test --locked
cargo build --release --locked
```

CI uploads one short-lived, unsigned Windows validation executable. It does not
build an installer or a distributable updater. Public stable distribution
remains blocked until the reviewed production Ed25519 key and Authenticode
signing are supplied. Internal test builds may use a disposable test key
without per-build publication approval.

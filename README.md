# BMS-IR Arena portable launcher

`BMS-IR Arena.exe` and `BMS-IR Arena.app` are portable Tauri 2 launchers. They
use the directory that contains the EXE or app bundle as the Arena oraja root.
There is no installer, MSI, registry write, administrator requirement, Start
Menu registration, or folder picker. Existing BAT and command launch remain
valid.

The window exposes three normal actions:

- launch Arena
- open the existing pre-launch configuration with `-c`
- check for updates

Update checks use the channel selected from the executable name. `BMS-IR
Arena.exe` reads `stable`; `BMS-IR Arena Test.exe` reads `test`, so separate
folders can coexist. The equivalent macOS app bundle names select the same
channels. Network failure does not newly block a valid installed body. Once a
signed mandatory update, revocation, or minimum-launcher requirement has been
verified, the launcher caches that signed policy and keeps blocking the old
version during later network failures. The Rust launch command enforces the
same decision as the WebView; disabling a button is not the security boundary.
When the executable is placed in an otherwise empty directory, the launcher
checks the selected channel immediately and offers the signed current release
as an initial download even when its version matches the launcher's body
version. A missing or incomplete body is never treated as already installed.
Optional updates retain a launch-current action; mandatory or revoked versions
do not. A client that has never downloaded the mandatory policy is still
subject to the Arena service compatibility gate.

The signed manifest carries Japanese and English release notes plus up to 20
newest-first announcements with an ISO date and title in both languages. The
launcher switches this content with its `🌐 日本語` / `🌐 English` control and
keeps the announcement list visible even when the installed version is current.
Legacy single-language release notes remain readable as a fallback.

Rust downloads the platform manifest and artifacts. The WebView never chooses
paths or verifies security metadata. The update is accepted only after the
canonical manifest passes its compiled Ed25519 key and every file matches the
signed path, size, and SHA-256. Transient transport, rate-limit, and server
errors are retried with a bounded backoff. Files are staged under the portable
root, signed executable flags are applied before a staged launcher is started,
and installation is backed up, replaced, and rolled back on failure. A launcher
update starts the verified staged executable as a helper, waits for the old
process, applies the same transaction, and then starts the game when requested.

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

CI uploads short-lived Windows and macOS validation outputs. It does not build
an installer. A manually dispatched CI run also builds `BMS-IR Arena Test.exe`
and an ad-hoc-signed `BMS-IR Arena Test.app` ZIP when
`ARENA_TEST_UPDATE_BASE_URL` and `ARENA_TEST_RELEASE_PUBLIC_KEY` repository
variables are present. Those artifacts are retained for one day and are only
for the internal test channel. Public stable distribution remains blocked
until the reviewed production Ed25519 key, Authenticode signing, and Apple
Developer ID/notarization are supplied. Internal test builds may use a
disposable test key without per-build publication approval.

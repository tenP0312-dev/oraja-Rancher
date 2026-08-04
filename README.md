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

The dedicated game body is `Arena-oraja.jar`. Existing `beatoraja.jar` and
versioned Arena oraja JARs remain launchable only as compatibility fallbacks;
the updater never deletes or overwrites a user's ordinary `beatoraja.jar`.

The header always identifies the installed body version, selected update
channel, and launcher version separately. During an installation or update,
the status band shows byte-accurate overall progress, transferred and total
size, and verified file count. After transfer it reports the verification,
application, and launcher-restart phases instead of leaving the window on an
indeterminate update message.

Update checks use the channel selected from the executable name. `BMS-IR
Arena.exe` reads `stable`; `BMS-IR Arena Test.exe` reads `test`, so separate
folders can coexist. The equivalent macOS app bundle names select the same
channels. Network failure does not newly block a valid installed body. Once a
signed mandatory update, revocation, or minimum-launcher requirement has been
verified, the launcher caches that signed policy and keeps blocking the old
version during later network failures. The Rust launch command enforces the
same decision as the WebView; disabling a button is not the security boundary.
For an existing installation, artifacts already matching the signed size,
SHA-256, and executable flag are not downloaded or replaced. When the
executable is placed in an otherwise empty directory, the launcher checks the
selected channel immediately and offers the signed current release as an
initial download even when its version matches the launcher's body version.
Current manifests may provide one signed compressed bootstrap ZIP plus a full
signed file inventory; the launcher verifies the ZIP, extracts only inventory
paths, verifies every extracted file, and then applies any newer sparse delta.
A missing or incomplete body is never treated as already installed.
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
process, applies the same transaction, removes staging and backup data, and
then starts the game when requested. The helper runs from a small verified copy
outside staging so Windows can remove the downloaded update immediately.

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

Pull-request CI tests and validates the ordinary portable outputs. It does not
build an installer. A manually dispatched CI run builds only the configured
`BMS-IR Arena Test.exe`
and an ad-hoc-signed `BMS-IR Arena Test.app` ZIP when
`ARENA_TEST_UPDATE_BASE_URL` and `ARENA_TEST_RELEASE_PUBLIC_KEY` repository
variables are present. Those artifacts are retained for one day and are only
for the internal test channel. Public stable distribution remains blocked
until the reviewed production Ed25519 key, Authenticode signing, and Apple
Developer ID/notarization are supplied. Internal test builds may use a
disposable test key without per-build publication approval. CI downloads the
official Tauri CLI 2.11.4 binary with a pinned SHA-256 instead of compiling the
CLI for every clean runner.

## Internal development loop

Use the macOS build as the normal implementation loop on the development Mac.
Run the focused Rust tests, build the macOS launcher, and verify a sparse update
in an isolated portable root. Do not rebuild both platforms after each edit.

After the code is stable, dispatch the configured CI workflow once to build and
validate both the Windows and macOS internal launchers. Run the complete
empty-directory compressed bootstrap test only when bootstrap extraction,
inventory verification, launcher self-update, post-update cleanup, or storage
behavior changed. Ordinary UI and client fixes need only the sparse-update
path. If final validation reveals a bug, fix it and repeat the affected check;
the reduced loop must not be used to skip a real regression.

# BMS-IR Arena Launcher Agent Guide

This repository owns the portable BMS-IR Arena launcher. Read `README.md` and
the code/tests around the affected behavior before editing.

## Working Rules

- Preserve unrelated changes and avoid destructive Git commands.
- Use an Issue, a scoped `codex/` branch, validation, a pull request, and
  passing CI for implementation work. Keep `main` protected.
- Use `apply_patch` for manual edits. Do not commit binaries, generated release
  trees, credentials, signing material, game data, databases, or logs.
- Run `cargo test --locked` under `src-tauri` for code changes and
  `git diff --check` for every change.

## GUI Boundary

Codex must not use Computer Use or equivalent desktop automation for launcher
or game debugging, QA, or acceptance. Do not launch, activate, focus, or
control the launcher, updater, or game body. Use tests, artifact inspection,
CLI/network probes, and logs; leave physical acceptance to the operator and
record only the evidence they return.

## Distribution Completion

- Source merge and launcher build do not authorize binary publication.
- Any BMS-IR-built body or plugin made downloadable through this launcher is
  gate-bound, including internal test builds, prereleases, sparse updates, and
  stable releases. Launcher availability, not a formal-release label, is the
  trigger.
- Before promoting the signed channel, complete every applicable ordinary-
  score body/plugin allowlist and Arena client-version/build gate, required
  guarded service reload, and effective check through `BMS-Mania/IR`'s
  `docs/PRODUCTION_VPS_OPERATIONS.md`. Once distribution is authorized, these
  additive gate steps need no separate per-artifact prompt.
- Use only the exact reviewed artifacts named by the signed manifest. Local
  previews and third-party or unreviewed builds are excluded. A launcher-only
  release has no body/plugin gate to add.
- Never report a release complete while the launcher can download an artifact
  that ordinary score submission or the Arena connection gate rejects.
- Stable/mandatory publication, gate removal or revocation, new gate
  semantics, public announcements, and credentials retain their existing
  explicit approval rules.

Signed update metadata and publication tooling live in
`tenP0312-dev/bms-ir-arena-patch-server`; the server gates and canonical
operations runbook live in `BMS-Mania/IR`.

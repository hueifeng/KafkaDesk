# KafkaDesk Release & Distribution Guide

This is the current public release path for KafkaDesk.

KafkaDesk is still pre 1.0. Desktop packaging is automated, but release sign off is still manual and verification first.

## What ships

KafkaDesk is a Tauri desktop application.

- frontend assets are built into `dist/`
- the desktop runtime is built from `src-tauri/`
- desktop bundles are produced by `npm run tauri:build`

This repository does not publish an npm package.

## Where to download builds

KafkaDesk builds currently show up on GitHub in two ways:

- **GitHub Releases** for pushed version tags such as `v0.1.0`
- **GitHub Actions artifacts** for manual `workflow_dispatch` packaging runs

Tag driven releases attach the native files produced from `src-tauri/target/release/bundle/`. Depending on platform, that can include:

- macOS `.dmg`
- Windows `.msi` or `.exe`
- Linux `.deb`, `.rpm`, or `.AppImage`

The workflow also uploads per platform bundle archives as Actions artifacts.

## How release publishing works

The workflow lives at `.github/workflows/package-desktop-bundles.yml`.

It runs in two modes:

- `workflow_dispatch` packages builds and uploads downloadable workflow artifacts
- `push` on tags matching `v*` packages builds, uploads workflow artifacts, and creates or updates the matching GitHub Release

Current GitHub hosted targets are:

- macOS x64 on `macos-13`
- macOS arm64 on `macos-14`
- Windows x64 on `windows-latest`
- Linux x64 on `ubuntu-22.04`

Each job uses the same checked-in build entry point:

```bash
npm run tauri:build
```

## Version alignment contract

Versioned releases only work when the version matches in all three checked-in files:

- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

For tag driven releases, the pushed tag must match that version exactly, for example `v0.1.0` for app version `0.1.0`. The workflow fails fast on any mismatch.

Release asset filenames are also expected to include the same app version.

## Local release build

Before building locally, make sure the machine has:

- Node.js 22
- npm 10+
- Rust stable toolchain
- Tauri platform prerequisites for the target OS

From the repository root:

```bash
npm ci
npm run check
npm run tauri:build
```

Tauri bundle output is generated under `src-tauri/target/release/bundle/`.

## Current caveats

- GitHub packaging currently produces unsigned desktop assets
- maintainer review, runtime verification, and release sign off are still manual
- no signing or notarization workflow is checked into the repository
- provenance and a fully automated signed public release pipeline are not in place yet

Treat current builds as manually verified engineering artifacts, not as a fully automated signed release train.

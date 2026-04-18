# KafkaDesk Release & Distribution Guide

This document describes the current truth for producing desktop build artifacts from the repository.

KafkaDesk is still pre-1.0. The current release posture is still **manual and verification-first** for release sign-off, but the repository now includes a first GitHub Actions packaging batch for unsigned desktop bundles.

## What Gets Distributed

KafkaDesk is a Tauri desktop application.

- frontend assets are built with Vite into `dist/`
- the desktop runtime is built from `src-tauri/`
- distributable desktop bundles are produced by `tauri build`

This repository is **not** currently set up for npm package publishing.

## GitHub Packaging Workflow

The repository now includes a packaging workflow at `.github/workflows/package-desktop-bundles.yml`.

It runs in two modes:

- `workflow_dispatch` for manual packaging runs
- `release.published` for packaging runs tied to a published GitHub Release

The workflow currently builds and archives unsigned bundle output for these GitHub-hosted targets:

- macOS x64 on `macos-13`
- macOS arm64 on `macos-14`
- Windows x64 on `windows-latest`
- Linux x64 on `ubuntu-22.04`

Each matrix job uses the existing repository entry point:

```bash
npm run tauri:build
```

That means GitHub packaging follows the same checked-in Tauri build path as local release builds.

## GitHub Download Paths

GitHub packaging output is currently exposed in two truthful ways:

- manual workflow runs upload one downloadable archive per target platform as a GitHub Actions artifact
- published release runs upload the same per-platform archives as workflow artifacts and also attach them to the matching GitHub Release

Each archive contains the platform's generated `src-tauri/target/release/bundle/` output. Exact installer/package file types remain operating-system-specific.

## Prerequisites

Before attempting a local release build, ensure the release machine has:

- Node.js 22
- npm 10+
- Rust stable toolchain
- Tauri platform prerequisites for the target OS
- any platform-specific signing/notarization prerequisites if you plan to distribute outside local/internal testing

For the GitHub packaging workflow, GitHub-hosted runners supply the build machine and the workflow installs the Linux-side packaging dependencies it needs for the current x64 Linux lane.

## Release Verification Baseline

From the repository root, run the full verification baseline first:

```bash
npm ci
npm run check
```

Do not cut or distribute a build that has not passed the current baseline.

The packaging workflow itself is focused on bundle creation and upload. It does **not** replace maintainer release review, runtime verification, signing, or notarization.

## Build Release Artifacts

Create desktop bundles from the repository root:

```bash
npm run tauri:build
```

Relevant Tauri configuration currently lives in `src-tauri/tauri.conf.json`.

That config currently uses:

- `beforeBuildCommand: npm run build`
- `bundle.active: true`
- `bundle.targets: all`

This local build path is also what the GitHub packaging workflow runs on each target runner.

## Expected Output Locations

Tauri release output is generated under `src-tauri/target/release/`.

Platform-specific bundle artifacts are typically placed under a bundle subdirectory such as:

- `src-tauri/target/release/bundle/`

Exact artifact types depend on the operating system and local Tauri toolchain support.

For GitHub-hosted packaging runs, the uploaded archive names include the target platform plus either the published release tag or the commit SHA used for the manual run.

## Pre-Distribution Checklist

Before sharing a build, confirm:

- `npm run check` passed on the release input revision
- any security-sensitive runtime/docs changes are reflected in `README.md`, `CONTRIBUTING.md`, and `SECURITY.md`
- replay/runtime caveats are still truthful for the build you are distributing
- the build environment still includes the expected Kafka TLS/OpenSSL support
- release notes mention any blocked/known limitations that affect operator trust

## Current Gaps / Non-Automated Areas

The following areas are not yet fully productized:

- no documented signing/notarization workflow is checked into the repository
- GitHub automation only covers unsigned bundle creation plus artifact/release-asset upload; it does not provide a fully automated signed public release pipeline
- no changelog/release-note generation workflow is automated
- no distribution provenance/attestation workflow is configured

Until those gaps are addressed, treat builds as manually verified engineering artifacts rather than a fully automated public release train.

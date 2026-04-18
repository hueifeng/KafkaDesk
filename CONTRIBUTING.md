# Contributing to KafkaDesk

Thanks for helping harden KafkaDesk.

This repository is currently focused on making the desktop workbench trustworthy, testable, and open-source-ready for its current scope. Contributions are most useful when they preserve that direction: truthful runtime behavior, explicit failure states, and a verified repository after every batch of work.

## Before You Start

- read `README.md` for the current product/runtime status
- review the design and implementation docs under `docs/`
- prefer narrow, verified changes over large speculative rewrites

## Local Environment

KafkaDesk currently builds as a Tauri desktop application with a React/Vite frontend and a Rust runtime.

Recommended local toolchain:

- Node.js 22
- npm 10+
- Rust stable toolchain
- platform prerequisites required by Tauri
  - macOS: Xcode Command Line Tools

## Setup

Install frontend dependencies from the repository root:

```bash
npm ci
```

## Common Development Commands

Run the frontend in browser/dev mode:

```bash
npm run dev
```

Run the desktop shell during development:

```bash
npm run tauri:dev
```

Start the Tauri dev shell through the helper script:

```bash
./scripts/start.sh
```

## Verification

Keep the repository verified after each meaningful change.

Fast local sanity pass:

```bash
npm run smoke
```

Frontend-only verification:

```bash
npm run check:frontend
```

Rust-only verification:

```bash
npm run check:rust
```

Full project verification:

```bash
npm run check
```

Frontend watch mode:

```bash
npm exec vitest -- --config vitest.config.ts --watch
```

## Contribution Expectations

- keep runtime semantics truthful; do not introduce optimistic placeholder success states
- prefer explicit validation, capability detection, and error categorization
- update docs when behavior, verification steps, or operational caveats change
- add or extend tests when changing Rust services, validation behavior, decode behavior, or replay lifecycle logic
- do not commit generated output drift unless the repository intentionally tracks it

## Planning and Task Tracking

When working against the current maturity backlog:

- prefer GitHub issues, pull requests, and accepted design/decision docs as the public planning trail
- keep pull requests narrowly scoped and explain what changed, how it was verified, and what remains out of scope
- update repository docs whenever behavior, verification steps, or operational caveats change
- use the pull request template to summarize scope, verification, and any residual risk notes

## Reporting Product or Runtime Gaps

Use the GitHub issue templates when available. For high-signal reports, include the following:

- what workflow you were attempting
- expected behavior
- actual behavior
- whether the failure was frontend-only, Rust/runtime-only, or end-to-end
- local verification command(s) run and their results
- cluster/security mode involved if Kafka connectivity was part of the failure

## Security and Secrets

- do not commit real credentials, broker secrets, private keys, or production certificate material
- use temporary/local-only secrets for testing
- prefer keyring-backed credential references over plaintext storage paths when the runtime supports them
- if you discover a security issue, follow `SECURITY.md` once it exists; until then, avoid public disclosure of exploitable details in commits or issue text

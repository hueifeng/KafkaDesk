# KafkaDesk

KafkaDesk is a desktop debugging workbench for Kafka-based event systems.

It gives engineers one local tool for the jobs that usually get split across CLIs, dashboards, browser tools, and ad hoc scripts: inspect topics, diagnose consumer lag, browse and decode messages, trace a business key across topics, and replay events with explicit safety boundaries.

<img width="3024" height="1762" alt="image" src="https://github.com/user-attachments/assets/0114d811-f714-4be6-924a-201867efc844" />


## What you can do today

- configure clusters with staged validation for input, reachability, auth, TLS, and schema registry readiness
- browse topics and inspect topic detail
- inspect consumer groups and lag
- run bounded message queries with decode status and detail views
- decode payloads through schema registry aware paths
- replay messages through policy checked, audited flows
- trace a business key across configured correlation rules
- save queries, bookmarks, audit history, and local operator preferences

Current app areas include Overview, Topics, Groups, Messages, Replay, Trace, Saved Queries, Audit, and Settings.

## Why it is desktop first

KafkaDesk is currently shipped as a Tauri desktop app with a React + Vite frontend, a Rust local runtime, and SQLite backed local persistence.

That fits the environments it targets. Many Kafka systems are reachable from an engineer workstation long before they are suitable for a shared hosted deployment.

## Downloads and releases

Versioned desktop builds are published on GitHub Releases when a pushed tag matches the checked-in app version, for example `v0.1.0`.

- tag driven releases attach native desktop assets from the Tauri bundle output
- manual `workflow_dispatch` runs still exist, but they publish workflow artifacts rather than a versioned GitHub Release
- current GitHub produced assets are unsigned, and release sign off is still manual and verification first

For the current release path and caveats, see [`docs/product/release-distribution.md`](./docs/product/release-distribution.md).

## Quick start for local development

Prerequisites:

- Node.js 22
- npm 10+
- Rust stable toolchain
- Tauri platform prerequisites for your OS

Install dependencies:

```bash
npm ci
```

Run the frontend:

```bash
npm run dev
```

Run the desktop app:

```bash
npm run tauri:dev
```

Helper script:

```bash
./scripts/start.sh
```

## Verification and safety

KafkaDesk favors truthful runtime behavior over optimistic success states.

- cluster validation reports staged readiness instead of a flat pass or fail
- secured Kafka paths reuse the same auth and TLS runtime wiring across validation, browsing, queries, trace, and replay
- schema registry validation checks endpoint reachability and credential readiness before reporting success
- replay stays an explicit, policy constrained workflow rather than a casual write action

Useful verification commands:

- `npm run smoke` for a fast local sanity pass
- `npm run check:frontend` for frontend lint, typecheck, tests, and production build
- `npm run check:rust` for Rust check and tests
- `npm run check` for the main repository baseline

For security and operator caveats, read [`SECURITY.md`](./SECURITY.md).

## Documentation

- [`docs/README.md`](./docs/README.md) for the main docs landing page
- [`docs/product/README.md`](./docs/product/README.md) for current product docs under `docs/`
- [`docs/reference/README.md`](./docs/reference/README.md) for architecture and technical reference material
- [`CONTRIBUTING.md`](./CONTRIBUTING.md) for setup and contribution workflow
- [`SECURITY.md`](./SECURITY.md) for vulnerability reporting and runtime caveats

Older planning material is still available under [`docs/archive/`](./docs/archive/README.md), but it is not the current product status.

## Contributing

KafkaDesk is MIT licensed. If you want to help, start with [`CONTRIBUTING.md`](./CONTRIBUTING.md), keep changes narrow and verified, and update docs whenever runtime behavior or safety expectations change.

# KafkaDesk

KafkaDesk is a desktop debugging workbench for Kafka-based event systems.

It is built for engineers who need one local tool to inspect topics, diagnose lag, browse and decode messages, trace a business key across topics, and replay events with explicit safety boundaries.

## Who It Is For

KafkaDesk is aimed at engineers who regularly need to:

- inspect live topic traffic without switching between multiple tools
- understand consumer lag and unhealthy group state quickly
- read payloads after decode, not just as opaque bytes
- replay messages deliberately with visible safety boundaries
- follow one business event across related topics and services

## Quick Start

### Prerequisites

Before running KafkaDesk locally, make sure the machine has:

- Node.js 22
- npm 10+
- Rust stable toolchain
- Tauri platform prerequisites for the host OS

### Install Dependencies

```bash
npm ci
```

### Run in Frontend Dev Mode

```bash
npm run dev
```

### Run the Desktop App During Development

```bash
npm run tauri:dev
```

You can also use the helper script:

```bash
./scripts/start.sh
```

## What KafkaDesk Does Today

The current desktop app includes these primary workflows:

- cluster configuration with staged validation for input, reachability, auth, and TLS readiness
- topic browsing and topic detail inspection
- consumer group inspection and lag diagnosis
- bounded message queries with detail views and decode status reporting
- schema-registry-aware payload decode paths
- controlled replay with policy checks and audit recording
- trace-by-key style investigation across configured correlation rules
- saved queries, bookmarks, audit history, and local operator preferences

The app surface currently includes Overview, Topics, Groups, Messages, Replay, Trace, Saved Queries, Audit, and Settings.

## Why KafkaDesk

Kafka debugging often gets split across CLIs, browser tools, dashboards, tracing systems, and ad-hoc scripts.

KafkaDesk is meant to reduce that context switching by keeping the most common operational paths in one local workbench:

- cluster state and validation
- topic and group inspection
- bounded message analysis
- decode-aware message detail
- replay with policy guardrails
- trace-oriented investigation

## Product Shape

KafkaDesk is currently implemented as:

- a Tauri desktop shell
- a React + Vite frontend
- a Rust local runtime
- a SQLite-backed local persistence layer

This product shape is deliberate: many Kafka environments are reachable from an engineer workstation long before they are suitable for central hosted deployment.

## Verification

Use these commands during development:

- `npm run smoke`
- `npm run check:frontend`
- `npm run check:rust`
- `npm exec vitest -- --config vitest.config.ts --watch`
- `npm run test:frontend:coverage`
- `npm run check`

Verification lanes:

- `npm run smoke` runs the fastest local sanity pass for lint, TypeScript, and frontend tests
- `npm run check:frontend` runs frontend lint, typecheck, tests, and a production Vite build
- `npm run check:rust` runs `cargo check` and Rust tests
- `npm run check` is the main repository baseline

## Runtime Guarantees and Safety Posture

KafkaDesk favors truthful runtime behavior over optimistic success states.

- cluster validation reports staged readiness instead of a flat pass/fail
- secured Kafka paths reuse the same auth/TLS runtime wiring across validation, browsing, queries, trace, and replay
- schema registry validation checks endpoint reachability and credential readiness before reporting success
- replay remains an explicit, policy-constrained workflow rather than a casual write action

For security and operator-safety details, read [`SECURITY.md`](./SECURITY.md).

## Documentation Map

Start with the documentation index, then choose the section that matches what you need:

- [`docs/README.md`](./docs/README.md) — main documentation entry point and reading order
- [`docs/product/README.md`](./docs/product/README.md) — current product-facing repository docs
- [`docs/reference/README.md`](./docs/reference/README.md) — technical and design reference material
- [`docs/archive/README.md`](./docs/archive/README.md) — archived planning and project-history material

Core repository docs:

- [`CONTRIBUTING.md`](./CONTRIBUTING.md) — setup, verification, and contribution workflow
- [`SECURITY.md`](./SECURITY.md) — vulnerability reporting and runtime caveats
- [`docs/product/release-distribution.md`](./docs/product/release-distribution.md) — how release artifacts are currently built and what is still manual

## Release and Open-Source Status

KafkaDesk is now published with a top-level MIT `LICENSE` and is structured to be shared as a public source repository.

- the repository is now license-cleared for reuse and redistribution under the terms in [`LICENSE`](./LICENSE)
- contributor, security, and release-process documentation already exist and can be used for local evaluation and engineering work
- GitHub now builds unsigned multi-platform desktop bundle archives for manual runs and published releases, but release verification/sign-off is still manual and verification-first
- signing, notarization, provenance, and fully automated public release management are not fully productized yet
- the current release/distribution posture is documented in [`docs/product/release-distribution.md`](./docs/product/release-distribution.md)

## Current Limitations

The biggest remaining external-release gaps are release operations rather than core workflows:

- distribution automation now covers unsigned GitHub-hosted packaging artifacts, but it is still not a fully automated signed release train

## Contributing

If you want to help harden the product:

- read [`CONTRIBUTING.md`](./CONTRIBUTING.md)
- keep changes narrow and verified
- update docs whenever runtime behavior, safety expectations, or verification steps change

## Documentation Notes

The active product narrative now lives in this README, the root contribution and security docs, and the current-doc sections under `docs/`.

Older planning material is still kept in the repository for traceability, but it has been moved under [`docs/archive/`](./docs/archive/README.md) so it does not read like the current product status.

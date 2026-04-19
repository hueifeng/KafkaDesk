# KafkaDesk

KafkaDesk is a desktop debugging workbench for Kafka-based event systems.

It gives engineers one local tool for the jobs that usually get split across CLIs, dashboards, browser tools, and ad hoc scripts: inspect topics, diagnose consumer lag, browse and decode messages, trace a business key across topics, and replay events safely.

<img width="3024" height="1762" alt="image" src="https://github.com/user-attachments/assets/0114d811-f714-4be6-924a-201867efc844" />

## Install

- Download the latest desktop build from [GitHub Releases](https://github.com/hueifeng/KafkaDesk/releases)
- For local development or custom builds, use the source workflow below

Release assets are published with explicit OS and architecture names, for example `KafkaDesk-0.1.0-macos-arm64.dmg`.

Release packaging details live in [`docs/product/release-distribution.md`](./docs/product/release-distribution.md).

## Features

- Configure Kafka clusters and Schema Registry connections
- Browse topics, inspect topic detail, and check consumer group lag
- Query and inspect messages with decode support
- Replay events through explicit, auditable workflows
- Trace business keys across correlated topics
- Save queries, bookmarks, audit history, and local preferences

## Development

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

Verification commands:

- `npm run smoke` for a fast local sanity pass
- `npm run check:frontend` for frontend lint, typecheck, tests, and production build
- `npm run check:rust` for Rust check and tests
- `npm run check` for the main repository baseline

For runtime caveats and security expectations, read [`SECURITY.md`](./SECURITY.md).

## Documentation

- [`docs/README.md`](./docs/README.md) for the main docs landing page
- [`docs/product/README.md`](./docs/product/README.md) for current product docs under `docs/`
- [`docs/reference/README.md`](./docs/reference/README.md) for architecture and technical reference material
- [`CONTRIBUTING.md`](./CONTRIBUTING.md) for setup and contribution workflow
- [`SECURITY.md`](./SECURITY.md) for vulnerability reporting and runtime caveats

Older planning material is still available under [`docs/archive/`](./docs/archive/README.md), but it is not the current product status.

## Contributing

KafkaDesk is MIT licensed. If you want to contribute, start with [`CONTRIBUTING.md`](./CONTRIBUTING.md).

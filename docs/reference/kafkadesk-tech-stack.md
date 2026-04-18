# KafkaDesk Tech Stack

This page describes the stack that is checked into the repository today.

## What exists now

### Desktop shell and frontend

* Tauri v2 for the desktop shell
* React 18 for the renderer
* Vite for frontend development and production builds
* React Router for page navigation
* TanStack Query for runtime backed data fetching
* Zustand for local UI state
* Tailwind CSS for styling

### Local runtime

* Rust 2021 in `src-tauri`
* Tokio for async runtime work
* `tracing` and `tracing-subscriber` for diagnostics
* Tauri commands as the frontend to runtime boundary

### Kafka, schema, and persistence

* `rdkafka` for Kafka connectivity
* `apache-avro` for Avro decode support
* `reqwest` for Schema Registry HTTP calls
* `sqlx` with SQLite for local persistence and migrations
* `keyring` for system backed secret storage

### Frontend quality and checks

* TypeScript
* ESLint
* Vitest

## Key boundaries

* The frontend depends on Tauri command names and typed DTOs, not direct Rust internals.
* The Rust runtime owns broker IO, schema registry IO, persistence, and secret lookup.
* SQLite is for local metadata and durable workflow records. The intended boundary is credential references plus OS-backed secret storage, but some secured validation paths are still catching up to that model.

## Important implementation notes

* The repository uses one app, not a desktop shell plus a separate bundled backend process.
* The current stack is local first by design. It assumes Kafka and Schema Registry are reachable from the operator machine.
* Tailwind, Zustand, and TanStack Query are present in the codebase now. Older reference text that framed them as options is out of date.
* `keyring` is part of the checked-in runtime, but the surrounding product docs still call out incomplete credential-backed validation in some secured flows.

## Known limits

* There is no shared contract generation layer between the TypeScript and Rust models.
* Running `npm run dev` alone is not enough for runtime backed workflows. Those need `tauri dev` or another Tauri launch path.
* Release signing and notarization are still documented elsewhere as manual work, see the product and release docs for that status.

## Related docs

* [`kafkadesk-desktop-architecture.md`](./kafkadesk-desktop-architecture.md)
* [`kafkadesk-data-model.md`](./kafkadesk-data-model.md)
* [`kafkadesk-api-contracts.md`](./kafkadesk-api-contracts.md)
* [`../archive/reference/traceforge-design-system.md`](../archive/reference/traceforge-design-system.md)

# KafkaDesk Desktop Architecture

KafkaDesk is a local desktop workbench. The current implementation is a Tauri app with a React renderer and a Rust runtime in `src-tauri`.

## What exists now

1. The desktop shell is Tauri v2.
2. The renderer lives under `src/` and uses React Router for the main workbench pages.
3. The privileged runtime lives under `src-tauri/` and registers Tauri commands for audit, bookmarks, clusters, correlation rules, groups, messages, preferences, replay, replay policy, saved queries, schema registry, topics, and trace.
4. App startup creates the KafkaDesk app data directory, opens `traceforge.sqlite3`, runs SQL migrations, and recovers stale replay publishing jobs.

## Key boundaries

* The renderer talks to the runtime through `invokeCommand` in [`src/lib/tauri.ts`](../../src/lib/tauri.ts).
* UI code does not open Kafka connections directly.
* Kafka access, schema registry calls, persistence, and secret resolution stay in Rust services and repositories.
* SQLite stores local metadata and workflow records. The runtime is designed to keep secret material out of plain SQLite fields and prefer OS-backed secret storage where the active workflow supports it.

## Important implementation notes

* KafkaDesk does not run a separate sidecar service or local HTTP server today. The service boundary is logical, not a separate process.
* The current route map covers Overview, Topics, Topic Detail, Groups, Group Detail, Messages, Message Detail, Replay, Trace, Saved Queries, Audit, and Settings.
* Running the frontend outside the Tauri shell is supported for UI work, but runtime backed calls fail with `runtime.unavailable` until the app is launched through Tauri.
* Replay is handled as a persisted workflow with job records and audit records, not as a direct fire and forget write.
* Credential-backed validation coverage is still uneven across some secured Kafka and Schema Registry paths, so operator-facing validation docs and settings flows should be read as truthful current limits rather than complete secret-management coverage.

## Known limits

* KafkaDesk is still a single user local app.
* There is no shared backend, sync service, or remote control plane.
* The frontend and runtime use request and response calls only. There is no event streaming channel in active use.

## Related docs

* [`kafkadesk-tech-stack.md`](./kafkadesk-tech-stack.md)
* [`kafkadesk-data-model.md`](./kafkadesk-data-model.md)
* [`kafkadesk-api-contracts.md`](./kafkadesk-api-contracts.md)
* [`../archive/reference/traceforge-design.md`](../archive/reference/traceforge-design.md)

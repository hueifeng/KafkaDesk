# KafkaDesk API Contracts

KafkaDesk uses typed Tauri commands as the boundary between the React renderer and the Rust runtime.

## What exists now

The frontend calls `invokeCommand` in [`src/lib/tauri.ts`](../../src/lib/tauri.ts), which wraps `@tauri-apps/api/core` `invoke` and normalizes runtime errors.

Current command groups in `src-tauri/src/lib.rs` are:

* audit
* bookmarks
* clusters
* correlation
* groups
* messages
* preferences
* replay
* replay policy
* saved queries
* schema registry
* topics
* trace

Representative command names include:

* `list_clusters`, `get_cluster_profile`, `create_cluster_profile`, `update_cluster_profile`, `test_cluster_connection`
* `list_topics`, `get_topic_detail`
* `list_groups`, `get_group_detail`
* `query_messages`, `get_message_detail`
* `create_replay_job`, `list_replay_jobs`, `get_replay_job`
* `run_trace_query`

## Key boundaries

* This is not an HTTP API. The contract is local to the desktop app.
* Most frontend feature modules pass a typed `request` object into a Tauri command and expect a typed DTO or array back.
* Success results are returned directly, not wrapped in a generic envelope.
* Errors are normalized to `category`, `code`, `message`, optional `details`, and optional `retriable` fields.

## Important implementation notes

* The active error categories exposed to the frontend are `validation_error`, `config_error`, `connectivity_error`, `auth_error`, `tls_error`, `timeout_error`, `unsupported_feature`, and `internal_error`.
* Rust maps internal failures into an `AppErrorDto`, then the frontend wrapper preserves that shape when possible.
* Replay already uses a job style contract. The UI creates a replay job, then reads job summaries and details from persisted state.
* Trace is still a direct request and response call, not a background job protocol.
* There is no active event subscription layer for routine data updates.

## Known limits

* TypeScript and Rust contract types are hand maintained. There is no generated shared schema.
* The contract surface is domain oriented, but it is still Tauri specific.
* List responses currently come back as plain arrays or DTOs. There is no common pagination envelope in active use.

## Related docs

* [`kafkadesk-desktop-architecture.md`](./kafkadesk-desktop-architecture.md)
* [`kafkadesk-tech-stack.md`](./kafkadesk-tech-stack.md)
* [`kafkadesk-data-model.md`](./kafkadesk-data-model.md)
* [`../archive/planning/traceforge-mvp-plan.md`](../archive/planning/traceforge-mvp-plan.md)

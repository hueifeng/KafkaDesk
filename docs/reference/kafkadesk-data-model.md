# KafkaDesk Data Model

KafkaDesk persists local operator state in SQLite and is designed to keep secret material out of the database when the active workflow supports that boundary.

## What exists now

The active SQLite schema is defined by migrations under `src-tauri/migrations/`.

| Table | Purpose |
| --- | --- |
| `cluster_profiles` | Kafka cluster connection metadata, auth mode, TLS file paths, favorites, archive state |
| `schema_registry_profiles` | Schema Registry endpoint metadata and credential references |
| `app_preferences` | Local UI and operator preferences |
| `replay_jobs` | Durable replay requests and outcomes |
| `replay_job_events` | Replay job progress history |
| `audit_events` | Durable audit trail for sensitive actions |
| `correlation_rules` | Trace correlation rules |
| `message_bookmarks` | Saved references to specific messages |
| `saved_queries` | Reusable local investigations |

## Key boundaries

* SQLite stores configuration metadata, saved workflows, replay records, and audit history.
* Raw Kafka and Schema Registry secrets should not live in SQLite by default.
* Cluster rows store credential references such as `auth_credential_ref`, not the secret values themselves.
* TLS certificate and key locations are stored as filesystem paths in `cluster_profiles`.
* Kafka message payloads are not mirrored into local tables as a general cache or warehouse.

## Important implementation notes

* The SQLite file is created in the KafkaDesk app data directory as `traceforge.sqlite3`.
* `cluster_profiles` is the main configuration table and now includes auth material references plus TLS certificate path fields from migration `0006_cluster_auth_materialization.sql`.
* Replay has two durable layers, `replay_jobs` for current state and `replay_job_events` for per step history.
* Audit is separate from replay so the app can record sensitive operations in a queryable stream.
* `saved_queries`, `message_bookmarks`, and `correlation_rules` are local productivity features. They are not shared across machines by the current implementation.
* A stored credential reference is not the same thing as complete secret-backed validation coverage across every secured runtime path today.

## Known limits

* There is no `query_history` table in the active schema.
* There are no dedicated cache tables for decoded payloads, topic snapshots, or trace results.
* Schema evolution is migration driven. There is no extra schema registry for the local SQLite model beyond the checked in SQL files.

## Related docs

* [`kafkadesk-desktop-architecture.md`](./kafkadesk-desktop-architecture.md)
* [`kafkadesk-tech-stack.md`](./kafkadesk-tech-stack.md)
* [`kafkadesk-api-contracts.md`](./kafkadesk-api-contracts.md)
* [`../archive/planning/traceforge-mvp-plan.md`](../archive/planning/traceforge-mvp-plan.md)

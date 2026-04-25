# Proposal: add-kafka-operations-console

## Why

KafkaDesk already covers inspection, trace, and controlled replay, but it still stops short of the operational actions engineers use daily when handling live Kafka systems.

Real-world workflows frequently require operators to:

- inspect and modify Topic configuration such as retention and cleanup policy
- increase Topic partition count when throughput or parallelism needs change
- reset consumer group progress to earliest, latest, timestamp, or explicit offsets
- apply operational tags to clusters, Topics, and consumer groups for filtering and change control
- understand Topic-level production and consumption traffic over time
- apply Topic-level throughput controls or quota-like guardrails where the connected Kafka platform supports them

These actions are high-value, but they are also version-sensitive, permission-sensitive, and potentially destructive. KafkaDesk needs a capability-aware management layer before it can safely move from a read-heavy workbench to a trusted Kafka operations console.

## What Changes

This proposal adds a new Kafka operations capability set to KafkaDesk with the following scope:

1. Topic administration
   - view and edit selected Topic configuration values
   - expand Topic partition count with validation and preview
2. Consumer group administration
   - reset consumer offsets using safe reset modes
3. Governance and organization
   - add tags to clusters, Topics, and consumer groups for filtering, grouping, and operational workflows
4. Traffic observability
   - display Topic production and consumption traffic summaries and detail views
5. Throughput control
   - expose Topic-level rate limiting / throttling controls where the cluster supports them
6. Capability safety model
   - detect cluster/version/platform support before enabling write actions
   - return explicit unsupported results instead of optimistic or partially broken flows
   - require confirmations, previews, and audit records for destructive operations

### Current implementation status

The first implemented slice is focused on safe Topic configuration management:

- Topic configuration inspection and editable allowlist support are implemented for selected keys
- Topic configuration updates require an explicit current-value snapshot and risk acknowledgement
- successful writes return applied/audit feedback, including warning states when verification or audit persistence is partial
- unsupported or unavailable capabilities are surfaced truthfully in the Topic operations overview

The remaining operations-console capabilities are still pending: Topic partition expansion execution, consumer offset reset execution, tag persistence/CRUD, traffic visibility, and Topic-level throttling or quota controls.

## Impact

### Product impact

- KafkaDesk moves beyond inspection and replay into operational Kafka management
- write actions become first-class UI workflows rather than hidden backend utilities
- unsupported cluster/version combinations remain visible but gated

### Technical impact

- new backend command groups for Topic administration, consumer offset reset, tagging, traffic stats, and rate limiting
- stronger capability detection and compatibility reporting across Kafka versions and distributions
- more SQLite persistence for local tags, audit history, and saved management views
- expanded release/test matrix for destructive-action safeguards and compatibility behavior

### Risk impact

- offset reset, Topic config changes, partition changes, and throttling are destructive or high-risk operations
- Kafka version and platform differences must be modeled explicitly
- all write paths need preview, confirmation, audit, and unsupported-feature reporting

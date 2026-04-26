# ADR: Defer Persistent Consumer Offset Reset FFI

## Status

Accepted

## Context

KafkaDesk now exposes several guarded Kafka operations: selected Topic configuration updates, Topic partition expansion, and local Topic / Consumer Group tags. Consumer Offset Reset remains the most destructive outstanding operations-console workflow because it changes committed offsets for a consumer group and can cause consumers to skip or replay data.

The repository currently uses `rdkafka` 0.39.0 and `rdkafka-sys` 4.10.0+2.12.1. Local source inspection confirms:

- `rdkafka` 0.39.0 exposes safe consumer `seek` / `seek_partitions` APIs for the current consumer instance
- `rdkafka` 0.39.0 does not expose safe `AdminClient` wrappers for arbitrary consumer-group offset list / alter operations
- `rdkafka-sys` exposes raw `rd_kafka_ListConsumerGroupOffsets_*` and `rd_kafka_AlterConsumerGroupOffsets_*` FFI symbols

Using those raw symbols would require KafkaDesk to own a new unsafe admin surface for a persistent, destructive operation.

## Decision

KafkaDesk defers persistent Consumer Offset Reset execution until one of these conditions is met:

1. the project upgrades to a Kafka client binding with safe consumer-group offset list / alter wrappers, or
2. a separately reviewed FFI shim design is accepted with explicit memory-lifetime, result-parsing, active-group, and rollback/error semantics.

Until then, KafkaDesk may continue to show read-only offset reset prechecks and candidate discovery, but it must not expose a misleading reset execution button or claim that committed offsets can be safely modified.

## Consequences

### Positive

- avoids introducing broad unsafe FFI around a destructive Kafka admin operation
- keeps current offset reset UI truthful as precheck / preview only
- prevents accidental committed-offset mutation while group active-state handling is incomplete

### Negative

- operators still need external Kafka tooling for persistent consumer-group offset reset
- the operations-console proposal remains partially open
- a future implementation requires either dependency upgrade work or a dedicated FFI safety review

## Follow-up

Before implementing persistent reset, define:

1. group active-state requirements and how KafkaDesk proves a group is empty / inactive
2. preview payload shape for affected group, topic, partitions, current offsets, target offsets, and watermarks
3. per-partition result parsing and partial-failure semantics
4. audit record content for successful, failed, and partially applied reset attempts
5. dependency upgrade versus FFI shim decision

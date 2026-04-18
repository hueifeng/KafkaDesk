# Archived: KafkaDesk MVP Plan v0.1

## Status

- Stage at creation time: pre-implementation execution plan
- Current role: historical planning reference
- Product form assumption: desktop-first with local embedded service runtime

This document captures the original MVP framing used to start the project. It is useful for historical scope decisions, but it is not the current source of truth for product status.

For current product-facing documentation, start with [`../../../README.md`](../../../README.md) and [`../../README.md`](../../README.md).

---

## 1. MVP Goal

The MVP should prove one thing clearly:

> an engineer can debug Kafka message flow faster with KafkaDesk than with today’s fragmented combination of CLI, admin UI, and lag dashboards.

The MVP does not need to prove everything.

It needs to prove the core value loop.

---

## 2. MVP Scope

The MVP should include only the capabilities required to support the primary debugging loop.

## 2.1 In Scope

1. desktop shell startup
2. local service runtime
3. cluster connection management
4. topic list and topic detail
5. consumer group list and group detail with lag summary
6. bounded message browser
7. message detail inspector
8. JSON decode baseline
9. basic replay to controlled target or sandbox topic

## 2.2 Optional-if-Time-Permits

1. schema registry integration
2. Avro decode
3. Protobuf decode
4. saved queries
5. simple trace-by-key lookup
6. controlled consumer offset reset workflow for explicit group/topic/partition targets

## 2.3 Out of Scope for MVP

1. full graph-based trace explorer
2. central team sync
3. shared audit backend
4. plugin ecosystem
5. multi-broker support beyond Kafka-compatible systems
6. advanced governance or approval flows
7. destructive broker operations presented as routine actions
8. record deletion or delete-records/truncation workflows as part of the normal debugging loop

---

## 3. Recommended MVP Stack Direction

This plan assumes:

- **desktop shell**: Tauri preferred
- **UI**: web-tech frontend rendered inside desktop shell
- **local service**: separate local runtime process or strongly isolated local backend module
- **metadata store**: SQLite
- **secret storage**: OS keychain or secure local credential storage where possible

This is enough to start implementation without prematurely hardening for enterprise platform complexity.

---

## 4. MVP Success Criteria

The MVP is successful if a user can:

1. configure a Kafka connection
2. open a topic and browse bounded messages
3. inspect a consumer group and see lag detail
4. open a message and understand its payload and headers
5. replay a selected message to a safe destination

If those five tasks feel good, the product is already useful.

---

## 5. Phase Plan

## Phase 0: Foundations

### Objective

Create the minimum runtime skeleton.

### Deliverables

- desktop shell bootstraps correctly
- local service bootstraps correctly
- local metadata DB initializes
- connection configuration can be stored and loaded

### Acceptance Criteria

- app launches from clean install
- service lifecycle is stable
- at least one cluster profile can be created and tested

---

## Phase 1: Cluster and Topic Visibility

### Objective

Give the user immediate cluster orientation and topic access.

### Deliverables

- overview page skeleton
- topics list page
- topic detail page
- partition metadata display

### Acceptance Criteria

- user can list topics
- user can open topic detail
- user can see partition-related metadata clearly

---

## Phase 2: Group and Lag Workflows

### Objective

Make KafkaDesk useful for consumer lag diagnosis.

### Deliverables

- groups list page
- group detail page
- lag summary and partition-level lag display
- groundwork for a later controlled offset-management workflow can be identified in group detail without expanding MVP scope

### Acceptance Criteria

- user can identify a lagging group
- user can drill from group to affected topic/partition context

---

## Phase 3: Message Inspection Core

### Objective

Deliver the core message debugging experience.

### Deliverables

- bounded message query interface
- results table
- message detail page
- raw payload and decoded JSON views
- header and metadata display

### Acceptance Criteria

- user can inspect a specific message without leaving the app
- payload readability is good enough for real debugging

---

## Phase 4: Safe Replay

### Objective

Add the first operational write workflow.

### Deliverables

- replay wizard
- replay job result state
- local audit trail for replay

### Acceptance Criteria

- user can replay a chosen message to a controlled target
- app makes risk and target explicit before execution
- replay is clearly separated from destructive broker operations such as record deletion or truncation

---

## Phase 5: Quality Pass

### Objective

Polish the product to feel like a serious tool rather than a prototype.

### Deliverables

- visual consistency pass
- empty/error/loading states
- keyboard and interaction cleanup
- install/run instructions

### Acceptance Criteria

- product feels coherent
- major workflows are legible and stable
- docs still match the implemented behavior

---

## 6. Work Breakdown by Layer

## 6.1 Desktop Shell

- app boot
- window lifecycle
- shell navigation host
- secure bridge to local service

## 6.2 Local Service

- cluster config model
- query orchestration
- lag analysis orchestration
- replay execution and audit

## 6.3 Connectors

- Kafka connectivity
- metadata reads
- bounded message reads
- produce path for replay

## 6.4 UI

- shell layout
- tables
- inspectors
- replay wizard
- state handling for loading/error/empty

## 6.5 Storage

- local SQLite schema
- saved profile model
- replay job table
- audit record table

---

## 7. Validation Strategy

Before calling the MVP ready, validate with realistic usage scenarios:

1. find a known message in a topic by time window
2. inspect a lagging consumer group
3. inspect a message’s headers and payload
4. replay a message to a sandbox destination

If any of these workflows feels clumsy, the MVP is not done yet.

---

## 8. Risks During MVP Delivery

## Risk A: Doing Too Much Before Core Flows Feel Good

### Mitigation

Prioritize quality of the main workflows over breadth of features.

## Risk B: Overbuilding Trace Too Early

### Mitigation

Keep trace lightweight or defer it until inspection and replay are strong.

## Risk C: UI Looks Generic

### Mitigation

Treat the design-system rules as implementation requirements, not decoration advice.

## Risk D: Local Runtime Gets Messy

### Mitigation

Keep UI and local service boundaries explicit from the beginning.

---

## 9. Exit Criteria for “Docs Complete, Ready to Code”

KafkaDesk is ready to move from documentation to implementation when:

1. product form is agreed
2. architecture direction is agreed
3. screen structure is agreed
4. visual language is agreed
5. MVP scope is agreed

This document, together with the other docs in `docs/`, is intended to satisfy that threshold.

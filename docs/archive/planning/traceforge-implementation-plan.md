# Archived: KafkaDesk Implementation Plan v0.1

## Status

- Stage at creation time: pre-coding execution plan
- Current role: historical implementation-planning reference
- Product form: desktop-first
- Companion docs:
  - [`traceforge-mvp-plan.md`](./traceforge-mvp-plan.md)
  - [`../../reference/kafkadesk-tech-stack.md`](../../reference/kafkadesk-tech-stack.md)
  - [`../../reference/kafkadesk-data-model.md`](../../reference/kafkadesk-data-model.md)
  - [`../../reference/kafkadesk-api-contracts.md`](../../reference/kafkadesk-api-contracts.md)
  - [`../reference/traceforge-screen-specs.md`](../reference/traceforge-screen-specs.md)

This document records the original slice sequencing and implementation assumptions. It should be read as planning history, not as the current release/status page for the repository.

For current product-facing documentation, start with [`../../../README.md`](../../../README.md) and [`../../README.md`](../../README.md).

---

## 1. Goal

This document converts the KafkaDesk design package into an implementation order.

It answers:

1. In what sequence should the project be built?
2. Which modules should exist first?
3. Which features can be stubbed vs must be real immediately?
4. What should each milestone prove?
5. What validation gates should block progress?

The purpose is to avoid starting implementation with a large undifferentiated backlog and no architectural discipline.

---

## 2. Implementation Principles

## 2.1 Workflow Value Before Feature Breadth

The implementation should first make one debugging workflow feel real.

It should not try to make every screen partially real at once.

## 2.2 Boundaries Before Features

The shell, local runtime boundary, and data model should be established early.

If boundaries are sloppy at the beginning, every later feature becomes harder.

## 2.3 Real Connectivity Early

Kafka connectivity should be validated early.

Do not spend weeks perfecting UI shells before proving the desktop runtime can actually talk to a cluster.

## 2.4 Visible Quality Gates

Each phase should end with a concrete validation checkpoint.

If the checkpoint fails, do not continue pretending the next phase is unblocked.

## 2.5 Avoid Accidental Platform Work

KafkaDesk is a product, not a framework.

Only generalize when the design package explicitly requires it.

---

## 3. Build Strategy

## 3.1 Recommended Strategy

Build KafkaDesk as a series of **vertical slices** over a stable runtime skeleton.

That means:

1. establish shell and runtime foundation
2. implement one complete useful path
3. expand into adjacent workflows
4. polish and harden only after the core loop is real

## 3.2 Vertical Slice Definition

A valid slice should usually include:

- data model support
- local service logic
- API/command contract
- UI screen or region
- validation

### Example

“Topic list” is not just a page.

It is:

- cluster selection context
- topic query contract
- local runtime topic service
- topic table UI
- loading/error/empty states

---

## 4. What Is Real vs Stubbed Early

## 4.1 Must Be Real Early

These should be real as early as possible:

- Tauri shell boot
- local runtime command boundary
- SQLite initialization
- cluster profile persistence
- Kafka connection test
- topic list fetch
- bounded message query

## 4.2 Can Be Stubbed Briefly

These can be skeletal early if necessary:

- overview page metrics
- saved queries
- bookmarks
- advanced trace view modes
- audit detail expansion

### Rule

Stubs are acceptable only when they do not block the primary debugging loop.

---

## 5. Phase Plan

## Phase 0: Project Bootstrap

### Objective

Create the codebase foundation with the agreed stack.

### Deliverables

- Tauri v2 project initialized
- React + TypeScript + Vite frontend initialized
- Rust core structure created under `src-tauri`
- base repo structure aligned with `kafkadesk-tech-stack.md`
- design tokens and basic shell styles scaffolded

### Module Work

- app shell layout
- Tauri command module skeleton
- Rust service layer skeleton
- storage module skeleton

### Validation Gate

- app boots successfully
- frontend renders inside desktop shell
- one sample typed command round-trip works end to end

---

## Phase 1: Local Configuration Foundation

### Objective

Make the desktop app remember cluster-related local state.

### Deliverables

- SQLite initialization and migrations
- `cluster_profiles` schema
- `schema_registry_profiles` schema
- `app_preferences` schema
- settings screen skeleton
- create/edit/test cluster profile flow

### Required Commands

- `list_clusters`
- `create_cluster_profile`
- `update_cluster_profile`
- `test_cluster_connection`

### Validation Gate

- a user can create a cluster profile
- a user can persist and reload it after app restart
- connection test returns success/failure predictably

---

## Phase 2: Topics Vertical Slice

### Objective

Deliver the first real broker-backed browsing workflow.

### Deliverables

- topics list screen
- topic detail screen
- topic-related contracts
- Kafka metadata read path

### Required Commands

- `list_topics`
- `get_topic_detail`

### Data/Storage Work

- no new major tables required beyond cluster config

### UI Work

- filter bar
- topics table
- topic header and partition table

### Validation Gate

- user can open a cluster and list topics
- topic list respects filters
- topic detail loads real partition data

---

## Phase 3: Groups and Lag Vertical Slice

### Objective

Make KafkaDesk useful for lag diagnosis.

### Deliverables

- groups list screen
- group detail screen
- lag summary logic
- partition-level lag breakdown

### Required Commands

- `list_groups`
- `get_group_detail`

### Validation Gate

- user can identify lagging groups
- user can drill into lag by topic and partition
- state and lag values are coherent and trustworthy

---

## Phase 4: Message Inspection Vertical Slice

### Objective

Deliver the core product value loop.

### Deliverables

- messages screen
- message query builder
- bounded message query logic
- message detail screen
- raw payload and JSON decode baseline

### Required Commands

- `query_messages`
- `get_message_detail`

### Data/Storage Work

- optional minimal query-history support can begin here if useful

### Validation Gate

- user can run a bounded query
- user can open a real message
- message detail is readable and useful

---

## Phase 5: Replay Vertical Slice

### Objective

Add the first sensitive operational workflow.

### Deliverables

- replay wizard UI
- replay job creation logic
- replay job persistence
- local audit event persistence
- replay result screen or detail view

### Required Tables

- `replay_jobs`
- `audit_events`

### Required Commands

- `create_replay_job`
- `list_replay_jobs`
- `get_replay_job`

### Validation Gate

- user can replay a message to a safe target
- replay status is visible and durable
- audit records are created correctly

---

## Phase 6: Product Quality Pass

### Objective

Make the app feel like a coherent tool rather than a stack demo.

### Deliverables

- design-system consistency pass
- loading/empty/error state completion
- keyboard and interaction cleanup
- route/state preservation cleanup
- performance pass on key screens

### Validation Gate

- the main workflows feel smooth and visually coherent
- no obvious broken-state transitions remain

---

## Phase 7: Optional MVP+ Enhancements

### Objective

Add the most valuable adjacent workflows after the core loop is proven.

### Candidate Deliverables

- Avro decode
- schema registry integration
- saved queries
- bookmarks
- simple trace-by-key
- controlled consumer offset reset workflow with preview, confirmation, and audit support
- admin-only destructive broker operations such as delete-records/truncation, explicitly deferred behind high-risk safeguards

### Rule

Do not begin this phase until the core inspection + replay loop already feels solid.

---

## 6. Module-by-Module Build Order

## 6.1 Frontend Modules

Recommended order:

1. shell
2. settings/cluster profiles
3. topics list/detail
4. groups list/detail
5. messages screen
6. message detail
7. replay wizard
8. audit / saved queries / trace

## 6.2 Rust Core Modules

Recommended order:

1. command boundary
2. config/profile service
3. SQLite repository layer
4. Kafka connector
5. topic service
6. group service
7. message query service
8. replay service
9. audit service
10. trace/correlation service

## 6.3 Storage Modules

Recommended order:

1. migrations
2. cluster profile repository
3. preferences repository
4. replay job repository
5. audit repository
6. saved query repository
7. correlation rule repository

---

## 7. Milestone Definition

## Milestone A: App Is Real

Criteria:

- desktop app launches
- local runtime works
- typed command boundary works

## Milestone B: Cluster Browsing Is Real

Criteria:

- cluster profiles persist
- topics load from Kafka
- groups and lag views work

## Milestone C: Debugging Value Is Real

Criteria:

- bounded message browsing works
- message detail is useful

## Milestone D: Controlled Action Is Real

Criteria:

- replay and other controlled write/admin actions work through explicit safe flows
- audit state is persisted

## Milestone E: Product Feels Cohesive

Criteria:

- UI quality and consistency are strong
- empty/loading/error states are complete
- the product feels intentional, not improvised

---

## 8. Validation Gates

Every phase should end with a validation gate.

## 8.1 Technical Gates

- app boots
- commands compile and respond predictably
- migrations apply on fresh and existing local DBs
- Kafka connectivity works against a real environment

## 8.2 Product Gates

- the target workflow can be completed without CLI fallback
- the UI is understandable without engineer-only tribal knowledge

## 8.3 Quality Gates

- no obviously broken layout states on target desktop sizes
- loading/error states exist for new surfaces
- dangerous actions remain explicit and bounded
- offset changes and destructive broker actions require preview, confirmation, and auditability

---

## 9. Suggested Sprint Grouping

If work is done in iterative chunks, a practical grouping is:

### Sprint 1

- Phase 0 + Phase 1

### Sprint 2

- Phase 2

### Sprint 3

- Phase 3

### Sprint 4

- Phase 4

### Sprint 5

- Phase 5

### Sprint 6

- Phase 6 + selected MVP+ work if justified

This is not a calendar commitment, only a planning shape.

---

## 10. What Not to Do During Implementation

1. do not start from a generic desktop admin template
2. do not implement trace before message inspection feels good
3. do not over-abstract the local runtime too early
4. do not let UI call into storage or Kafka logic directly
5. do not store raw secrets in SQLite
6. do not turn replay into a casual one-click action without safeguards
7. do not turn offset changes or destructive broker actions into casual one-click actions without safeguards

---

## 11. Exit Criteria for “Ready to Start Coding”

Implementation can begin once the team accepts that:

1. product shape is fixed enough
2. runtime architecture is fixed enough
3. technical stack is fixed enough
4. data model is fixed enough
5. command contracts are fixed enough
6. screen behavior is fixed enough

The current KafkaDesk docs package is intended to meet that threshold.

---

## 12. Final Recommendation

The implementation should start with **foundations + one real vertical slice**, not with parallel half-finished surfaces.

The most important early success is not “many files changed.”

It is:

> **a user can open KafkaDesk, connect to Kafka, inspect a real topic, inspect a real group, and then inspect a real message through a coherent desktop experience.**

If that happens early, the rest of the product can grow on solid ground.

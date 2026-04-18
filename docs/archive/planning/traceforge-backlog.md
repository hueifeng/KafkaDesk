# Archived: KafkaDesk Backlog v0.1

## Status

- Stage at creation time: pre-coding execution backlog
- Product form: desktop-first
- Scope at creation time: MVP-first
- Companion docs:
  - [`traceforge-mvp-plan.md`](./traceforge-mvp-plan.md)
  - [`traceforge-implementation-plan.md`](./traceforge-implementation-plan.md)
  - [`../../reference/kafkadesk-tech-stack.md`](../../reference/kafkadesk-tech-stack.md)
  - [`../reference/traceforge-screen-specs.md`](../reference/traceforge-screen-specs.md)

This document preserves the original backlog framing used during project bootstrap. It is useful as historical planning context, but the current implementation and maturity truth now lives in the active repository and public docs.

For current product-facing documentation, start with [`../../../README.md`](../../../README.md) and [`../../README.md`](../../README.md).

---

## 1. Goal

This backlog turns the KafkaDesk design and implementation plan into executable work items.

It is intended to answer:

1. What are the MVP epics?
2. What stories belong to each epic?
3. What should be built first?
4. What blocks what?
5. What counts as done for each slice?

The backlog is intentionally **product-first and implementation-aware**, not a generic issue dump.

---

## 2. Backlog Principles

### 2.1 One Real Slice Beats Five Fake Slices

Each epic should move KafkaDesk closer to a working debugging workflow.

### 2.2 Prioritize User-Visible Capability Over Internal Vanity

Internal structure matters, but only where it unlocks a real product slice.

### 2.3 Every Story Should Have a Validation Outcome

If a story cannot be verified in behavior, it is not ready.

### 2.4 Respect the Product Boundaries

Do not let backlog growth drag the MVP into:

- full observability platform work
- enterprise governance suite work
- speculative plugin platform work

---

## 3. Priority Levels

Use three priority levels:

- **P0** — required for MVP and blocks core workflow progress
- **P1** — important for MVP quality or adjacent usability
- **P2** — valuable but deferrable beyond MVP core

---

## 4. Epic Overview

## Epic A — Desktop Foundation

**Goal:** make KafkaDesk boot and support a stable shell/runtime boundary.

## Epic B — Local Configuration and Connection Setup

**Goal:** let the app store cluster profiles and test Kafka connectivity.

## Epic C — Topic Browsing

**Goal:** allow users to browse real topic metadata.

## Epic D — Group and Lag Diagnosis

**Goal:** allow users to inspect consumer groups and lag.

## Epic E — Message Inspection

**Goal:** allow users to run bounded message queries and inspect real messages.

## Epic F — Replay Workflow

**Goal:** allow users to safely replay a selected message to a controlled target.

## Epic G — Product Quality and UX Completion

**Goal:** make the product feel coherent, polished, and trustworthy.

## Epic H — MVP+ Extensions

**Goal:** add adjacent value after the core loop is already real.

---

## 5. Detailed Backlog

## Epic A — Desktop Foundation

### A1. Initialize Tauri + React + TypeScript project

- Priority: **P0**
- Depends on: none
- Outcome: app boots in desktop shell with frontend rendering correctly

#### Tasks

- scaffold Tauri v2 app
- scaffold React + TypeScript + Vite app
- verify local dev startup
- establish base folder structure per tech stack doc

#### Definition of Done

- local desktop app launches successfully
- frontend is visible in Tauri shell
- base project structure is committed locally (not git commit, just filesystem state)

---

### A2. Establish UI/runtime command boundary skeleton

- Priority: **P0**
- Depends on: A1
- Outcome: frontend can call one typed command into Rust runtime and receive a typed result

#### Tasks

- add command module structure
- add base DTO serialization setup
- add shared error mapping shape
- wire one smoke-test command

#### Definition of Done

- one command round-trip works end to end
- errors can be surfaced in the UI predictably

---

### A3. Establish design-system scaffolding

- Priority: **P0**
- Depends on: A1
- Outcome: shell styling and design tokens exist before page sprawl starts

#### Tasks

- add CSS variable token layer
- add Tailwind setup
- establish typography, spacing, surface, and status token base
- create initial shell layout primitives

#### Definition of Done

- the app shell reflects the intended visual direction rather than default starter styles

---

## Epic B — Local Configuration and Connection Setup

### B1. Add SQLite bootstrap and migrations

- Priority: **P0**
- Depends on: A2
- Outcome: local persistence exists and is versioned

#### Tasks

- wire SQLite access
- add migration runner
- create initial schema baseline

#### Definition of Done

- app creates/opens local DB successfully
- migrations run on fresh start

---

### B2. Implement cluster profile persistence

- Priority: **P0**
- Depends on: B1
- Outcome: users can create, edit, archive, and reload cluster profiles

#### Tasks

- implement `cluster_profiles` table support
- implement repository/service layer for profiles
- implement settings UI for profile CRUD

#### Definition of Done

- profile survives app restart
- invalid configuration is rejected cleanly

---

### B3. Implement connection test workflow

- Priority: **P0**
- Depends on: B2
- Outcome: users can verify whether a cluster profile actually works

#### Tasks

- implement `test_cluster_connection`
- map connection/auth/config errors
- surface result in settings UI

#### Definition of Done

- success/failure is clearly visible
- failure reason is usable, not opaque

---

### B4. Add schema registry profile support

- Priority: **P1**
- Depends on: B1
- Outcome: registry connection metadata can be configured early even if decode support lands later

#### Definition of Done

- schema registry profile CRUD exists
- profile can be linked to a cluster profile

---

## Epic C — Topic Browsing

### C1. Implement topic listing service and command

- Priority: **P0**
- Depends on: B3
- Outcome: app can fetch topics from a real cluster

#### Tasks

- implement topic metadata read path
- implement `list_topics`
- map topic summaries to UI DTO

#### Definition of Done

- topic list loads from a real cluster profile

---

### C2. Build Topics list screen

- Priority: **P0**
- Depends on: C1, A3
- Outcome: users can browse topics via a real table UI

#### Tasks

- implement filter bar
- implement topics table
- implement topic row actions
- add loading/empty/error states

#### Definition of Done

- topic list screen is usable and visually aligned with the design system

---

### C3. Implement topic detail workflow

- Priority: **P0**
- Depends on: C1
- Outcome: users can inspect partition metadata and related group context

#### Tasks

- implement `get_topic_detail`
- build topic detail screen
- implement partition table and related groups region

#### Definition of Done

- a selected topic opens into a useful detail page with real data

---

## Epic D — Group and Lag Diagnosis

### D1. Implement groups list service and command

- Priority: **P0**
- Depends on: B3
- Outcome: app can fetch group summaries and lag signals

#### Tasks

- implement group summary read path
- implement `list_groups`
- define group state and lag DTOs

#### Definition of Done

- group list reflects real cluster data

---

### D2. Build Groups list screen

- Priority: **P0**
- Depends on: D1, A3
- Outcome: users can find lagging groups quickly

#### Tasks

- implement groups table
- implement lagging-only filter
- implement sort behavior
- add state handling

#### Definition of Done

- users can sort by lag and identify problematic groups quickly

---

### D3. Implement group detail workflow

- Priority: **P0**
- Depends on: D1
- Outcome: users can drill into topic-level and partition-level lag

#### Tasks

- implement `get_group_detail`
- build group detail screen
- connect drill-down into topic/message flows

#### Definition of Done

- user can identify which partitions are behind and move toward message inspection

---

### D4. Add controlled consumer offset reset workflow

- Priority: **P1**
- Depends on: D3, F1
- Outcome: operators can intentionally reset consumer offsets without leaving KafkaDesk for routine diagnosis and recovery workflows

#### Tasks

- implement offset reset command support for earliest/latest/explicit offset/timestamp targets
- add affected partition preview with current vs target offset visibility
- require explicit confirmation before execution
- persist an audit event for each offset change
- expose the action from group detail with clear risk language

#### Definition of Done

- user can reset offsets for an intended scope without ambiguity
- the UI makes affected partitions and target positions explicit before execution
- every offset change is auditable locally

---

## Epic E — Message Inspection

### E1. Implement bounded message query service

- Priority: **P0**
- Depends on: B3
- Outcome: app can query real messages with explicit bounds

#### Tasks

- define bounded query validation
- implement `query_messages`
- implement preview mapping for results

#### Definition of Done

- unbounded reads are rejected
- bounded reads return useful results

---

### E2. Build Messages screen

- Priority: **P0**
- Depends on: E1, A3
- Outcome: users can run bounded queries from the UI

#### Tasks

- build query builder
- build results table
- connect loading/error/empty states

#### Definition of Done

- a user can run a query and understand the result set immediately

---

### E3. Implement message detail retrieval

- Priority: **P0**
- Depends on: E1
- Outcome: users can inspect a single message deeply

#### Tasks

- implement `get_message_detail`
- define decoded/raw/header/meta DTOs
- add JSON decode baseline

#### Definition of Done

- message detail contains enough information to debug a real message

---

### E4. Build Message detail screen

- Priority: **P0**
- Depends on: E3, A3
- Outcome: KafkaDesk’s flagship inspection screen becomes real

#### Tasks

- build tabbed inspector
- build right-side action/context area
- add bookmark and copy behaviors if feasible

#### Definition of Done

- the screen is readable, high-quality, and useful enough to replace ad hoc CLI inspection for common cases

---

## Epic F — Replay Workflow

### F1. Implement replay job persistence and audit base

- Priority: **P0**
- Depends on: B1
- Outcome: replay operations have durable local state and audit records

#### Tasks

- implement `replay_jobs` table support
- implement `audit_events` table support
- add repository/service logic

#### Definition of Done

- replay jobs and audit events persist locally and can be read back

---

### F2. Implement replay runtime command flow

- Priority: **P0**
- Depends on: E3, F1
- Outcome: app can stage and execute a replay job to a safe target

#### Tasks

- implement `create_replay_job`
- implement validation and risk checks
- implement job result/status handling

#### Definition of Done

- replay can succeed or fail with structured outcomes

---

### F3. Build replay wizard UI

- Priority: **P0**
- Depends on: F2, A3
- Outcome: users can safely replay from the UI

#### Tasks

- implement source confirmation step
- implement target selection step
- implement edit step
- implement risk confirmation step
- implement result step

#### Definition of Done

- replay is explicit, auditable, and not a casual blind action

---

## Epic G — Product Quality and UX Completion

### G1. Complete loading/empty/error states across all MVP screens

- Priority: **P0**
- Depends on: C2, C3, D2, D3, E2, E4, F3

#### Definition of Done

- every MVP screen has complete state coverage

---

### G2. Apply design-system consistency pass

- Priority: **P0**
- Depends on: most visible MVP screens implemented

#### Tasks

- spacing cleanup
- typography cleanup
- status color consistency
- inspector consistency
- table density consistency

#### Definition of Done

- the product looks like one intentional tool, not stitched screens

---

### G3. Improve route and context preservation

- Priority: **P1**
- Depends on: main screens implemented

#### Definition of Done

- returning from detail views preserves useful context
- cluster switching behaves predictably

---

### G4. Performance and responsiveness pass

- Priority: **P1**
- Depends on: core message/group/topic flows working

#### Definition of Done

- main screens remain responsive under realistic result sizes

---

## Epic H — MVP+ Extensions

### H1. Schema registry integration

- Priority: **P1**
- Depends on: B4, E3

### H2. Avro decode

- Priority: **P1**
- Depends on: H1

### H3. Protobuf decode

- Priority: **P1**
- Depends on: H1

### H4. Saved queries

- Priority: **P1**
- Depends on: E2, D3

### H5. Message bookmarks

- Priority: **P1**
- Depends on: E4

### H6. Simple trace-by-key

- Priority: **P2**
- Depends on: E3, correlation rules base

### H7. Admin-only delete-records / truncation workflow

- Priority: **P2**
- Depends on: F1, D3, E3
- Outcome: destructive broker operations, if ever added, are treated as explicit high-risk admin workflows rather than routine debugging actions

#### Tasks

- define a partition-and-offset-scoped delete-records or truncation action model
- require admin-level gating, explicit preview, and strong confirmation language
- record durable audit events for destructive operations
- avoid presenting the capability as single-message deletion inside the normal message viewer flow

#### Definition of Done

- destructive operations are clearly separated from replay and inspection workflows
- the user must explicitly acknowledge scope and risk before execution
- audit records are created for every destructive action

---

## 6. MVP Critical Path

The MVP critical path is:

1. A1
2. A2
3. A3
4. B1
5. B2
6. B3
7. C1
8. C2
9. D1
10. D2
11. D3
12. E1
13. E2
14. E3
15. E4
16. F1
17. F2
18. F3
19. G1
20. G2

If these items are done well, KafkaDesk has a real MVP.

---

## 7. Suggested First Coding Sprint

If you want to begin coding immediately, the first concrete sprint should include only:

### Sprint 1 Scope

- A1
- A2
- A3
- B1
- B2
- B3

### Sprint 1 Success Condition

A user can:

- launch the app
- create a cluster profile
- save it
- test a connection

This is the right first checkpoint because it proves:

- shell works
- runtime works
- persistence works
- Kafka connectivity works

---

## 8. Exit Criteria per Epic

An epic should only be considered complete if:

1. the core user story works end-to-end
2. loading/error/empty states exist
3. the slice follows the design system
4. the slice follows the data and contract docs
5. the result is demonstrably usable, not just technically wired

---

## 9. What Can Be Deferred Without Hurting MVP

Safe to defer:

- graph-heavy trace visualizations
- advanced cache layers
- central shared-mode sync
- richer audit detail drill-down
- advanced personalization
- plugin architecture

These should not compete with the MVP critical path.

---

## 10. Final Recommendation

Yes — once this backlog is accepted together with the other docs, **KafkaDesk is ready to begin implementation**.

The right way to start is not “build everything.”

It is:

> **execute the backlog from Desktop Foundation → Local Configuration → Topics → Groups → Messages → Replay → Quality Pass**

That sequence gives the project the highest chance of becoming a real tool quickly instead of a half-finished concept.

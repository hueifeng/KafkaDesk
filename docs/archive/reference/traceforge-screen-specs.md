# KafkaDesk Screen Specifications v0.1

> Historical reference: this document preserves early screen expectations and may not fully match the current shipped UI.

## Status

- Stage: pre-implementation page specification
- Product form: desktop-first
- Companion docs:
  - [`traceforge-product-design.md`](./traceforge-product-design.md)
  - [`traceforge-wireframes.md`](./traceforge-wireframes.md)
  - [`traceforge-design-system.md`](./traceforge-design-system.md)
  - [`../../reference/kafkadesk-api-contracts.md`](../../reference/kafkadesk-api-contracts.md)

---

## 1. Goal

This document turns the wireframes into implementation-grade screen specifications.

It answers:

1. What regions does each screen have?
2. What data appears in each region?
3. What filters, tables, tabs, and actions are required?
4. What empty, loading, and error states should exist?
5. What are the minimum implementation expectations for each page?

The purpose is to reduce ambiguity before frontend implementation starts.

---

## 2. Global Shell Specification

## 2.1 Persistent Shell Regions

All primary screens should render inside a stable application shell with these regions:

1. **Left Navigation Rail**
2. **Top Context Header**
3. **Main Page Canvas**
4. **Optional Right Inspector / Overlay Region**

### Left Navigation Rail

Required navigation items:

- Overview
- Topics
- Groups
- Messages
- Replay
- Trace
- Saved Queries
- Audit
- Settings

### Top Context Header

Required content:

- active cluster selector
- environment badge
- quick search entry
- recent items trigger
- user/app menu

### Shell Rules

- navigation should remain visually stable between pages
- active route should be unmistakable
- cluster context must remain visible at all times

---

## 2.2 Shared UI Behaviors

### Loading States

Every data-loading page should expose:

- initial loading state
- refresh/loading state
- partial content refresh behavior where appropriate

### Empty States

Every page with list/table content should have an actionable empty state.

### Error States

Every page must support:

- inline validation feedback
- page-level recoverable error state
- severe failure fallback state

---

## 3. Overview Screen

## 3.1 Purpose

Help the user orient to a cluster and immediately enter a debugging workflow.

## 3.2 Required Regions

1. page header
2. summary metrics strip
3. cluster health region
4. consumer group health region
5. recent activity region
6. quick actions region

## 3.3 Page Header

### Required Content

- title: `Overview`
- active cluster name
- schema registry connection status
- active environment indicator

### Optional Actions

- refresh overview
- open cluster settings

## 3.4 Summary Metrics Strip

### Required Metrics

- topic count
- group count
- lagging group count
- replay jobs today

### Design Rule

Keep metrics compact and horizontally aligned.

## 3.5 Cluster Health Region

### Minimum Content

- broker count
- connection status
- cluster identifier if available
- basic health summary

## 3.6 Consumer Group Health Region

### Minimum Content

- top lagging groups
- unhealthy/rebalancing groups summary
- partition hotspot hint if available

## 3.7 Recent Activity Region

### Minimum Content

- recent replay jobs
- recent trace queries
- recent saved queries

## 3.8 Quick Actions Region

### Required Actions

- browse messages
- open lagging groups
- run trace query

## 3.9 States

### Loading

- skeleton metrics strip
- skeleton summary panels

### Empty

- if no cluster selected, show cluster setup CTA

### Error

- cluster overview failed → show retry and connection troubleshooting hint

---

## 4. Topics List Screen

## 4.1 Purpose

Allow the user to find a topic quickly and move into message debugging.

## 4.2 Required Regions

1. page header
2. filter bar
3. topics table
4. optional right inspector for topic preview

## 4.3 Filter Bar

### Required Inputs

- text search
- include internal topics toggle
- favorites only toggle
- sort selector

### Optional Inputs

- environment-specific visibility hints later

## 4.4 Topics Table

### Required Columns

- topic name
- partition count
- replication factor
- schema type / schema presence
- retention summary
- activity hint
- favorite indicator

### Required Row Actions

- open topic detail
- toggle favorite
- copy topic name

### Sorting Expectations

Initial useful sorts:

- topic name
- partition count
- activity hint

## 4.5 States

### Empty

- no topics found for current filter

### Error

- unable to load topics

### Loading

- table skeleton rows

---

## 5. Topic Detail Screen

## 5.1 Purpose

Show topic context and launch message browsing.

## 5.2 Required Regions

1. topic header
2. topic summary strip
3. partition table
4. related consumer groups table
5. advanced config section

## 5.3 Topic Header

### Required Content

- topic name
- partition count
- schema summary
- retention summary

### Required Primary Action

- browse messages

## 5.4 Partition Table

### Required Columns

- partition id
- earliest offset
- latest offset
- leader
- replica status
- consumer groups count or summary

### Optional Row Actions

- browse this partition

## 5.5 Related Consumer Groups Table

### Required Columns

- group name
- total lag
- state
- open group action

## 5.6 Advanced Config Section

### Rule

Collapsed by default.

### Content

- selected topic configuration entries useful for debugging or operational context

## 5.7 States

### Empty

- if topic metadata loads but related groups are absent, show explicit “no related groups” state

### Error

- topic not found or broker read failed

---

## 6. Groups List Screen

## 6.1 Purpose

Surface lagging or unhealthy groups quickly.

## 6.2 Required Regions

1. page header
2. filter/sort bar
3. groups table

## 6.3 Filter/Sort Bar

### Required Inputs

- text search
- lagging only toggle
- topic filter (optional initial implementation if data already available)
- sort selector

## 6.4 Groups Table

### Required Columns

- group name
- state
- total lag
- topic count
- partition count
- last seen / last update hint

### Required Sorting

- total lag descending
- group name
- state

### Required Row Action

- open group detail

## 6.5 States

### Empty

- no groups match filters

### Error

- broker read failed or group metadata unavailable

---

## 7. Group Detail Screen

## 7.1 Purpose

Move from lag symptom to partition/topic-level diagnosis.

## 7.2 Required Regions

1. group header
2. group summary strip
3. topic-level lag table
4. partition-level lag table
5. optional group metadata panel

## 7.3 Group Header

### Required Content

- group name
- group state
- total lag

### Required Actions

- save query
- refresh

## 7.4 Topic-Level Lag Table

### Required Columns

- topic name
- total lag
- partitions impacted
- open topic action

## 7.5 Partition-Level Lag Table

### Required Columns

- topic
- partition
- committed offset
- log end offset
- lag
- browse messages action

### Required Action

- open bounded message view for that partition
- open controlled offset reset flow for that group/partition context

## 7.6 States

### Empty

- group exists but no active lag data available

### Error

- group detail fetch failed

## 7.7 Controlled Offset Reset Flow

## 7.7.1 Purpose

Allow an operator to intentionally reset consumer offsets from the lag diagnosis surface.

## 7.7.2 Required Regions

1. scope summary
2. current-versus-target offset preview
3. risk/permission notice
4. confirmation action area

## 7.7.3 Scope Summary

### Required Content

- cluster
- consumer group
- topic/partition targets
- reset mode (earliest / latest / explicit offset / timestamp)

## 7.7.4 Offset Preview

### Required Content

- current committed offset
- target offset
- lag delta impact
- affected partitions

## 7.7.5 Confirmation Action Area

### Required Content

- explicit acknowledgement control
- permission or admin warning when relevant
- audit notice

### Required Actions

- preview reset
- confirm reset
- cancel

### Design Notes

- this flow belongs next to group detail, not message detail
- it should feel intentional, bounded, and auditable
- destructive broker operations such as record deletion/truncation must not appear as normal message actions here

---

## 8. Messages Screen

## 8.1 Purpose

Provide a direct bounded message query surface.

## 8.2 Required Regions

1. page header
2. query builder region
3. results table
4. optional right-side quick inspector

## 8.3 Query Builder

### Required Inputs

- cluster (implicitly from global selection or explicit display)
- topic
- partitions (optional)
- time range and/or offset range
- key filter
- header filter(s)
- max results

### Required Actions

- run query
- reset filters

### Validation Rules

- topic required
- query must be bounded
- max results must be capped
- invalid combinations must show inline validation

## 8.4 Results Table

### Required Columns

- timestamp
- partition
- offset
- key preview
- decode status
- payload preview

### Required Row Actions

- open message detail
- open quick inspector (optional if row click already opens detail)

## 8.5 Quick Inspector (Optional MVP Enhancement)

### Minimum Content

- message ref
- key preview
- payload preview
- open full detail action

## 8.6 States

### Pre-query Empty State

- prompt user to run a bounded query

### No-results State

- query succeeded but returned no messages

### Error State

- query rejected or broker read failed

### Loading State

- query in progress indication with visible scope

---

## 9. Message Detail Screen

## 9.1 Purpose

Deliver the highest-quality message inspection experience in the product.

## 9.2 Required Regions

1. header
2. main inspector pane
3. context/action sidebar

## 9.3 Header

### Required Content

- topic
- partition
- offset
- timestamp

### Optional Actions

- bookmark message
- copy message reference

## 9.4 Main Inspector Pane

### Required Tabs

- Decoded
- Raw
- Headers
- Metadata

### Decoded Tab Requirements

- tree or structured payload view
- fold/unfold behavior
- copy actions
- readable nested structure

### Raw Tab Requirements

- raw payload view
- copy raw content

### Headers Tab Requirements

- key/value list
- copy action per header or whole set

### Metadata Tab Requirements

- message reference
- decode status
- schema summary
- topic/partition/offset details

## 9.5 Context / Action Sidebar

### Required Content

- key summary
- schema summary
- trace action
- replay action
- bookmark status

### Required Actions

- replay this message
- run trace using key/header-derived context where possible

### Design Notes

- do not surface record deletion or truncation here
- controlled offset reset belongs to group/lag workflows, not the message inspector

## 9.6 States

### Decode Failure State

- raw payload still visible
- decoded view explains decode failure without blocking other tabs

### Error State

- message not found / read failed

---

## 10. Replay Screen / Wizard

## 10.1 Purpose

Guide a sensitive operational action safely.

## 10.2 Required Steps

1. source confirmation
2. target selection
3. payload/key/header edit
4. risk confirmation
5. execution result

## 10.3 Step 1: Source Confirmation

### Required Content

- source topic/partition/offset
- source timestamp
- source payload preview

## 10.4 Step 2: Target Selection

### Required Inputs

- target topic
- optional target cluster later if supported

### Validation

- target required
- policy restrictions surfaced early if known

## 10.5 Step 3: Edit Step

### Required Inputs

- edited payload
- edited key
- edited headers

### UX Rule

- show clearly whether values are original or modified

## 10.6 Step 4: Risk Confirmation

### Required Content

- environment indicator
- risk level
- target topic summary
- explicit acknowledgement control
- dry-run toggle if available

## 10.7 Step 5: Execution Result

### Required Content

- job status
- success/failure summary
- audit reference if available
- link to replay history or job detail

## 10.8 Persistent Sidebar / Summary

The replay screen should keep a persistent summary with:

- source message ref
- target topic
- modified fields summary
- risk level

## 10.9 States

### Validation Errors

- must be inline and step-specific

### Runtime Errors

- must appear in a way that preserves the draft state for retry or correction

---

## 11. Trace Screen

## 11.1 Purpose

Follow a business or technical key across topics.

## 11.2 Required Regions

1. page header
2. trace query form
3. result mode tabs
4. result canvas
5. trace notes/footer

## 11.3 Trace Query Form

### Required Inputs

- key type
- key value
- topic scope
- time range
- result mode preference (optional)

### Required Action

- run trace

### Validation Rules

- key type required
- key value required
- time range required
- query must remain bounded

## 11.4 Result Mode Tabs

### Required Modes

- Timeline
- Table

### Optional Mode

- Graph (if it adds real debugging value)

## 11.5 Timeline Mode

### Required Content

- ordered event list
- time relationship cues
- open message detail action

## 11.6 Table Mode

### Required Columns

- timestamp
- topic
- partition
- offset
- key summary
- correlation note/confidence hint

## 11.7 Notes/Footer Region

### Required Content

- trace scope summary
- inferred correlation warning if applicable
- query confidence notes if applicable

## 11.8 States

### Empty

- no related events found

### Error

- trace query failed

### Loading

- bounded trace query progress hint

---

## 12. Saved Queries Screen

## 12.1 Purpose

Allow users to reuse common investigations.

## 12.2 Required Regions

1. page header
2. filter bar
3. saved queries table

## 12.3 Saved Queries Table

### Required Columns

- query name
- type
- cluster
- owner/profile
- last run
- favorite indicator

### Required Actions

- open/run query
- edit query
- delete query

---

## 13. Audit Screen

## 13.1 Purpose

Provide a reviewable record of sensitive operations.

## 13.2 Required Regions

1. page header
2. filter bar
3. audit table
4. optional audit detail inspector

## 13.3 Filter Bar

### Required Inputs

- event type filter
- outcome filter
- date range filter

## 13.4 Audit Table

### Required Columns

- timestamp
- event type
- target type
- summary
- outcome

### Optional Columns Later

- actor/profile
- cluster

### Required Row Action

- open audit detail

---

## 14. Settings Screen

## 14.1 Purpose

Manage local configuration and product behavior.

## 14.2 Required Sections

1. cluster profiles
2. schema registry profiles
3. app preferences
4. correlation rules
5. replay policy/settings (local policy view)
6. controlled offset-reset policy defaults

## 14.3 Cluster Profiles Section

### Required Fields

- name
- environment
- bootstrap servers
- auth mode
- TLS mode
- linked schema registry profile
- notes

### Required Actions

- create
- edit
- archive
- test connection

## 14.4 Schema Registry Profiles Section

### Required Fields

- name
- base URL
- auth mode

### Required Actions

- create
- edit
- test connection

## 14.5 App Preferences Section

### Required Settings

- default query window
- table density
- preferred trace view
- preferred cluster

## 14.6 Correlation Rules Section

### Required Fields

- rule name
- cluster scope
- strategy type
- enabled flag
- rule definition

### Required Actions

- create
- edit
- enable/disable

---

## 15. Cross-Screen State Rules

## 15.1 Navigation Preservation

When moving from list → detail → action flow, the UI should preserve enough route/query state that returning feels natural.

Examples:

- group list filters remain when returning from group detail
- message query state remains when returning from message detail

## 15.2 Cluster Context Preservation

Switching clusters should:

- clear cluster-specific data views safely
- preserve global UI shell state where appropriate
- avoid silently mixing data from two clusters

## 15.3 Draft Preservation

Replay drafts and complex settings forms should preserve draft state until explicitly abandoned or completed.

---

## 16. Screen Implementation Priority

The recommended build order is:

1. shell
2. settings (cluster profiles)
3. overview
4. topics list
5. topic detail
6. groups list
7. group detail
8. messages
9. message detail
10. replay wizard
11. audit
12. saved queries
13. trace

This order matches both MVP value and dependency reality.

---

## 17. Final Recommendation

KafkaDesk should implement screens as **workflow surfaces**, not merely route containers.

Each page should have:

- clear purpose
- explicit regions
- disciplined actions
- predictable states

If these page-level specs are followed, the frontend implementation can move quickly without sacrificing coherence or visual quality.

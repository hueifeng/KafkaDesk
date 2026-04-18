# KafkaDesk Wireframes and Visual Direction v0.1

> Historical reference: these wireframes are preserved for design traceability and do not represent the current shipped UI one-for-one.

## Status

- Stage: Initial low-fidelity product surface draft
- Companion docs:
  - [`traceforge-design.md`](./traceforge-design.md)
  - [`traceforge-product-design.md`](./traceforge-product-design.md)
- Focus: page layouts, screen structure, navigation mechanics, and visual direction

---

## 1. Purpose of This Document

This document translates the product design into concrete screen structure.

It answers:

1. What should the main pages look like structurally?
2. How should users move between overview, topics, groups, messages, replay, and trace?
3. What layout patterns should stay consistent across the product?
4. What visual direction should make KafkaDesk feel strong, technical, and beautiful instead of generic?

This is intentionally **low-fidelity but opinionated**.

It is not final visual design, but it should strongly constrain future UI decisions.

---

## 2. Desired Product Feel

KafkaDesk should avoid looking like a generic admin template, observability dashboard clone, or fluffy SaaS control panel.

KafkaDesk should feel like:

> **a precision instrument for inspecting live event systems**

### Emotional Tone

- controlled
- sharp
- technical
- confident
- dense, but not cluttered
- beautiful in an industrial way

### Recommended Aesthetic Direction

Adopt an **industrial signal-room** aesthetic.

That means:

- dark-first interface
- structured grids
- crisp typography
- restrained but powerful accent colors
- high-information density
- deliberate use of glow, linework, and depth only where meaning benefits

The interface should feel like a fusion of:

- a serious IDE
- a modern systems console
- a high-end network operations view

But it must still be easy to read for long debugging sessions.

---

## 3. Visual Design Direction

## 3.1 Color System

### Base Palette

- Near-black background with subtle tonal separation
- One cold accent family for “inspect / neutral / navigation”
- One warm accent family for “risk / replay / warnings”
- One success color for healthy system indicators

### Suggested Mood

- background: graphite / midnight / deep slate
- neutral surfaces: charcoal / gunmetal
- inspect accent: electric cyan or cold teal
- warning accent: ember orange or muted amber
- danger accent: controlled crimson, used sparingly

### Rules

- color must encode status and intent, not decoration
- avoid rainbow dashboards
- avoid neon overload

---

## 3.2 Typography

### Principle

The typography should make the product feel credible and premium.

### Roles

- **Display / section labels**: something structured, technical, and slightly distinctive
- **Body / table text**: highly legible for dense information
- **Code / payload text**: strong monospace with good punctuation clarity

### Visual Rules

- payload and metadata views should rely on monospace where useful
- section headings should feel engineered, not editorial
- avoid overly soft rounded product-marketing typography

---

## 3.3 Surfaces and Depth

### Surface Strategy

Use layered surfaces with subtle separation:

1. application frame
2. nav rail
3. page canvas
4. data cards / tables / inspectors
5. overlays / drawers / modals

### Depth Rules

- use depth through contrast, borders, and shadow restraint
- avoid giant floating cards everywhere
- let tables and inspectors feel embedded in a real system console

---

## 3.4 Motion

Motion should clarify state changes, not distract.

Use motion for:

- table-to-detail transitions
- drawer open/close
- replay step progression
- trace result focus transitions
- filter changes / result refresh cues

Avoid constant animations, meaningless pulses, or excessive hover effects.

---

## 4. Global Layout System

## 4.1 App Shell

All main screens should share the same shell.

For v1, this shell is a **desktop application shell**, even though the internal layout language still resembles a high-end web/workbench UI.

```text
+--------------------------------------------------------------------------------+
| Top Header: Cluster | Env | Global Search | Recent | Alerts | User             |
+----------------------+---------------------------------------------------------+
| Left Nav Rail        | Main Content Area                                        |
| Overview             |                                                         |
| Topics               |                                                         |
| Groups               |                                                         |
| Messages             |                                                         |
| Replay               |                                                         |
| Trace                |                                                         |
| Saved Queries        |                                                         |
| Audit                |                                                         |
| Settings             |                                                         |
+----------------------+---------------------------------------------------------+
```

### Shell Rules

- left rail should remain stable across the application
- top header should persist cluster context and search
- the main content area should allow both table-heavy and detail-heavy screens

---

## 4.2 Standard Page Pattern

Most pages should follow this structure:

```text
+--------------------------------------------------------------------------+
| Page Title | Context Tags | Quick Actions                                |
+--------------------------------------------------------------------------+
| Optional Summary Strip / Filters / Query Controls                         |
+--------------------------------------------------------------------------+
| Primary Content Region                                                    |
| - table / chart / inspector / graph                                       |
| - supports drill-down                                                     |
+--------------------------------------------------------------------------+
| Secondary Region (optional)                                               |
| - detail panel / activity feed / related entities                         |
+--------------------------------------------------------------------------+
```

### Page Rules

- every page should answer one dominant question
- filters belong near the top, not scattered randomly
- details should either open in a right-side inspector or navigate to a dedicated detail page

---

## 4.3 Right-Side Inspector Pattern

For many workflows, a right-side inspector will be more efficient than full-page navigation.

Use it for:

- quick message preview
- topic metadata preview
- replay pre-check summary
- group partition breakdown preview

Use a full page when:

- the user is doing a deep task
- multiple tabs or actions are involved
- the content needs comparison or editing

---

## 5. Core Page Wireframes

## 5.1 Overview Page

### Main Job

Help the user orient to a cluster and immediately see where to investigate.

### Wireframe

```text
+--------------------------------------------------------------------------------+
| Overview | Cluster: prod-east | Schema Registry: Connected | 3 Alerts          |
+--------------------------------------------------------------------------------+
| [Topics: 248] [Groups: 91] [Lagging Groups: 7] [Replay Jobs Today: 14]        |
+--------------------------------------------------------------------------------+
| Cluster Health                          | Consumer Group Health                |
| - broker count                          | - top lagging groups                 |
| - active topics                         | - unhealthy states                   |
| - connection status                     | - partition hotspots                 |
+----------------------------------------+---------------------------------------+
| Recent Activity                         | Quick Actions                        |
| - recent replay jobs                    | - browse messages                    |
| - saved trace queries                   | - search trace key                   |
| - audit highlights                      | - open lagging groups                |
+--------------------------------------------------------------------------------+
```

### Key Notes

- this page is an orientation surface, not a wall of charts
- summary cards should be compact and information-rich
- quick actions should feel like launch points into real workflows

---

## 5.2 Topics List Page

### Main Job

Find the relevant topic quickly.

### Wireframe

```text
+--------------------------------------------------------------------------------+
| Topics | Search [........] | Filters: Internal Off | Favorites | Sort            |
+--------------------------------------------------------------------------------+
| Topic Name              | Partitions | RF | Schema | Retention | Activity | Fav  |
| orders.events           | 24         | 3  | Avro   | 7d        | High     | ★    |
| payments.retry          | 12         | 3  | JSON   | 3d        | Medium   |      |
| inventory.snapshot      | 8          | 2  | Proto  | 1d        | Low      |      |
| ...                                                                          |
+--------------------------------------------------------------------------------+
```

### Key Notes

- prefer a dense but elegant table
- highlight schema presence and activity level without over-designing badges
- topic names must remain scannable

---

## 5.3 Topic Detail Page

### Main Job

Understand the topic and launch message inspection.

### Wireframe

```text
+--------------------------------------------------------------------------------+
| Topic: orders.events | 24 partitions | Avro | Retention: 7d | Browse Messages  |
+--------------------------------------------------------------------------------+
| Summary Strip: message flow hints | related groups count | last activity       |
+--------------------------------------------------------------------------------+
| Partition Table                                                                 |
| Partition | Earliest | Latest | Leader | Replica Status | Consumer Groups     |
| ...                                                                            |
+--------------------------------------------------------------------------------+
| Related Consumer Groups                                                         |
| Group Name | Total Lag | State | Open Group                                     |
+--------------------------------------------------------------------------------+
| Advanced Config (collapsed by default)                                          |
+--------------------------------------------------------------------------------+
```

### Key Notes

- “Browse Messages” must be prominent
- advanced Kafka config belongs behind disclosure, not in the main path

---

## 5.4 Groups List Page

### Main Job

Find lagging or unhealthy groups fast.

### Wireframe

```text
+--------------------------------------------------------------------------------+
| Groups | Search [........] | Show: Lagging Only | Sort: Total Lag Desc         |
+--------------------------------------------------------------------------------+
| Group Name            | State     | Total Lag | Topics | Partitions | Last Seen |
| payment-worker        | Rebalancing| 48291    | 3      | 48         | now       |
| order-materializer    | Stable    | 11890     | 2      | 24         | now       |
| ...                                                                            |
+--------------------------------------------------------------------------------+
```

### Key Notes

- state and lag should visually dominate secondary metadata
- a filter-first workflow matters here more than pretty charts

---

## 5.5 Group Detail Page

### Main Job

Move from “this group is unhealthy” to “which topic/partition is the problem?”

### Wireframe

```text
+--------------------------------------------------------------------------------+
| Group: payment-worker | State: Rebalancing | Total Lag: 48,291 | Save Query      |
+--------------------------------------------------------------------------------+
| Summary Strip: topics affected | partitions affected | coordinator | last update    |
+--------------------------------------------------------------------------------+
| Topic-Level Lag Breakdown                                                          |
| Topic Name           | Total Lag | Partitions Impacted | Open Topic                     |
+-----------------------------------------------------------------------------------+
| Partition-Level Detail                                                            |
| Topic | Partition | Committed Offset | Log End Offset | Lag | Browse Messages        |
+-----------------------------------------------------------------------------------+
```

### Key Notes

- the page should make partition drill-down easy
- “Browse Messages” should be available directly from partition rows

---

## 5.6 Messages Page

### Main Job

Run a bounded message inspection query directly.

### Layout Pattern

Use a two-zone layout:

- upper query builder
- lower results table

### Wireframe

```text
+--------------------------------------------------------------------------------+
| Messages                                                                        |
+--------------------------------------------------------------------------------+
| Query Builder                                                                   |
| Topic [orders.events]  Partition [all]  Time [last 30m]  Key [........]        |
| Header Filter [........]  Max Results [100]  [Run Query]                        |
+--------------------------------------------------------------------------------+
| Results                                                                         |
| Time      | Partition | Offset | Key        | Decode | Payload Preview          |
| 12:03:11  | 7         | 182819 | ord-123    | Avro   | {"orderId":"..."}     |
| ...                                                                            |
+--------------------------------------------------------------------------------+
| Optional Right Inspector: selected message preview                              |
+--------------------------------------------------------------------------------+
```

### Key Notes

- all query bounds must be visible
- never hide the scope of the read
- result rows should support keyboard-friendly navigation later

---

## 5.7 Message Detail Page

### Main Job

Inspect one message deeply and take contextual actions.

### Layout Recommendation

Use a 70/30 split.

```text
+--------------------------------------------------------------------------------+
| Message Detail | Topic: orders.events | Partition 7 | Offset 182819             |
+--------------------------------------------------------------------------------+
| LEFT: Main Inspector                      | RIGHT: Context and Actions          |
|-------------------------------------------+------------------------------------|
| [Tabs] Decoded | Raw | Headers | Metadata | Key                                |
|                                           | Timestamp                          |
| Decoded payload tree / JSON viewer        | Schema                             |
|                                           | Topic / partition                  |
| Raw payload viewer                        | Trace from this key                |
|                                           | Replay this message                |
| Diff panel (optional state)               | Bookmark / copy                    |
+--------------------------------------------------------------------------------+
```

### Key Notes

- this should feel like a premium debugger surface
- payload viewing quality matters more than visual gimmicks
- copy, fold, expand, and compare must be first-class

---

## 5.8 Replay Wizard

### Main Job

Guide the user through a safe operational write path.

### Layout Recommendation

Use a stepped flow with a persistent right-side summary.

### Wireframe

```text
+--------------------------------------------------------------------------------+
| Replay Message                                                                  |
+--------------------------------------------------------------------------------+
| Step 1: Source      | Step 2: Target | Step 3: Edit | Step 4: Risk | Confirm   |
+--------------------------------------------------------------------------------+
| MAIN STEP CONTENT                         | PERSISTENT SUMMARY                  |
|-------------------------------------------+------------------------------------|
| source preview / target form / editor     | Source topic/partition/offset      |
| risk messaging / confirmation             | Target topic                       |
|                                           | payload diff summary               |
|                                           | permission / environment           |
+--------------------------------------------------------------------------------+
```

### Key Notes

- a wizard is better than a single form here
- use strong warning design only in the risk and confirm steps
- the final step should clearly show exactly what will be sent

---

## 5.9 Trace Page

### Main Job

Find related events across topics by key.

### Layout Recommendation

Use search input on top and switchable result views below.

### Wireframe

```text
+--------------------------------------------------------------------------------+
| Trace                                                                           |
+--------------------------------------------------------------------------------+
| Key Type [traceId] | Value [........] | Scope [selected topics] | Time [2h]     |
| [Run Trace]                                                                     |
+--------------------------------------------------------------------------------+
| Result View Tabs: Timeline | Graph | Table                                      |
+--------------------------------------------------------------------------------+
| Result Canvas                                                                     |
| - timeline of events                                                               |
| - graph/path view                                                                  |
| - row opens message detail                                                         |
+-----------------------------------------------------------------------------------+
| Notes: correlation method / confidence / inferred-path disclaimer                 |
+-----------------------------------------------------------------------------------+
```

### Key Notes

- timeline is likely the best default
- graph should be used when it genuinely helps, not as decoration
- users must understand whether results are inferred or direct

---

## 5.10 Saved Queries Page

### Main Job

Help teams repeat useful debugging workflows.

### Wireframe

```text
+--------------------------------------------------------------------------------+
| Saved Queries | Filter [mine/team/all]                                          |
+--------------------------------------------------------------------------------+
| Query Name | Type | Cluster | Owner | Last Run | Open                           |
+--------------------------------------------------------------------------------+
```

### Key Notes

- keep this page simple in v1
- usefulness matters more than sophistication

---

## 5.11 Audit Page

### Main Job

Make risky activity visible and reviewable.

### Wireframe

```text
+--------------------------------------------------------------------------------+
| Audit | Filter: replay / send / config / auth                                  |
+--------------------------------------------------------------------------------+
| Time | Actor | Action | Target | Outcome | Open                                |
+--------------------------------------------------------------------------------+
```

### Key Notes

- this should look formal and trustworthy
- operational history must feel reviewable, not decorative

---

## 6. Reusable UI Patterns

## 6.1 Summary Strip

Use compact horizontal summary strips at the top of detail pages.

Examples:

- topic summary
- group health summary
- replay summary

### Rule

They should compress context, not replace detailed content.

---

## 6.2 Dense Data Tables

Tables are central to the product.

### Table Rules

- row density should be medium-high
- headers must stay crisp and sticky when useful
- sorting and filtering should be obvious
- row click behavior must be predictable

### Visual Rule

Avoid generic cardified tables.

KafkaDesk tables should feel like serious system consoles.

---

## 6.3 Inspector Panels

Inspector panels should be used for:

- quick preview
- short metadata reads
- action staging

Avoid placing full workflows inside tiny inspectors.

---

## 6.4 Badge System

Badges should be limited to meaningful signals:

- environment
- decode status
- replay risk
- consumer group state
- schema type

Badge overuse will make the product noisy.

---

## 6.5 Empty / Error / Loading States

### Empty State Rule

Always help users take the next action.

### Error State Rule

Always explain the scope and likely cause.

### Loading State Rule

Where useful, show what is being loaded.

Example:

- “Loading last 100 messages from orders.events over the previous 30 minutes”

---

## 7. Responsive Behavior

### Desktop First

KafkaDesk should be desktop-first.

It is a dense operator workflow product, not a mobile-first app.

### Tablet Support

Should remain usable for inspection, but not necessarily optimal for deep replay editing.

### Mobile Support

Should degrade gracefully for read-only status checks, not full workflows.

---

## 8. “Make It Beautiful” Constraints

The user explicitly wants the pages to look good.

That should be interpreted as:

1. **Do not ship a generic admin template**
2. **Do not rely on default dashboard aesthetics**
3. **Make the product visually memorable without reducing readability**

### What “Good Looking” Means for KafkaDesk

It means:

- elegant data density
- premium message inspectors
- visually disciplined status systems
- consistent layout rhythm
- subtle but meaningful motion
- strong contrast and hierarchy

It does **not** mean:

- flashy charts everywhere
- giant gradients with weak information design
- over-rounded consumer-app components
- pretty but impractical payload views

---

## 9. Recommended Next Design Artifacts

The best next documents after this one are:

1. `traceforge-screen-specs.md`
   - exact region-by-region page specs
2. `traceforge-design-system.md`
   - tokens, typography, color, spacing, component rules
3. `traceforge-user-flows.md`
   - step-by-step flows for core tasks
4. initial visual mockups or frontend prototype

---

## 10. Final Recommendation

The first implemented UI should optimize for three things simultaneously:

1. **message debugging quality**
2. **lag diagnosis speed**
3. **aesthetic credibility**

If KafkaDesk gets these right, it will feel distinct from today’s Kafka tooling.

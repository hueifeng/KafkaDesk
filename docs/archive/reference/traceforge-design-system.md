# KafkaDesk Design System v0.1

> Historical reference: this document captures an earlier UI system direction and should be read as design history, not current component truth.

## Status

- Stage: Initial design-system specification
- Product form: desktop-first
- Companion docs:
  - [`traceforge-product-design.md`](./traceforge-product-design.md)
  - [`traceforge-wireframes.md`](./traceforge-wireframes.md)

---

## 1. Goal

This document defines the visual and interaction rules that should make KafkaDesk feel cohesive, premium, and technically credible.

It is intended to prevent the UI from drifting into:

- generic admin-template design
- inconsistent component styling
- random dashboard aesthetics
- weak hierarchy in high-density data views

---

## 2. Design Philosophy

KafkaDesk should embody a **precision instrument** aesthetic.

### Core Aesthetic

- industrial
- dark-first
- sharp
- dense but calm
- visually disciplined
- powerful without being flashy

### Product Character

The product should visually sit somewhere between:

- an IDE
- a packet/network analysis tool
- a high-end operations console

---

## 3. Design Principles

### 3.1 Clarity Over Decoration

Every visual treatment should support:

- faster scanning
- stronger hierarchy
- clearer risk understanding

### 3.2 Dense but Breathable

The UI should support serious information density while keeping enough spacing for readability.

### 3.3 Color for Meaning

Color should encode:

- health
- warning
- danger
- focus
- decode state

Color should not be used as random ornament.

### 3.4 Consistency Across Depth Levels

Overview pages, tables, detail pages, and inspectors must feel like they belong to the same system.

### 3.5 Premium Tooling Feel

This is not a marketing site and not a toy interface.

The product should feel expensive, reliable, and exact.

---

## 4. Token Foundations

## 4.1 Color Roles

Define colors by role first, not by arbitrary hex lists.

### Core Roles

- `bg.app`
- `bg.surface`
- `bg.panel`
- `bg.elevated`
- `border.subtle`
- `border.strong`
- `text.primary`
- `text.secondary`
- `text.muted`
- `accent.inspect`
- `accent.info`
- `accent.success`
- `accent.warning`
- `accent.danger`
- `accent.trace`

### Suggested Mood

- app background: deep graphite / midnight slate
- surfaces: layered charcoal tones
- inspect focus: cyan-leaning or teal-leaning cool signal color
- warning/replay: ember orange / muted amber
- danger: controlled red, not neon
- trace/correlation: electric but restrained blue-violet or blue-cyan branch

### Rule

Never let accent colors occupy too much surface area.

Accents should punctuate the interface, not flood it.

---

## 4.2 Typography Roles

### Roles

- `type.display`
- `type.section`
- `type.body`
- `type.table`
- `type.code`
- `type.label`

### Typography Requirements

- table text must remain highly legible at dense sizes
- code and payload views must use a strong monospace family
- headings should feel engineered, not soft or editorial

### Usage Rules

- use heavier weights sparingly
- prefer hierarchy through size, spacing, and contrast rather than constant bolding
- payload/code blocks should prioritize punctuation clarity and line alignment

---

## 4.3 Spacing Scale

Use a consistent spacing scale for all layout and components.

Suggested conceptual scale:

- xs
- sm
- md
- lg
- xl
- 2xl

### Rule

Do not improvise spacing per component.

Consistent spacing ensures the UI feels intentional and cohesive.

---

## 4.4 Radius and Corners

KafkaDesk should not use exaggerated softness.

### Rule

- small radius for inputs and tags
- medium radius for panels if needed
- avoid over-rounded “consumer SaaS” corners

The product should feel precise and engineered, avoiding bubbly aesthetics.

---

## 4.5 Border Strategy

Borders play a key role in defining structure within dark, dense UIs.

### Border Rules

- use subtle borders to define structure
- use stronger borders only for selection, focus, or critical separation
- avoid relying only on shadow for structure

---

## 4.6 Shadow and Glow

### Shadow Rules

- use restrained shadows for elevation
- avoid soft, muddy shadows that reduce sharpness

### Glow Rules

- glow may be used sparingly for active trace/focus or key status moments
- glow should never become a constant styling crutch

---

## 5. Layout System

## 5.1 Application Shell

The shell has three permanent ideas:

1. left navigation rail
2. top cluster/context header
3. main content canvas

### Layout Rule

The shell should remain visually stable so the user always feels anchored.

---

## 5.2 Page Structure

Every page should usually contain:

1. page title and context tags
2. summary strip or filters
3. primary content region
4. optional secondary region or inspector

### Rule

Do not invent page structure ad hoc.

The product should feel systematic.

---

## 5.3 Inspector Pattern

Inspector panels are a core product pattern.

Use them for:

- metadata preview
- quick record preview
- secondary details
- contextual action staging

### Rule

If the task requires deep comparison, editing, or multi-step action, use a dedicated page instead.

---

## 6. Component Guidelines

## 6.1 Navigation Rail

### Purpose

Provide stable task-oriented navigation.

### Design Rules

- compact vertical rhythm
- active state should feel precise, not oversized
- icons should support scanning but not dominate labels

---

## 6.2 Header Bar

### Content

- cluster selector
- environment badge
- search entry
- recent items
- user menu

### Design Rules

- keep the header lean
- preserve focus on the page body
- environment and cluster context should remain unmistakable

---

## 6.3 Summary Cards

Summary cards exist to compress important signals.

### Rules

- small count of cards per strip
- compact copy
- strong number hierarchy
- never use cards where a table or list would be clearer

---

## 6.4 Data Tables

Tables are one of the most important components in KafkaDesk.

### Table Rules

- medium-high density
- clean row separators
- sticky headers where useful
- clear hover and selection state
- predictable sorting controls
- first column must be highly scannable

### Avoid

- giant row heights
- excessive cardification
- noisy alternating backgrounds

---

## 6.5 Status Badges

Status badges should be used sparingly and consistently.

### Good Badge Use

- stable / rebalancing / error
- schema type
- decode status
- environment
- replay risk level

### Rule

If everything becomes a badge, nothing carries meaning.

---

## 6.6 Buttons

### Primary Buttons

Use for core task progression only.

### Secondary Buttons

Use for routine local actions.

### Destructive or Risk Buttons

Use with explicit visual distinction and confirmation flow.

### Rule

Do not place multiple equally loud primary actions next to each other.

---

## 6.7 Forms and Filters

Filters are part of the core debugging loop.

### Rules

- keep filter groups compact
- make bounds explicit
- prefer visible scope over “magic defaults”
- indicate what a query will actually do before running it

---

## 6.8 JSON / Payload Viewer

This is one of the most important high-value components.

### Requirements

- tree expansion/collapse
- copy actions
- line wrapping strategy
- readable nested hierarchy
- raw and decoded side-by-side support when needed
- diff-friendly view modes in the future

### Design Rules

- maximize readability over style tricks
- preserve monospace integrity
- use color lightly for structure, not for rainbow syntax overload

---

## 6.9 Replay Wizard

### Purpose

Guide a risky operation safely.

### Design Rules

- use clear step progression
- keep a persistent summary sidebar
- use warning styling only at actual risk points
- make the final confirmation extremely explicit

---

## 6.10 Trace Visualization

### Purpose

Help users reason about event relationships.

### Preferred Modes

- timeline first
- table second
- graph when it adds real value

### Rule

Graph views should clarify relationships, not exist for visual novelty.

---

## 7. Motion System

Motion should support comprehension.

### Good Motion Uses

- inspector open/close
- row to detail transition
- step progression in replay
- trace result focus changes
- skeleton to loaded state transitions

### Avoid

- permanent shimmer noise
- decorative ambient animation everywhere
- meaningless hover jumps

### Rule

If motion does not improve orientation, remove it.

---

## 8. State Design Rules

## 8.1 Empty States

Every empty state should answer:

- what is missing?
- why is it empty?
- what should the user do next?

## 8.2 Error States

Errors should be interpretable, not raw exceptions.

Examples:

- broker auth failed
- schema decode failed
- query scope too broad
- replay blocked by policy

## 8.3 Loading States

Loading text should communicate scope when helpful.

Example:

- “Loading the last 100 messages from topic orders.events over the last 30 minutes”

---

## 9. Iconography and Illustration

### Icon Rules

- use icons to reinforce recognition, not decorate every line
- topic, group, message, replay, and trace should each have distinct visual anchors

### Illustration Rules

- minimal illustrative treatment only
- no heavy marketing-style hero art inside the product UI

---

## 10. Theme Rules

KafkaDesk v1 should be designed dark-first.

### Why

- better fit for dense operational tools
- stronger contrast control for tables and payload views
- aligns with the industrial signal-room aesthetic

### Light Theme

Can be considered later, but should not dilute the primary visual direction in v1.

---

## 11. Implementation Guardrails

When the UI implementation begins, the following should be treated as hard constraints:

1. do not start from a generic admin template and “skin it later”
2. do not use random dashboard widgets without workflow purpose
3. do not over-round everything
4. do not overuse gradients
5. do not sacrifice payload readability for visual novelty
6. do not let tables feel like low-priority components

---

## 12. Initial Component Priority

The first components worth designing carefully are:

1. app shell
2. navigation rail
3. data table
4. summary strip
5. message inspector
6. payload viewer
7. replay wizard stepper
8. status badge system
9. filter toolbar
10. right-side inspector panel

---

## 13. Final Recommendation

KafkaDesk should look like a serious piece of engineering equipment.

The UI should be:

- beautiful
- dense
- exact
- restrained
- memorable

If the visual system stays disciplined, the product can feel premium without becoming noisy.

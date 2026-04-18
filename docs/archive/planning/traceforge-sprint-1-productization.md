# KafkaDesk Sprint 1 Productization Plan

## Status

- Stage at creation time: sprint-level productization plan for the first hardening phase
- Current role: historical sprint-planning reference

This document preserves the original Sprint 1 execution framing that moved KafkaDesk from an early alpha shell toward a safer, more truthful desktop product. It is useful as project history and decision context, but it is not the current status page for the repository.

## Original Goal

Move KafkaDesk from a loosely validated alpha shell into a safer product engineering phase by delivering two outcomes:

1. a minimal engineering safety baseline
2. truthful capability and error reporting for cluster and schema registry validation

## Original Scope

This sprint intentionally did **not** try to complete all P0 work. It focused on the minimum slice that unlocked safe iteration and removed the most misleading validation behavior.

### In scope

- engineering foundation: scripts, tests baseline, CI baseline, repo hygiene
- shared error and capability-result contract between frontend and Tauri backend
- cluster validation v2 with staged capability reporting
- schema registry validation v2 with staged API-level reporting
- small supporting documentation updates for the new workflow

### Out of scope

- full replay implementation
- schema decode pipeline completion
- deep trace engine work
- large UI refactors outside affected settings flows
- broad accessibility pass

## Success Criteria

Sprint 1 was defined as successful when all of the following were true:

1. the project had a repeatable local/CI check path
2. backend commands could return structured error categories and staged capability results
3. cluster validation no longer reported success from TCP reachability alone
4. schema registry validation no longer reported success from TCP reachability alone
5. settings UI clearly showed which validation stage passed or failed
6. current limitations around secure credential references were described truthfully

## Workstreams

### 1. Engineering Foundation

#### Deliverables

- `typecheck`, `lint`, `test`, and `check` scripts
- initial frontend test baseline
- initial Rust test baseline
- CI workflow for install/check/build
- repo hygiene cleanup for generated artifacts and ignore rules

#### Acceptance

- a contributor can run one command to execute the main checks
- CI runs the same baseline checks automatically
- the repository no longer mixes obvious generated output into the source workflow

### 2. Shared Error and Capability Contract

#### Deliverables

- shared error categories used by backend and frontend
- staged capability result shape used by validation commands
- frontend tauri bridge mapping for the new structures
- settings-page rendering rules for stage-by-stage results

### 3. Cluster Validation V2

#### Deliverables

- explicit profile-to-runtime mapping for validation inputs
- staged validation flow covering at least bootstrap reachability, metadata fetch, auth/TLS application, and topic-list capability
- settings UI panel showing stage outcomes

### 4. Schema Registry Validation V2

#### Deliverables

- staged validation covering endpoint reachability, auth/TLS application, registry API reachability, and minimal successful API operation
- structured frontend rendering of stage outcomes
- truthful wording around `credential_ref`

### 5. Sprint Documentation

#### Deliverables

- short design note / ADR capturing the Sprint 1 contract and validation decisions
- README update for local checks and current validation scope

## Historical Execution Order

1. engineering foundation baseline
2. shared error and capability contract
3. cluster validation v2
4. schema registry validation v2
5. documentation and verification pass

## Historical Risks and Controls

- avoid letting sprint scope grow into replay/decode/trace
- ship backend and settings UI changes as one unit
- do not overstate credential-storage behavior if the implementation is not actually present

## Reading Guidance

Use this file to understand the original sprint boundary and why the first hardening wave was sequenced the way it was. For current product capabilities, verification commands, and release posture, use the root `README.md` and active docs under `docs/` instead.

# KafkaDesk Documentation

This directory is organized to separate current product documentation from reference material and archived planning history.

## Start Here

If you are new to the repository, read in this order:

1. [`../README.md`](../README.md) — product overview, quick start, current workflows, and release posture
2. [`../CONTRIBUTING.md`](../CONTRIBUTING.md) — contributor workflow, local environment, and verification lanes
3. [`../SECURITY.md`](../SECURITY.md) — vulnerability reporting and runtime caveats
4. [`product/README.md`](./product/README.md) — current product docs collected under `docs/`

## Current Product Docs

- [`product/README.md`](./product/README.md) — index for current product-facing docs in this directory
- [`product/release-distribution.md`](./product/release-distribution.md) — current release and distribution posture
- [`decisions/adr-2026-04-sprint-1-validation-hardening.md`](./decisions/adr-2026-04-sprint-1-validation-hardening.md) — accepted hardening decision record

## Technical Reference

- [`reference/README.md`](./reference/README.md) — index for architecture, contract, and UX reference material
- [`reference/kafkadesk-desktop-architecture.md`](./reference/kafkadesk-desktop-architecture.md) — desktop/runtime architecture notes
- [`reference/kafkadesk-tech-stack.md`](./reference/kafkadesk-tech-stack.md) — stack decisions
- [`reference/kafkadesk-data-model.md`](./reference/kafkadesk-data-model.md) — local data model and persistence rules
- [`reference/kafkadesk-api-contracts.md`](./reference/kafkadesk-api-contracts.md) — runtime contract reference

## Archived Planning History

These files are preserved for project history and decision traceability. They are not the current product status page.

- [`archive/README.md`](./archive/README.md) — archive index and usage guidance
- [`archive/planning/traceforge-mvp-plan.md`](./archive/planning/traceforge-mvp-plan.md)
- [`archive/planning/traceforge-implementation-plan.md`](./archive/planning/traceforge-implementation-plan.md)
- [`archive/planning/traceforge-backlog.md`](./archive/planning/traceforge-backlog.md)

Archived design and UX material also lives under [`archive/reference/`](./archive/reference/README.md).

## Current Source of Truth

For current implementation and maturity status, use:

- the code in `src/` and `src-tauri/`
- [`../README.md`](../README.md)
- [`product/README.md`](./product/README.md)

For historical planning context, use the archive section instead of internal working files.

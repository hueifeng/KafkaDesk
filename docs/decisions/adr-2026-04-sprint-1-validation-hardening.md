# ADR: Sprint 1 Validation Hardening Baseline

## Status

Accepted

## Context

KafkaDesk has already moved beyond a documentation-only state and now contains a real Tauri + React + Rust + SQLite implementation. However, the project still showed two maturity gaps that blocked trustworthy iteration:

1. engineering checks were not standardized across local development and CI
2. cluster and schema registry "test connection" flows could report success from shallow reachability checks that did not match real product expectations

## Decision

Sprint 1 establishes a minimal product-hardening baseline with four explicit choices.

### 1. Standardize the local and CI verification path

The repository now exposes:

- `npm run lint`
- `npm run typecheck`
- `npm run test`
- `npm run check`

CI runs the same baseline rather than a separate custom workflow.

### 2. Keep the existing Tauri command names and page plumbing

Sprint 1 does **not** rename the validation commands or replace the page-level mutation flow. Instead, it extends the existing command payloads with a richer contract so the project can harden validation behavior without a routing or API migration.

### 3. Introduce staged capability reporting

Validation commands now return structured stages rather than only a flat boolean/message pair. This allows the settings pages to distinguish:

- input/config problems
- linked-profile problems
- TCP reachability
- API/metadata checks
- unsupported or not-yet-implemented capability gaps

### 4. Be explicit about unsupported credential-backed validation

Sprint 1 does not pretend secure credential resolution already exists.

- cluster validation remains truthful about unsupported SASL / mTLS paths
- schema registry validation is fully API-backed only for `authMode = none`
- `credentialRef` is treated as a stored reference label, not as an active secret retrieval mechanism

## Consequences

### Positive

- local and CI verification now share the same baseline
- settings pages can explain *why* validation failed instead of collapsing everything into a single banner
- the product stops over-claiming connection success when only TCP reachability was proven

### Negative

- some validation results now intentionally degrade from a misleading “success” to a truthful warning/failure
- authenticated schema registry validation is still incomplete until secret retrieval is implemented
- SASL and mTLS Kafka validation still require follow-up Sprint work

## Follow-up

Next productization work should build on this baseline:

1. secure credential resolution for schema registry and Kafka auth flows
2. deeper Kafka capability validation beyond metadata/topic visibility
3. replay semantics hardening and stronger end-to-end workflow tests

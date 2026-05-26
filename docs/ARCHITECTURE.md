# Architecture

See [`plans/continuum-autonomous-production-engineer-reactive-parasol.md`](../plans/continuum-autonomous-production-engineer-reactive-parasol.md)
for the authoritative system architecture plan.

Continuum is an autonomous production engineering runtime implemented as a
Rust Cargo workspace. It coordinates repository analysis, planning,
multi-agent execution, validation, security hardening, memory, sandboxing, and
recovery through small domain crates wired together by the runtime.

## Execution pipeline

Every autonomous task moves through the same disciplined pipeline:

1. Plan
2. Analyze
3. Implement
4. Run
5. Test
6. Fix
7. Re-test
8. Security implementation
9. Security validation
10. Harden
11. Final review
12. Complete

The runtime must not mark a task complete until the configured validation
contract passes.

## Workspace summary

Continuum is a Cargo workspace structured in four tiers:

### Foundation tier
- `continuum-core` — shared traits, error types, IDs, capability tokens
- `continuum-config` — layered configuration loading
- `continuum-telemetry` — tracing + OpenTelemetry + Prometheus
- `continuum-markdown` — engineering docs parser

### Infrastructure tier
- `continuum-storage` — SQLite + `sqlite-vec` (Qdrant feature-gated)
- `continuum-memory` — three-layer memory engine
- `continuum-models` — multi-provider model orchestration
- `continuum-models-registry` — vendored models.dev snapshot + codegen
- `continuum-sandbox` — Docker sandbox (Firecracker feature-gated)
- `continuum-tools` + sub-crates — linters, security, testing, browser

### Domain tier
- `continuum-repo` — tree-sitter repository intelligence
- `continuum-planner` — DAG construction and estimation
- `continuum-agents` — eight subagents
- `continuum-validation` — 10-stage validation pipeline
- `continuum-security` — security implementation layer
- `continuum-recovery` — checkpoints, rollback, replay
- `continuum-runtime` — orchestrator

### Binary tier
- `continuum-cli` — the `continuum` binary with ratatui live dashboard
- `xtask` — workspace automation

## Architectural invariants

- **No sideways edges in the domain tier.** Cross-domain coordination happens only in `continuum-runtime`.
- **Tools spawn processes only inside the sandbox.** No host-side `Command::spawn` in any tool crate.
- **`continuum-core` stays minimal.** No tokio runtime, no I/O. Only types, traits, errors.
- **Centralized persistence.** Domain crates access SQLite through `continuum-storage` only.
- **Retrieval-driven repo loading.** The runtime never loads an entire repository into model context.

## Validation stages

1. Compilation
2. Linting
3. Type checking
4. Unit testing
5. Integration testing
6. E2E testing
7. Security scanning
8. Startup validation
9. Performance validation
10. Regression validation

## Recovery model

Long-running execution is checkpointed through `continuum-recovery`. Sessions
must be resumable, replayable, and roll-backable once the recovery crate is
implemented. Stuck-task detection and infinite-loop prevention are required
runtime responsibilities.

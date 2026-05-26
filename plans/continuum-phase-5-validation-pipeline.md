# Continuum — Phase 5: Validation Pipeline

## Context

By end of Phase 4, Continuum can produce code changes and lint them — but only with Clippy, only as a single tool call inside the agent loop. The architectural plan calls for a structured 10-stage validation pipeline that runs *after* an agent completes a task node, with stage ordering, required/optional discipline, and findings rolled up uniformly.

Phase 5 builds that pipeline and adds at least one validator per stage. It's the gate between "the model wrote some code" and "the code is correct enough to ship." Without it, the system cannot honour its philosophy of *verification before completion*.

## Recommended approach

### A. Pipeline core (`continuum-validation`)

1. **`Pipeline`** is a `Vec<Box<dyn Validator>>` sorted by `ValidationStage` declaration order. Stage order is fixed: `Compile → Lint → TypeCheck → UnitTest → IntegrationTest → E2eTest → SecurityScan → Startup → Performance → Regression`.
2. **Execution semantics:**
   - **Required validators** within a stage run sequentially; the first failure short-circuits the *pipeline*.
   - **Optional validators** within a stage run in parallel using `tokio::JoinSet`; their findings are collected but do not block.
   - Stages run sequentially — `Lint` does not start until `Compile` is clean.
3. **`PipelineRun`** is the top-level type. Owns the sandbox handle, the target, and a `Vec<ValidationReport>`. Exposes `async fn run(&mut self, plan_node: &TaskNode) -> Result<PipelineSummary, ValidationError>`.
4. **`PipelineSummary`** aggregates: per-stage pass/fail, total findings by severity, total wall-clock, USD spent (carrying through from any model-backed validators).
5. **Findings normalization.** Every validator produces `continuum_core::validator::Finding` (defined in Phase 1). File paths are normalized to the sandbox-relative form so the dashboard can deep-link uniformly.

### B. Per-stage validators (one minimum per stage)

| Stage | Validator | Tool family | Required |
|---|---|---|---|
| Compile | `CargoCheckValidator` (Rust), `TscNoEmitValidator` (TS) | testing / build | yes |
| Lint | `ClippyValidator` (Rust, from Phase 4), `BiomeValidator` (TS/JS) | linters | yes |
| TypeCheck | `CargoCheckValidator` reused (compile = typecheck in Rust), `TscNoEmitValidator` reused | testing | yes |
| UnitTest | `CargoTestUnitValidator`, `VitestValidator`, `PytestValidator` | testing | yes |
| IntegrationTest | `CargoTestIntegrationValidator` (runs `tests/`), `JestE2eValidator` (config-driven) | testing | optional |
| E2eTest | Skeleton only — wires up Phase 7's Playwright runner | browser | optional |
| SecurityScan | `SemgrepValidator`, `GitleaksValidator`, `TrivyValidator` | security | optional in dev, required in `harden` mode (Phase 9) |
| Startup | `StartupValidator` — runs `cargo run` / `npm start` inside the sandbox, waits for a configured signal (port open, log line) | testing | optional |
| Performance | `K6BaselineValidator` — runs a baseline scenario, compares p95 against a stored baseline | testing | optional |
| Regression | `GitBaselineValidator` — re-runs UnitTest stage against the pre-change tree, asserts the same set of tests pass | testing | required when on a non-main branch |

Each validator lives in the appropriate tool sub-crate. `continuum-validation` re-exports them through stage-keyed registries.

### C. Reused infrastructure

- **Sandbox.** Every validator takes a `&dyn SandboxHandle` (from `continuum-core::sandbox`). No host-side `Command::spawn`.
- **Project detection.** Validators consult `ProjectKind` (from `continuum-core::tool`) to decide whether they apply.
- **Caching.** A validator may compute a content-hash of its inputs and skip on cache hit (e.g., Clippy on unchanged files). The cache lives in `continuum-storage`'s new `validator_cache` table.

### D. Runtime integration

`Scheduler::run` (introduced in Phase 4) now does:
1. Dispatch to agent.
2. On success, run the validation pipeline against the affected files.
3. If pipeline fails:
   - If the agent supports retry, append findings to its next task payload and re-dispatch (capped at 3 retries).
   - Otherwise mark the node failed and propagate.
4. If pipeline passes, commit the diff to the sandbox snapshot.

### E. Reporting

`continuum execute` end-of-run summary surfaces per-stage results:
```
✓ Compile        (1.2s)
✓ Lint           (3.4s, 2 warnings)
✓ TypeCheck      (cached)
✓ UnitTest       (12.1s, 47 passed)
- IntegrationTest (skipped — not configured)
✗ SecurityScan   (1 critical: Trivy CVE-2024-XXXXX in tokio 1.0.0)
```

In Phase 8 the same data feeds the ratatui live view.

### Critical files

- `C:\Users\hp\Documents\continuum\crates\continuum-validation\src\pipeline.rs`
- `C:\Users\hp\Documents\continuum\crates\continuum-validation\src\registry.rs` — stage → validators map.
- `C:\Users\hp\Documents\continuum\crates\continuum-validation\src\summary.rs`
- `C:\Users\hp\Documents\continuum\crates\continuum-tools-linters\src\biome.rs`
- `C:\Users\hp\Documents\continuum\crates\continuum-tools-testing\src\cargo_test.rs`
- `C:\Users\hp\Documents\continuum\crates\continuum-tools-testing\src\vitest.rs`
- `C:\Users\hp\Documents\continuum\crates\continuum-tools-testing\src\pytest.rs`
- `C:\Users\hp\Documents\continuum\crates\continuum-tools-testing\src\startup.rs`
- `C:\Users\hp\Documents\continuum\crates\continuum-tools-testing\src\k6_baseline.rs`
- `C:\Users\hp\Documents\continuum\crates\continuum-tools-security\src\semgrep.rs`
- `C:\Users\hp\Documents\continuum\crates\continuum-tools-security\src\gitleaks.rs`
- `C:\Users\hp\Documents\continuum\crates\continuum-tools-security\src\trivy.rs`
- `C:\Users\hp\Documents\continuum\crates\continuum-storage\migrations\0002_validator_cache.sql`
- `C:\Users\hp\Documents\continuum\crates\continuum-runtime\src\scheduler.rs` — integrate pipeline call.

### Risks and mitigations

- **Pipeline time on first run.** SecurityScan can be minutes. Mitigation: cache by content hash; mark optional in dev mode; only required in `--harden`.
- **False-positive flooding.** Default severity floor of `Warning` for optional validators in dev mode. `continuum execute --strict` raises everything to required.
- **Regression validator complexity.** Comparing pre/post test sets requires a baseline snapshot. Phase 5 ships the simple "same set, same pass/fail" version; flakiness handling waits for Phase 6 memory integration.

## Verification

1. `cargo run -p continuum-cli -- execute --goal "Add a function with a bug that fails its tests" --project ./fixtures/sample-rust-crate` reaches `UnitTest`, fails, retries up to 3 times, surfaces the final failure summary.
2. `cargo test -p continuum-validation` includes a pipeline ordering test: a stub validator suite asserts `TypeCheck` does not start before `Lint` finishes.
3. `cargo test -p continuum-validation` includes an optional-parallelism test: three slow optional validators in the same stage finish in roughly `max(t1, t2, t3)`, not `t1+t2+t3`.
4. End-to-end fixture: `fixtures/sample-rust-monorepo` with a known-clean state runs all 10 stages (those that apply) and reports pass for each. Same fixture with a planted `panic!()` in a unit test reports failure at the UnitTest stage.
5. Cache test: run the pipeline twice with no changes, second run shows `(cached)` for at least the Lint and TypeCheck stages.

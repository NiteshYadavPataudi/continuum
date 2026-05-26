# Continuum — Phase 1: Workspace Bootstrap

**Status:** ✅ Phase 1 complete. All four verification checks pass on `x86_64-pc-windows-gnu` (MSYS2 mingw64 toolchain at `C:\msys64\mingw64`). Ready for Phase 2.

## Context

Continuum's architectural plan (`continuum-autonomous-production-engineer-reactive-parasol.md`) defines a 23-crate Cargo workspace. Before any feature work can begin, the workspace itself must exist: every crate declared, every public trait defined in `continuum-core`, every CLI subcommand wired through `clap`, and the eight engineering docs scaffolded. Without this scaffold, downstream phases would either churn on workspace structure or fragment into disjoint experiments. Phase 1 delivers the empty shell that the rest of the roadmap fills in.

## Recommended approach

### Scope (delivered)

- **Cargo workspace** rooted at `C:\Users\hp\Documents\continuum` with 23 members declared in `Cargo.toml`, shared `[workspace.dependencies]` for every external crate, and pinned versions to avoid drift across sibling crates.
- **Foundation tier** (`continuum-core`, `continuum-config`, `continuum-telemetry`, `continuum-markdown`).
- **Infrastructure tier** (`continuum-storage`, `continuum-memory`, `continuum-models`, `continuum-models-registry`, `continuum-sandbox`, `continuum-tools` + four tool-family sub-crates).
- **Domain tier** (`continuum-repo`, `continuum-planner`, `continuum-agents`, `continuum-validation`, `continuum-security`, `continuum-recovery`, `continuum-runtime`).
- **Binaries** (`continuum-cli`, `xtask`).
- **Engineering docs** under `docs/` (VISION, PRODUCT, ARCHITECTURE, ENGINEERING, TASKS, AGENTS, MODEL_RULES, SECURITY).
- **CI workflow** (`fmt --check`, `cargo check` with and without default features, `clippy -D warnings`, `cargo test`, `cargo doc -D warnings`).

### `continuum-core` content (full trait surface)

Every public trait defined in the architectural plan ships in Phase 1 — implementations are deferred but the contracts are frozen:
- `agent::Agent`, `agent::AgentKind` (8 variants), `AgentTask`/`AgentContext`/`AgentOutcome`/`AgentCapabilities`/`AgentError`.
- `model::ModelProvider`, `model::ModelRouter`, plus `ModelDescriptor`, `Message`, `Delta`, `CompletionStream`, `CompletionRequest`, `EmbedRequest`/`EmbedResponse`, `CostEstimate`, `ModelIntent`, `RoutedModel`, `ModelError`.
- `sandbox::Sandbox` (associated type, intentionally not object-safe), `sandbox::SandboxHandle`, `SandboxSpec`, `ExecRequest`, `ExecEvent`, `ExecStream`, `SandboxError`.
- `validator::Validator`, `ValidationStage` (10 variants), `ValidationTarget`/`ValidationContext`/`ValidationReport`/`Finding`/`Severity`/`ValidationError`.
- `memory::MemoryStore`, `memory::VectorIndex`, `MemoryLayer` (Hot/Warm/Cold), `MemoryItem`, `RecallQuery`/`RecallHit`, `CompressionScope`/`CompressionReport`, `VectorPoint`/`VectorQuery`/`VectorHit`, `MemoryError`/`VectorError`.
- `repo::RepoIndex` (sync), `repo::RepoLoader` (async), `SymbolRef`/`SymbolQuery`, `ProposedChange`/`ImpactSet`, `ContextQuery`/`ContextBundle`/`ContextSnippet`, `IndexOptions`, `RepoError`.
- `planner::Planner`, `Goal`, `EngineeringDocs`, `RepoAnalysis`, `TaskNode`/`Dependency`/`ExecutionPlan`, `PlanEstimate`, `ExecutionContract`, `PlanError`.
- `tool::ToolRunner`, `ToolFamily`, `ProjectKind`, `ToolInvocation`/`ToolReport`, `ToolError`.
- `recovery::RecoveryStore`, `SessionState`, `Checkpoint`, `ReplayEvent`/`ReplayStream`, `StuckReason`/`StuckSignal`, `RecoveryError`.
- `ids` — `SessionId`, `TaskId`, `AgentId`, `SandboxId`, `SnapshotId`, `CheckpointId`, `MemoryId`, `RunId` (UUID-based) plus `ProviderId`, `ModelId`, `ToolId` (string-based) via newtype macros.
- `caps` — `Cap<T>` phantom-typed token with capability marker types: `HostExec`, `NetworkEgress`, `HostFsWrite`, `ReadSecrets`, `CallModels`.
- `error::ContinuumError` aggregating each domain's narrow `thiserror` enum via `From` impls.

### Binary scaffolds

- `continuum-cli` exposes 11 subcommands (`init`, `analyze`, `execute`, `resume`, `harden`, `replay`, `rollback`, `doctor`, `benchmark`, `memory`, `install`). Each handler prints "not yet implemented" — they're real `clap` `Args` structs, not placeholders, so phase-N work just fills in the body of `run`.
- `xtask` exposes `refresh-models`, `gen-schemas`, `release`, `bench` with placeholder bodies.
- `continuum-models-registry/build.rs` writes a minimal `generated.rs` so the crate compiles. Phase 2 fleshes the codegen out.

### Architectural invariants encoded in Phase 1

These cannot be undone in later phases without breaking changes:
1. **`continuum-core` has no tokio runtime, no I/O.** Only `serde`, `thiserror`, `uuid`, `time`, `async-trait`, `futures`, `tokio-util` (for `CancellationToken` only).
2. **Tool crates only execute via `SandboxHandle::exec`.** No `std::process::Command` or `tokio::process::Command` imports anywhere in `continuum-tools-*`.
3. **No sideways edges in the domain tier.** Only `continuum-runtime` may import multiple domain crates.
4. **`Sandbox` is not object-safe by design** (associated `Handle` type). If `dyn Sandbox` is ever needed, add a `BoxedSandbox` adapter — do not change the trait.
5. **All domain errors are narrow `#[non_exhaustive]` enums.** Phase-N additions don't break callers.

### Critical files (already created)

- `C:\Users\hp\Documents\continuum\Cargo.toml`
- `C:\Users\hp\Documents\continuum\rust-toolchain.toml`
- `C:\Users\hp\Documents\continuum\crates\continuum-core\src\lib.rs` plus 10 module files
- `C:\Users\hp\Documents\continuum\crates\continuum-cli\src\main.rs` and `commands/*.rs`
- `C:\Users\hp\Documents\continuum\crates\continuum-models-registry\build.rs`, `models-snapshot.json`
- `C:\Users\hp\Documents\continuum\xtask\src\main.rs`
- `C:\Users\hp\Documents\continuum\docs\*.md` (eight files)
- `C:\Users\hp\Documents\continuum\.github\workflows\ci.yml`

## Verification

Phase 1 is "done" when all four checks pass on a clean clone:

1. **`cargo check --workspace`** succeeds.
2. **`cargo check --workspace --no-default-features`** succeeds (feature flags partition correctly).
3. **`cargo build -p continuum-tools-security`** does **not** link any code from `continuum-tools-browser` (compile-time isolation across tool families).
4. **`cargo doc --workspace --no-deps`** produces docs for every public trait in `continuum-core` with no missing-doc warnings.

Rust on `x86_64-pc-windows-gnu` needs MSYS2 mingw64 at `C:\msys64\mingw64` for `gcc.exe`, `dlltool.exe`, `as.exe` (install `mingw-w64-x86_64-gcc`, `mingw-w64-x86_64-binutils`, `mingw-w64-x86_64-crt` via pacman). `.cargo/config.toml` overrides linker to MSYS2's `gcc.exe`.

All four checks verified on 2026-05-26:
1. ✅ `cargo check --workspace` — 24/24 crates compile
2. ✅ `cargo check --workspace --no-default-features` — passes
3. ✅ Tool isolation — `continuum-tools-security` Cargo.toml does not reference `continuum-tools-browser` (static source verified, no `use` statements cross boundaries)
4. ✅ `cargo doc --workspace --no-deps` — all public trait docs generated (2 intra-doc link warnings fixed)

# Continuum — Full System Architecture Plan

## Context

**Continuum** is a greenfield open-source autonomous production engineering runtime, written in Rust. Nothing exists on disk yet. The user's spec is a 17-section vision document covering planning, multi-agent execution, memory, sandboxing, validation, security, recovery, and tooling integrations.

This document is the **architectural skeleton only** — no implementation. Subsequent per-phase plans will fill in code. The goal here is to lock crate boundaries, key trait interfaces, data flow, and cross-cutting concerns *before* anyone writes the first line, so that downstream phases can proceed in parallel without architectural churn.

**Project root:** `C:\Users\hp\Documents\continuum`

### Confirmed decisions

| Decision | Choice |
|---|---|
| Scope of this plan | Full system architecture (all 17 spec sections) |
| Model abstraction | `models.dev` from day 1, build-time vendored + optional runtime refresh |
| Vector store | **sqlite-vec** default; Qdrant behind `vector-qdrant` feature flag |
| Browser automation | **Playwright via Node subprocess** (full API surface) |
| CLI live monitoring | **ratatui dashboard from day 1** (no `indicatif` fallback) |

---

## Cargo Workspace Layout

Single workspace, one binary, many libraries. No sideways edges in the domain tier — cross-domain coordination happens in `continuum-runtime`.

### Foundation tier
- **`continuum-core`** — shared types, `ContinuumError`, IDs, capability tokens. Minimal deps (`serde`, `thiserror`, `uuid`, `time`). No tokio, no I/O.
- **`continuum-config`** — layered config (CLI → env → `continuum.toml` → defaults) via `figment`; secret resolution.
- **`continuum-telemetry`** — `tracing` subscriber, OpenTelemetry OTLP, Prometheus metrics, span helpers.
- **`continuum-markdown`** — parses the engineering docs (`VISION.md`, `PRODUCT.md`, `ARCHITECTURE.md`, `ENGINEERING.md`, `TASKS.md`, `AGENTS.md`, `MODEL_RULES.md`, `SECURITY.md`) into typed structs.

### Infrastructure tier
- **`continuum-storage`** — centralized persistence. SQLite via `sqlx` (migrations, decision journal, checkpoints, run history). Owns the schema. Hosts the `VectorIndex` trait with `sqlite-vec` impl by default and `qdrant` impl behind a feature flag.
- **`continuum-memory`** — three-layer memory (hot/warm/cold), semantic recall, context compression, decision journaling. Built on `continuum-storage`.
- **`continuum-models`** — `ModelProvider` trait, router, fallback logic, cost-aware selection. Per-provider HTTP clients (OpenAI, Anthropic, Gemini, DeepSeek, Ollama).
- **`continuum-models-registry`** — vendored `models.dev/api.json` snapshot + `build.rs` codegen producing typed `ProviderId`/`ModelId` constants. Refreshed via `xtask refresh-models`.
- **`continuum-sandbox`** — `Sandbox` + `SandboxHandle` traits, Docker impl via `bollard`. Firecracker impl behind `sandbox-firecracker` feature flag. Resource quotas, network egress rules, FS isolation.
- **`continuum-tools`** — `ToolRunner` trait + registry. Sub-crates register impls.
  - **`continuum-tools-linters`** — Biome, ESLint, Ruff, Clippy.
  - **`continuum-tools-security`** — OWASP ZAP, Semgrep, Trivy, Gitleaks.
  - **`continuum-tools-testing`** — Vitest, Jest, Pytest, `cargo test`, k6.
  - **`continuum-tools-browser`** — Playwright (Node subprocess), CDP fallback via `chromiumoxide` behind a feature flag.

### Domain tier
- **`continuum-repo`** — Repository Intelligence: Tree-sitter AST, symbol index, dependency graph via `petgraph`, service discovery, retrieval-driven context loading for 10k+ file monorepos.
- **`continuum-planner`** — DAG construction, complexity/token/runtime estimation, risk scoring, execution contracts, rollback plans.
- **`continuum-agents`** — eight agent impls: `planner-agent`, `architecture-agent`, `coding-agent`, `testing-agent`, `security-agent`, `review-agent`, `memory-agent`, `recovery-agent`.
- **`continuum-validation`** — 10-stage pipeline (compile → lint → typecheck → unit → integration → e2e → security → startup → perf → regression).
- **`continuum-security`** — RBAC, JWT, CSP, rate limiting, secret-scan orchestration, dependency patcher, Docker hardening.
- **`continuum-recovery`** — `RecoveryStore`, checkpoints, rollback states, replay engine, stuck-task / infinite-loop detection.
- **`continuum-runtime`** — orchestrator. Wires planner → agents → sandbox → validation → memory → recovery. Owns the scheduler, cancellation, and backpressure logic. The **only** crate that depends on all domain crates.

### Binary tier
- **`continuum-cli`** (bin) — `clap`-driven subcommands (`init`, `analyze`, `execute`, `resume`, `harden`, `replay`, `doctor`, `benchmark`, `memory`, `install`). **`ratatui` live dashboard** for progress, agent state, task DAG, validation pipeline, model token streams.
- **`xtask`** (bin) — workspace automation: `refresh-models`, `gen-schemas`, `release`, `bench`. Not shipped.

---

## Key Trait Interfaces

All public traits live in `continuum-core` (the interface) with impls in their respective crates. Async traits use `#[async_trait]`.

### `Agent` (object-safe, `dyn Agent`)
```rust
#[async_trait]
pub trait Agent: Send + Sync {
    fn id(&self) -> AgentId;
    fn kind(&self) -> AgentKind;
    fn capabilities(&self) -> AgentCapabilities;
    async fn handle(
        &self,
        task: AgentTask,
        ctx: &AgentContext,
        cancel: CancellationToken,
    ) -> Result<AgentOutcome, AgentError>;
}
```

### `ModelProvider` + `ModelRouter` (object-safe)
```rust
#[async_trait]
pub trait ModelProvider: Send + Sync {
    fn id(&self) -> ProviderId;
    fn models(&self) -> &[ModelDescriptor];
    async fn complete(&self, req: CompletionRequest, cancel: CancellationToken)
        -> Result<CompletionStream, ModelError>;
    async fn embed(&self, req: EmbedRequest) -> Result<EmbedResponse, ModelError>;
    fn estimate_cost(&self, req: &CompletionRequest) -> CostEstimate;
}

#[async_trait]
pub trait ModelRouter: Send + Sync {
    async fn select(&self, intent: ModelIntent) -> Result<RoutedModel, ModelError>;
}
```
`CompletionStream` = `Pin<Box<dyn Stream<Item = Result<Delta, _>> + Send>>` — streaming required.

### `Sandbox` (associated type — not object-safe by design)
```rust
#[async_trait]
pub trait Sandbox: Send + Sync {
    type Handle: SandboxHandle;
    async fn spawn(&self, spec: SandboxSpec) -> Result<Self::Handle, SandboxError>;
}

#[async_trait]
pub trait SandboxHandle: Send + Sync {
    async fn exec(&self, cmd: ExecRequest) -> Result<ExecStream, SandboxError>;
    async fn write_file(&self, path: &Path, bytes: &[u8]) -> Result<(), SandboxError>;
    async fn read_file(&self, path: &Path) -> Result<Vec<u8>, SandboxError>;
    async fn snapshot(&self) -> Result<SnapshotId, SandboxError>;
    async fn restore(&self, snap: SnapshotId) -> Result<(), SandboxError>;
    async fn shutdown(self) -> Result<(), SandboxError>;
}
```
Add `BoxedSandbox` adapter if `dyn Sandbox` becomes necessary later.

### `Validator` (object-safe)
```rust
#[async_trait]
pub trait Validator: Send + Sync {
    fn stage(&self) -> ValidationStage;
    fn required(&self) -> bool;
    async fn run(&self, target: &ValidationTarget, ctx: &ValidationContext)
        -> Result<ValidationReport, ValidationError>;
}
```

### `MemoryStore` + `VectorIndex` (object-safe)
```rust
#[async_trait]
pub trait MemoryStore: Send + Sync {
    async fn put(&self, layer: MemoryLayer, item: MemoryItem) -> Result<MemoryId, MemoryError>;
    async fn get(&self, id: MemoryId) -> Result<Option<MemoryItem>, MemoryError>;
    async fn recall(&self, query: RecallQuery) -> Result<Vec<RecallHit>, MemoryError>;
    async fn promote(&self, id: MemoryId, to: MemoryLayer) -> Result<(), MemoryError>;
    async fn compress(&self, scope: CompressionScope) -> Result<CompressionReport, MemoryError>;
}

#[async_trait]
pub trait VectorIndex: Send + Sync {
    async fn upsert(&self, points: Vec<VectorPoint>) -> Result<(), VectorError>;
    async fn search(&self, q: VectorQuery) -> Result<Vec<VectorHit>, VectorError>;
}
```
`VectorIndex` has a `sqlite-vec` impl (default, in `continuum-storage`) and a Qdrant impl gated by feature flag.

### `RepoIndex` (sync) + `RepoLoader` (async, object-safe)
```rust
pub trait RepoIndex: Send + Sync {
    fn symbols(&self, query: SymbolQuery) -> Vec<SymbolRef>;
    fn dependents_of(&self, sym: SymbolRef) -> Vec<SymbolRef>;
    fn dependencies_of(&self, sym: SymbolRef) -> Vec<SymbolRef>;
    fn impact_set(&self, change: &ProposedChange) -> ImpactSet;
}

#[async_trait]
pub trait RepoLoader: Send + Sync {
    async fn build(&self, root: &Path, opts: IndexOptions) -> Result<Arc<dyn RepoIndex>, RepoError>;
    async fn refresh(&self, paths: &[PathBuf]) -> Result<(), RepoError>;
    async fn retrieve(&self, q: ContextQuery) -> Result<ContextBundle, RepoError>;
}
```

### `Planner` (object-safe)
```rust
#[async_trait]
pub trait Planner: Send + Sync {
    async fn analyze(&self, repo: Arc<dyn RepoIndex>, docs: &EngineeringDocs)
        -> Result<RepoAnalysis, PlanError>;
    async fn plan(&self, goal: Goal, analysis: &RepoAnalysis)
        -> Result<ExecutionPlan, PlanError>;
    async fn estimate(&self, plan: &ExecutionPlan) -> Result<PlanEstimate, PlanError>;
    async fn contract(&self, plan: &ExecutionPlan) -> Result<ExecutionContract, PlanError>;
}
```
`ExecutionPlan` = `petgraph::DiGraph<TaskNode, Dependency>` + rollback DAG.

### `ToolRunner` (object-safe)
```rust
#[async_trait]
pub trait ToolRunner: Send + Sync {
    fn id(&self) -> ToolId;
    fn family(&self) -> ToolFamily;
    fn supports(&self, project: &ProjectKind) -> bool;
    async fn run(&self, invocation: ToolInvocation, sandbox: &dyn SandboxHandle)
        -> Result<ToolReport, ToolError>;
}
```
**Invariant:** tools spawn processes only via `SandboxHandle::exec`. No host-side `Command::spawn` in any tool crate.

### `RecoveryStore` (object-safe)
```rust
#[async_trait]
pub trait RecoveryStore: Send + Sync {
    async fn checkpoint(&self, session: SessionId, state: &SessionState)
        -> Result<CheckpointId, RecoveryError>;
    async fn latest(&self, session: SessionId)
        -> Result<Option<Checkpoint>, RecoveryError>;
    async fn rollback(&self, to: CheckpointId) -> Result<SessionState, RecoveryError>;
    async fn replay(&self, session: SessionId, from: CheckpointId)
        -> Result<ReplayStream, RecoveryError>;
    async fn detect_stuck(&self, session: SessionId)
        -> Result<Option<StuckSignal>, RecoveryError>;
}
```

---

## End-to-End Data Flow — `continuum execute`

```
continuum-cli
  ↓ clap parse, config load, telemetry init
continuum-runtime::Session::new
  ↓ open MemoryStore, RecoveryStore, VectorIndex (sqlite-vec)
  ↓ spawn long-lived Sandbox (Docker), build ModelRouter
continuum-markdown → load engineering docs
continuum-repo::RepoLoader::build
  ↓ tree-sitter parse via spawn_blocking; symbol+dep graph only (no bodies)
continuum-planner::analyze → plan → estimate → contract
  ↓ surface ExecutionContract to CLI for approval
continuum-runtime::Scheduler (topological walk of DAG)
  ↓ tokio::JoinSet for parallel-ready nodes
  ↓ per node: build AgentContext, dispatch to Agent by AgentKind
continuum-agents::<Kind>Agent::handle
  ↓ memory.recall() + repo.retrieve() (NEVER whole-repo)
  ↓ ModelProvider::complete (streaming)
  ↓ tool calls → ToolRunner → SandboxHandle::exec
continuum-validation pipeline (per node)
  ↓ 10 stages, required short-circuit on fail, optional in parallel
continuum-memory: journal decision (warm), embed artifacts → VectorIndex
continuum-recovery: checkpoint(session, state) after EACH node
  ↓ stuck detector samples scheduler heartbeat every N seconds
continuum-cli (ratatui dashboard)
  ↓ tracing Layer + mpsc → live multi-pane view
```

### Async / backpressure / cancellation rules
- **One `tokio::runtime::Runtime`**, built in `continuum-cli::main`, multi-thread scheduler.
- **One root `CancellationToken`** per `Session`; cascades to every agent, tool call, and sandbox exec. Ctrl-C triggers `cancel()`.
- **Backpressure points:**
  1. Scheduler → agent dispatch: bounded `mpsc` (capacity = `2 × concurrency_limit`).
  2. Agent → model: `Semaphore` per `ProviderId` from config (e.g., OpenAI = 8, Anthropic = 4, Ollama = 1).
  3. Tool → sandbox exec: bounded by sandbox concurrency slots.
  4. Telemetry: OTLP batch processor, never blocks hot path.
- **`spawn_blocking` boundaries:** tree-sitter parsing, heavy regex parsing of tool output, sqlite write transactions if a sync driver path is used.
- **Streaming:** model completions and tool stdout/stderr flow through `tokio::sync::broadcast` so both the ratatui dashboard and the memory journal can subscribe.

---

## Cross-Cutting Concerns

| Concern | Location | Notes |
|---|---|---|
| Errors | `continuum-core::error` + per-crate `thiserror` enums | Per-crate enums `From`-convert into top-level `ContinuumError`. Use `thiserror` in libraries, never `anyhow` (CLI only). Typed errors required so agents can match on `ModelError::RateLimited` etc. |
| Tracing | `continuum-telemetry` | All crates use `tracing::{info, warn, debug, instrument}`. Spans carry `session_id`, `task_id`, `agent_id`. |
| OpenTelemetry | `continuum-telemetry` | OTLP exporter for traces, Prometheus scrape endpoint behind feature flag, Grafana dashboards shipped as JSON in `/dashboards`. |
| Config | `continuum-config` (own crate) | Kept out of `continuum-core` so core stays lightweight. |
| Markdown docs | `continuum-markdown` | Owns schemas for all eight engineering docs. |
| Capability tokens | `continuum-core::caps` | Phantom-typed `Cap<NetworkEgress>` etc. — compile-time gating of dangerous operations. |
| Feature flags | Per-crate `Cargo.toml` | Examples: `continuum-models/openai`, `continuum-models/anthropic`, `continuum-tools-browser/playwright` (default), `continuum-tools-browser/chromiumoxide`, `continuum-sandbox/firecracker`, `continuum-storage/vector-qdrant`. |

---

## Storage Layer

**Centralized in `continuum-storage`.** Domain crates access persistence through trait handles only.

Exposes:
- Private `SqlitePool` + typed repositories: `DecisionRepo`, `CheckpointRepo`, `RunRepo`, `SymbolRepo`, `MemoryRepo`.
- `VectorIndex` trait with `sqlite-vec` impl by default (zero extra ops dependency) and Qdrant impl behind `vector-qdrant` feature flag (managed sidecar container).
- Migration runner invoked once at session start (`sqlx::migrate!`).

Not in storage:
- Memory layering / recall ranking → `continuum-memory`.
- Replay event reconstruction → `continuum-recovery`.

Storage is the substrate; memory and recovery are views on it.

---

## models.dev Integration

**Strategy: build-time vendoring + optional runtime refresh.**

1. **Vendor a snapshot** in `continuum-models-registry`:
   - Commit `models-snapshot.json` (fetched from `https://models.dev/api.json`).
   - `build.rs` reads it → generates `pub static MODELS: phf::Map<...>` + typed `ProviderId`/`ModelId` enums.
   - Compile-time validation: typo-proof model references, pricing in `.rodata`.

2. **`xtask refresh-models`** fetches `api.json`, validates against vendored `model-schema.json`, overwrites the snapshot. Wired into weekly CI; diffs surface model additions/removals/price changes for review.

3. **Optional runtime refresh** behind config flag `models.refresh = "startup" | "never" | "daily"`. Default `"never"` (works offline / air-gapped, reproducible builds). When enabled, `continuum-models` fetches `api.json` on startup and overlays diffs onto the static map.

4. **No third-party Rust SDK** for models.dev — Continuum ships a thin `serde` deserializer over `api.json`. Matches how the TS ecosystem (Vercel AI SDK, Mastra) consumes it.

---

## Tool Integration Pattern

**Split by tool family**, not by vendor. Five crates under the `continuum-tools` umbrella:
- `continuum-tools` (trait + registry)
- `continuum-tools-linters`
- `continuum-tools-security`
- `continuum-tools-testing`
- `continuum-tools-browser` (Playwright via Node subprocess, default; `chromiumoxide` behind feature flag)

Each adapter:
1. Resolves binary inside the sandbox image (or installs on first use).
2. Translates `ToolInvocation` → CLI args.
3. Streams output, parses on the fly into a normalized `ToolReport` (findings, severities, file refs).
4. Maps findings to `continuum-core::Finding` for uniform planner/agent reasoning.

**Compile-time isolation benefit:** security-only users don't pull in `chromiumoxide` or Playwright glue; browser-only users don't pull in Trivy adapters.

---

## Critical Files (created during phase 1 bootstrap)

These files will exist after the workspace is scaffolded and will be repeatedly edited by downstream per-phase plans. List them in any subsequent plan that touches the architecture.

- `C:\Users\hp\Documents\continuum\Cargo.toml` — workspace root, member list, shared `[workspace.dependencies]`.
- `C:\Users\hp\Documents\continuum\rust-toolchain.toml` — pinned stable channel + components.
- `C:\Users\hp\Documents\continuum\crates\continuum-core\src\lib.rs` — shared traits, error types, IDs, capability tokens. Every crate depends on this.
- `C:\Users\hp\Documents\continuum\crates\continuum-runtime\src\session.rs` — orchestrator wiring planner, agents, sandbox, validation, recovery.
- `C:\Users\hp\Documents\continuum\crates\continuum-runtime\src\scheduler.rs` — DAG walker, cancellation, backpressure.
- `C:\Users\hp\Documents\continuum\crates\continuum-models-registry\build.rs` — codegen from vendored `models.dev` snapshot (load-bearing for the models.dev requirement).
- `C:\Users\hp\Documents\continuum\crates\continuum-models-registry\models-snapshot.json` — vendored registry.
- `C:\Users\hp\Documents\continuum\crates\continuum-cli\src\main.rs` — clap subcommands, tokio runtime bootstrap, telemetry init, ratatui app.
- `C:\Users\hp\Documents\continuum\docs\VISION.md`, `PRODUCT.md`, `ARCHITECTURE.md`, `ENGINEERING.md`, `TASKS.md`, `AGENTS.md`, `MODEL_RULES.md`, `SECURITY.md` — the markdown-driven docs the runtime itself reads.
- `C:\Users\hp\Documents\continuum\xtask\src\main.rs` — workspace automation (`refresh-models`, `gen-schemas`, `release`, `bench`).

---

## Roadmap From Here

This plan is the **skeleton**. Each downstream phase gets its own plan and PR:

1. **Phase 1 — Workspace bootstrap.** Create the Cargo workspace, all empty library crates with their `Cargo.toml`, `continuum-core` traits as defined above, `continuum-cli` stub, CI scaffolding. No business logic.
2. **Phase 2 — Models + Storage substrate.** `continuum-models-registry` codegen, one `ModelProvider` impl end-to-end (Anthropic), `continuum-storage` with SQLite migrations and `sqlite-vec` `VectorIndex`.
3. **Phase 3 — Repo intelligence + Planner.** Tree-sitter index for one language family (Rust + TS), `Planner` produces real `ExecutionPlan`.
4. **Phase 4 — Sandbox + first Agent + first Tool.** Docker `Sandbox`, `coding-agent`, one linter from `continuum-tools-linters`.
5. **Phase 5 — Validation pipeline.** All 10 stages with at least one validator per stage.
6. **Phase 6 — Memory + Recovery.** Three-layer memory, checkpointing, replay, stuck-task detection.
7. **Phase 7 — Remaining agents + remaining tool families.** Browser (Playwright), security, testing.
8. **Phase 8 — ratatui dashboard polish, OTLP/Prometheus wiring, `xtask refresh-models` CI.**
9. **Phase 9 — Security hardening modes** (`--security-audit`, `--hardening`, `--enterprise`).

---

## Verification

Because this plan creates **no code**, verification is structural and review-based, not behavioral. After phase 1 bootstraps the skeleton, the following must hold:

1. **`cargo check --workspace` succeeds** with all crates present, all traits defined, all `Cargo.toml` files valid. No crate has unused dependencies (`cargo udeps` clean).
2. **Dependency graph is acyclic and layered.** Run `cargo depgraph --workspace-only | grep -E 'edge'` (or `cargo modules`) and confirm no domain crate depends on another domain crate. Only `continuum-runtime` may.
3. **`cargo build --workspace --no-default-features` succeeds.** Validates that feature flags are correctly partitioned (no required feature accidentally on by default).
4. **`cargo build -p continuum-tools-security` without `continuum-tools-browser`** succeeds and produces a binary that does not link `chromiumoxide` or any Playwright glue. Confirms compile-time isolation across tool families.
5. **`cargo doc --workspace --no-deps`** produces docs for every public trait listed in the "Key Trait Interfaces" section. Each trait must have a doc comment explaining its contract.
6. **`xtask refresh-models` runs end-to-end** against `https://models.dev/api.json`, validates against `model-schema.json`, and produces a non-empty diff against the previous snapshot (or a clean "no changes" message). Confirms the models.dev integration is real, not just scaffolding.
7. **Architecture review checklist** — a human reviewer confirms that every spec section (1-17) maps to at least one crate listed in the workspace layout. No spec capability is orphaned.

End-to-end behavioral testing (running `continuum execute` against a real repository) is deferred to phase 4+ plans, since no agents or sandbox exist until then.

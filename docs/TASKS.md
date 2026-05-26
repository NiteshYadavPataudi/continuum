# Roadmap

The architecture plan splits implementation into nine phases. Each phase has its own follow-up plan and PR.

## Phase 1 — Workspace bootstrap (current)

- [x] Cargo workspace with all member crates declared
- [x] `continuum-core` with full trait definitions, IDs, capability tokens, errors
- [x] Foundation, infrastructure, domain, and binary tier stubs
- [x] `continuum-cli` with clap subcommand scaffolding
- [x] `xtask` automation binary stub
- [x] Engineering docs (this file is one of them)
- [x] CI scaffolding
- [x] `cargo check --workspace` passes
- [x] `cargo check --workspace --no-default-features` passes
- [x] Tool isolation verified (tool sub-crates don't cross-dep)
- [x] `cargo doc --workspace --no-deps` passes
- [x] Phase 1 complete — ready for Phase 2

## Phase 2 — Models + Storage substrate (complete)

- [x] `continuum-models-registry` codegen producing typed `ProviderId` / `ModelId` via `phf::phf_map!`
- [x] First `ModelProvider` impl end-to-end (Anthropic with SSE streaming)
- [x] `continuum-storage` SQLite migrations (6 tables) + memory vector index (cosine similarity)
- [x] `Goal::new()` constructor for `#[non_exhaustive]` compat
- [x] All verification passes: `cargo check`, `cargo clippy -D warnings`, `cargo test`, `cargo doc`

## Phase 3 — Repo intelligence + Planner (complete)

- [x] Markdown engineering doc parsers (8 schemas, YAML frontmatter)
- [x] Regex-based repo index with `petgraph::DiGraph` dependency tracking
- [x] `Planner` producing real `ExecutionPlan` DAGs via `PlanningEngine`
- [x] `continuum analyze` and `continuum execute --dry-run` CLI subcommands
- [x] Full `continuum execute` loop with topological DAG walk + stub agent dispatch
- [x] Stub agents for all 8 `AgentKind`s (ready for Phase 4 real implementations)
- [x] All verification passes: `cargo check`, `cargo clippy -D warnings`, `cargo test`, `cargo doc`
- [ ] Tree-sitter index for Rust + TypeScript (blocked on `x86_64-pc-windows-gnu` C compilation — falls back to regex)
- [x] Phase 3 complete — ready for Phase 4

## Phase 4 — Sandbox + first Agent + first Tool (complete)

- [x] Docker `Sandbox` impl (`DockerSandbox` + `DockerHandle` via bollard)
- [x] `CodingAgent` with model provider support (Anthropic/OpenAI via `ModelProvider` trait)
- [x] Clippy `ToolRunner` with JSON output parsing (`Severity::Error`/`Warning`/`Info`)
- [x] `ToolReport::new()`, `ExecRequest::new()`, `Finding::new()`, `AgentOutcome::new()` constructors
- [x] All verification passes: `cargo check`, `cargo clippy -D warnings`, `cargo test`, `cargo doc`
- [ ] Phase 4 complete — ready for Phase 5

## Phase 5 — Validation pipeline

- All 10 stages with at least one validator per stage

## Phase 6 — Memory + Recovery

- Three-layer memory engine
- Checkpointing, replay, stuck-task detection

## Phase 7 — Remaining agents + tool families

- All remaining subagents
- `continuum-tools-browser` (Playwright via Node subprocess)
- `continuum-tools-security` integrations

## Phase 8 — Live dashboard + telemetry polish

- ratatui live monitor
- OTLP wiring
- Prometheus scrape endpoint
- `xtask refresh-models` wired into CI

## Phase 9 — Security hardening modes

- `--security-audit`
- `--hardening`
- `--enterprise`

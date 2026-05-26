# Continuum — Phase 3: Repo Intelligence + Planner

## Context

With models and storage working (Phase 2), Continuum needs to understand the target repository it's acting on and produce an execution plan. Phase 3 builds the tree-sitter-backed repo index, the markdown engineering-docs parser, and the first real planner that turns a `Goal` into an `ExecutionPlan` DAG. Together they answer two questions at runtime: *what is this repo?* and *what should we do about it?*

Tree-sitter is mandatory for handling 10k+ file monorepos without loading bodies into model context. The planner's output is the contract surfaced to the user for approval before any code changes happen.

## Recommended approach

### A. Markdown engineering docs (`continuum-markdown`)

1. **Schemas.** One typed struct per canonical doc (`VISION`, `PRODUCT`, `ARCHITECTURE`, `ENGINEERING`, `TASKS`, `AGENTS`, `MODEL_RULES`, `SECURITY`). Each accepts optional YAML frontmatter via `gray_matter` plus the markdown body parsed with `pulldown-cmark`.
2. **`EngineeringDocs::load(root: &Path)`** discovers `docs/<NAME>.md` (case-insensitive), parses each, returns a populated `EngineeringDocs` struct (already declared in `continuum-core::planner`). Missing optional docs are tolerated; missing required docs (`ARCHITECTURE`, `MODEL_RULES`) error.
3. **`MODEL_RULES.md` is the only doc with structured semantics in Phase 3.** Its frontmatter declares tier routing (`draft` / `standard` / `deep` → `provider/model-id`). The planner's complexity heuristic consults this.

### B. Repo intelligence (`continuum-repo`)

1. **Language support.** Phase 3 ships Rust + TypeScript only. Other languages (Python, Go, Java) wait for Phase 7. Grammars come from `tree-sitter-rust` and `tree-sitter-typescript` crates.
2. **`RepoLoader::build`** walks the repo via `ignore` (respects `.gitignore`), spawns one parser per language, extracts symbols (functions, structs, classes, traits, modules) via tree-sitter queries living in `crates/continuum-repo/queries/<lang>/symbols.scm`.
3. **Symbol index** is a `BTreeMap<SymbolRef, SymbolNode>` plus a `petgraph::DiGraph` for dependency edges. Stored both in memory (hot) and persisted to `symbols` table in `continuum-storage` for incremental refresh.
4. **Bodies are NOT extracted up front.** Only headers and ranges. `RepoLoader::retrieve(ContextQuery)` is the only path that reads bytes from disk; results respect the token budget and are sorted by relevance (symbol-graph proximity + embedding similarity via `continuum-memory`).
5. **Incremental refresh.** `refresh(paths)` reparses only the listed files, updates symbol rows, recomputes affected edges. CI watches symbol delta size as a regression metric.
6. **CPU offload.** Parsing happens inside `tokio::task::spawn_blocking` because tree-sitter is sync and CPU-bound. The loader exposes async methods but offloads internally.

### C. Planner (`continuum-planner`)

1. **`analyze(repo, docs) -> RepoAnalysis`** runs a multi-prompt sweep against the cheapest tier model:
   - Detect language / framework / package manager.
   - Identify service entry points (binaries, web handlers, main exports).
   - Summarise the dependency graph at module granularity.
   Each sub-result is cached in `continuum-storage` keyed by repo content hash — re-analysing an unchanged repo is free.
2. **`plan(goal, analysis) -> ExecutionPlan`** is a two-step LLM call:
   - **Decompose.** Top-tier model splits the goal into a list of `TaskNode`s with `AgentKind` annotations. Each node lists which symbols / files it expects to touch.
   - **Order.** A deterministic post-processor inserts `Dependency::HappensBefore` edges using the symbol impact graph from `continuum-repo`. Soft `Dependency::Informs` edges come from the LLM output.
   - Output validates against an in-process JSON schema; malformed plans round-trip back to the model up to 3 times.
3. **`estimate(plan) -> PlanEstimate`** uses Phase 2's `MODELS` map: sum estimated tokens per node × per-tier pricing. Wall-clock estimate is a linear function of nodes + parallelism factor from the DAG.
4. **`contract(plan) -> ExecutionContract`** produces the human-readable summary lines. Each summary line cites the symbols/files involved so the user can spot scope creep.
5. **Rollback DAG.** Phase 3 ships an empty `rollback_edges` vector — real rollback planning comes in Phase 6 alongside checkpoints. The trait shape is already correct.

### D. CLI wiring

`continuum analyze` becomes real:
- Loads `EngineeringDocs::load(project)`.
- Builds the repo index (caches result on disk).
- Prints `RepoAnalysis.summary` plus the detected services and a table of top-symbol-by-incoming-edges.

`continuum execute --dry-run` (new flag) runs `analyze + plan + estimate + contract` and prints the contract without scheduling execution. Useful for iteration.

### Critical files

- `C:\Users\hp\Documents\continuum\crates\continuum-markdown\src\schemas\*.rs` — one per doc.
- `C:\Users\hp\Documents\continuum\crates\continuum-markdown\src\lib.rs` — `EngineeringDocs::load`.
- `C:\Users\hp\Documents\continuum\crates\continuum-repo\src\loader.rs` — `RepoLoader` impl.
- `C:\Users\hp\Documents\continuum\crates\continuum-repo\src\index.rs` — `RepoIndex` impl.
- `C:\Users\hp\Documents\continuum\crates\continuum-repo\src\retrieve.rs` — context retrieval.
- `C:\Users\hp\Documents\continuum\crates\continuum-repo\queries\rust\symbols.scm` — tree-sitter query.
- `C:\Users\hp\Documents\continuum\crates\continuum-repo\queries\typescript\symbols.scm` — tree-sitter query.
- `C:\Users\hp\Documents\continuum\crates\continuum-planner\src\analyze.rs` — repo analysis.
- `C:\Users\hp\Documents\continuum\crates\continuum-planner\src\plan.rs` — DAG construction.
- `C:\Users\hp\Documents\continuum\crates\continuum-planner\src\estimate.rs` — token/cost/risk.
- `C:\Users\hp\Documents\continuum\crates\continuum-cli\src\commands\analyze.rs` — first real handler.
- `C:\Users\hp\Documents\continuum\crates\continuum-cli\src\commands\execute.rs` — `--dry-run` path.

### Risks and mitigations

- **Tree-sitter grammar size at compile time.** Rust + TypeScript grammars together add ~5 MB to the binary. Acceptable; revisit only if Phase 7 brings the total above 30 MB.
- **Plan JSON drift.** Strict schema validation + retry-with-error-message keeps malformed LLM output recoverable.
- **Cache invalidation.** Repo-content hash uses fast `xxhash` of the symbol-name list, not file contents — survives whitespace-only edits.

## Verification

1. `cargo run -p continuum-cli -- analyze --path ./fixtures/sample-rust-monorepo` produces a `RepoAnalysis` with non-empty services and a symbol count matching the fixture's known total (±5%).
2. `cargo run -p continuum-cli -- execute --dry-run --goal "Add a /health endpoint"` against the fixture produces an `ExecutionContract` whose summary mentions the right files.
3. `cargo test -p continuum-repo` includes an incremental-refresh test: index a 200-file repo, mutate one file, call `refresh([file])`, assert that exactly the symbols in that file were re-parsed.
4. `cargo test -p continuum-planner` includes a plan-shape test: synthetic `RepoAnalysis` + canned LLM responses (recorded fixtures) produce a deterministic DAG.
5. Memory budget: `continuum analyze` on a 10k-file repo stays under 500 MB RSS. CI tracks the value across runs.

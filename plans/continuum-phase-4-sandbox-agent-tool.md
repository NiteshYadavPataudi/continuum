# Continuum — Phase 4: Sandbox + First Agent + First Tool

## Context

Phases 2 and 3 give us calls (models), storage, and plans. Phase 4 is where Continuum first *executes*: spawns a sandbox, runs an agent that actually edits code, and runs a tool that grades the result. End of Phase 4, `continuum execute` against a small fixture should: produce a plan, get user approval, spawn a Docker sandbox, have the coding-agent generate a one-file change, run Clippy on it, and report success or failure.

This phase touches more crates than any other — but each only minimally. The thinnest vertical slice that proves the architecture works end-to-end.

## Recommended approach

### A. `continuum-sandbox` Docker backend

1. **`DockerSandbox`** implements `continuum_core::sandbox::Sandbox`. Uses `bollard` to talk to the local Docker daemon. Spec → container creation → mount workspace as `/workspace`.
2. **`DockerSandboxHandle`** implements `SandboxHandle`. `exec` streams stdout/stderr via `bollard::container::AttachContainerOptions` with `stream=true`, demuxes Docker's multiplexed frames into `ExecEvent::Stdout` / `ExecEvent::Stderr`, terminates with `ExecEvent::Exit(code)` on container exit.
3. **`snapshot` / `restore`** use Docker's `commit` / image export. Cheap enough for per-task checkpoints in a small workspace; if too slow, Phase 6 swaps in btrfs/zfs reflink snapshots on Linux hosts.
4. **Resource limits.** `SandboxSpec::cpu_quota` → `HostConfig.cpu_period`/`cpu_quota`; `memory_mb` → `HostConfig.memory`; `network_egress=false` → `NetworkMode: "none"`.
5. **Base image.** A `Dockerfile` in `crates/continuum-sandbox/images/runtime/` produces `continuum/runtime:dev` — Ubuntu + Node 20 + Python 3.12 + rustup + curl + git + a non-root `continuum` user. CI builds and pushes this on tag.
6. **Lifecycle.** Sandboxes are session-scoped (created once in `Session::new`, shut down in `Session::drop`). Per-task isolation is achieved through snapshots, not new containers — keeps startup latency negligible after the first task.

### B. First agent: `coding-agent`

1. **Lives in `continuum-agents`** as `pub struct CodingAgent`. Implements `continuum_core::agent::Agent` with `kind() == AgentKind::Coding`.
2. **`handle(task, ctx, cancel)`** runs a model → tool-call loop:
   - Build prompt from `task.payload` (goal description, target files) + retrieved context from `RepoLoader::retrieve`.
   - Call `ModelProvider::complete` (Anthropic, from Phase 2) at `standard` tier per `MODEL_RULES.md`.
   - Parse tool-call deltas. Three tool calls supported in Phase 4: `read_file`, `write_file`, `run_clippy`.
   - `read_file` / `write_file` resolve to `SandboxHandle::read_file` / `write_file`.
   - `run_clippy` resolves to the Clippy tool runner below.
   - Loop terminates when the model emits `<done/>` or budget is exhausted.
3. **`AgentContext`** is populated by `continuum-runtime`. Phase 4 fills it in for real: model handle, sandbox handle, repo loader, memory store, capability tokens.
4. **Cancellation discipline.** Every loop iteration checks `cancel.is_cancelled()` between model call and tool execution. Long tool calls accept the same token via the runtime's tool dispatcher.
5. **Artifacts.** `AgentOutcome.artifacts` carries a JSON diff (file → before/after hashes) so `continuum-validation` and `continuum-recovery` can journal the change.

### C. First tool: Clippy (`continuum-tools-linters`)

1. **`ClippyRunner`** implements `continuum_core::tool::ToolRunner` with `family() == ToolFamily::Linter`.
2. **`supports(project)`** returns true iff `project.language == "rust"`.
3. **`run(invocation, sandbox)`** invokes `cargo clippy --workspace --message-format=json -- -D warnings` via `SandboxHandle::exec`. Streams JSON output, parses each `compiler-message` event into `Finding` rows mapped to file/line/severity.
4. **No host process spawning.** All execution is through `sandbox`. Enforced by code review now; Phase 9 adds a lint that rejects `std::process::Command` imports in tool crates.

### D. Runtime wiring (`continuum-runtime`)

1. **`Session::new(config, project)`** opens the storage handle (Phase 2), builds the model router (Phase 2), opens the repo loader (Phase 3), spawns the Docker sandbox (Phase 4), constructs the agent registry (one entry: `AgentKind::Coding → CodingAgent`), constructs the tool registry (one entry: Clippy).
2. **`Scheduler::run(plan, session, cancel)`** does a topological walk of the plan. For each ready node:
   - Pick an agent by `AgentKind`.
   - Build `AgentContext` with handles + capability tokens.
   - Call `agent.handle(...).await`.
   - On success, journal the outcome and proceed.
   - On failure, propagate `ContinuumError` and stop.
3. **Approval.** Before scheduling, `continuum-cli` prints the `ExecutionContract` and asks for confirmation (skippable with `--yes`).
4. **No validation, memory, or recovery integration yet.** Phase 5 adds validation; Phase 6 adds memory + recovery. Phase 4 deliberately ships without them so the slice is small.

### E. CLI

`continuum execute --goal "..." --project <path>` becomes real:
- `analyze` → `plan` → `estimate` → `contract` (Phase 3 path).
- User approval (or `--yes`).
- `Scheduler::run`.
- Print final diff summary.

### Critical files

- `C:\Users\hp\Documents\continuum\crates\continuum-sandbox\src\docker\sandbox.rs`
- `C:\Users\hp\Documents\continuum\crates\continuum-sandbox\src\docker\handle.rs`
- `C:\Users\hp\Documents\continuum\crates\continuum-sandbox\images\runtime\Dockerfile`
- `C:\Users\hp\Documents\continuum\crates\continuum-agents\src\coding.rs`
- `C:\Users\hp\Documents\continuum\crates\continuum-agents\src\context.rs` (the real `AgentContext`)
- `C:\Users\hp\Documents\continuum\crates\continuum-tools-linters\src\clippy.rs`
- `C:\Users\hp\Documents\continuum\crates\continuum-runtime\src\session.rs` (flesh out)
- `C:\Users\hp\Documents\continuum\crates\continuum-runtime\src\scheduler.rs` (flesh out)
- `C:\Users\hp\Documents\continuum\crates\continuum-runtime\src\registry.rs` (new — agent + tool registries)
- `C:\Users\hp\Documents\continuum\crates\continuum-cli\src\commands\execute.rs` (real handler)

### Risks and mitigations

- **Docker daemon availability.** `Session::new` runs a probe (`bollard::Docker::ping`) and surfaces a useful error pointing to `continuum doctor`. Phase 4 adds Docker checks to `doctor`.
- **Tool-call parsing brittleness.** Use Anthropic's native tool-use API (not freeform XML) — schema-validated by the provider.
- **Snapshot cost.** If commit-image snapshots are slow on Windows hosts, fall back to `tar` of `/workspace` to a host volume per checkpoint. Decision deferred to Phase 6.

## Verification

1. `cargo test -p continuum-sandbox` includes a Docker-required test: spawn, `exec("echo hi")`, assert `Stdout(b"hi\n")` + `Exit(0)`. Gated behind a `CONTINUUM_TEST_DOCKER=1` env var so CI without Docker still passes.
2. `cargo run -p continuum-cli -- execute --yes --goal "Add a function `add(a,b)` to src/lib.rs" --project ./fixtures/empty-rust-crate` produces a non-empty diff, runs Clippy, exits 0.
3. The same command with `--goal "Introduce an unused variable named _x"` runs Clippy, surfaces a finding, the agent attempts a fix on the next iteration, and the run terminates only when Clippy is clean (or budget exhausted).
4. Snapshot/restore roundtrip test: create a sandbox, write `/workspace/foo`, snapshot, write `/workspace/bar`, restore, assert `bar` is gone and `foo` is present.
5. Cancellation test: `tokio::select!` between `Scheduler::run` and a token cancel after 100 ms; assert the run returns `AgentError::Cancelled` within 500 ms (no orphaned containers).

# Continuum — Phase 7: Remaining Agents + Tool Families

## Context

By end of Phase 6, Continuum runs end-to-end with one agent (coding), one provider (Anthropic), one full tool family (linters via Clippy/Biome), one validator per stage, and a working memory + recovery layer. The architectural plan calls for *eight* agents and *four* tool families. Phase 7 fills the gap.

This is the phase where Continuum stops being a Rust-only coding tool and becomes a real multi-language autonomous engineer. It also adds the security and browser surfaces that distinguish it from existing AI coding assistants.

## Recommended approach

### A. Remaining six agents

Each lives in `continuum-agents/src/<name>.rs` and follows the `coding-agent` pattern: model → tool-call loop, retrieval-driven context, capability-gated.

1. **`PlannerAgent`** (`AgentKind::Planner`) — wraps the Phase 3 `continuum-planner` so the planner itself can be invoked as a sub-task (e.g., for re-planning after a stuck signal). Distinct from the planner *crate* — the crate is the engine, the agent is the API surface.
2. **`ArchitectureAgent`** (`AgentKind::Architecture`) — reads `ARCHITECTURE.md`, the dependency graph, and the proposed change; emits architectural findings (e.g., "this introduces a sideways edge between domain crates"). Runs at the start of each task node as a soft-gate.
3. **`TestingAgent`** (`AgentKind::Testing`) — given a recently changed symbol, proposes test cases and writes them. Calls into `continuum-tools-testing` runners to verify locally before claiming done.
4. **`SecurityAgent`** (`AgentKind::Security`) — owns the `--harden` modes (deferred to Phase 9). In Phase 7, ships only the audit path: invoke security tool runners, surface findings, suggest remediations.
5. **`ReviewAgent`** (`AgentKind::Review`) — reads the diff produced by the coding-agent, comments on maintainability and obvious optimisations. Non-blocking: contributes warnings, never errors.
6. **`RecoveryAgent`** (`AgentKind::Recovery`) — already had Phase 6 stub behaviour; in Phase 7 gains the ability to read replay events and propose a fix when `detect_stuck` fires. Output is a new plan slice, not raw code.

`MemoryAgent` (Phase 6) and `CodingAgent` (Phase 4) round out the eight.

### B. Browser tool family (`continuum-tools-browser`)

1. **Default driver: Playwright via Node subprocess.**
   - `PlaywrightRunner` implements `ToolRunner` with `family() == ToolFamily::Browser`.
   - Stages a small Node bootstrap script in the sandbox (`playwright-bootstrap.js`) shipped as a `include_bytes!`-embedded asset.
   - `SandboxHandle::write_file` plants the script, then `SandboxHandle::exec` runs `node playwright-bootstrap.js <invocation-payload>`.
   - Bootstrap reads the invocation from stdin as JSON, drives Playwright accordingly, writes results as JSON to stdout.
   - Supports: navigation, screenshot, network mocking, trace export, full-page snapshot for visual regression.
2. **Sandbox image update.** `continuum/runtime:dev` Dockerfile gains `RUN npx playwright install --with-deps chromium`. ~400 MB image bloat; document it.
3. **`chromiumoxide` fallback** behind feature flag. Same trait surface, dropping the auto-wait niceties. Useful for users who cannot run Node (rare).

### C. Security tool family (`continuum-tools-security`)

Already partially landed in Phase 5 (Semgrep, Trivy, Gitleaks for validation). Phase 7 adds OWASP ZAP:
- **`ZapRunner`** spawns `zaproxy` in headless mode inside the sandbox.
- Requires a live target (e.g., the startup-validator's running app). Wires through `SandboxSpec::network_egress = true` only when the target is loopback.

### D. Testing tool family

Phase 5 shipped cargo test / Vitest / Pytest. Phase 7 adds:
- **`JestRunner`** — Vitest-like adapter for codebases on Jest.
- **`K6LoadRunner`** — fuller k6 wrapper (beyond the baseline validator) for ad-hoc load tests dispatched by `testing-agent`.

### E. Additional model providers

Once multi-language tasks are in scope, single-provider lock-in matters less, but the routing layer benefits from diversity. Phase 7 adds:
- **`OpenAIProvider`** — same shape as Anthropic, different SSE event schema. Feature `openai`.
- **`OllamaProvider`** — local models for cheap-tier draft work. Feature `ollama`. Critical for offline / cost-sensitive setups.
- Gemini and DeepSeek can wait — feature stubs but no impl in Phase 7.

The router (`continuum-models::router`) gains cost-aware fallback: route the `draft` tier to Ollama when available, fall back to Anthropic Haiku on Ollama unavailability.

### F. Multi-language repo intelligence

`continuum-repo` adds parsers for **Python** (`tree-sitter-python`) and **Go** (`tree-sitter-go`). Symbol queries under `crates/continuum-repo/queries/{python,go}/symbols.scm`. Symbol-index storage is language-agnostic; only the parser dispatch is per-language.

### Critical files

- `C:\Users\hp\Documents\continuum\crates\continuum-agents\src\{planner,architecture,testing,security,review,recovery}.rs`
- `C:\Users\hp\Documents\continuum\crates\continuum-tools-browser\src\playwright.rs`
- `C:\Users\hp\Documents\continuum\crates\continuum-tools-browser\src\playwright-bootstrap.js` (embedded asset)
- `C:\Users\hp\Documents\continuum\crates\continuum-tools-browser\src\chromiumoxide.rs` (feature-gated)
- `C:\Users\hp\Documents\continuum\crates\continuum-tools-security\src\zap.rs`
- `C:\Users\hp\Documents\continuum\crates\continuum-tools-testing\src\jest.rs`
- `C:\Users\hp\Documents\continuum\crates\continuum-tools-testing\src\k6.rs`
- `C:\Users\hp\Documents\continuum\crates\continuum-models\src\openai.rs`
- `C:\Users\hp\Documents\continuum\crates\continuum-models\src\ollama.rs`
- `C:\Users\hp\Documents\continuum\crates\continuum-models\src\router.rs` — cost-aware fallback.
- `C:\Users\hp\Documents\continuum\crates\continuum-repo\queries\python\symbols.scm`
- `C:\Users\hp\Documents\continuum\crates\continuum-repo\queries\go\symbols.scm`
- `C:\Users\hp\Documents\continuum\crates\continuum-sandbox\images\runtime\Dockerfile` — Playwright deps.
- `C:\Users\hp\Documents\continuum\crates\continuum-runtime\src\registry.rs` — register the new agents and tools.

### Risks and mitigations

- **Sandbox image bloat.** Playwright + Chromium = ~400 MB. Mitigation: ship a `continuum/runtime:slim` without browser deps; default to it; require `continuum/runtime:full` only when a Playwright-backed validator is in the plan.
- **Provider parity drift.** Each provider's tool-use schema differs. Build an internal `NormalizedToolCall` shape; adapters map provider-native → normalized at the boundary.
- **Ollama heterogeneity.** Local models vary wildly. Pin a known-good model list per task class in `MODEL_RULES.md`; the router refuses Ollama for `deep` tier.

## Verification

1. **Multi-language e2e.** Three fixture projects (Rust, TS+Vitest, Python+Pytest) each pass `continuum execute --goal "Add a hello function and a test"`.
2. **Browser e2e.** Fixture project with a tiny Express server: `continuum execute --goal "Add a /health route and verify with Playwright"` produces an explicit `PlaywrightRunner` invocation, screenshot artifact, and a passing assertion.
3. **Router fallback.** With Ollama unreachable, draft-tier calls fall back to Anthropic Haiku, surfacing a warning span. With Ollama up, draft-tier never hits Anthropic.
4. **Agent dispatch.** All eight `AgentKind` variants resolve to a registered agent in the runtime registry; missing registration is a compile-time error (verified via `match` exhaustiveness in the registry builder).
5. **Sandbox image variants.** `continuum doctor` reports which image variant is in use and warns when Playwright is requested but `:slim` is loaded.

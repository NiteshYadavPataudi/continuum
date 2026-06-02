<p align="center">
  <img src="https://img.shields.io/badge/status-pre--alpha-yellow" alt="pre-alpha"/>
  <img src="https://img.shields.io/badge/rust-1.78%2B-orange" alt="Rust 1.78+"/>
  <img src="https://img.shields.io/badge/license-Apache--2.0%20%7C%20MIT-blue" alt="License"/>
  <img src="https://img.shields.io/badge/crates-24-8A2BE2" alt="24 crates"/>
  <img src="https://img.shields.io/badge/providers-10-green" alt="10 providers"/>
  <img src="https://img.shields.io/badge/tests-72-passing-brightgreen" alt="72 tests"/>
  <img src="https://img.shields.io/badge/CI-passing-brightgreen" alt="CI passing"/>
</p>

<h1 align="center">Continuum</h1>

<p align="center">
  <strong>Autonomous production engineering runtime — written in Rust.</strong><br>
  A natural-language goal → production-ready, security-hardened, test-passing result.
</p>

---

## Overview

Continuum is a **multi-agent autonomous engineering runtime** that takes a natural-language goal and drives it through planning, implementation, validation, security hardening, and recovery — without step-by-step human guidance.

```sh
continuum execute --goal "Add rate-limiting middleware to the API"
```

This triggers:
1. **Repository analysis** via tree-sitter symbol graphs
2. **LLM-driven planning** decomposing the goal into an execution DAG
3. **Architecture review** against your `ARCHITECTURE.md`
4. **Code generation** via the CodingAgent
5. **Test generation** via the TestingAgent
6. **10-stage validation** (compile → lint → typecheck → unit → integration → e2e → security → startup → performance → regression)
7. **Security hardening** with semgrep + trivy + gitleaks
8. **Code review** of the produced diff
9. **Memory compression** (hot → warm → cold tiers)
10. **Checkpoint** at every step — `resume` from any failure

---

## Features

| Capability | Details |
|---|---|
| **8 specialized agents** | Planner, Architecture, Coding, Testing, Security, Review, Memory, Recovery — coordinated by a DAG scheduler |
| **10-stage validation pipeline** | Compile → Lint → TypeCheck → Unit → Integration → E2E → SecurityScan → Startup → Performance → Regression |
| **Rust + TypeScript + Python** | Language-aware validation with Biome, Vitest, pytest, cargo |
| **10 LLM providers** | Anthropic, OpenAI, Gemini, Groq, DeepSeek, Mistral, Cohere, Together, Fireworks, Ollama |
| **40+ models** | Vendored model registry, build-time codegen, zero-overhead lookups |
| **3-tier memory engine** | Hot (SQLite) → Warm (LLM summaries) → Cold (vector embeddings) |
| **Crash recovery** | Checkpoint after every plan node; resume, replay, and rollback |
| **Security hardening** | 3 modes: audit / hardening / enterprise with compliance attestation |
| **Docker sandbox** | All tool execution inside isolated containers |
| **Live TUI dashboard** | 6-pane ratatui interface with real-time cost, token, validation, and live agent execution tracking |
| **OpenTelemetry + Prometheus** | OTLP export, 4 Grafana dashboards |
| **CLI + REPL + TUI** | Three interaction modes — REPL wired to full planning+execution, TUI with live scheduler events |

---

## Quick Start

```sh
# 1. Configure your API key
continuum config set anthropic.api_key sk-ant-...

# 2. Scaffold engineering docs
continuum init

# 3. Verify your environment
continuum doctor

# 4. Launch interactive REPL
continuum

# 5. Or execute a goal directly
continuum execute --goal "Add a /health endpoint with uptime and version"

# 6. Resume from failures
continuum resume

# 7. Run a security audit
continuum harden --mode audit

# 8. Launch full-screen TUI
continuum --tui
```

---

## Installation

### Prerequisites

| Dependency | Version | Purpose |
|---|---|---|
| [Rust](https://rustup.rs) | 1.78+ | Build the workspace |
| [Docker](https://docker.com) | 24+ | Sandbox execution (required) |
| Node.js | 18+ | TypeScript validation |
| Python | 3.9+ | Python validation |
| [Semgrep](https://semgrep.dev) | latest | Static security analysis |
| [Trivy](https://aquasecurity.github.io/trivy) | latest | Vulnerability scanning |
| [Gitleaks](https://gitleaks.io) | latest | Secret detection |

### Build from source

```sh
git clone https://github.com/continuum-rs/continuum.git
cd continuum
cargo build --release
./target/release/continuum doctor
```

### Installer

**Windows**
```powershell
irm https://raw.githubusercontent.com/continuum-rs/continuum/main/install.ps1 | iex
```

**macOS / Linux**
```sh
curl -fsSL https://raw.githubusercontent.com/continuum-rs/continuum/main/install.sh | bash
```

### Environment variables

- `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `GEMINI_API_KEY`, etc. — provider-specific keys
- `CONTINUUM_<PROVIDER>_API_KEY` — generic override for any provider
- `CONTINUUM_OTLP_ENDPOINT` — enable OpenTelemetry export
- `CONTINUUM_METRICS_ADDR` — Prometheus metrics address (default `127.0.0.1:9477`)

---

## CLI Usage

### Modes

| Mode | Example | Description |
|---|---|---|
| **Subcommand** | `continuum execute --goal "..."` | Traditional CLI subcommands |
| **Direct goal** | `continuum "add rate limiting"` | Execute without subcommand |
| **REPL** | `continuum` | Interactive shell with history |
| **TUI** | `continuum --tui` | Full-screen terminal dashboard |

### Commands

| Command | Description |
|---|---|
| `init` | Scaffold engineering docs (8 files) |
| `analyze` | Analyze repository, produce architecture report |
| `execute --goal "..."` | Run autonomous execution session |
| `resume` | Resume most recent interrupted session |
| `harden --mode <mode>` | Security hardening (audit/hardening/enterprise) |
| `doctor` | Full environment diagnostics (tools, API keys, storage, vectors) |
| `doctor --fix` | Show setup guidance for missing dependencies |
| `doctor --check-stuck` | Detect stuck sessions (retry > 5) |
| `memory <list\|compress\|purge>` | Manage memory engine |
| `config set\|get\|unset\|list\|providers` | Manage provider configuration |
| `login` | Interactive provider setup |
| `install` | Install optional dependencies |
| `benchmark` | Run validation benchmarks |
| `replay <session>` | Replay past session |
| `rollback <session> --to <id>` | Roll back to checkpoint |

### Global options

| Flag | Description |
|---|---|
| `--project <path>` | Target project directory |
| `-v` / `--verbose` | Enable verbose logging |
| `--tui` | Launch full-screen TUI dashboard |
| `-c` / `--continue` | Continue most recent session |
| `-r` / `--resume <id>` | Resume specific session |
| `--model <name>` | Override model for session |
| `--dry-run` | Build plan and show contract without executing |
| `--yes` | Auto-approve execution contract |

---

## TUI Dashboard

```sh
continuum --tui
continuum execute --goal "..." --follow-tui
```

The live TUI provides real-time visibility into execution:

| Area | Shows |
|---|---|
| **Header bar** | Model name, provider status (`connected`/`no-key`), project, git branch, cost, token count, validation summary, model errors |
| **Timeline** | Scrollable conversation with user/assistant/system/tool messages |
| **Agent sidebar** | Status, current task, elapsed time, tokens used, live cost, validation pass/fail, model errors |
| **Tasks sidebar** | Task count (ready/running/done/failed), per-task progress with percentages, dependency info |
| **Composer** | Multi-line input, slash-command autocomplete, live cost hint bar, model error display |
| **Model selector** | Searchable model list with pricing; `/model` to switch |
| **Model error popup** | Dismissable overlay with troubleshooting tips when API calls fail |

### Key bindings

| Key | Action |
|---|---|
| `Enter` | Send message |
| `Shift+Enter` | Newline (multiline mode) |
| `Tab` | Cycle panels / autocomplete |
| `Escape` | Close panel / cancel |
| `Ctrl+C` / `Ctrl+D` | Exit TUI |
| `Ctrl+T` | Focus tasks panel |
| `Ctrl+L` | Clear conversation |
| `Ctrl+A` | Agent activity panel |
| `Ctrl+X` | Stop current agent |
| `Ctrl+R` | Retry step |
| `/help` | Show all commands |

### Slash commands

| Command | Description |
|---|---|
| `/model` | Switch model |
| `/providers` | List providers with status |
| `/effort <level>` | Set reasoning effort |
| `/cost` | Session token/cost stats |
| `/doctor` | Run diagnostics |
| `/init` | Scaffold engineering docs |
| `/memory` | Inspect memory engine |
| `/harden` | Security audit |
| `/diff` | Show uncommitted changes |
| `/clear` | Clear screen |
| `/exit` | Exit Continuum |

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│  continuum CLI (clap + ratatui TUI + REPL)               │
│  init · analyze · execute · doctor · harden · config     │
└─────────────────────────┬───────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────┐
│              Scheduler (DAG walk + JoinSet)               │
│  Planner │ Coding │ Testing │ Security │ Arch │ Review   │
│  Memory  │ Recovery Agent                                │
└─────────────────────────┬───────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────┐
│  10-stage Validation Pipeline (Rust/TS/Python)            │
│  Compile→Lint→TypeCheck→Unit→Integration→E2E→            │
│  SecurityScan→Startup→Performance→Regression              │
└─────────────────────────┬───────────────────────────────┘
                          │
              ┌───────────┼───────────┐
              │           │           │
┌──────────────▼──┐ ┌─────▼─────┐ ┌──▼──────────────┐
│  Repo           │ │  Memory   │ │  Recovery        │
│  Intelligence   │ │  Hot/Warm │ │  Checkpoint      │
│  (tree-sitter)  │ │  /Cold    │ │  Replay/Resume   │
│  4 langs        │ │  SQLite+  │ │  Heartbeat       │
│  Impact analysis│ │  Vectors  │ │  Audit log       │
└────────────────┘ └───────────┘ └─────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────┐
│  Model Providers (10 providers, 40+ models)               │
│  Anthropic · OpenAI · Gemini · Groq · DeepSeek · Mistral │
│  Cohere · Together · Fireworks · Ollama                  │
└─────────────────────────────────────────────────────────┘
```

---

## Model Providers

| Provider | Env var | Notable models |
|---|---|---|
| Anthropic | `ANTHROPIC_API_KEY` | Claude Sonnet 4/4.5, Haiku 4/4.5, Opus 4.5 |
| OpenAI | `OPENAI_API_KEY` | GPT-4o, GPT-4o Mini, o1, o3-mini |
| Google | `GEMINI_API_KEY` | Gemini 2.5 Pro/Flash, 1.5 Pro/Flash |
| Groq | `GROQ_API_KEY` | Llama 3.3 70B, Mixtral, Gemma2 |
| DeepSeek | `DEEPSEEK_API_KEY` | DeepSeek V3, R1 |
| Mistral | `MISTRAL_API_KEY` | Large, Small, Codestral |
| Cohere | `COHERE_API_KEY` | Command R+, Command R |
| Together | `TOGETHER_API_KEY` | Llama 3.1, Qwen 2.5, Mixtral |
| Fireworks | `FIREWORKS_API_KEY` | Llama 3.3, DeepSeek R1 |
| Ollama | *(none)* | llama3.2, qwen2.5, codellama |

Any OpenAI-compatible API works via `CompatProvider`.

---

## Agent System

| Agent | Role | Sandbox |
|---|---|---|
| `PlannerAgent` | Decompose goals into execution DAG | No |
| `ArchitectureAgent` | Validate design constraints | No |
| `CodingAgent` | Generate code via LLM | No |
| `TestingAgent` | Generate and run tests | Yes |
| `SecurityAgent` | Semgrep/Trivy/Gitleaks/ZAP audits | Yes |
| `ReviewAgent` | Code review (correctness, style, perf) | No |
| `MemoryAgent` | Compress hot → warm summaries | No |
| `RecoveryAgent` | Retry/skip/replan stuck tasks | No |

---

## Validation Pipeline

| # | Stage | Required | Languages |
|---|---|---|---|
| 1 | Compile | Yes | Rust (cargo check) |
| 2 | Lint | Yes | Rust (clippy), TypeScript (Biome) |
| 3 | TypeCheck | Yes | Rust (cargo check --tests) |
| 4 | UnitTest | Yes | Rust, TypeScript (Vitest), Python (pytest) |
| 5 | IntegrationTest | No | Cargo test | |
| 6 | E2ETest | No | Playwright / custom |
| 7 | SecurityScan | Yes | Cargo audit, semgrep, trivy, gitleaks |
| 8 | Startup | No | Sandbox health probe |
| 9 | Performance | No | Build time regression check |
| 10 | Regression | No | Full test suite |

---

## Memory & Recovery

**3-tier memory engine:**

```
Hot  → full SQLite context (recent session)
Warm → LLM-compressed summaries
Cold → vector embeddings for semantic recall
```

**Recovery system:**
- Checkpoints after every plan node
- Heartbeat monitor detects stuck tasks (configurable)
- `continuum resume` — restart from last checkpoint
- `continuum replay <session>` — debug via event stream
- `continuum rollback --to <checkpoint>` — revert state
- Append-only audit log for compliance

---

## Development

```sh
cargo build --workspace
cargo test  --workspace        # 72+ tests
cargo clippy --workspace       # must pass clean -D warnings
cargo fmt --all -- --check     # must pass clean
cargo doc --workspace --no-deps # must pass -D warnings

cargo xtask refresh-models     # fetch latest model registry
cargo xtask gen-schemas        # generate JSON schemas
cargo xtask security-lint      # check Cap::grant() usage
cargo xtask release            # release readiness checks
cargo xtask bench              # benchmark + LOC stats
```

### Design principles

- **Capability tokens** (`Cap<T>`) — dangerous operations require compile-time tokens
- **No `unsafe`** — `#![forbid(unsafe_code)]` in every crate
- **All tools in sandbox** — Security scanners (semgrep/trivy/gitleaks) run in workspace directory, not bare host
- **API key safety** — Keys sent via headers only, never in URL query parameters
- **Path traversal prevention** — Agent artifact writes are validated against workspace boundaries
- **Trait-based** — `ModelProvider`, `Agent`, `SandboxHandle`, `Planner` are all async traits
- **Snapshot-driven registry** — model metadata vendored + build-time codegen via `phf`

---

## Workspace

```
crates/continuum-core/           # Foundation traits, types, errors, Cap<T>
crates/continuum-config/         # Layered config (env → config.toml)
crates/continuum-telemetry/      # OTLP + Prometheus + tracing
crates/continuum-markdown/       # Engineering doc parser
crates/continuum-storage/        # SQLite pool + vector backend
crates/continuum-memory/         # 3-tier memory engine
crates/continuum-models/         # 10 provider implementations
crates/continuum-models-registry/# Vendored snapshot + build-time codegen
crates/continuum-sandbox/        # Docker sandbox (Firecracker feature-gated)
crates/continuum-repo/           # Tree-sitter indexer + symbol graph
crates/continuum-planner/        # LLM planning engine + DAG
crates/continuum-agents/         # 8 agent implementations
crates/continuum-validation/     # 10-stage validation pipeline
crates/continuum-security/       # Hardening modes + compliance
crates/continuum-recovery/       # Checkpoints, replay, audit log
crates/continuum-runtime/        # DAG scheduler + session orchestration
crates/continuum-tools/          # Tool registry
crates/continuum-tools-linters/  # Clippy runner
crates/continuum-tools-security/ # Semgrep/Trivy/Gitleaks/ZAP/cargo-audit
crates/continuum-tools-testing/  # Cargo-test/Jest/K6
crates/continuum-tools-browser/  # Playwright/chromiumoxide
crates/continuum-cli/            # Binary + TUI + dashboard + REPL
xtask/                           # Workspace automation
docs/                            # 8 engineering docs
dashboards/                      # 4 Grafana dashboards
fixtures/                        # Test fixtures (Rust, TS, Python)
```

---

## License

Dual-licensed under [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT).

---

<p align="center">
  <a href="docs/ARCHITECTURE.md">Architecture</a> ·
  <a href="docs/TASKS.md">Roadmap</a> ·
  <a href="docs/AGENTS.md">Agents</a> ·
  <a href="docs/SECURITY.md">Security</a>
</p>

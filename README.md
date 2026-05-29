<p align="center">
  <img src="https://img.shields.io/badge/status-pre--alpha-yellow" alt="pre-alpha"/>
  <img src="https://img.shields.io/badge/rust-1.78%2B-orange" alt="Rust 1.78+"/>
  <img src="https://img.shields.io/badge/license-Apache--2.0%20%7C%20MIT-blue" alt="License"/>
  <img src="https://img.shields.io/badge/crates-24-8A2BE2" alt="24 crates"/>
  <img src="https://img.shields.io/badge/providers-10-green" alt="10 providers"/>
</p>

<h1 align="center">Continuum</h1>

<p align="center">
  <strong>Open-source autonomous production engineering runtime — written in Rust.</strong><br>
  Plans, implements, validates, secures, and recovers software at production scale.
</p>

---

## What is Continuum?

Continuum is not a code autocomplete tool. It is a **full-stack autonomous engineering runtime** that takes a natural-language goal and drives it all the way to a production-ready, security-hardened, test-passing result — without human intervention at every step.

```
continuum execute --goal "Add rate-limiting middleware to the API"
```

That single command triggers:
1. **Repository analysis** — tree-sitter symbol graph of your codebase
2. **LLM planning** — goal decomposed into an execution DAG by agent kind
3. **Architecture review** — design validated against your `ARCHITECTURE.md`
4. **Code generation** — CodingAgent implements the change
5. **Test generation** — TestingAgent writes tests
6. **10-stage validation** — compile → lint → typecheck → unit → integration → e2e → security scan → startup → performance → regression
7. **Security hardening** — semgrep + trivy + gitleaks scans
8. **Code review** — ReviewAgent checks the diff
9. **Memory compression** — session context archived from hot → warm → cold tiers
10. **Checkpoint** — state saved at every step so you can `resume` after any failure

---

## Architecture

```
┌─────────────────────────────────────────────────┐
│  continuum CLI  (clap + ratatui live dashboard)  │
│  init · analyze · execute · config · harden …   │
└───────────────────────┬─────────────────────────┘
                        │
┌───────────────────────▼─────────────────────────┐
│            Scheduler  (DAG walk)                 │
│  ┌──────────┐ ┌────────┐ ┌────────┐ ┌────────┐  │
│  │ Planner  │ │ Coding │ │Testing │ │Security│  │
│  │  Agent   │ │ Agent  │ │ Agent  │ │ Agent  │  │
│  └──────────┘ └────────┘ └────────┘ └────────┘  │
│  ┌──────────┐ ┌────────┐ ┌────────┐ ┌────────┐  │
│  │  Arch    │ │ Review │ │ Memory │ │Recovery│  │
│  │  Agent   │ │ Agent  │ │ Agent  │ │ Agent  │  │
│  └──────────┘ └────────┘ └────────┘ └────────┘  │
└───────────────────────┬─────────────────────────┘
                        │
┌───────────────────────▼─────────────────────────┐
│  Validation Pipeline  (10 stages)                │
│  Compile→Lint→TypeCheck→Unit→Integration→        │
│  E2E→SecurityScan→Startup→Perf→Regression        │
└───────────────────────┬─────────────────────────┘
                        │
         ┌──────────────┼──────────────┐
         │              │              │
┌────────▼───┐  ┌───────▼──────┐  ┌───▼──────────┐
│  Repo      │  │  Memory      │  │  Recovery    │
│Intelligence│  │  Hot/Warm/   │  │  Checkpoint  │
│(tree-sitter│  │  Cold tiers  │  │  Replay      │
│  4 langs)  │  │  (SQLite +   │  │  Heartbeat   │
│            │  │   vectors)   │  │              │
└────────────┘  └──────────────┘  └──────────────┘
         │
┌────────▼─────────────────────────────────────────┐
│  Model Providers  (10 providers, 40+ models)     │
│  Anthropic · OpenAI · Gemini · Groq · DeepSeek  │
│  Mistral · Cohere · Together · Fireworks · Ollama│
└──────────────────────────────────────────────────┘
```

---

## Features at a glance

| Capability | Details |
|---|---|
| **Multi-agent orchestration** | 8 specialized agents coordinated by a DAG scheduler |
| **10-stage validation** | Every change must pass compile → regression before merging |
| **3-tier memory** | Hot (full context) → Warm (LLM summaries) → Cold (semantic search) |
| **Crash recovery** | Checkpoint after every plan node; `continuum resume` from any failure |
| **Repo intelligence** | Tree-sitter symbol graphs for Rust, TypeScript, Python, Go |
| **Security hardening** | 3 modes (audit / hardening / enterprise) with automated remediation |
| **10 model providers** | Anthropic, OpenAI, Gemini, Groq, DeepSeek, Mistral, Cohere, Together, Fireworks, Ollama |
| **Docker sandbox** | All tool execution runs inside an isolated container |
| **Live TUI dashboard** | 4-pane ratatui dashboard (plan DAG · agent log · validation · cost) |
| **OpenTelemetry + Prometheus** | OTLP export, Grafana dashboards included |

---

## Prerequisites

| Dependency | Version | Purpose |
|---|---|---|
| [Rust](https://rustup.rs) | 1.78+ | Build the workspace |
| [Docker](https://www.docker.com) | 24+ | Sandbox execution |
| Node.js | 18+ | TypeScript / JavaScript validation |
| Python | 3.9+ | Python validation |
| [Semgrep](https://semgrep.dev) | latest | Static security analysis |
| [Trivy](https://aquasecurity.github.io/trivy) | latest | Vulnerability scanning |
| [Gitleaks](https://gitleaks.io) | latest | Secret detection |

Grafana and Prometheus are optional (for the dashboards).

---

## Installation

### One-line installer

**Windows**

```powershell
irm https://raw.githubusercontent.com/NiteshYadavPataudi/continuum/main/install.ps1 | iex
```

**macOS / Linux**

```sh
curl -fsSL https://raw.githubusercontent.com/NiteshYadavPataudi/continuum/main/install.sh | bash
```

The installer will:

- build and install `continuum-cli`
- place the binary in a dedicated install root
- update your PATH for the current shell
- persist PATH changes in your user shell profile

### Environment variables

- `CONTINUUM_VERSION`: install a specific version instead of `latest`
- `CONTINUUM_INSTALL_DIR`: choose the install root used by the installer
- `INSTALL_DIR`: fallback install root variable supported by both scripts

Default install root:

- Windows: `%USERPROFILE%\.local\continuum`
- macOS/Linux: `~/.local/continuum`

### Build from source

```sh
git clone https://github.com/NiteshYadavPataudi/continuum.git
cd continuum
cargo build --release
./target/release/continuum doctor   # verify environment
```

### From crates.io *(once published)*

```sh
cargo install continuum-cli
```

---

## Quick Start

```sh
# 1. Configure your API key
continuum config set anthropic.api_key sk-ant-...

# 2. Scaffold engineering docs (VISION.md, ARCHITECTURE.md, etc.)
continuum init

# 3. Analyze the repository
continuum analyze

# 4. Run an autonomous session
continuum execute --goal "Add a /health endpoint with uptime and version"

# 5. If it gets interrupted — resume from the last checkpoint
continuum resume

# 6. Run a security audit
continuum harden --mode audit
```

---

## CLI Commands

| Command | Description |
|---|---|
| `continuum init` | Write the eight engineering docs into the project |
| `continuum analyze` | Analyze the repository and print an architecture report |
| `continuum execute` | Run an autonomous engineering session |
| `continuum resume` | Resume the most recent interrupted session |
| `continuum harden` | Security hardening pass (`audit` · `hardening` · `enterprise`) |
| `continuum rollback` | Roll back to a specific checkpoint |
| `continuum replay` | Replay a past session for debugging |
| `continuum doctor` | Diagnose the local environment |
| `continuum benchmark` | Run benchmarks against fixture projects |
| `continuum memory` | Inspect or compact the memory engine |
| `continuum install` | Install optional tool dependencies |
| **`continuum config`** | **Read and write provider API keys and settings** |

### `continuum config` — provider configuration

```sh
# Set an API key
continuum config set anthropic.api_key  sk-ant-...
continuum config set openai.api_key     sk-...
continuum config set gemini.api_key     AIza...
continuum config set groq.api_key       gsk_...
continuum config set deepseek.api_key   sk-...

# Override the base URL (proxies, local deployments)
continuum config set openai.base_url    https://my-proxy.example.com/v1

# Read a value (API keys are masked)
continuum config get anthropic.api_key

# Show everything that's configured
continuum config list

# List all 10 providers with status and model count
continuum config providers

# Remove a value
continuum config unset openai.base_url
```

Config is stored at `~/.continuum/config.toml`.  
Environment variables (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `GEMINI_API_KEY`, `GROQ_API_KEY`, `DEEPSEEK_API_KEY`, `MISTRAL_API_KEY`, `CONTINUUM_<PROVIDER>_API_KEY`) take precedence over the config file.

---

## Model Providers

Continuum ships with 10 providers and 40+ models, all loaded at build time from [models.dev](https://models.dev).

| Provider | ID | Env var | Models |
|---|---|---|---|
| Anthropic | `anthropic` | `ANTHROPIC_API_KEY` | Opus 4.5, Sonnet 4/4.5, Haiku 4/4.5 |
| OpenAI | `openai` | `OPENAI_API_KEY` | GPT-4o, GPT-4o Mini, o1, o3-mini, GPT-4 Turbo |
| Google | `google` | `GEMINI_API_KEY` | Gemini 2.5 Pro/Flash, 1.5 Pro/Flash |
| Groq | `groq` | `GROQ_API_KEY` | Llama 3.3 70B, Llama 3.1 8B, Mixtral, Gemma2 |
| DeepSeek | `deepseek` | `DEEPSEEK_API_KEY` | DeepSeek V3, R1 |
| Mistral AI | `mistral` | `MISTRAL_API_KEY` | Large, Small, Codestral, NeMo |
| Cohere | `cohere` | `COHERE_API_KEY` | Command R+, Command R |
| Together AI | `together` | `TOGETHER_API_KEY` | Llama 3.1 70B Turbo, Qwen 2.5, Mixtral 8x22B |
| Fireworks AI | `fireworks` | `FIREWORKS_API_KEY` | Llama 3.3 70B, DeepSeek R1, Qwen 2.5 |
| Ollama | `ollama` | *(no key)* | llama3.2, qwen2.5, codellama, mistral, phi4 |

Update models at any time:

```sh
cargo xtask refresh-models   # fetches https://models.dev/api.json
```

### Adding a new provider

Any OpenAI-compatible API can be used with `CompatProvider`:

```rust
use continuum_models::CompatProvider;
use continuum_core::caps::Cap;

let provider = CompatProvider::new(
    "my-provider",
    api_key,
    "https://api.my-provider.com/v1",
    Cap::grant(),
);
```

---

## Agent System

Eight specialized agents, each implementing the `Agent` trait:

| Agent | Role | Sandbox | Parallelizable |
|---|---|---|---|
| `PlannerAgent` | Decomposes goals into an execution DAG via LLM | No | No |
| `ArchitectureAgent` | Reviews diffs for SOLID violations and layer coupling | No | Yes |
| `CodingAgent` | Streams code from the model provider | No | No |
| `TestingAgent` | Generates idiomatic tests for produced code | Yes | No |
| `SecurityAgent` | Runs semgrep / trivy / gitleaks, produces audit reports | Yes | Yes |
| `ReviewAgent` | Code review — correctness, style, performance, security | No | Yes |
| `MemoryAgent` | Compresses hot memory items into warm summaries | No | No |
| `RecoveryAgent` | Decides retry / skip / replan when a task gets stuck | No | No |

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

---

## Validation Pipeline

Every change passes 10 stages before it is accepted:

| # | Stage | Tool | Blocking |
|---|---|---|---|
| 1 | Compile | `cargo check` | Yes |
| 2 | Lint | `cargo clippy` | Yes |
| 3 | TypeCheck | `cargo check --tests` | Yes |
| 4 | UnitTest | `cargo test --lib` | Yes |
| 5 | IntegrationTest | `cargo test --test *` | Optional |
| 6 | E2ETest | Playwright / custom | Optional |
| 7 | SecurityScan | `cargo audit` + semgrep | Yes |
| 8 | Startup | Health check probe | Optional |
| 9 | Performance | Build-time regression | Optional |
| 10 | Regression | Full test suite | Optional |

TypeScript uses Biome + Vitest; Python uses pytest.

---

## Security Hardening

```sh
continuum harden --mode audit        # passive scan, no changes
continuum harden --mode hardening    # scan + automated fixes
continuum harden --mode enterprise   # hardening + compliance attestation
```

| Mode | Capabilities | What it does |
|---|---|---|
| `audit` | Read-only | Semgrep + Trivy + Gitleaks; reports findings |
| `hardening` | Network + secrets + models | Remediates findings, applies patches |
| `enterprise` | All | Hardening + SOC2 compliance attestation + audit log |

---

## Memory & Recovery

### Three-tier memory

```
Hot  — full fidelity, recent context, SQLite
  ↓  auto-compress when token cap is reached
Warm — LLM-compressed summaries of completed sessions
  ↓  background archive
Cold — vector embeddings for semantic recall (SQLite-vec / Qdrant)
```

### Recovery

- **Checkpoint** after every plan node
- **Heartbeat monitor** detects stuck tasks
- **`continuum resume`** restarts from the last checkpoint
- **`continuum replay <session>`** replays events for debugging
- **`continuum rollback <session> --to <checkpoint>`** reverts state

---

## Repository Intelligence

Tree-sitter-based symbol extraction for four languages:

| Language | Extracts |
|---|---|
| Rust | functions, structs, enums, traits, impl blocks, modules |
| TypeScript | functions, classes, interfaces, types, enums |
| Python | functions, classes, async functions |
| Go | functions, methods, structs, interfaces |

Impact analysis computes the full transitive dependent set for any changed symbol — so agents only re-validate what actually changed.

---

## Workspace Structure

```
continuum/
├── crates/
│   ├── continuum-core/           # All traits + types (Agent, Model, Sandbox, …)
│   ├── continuum-config/         # Layered config (env → ~/.continuum/config.toml)
│   ├── continuum-telemetry/      # OpenTelemetry + Prometheus + tracing
│   ├── continuum-markdown/       # Engineering doc parser
│   ├── continuum-repo/           # Tree-sitter repo indexer + symbol graph
│   ├── continuum-planner/        # LLM planning engine + DAG construction
│   ├── continuum-models/         # 10 provider implementations
│   ├── continuum-models-registry/# Snapshot + build-time codegen (phf maps)
│   ├── continuum-sandbox/        # Docker sandbox
│   ├── continuum-storage/        # SQLite pool + migrations + vector backend
│   ├── continuum-memory/         # Hot / warm / cold memory engine
│   ├── continuum-agents/         # 8 agent implementations
│   ├── continuum-validation/     # 10-stage validation pipeline
│   ├── continuum-security/       # Hardening modes + compliance
│   ├── continuum-recovery/       # Checkpoints, replay, heartbeat
│   ├── continuum-runtime/        # DAG scheduler + session
│   ├── continuum-tools/          # Unified tool registry
│   ├── continuum-tools-linters/  # Clippy runner
│   ├── continuum-tools-security/ # Semgrep / Trivy / Gitleaks / ZAP / cargo-audit
│   ├── continuum-tools-testing/  # cargo-test / Jest / K6
│   ├── continuum-tools-browser/  # Playwright / chromiumoxide
│   └── continuum-cli/            # Binary + ratatui TUI dashboard
├── xtask/                        # Workspace automation (refresh-models, security-lint…)
├── docs/                         # VISION · PRODUCT · ARCHITECTURE · ENGINEERING
│                                 # TASKS · AGENTS · MODEL_RULES · SECURITY
├── dashboards/                   # Grafana JSON dashboards
├── plans/                        # Phase implementation plans
└── Cargo.toml                    # Workspace root (24 members)
```

---

## Development

```sh
# Build
cargo build --workspace
cargo build --release

# Test
cargo test --workspace

# Lint (must pass cleanly)
cargo clippy --workspace -- -D warnings

# Refresh model registry from models.dev
cargo xtask refresh-models

# Security lint (checks Cap::grant() usage)
cargo xtask security-lint
```

### Key design principles

- **Capability tokens** (`Cap<T>`) — dangerous operations (model calls, sandbox exec, secrets) require an explicitly minted token; enforced at compile time
- **No `unsafe` code** — `#![forbid(unsafe_code)]` in every crate
- **Trait-based providers** — `ModelProvider`, `Agent`, `SandboxHandle`, `Planner` are all async traits; swap any backend without touching callers
- **All tools run in sandbox** — zero `Command::spawn` on the host outside `continuum-sandbox`
- **Snapshot-driven registry** — model metadata is vendored at `crates/continuum-models-registry/models-snapshot.json` and codegen'd at build time via `phf` for zero-overhead lookups

---

## Configuration reference

`~/.continuum/config.toml`

```toml
[providers.anthropic]
api_key = "sk-ant-..."

[providers.openai]
api_key  = "sk-..."
base_url = "https://my-proxy.example.com/v1"   # optional

[providers.google]
api_key = "AIza..."

[providers.groq]
api_key = "gsk_..."

[providers.deepseek]
api_key = "sk-..."

[providers.ollama]
base_url = "http://localhost:11434/v1"          # default
```

Environment variables are checked before the file:

```
ANTHROPIC_API_KEY
OPENAI_API_KEY
GEMINI_API_KEY
GROQ_API_KEY
DEEPSEEK_API_KEY
MISTRAL_API_KEY
COHERE_API_KEY
TOGETHER_API_KEY
FIREWORKS_API_KEY
CONTINUUM_<PROVIDER>_API_KEY   # generic override for any provider
CONTINUUM_<PROVIDER>_BASE_URL  # generic base URL override
```

---

## Roadmap

See [`docs/TASKS.md`](docs/TASKS.md) for the phased implementation plan.

| Phase | Status | Description |
|---|---|---|
| 1 | ✅ Done | Workspace bootstrap, 24 crates, CI, engineering docs |
| 2 | ✅ Done | Models + storage substrate, AnthropicProvider, SQLite |
| 3 | ✅ Done | Repo intelligence (tree-sitter), LLM planner, DAG |
| 4 | ✅ Done | Docker sandbox, CodingAgent, Clippy runner, Scheduler |
| 5 | ✅ Done | 10-stage validation pipeline |
| 6 | ✅ Done | Memory engine (hot/warm/cold), recovery, replay |
| 7 | ✅ Done | All 8 agents, Gemini/DeepSeek/Groq providers, `continuum config` |
| 8 | 🔨 In Progress | Live TUI dashboard, OpenTelemetry polish |
| 9 | 🔨 In Progress | Security hardening modes, enterprise compliance |

---

## Contributing

1. Read [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) and [`docs/AGENTS.md`](docs/AGENTS.md)
2. Check [`docs/TASKS.md`](docs/TASKS.md) for open work
3. Open an issue before large changes
4. All code must pass `cargo clippy --workspace -- -D warnings`
5. No `unsafe` — the workspace forbids it
6. Sign your commits (`git commit -s`)

```sh
git clone https://github.com/your-org/continuum.git
cd continuum
git checkout -b feat/my-feature

cargo build --workspace
cargo test  --workspace
cargo clippy --workspace -- -D warnings

git commit -s -m "feat: describe the change"
git push origin feat/my-feature
# open a pull request
```

---

## License

Dual-licensed under [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) — your choice.

---

<p align="center">
  <strong>Continuum</strong> — autonomous engineering, at production scale.<br>
  <a href="docs/ARCHITECTURE.md">Architecture</a> ·
  <a href="docs/TASKS.md">Roadmap</a> ·
  <a href="docs/AGENTS.md">Agents</a> ·
  <a href="docs/SECURITY.md">Security</a>
</p>

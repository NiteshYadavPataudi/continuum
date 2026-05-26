# Product

## Core capabilities

1. **Analyze large repositories** — understand 10k+ file repos and monorepos.
2. **Create execution plans automatically** — produce DAGs with estimates.
3. **Build projects from specifications** — implement from engineering docs.
4. **Execute tasks autonomously for hours** — long-running agent orchestration.
5. **Validate code quality continuously** — 10-stage validation pipeline.
6. **Detect regressions automatically** — regression-aware test selection.
7. **Perform security implementation and auditing** — auth, RBAC, CSP, JWT,
   rate limiting, secret scanning, dependency patching.
8. **Recover from failures autonomously** — checkpoints, replay, rollback,
   stuck-task recovery.
9. **Maintain long-term repository memory** — hot/warm/cold tiers via SQLite
   and Qdrant.
10. **Support multi-model orchestration** — OpenAI, Anthropic, Gemini,
    DeepSeek, Ollama, local models with dynamic routing.

## Surfaces

- `continuum` CLI binary (primary)
- Future: LSP server, web UI, daemon mode

## Toolchain integrations

| Category | Tools |
|----------|-------|
| Code quality | Biome, ESLint, Ruff, Clippy |
| Security | OWASP ZAP, Semgrep, Trivy, Gitleaks |
| Testing | Vitest, Jest, Pytest, cargo test, k6 |
| Infrastructure | Docker, OpenTelemetry, Prometheus, Grafana |

## Installation system

`continuum install` performs fully automated setup:

- detect environment
- install dependencies
- configure Docker
- install validators
- configure Playwright
- setup security tooling
- initialize memory database

## Distinguishing features

- Long-running autonomous execution
- Production-grade validation
- Autonomous regression prevention
- Architecture-aware reasoning
- Security implementation
- Persistent engineering memory
- Full-system verification
- Multi-agent orchestration
- Repository intelligence
- Recovery and replay systems

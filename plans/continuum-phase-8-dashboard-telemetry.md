# Continuum — Phase 8: Live Dashboard + Telemetry Polish

## Context

By Phase 7, Continuum is functional but observability is text-only and ad hoc: progress shows up as `tracing` lines on stderr, validation findings as end-of-run tables, costs as a single number. For long-running autonomous execution — the system's flagship use case — that's not enough. Operators need a live view, and ops teams need real telemetry.

Phase 8 ships the `ratatui` live dashboard (a confirmed day-1 architectural decision), wires OpenTelemetry traces and Prometheus metrics through `continuum-telemetry`, and makes `xtask refresh-models` a scheduled CI job so the registry never drifts.

## Recommended approach

### A. Live dashboard (`continuum-cli`)

1. **Trigger.** `continuum execute` always opens the dashboard unless `--no-tui` or non-TTY stdout. `continuum execute --watch <session>` attaches to an in-flight session.
2. **Architecture.** Dashboard is a separate `tokio::task` reading from a `broadcast::Receiver<DashboardEvent>`. The runtime emits `DashboardEvent`s via a `tracing::Layer` so the dashboard observes the same stream as `replay_events` storage in `continuum-recovery` — no parallel instrumentation.
3. **Layout (4-pane).**
   - **Top-left:** Plan DAG. Nodes coloured by state (pending / running / done / failed). Selected node shows its agent, retries, USD spent.
   - **Top-right:** Validation pipeline. 10 rows, one per stage; running stages animate; failed stages expand to show findings.
   - **Bottom-left:** Agent log. Tailing `tracing` lines from the currently selected node.
   - **Bottom-right:** Token / cost meter. Live USD vs. budget, per-provider rate, retry counters.
4. **Input.** Arrow keys navigate the DAG, `Enter` expands a node, `q` quits (cancelling the run with confirmation), `p` pauses scheduling at the next safe point.
5. **Crash-safe.** Dashboard panics in its own task; the run continues. On dashboard crash, the runtime falls back to plain stderr logging.
6. **Approval prompts.** `ExecutionContract` approval is a modal overlay before the dashboard goes live; replaces the current line-based prompt for TTY runs.

### B. OpenTelemetry (`continuum-telemetry`)

1. **Init.** `init(config)` builds a layered subscriber: `tracing-subscriber::fmt` (stderr) + `tracing-opentelemetry::layer` (OTLP exporter) + the dashboard broadcast layer (Phase 8 part A) + the replay-events layer (Phase 6).
2. **OTLP endpoint** read from `CONTINUUM_OTLP_ENDPOINT` env or `[telemetry]` section in `continuum.toml`. No exporter when unset — telemetry is opt-in.
3. **Span schema.** Every span carries `session_id`, and where applicable `task_id`, `agent_id`, `provider_id`, `model_id`, `tool_id`. Field names match across crates so OTel queries are uniform.
4. **Sampling.** 100% trace sampling for now; revisit if exporter overhead is observable.

### C. Prometheus metrics

1. **Behind `prometheus` feature flag** on `continuum-telemetry` (currently disabled by default; enable in Phase 8).
2. **Scrape endpoint.** `continuum-cli` starts a small `hyper` server on a configurable port (default `9477`) that serves `/metrics`. Disabled when no `[telemetry.prometheus]` config.
3. **Metric surface.**
   - `continuum_tasks_total{state}` — counter.
   - `continuum_task_duration_seconds{agent_kind}` — histogram.
   - `continuum_model_tokens_total{provider, model, direction}` — counter.
   - `continuum_model_cost_usd{provider}` — counter.
   - `continuum_validation_findings_total{stage, severity}` — counter.
   - `continuum_sandbox_exec_seconds{tool}` — histogram.
   - `continuum_memory_items{layer}` — gauge.
   - `continuum_recovery_checkpoints_total` — counter.
4. **Grafana dashboards.** JSON dashboards in `dashboards/` (run-overview, model-cost, validation-funnel, memory-pressure). CI lints them with `grafonnet` or schema check.

### D. `xtask refresh-models` as scheduled CI

1. **Workflow.** `.github/workflows/refresh-models.yml` runs weekly (cron `0 12 * * MON`).
2. **Steps.** `cargo xtask refresh-models` → `cargo check --workspace` → if snapshot changed, open a PR titled `chore(models): weekly registry refresh` with the diff in the body.
3. **Diff review.** Human reviews the diff (typically additions / price changes / context-window updates). Merged PRs trigger no special action — next CI run picks up the new snapshot.

### E. Documentation + UX polish

- `continuum doctor` becomes the canonical "is Continuum healthy?" command. Surfaces: Docker version, sandbox image presence, model provider reachability, storage path writable, OTLP endpoint reachable (if configured).
- `continuum benchmark` becomes real: runs the Phase 5 fixture projects, records per-stage durations, compares against a stored baseline, exits nonzero on regression > 20%.

### Critical files

- `C:\Users\hp\Documents\continuum\crates\continuum-cli\src\dashboard\app.rs`
- `C:\Users\hp\Documents\continuum\crates\continuum-cli\src\dashboard\panes\{plan,validation,log,cost}.rs`
- `C:\Users\hp\Documents\continuum\crates\continuum-cli\src\dashboard\events.rs` — `DashboardEvent` types.
- `C:\Users\hp\Documents\continuum\crates\continuum-telemetry\src\subscriber.rs`
- `C:\Users\hp\Documents\continuum\crates\continuum-telemetry\src\otlp.rs`
- `C:\Users\hp\Documents\continuum\crates\continuum-telemetry\src\prometheus.rs`
- `C:\Users\hp\Documents\continuum\crates\continuum-telemetry\src\dashboard_layer.rs` — tracing `Layer` → `DashboardEvent`.
- `C:\Users\hp\Documents\continuum\dashboards\{run-overview,model-cost,validation-funnel,memory-pressure}.json`
- `C:\Users\hp\Documents\continuum\.github\workflows\refresh-models.yml`
- `C:\Users\hp\Documents\continuum\crates\continuum-cli\src\commands\doctor.rs` — flesh out.
- `C:\Users\hp\Documents\continuum\crates\continuum-cli\src\commands\benchmark.rs` — real handler.

### Risks and mitigations

- **Dashboard event volume.** Token-streaming agents can produce 100s of events/sec. Mitigation: per-pane rate limiter, drop on dashboard back-pressure. The replay layer is unaffected (separate channel, separate persistence).
- **Prometheus + Windows.** `metrics-exporter-prometheus` works on Windows but the scrape server should bind to `localhost` only by default; document the bind config explicitly.
- **OTLP cost in dev.** Exporter not enabled unless configured — no surprise network traffic.

## Verification

1. **Dashboard launches and exits cleanly.** `continuum execute --goal "noop"` against an empty fixture shows the dashboard, all panes render, `q` quits without orphaned containers.
2. **Crash isolation.** Inject a panic into the dashboard's plan pane; assert the run continues and stderr falls back to `tracing::fmt` lines.
3. **OTLP roundtrip.** Run with `CONTINUUM_OTLP_ENDPOINT=http://localhost:4318` and a Jaeger receiver; a full session shows up as a connected trace with all expected spans.
4. **Prometheus scrape.** `curl http://localhost:9477/metrics` returns the documented metric names with non-zero values mid-session.
5. **`xtask refresh-models` CI.** Manual workflow dispatch produces a PR when models.dev has new entries; produces no PR on an unchanged snapshot.
6. **Benchmark baseline.** Two consecutive `continuum benchmark` runs report within 10% on the same hardware; an artificial 30% regression (a `sleep` injected into a validator) exits nonzero.

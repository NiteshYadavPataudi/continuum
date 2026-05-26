# Engineering Standards

## Language

- Rust 2021 edition
- Pinned stable channel via `rust-toolchain.toml`
- `unsafe_code = "forbid"` on every crate

## Error handling

- Libraries: `thiserror` with narrow per-crate enums; `From`-convert into `continuum-core::ContinuumError`
- Binaries: `anyhow` or `Box<dyn std::error::Error>` at the top level
- No `unwrap()` outside tests; prefer `?` and explicit error variants

## Async

- One `tokio::runtime::Runtime` per process, constructed in `continuum-cli::main`
- I/O-bound traits use `#[async_trait]`
- CPU-bound work (tree-sitter parsing, regex output parsing) goes through `tokio::task::spawn_blocking`
- Cancellation cascades from `Session`'s root `CancellationToken`

## Logging / telemetry

- All crates use `tracing::{info, warn, debug, instrument}` macros
- Spans carry `session_id`, `task_id`, `agent_id` fields
- Never block the hot path on the OTLP exporter

## Testing

- Unit tests live alongside the code they cover
- Integration tests live in each crate's `tests/` directory
- E2E tests live in the workspace-level `tests/` directory (phase 4+)

## Documentation

- Every public item has a doc comment
- Trait contracts include cancellation, error, and concurrency notes
- Examples in doc comments compile (verified via `cargo test --doc`)

## Linting

- `cargo clippy --workspace --all-targets -- -D warnings` must pass
- `cargo fmt --check` must pass
- `cargo udeps` should report no unused dependencies

# Continuum — Phase 2: Models + Storage Substrate

## Context

Phase 1 declared every public trait but shipped no I/O. Before any agent can run, two pieces of infrastructure must work end-to-end: (1) calling a real model provider and getting tokens back, and (2) persisting state somewhere durable. Phase 2 builds the thinnest possible substrate for both — one provider (Anthropic), one storage backend (SQLite + `sqlite-vec`) — so Phase 3 onward has something real to call.

The architectural plan locks `models.dev` from day 1 and `sqlite-vec` as the default vector store. Both decisions land in this phase.

## Recommended approach

### A. `continuum-models-registry` codegen

The Phase 1 `build.rs` is a placeholder. Replace it with real codegen:

1. **Refresh the snapshot.** First task is to actually populate `crates/continuum-models-registry/models-snapshot.json` from `https://models.dev/api.json`. The `xtask refresh-models` subcommand (currently a stub) does this — implement it as part of Phase 2 so the snapshot is real, not hand-edited.
2. **`build.rs` parses the snapshot** and emits `generated.rs` containing:
   - `pub static MODELS: phf::Map<&'static str, ModelMeta>` keyed by `"provider/model-id"`.
   - `pub static PROVIDERS: phf::Map<&'static str, ProviderMeta>`.
   - Typed `ProviderId` / `ModelId` constants for the common cases (`anthropic`, `openai`, `gemini`, `deepseek`, `ollama`) so call sites use `models::CLAUDE_SONNET` instead of stringly-typed lookups.
   - `pub const SCHEMA_VERSION: &str = "<value from snapshot>"`.
3. **`ModelMeta`** carries: name, context window, tool-call support, streaming support, input/output prices in USD per million tokens.
4. **Schema validation.** `xtask refresh-models` fetches `https://models.dev/model-schema.json` alongside `api.json` and validates the snapshot against it before writing. Refuse to overwrite on validation failure.

### B. `continuum-models` provider abstraction

Implement only enough to call Anthropic. Other providers stay behind feature flags but unimplemented.

1. **Trait scaffolding.** Concrete `AnthropicProvider` implements `continuum_core::model::ModelProvider`. Uses `reqwest` with `rustls-tls`, streams via Anthropic's SSE endpoint.
2. **Streaming.** `CompletionStream` is built from `reqwest::Response::bytes_stream()` → SSE parser → `Delta` items. Use `eventsource-stream` or roll a minimal SSE parser (~80 LOC); avoid pulling a heavy dep.
3. **Auth.** API key resolution is delegated to `continuum-config` (env var `ANTHROPIC_API_KEY` by default, overridable). The provider takes a `Cap<ReadSecrets>` at construction so secret access is auditable.
4. **`ModelRouter`.** Phase 2 ships a trivial router: always pick Anthropic, pass through `MODEL_RULES.md`-derived tier mapping (`draft` → cheapest, `standard` → mid, `deep` → top). Real cost-aware routing lands in Phase 8.
5. **Cost estimation.** `estimate_cost` reads from the generated `MODELS` map using the model in the request — no live pricing calls.

### C. `continuum-storage` SQLite + sqlite-vec

1. **Connection pool.** `Storage::open(path: &Path) -> Result<Storage>` returns a struct wrapping `sqlx::SqlitePool`. WAL mode, `synchronous = NORMAL`, `foreign_keys = ON`.
2. **Migrations.** `sqlx::migrate!("./migrations")` with the first migration creating: `runs`, `tasks`, `decisions`, `checkpoints`, `memory_items`, `symbols`, plus `memory_vectors` (a `sqlite-vec` virtual table holding `MemoryId` → embedding).
3. **`sqlite-vec` loading.** The extension binary is bundled via `sqlite-vec` crate (or loaded from a Cargo-managed path). Connection pool registers it on every checkout.
4. **Typed repositories.** One module per repository: `runs`, `decisions`, `checkpoints`, `memory_items`, `vector_index`. Each exposes `async fn` methods returning `Result<T, StorageError>`. No raw SQL escapes the module boundary.
5. **`VectorIndex` impl.** `SqliteVecIndex` implements `continuum_core::memory::VectorIndex`. `upsert` does an `INSERT OR REPLACE`; `search` runs the standard `sqlite-vec` `vec_distance_cosine` query with a `LIMIT`.
6. **Qdrant impl.** Skeleton struct behind `vector-qdrant` feature flag. Returns `VectorError::Backing("not implemented in phase 2")` from every method. Real impl waits until someone needs it.

### D. Wire it together

`continuum-cli`'s `doctor` subcommand becomes the first real handler in Phase 2:

- Reads `continuum.toml` from `--project`.
- Opens `continuum-storage` against `.continuum/state.db`.
- Runs migrations.
- Constructs `AnthropicProvider`, makes a one-token completion call (`"Reply with: OK"`).
- Embeds a fixed test string and stores it in `memory_vectors`, then recalls it.
- Reports pass/fail for each substep.

This proves the substrate works end-to-end before anything depends on it.

### Critical files

- `C:\Users\hp\Documents\continuum\crates\continuum-models-registry\build.rs` — real codegen.
- `C:\Users\hp\Documents\continuum\crates\continuum-models-registry\models-snapshot.json` — refreshed via `xtask`.
- `C:\Users\hp\Documents\continuum\crates\continuum-models\src\anthropic.rs` — new.
- `C:\Users\hp\Documents\continuum\crates\continuum-models\src\router.rs` — new.
- `C:\Users\hp\Documents\continuum\crates\continuum-storage\src\lib.rs` — pool + migrations entry point.
- `C:\Users\hp\Documents\continuum\crates\continuum-storage\migrations\0001_initial.sql` — new.
- `C:\Users\hp\Documents\continuum\crates\continuum-storage\src\vector\sqlite_vec.rs` — new.
- `C:\Users\hp\Documents\continuum\crates\continuum-storage\src\repos\*.rs` — one per table.
- `C:\Users\hp\Documents\continuum\xtask\src\main.rs` — real `refresh-models` implementation.
- `C:\Users\hp\Documents\continuum\crates\continuum-cli\src\commands\doctor.rs` — first real handler.

### Risks and mitigations

- **`sqlite-vec` packaging on Windows.** The extension binary may need to be pre-built per target. Mitigation: depend on the `sqlite-vec` crate that bundles the C source; cross-compile as part of CI.
- **Anthropic SSE event shape changes.** Pin the API version header (`anthropic-version: 2023-06-01`) so the parser doesn't break under our feet.
- **Snapshot freshness.** `xtask refresh-models` runs in CI weekly; diff-only commits keep the working tree quiet.

## Verification

1. `cargo build --workspace` succeeds (clippy and fmt clean).
2. `cargo xtask refresh-models` fetches `api.json`, validates against `model-schema.json`, writes `models-snapshot.json`. The next `cargo check` rebuilds `generated.rs` and the workspace still compiles.
3. `cargo run -p continuum-cli -- doctor --project ./fixtures/empty-project` prints `OK` for each substep: migrations, Anthropic completion, embed-then-recall round-trip.
4. New integration test `continuum-storage/tests/vector_roundtrip.rs` opens an in-memory SQLite, upserts 100 random 384-d vectors, searches for one, asserts the inserted vector is rank-1.
5. `cargo build -p continuum-storage --features vector-qdrant --no-default-features` compiles — the Qdrant skeleton is real enough to link, even if methods return errors.

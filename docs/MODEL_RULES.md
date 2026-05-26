# Model Rules

The `ModelRouter` in `continuum-models` uses these rules to select a provider/model for each `ModelIntent`.

## Routing dimensions

- **task_class**: `code`, `plan`, `summarize`, `review`, `embed`
- **complexity**: `draft`, `standard`, `deep`
- **budget_usd**: optional cap on the call
- **require_streaming**: must support token streaming

## Default tiers

- **draft**: cheap, fast model used for estimation and early iteration
- **standard**: default working model for coding, planning, reviewing
- **deep**: high-capability model reserved for complex reasoning and architecture

## Fallback chain

Every routing decision produces a primary plus an ordered fallback chain. The runtime falls back when:
- Provider returns `ModelError::RateLimited`
- Provider returns `ModelError::AuthFailed` (warns once per session)
- Network error persists past three retries

## Provider availability

Pulled from the vendored `models.dev` snapshot in `continuum-models-registry`. Refreshed weekly via `cargo xtask refresh-models`.

## Configuration

Per-project overrides live in `continuum.toml` under `[models]`. Per-tier model selection can be pinned to specific provider/model IDs.

# Agents

Continuum ships eight built-in subagents. Each implements `continuum_core::agent::Agent` and is dispatched by `AgentKind`.

## planner-agent
Creates execution DAGs from goals. Consults the engineering docs and the repo analysis to produce a `TaskNode` sequence.

## architecture-agent
Validates system design. Checks proposed changes against `ARCHITECTURE.md` invariants and the repo's dependency graph.

## coding-agent
Implements code changes and refactors. Owns the model→tool-call loop for source modifications.

## testing-agent
Generates and runs tests. Feeds findings into the validation pipeline.

## security-agent
Implements and audits security controls. Owns the `--harden` modes (`audit`, `hardening`, `enterprise`).

## review-agent
Reviews changes for maintainability and optimization. Surfaces refactor suggestions without blocking forward progress.

## memory-agent
Summarizes and compresses memory tiers. Runs in the background to keep the hot tier from overflowing.

## recovery-agent
Drives replay and crash recovery. Owns checkpoint cadence and stuck-task detection.

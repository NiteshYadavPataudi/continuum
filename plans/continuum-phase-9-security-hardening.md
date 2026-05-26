# Continuum — Phase 9: Security Hardening Modes

## Context

Continuum has been "security-aware" since Phase 5 — security scanners run as optional validators. Phase 9 promotes security from a check to a *capability*: dedicated operating modes that proactively audit, harden, and (in `enterprise` mode) attest. This is the phase that delivers on the spec's "security-first architecture" pillar and the spec's three top-level modes: `--security-audit`, `--hardening`, `--enterprise`.

Phase 9 also closes the loop on the capability-token system declared in `continuum-core::caps` at bootstrap: every dangerous operation now gates on a `Cap<T>` minted by the security layer, with audit logging on mint and on use.

## Recommended approach

### A. Three hardening modes (`continuum-cli`)

`continuum harden --mode {audit|hardening|enterprise}` invokes the security agent with a different policy bundle:

1. **`audit`** — passive only. Runs full security validator suite (Semgrep, Trivy, Gitleaks, OWASP ZAP if a startup target exists), aggregates findings, produces an `AUDIT_REPORT.md` in `.continuum/reports/`. No code changes. Suitable for CI gating.
2. **`hardening`** — `audit` plus active remediation. The security-agent proposes fixes for each finding: dependency bumps for CVEs, secret rotation guidance for leaks, missing security headers, missing CSP, weak JWT configs. Fixes ride through the standard plan → agent → validation pipeline so nothing lands unverified.
3. **`enterprise`** — `hardening` plus organisational hooks:
   - SSO/SAML scaffolding (if the project exposes auth surfaces).
   - Audit-log emission (writes structured JSON to a configurable sink).
   - Compliance attestation generation: SOC2-style control mapping in `COMPLIANCE.md`.
   - Required reviewer + signed-commit policies wired into `.github/CODEOWNERS` / `.gitlab/CODEOWNERS`.

### B. Security agent (`continuum-agents`)

Already scaffolded in Phase 7. Phase 9 implements:
1. **Policy bundles** loaded from `docs/SECURITY.md` frontmatter — mode-specific lists of required checks, severity thresholds, remediation patterns.
2. **Finding → fix** model loop. For each finding above the mode's severity floor, the agent:
   - Builds a remediation prompt from the finding + the affected code (via `RepoLoader::retrieve`).
   - Generates a patch.
   - Re-runs the originating validator on the patched code (cached pipeline pass).
   - If the validator now passes, commit the fix; if not, retry up to 3 times then surface unfixed.
3. **Suggestion-only mode.** `harden --suggest` produces patches as PR comments / diff hunks but does not apply them. Bridges Phase 9 with manual review workflows.

### C. Capability-token enforcement (`continuum-core::caps` + runtime)

Phase 1 declared the `Cap<T>` machinery but no code yet *requires* it. Phase 9 audits every dangerous call site:

1. **Mint points.** A new `SecurityPolicy` type in `continuum-security` owns mint authority. Built from `continuum.toml` + the active mode. Exposes `mint::<T>(&self, reason: &str) -> Option<Cap<T>>` — returns `None` when policy forbids.
2. **Required at:**
   - `ModelProvider::complete` requires `Cap<CallModels>`.
   - `SandboxHandle::exec` with `network_egress=true` requires `Cap<NetworkEgress>`.
   - Any path writing outside `/workspace` requires `Cap<HostFsWrite>` — should be empty after the audit; flagged if anything is found.
   - Reading API keys from env requires `Cap<ReadSecrets>`.
3. **Audit log.** Every mint emits a `tracing::info` span with `capability`, `reason`, `session_id`. Telemetry pipes these into a dedicated `audit.log` sink (separate from `replay_events`).
4. **Static lint.** A `cargo xtask security-lint` job greps for `Cap::<*>::grant()` outside `continuum-security`; fails CI if found.

### D. Security-tool maturity

Lift Phase 5's stub-grade adapters:
- **Trivy:** scan the sandbox image as well as the project's lockfiles; report base-image CVEs separately.
- **Semgrep:** ship a curated rule set in `crates/continuum-tools-security/rules/` so users get sane defaults; allow overlay via `docs/SECURITY.md`.
- **Gitleaks:** scan history not just working tree; in `enterprise` mode, refuse to run if no `.gitleaks.toml` exists.
- **OWASP ZAP:** introduce an `--active-scan` flag separate from `--passive` so a hardening run can attack a running target.

### E. Hardened sandbox image

`continuum/runtime:hardened` — a third variant alongside `:slim` and `:full`:
- Non-root only.
- `--cap-drop=ALL` plus only `NET_BIND_SERVICE` if needed.
- No-new-privileges flag.
- Read-only root filesystem; `/workspace` and `/tmp` are the only writable mounts.
- SBOM produced at build time, attested via `cosign` (if available in CI).

`continuum harden --mode enterprise` swaps the runtime image to `:hardened` for the duration of the run.

### F. CLI

`continuum harden --mode <m> --project <p>` becomes the real handler. Phase 1's stub goes away. Adds:
- `--suggest` (no-apply mode).
- `--baseline <path>` to diff against a prior audit report.
- `--exit-code` (default true): exit nonzero on findings above mode's severity floor.

### Critical files

- `C:\Users\hp\Documents\continuum\crates\continuum-security\src\policy.rs` — `SecurityPolicy`, capability minting.
- `C:\Users\hp\Documents\continuum\crates\continuum-security\src\audit.rs` — audit report builder.
- `C:\Users\hp\Documents\continuum\crates\continuum-security\src\compliance.rs` — `enterprise`-mode attestation.
- `C:\Users\hp\Documents\continuum\crates\continuum-agents\src\security.rs` — fill in the Phase 7 stub.
- `C:\Users\hp\Documents\continuum\crates\continuum-tools-security\rules\semgrep\` — curated rule set.
- `C:\Users\hp\Documents\continuum\crates\continuum-tools-security\src\{trivy,semgrep,gitleaks,zap}.rs` — maturity passes.
- `C:\Users\hp\Documents\continuum\crates\continuum-sandbox\images\hardened\Dockerfile`
- `C:\Users\hp\Documents\continuum\xtask\src\security_lint.rs` — capability-grant lint.
- `C:\Users\hp\Documents\continuum\crates\continuum-cli\src\commands\harden.rs` — real handler.
- `C:\Users\hp\Documents\continuum\docs\SECURITY.md` — policy frontmatter schema, mode definitions.

### Risks and mitigations

- **False-positive flood** from Semgrep / Trivy on real repos can drown remediations. Mitigation: severity floor per mode; `--baseline` lets users grandfather existing findings; the security-agent batches similar findings into one fix where possible.
- **OWASP ZAP active scan against production targets.** Catastrophic if misconfigured. Mitigation: ZAP active scans refuse non-loopback URLs unless a `--allow-target <url>` flag is passed with explicit acknowledgement.
- **Capability migration ripple.** Adding `Cap<T>` requirements to existing call sites is a workspace-wide diff. Mitigation: land one capability per PR, in dependency order (`CallModels` first, `NetworkEgress` last).
- **Compliance attestation accuracy.** SOC2 control mapping is suggestive, not certified. Document the limitation prominently in `COMPLIANCE.md`.

## Verification

1. **Audit reproducibility.** `continuum harden --mode audit` twice on the same repo produces byte-identical `AUDIT_REPORT.md` (modulo timestamp).
2. **Hardening loop.** Fixture with a planted CVE-vulnerable dependency: `harden --mode hardening` produces a bumped `Cargo.lock` / `package-lock.json`, re-runs Trivy clean, commits the fix.
3. **Enterprise attestation.** `harden --mode enterprise` produces a `COMPLIANCE.md` listing each SOC2 control with `met` / `not-met` / `n/a` and a citation to a code path or config.
4. **Capability enforcement.** `cargo xtask security-lint` exits zero on a clean tree; a planted `Cap::<NetworkEgress>::grant()` in `continuum-agents` fails the lint.
5. **Audit log isolation.** A `harden` session writes to `audit.log` but not to `replay_events`; reverse is true for normal `execute` sessions.
6. **Hardened image baseline.** `continuum harden --mode enterprise` uses `:hardened`; `cargo run -- doctor` confirms image variant and lists the dropped capabilities.
7. **CI gating.** `continuum harden --mode audit --exit-code` exits nonzero when fixture has a HIGH-severity finding, zero when none.

---

## Closing the roadmap

Phase 9 is the last numbered phase in the architectural plan. Post-Phase 9 work (per the spec's "Future Roadmap"): distributed workers, cloud execution, autonomous maintenance + CI/CD, multi-user collaboration. Each will warrant its own plan file when prioritised.

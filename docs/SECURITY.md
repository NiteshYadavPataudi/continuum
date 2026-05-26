# Security Policy

## Security implementation modes

Continuum exposes three top-level security modes via `continuum harden --mode`:

- **`audit`** — passive scan only. Runs Semgrep, Trivy, Gitleaks. Reports findings; makes no changes.
- **`hardening`** — applies fixes: dependency updates, secret rotation guidance, header injection, RBAC scaffolding.
- **`enterprise`** — `hardening` plus organisational policies: SSO/SAML scaffolding, audit logging, compliance attestation hooks.

## Default protections

The security layer enforces (when in scope):
- Auth hardening
- RBAC validation
- CSP headers
- JWT validation
- Rate limiting
- Secret scanning
- Docker hardening
- Dependency patching
- API protection
- XSS prevention
- SQL-injection prevention

## Capability tokens

`continuum-core::caps` defines compile-time gates:
- `Cap<HostExec>` — spawn processes outside the sandbox
- `Cap<NetworkEgress>` — open outbound connections
- `Cap<HostFsWrite>` — write outside the project directory
- `Cap<ReadSecrets>` — read API keys and sensitive env vars
- `Cap<CallModels>` — invoke the model provider layer

Capabilities are minted only by the runtime's policy layer after verifying configuration.

## Sandbox invariant

External tool processes spawn **only** through `SandboxHandle::exec`. No host-side `Command::spawn` exists in any tool crate. Violations are caught in code review and (eventually) by a static lint.

## Reporting

Security issues should be reported privately. (TODO: dedicated reporting channel before the public alpha.)

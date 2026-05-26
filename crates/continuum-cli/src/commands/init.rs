//! `continuum init` — scaffold engineering docs in a new project.

use super::{CmdResult, InitArgs};
use std::{fs, io, path::Path};

const DOCS: &[(&str, &str)] = &[
    (
        "VISION.md",
        "# Vision\n\n## Mission\n\nDefine the production engineering outcome this project exists to achieve.\n\n## Philosophy\n\n- Correctness over speed\n- Verification before completion\n- Production-ready by default\n- Security-first architecture\n- Autonomous execution with human oversight\n",
    ),
    (
        "PRODUCT.md",
        "# Product\n\n## Requirements\n\nDescribe the user-facing capabilities, constraints, and acceptance criteria.\n\n## Non-goals\n\nList explicit boundaries so autonomous execution does not expand scope silently.\n",
    ),
    (
        "ARCHITECTURE.md",
        "# Architecture\n\n## System Design\n\nDescribe services, crates, modules, data flows, and deployment boundaries.\n\n## Invariants\n\n- Document dependency direction\n- Document persistence ownership\n- Document security boundaries\n",
    ),
    (
        "ENGINEERING.md",
        "# Engineering Standards\n\n## Validation\n\n- Build must pass\n- Linting must pass\n- Tests must pass\n- Security checks must pass when in scope\n\n## Style\n\nDocument language, formatting, error handling, and testing rules.\n",
    ),
    (
        "TASKS.md",
        "# Tasks\n\n## Current Roadmap\n\n- [ ] Define the first production milestone\n- [ ] Add validation commands\n- [ ] Add security checks\n\n## Completed\n\nRecord completed work with validation evidence.\n",
    ),
    (
        "AGENTS.md",
        "# Agents\n\n## planner-agent\nCreates execution plans.\n\n## architecture-agent\nValidates design constraints.\n\n## coding-agent\nImplements changes.\n\n## testing-agent\nRuns validation.\n\n## security-agent\nAudits and hardens security controls.\n\n## review-agent\nReviews maintainability and regressions.\n\n## memory-agent\nMaintains project memory.\n\n## recovery-agent\nHandles checkpoints and replay.\n",
    ),
    (
        "MODEL_RULES.md",
        "# Model Rules\n\n## Routing\n\nDocument model selection rules by task class, complexity, budget, and fallback requirements.\n\n## Constraints\n\n- Prefer cost-aware routing\n- Require fallbacks for long-running work\n- Use local models only when explicitly configured\n",
    ),
    (
        "SECURITY.md",
        "# Security Policy\n\n## Required Checks\n\n- Secret scanning\n- Dependency scanning\n- Auth and authorization review\n- Input validation review\n- Container hardening when Docker is in scope\n\n## Reporting\n\nDocument private reporting and triage expectations before public release.\n",
    ),
];

pub async fn run(args: InitArgs) -> CmdResult {
    let root = std::env::current_dir()?;
    let mut written = 0usize;
    let mut skipped = 0usize;

    for (name, contents) in DOCS {
        let path = root.join(name);
        match write_doc(&path, contents, args.force) {
            Ok(true) => written += 1,
            Ok(false) => skipped += 1,
            Err(err) => return Err(Box::new(err)),
        }
    }

    println!("continuum init: wrote {written} engineering docs, skipped {skipped} existing docs.");

    if skipped > 0 && !args.force {
        println!("Use --force to overwrite existing docs.");
    }

    Ok(())
}

fn write_doc(path: &Path, contents: &str, force: bool) -> io::Result<bool> {
    if path.exists() && !force {
        return Ok(false);
    }

    fs::write(path, contents)?;
    Ok(true)
}

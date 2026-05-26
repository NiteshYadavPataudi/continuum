//! `continuum install` — install required dependencies and runtimes.
//!
//! Detects the environment and guides installation of missing tooling.

use super::{CmdResult, InstallArgs};
use std::process::Command;

struct InstallStep {
    name: &'static str,
    check_cmd: &'static [&'static str],
    install_hint: &'static str,
    skipable: bool,
}

const STEPS: &[InstallStep] = &[
    InstallStep {
        name: "Rust toolchain",
        check_cmd: &["rustc", "--version"],
        install_hint: "https://rustup.rs",
        skipable: false,
    },
    InstallStep {
        name: "git",
        check_cmd: &["git", "--version"],
        install_hint: "https://git-scm.com",
        skipable: false,
    },
    InstallStep {
        name: "Node.js",
        check_cmd: &["node", "--version"],
        install_hint: "https://nodejs.org",
        skipable: true,
    },
    InstallStep {
        name: "npm",
        check_cmd: &["npm", "--version"],
        install_hint: "https://nodejs.org",
        skipable: true,
    },
    InstallStep {
        name: "Python 3",
        check_cmd: &["python3", "--version"],
        install_hint: "https://python.org",
        skipable: true,
    },
    InstallStep {
        name: "Docker",
        check_cmd: &["docker", "--version"],
        install_hint: "https://docker.com",
        skipable: true,
    },
];

fn is_available(cmd: &[&str]) -> bool {
    Command::new(cmd[0])
        .args(&cmd[1..])
        .output()
        .ok()
        .is_some_and(|o| o.status.success())
}

pub async fn run(args: InstallArgs) -> CmdResult {
    println!("Continuum install — environment setup\n");

    let mut missing = Vec::new();

    for step in STEPS {
        let ok = is_available(step.check_cmd);
        if ok {
            println!("  ✓ {:<20} found", step.name);
        } else if step.skipable {
            println!("  - {:<20} not found (optional)", step.name);
            println!("      install: {}", step.install_hint);
            missing.push(step.name);
        } else {
            println!("  ✗ {:<20} MISSING (required)", step.name);
            println!("      install: {}", step.install_hint);
            missing.push(step.name);
        }
    }

    if args.skip_docker {
        println!("  - {:<20} skipped (--skip-docker)", "Docker");
    }

    // Validate Rust components
    println!();

    let needed_components = [
        ("rustfmt", "rustup component add rustfmt"),
        ("clippy", "rustup component add clippy"),
    ];

    for (comp, hint) in &needed_components {
        let found = Command::new("rustup")
            .args(["component", "list", "--installed"])
            .output()
            .ok()
            .is_some_and(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .any(|l| l.contains(comp))
            });
        if found {
            println!("  ✓ {:<20} available", format!("rustup {comp}"));
        } else {
            println!("  - {:<20} not installed", format!("rustup {comp}"));
            println!("      run: {hint}");
            missing.push(comp);
        }
    }

    println!();

    if missing.is_empty() {
        println!("  ✓ All dependencies satisfied.");
    } else {
        println!(
            "  ⚠  {} items need attention (see above for install instructions).",
            missing.len()
        );
    }

    println!("\n  Run `continuum doctor` to re-check after installing.");

    Ok(())
}

//! Subcommand handlers.

use clap::{Args, Subcommand};
use std::path::PathBuf;

pub mod analyze;
pub mod benchmark;
pub mod config;
pub mod doctor;
pub mod execute;
pub mod harden;
pub mod init;
pub mod install;
pub mod memory;
pub mod replay;
pub mod resume;
pub mod rollback;

type CmdResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Debug, Args)]
pub struct InitArgs {
    /// Force overwriting existing engineering docs.
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct AnalyzeArgs {
    /// Path to the repository to analyze. Defaults to the project directory.
    #[arg(long)]
    pub path: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct ExecuteArgs {
    /// Goal prompt (natural language).
    pub goal: Option<String>,
    /// USD cap for this run.
    #[arg(long)]
    pub budget_usd: Option<f64>,
    /// Auto-approve the execution contract (skip the prompt).
    #[arg(long)]
    pub yes: bool,
    /// Dry-run: build the plan, show the contract, but do not execute.
    #[arg(long)]
    pub dry_run: bool,
    /// Disable the live TUI dashboard (use plain terminal output).
    #[arg(long)]
    pub no_tui: bool,
}

#[derive(Debug, Args)]
pub struct ResumeArgs {
    /// Session ID to resume. If omitted, resumes the most recent session.
    #[arg(long)]
    pub session: Option<String>,
}

#[derive(Debug, Args)]
pub struct HardenArgs {
    /// Hardening mode: `audit`, `hardening`, or `enterprise`.
    #[arg(long, default_value = "audit")]
    pub mode: String,
    /// Path to the project directory. Defaults to the current working directory.
    #[arg(long)]
    pub project: Option<std::path::PathBuf>,
}

#[derive(Debug, Args)]
pub struct ReplayArgs {
    /// Session ID to replay.
    pub session: String,
    /// Checkpoint ID to replay from. Defaults to the first checkpoint.
    #[arg(long)]
    pub from: Option<String>,
}

#[derive(Debug, Args)]
pub struct RollbackArgs {
    /// Session ID to roll back.
    pub session: String,
    /// Checkpoint ID to roll back to. Defaults to the previous checkpoint.
    #[arg(long)]
    pub to: Option<String>,
}

#[derive(Debug, Args)]
pub struct DoctorArgs {
    /// Try to fix detected issues automatically.
    #[arg(long)]
    pub fix: bool,
    /// Check for stuck sessions.
    #[arg(long)]
    pub check_stuck: bool,
    /// Path to the project directory. Defaults to the current working directory.
    #[arg(long)]
    pub project: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct BenchmarkArgs {
    /// Benchmark category: `validation`, `planner`, `memory`, `all`.
    #[arg(long, default_value = "all")]
    pub category: String,
}

#[derive(Debug, Args)]
pub struct MemoryArgs {
    /// Subverb: `list`, `compress`, `purge`.
    pub action: String,
}

#[derive(Debug, Args)]
pub struct InstallArgs {
    /// Skip Docker setup.
    #[arg(long)]
    pub skip_docker: bool,
}

/// `continuum config` — read and write provider API keys and settings.
#[derive(Debug, Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ConfigSubcommand {
    /// Set a config value: `set <provider>.<field> <value>`
    ///
    /// Examples:
    ///   continuum config set anthropic.api_key sk-ant-...
    ///   continuum config set openai.base_url https://my-proxy/v1
    Set {
        /// Config key in `<provider>.<field>` format.
        key: String,
        /// Value to set.
        value: String,
    },
    /// Get a config value: `get <provider>.<field>`
    Get {
        /// Config key in `<provider>.<field>` format.
        key: String,
    },
    /// Remove a config value: `unset <provider>.<field>`
    Unset {
        /// Config key in `<provider>.<field>` format.
        key: String,
    },
    /// Show all configured providers and their settings.
    List,
    /// List all providers available in the model registry.
    Providers,
}

#[allow(dead_code)]
pub(crate) fn not_yet_implemented(name: &str) -> CmdResult {
    println!(
        "continuum {name}: not yet implemented in this build. See docs/TASKS.md for the roadmap."
    );
    Ok(())
}

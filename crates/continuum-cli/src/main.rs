//! `continuum` — the CLI entry point.
//!
//! Supports three modes:
//! 1. `continuum` (no args) — interactive REPL
//! 2. `continuum "goal"` — execute a goal directly
//! 3. `continuum <subcommand>` — traditional subcommand

#![allow(clippy::useless_conversion, dead_code)]

use clap::{Parser, Subcommand};

mod commands;
mod custom_commands;
mod dashboard;
mod output;
mod redact;
mod repl;
mod tui;

#[derive(Debug, Parser)]
#[command(
    name = "continuum",
    version,
    about = "Autonomous production engineering runtime.",
    args_conflicts_with_subcommands = true
)]
struct Cli {
    /// Path to the project directory. Defaults to the current working directory.
    #[arg(long, global = true)]
    project: Option<std::path::PathBuf>,

    /// Verbose logging.
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Print mode: output result and exit (non-interactive).
    #[arg(short, long)]
    print: bool,

    /// Launch full-screen TUI mode.
    #[arg(long)]
    tui: bool,

    /// Continue the most recent session.
    #[arg(short, long)]
    r#continue: bool,

    /// Resume a specific session by ID.
    #[arg(short, long)]
    resume: Option<String>,

    /// Override the model for this session.
    #[arg(long)]
    model: Option<String>,

    /// Goal to execute (natural language). When provided, executes directly.
    goal: Option<String>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand, Clone)]
enum Command {
    /// Scaffold a new Continuum-managed project (writes the eight engineering docs).
    Init(commands::InitArgs),
    /// Analyze a repository and produce an architecture report.
    Analyze(commands::AnalyzeArgs),
    /// Run an autonomous execution session.
    Execute(commands::ExecuteArgs),
    /// Resume a previously interrupted session from the latest checkpoint.
    Resume(commands::ResumeArgs),
    /// Run a security hardening pass.
    Harden(commands::HardenArgs),
    /// Replay a past session for inspection.
    Replay(commands::ReplayArgs),
    /// Roll back a session to a checkpoint.
    Rollback(commands::RollbackArgs),
    /// Diagnose the local environment.
    Doctor(commands::DoctorArgs),
    /// Run validation benchmarks.
    Benchmark(commands::BenchmarkArgs),
    /// Inspect or compact the memory engine.
    Memory(commands::MemoryArgs),
    /// Install all required dependencies and runtimes.
    Install(commands::InstallArgs),
    /// Read and write provider API keys and configuration.
    Config(commands::ConfigArgs),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    continuum_telemetry::init();

    let cli = Cli::parse();

    // TUI mode
    if cli.tui {
        let session = repl::ReplSession::new();
        return tui::run_tui(session);
    }

    // Check for --continue or --resume flags
    if cli.r#continue {
        println!("  Resuming last session...");
        return repl::run_repl(Some("continue working on the last task".to_string())).await;
    }

    if let Some(session_id) = &cli.resume {
        println!("  Resuming session: {session_id}");
        return repl::run_repl(Some(format!("continue session {session_id}"))).await;
    }

    // Handle subcommand vs goal vs REPL
    match (cli.command, cli.goal) {
        // No subcommand, no goal — start interactive REPL
        (None, None) => repl::run_repl(None).await,

        // No subcommand, goal provided — execute goal
        (None, Some(goal_text)) => {
            if cli.print {
                // Print mode: execute and output result as JSON
                let (safe_goal, redactions) = redact::redact(&goal_text);
                let result = serde_json::json!({
                    "goal": safe_goal,
                    "model": "default",
                    "redactions": redactions,
                    "status": "queued",
                    "message": "Full execution pipeline will be wired with the execution engine"
                });
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
                Ok(())
            } else {
                // Start REPL with initial prompt
                repl::run_repl(Some(goal_text)).await
            }
        }

        // Subcommand provided — run it
        (Some(cmd), _) => match cmd {
            Command::Init(args) => commands::init::run(args).await,
            Command::Analyze(args) => commands::analyze::run(args).await,
            Command::Execute(args) => commands::execute::run(args).await,
            Command::Resume(args) => commands::resume::run(args).await,
            Command::Harden(args) => commands::harden::run(args).await,
            Command::Replay(args) => commands::replay::run(args).await,
            Command::Rollback(args) => commands::rollback::run(args).await,
            Command::Doctor(args) => commands::doctor::run(args).await,
            Command::Benchmark(args) => commands::benchmark::run(args).await,
            Command::Memory(args) => commands::memory::run(args).await,
            Command::Install(args) => commands::install::run(args).await,
            Command::Config(args) => commands::config::run(args).await,
        },
    }
}

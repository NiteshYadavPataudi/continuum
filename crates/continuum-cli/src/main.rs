//! `continuum` — the CLI entry point.
//!
//! Phase 1 ships subcommand scaffolding and a no-op handler for each verb.
//! Real wiring lands incrementally as downstream phases add capability.

use clap::{Parser, Subcommand};

mod commands;
mod dashboard;

#[derive(Debug, Parser)]
#[command(
    name = "continuum",
    version,
    about = "Autonomous production engineering runtime."
)]
struct Cli {
    /// Path to the project directory. Defaults to the current working directory.
    #[arg(long, global = true)]
    project: Option<std::path::PathBuf>,

    /// Verbose logging.
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
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
    match cli.command {
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
    }
}

use std::sync::Arc;

use continuum_core::planner::Planner;
use continuum_core::repo::{IndexOptions, RepoLoader};

use super::{AnalyzeArgs, CmdResult};

pub async fn run(args: AnalyzeArgs) -> CmdResult {
    let root = args
        .path
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    println!("Analyzing repository at: {}", root.display());

    let docs = continuum_markdown::load(&root)
        .map_err(|e| format!("failed to load engineering docs: {e}"))?;

    let loader = continuum_repo::Loader::new(root.clone());
    let index: Arc<dyn continuum_core::repo::RepoIndex> = loader
        .build(
            &root,
            IndexOptions {
                respect_gitignore: true,
                max_files: 10000,
            },
        )
        .await
        .map_err(|e| format!("failed to build repo index: {e}"))?;

    let engine = continuum_planner::PlanningEngine::default();
    let analysis = engine
        .analyze(index, &docs)
        .await
        .map_err(|e| format!("analysis failed: {e}"))?;

    println!("\n── Repo Analysis ──");
    println!("{}", analysis.summary);
    if !analysis.services.is_empty() {
        println!("\nEntry points:");
        for svc in &analysis.services {
            println!("  • {svc}");
        }
    }
    Ok(())
}

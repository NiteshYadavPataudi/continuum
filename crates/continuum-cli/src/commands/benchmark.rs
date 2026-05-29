use std::sync::Arc;
use std::time::Instant;

use continuum_core::planner::{Goal, Planner};
use continuum_core::repo::{IndexOptions, RepoLoader};

use super::{BenchmarkArgs, CmdResult};

pub async fn run(args: BenchmarkArgs) -> CmdResult {
    let root = std::env::current_dir().unwrap_or_default();

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

    let docs = continuum_markdown::load(&root)
        .map_err(|e| format!("failed to load engineering docs: {e}"))?;

    match args.category.as_str() {
        "planner" | "all" => bench_planner(index.clone(), &docs).await?,
        _ => {}
    }

    if args.category == "validation" || args.category == "all" {
        bench_validation(&root).await?;
    }
    if args.category == "memory" || args.category == "all" {
        bench_memory().await?;
    }

    Ok(())
}

async fn bench_planner(
    index: Arc<dyn continuum_core::repo::RepoIndex>,
    docs: &continuum_core::planner::EngineeringDocs,
) -> CmdResult {
    println!("\n── Planner benchmarks ──\n");

    let engine = continuum_planner::PlanningEngine::default();
    let goals = [
        "implement the next feature",
        "add authentication",
        "refactor the core module",
    ];

    for goal_text in &goals {
        let goal = Goal::new(*goal_text);

        let start = Instant::now();
        let analysis = engine
            .analyze(index.clone(), docs)
            .await
            .map_err(|e| format!("analysis failed: {e}"))?;
        let analysis_ms = start.elapsed().as_secs_f64() * 1000.0;

        let start = Instant::now();
        let plan = engine
            .plan(goal, &analysis)
            .await
            .map_err(|e| format!("planning failed: {e}"))?;
        let plan_ms = start.elapsed().as_secs_f64() * 1000.0;

        let start = Instant::now();
        let estimate = engine
            .estimate(&plan)
            .await
            .map_err(|e| format!("estimation failed: {e}"))?;
        let estimate_ms = start.elapsed().as_secs_f64() * 1000.0;

        println!("  goal: {goal_text}");
        println!(
            "    analyze  {:8.1} ms  {} services",
            analysis_ms,
            analysis.services.len()
        );
        println!(
            "    plan     {:8.1} ms  {} nodes",
            plan_ms,
            plan.nodes.len()
        );
        println!(
            "    estimate {:8.1} ms  {:.2}s / ${:.4}",
            estimate_ms, estimate.runtime_secs, estimate.usd
        );
        println!();
    }

    Ok(())
}

async fn bench_validation(root: &std::path::Path) -> CmdResult {
    println!("\n── Validation benchmarks (fixture-based) ──\n");
    let fixture = root.join("fixtures").join("rust-fixture");
    if !fixture.exists() {
        println!("  rust-fixture not found at {}", fixture.display());
        return Ok(());
    }

    let start = Instant::now();
    use std::process::Command;
    let output = Command::new("cargo")
        .args([
            "check",
            "--manifest-path",
            &fixture.join("Cargo.toml").display().to_string(),
        ])
        .output()
        .map_err(|e| format!("cargo check failed: {e}"))?;
    let check_ms = start.elapsed().as_millis();
    println!(
        "  cargo check  {:8} ms  {}",
        check_ms,
        if output.status.success() {
            "OK"
        } else {
            "FAIL"
        }
    );

    let start = Instant::now();
    let output = Command::new("cargo")
        .args([
            "test",
            "--manifest-path",
            &fixture.join("Cargo.toml").display().to_string(),
            "--",
            "--quiet",
        ])
        .output()
        .map_err(|e| format!("cargo test failed: {e}"))?;
    let test_ms = start.elapsed().as_millis();
    println!(
        "  cargo test  {:8} ms  {}",
        test_ms,
        if output.status.success() {
            "OK"
        } else {
            "FAIL"
        }
    );

    println!();
    Ok(())
}

async fn bench_memory() -> CmdResult {
    println!("\n── Vector index benchmarks ──\n");
    let store = continuum_storage::MemoryIndex::new();
    let start = Instant::now();
    for i in 0..1000 {
        let id = continuum_core::ids::MemoryId::new();
        let vec = vec![i as f32 / 1000.0; 128];
        store
            .upsert(id, vec)
            .await
            .map_err(|e| format!("upsert failed: {e}"))?;
    }
    let insert_ms = start.elapsed().as_micros() as f64 / 1000.0;
    println!(
        "  upsert 1000 vectors  {:8.1} ms  {:.1} µs/item",
        insert_ms,
        insert_ms * 1000.0 / 1000.0
    );

    let start = Instant::now();
    let query = vec![0.5f32; 128];
    let _results = store
        .search(&query, 10)
        .await
        .map_err(|e| format!("search failed: {e}"))?;
    let search_ms = start.elapsed().as_micros() as f64 / 1000.0;
    println!("  search 10/1000      {:8.1} ms", search_ms);
    println!();
    Ok(())
}

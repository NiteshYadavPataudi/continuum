use std::sync::Arc;

use tokio::sync::broadcast;

use continuum_core::planner::{Goal, Planner};
use continuum_core::repo::{IndexOptions, RepoLoader};
use continuum_core::CancellationToken;
use continuum_telemetry::DashboardEvent;

use super::{CmdResult, ExecuteArgs};

pub async fn run(args: ExecuteArgs) -> CmdResult {
    let root = std::env::current_dir().unwrap_or_default();

    let goal_text = args
        .goal
        .clone()
        .unwrap_or_else(|| "implement the next feature".to_string());

    let goal = Goal::new(&goal_text);

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

    // Set up the dashboard broadcast channel (used even without TUI for event capture)
    let (dash_tx, dash_rx) = broadcast::channel(512);

    // Spawn the TUI if not disabled
    let no_tui = args.no_tui;
    let dashboard_task = if !no_tui {
        let rx = dash_rx;
        Some(tokio::spawn(async move {
            let mut app = crate::dashboard::DashboardApp::new(rx);
            if let Err(e) = app.run().await {
                eprintln!("dashboard error: {e}");
            }
        }))
    } else {
        None
    };

    // Helper to emit dashboard events (no-op if TUI is disabled, but fills the channel)
    let emit = |event: DashboardEvent| {
        let _ = dash_tx.send(event);
    };

    // ── Planning phase ──────────────────────────────────────────────
    let engine = continuum_planner::PlanningEngine::default();
    let analysis = engine
        .analyze(index, &docs)
        .await
        .map_err(|e| format!("analysis failed: {e}"))?;

    let plan = engine
        .plan(goal, &analysis)
        .await
        .map_err(|e| format!("planning failed: {e}"))?;
    let estimate = engine
        .estimate(&plan)
        .await
        .map_err(|e| format!("estimation failed: {e}"))?;
    let contract = engine
        .contract(&plan)
        .await
        .map_err(|e| format!("contract failed: {e}"))?;

    if args.dry_run {
        println!("\n── Execution Contract (dry run) ──");
        println!(
            "Goal: {}",
            contract
                .plan
                .nodes
                .first()
                .map_or("(empty plan)", |n| &n.label)
        );
        println!("Nodes: {}", contract.plan.nodes.len());
        println!(
            "Estimated: {} in tokens / {} out tokens / {:.2}s / ${:.4}",
            estimate.input_tokens, estimate.output_tokens, estimate.runtime_secs, estimate.usd
        );
        println!("Risk: {:.1}%", estimate.risk * 100.0);
        println!("\nSummary:");
        for line in &contract.summary {
            println!("  • {line}");
        }

        emit(DashboardEvent::Log {
            level: "INFO".into(),
            target: "executor".into(),
            message: "dry-run complete".into(),
        });
        emit(DashboardEvent::Shutdown);

        // Brief pause so the TUI can show the final state
        if dashboard_task.is_some() {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
        return Ok(());
    }

    if no_tui {
        println!("\n── Execution ──");
        println!("Analysis: {}", analysis.summary);
        println!("Plan: {} node(s)", plan.nodes.len());
    }

    // ── Execution phase ─────────────────────────────────────────────
    emit(DashboardEvent::Log {
        level: "INFO".into(),
        target: "executor".into(),
        message: format!("starting execution of {} node(s)", plan.nodes.len()),
    });

    let cancel = CancellationToken::new();
    let scheduler = continuum_runtime::Scheduler::new();
    let outcomes = scheduler.run(&plan, cancel).await?;

    // Emit task results as dashboard events
    for outcome in &outcomes {
        let status = outcome
            .artifacts
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let agent = outcome
            .artifacts
            .get("agent")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        emit(DashboardEvent::TaskCompleted {
            agent: agent.to_string(),
            status: status.to_string(),
        });
    }

    // Cost update
    emit(DashboardEvent::CostUpdate {
        usd: estimate.usd,
        tokens: estimate.input_tokens + estimate.output_tokens,
    });

    emit(DashboardEvent::Shutdown);

    if no_tui {
        println!(
            "\nExecution complete: {} tasks dispatched.",
            outcomes.len()
        );
        for outcome in &outcomes {
            let status = outcome
                .artifacts
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let agent = outcome
                .artifacts
                .get("agent")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            println!("  • [{agent}] {status}");
        }

        if !args.yes {
            println!("\nTip: use --yes to auto-approve future executions.");
        }
    } else {
        // Wait for user to press q (dashboard handles this in its own loop)
        if let Some(task) = dashboard_task {
            let _ = task.await;
        }
    }

    Ok(())
}

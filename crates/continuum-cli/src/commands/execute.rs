use std::sync::Arc;

use tokio::sync::broadcast;

use continuum_core::planner::{Goal, Planner};
use continuum_core::repo::{IndexOptions, RepoLoader};
use continuum_core::CancellationToken;
use continuum_telemetry::DashboardEvent;

use super::{CmdResult, ExecuteArgs};

pub async fn run(args: ExecuteArgs, model_override: Option<String>) -> CmdResult {
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
    let (dash_tx, _dash_rx) = broadcast::channel(512);
    let follow_tui = args.follow_tui;

    // Spawn the dashboard if not using the full-screen follow mode.
    let no_tui = args.no_tui;
    let dashboard_task = if follow_tui {
        None
    } else if !no_tui {
        let rx = dash_tx.subscribe();
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

    // Load config and resolve provider/model
    let config = continuum_config::Config::load();
    let (provider_name, model_name) = crate::repl::detect_default_provider(&config);
    let model_to_use = model_override.unwrap_or(model_name);

    let (provider_id, model_id_str) = if model_to_use.contains('/') {
        let parts: Vec<&str> = model_to_use.splitn(2, '/').collect();
        (parts[0].to_string(), parts[1].to_string())
    } else {
        (provider_name, model_to_use)
    };

    let model_provider = continuum_models::load_from_config(&config, &provider_id);
    if model_provider.is_none() {
        emit(DashboardEvent::Log {
            level: "WARN".into(),
            target: "executor".into(),
            message: format!(
                "No configured API key found for provider '{}'. Running with fallback/stubs.",
                provider_id
            ),
        });
    }

    // ── Planning phase ──────────────────────────────────────────────
    let engine = continuum_planner::PlanningEngine::new(
        model_provider.clone(),
        continuum_core::ids::ModelId::new(&model_id_str),
    );

    let p_analysis = crate::output::Progress::new("Analyzing repository");
    let analysis = engine
        .analyze(index, &docs)
        .await
        .map_err(|e| format!("analysis failed: {e}"))?;
    p_analysis.done(&analysis.summary);

    let p_plan = crate::output::Progress::new("Planning execution strategy");
    let plan = engine
        .plan(goal, &analysis)
        .await
        .map_err(|e| format!("planning failed: {e}"))?;
    p_plan.done(&format!("{} nodes", plan.nodes.len()));

    let p_est = crate::output::Progress::new("Estimating cost and time");
    let estimate = engine
        .estimate(&plan)
        .await
        .map_err(|e| format!("estimation failed: {e}"))?;
    p_est.done(&format!(
        "${:.4}, {:.1}s",
        estimate.usd, estimate.runtime_secs
    ));

    let p_contract = crate::output::Progress::new("Building execution contract");
    let contract = engine
        .contract(&plan)
        .await
        .map_err(|e| format!("contract failed: {e}"))?;
    p_contract.done(&format!("{} tasks", contract.summary.len()));

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
    let scheduler = continuum_runtime::Scheduler::with_models(model_provider, None);
    let runtime_session = continuum_runtime::Session::new().with_workspace_root(root.clone());

    if follow_tui {
        let exec_tx = dash_tx.clone();
        let exec_plan = plan.clone();
        let exec_scheduler = scheduler;
        let exec_session = runtime_session;
        let exec_handle = tokio::spawn(async move {
            exec_scheduler
                .run_with_session_events(&exec_plan, &exec_session, cancel, Some(exec_tx))
                .await
        });

        let tui_session = crate::repl::ReplSession::new();
        let follow_rx = dash_tx.subscribe();
        let tui_handle = tokio::task::spawn_blocking(move || {
            crate::tui::run_tui_with_events(tui_session, Some(follow_rx)).map_err(|e| e.to_string())
        });

        let outcomes = exec_handle
            .await
            .map_err(|e| format!("execution task failed: {e}"))??;

        emit(DashboardEvent::CostUpdate {
            usd: estimate.usd,
            tokens: estimate.input_tokens + estimate.output_tokens,
        });
        emit(DashboardEvent::Shutdown);

        tui_handle.await.map_err(|e| format!("tui failed: {e}"))??;

        println!("\nExecution complete: {} tasks dispatched.", outcomes.len());
        for outcome in &outcomes {
            let status = outcome
                .artifacts
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("done");
            let agent = outcome
                .artifacts
                .get("agent")
                .and_then(|v| v.as_str())
                .unwrap_or("agent");
            println!("  • [{agent}] {status}");
        }

        return Ok(());
    }

    let outcomes = scheduler
        .run_with_session_events(&plan, &runtime_session, cancel, Some(dash_tx.clone()))
        .await?;

    // Cost update
    emit(DashboardEvent::CostUpdate {
        usd: estimate.usd,
        tokens: estimate.input_tokens + estimate.output_tokens,
    });

    emit(DashboardEvent::Shutdown);

    if no_tui {
        println!("\nExecution complete: {} tasks dispatched.", outcomes.len());
        for outcome in &outcomes {
            let status = outcome
                .artifacts
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("done");
            let agent = outcome
                .artifacts
                .get("agent")
                .and_then(|v| v.as_str())
                .unwrap_or("agent");
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

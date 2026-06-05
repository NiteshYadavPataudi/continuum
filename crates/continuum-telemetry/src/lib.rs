//! Telemetry: structured logging, OTLP span export, and Prometheus metrics.
//!
//! Called once from `main`. The dashboard is wired separately via
//! `continuum-cli::dashboard` (not through the tracing subscriber).

use opentelemetry_otlp::WithExportConfig;

/// Events consumed by the live dashboard TUI.
#[derive(Debug, Clone)]
pub enum DashboardEvent {
    /// The execution plan has been loaded.
    PlanLoaded {
        tasks: Vec<continuum_core::agent::TaskSnapshot>,
    },
    /// A task has entered the queue and is waiting on dependencies.
    TaskQueued {
        task_id: String,
        agent: String,
        label: String,
        ready_group: usize,
        depends_on: Vec<String>,
    },
    /// A task node has started.
    TaskStarted {
        task_id: String,
        agent: String,
        label: String,
        workspace: Option<String>,
    },
    /// A task emitted progress while running.
    TaskProgress {
        task_id: String,
        agent: String,
        message: String,
        percent: Option<u8>,
    },
    /// A task is blocked on approval, dependencies, or external state.
    TaskBlocked {
        task_id: String,
        agent: String,
        reason: String,
    },
    /// A task node completed.
    TaskCompleted {
        task_id: String,
        agent: String,
        status: String,
        percent: Option<u8>,
    },
    /// A task failed.
    TaskFailed {
        task_id: String,
        agent: String,
        error: String,
    },
    /// Validation stage update.
    ValidationUpdate {
        stage: String,
        passed: bool,
        findings: usize,
    },
    /// Token / cost update.
    CostUpdate { usd: f64, tokens: u64 },
    /// A workspace has been prepared for isolated execution.
    WorkspacePrepared {
        task_id: String,
        agent: String,
        workspace: String,
        isolated: bool,
        source: Option<String>,
    },
    /// A merge/conflict event.
    Merge {
        task_id: String,
        workspace: String,
        merged_files: usize,
        conflict: Option<String>,
    },
    /// A generic log line.
    Log {
        level: String,
        target: String,
        message: String,
    },
    /// A live assistant turn has started streaming.
    AssistantTurnStarted { turn_id: u64, prompt: String },
    /// A live assistant turn produced another text delta.
    AssistantTurnDelta { turn_id: u64, delta: String },
    /// A live assistant turn completed successfully.
    AssistantTurnCompleted { turn_id: u64 },
    /// A live assistant turn failed.
    AssistantTurnFailed { turn_id: u64, error: String },
    /// The session goal changed.
    GoalUpdated { goal: Option<String> },
    /// Shutdown signal.
    Shutdown,
}

/// Initialize the tracing subscriber with optional OTLP and Prometheus.
pub fn init() {
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::EnvFilter;
    use tracing_subscriber::Registry;

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,continuum=debug"));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_ansi(true);

    // Build subscriber starting from Registry
    let subscriber = Registry::default().with(filter).with(fmt_layer);

    // OTLP: enabled via CONTINUUM_OTLP_ENDPOINT env var
    if let Ok(endpoint) = std::env::var("CONTINUUM_OTLP_ENDPOINT") {
        if !endpoint.is_empty() {
            let tracer = match build_otlp_tracer(&endpoint) {
                Some(t) => t,
                None => {
                    eprintln!("warning: failed to build OTLP tracer for {endpoint}");
                    #[cfg(feature = "prometheus")]
                    start_prometheus_server();
                    let _ = subscriber.try_init();
                    return;
                }
            };
            let otlp_layer = tracing_opentelemetry::layer().with_tracer(tracer);
            let subscriber = subscriber.with(otlp_layer);
            #[cfg(feature = "prometheus")]
            start_prometheus_server();
            eprintln!("OTLP tracing enabled -> {endpoint}");
            let _ = subscriber.try_init();
            return;
        }
    }

    #[cfg(feature = "prometheus")]
    start_prometheus_server();

    let _ = subscriber.try_init();
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_event_plan_loaded() {
        let event = DashboardEvent::PlanLoaded { tasks: vec![] };
        match event {
            DashboardEvent::PlanLoaded { ref tasks } => assert!(tasks.is_empty()),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_dashboard_event_task_queued() {
        let event = DashboardEvent::TaskQueued {
            task_id: "task-1".into(),
            agent: "test-agent".into(),
            label: "do something".into(),
            ready_group: 0,
            depends_on: vec![],
        };
        match event {
            DashboardEvent::TaskQueued {
                ref task_id,
                ref agent,
                ..
            } => {
                assert_eq!(task_id, "task-1");
                assert_eq!(agent, "test-agent");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_dashboard_event_log() {
        let event = DashboardEvent::Log {
            level: "INFO".into(),
            target: "executor".into(),
            message: "started".into(),
        };
        match event {
            DashboardEvent::Log {
                ref level,
                ref target,
                ref message,
            } => {
                assert_eq!(level, "INFO");
                assert_eq!(target, "executor");
                assert_eq!(message, "started");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_dashboard_event_cost_update() {
        let event = DashboardEvent::CostUpdate {
            usd: 0.05,
            tokens: 1500,
        };
        match event {
            DashboardEvent::CostUpdate { usd, tokens } => {
                assert!((usd - 0.05).abs() < 1e-6);
                assert_eq!(tokens, 1500);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_dashboard_event_validation_update() {
        let event = DashboardEvent::ValidationUpdate {
            stage: "compile".into(),
            passed: true,
            findings: 0,
        };
        match event {
            DashboardEvent::ValidationUpdate {
                ref stage,
                passed,
                findings,
            } => {
                assert_eq!(stage, "compile");
                assert!(passed);
                assert_eq!(findings, 0);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_dashboard_event_shutdown() {
        let event = DashboardEvent::Shutdown;
        match event {
            DashboardEvent::Shutdown => {}
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_dashboard_event_goal_updated() {
        let event = DashboardEvent::GoalUpdated {
            goal: Some("ship the TUI".into()),
        };
        match event {
            DashboardEvent::GoalUpdated { ref goal } => {
                assert_eq!(goal.as_deref(), Some("ship the TUI"));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_dashboard_event_clone() {
        let event = DashboardEvent::Log {
            level: "WARN".into(),
            target: "test".into(),
            message: "warning message".into(),
        };
        let cloned = event.clone();
        match cloned {
            DashboardEvent::Log { ref level, .. } => assert_eq!(level, "WARN"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_dashboard_event_task_progress() {
        let event = DashboardEvent::TaskProgress {
            task_id: "task-2".into(),
            agent: "coding".into(),
            message: "50% done".into(),
            percent: Some(50),
        };
        match event {
            DashboardEvent::TaskProgress { percent, .. } => {
                assert_eq!(percent, Some(50));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_dashboard_event_merge() {
        let event = DashboardEvent::Merge {
            task_id: "task-3".into(),
            workspace: "/workspace".into(),
            merged_files: 3,
            conflict: None,
        };
        match event {
            DashboardEvent::Merge {
                merged_files,
                ref conflict,
                ..
            } => {
                assert_eq!(merged_files, 3);
                assert!(conflict.is_none());
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_dashboard_event_workspace_prepared() {
        let event = DashboardEvent::WorkspacePrepared {
            task_id: "task-4".into(),
            agent: "security".into(),
            workspace: "/tmp/sandbox".into(),
            isolated: true,
            source: Some("main".into()),
        };
        match event {
            DashboardEvent::WorkspacePrepared { isolated, .. } => {
                assert!(isolated);
            }
            _ => panic!("wrong variant"),
        }
    }
}

fn build_otlp_tracer(endpoint: &str) -> Option<opentelemetry_sdk::trace::Tracer> {
    use opentelemetry::trace::TracerProvider as _;

    let provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(endpoint),
        )
        .with_trace_config(opentelemetry_sdk::trace::Config::default().with_resource(
            opentelemetry_sdk::Resource::new(vec![opentelemetry::KeyValue::new(
                "service.name",
                "continuum",
            )]),
        ))
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .ok()?;

    Some(provider.tracer("continuum"))
}

#[cfg(feature = "prometheus")]
fn start_prometheus_server() {
    let addr: std::net::SocketAddr = std::env::var("CONTINUUM_METRICS_ADDR")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| "127.0.0.1:9477".parse().unwrap());

    tokio::spawn(async move {
        let builder = metrics_exporter_prometheus::PrometheusBuilder::new();
        match builder.with_http_listener(addr).build() {
            Ok(_exporter) => {
                eprintln!("Prometheus metrics at http://{addr}/metrics");
                std::future::pending::<()>().await;
            }
            Err(e) => {
                eprintln!("warning: failed to start Prometheus exporter: {e}");
            }
        }
    });
}

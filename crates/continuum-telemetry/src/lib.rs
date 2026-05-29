//! Telemetry: structured logging, OTLP span export, and Prometheus metrics.
//!
//! Called once from `main`. The dashboard is wired separately via
//! `continuum-cli::dashboard` (not through the tracing subscriber).

use opentelemetry_otlp::WithExportConfig;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

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
    /// Shutdown signal.
    Shutdown,
}

/// Helper to hold an optional OTLP layer alongside the fmt layer.
/// Builds the subscriber and initialises it.
pub fn init() {
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,continuum=debug"));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_ansi(true);

    let subscriber = tracing_subscriber::registry().with(filter).with(fmt_layer);

    #[cfg(feature = "prometheus")]
    start_prometheus_server();

    let _ = subscriber.try_init();
}

#[allow(dead_code)]
fn build_otlp_layer(
    endpoint: &str,
) -> Option<Box<dyn tracing_subscriber::Layer<tracing_subscriber::Registry> + Send + Sync>> {
    use opentelemetry::trace::TracerProvider as _;

    let tracer = opentelemetry_otlp::new_pipeline()
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

    let sdk_tracer = tracer.tracer("continuum");
    let layer = tracing_opentelemetry::layer().with_tracer(sdk_tracer);
    Some(Box::new(layer) as Box<dyn tracing_subscriber::Layer<_> + Send + Sync>)
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

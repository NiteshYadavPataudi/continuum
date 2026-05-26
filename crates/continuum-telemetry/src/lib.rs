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
    /// A task node has started.
    TaskStarted {
        agent: String,
        label: String,
    },
    /// A task node completed.
    TaskCompleted {
        agent: String,
        status: String,
    },
    /// Validation stage update.
    ValidationUpdate {
        stage: String,
        passed: bool,
        findings: usize,
    },
    /// Token / cost update.
    CostUpdate {
        usd: f64,
        tokens: u64,
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
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_ansi(true);

    // Build at most one subscriber; use conditional stacking.
    let subscriber = make_subscriber(fmt_layer);

    #[cfg(feature = "prometheus")]
    start_prometheus_server();

    let _ = subscriber.try_init();
}

/// Compose the layered subscriber.
///
/// We use a Vec of boxed layers so that the type is uniform regardless of
/// whether OTLP is enabled.
fn make_subscriber(
    fmt_layer: impl tracing_subscriber::Layer<tracing_subscriber::Registry>
        + Send
        + Sync
        + 'static,
) -> impl tracing::Subscriber + Send + Sync {
    let mut layers: Vec<
        Box<dyn tracing_subscriber::Layer<tracing_subscriber::Registry> + Send + Sync>,
    > = vec![Box::new(fmt_layer)];

    if let Ok(endpoint) = std::env::var("CONTINUUM_OTLP_ENDPOINT") {
        if let Some(otlp_layer) = build_otlp_layer(&endpoint) {
            layers.push(otlp_layer);
        }
    }

    tracing_subscriber::registry().with(layers)
}

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
        .with_trace_config(
            opentelemetry_sdk::trace::Config::default().with_resource(
                opentelemetry_sdk::Resource::new(vec![
                    opentelemetry::KeyValue::new("service.name", "continuum"),
                ]),
            ),
        )
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

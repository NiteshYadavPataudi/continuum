use std::sync::Arc;

use tracing_subscriber::layer::Layer;
use tracing_subscriber::registry::LookupSpan;

use continuum_storage::ReplayEventRepo;

/// A tracing layer that persists events to `replay_events` for later replay.
pub struct ReplayLayer {
    repo: Arc<ReplayEventRepo>,
    session_id: String,
}

impl ReplayLayer {
    /// Create a new replay layer for the given session.
    pub fn new(repo: ReplayEventRepo, session_id: String) -> Self {
        Self {
            repo: Arc::new(repo),
            session_id,
        }
    }
}

impl<S> Layer<S> for ReplayLayer
where
    S: tracing::Subscriber + for<'span> LookupSpan<'span>,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let mut fields = serde_json::Map::new();
        let mut visitor = JsonVisitor(&mut fields);
        event.record(&mut visitor);

        let payload = serde_json::Value::Object(fields);
        let target = event.metadata().target();
        let payload_str = payload.to_string();

        let repo = self.repo.clone();
        let session_id = self.session_id.clone();
        let target = target.to_string();

        tokio::spawn(async move {
            let _ = repo.create(&session_id, &target, &payload_str).await;
        });
    }
}

struct JsonVisitor<'a>(&'a mut serde_json::Map<String, serde_json::Value>);

impl<'a> tracing::field::Visit for JsonVisitor<'a> {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.0
            .insert(field.name().to_string(), serde_json::Value::String(value.to_string()));
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0.insert(
            field.name().to_string(),
            serde_json::Value::String(format!("{value:?}")),
        );
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.0
            .insert(field.name().to_string(), serde_json::Value::Number(value.into()));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.0
            .insert(field.name().to_string(), serde_json::Value::Number(value.into()));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.0
            .insert(field.name().to_string(), serde_json::Value::Bool(value));
    }
}

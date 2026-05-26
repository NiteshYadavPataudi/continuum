use std::sync::Mutex;

use async_trait::async_trait;

/// An audit event recorded for compliance and forensics.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditEvent {
    /// Category of event (e.g. "tool_call", "model_call", "sandbox_exec").
    pub event_type: String,
    /// Agent that produced the event.
    pub agent: String,
    /// Associated task ID, if any.
    pub task_id: Option<String>,
    /// Structured event detail.
    pub detail: serde_json::Value,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
}

/// Isolated audit log that prevents tampering after event recording.
#[async_trait]
pub trait AuditLog: Send + Sync {
    /// Record an audit event.
    async fn record(&self, event: AuditEvent) -> Result<(), String>;

    /// Query recent events by type.
    async fn query(&self, event_type: &str, limit: u64) -> Result<Vec<AuditEvent>, String>;
}

/// In-memory audit log with append-only semantics.
pub struct MemAuditLog {
    events: Mutex<Vec<AuditEvent>>,
}

impl MemAuditLog {
    /// Create an empty in-memory audit log.
    pub fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
        }
    }
}

impl Default for MemAuditLog {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AuditLog for MemAuditLog {
    async fn record(&self, event: AuditEvent) -> Result<(), String> {
        let mut events = self.events.lock().map_err(|e| e.to_string())?;
        events.push(event);
        Ok(())
    }

    async fn query(&self, event_type: &str, limit: u64) -> Result<Vec<AuditEvent>, String> {
        let events = self.events.lock().map_err(|e| e.to_string())?;
        let filtered: Vec<AuditEvent> = events
            .iter()
            .filter(|e| e.event_type == event_type)
            .rev()
            .take(limit as usize)
            .cloned()
            .collect();
        Ok(filtered)
    }
}

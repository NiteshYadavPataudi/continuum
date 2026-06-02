use std::sync::Arc;
use std::time::Duration;

use continuum_core::ids::SessionId;
use tokio::sync::Mutex;

use crate::store::SqliteRecovery;

/// Periodically writes heartbeats for a session.
pub struct HeartbeatMonitor {
    recovery: Arc<SqliteRecovery>,
    session: SessionId,
    running: Arc<Mutex<bool>>,
    retry_count: Arc<Mutex<i32>>,
    cumulative_usd: Arc<Mutex<f64>>,
}

impl HeartbeatMonitor {
    /// Create a new heartbeat monitor for `session`.
    pub fn new(recovery: Arc<SqliteRecovery>, session: SessionId) -> Self {
        Self {
            recovery,
            session,
            running: Arc::new(Mutex::new(false)),
            retry_count: Arc::new(Mutex::new(0)),
            cumulative_usd: Arc::new(Mutex::new(0.0)),
        }
    }

    /// Start the heartbeat loop. Runs until the returned handle is dropped
    /// or `stop` is called.
    pub async fn start(&self) {
        let mut running = self.running.lock().await;
        *running = true;
        drop(running);

        let recovery = self.recovery.clone();
        let session = self.session;
        let running = self.running.clone();
        let retry_count = self.retry_count.clone();
        let cumulative_usd = self.cumulative_usd.clone();

        tokio::spawn(async move {
            loop {
                {
                    let r = running.lock().await;
                    if !*r {
                        break;
                    }
                }
                let (rc, cu) = {
                    let rc = retry_count.lock().await;
                    let cu = cumulative_usd.lock().await;
                    (*rc, *cu)
                };
                let _ = recovery.record_heartbeat(session, rc, cu).await;
                tokio::time::sleep(Duration::from_secs(30)).await;
            }
        });
    }

    /// Update the retry count tracked by heartbeat.
    pub async fn increment_retry(&self) {
        let mut rc = self.retry_count.lock().await;
        *rc += 1;
    }

    /// Update the cumulative USD tracked by heartbeat.
    pub async fn add_cost(&self, usd: f64) {
        let mut cu = self.cumulative_usd.lock().await;
        *cu += usd;
    }

    /// Stop the heartbeat loop.
    pub async fn stop(&self) {
        let mut running = self.running.lock().await;
        *running = false;
    }
}

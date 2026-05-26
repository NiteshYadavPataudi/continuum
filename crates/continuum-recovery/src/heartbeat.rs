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
}

impl HeartbeatMonitor {
    /// Create a new heartbeat monitor for `session`.
    pub fn new(recovery: Arc<SqliteRecovery>, session: SessionId) -> Self {
        Self {
            recovery,
            session,
            running: Arc::new(Mutex::new(false)),
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

        tokio::spawn(async move {
            loop {
                {
                    let r = running.lock().await;
                    if !*r {
                        break;
                    }
                }
                let _ = recovery.record_heartbeat(session, 0, 0.0).await;
                tokio::time::sleep(Duration::from_secs(30)).await;
            }
        });
    }

    /// Stop the heartbeat loop.
    pub async fn stop(&self) {
        let mut running = self.running.lock().await;
        *running = false;
    }
}

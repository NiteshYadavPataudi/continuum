use continuum_core::ids::{CheckpointId, SessionId};
use continuum_core::recovery::RecoveryStore;

use super::{CmdResult, RollbackArgs};

pub async fn run(args: RollbackArgs) -> CmdResult {
    let root = std::env::current_dir().unwrap_or_default();
    let db_path = root.join(".continuum").join("state.db");

    let storage = continuum_storage::Storage::open(&db_path)
        .await
        .map_err(|e| format!("failed to open storage: {e}"))?;

    let checkpoints = continuum_storage::CheckpointRepo::new(storage.pool().clone());
    let heartbeats = continuum_storage::HeartbeatRepo::new(storage.pool().clone());
    let events = continuum_storage::ReplayEventRepo::new(storage.pool().clone());
    let memory_repo = continuum_storage::MemoryRepo::new(storage.pool().clone());
    let vector = continuum_storage::VectorBackend::Memory(continuum_storage::MemoryIndex::new());

    let recovery = continuum_recovery::SqliteRecovery::new(checkpoints, heartbeats, events, memory_repo, vector);

    let to = match args.to {
        Some(ref s) => CheckpointId::from(
            uuid::Uuid::parse_str(s)
                .map_err(|e| format!("invalid checkpoint ID: {e}"))?,
        ),
        None => {
            let session_id = SessionId::from(
                uuid::Uuid::parse_str(&args.session)
                    .map_err(|e| format!("invalid session ID: {e}"))?,
            );
            match recovery
                .latest(session_id)
                .await
                .map_err(|e| format!("failed to get latest checkpoint: {e}"))?
            {
                Some(prev) => prev.id,
                None => {
                    println!("No checkpoints found for session {}", args.session);
                    return Ok(());
                }
            }
        }
    };

    let state = recovery
        .rollback(to)
        .await
        .map_err(|e| format!("rollback failed: {e}"))?;

    println!(
        "Rolled back to checkpoint {to}. State: {}",
        serde_json::to_string_pretty(&state.payload).unwrap_or_default()
    );

    Ok(())
}

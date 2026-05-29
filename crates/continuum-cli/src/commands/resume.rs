use continuum_core::ids::SessionId;
use continuum_core::recovery::RecoveryStore;

use super::{CmdResult, ResumeArgs};

pub async fn run(args: ResumeArgs) -> CmdResult {
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

    let recovery = std::sync::Arc::new(continuum_recovery::SqliteRecovery::new(
        checkpoints,
        heartbeats,
        events,
        memory_repo,
        vector,
    ));

    let session_id = match args.session {
        Some(s) => SessionId::from(
            uuid::Uuid::parse_str(&s).map_err(|e| format!("invalid session ID: {e}"))?,
        ),
        None => {
            // Find the most recent session with checkpoints
            let _ = recovery;
            println!("No session ID provided. Use `continuum resume --session <ID>`.");
            return Ok(());
        }
    };

    let latest = recovery
        .latest(session_id)
        .await
        .map_err(|e| format!("failed to get latest checkpoint: {e}"))?;

    match latest {
        Some(ckpt) => {
            let state_outline = ckpt
                .state
                .payload
                .get("last_agent")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            println!(
                "Resuming session {} from checkpoint {} (last agent: {})",
                ckpt.session, ckpt.id, state_outline
            );
            println!(
                "Checkpoint created at: {}",
                ckpt.created_at
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_default()
            );
        }
        None => {
            println!("No checkpoints found for session {session_id}");
        }
    }

    Ok(())
}

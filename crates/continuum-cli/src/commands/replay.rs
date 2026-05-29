use continuum_core::ids::{CheckpointId, SessionId};
use continuum_core::recovery::RecoveryStore;

use super::{CmdResult, ReplayArgs};

pub async fn run(args: ReplayArgs) -> CmdResult {
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

    let recovery = continuum_recovery::SqliteRecovery::new(
        checkpoints,
        heartbeats,
        events,
        memory_repo,
        vector,
    );

    let session_id = SessionId::from(
        uuid::Uuid::parse_str(&args.session).map_err(|e| format!("invalid session ID: {e}"))?,
    );

    let from = match args.from {
        Some(ref s) => CheckpointId::from(
            uuid::Uuid::parse_str(s).map_err(|e| format!("invalid checkpoint ID: {e}"))?,
        ),
        None => CheckpointId::from(uuid::Uuid::nil()),
    };

    let mut stream = recovery
        .replay(session_id, from)
        .await
        .map_err(|e| format!("failed to start replay: {e}"))?;

    use futures::StreamExt;
    let mut count = 0usize;
    while let Some(event) = stream.next().await {
        match event {
            Ok(ev) => {
                println!(
                    "[{}] {}",
                    ev.at
                        .format(&time::format_description::well_known::Rfc3339)
                        .unwrap_or_default(),
                    serde_json::to_string_pretty(&ev.payload).unwrap_or_default()
                );
                count += 1;
            }
            Err(e) => eprintln!("replay error: {e}"),
        }
    }

    println!("\nReplay complete: {count} events");
    Ok(())
}

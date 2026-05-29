use continuum_core::memory::MemoryStore;
use std::sync::Arc;

use super::{CmdResult, MemoryArgs};

pub async fn run(args: MemoryArgs) -> CmdResult {
    let root = std::env::current_dir().unwrap_or_default();
    let db_path = root.join(".continuum").join("state.db");

    let storage = continuum_storage::Storage::open(&db_path)
        .await
        .map_err(|e| format!("failed to open storage: {e}"))?;

    let memory_repo = continuum_storage::MemoryRepo::new(storage.pool().clone());
    let vector = continuum_storage::VectorBackend::Memory(continuum_storage::MemoryIndex::new());

    let memory: Arc<dyn MemoryStore> =
        Arc::new(continuum_memory::LayeredMemory::new(memory_repo, vector));

    match args.action.as_str() {
        "list" => {
            let repo = continuum_storage::MemoryRepo::new(storage.pool().clone());
            let items = repo
                .list_by_run("session-0")
                .await
                .map_err(|e| format!("failed to list memory: {e}"))?;

            if items.is_empty() {
                println!("No memory items found.");
            } else {
                println!("Memory items:");
                for item in &items {
                    println!(
                        "  [{}] layer={}, tokens={}, tags={}",
                        item.id, item.layer, item.token_count, item.summary
                    );
                }
                println!("\nTotal: {} items", items.len());
            }
        }
        "compress" => {
            let report = memory
                .compress(continuum_core::memory::CompressionScope::AllHot)
                .await
                .map_err(|e| format!("compression failed: {e}"))?;
            println!(
                "Compression complete: processed={}, promoted={}, tokens_freed={}",
                report.processed, report.promoted, report.tokens_freed
            );
        }
        "purge" => {
            println!("Purge not yet implemented. Use `continuum memory list` then clear manually.");
        }
        other => {
            println!("Unknown memory action: {other}");
            println!("Usage: continuum memory <list|compress|purge>");
        }
    }

    Ok(())
}

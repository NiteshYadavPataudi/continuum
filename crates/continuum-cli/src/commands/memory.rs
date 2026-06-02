use super::{CmdResult, MemoryArgs};
use continuum_core::memory::MemoryStore;

pub async fn run(args: MemoryArgs) -> CmdResult {
    let root = std::env::current_dir().unwrap_or_default();
    let db_path = root.join(".continuum").join("state.db");

    let storage = continuum_storage::Storage::open(&db_path)
        .await
        .map_err(|e| format!("failed to open storage: {e}"))?;

    let pool = storage.pool().clone();

    match args.action.as_str() {
        "list" => {
            let repo = continuum_storage::MemoryRepo::new(pool);
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
            let memory_repo = continuum_storage::MemoryRepo::new(pool);
            let vector =
                continuum_storage::VectorBackend::Memory(continuum_storage::MemoryIndex::new());
            let memory =
                std::sync::Arc::new(continuum_memory::LayeredMemory::new(memory_repo, vector));
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
            let repo = continuum_storage::MemoryRepo::new(pool);
            let count = repo
                .delete_by_run("session-0")
                .await
                .map_err(|e| format!("failed to purge memory: {e}"))?;
            println!("Purged {} memory item(s).", count);
        }
        other => {
            println!("Unknown memory action: {other}");
            println!("Usage: continuum memory <list|compress|purge>");
        }
    }

    Ok(())
}

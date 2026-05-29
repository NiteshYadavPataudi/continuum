use std::sync::Arc;

use continuum_core::memory::{CompressionReport, CompressionScope, MemoryError};
use continuum_storage::MemoryRepo;
use continuum_storage::VectorBackend;

/// Compresses hot memory items into warm summaries to free token budget.
pub struct Compressor {
    repo: Arc<MemoryRepo>,
    #[allow(dead_code)]
    vector: Arc<VectorBackend>,
}

impl Compressor {
    /// Create a new compressor backed by the given storage repos.
    pub fn new(repo: Arc<MemoryRepo>, vector: Arc<VectorBackend>) -> Self {
        Self { repo, vector }
    }

    /// Compress memory items matching `scope`, returning a summary report.
    pub async fn compress(
        &mut self,
        scope: CompressionScope,
    ) -> Result<CompressionReport, MemoryError> {
        let run_id = "session-0";
        let hot_items = self
            .repo
            .list_by_run(run_id)
            .await
            .map_err(|e| MemoryError::Storage(e.to_string()))?;

        let hot: Vec<_> = hot_items.iter().filter(|r| r.layer == "hot").collect();

        let batch_size = 8usize;
        let total = hot.len();
        let mut promoted = 0u32;
        let mut tokens_freed = 0u32;

        match scope {
            CompressionScope::AllHot => {
                for chunk in hot.chunks(batch_size) {
                    let summary = format!(
                        "summary of {} items: {}",
                        chunk.len(),
                        chunk
                            .iter()
                            .map(|r| r.summary.as_str())
                            .collect::<Vec<_>>()
                            .join("; ")
                    );
                    let total_tokens: i32 = chunk.iter().map(|r| r.token_count).sum();
                    let _ = self
                        .repo
                        .create(
                            &uuid::Uuid::new_v4().to_string(),
                            run_id,
                            "warm",
                            "compressed",
                            &summary,
                            total_tokens,
                        )
                        .await
                        .map_err(|e| MemoryError::Storage(e.to_string()));
                    promoted += 1;
                    tokens_freed += total_tokens as u32;
                }
            }
            CompressionScope::HotOlderThanSecs(_secs) => {}
            _ => {}
        }

        Ok(CompressionReport::new(total as u32, promoted, tokens_freed))
    }
}

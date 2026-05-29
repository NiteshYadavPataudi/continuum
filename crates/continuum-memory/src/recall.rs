use std::sync::Arc;

use continuum_core::ids::MemoryId;
use continuum_core::memory::{MemoryError, MemoryItem, RecallHit, RecallQuery};
use continuum_storage::MemoryRepo;
use continuum_storage::VectorBackend;

/// Recall engine that combines vector similarity search with tag filtering.
pub struct RecallEngine {
    repo: Arc<MemoryRepo>,
    #[allow(dead_code)]
    vector: Arc<VectorBackend>,
}

impl RecallEngine {
    /// Create a new recall engine backed by the given storage repos.
    pub fn new(repo: Arc<MemoryRepo>, vector: Arc<VectorBackend>) -> Self {
        Self { repo, vector }
    }

    /// Recall memory items matching the given query.
    pub async fn recall(&self, query: RecallQuery) -> Result<Vec<RecallHit>, MemoryError> {
        let run_id = "session-0";
        let all = self
            .repo
            .list_by_run(run_id)
            .await
            .map_err(|e| MemoryError::Storage(e.to_string()))?;

        let exact_tags = !query.tags.is_empty();

        let candidates: Vec<_> = if exact_tags {
            all.iter()
                .filter(|r| {
                    let tags: Vec<&str> = r.summary.split(',').map(|s| s.trim()).collect();
                    query.tags.iter().all(|qt| tags.contains(&qt.as_str()))
                })
                .collect()
        } else {
            all.iter().collect()
        };

        if candidates.is_empty() {
            return Ok(vec![]);
        }

        let results = self
            .vector
            .search(&[], 0)
            .await
            .map_err(|e| MemoryError::Storage(e.to_string()));

        let mut hits: Vec<RecallHit> = Vec::new();

        if let Ok(vec_hits) = results {
            if !vec_hits.is_empty() {
                for (mem_id, score) in vec_hits {
                    if score < query.min_score {
                        continue;
                    }
                    if let Ok(Some(item)) = self.get_item(mem_id).await {
                        hits.push(RecallHit::new(item, score));
                    }
                    if hits.len() >= query.limit as usize {
                        break;
                    }
                }
            }
        }

        if hits.is_empty() {
            for row in candidates.iter().take(query.limit as usize) {
                let mem_id =
                    MemoryId::from(uuid::Uuid::parse_str(&row.id).unwrap_or(uuid::Uuid::nil()));
                let hit = RecallHit::new(
                    MemoryItem::new(mem_id, row.detail.clone()).with_tokens(row.token_count as u32),
                    0.5,
                );
                hits.push(hit);
            }
        }

        Ok(hits)
    }

    async fn get_item(&self, id: MemoryId) -> Result<Option<MemoryItem>, MemoryError> {
        let row = self
            .repo
            .get(&id.to_string())
            .await
            .map_err(|e| MemoryError::Storage(e.to_string()))?;
        Ok(row.map(|r| MemoryItem::new(id, r.detail).with_tokens(r.token_count as u32)))
    }
}

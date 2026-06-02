use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;
use uuid::Uuid;

use continuum_core::ids::MemoryId;
use continuum_core::memory::{
    CompressionReport, CompressionScope, MemoryError, MemoryItem, MemoryLayer, MemoryStore,
    RecallHit, RecallQuery,
};
use continuum_storage::MemoryRepo;
use continuum_storage::VectorBackend;

use super::compress::Compressor;
use super::recall::RecallEngine;

const DEFAULT_HOT_CAP_TOKENS: u64 = 64_000;

/// Three-layer memory store (hot / warm / cold) backed by SQLite and a vector index.
pub struct LayeredMemory {
    hot_cap_tokens: u64,
    repo: Arc<MemoryRepo>,
    #[allow(dead_code)]
    vector: Arc<VectorBackend>,
    compressor: Arc<Mutex<Compressor>>,
    recall: RecallEngine,
}

impl LayeredMemory {
    /// Create a new layered memory store.
    pub fn new(repo: MemoryRepo, vector: VectorBackend) -> Self {
        let repo = Arc::new(repo);
        let vector = Arc::new(vector);
        Self {
            hot_cap_tokens: DEFAULT_HOT_CAP_TOKENS,
            repo: repo.clone(),
            vector: vector.clone(),
            compressor: Arc::new(Mutex::new(Compressor::new(repo.clone(), vector.clone()))),
            recall: RecallEngine::new(repo.clone(), vector.clone()),
        }
    }

    /// Override the hot-layer token capacity (default 64 000).
    pub fn with_hot_cap(mut self, tokens: u64) -> Self {
        self.hot_cap_tokens = tokens;
        self
    }

    /// Return the current hot-layer token capacity.
    pub fn hot_cap(&self) -> u64 {
        self.hot_cap_tokens
    }
}

#[async_trait]
impl MemoryStore for LayeredMemory {
    async fn put(&self, layer: MemoryLayer, item: MemoryItem) -> Result<MemoryId, MemoryError> {
        let run_id = "session-0";
        let id = Uuid::new_v4().to_string();
        let layer_str = match layer {
            MemoryLayer::Hot => "hot",
            MemoryLayer::Warm => "warm",
            MemoryLayer::Cold => "cold",
        };
        let tags = item.tags.join(",");
        self.repo
            .create(
                &id,
                run_id,
                layer_str,
                &tags,
                &item.content,
                item.tokens as i32,
            )
            .await
            .map_err(|e| MemoryError::Storage(e.to_string()))?;

        let mem_id = MemoryId::from(Uuid::parse_str(&id).unwrap_or_else(|_| Uuid::nil()));
        Ok(mem_id)
    }

    async fn get(&self, id: MemoryId) -> Result<Option<MemoryItem>, MemoryError> {
        let row = self
            .repo
            .get(&id.to_string())
            .await
            .map_err(|e| MemoryError::Storage(e.to_string()))?;
        Ok(row.map(|r| {
            MemoryItem::new(
                MemoryId::from(Uuid::parse_str(&r.id).unwrap_or_else(|_| Uuid::nil())),
                r.detail,
            )
            .with_tokens(r.token_count as u32)
        }))
    }

    async fn recall(&self, query: RecallQuery) -> Result<Vec<RecallHit>, MemoryError> {
        self.recall.recall(query).await
    }

    async fn promote(&self, id: MemoryId, to: MemoryLayer) -> Result<(), MemoryError> {
        let to_str = match to {
            MemoryLayer::Hot => "hot",
            MemoryLayer::Warm => "warm",
            MemoryLayer::Cold => "cold",
        };
        tracing::info!(mem_id = %id, to = %to_str, "promote (pending layer update)");
        Ok(())
    }

    async fn compress(&self, scope: CompressionScope) -> Result<CompressionReport, MemoryError> {
        let mut compressor = self.compressor.lock().await;
        compressor.compress(scope).await
    }
}

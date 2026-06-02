//! Memory and vector-index traits.
//!
//! `continuum-memory` implements the three-layer engine (hot/warm/cold) on
//! top of `continuum-storage`'s SQLite + vector index. Agents call into
//! [`MemoryStore::recall`] to fetch relevant context; the runtime journals
//! decisions and compresses on overflow.

use crate::ids::MemoryId;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Which tier a memory item lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MemoryLayer {
    /// Recent, full-fidelity context. Capped in size; evicts to warm.
    Hot,
    /// Compressed summaries of recent work. Cheap to recall.
    Warm,
    /// Long-term archive, semantically indexed only.
    Cold,
}

/// A single memory item.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MemoryItem {
    /// Stable ID assigned by the store.
    pub id: MemoryId,
    /// Free-form text content.
    pub content: String,
    /// Tags for filtering during recall.
    pub tags: Vec<String>,
    /// Token count, for compression accounting.
    pub tokens: u32,
}

impl MemoryItem {
    /// Create a new memory item.
    pub fn new(id: MemoryId, content: impl Into<String>) -> Self {
        Self {
            id,
            content: content.into(),
            tags: Vec::new(),
            tokens: 0,
        }
    }

    /// Add a tag.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set token count.
    pub fn with_tokens(mut self, tokens: u32) -> Self {
        self.tokens = tokens;
        self
    }
}

/// A recall query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RecallQuery {
    /// Natural-language query.
    pub text: String,
    /// Required tags (AND across items).
    pub tags: Vec<String>,
    /// Maximum number of hits.
    pub limit: u32,
    /// Minimum similarity score (0.0 = no filter).
    pub min_score: f32,
}

impl RecallQuery {
    /// Create a new recall query.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            tags: Vec::new(),
            limit: 10,
            min_score: 0.0,
        }
    }

    /// Add a required tag.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set result limit.
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = limit;
        self
    }

    /// Set minimum similarity score.
    pub fn with_min_score(mut self, min: f32) -> Self {
        self.min_score = min;
        self
    }
}

/// A single recall result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RecallHit {
    /// Matched item.
    pub item: MemoryItem,
    /// Similarity score (0.0..=1.0).
    pub score: f32,
}

impl RecallHit {
    /// Create a new recall hit.
    pub fn new(item: MemoryItem, score: f32) -> Self {
        Self { item, score }
    }
}

/// Scope of a compression pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum CompressionScope {
    /// Compress all hot-tier items older than N seconds.
    HotOlderThanSecs(u64),
    /// Compress everything in the hot tier.
    AllHot,
}

/// Report from a compression pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CompressionReport {
    /// Items considered.
    pub processed: u32,
    /// Items moved to warm tier.
    pub promoted: u32,
    /// Tokens reclaimed.
    pub tokens_freed: u32,
}

impl CompressionReport {
    /// Create a new compression report.
    pub fn new(processed: u32, promoted: u32, tokens_freed: u32) -> Self {
        Self {
            processed,
            promoted,
            tokens_freed,
        }
    }
}

/// Errors specific to memory operations.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum MemoryError {
    /// Storage layer failure.
    #[error("storage: {0}")]
    Storage(String),
    /// Recall produced no results when at least one was required.
    #[error("not found")]
    NotFound,
    /// Catch-all.
    #[error("memory error: {0}")]
    Other(String),
}

/// Three-layer memory engine contract.
#[async_trait]
pub trait MemoryStore: Send + Sync {
    /// Insert an item into the given layer.
    async fn put(&self, layer: MemoryLayer, item: MemoryItem) -> Result<MemoryId, MemoryError>;

    /// Fetch an item by ID.
    async fn get(&self, id: MemoryId) -> Result<Option<MemoryItem>, MemoryError>;

    /// Semantic recall.
    async fn recall(&self, query: RecallQuery) -> Result<Vec<RecallHit>, MemoryError>;

    /// Move an item to a different layer.
    async fn promote(&self, id: MemoryId, to: MemoryLayer) -> Result<(), MemoryError>;

    /// Compress the hot tier into warm summaries.
    async fn compress(&self, scope: CompressionScope) -> Result<CompressionReport, MemoryError>;
}

// ---- Vector index ----------------------------------------------------------

/// A point upserted into the vector index.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct VectorPoint {
    /// Stable ID for the point (typically a [`MemoryId`]).
    pub id: String,
    /// The embedding.
    pub vector: Vec<f32>,
    /// Optional sidecar metadata.
    pub metadata: serde_json::Value,
}

/// A query against the vector index.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct VectorQuery {
    /// Query vector.
    pub vector: Vec<f32>,
    /// Maximum hits.
    pub limit: u32,
    /// Optional metadata filter (JSON path → expected value).
    pub filter: Option<serde_json::Value>,
}

/// One vector-search hit.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct VectorHit {
    /// ID of the matched point.
    pub id: String,
    /// Cosine similarity (0.0..=1.0).
    pub score: f32,
    /// Sidecar metadata.
    pub metadata: serde_json::Value,
}

impl VectorHit {
    /// Create a new vector hit.
    pub fn new(id: String, score: f32, metadata: serde_json::Value) -> Self {
        Self { id, score, metadata }
    }
}

/// Errors specific to the vector index.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum VectorError {
    /// Backing store failure.
    #[error("backing store: {0}")]
    Backing(String),
    /// Dimension mismatch.
    #[error("dimension mismatch: expected {expected}, got {got}")]
    Dimension {
        /// Expected dimensionality.
        expected: u32,
        /// Actual dimensionality of the query/point.
        got: u32,
    },
    /// Catch-all.
    #[error("vector error: {0}")]
    Other(String),
}

/// Pluggable vector store. `sqlite-vec` is the default in
/// `continuum-storage`; Qdrant is available behind a feature flag.
#[async_trait]
pub trait VectorIndex: Send + Sync {
    /// Insert or update points.
    async fn upsert(&self, points: Vec<VectorPoint>) -> Result<(), VectorError>;

    /// Search for nearest neighbours.
    async fn search(&self, q: VectorQuery) -> Result<Vec<VectorHit>, VectorError>;
}

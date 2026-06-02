pub mod memory;

#[cfg(feature = "vector-qdrant")]
pub mod qdrant;

use crate::error::VectorError;
use continuum_core::ids::MemoryId;

/// A vector similarity backend that can be either in-memory or backed by Qdrant.
pub enum VectorBackend {
    /// An in-memory cosine-similarity index.
    Memory(memory::MemoryIndex),
    /// A Qdrant-backed index (behind the `vector-qdrant` feature).
    #[cfg(feature = "vector-qdrant")]
    Qdrant(qdrant::QdrantIndex),
}

impl VectorBackend {
    /// Insert or update a vector by its `MemoryId`.
    ///
    /// # Errors
    /// Returns `VectorError::DimensionMismatch` if the vector dimension is wrong.
    pub async fn upsert(&self, id: MemoryId, vector: Vec<f32>) -> Result<(), VectorError> {
        match self {
            VectorBackend::Memory(idx) => idx.upsert(id, vector).await,
            #[cfg(feature = "vector-qdrant")]
            VectorBackend::Qdrant(_idx) => Err(VectorError::Backing(
                "Qdrant upsert not yet implemented".into(),
            )),
        }
    }

    /// Search the vector index by cosine similarity, returning up to `limit` results
    /// as `(MemoryId, score)` pairs.
    ///
    /// # Errors
    /// Returns `VectorError::Backing` if the backend is not yet implemented.
    pub async fn search(
        &self,
        query: &[f32],
        limit: usize,
    ) -> Result<Vec<(MemoryId, f32)>, VectorError> {
        match self {
            VectorBackend::Memory(idx) => idx.search(query, limit).await,
            #[cfg(feature = "vector-qdrant")]
            VectorBackend::Qdrant(_idx) => Err(VectorError::Backing(
                "Qdrant search not yet implemented".into(),
            )),
        }
    }

    /// Delete a vector by its `MemoryId`.
    ///
    /// # Errors
    /// Returns `VectorError::Backing` if the backend is not yet implemented.
    pub async fn delete(&self, id: MemoryId) -> Result<(), VectorError> {
        match self {
            VectorBackend::Memory(idx) => idx.delete(id).await,
            #[cfg(feature = "vector-qdrant")]
            VectorBackend::Qdrant(_idx) => Err(VectorError::Backing(
                "Qdrant delete not yet implemented".into(),
            )),
        }
    }
}

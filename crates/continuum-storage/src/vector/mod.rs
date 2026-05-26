pub mod memory;

#[cfg(feature = "vector-qdrant")]
pub mod qdrant;

use crate::error::VectorError;
use continuum_core::ids::MemoryId;

pub enum VectorBackend {
    Memory(memory::MemoryIndex),
}

impl VectorBackend {
    pub async fn upsert(&self, id: MemoryId, vector: Vec<f32>) -> Result<(), VectorError> {
        match self {
            VectorBackend::Memory(idx) => idx.upsert(id, vector).await,
        }
    }

    pub async fn search(
        &self,
        query: &[f32],
        limit: usize,
    ) -> Result<Vec<(MemoryId, f32)>, VectorError> {
        match self {
            VectorBackend::Memory(idx) => idx.search(query, limit).await,
        }
    }

    pub async fn delete(&self, id: MemoryId) -> Result<(), VectorError> {
        match self {
            VectorBackend::Memory(idx) => idx.delete(id).await,
        }
    }
}

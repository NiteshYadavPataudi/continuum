use async_trait::async_trait;
use continuum_core::memory::{VectorHit, VectorIndex, VectorPoint, VectorQuery};

/// A Qdrant-backed vector index stub (not yet implemented).
pub struct QdrantIndex;

#[async_trait]
impl VectorIndex for QdrantIndex {
    async fn upsert(
        &self,
        _points: Vec<VectorPoint>,
    ) -> Result<(), continuum_core::memory::VectorError> {
        Err(continuum_core::memory::VectorError::Backing(
            "not implemented in phase 2".into(),
        ))
    }

    async fn search(
        &self,
        _q: VectorQuery,
    ) -> Result<Vec<VectorHit>, continuum_core::memory::VectorError> {
        Err(continuum_core::memory::VectorError::Backing(
            "not implemented in phase 2".into(),
        ))
    }
}

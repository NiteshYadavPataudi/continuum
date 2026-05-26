use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use continuum_core::ids::MemoryId;
use continuum_core::memory::{VectorHit, VectorIndex, VectorPoint, VectorQuery};
use uuid::Uuid;

use crate::error::VectorError;

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

pub struct MemoryIndex {
    vectors: Mutex<HashMap<Uuid, Vec<f32>>>,
}

impl MemoryIndex {
    pub fn new() -> Self {
        Self {
            vectors: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MemoryIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryIndex {
    pub async fn upsert(&self, id: MemoryId, vector: Vec<f32>) -> Result<(), VectorError> {
        let uuid = *id.as_uuid();
        let mut map = self.vectors.lock().unwrap();
        if let Some(existing) = map.get(&uuid) {
            if existing.len() != vector.len() {
                return Err(VectorError::DimensionMismatch {
                    expected: existing.len(),
                    got: vector.len(),
                });
            }
        }
        map.insert(uuid, vector);
        Ok(())
    }

    pub async fn search(
        &self,
        query: &[f32],
        limit: usize,
    ) -> Result<Vec<(MemoryId, f32)>, VectorError> {
        let map = self.vectors.lock().unwrap();
        let mut scored: Vec<(Uuid, f32)> = map
            .iter()
            .map(|(id, vec)| (*id, cosine_similarity(query, vec)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        let results = scored
            .into_iter()
            .map(|(uuid, score)| (MemoryId::from(uuid), score))
            .collect();
        Ok(results)
    }

    pub async fn delete(&self, id: MemoryId) -> Result<(), VectorError> {
        let mut map = self.vectors.lock().unwrap();
        map.remove(id.as_uuid());
        Ok(())
    }
}

#[async_trait]
impl VectorIndex for MemoryIndex {
    async fn upsert(
        &self,
        points: Vec<VectorPoint>,
    ) -> Result<(), continuum_core::memory::VectorError> {
        for point in points {
            let uuid = Uuid::parse_str(&point.id)
                .map_err(|e| continuum_core::memory::VectorError::Backing(e.to_string()))?;
            self.upsert(MemoryId::from(uuid), point.vector)
                .await
                .map_err(continuum_core::memory::VectorError::from)?;
        }
        Ok(())
    }

    async fn search(
        &self,
        q: VectorQuery,
    ) -> Result<Vec<VectorHit>, continuum_core::memory::VectorError> {
        let results = self
            .search(&q.vector, q.limit as usize)
            .await
            .map_err(continuum_core::memory::VectorError::from)?;
        let hits = results
            .into_iter()
            .map(|(id, score)| {
                serde_json::from_value(serde_json::json!({
                    "id": id.to_string(),
                    "score": score,
                    "metadata": null,
                }))
                .unwrap()
            })
            .collect();
        Ok(hits)
    }
}

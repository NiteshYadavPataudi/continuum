use continuum_core::ids::MemoryId;
#[allow(unused_imports)]
use continuum_core::memory::VectorIndex;
use continuum_storage::MemoryIndex;

#[tokio::test]
async fn test_vector_roundtrip() {
    let idx = MemoryIndex::new();
    let mut ids = Vec::new();

    for i in 0..100 {
        let id = MemoryId::new();
        let vec: Vec<f32> = (0..384).map(|j| (j as f32 + i as f32) * 0.01).collect();
        idx.upsert(id, vec).await.unwrap();
        ids.push(id);
    }

    let query: Vec<f32> = (0..384).map(|j| j as f32 * 0.01).collect();
    let results = idx.search(&query, 5).await.unwrap();

    assert_eq!(results[0].0, ids[0]);
}

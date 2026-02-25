use anyhow::Result;
use ruvector_core::index::hnsw::HnswIndex;
use ruvector_core::index::VectorIndex;
use ruvector_core::types::{DistanceMetric, HnswConfig};
use std::path::Path;

/// Embedding dimension for bge-small-en-v1.5.
pub const DIMENSIONS: usize = 384;

/// How many results to return from a search by default.
pub const DEFAULT_TOP_K: usize = 25;

/// A result from the search index: (note_id, similarity_score).
/// score is in [0.0, 1.0] — higher means more semantically similar.
pub struct IndexResult {
    pub note_id: String,
    pub score: f32,
}

/// Wraps the ruvector-core HNSW index.
/// Stores note UUID → embedding mappings and supports ANN search.
pub struct SearchIndex {
    hnsw: HnswIndex,
}

impl SearchIndex {
    /// Create a new empty index.
    /// `max_elements` is the expected upper bound of notes — can be generous.
    pub fn new(max_elements: usize) -> Result<Self> {
        let config = HnswConfig {
            m: 16,                  // connections per layer — 16 is a good default
            ef_construction: 200,   // build-time quality — higher = better index
            ef_search: 50,          // search-time recall — higher = better recall
            max_elements,
        };
        let hnsw = HnswIndex::new(DIMENSIONS, DistanceMetric::Cosine, config)
            .map_err(|e| anyhow::anyhow!("Failed to create HNSW index: {e}"))?;
        Ok(Self { hnsw })
    }

    /// Add a single note embedding to the index.
    pub fn add(&mut self, note_id: String, embedding: Vec<f32>) -> Result<()> {
        self.hnsw
            .add(note_id, embedding)
            .map_err(|e| anyhow::anyhow!("Index add failed: {e}"))
    }

    /// Add many note embeddings at once (more efficient than repeated add).
    pub fn add_batch(&mut self, entries: Vec<(String, Vec<f32>)>) -> Result<()> {
        self.hnsw
            .add_batch(entries)
            .map_err(|e| anyhow::anyhow!("Index batch add failed: {e}"))
    }

    /// Search for the `k` most semantically similar notes to `query_embedding`.
    /// Returns results sorted by descending similarity (highest first).
    pub fn search(&self, query_embedding: &[f32], k: usize) -> Result<Vec<IndexResult>> {
        let raw = self
            .hnsw
            .search(query_embedding, k)
            .map_err(|e| anyhow::anyhow!("Index search failed: {e}"))?;

        // ruvector-core returns cosine *distance* (lower = more similar).
        // Convert to similarity: score = 1.0 - distance, clamp to [0, 1].
        let mut results: Vec<IndexResult> = raw
            .into_iter()
            .map(|r| IndexResult {
                note_id: r.id,
                score: (1.0 - r.score).clamp(0.0, 1.0),
            })
            .collect();

        // Sort descending by similarity score.
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        Ok(results)
    }

    /// Number of notes currently in the index.
    pub fn len(&self) -> usize {
        self.hnsw.len()
    }

    pub fn is_empty(&self) -> bool {
        self.hnsw.len() == 0
    }

    /// Persist the index to disk atomically (write temp file, then rename).
    /// Prevents partial writes from corrupting the saved index.
    pub fn save(&self, path: &Path) -> Result<()> {
        let bytes = self
            .hnsw
            .serialize()
            .map_err(|e| anyhow::anyhow!("Index serialize failed: {e}"))?;

        // Write to a temp file alongside the target, then atomically rename.
        let tmp_path = path.with_extension("bin.tmp");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&tmp_path, bytes)?;
        std::fs::rename(&tmp_path, path)?;
        Ok(())
    }

    /// Load a previously saved index from disk.
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path)?;
        let hnsw = HnswIndex::deserialize(&bytes)
            .map_err(|e| anyhow::anyhow!("Index deserialize failed: {e}"))?;
        Ok(Self { hnsw })
    }
}

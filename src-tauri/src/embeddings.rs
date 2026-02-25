use std::path::Path;
use std::sync::Mutex;

use anyhow::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

/// Wraps the fastembed TextEmbedding model (bge-small-en-v1.5, 384 dims).
/// Model is downloaded and cached on first use (~33MB, one-time).
///
/// The inner `TextEmbedding` session is protected by a `Mutex` so that
/// concurrent calls from search queries and background delta indexing are
/// serialized, preventing heap corruption in the ONNX Runtime C++ layer.
pub struct EmbeddingPipeline {
    model: Mutex<TextEmbedding>,
}

impl EmbeddingPipeline {
    /// Initialize the embedding model. Downloads on first run, cached afterwards.
    /// `cache_dir` is the directory where the ONNX model files are stored.
    /// `show_progress` controls whether download progress is printed to stdout.
    pub fn new(cache_dir: &Path, show_progress: bool) -> Result<Self> {
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::BGESmallENV15)
                .with_cache_dir(cache_dir.to_path_buf())
                .with_show_download_progress(show_progress),
        )?;
        Ok(Self { model: Mutex::new(model) })
    }

    /// Embed a single text string. Returns a 384-dimensional vector.
    pub fn embed_one(&self, text: &str) -> Result<Vec<f32>> {
        let model = self.model.lock().map_err(|e| anyhow::anyhow!("model lock poisoned: {e}"))?;
        let mut results = model.embed(vec![text], None)?;
        let embedding = results.remove(0);
        Ok(normalize(embedding))
    }

    /// Embed a batch of texts. More efficient than calling embed_one repeatedly.
    /// Returns one 384-dim vector per input text, in the same order.
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }
        let model = self.model.lock().map_err(|e| anyhow::anyhow!("model lock poisoned: {e}"))?;
        let results = model.embed(texts.to_vec(), None)?;
        Ok(results.into_iter().map(normalize).collect())
    }
}

/// L2-normalize a vector so cosine similarity == dot product.
/// bge-small-en-v1.5 outputs are already normalized, but we normalize
/// defensively to guarantee correctness.
fn normalize(mut v: Vec<f32>) -> Vec<f32> {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 1e-10 {
        v.iter_mut().for_each(|x| *x /= norm);
    }
    v
}

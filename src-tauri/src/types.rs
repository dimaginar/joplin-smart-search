use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: String, // Joplin uses string UUIDs
    pub title: String,
    pub body: String,
    pub updated_time: i64, // Unix timestamp in ms
}

/// Lightweight note metadata kept in the in-memory cache.
/// Body is not stored to avoid holding all note content in RAM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteMetadata {
    pub id: String,
    pub title: String,
    pub updated_time: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub note: NoteMetadata,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStatus {
    pub total_notes: usize,
    pub indexed_notes: usize,
    pub is_ready: bool,
    pub is_downloading_model: bool,
    pub download_progress: f32, // 0.0 to 1.0
    pub error: Option<String>,
}

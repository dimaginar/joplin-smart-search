pub mod commands;
pub mod db;
pub mod embeddings;
pub mod index;
pub mod types;
pub mod watcher;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;

use tauri::Manager;

use crate::embeddings::EmbeddingPipeline;
use crate::index::SearchIndex;
use crate::types::{IndexStatus, NoteMetadata};

/// All runtime state shared across Tauri commands.
pub struct AppState {
    /// Path to the Joplin SQLite database.
    pub db_path: Option<String>,
    /// Loaded embedding model (all-MiniLM-L6-v2).
    /// Wrapped in Arc so it can be cloned out of the mutex for lock-free inference.
    pub embedding_pipeline: Option<Arc<EmbeddingPipeline>>,
    /// HNSW vector index.
    /// Wrapped in Arc<RwLock<_>> so searches hold a read lock concurrently while
    /// delta inserts hold a brief write lock.
    pub search_index: Option<Arc<tokio::sync::RwLock<SearchIndex>>>,
    /// In-memory note metadata cache for fast lookup after search.
    /// Maps note UUID → NoteMetadata (no body to keep RAM usage low).
    pub note_cache: HashMap<String, NoteMetadata>,
    /// Updated_time of the most-recently indexed note (Unix ms).
    /// Used by the file watcher for delta queries.
    pub last_scan_timestamp: i64,
    /// IDs of notes soft-deleted since the last full rebuild.
    /// Search results are filtered against this set so deleted notes
    /// disappear immediately without waiting for the next full rebuild.
    pub deleted_note_ids: HashSet<String>,
    /// Wall-clock instant of the last full index rebuild.
    /// Used to schedule the periodic background rebuild (every 5 minutes).
    pub last_full_rebuild_ts: std::time::Instant,
    /// Status reported to the frontend.
    pub index_status: IndexStatus,
    /// True while a full index build is running. Prevents concurrent rebuilds.
    pub is_indexing: bool,
    /// True while the embedding model is being loaded. Prevents duplicate downloads.
    pub is_pipeline_loading: bool,
    /// True while a delta update is running. Prevents overlapping delta passes
    /// from double-inserting embeddings into the HNSW index.
    pub is_delta_updating: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            db_path: None,
            embedding_pipeline: None,
            search_index: None,
            note_cache: HashMap::new(),
            last_scan_timestamp: 0,
            deleted_note_ids: HashSet::new(),
            last_full_rebuild_ts: std::time::Instant::now(),
            index_status: IndexStatus {
                total_notes: 0,
                indexed_notes: 0,
                is_ready: false,
                is_downloading_model: false,
                download_progress: 0.0,
                error: None,
            },
            is_indexing: false,
            is_pipeline_loading: false,
            is_delta_updating: false,
        }
    }
}

/// Type alias used in Tauri command signatures and background tasks.
pub type AppMutex = Mutex<AppState>;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Only log WARN and above in production to avoid leaking note content
    #[cfg(debug_assertions)]
    tracing_subscriber::fmt::init();
    #[cfg(not(debug_assertions))]
    tracing_subscriber::fmt().with_max_level(tracing::Level::WARN).init();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppMutex::new(AppState::default()))
        .invoke_handler(tauri::generate_handler![
            commands::detect_db_path,
            commands::set_joplin_db_path,
            commands::search_notes,
            commands::get_index_status,
            commands::get_note,
            commands::trigger_reindex,
            commands::open_in_joplin,
        ])
        .setup(|app| {
            // Set the window icon explicitly so the taskbar shows our icon on Linux.
            if let Some(window) = app.get_webview_window("main") {
                if let Some(icon) = app.default_window_icon() {
                    let _ = window.set_icon(icon.clone()).ok();
                }
            }
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                commands::startup_init(handle).await;
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

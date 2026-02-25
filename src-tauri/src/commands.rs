use std::collections::HashMap;
use std::sync::Arc;

use tauri::{Emitter, Manager};

use crate::types::{IndexStatus, Note, NoteMetadata, SearchResult};
use crate::AppMutex;

// ─── Tauri commands ────────────────────────────────────────────────────────────

/// Try to auto-detect the Joplin SQLite path. Returns None if not found.
/// Frontend uses this to pre-fill the path or prompt user to browse.
#[tauri::command]
pub async fn detect_db_path() -> Option<String> {
    crate::db::detect_joplin_db_path().map(|p| p.to_string_lossy().to_string())
}

/// Set the Joplin DB path manually (user browsed to it) and trigger full indexing.
#[tauri::command]
pub async fn set_joplin_db_path(
    path: String,
    state: tauri::State<'_, AppMutex>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    {
        let mut s = state.lock().await;
        s.db_path = Some(path);
        s.index_status.is_ready = false;
        s.index_status.indexed_notes = 0;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        run_full_indexing(app).await;
    });
    Ok(())
}

/// Semantic search. Returns up to 10 results ranked by similarity.
/// Returns an error string if the index is not yet ready.
#[tauri::command]
pub async fn search_notes(
    query: String,
    state: tauri::State<'_, AppMutex>,
) -> Result<Vec<SearchResult>, String> {
    // Clone Arc pointers + snapshot the cache and tombstones while holding the
    // lock, then release the lock before the expensive ML inference.
    let (pipeline, index_arc, cache_snapshot, tombstones) = {
        let s = state.lock().await;
        if !s.index_status.is_ready {
            return Err("index_not_ready".to_string());
        }
        let pipeline = s.embedding_pipeline.clone().ok_or("model_not_loaded")?;
        let index = s.search_index.clone().ok_or("index_not_ready")?;
        let cache_snapshot = s.note_cache.clone();
        let tombstones = s.deleted_note_ids.clone();
        (pipeline, index, cache_snapshot, tombstones)
    }; // lock released here

    let query_embedding = pipeline.embed_one(&query).map_err(|e| e.to_string())?;
    let index = index_arc.read().await;
    let hits = index
        .search(&query_embedding, crate::index::DEFAULT_TOP_K)
        .map_err(|e| e.to_string())?;
    drop(index);

    const MIN_SCORE: f32 = 0.30;
    // Deduplicate by note_id: HNSW may have multiple nodes for the same note
    // if it was edited/restored between full rebuilds. Keep the first (highest-score) hit.
    let mut seen_ids = std::collections::HashSet::new();
    let results: Vec<SearchResult> = hits
        .into_iter()
        .filter(|hit| !tombstones.contains(&hit.note_id))
        .filter_map(|hit| {
            cache_snapshot.get(&hit.note_id).map(|meta| SearchResult {
                note: meta.clone(),
                score: hit.score,
            })
        })
        .filter(|r| r.score >= MIN_SCORE)
        .filter(|r| seen_ids.insert(r.note.id.clone()))
        .collect();

    Ok(results)
}

/// Current indexing status — polled by the frontend status indicator.
#[tauri::command]
pub async fn get_index_status(
    state: tauri::State<'_, AppMutex>,
) -> Result<IndexStatus, String> {
    Ok(state.lock().await.index_status.clone())
}

/// Fetch the full note (including body) by ID. Called when user selects a result.
#[tauri::command]
pub async fn get_note(
    id: String,
    state: tauri::State<'_, AppMutex>,
) -> Result<Note, String> {
    let db_path = state
        .lock()
        .await
        .db_path
        .clone()
        .ok_or("db_not_configured")?;
    let conn = crate::db::open_joplin_db(&db_path).map_err(|e| e.to_string())?;
    crate::db::get_note_by_id(&conn, &id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "note_not_found".to_string())
}

/// Refresh: run a delta update to catch any notes added/edited/deleted since the
/// last scan. Fast (~1-2s for a handful of changes). Never blanks out the search UI.
#[tauri::command]
pub async fn trigger_reindex(
    state: tauri::State<'_, AppMutex>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let db_path = state
        .lock()
        .await
        .db_path
        .clone()
        .ok_or("db_not_configured")?;
    tauri::async_runtime::spawn(async move {
        run_delta_update(app, db_path).await;
    });
    Ok(())
}

// ─── Internal helpers ──────────────────────────────────────────────────────────

/// Called once on startup: auto-detect DB, load or build index, start watcher.
pub async fn startup_init(app: tauri::AppHandle) {
    let db_path = crate::db::detect_joplin_db_path();

    if let Some(path) = db_path {
        let path_str = path.to_string_lossy().to_string();
        {
            let state = app.state::<AppMutex>();
            let mut s = state.lock().await;
            s.db_path = Some(path_str.clone());
        }
        run_full_indexing(app.clone()).await;
        // Catch any notes added/edited/deleted while the app was closed.
        // The cached index.bin may be older than the DB, so run a delta
        // pass immediately rather than waiting for the file watcher to fire.
        run_delta_update(app.clone(), path_str).await;
        crate::watcher::start_watcher(app).await;
    }
    // If DB not found: index_status remains is_ready=false, db_path=None.
    // The frontend first-launch screen will prompt the user to locate it.
}

/// Build (or rebuild) the full HNSW index from the Joplin SQLite database.
/// Emits "index-status" events so the frontend can show progress.
pub async fn run_full_indexing(app: tauri::AppHandle) {
    // 0. Guard against concurrent rebuilds
    {
        let state = app.state::<AppMutex>();
        let mut s = state.lock().await;
        if s.is_indexing {
            return;
        }
        s.is_indexing = true;
    }

    run_full_indexing_inner(app.clone()).await;

    let state = app.state::<AppMutex>();
    state.lock().await.is_indexing = false;
}

async fn run_full_indexing_inner(app: tauri::AppHandle) {
    // 1. Grab db_path
    let db_path = {
        let state = app.state::<AppMutex>();
        let s = state.lock().await;
        match s.db_path.clone() {
            Some(p) => p,
            None => return,
        }
    };

    // 2. Try loading a saved index (avoids re-embedding on every launch)
    let index_path = index_file_path(&app);
    if index_path.exists() {
        if let Ok(loaded) = crate::index::SearchIndex::load(&index_path) {
            if let Ok(conn) = crate::db::open_joplin_db(&db_path) {
                if let Ok(notes) = crate::db::get_all_notes(&conn) {
                    let total = notes.len();
                    let max_ts = notes.iter().map(|n| n.updated_time).max().unwrap_or(0);
                    let note_cache: HashMap<String, NoteMetadata> =
                        notes.into_iter().map(|n| (n.id.clone(), NoteMetadata {
                            id: n.id,
                            title: n.title,
                            updated_time: n.updated_time,
                        })).collect();

                    let state = app.state::<AppMutex>();
                    let mut s = state.lock().await;
                    s.search_index = Some(Arc::new(tokio::sync::RwLock::new(loaded)));
                    s.note_cache = note_cache;
                    s.last_scan_timestamp = max_ts;
                    s.deleted_note_ids.clear();
                    s.last_full_rebuild_ts = std::time::Instant::now();
                    s.index_status = IndexStatus {
                        total_notes: total,
                        indexed_notes: total,
                        is_ready: false, // not yet — need the model for queries
                        is_downloading_model: true,
                        download_progress: 1.0,
                        error: None,
                    };
                    let _ = app.emit("index-status", &s.index_status);
                    drop(s);

                    // Init model in background — index was already loaded
                    ensure_pipeline_loaded(app.clone()).await;

                    let state = app.state::<AppMutex>();
                    let mut s = state.lock().await;
                    s.index_status.is_downloading_model = false;
                    if s.embedding_pipeline.is_some() {
                        s.index_status.is_ready = true;
                    }
                    let _ = app.emit("index-status", &s.index_status);
                    return;
                }
            }
        }
    }

    // 3. Full build: init model
    {
        let state = app.state::<AppMutex>();
        let mut s = state.lock().await;
        s.index_status.is_downloading_model = true;
        let _ = app.emit("index-status", &s.index_status);
    }

    ensure_pipeline_loaded(app.clone()).await;

    {
        let state = app.state::<AppMutex>();
        let mut s = state.lock().await;
        s.index_status.is_downloading_model = false;
        let _ = app.emit("index-status", &s.index_status);
    }

    // 4. Read all notes
    let notes = match crate::db::open_joplin_db(&db_path)
        .and_then(|conn| crate::db::get_all_notes(&conn))
    {
        Ok(n) => n,
        Err(e) => {
            let state = app.state::<AppMutex>();
            let mut s = state.lock().await;
            s.index_status.error = Some(format!("Failed to read database: {e}"));
            let _ = app.emit("index-status", &s.index_status);
            return;
        }
    };

    let total = notes.len();
    {
        let state = app.state::<AppMutex>();
        let mut s = state.lock().await;
        s.index_status.total_notes = total;
        let _ = app.emit("index-status", &s.index_status);
    }

    // 5. Embed in batches and build the HNSW index
    // Allocate 2× headroom so delta inserts don't hit capacity before the next full rebuild.
    let mut search_index = match crate::index::SearchIndex::new((total * 2).max(2000)) {
        Ok(i) => i,
        Err(e) => {
            let state = app.state::<AppMutex>();
            let mut s = state.lock().await;
            s.index_status.error = Some(format!("Failed to create index: {e}"));
            let _ = app.emit("index-status", &s.index_status);
            return;
        }
    };

    let mut note_cache: HashMap<String, NoteMetadata> = HashMap::new();
    let mut max_ts: i64 = 0;
    let mut indexed = 0;
    const BATCH: usize = 64;

    for chunk in notes.chunks(BATCH) {
        // Clone the Arc outside the lock so inference happens lock-free
        let texts_owned: Vec<String> = chunk
            .iter()
            .map(|n| format!("{}\n\n{}", n.title, n.body))
            .collect();
        let texts: Vec<&str> = texts_owned.iter().map(|s| s.as_str()).collect();
        let pipeline_arc = {
            let state = app.state::<AppMutex>();
            let guard = state.lock().await;
            guard.embedding_pipeline.clone()
        };
        let embeddings = pipeline_arc.and_then(|p| p.embed_batch(&texts).ok());

        if let Some(embeddings) = embeddings {
            let entries: Vec<(String, Vec<f32>)> = chunk
                .iter()
                .zip(embeddings)
                .filter(|(note, _)| is_valid_joplin_id(&note.id))
                .map(|(note, emb)| (note.id.clone(), emb))
                .collect();
            let _ = search_index.add_batch(entries);
        }

        for note in chunk {
            max_ts = max_ts.max(note.updated_time);
            note_cache.insert(note.id.clone(), NoteMetadata {
                id: note.id.clone(),
                title: note.title.clone(),
                updated_time: note.updated_time,
            });
        }

        indexed += chunk.len();
        let state = app.state::<AppMutex>();
        let mut s = state.lock().await;
        s.index_status.indexed_notes = indexed;
        s.index_status.download_progress = indexed as f32 / total.max(1) as f32;
        let _ = app.emit("index-status", &s.index_status);
    }

    // 6. Persist index to disk
    let _ = search_index.save(&index_path);

    // 7. Update state and mark ready (only if pipeline loaded successfully)
    let state = app.state::<AppMutex>();
    let mut s = state.lock().await;
    s.search_index = Some(Arc::new(tokio::sync::RwLock::new(search_index)));
    s.note_cache = note_cache;
    s.last_scan_timestamp = max_ts;
    s.deleted_note_ids.clear();
    s.last_full_rebuild_ts = std::time::Instant::now();
    if s.embedding_pipeline.is_some() {
        s.index_status.is_ready = true;
    }
    s.index_status.download_progress = 1.0;
    s.index_status.error = None;
    let _ = app.emit("index-status", &s.index_status);
}

/// Run a delta update: immediately handle new, edited, and deleted notes.
/// Schedules a background full rebuild if 5 minutes have passed since the last one.
pub async fn run_delta_update(app: tauri::AppHandle, db_path: String) {
    // Guard: prevent two overlapping delta passes from double-inserting embeddings.
    {
        let state = app.state::<AppMutex>();
        let mut s = state.lock().await;
        if s.is_delta_updating || s.is_indexing {
            return;
        }
        s.is_delta_updating = true;
    }

    run_delta_update_inner(app.clone(), db_path).await;

    let state = app.state::<AppMutex>();
    state.lock().await.is_delta_updating = false;
}

async fn run_delta_update_inner(app: tauri::AppHandle, db_path: String) {
    // 1. Grab last timestamps
    let (last_ts, last_rebuild_ts) = {
        let state = app.state::<AppMutex>();
        let s = state.lock().await;
        (s.last_scan_timestamp, s.last_full_rebuild_ts)
    };

    // 2. Cheap check: anything changed at all?
    let conn = match crate::db::open_joplin_db(&db_path) {
        Ok(c) => c,
        Err(_) => return,
    };
    match crate::db::has_notes_since(&conn, last_ts) {
        Ok(false) => return,
        Err(_) => return,
        Ok(true) => {}
    }

    // 3. Handle deleted notes — update tombstone set + remove from cache
    let deleted_ids = crate::db::get_deleted_note_ids_since(&conn, last_ts)
        .unwrap_or_default();
    if !deleted_ids.is_empty() {
        let state = app.state::<AppMutex>();
        let mut s = state.lock().await;
        for id in &deleted_ids {
            s.deleted_note_ids.insert(id.clone());
            s.note_cache.remove(id);
        }
    }

    // 4. Embed and insert new/edited notes into the live index
    let changed_notes = crate::db::get_notes_since(&conn, last_ts).unwrap_or_default();

    if !changed_notes.is_empty() {
        // Clone pipeline Arc outside the lock
        let pipeline_arc = {
            let state = app.state::<AppMutex>();
            let s = state.lock().await;
            s.embedding_pipeline.clone()
        };

        if let Some(pipeline) = pipeline_arc {
            let texts_owned: Vec<String> = changed_notes
                .iter()
                .map(|n| format!("{}\n\n{}", n.title, n.body))
                .collect();
            let texts: Vec<&str> = texts_owned.iter().map(|s| s.as_str()).collect();

            // Embed outside both locks
            if let Ok(embeddings) = pipeline.embed_batch(&texts) {
                let entries: Vec<(String, Vec<f32>)> = changed_notes
                    .iter()
                    .zip(embeddings)
                    .filter(|(note, _)| is_valid_joplin_id(&note.id))
                    .map(|(note, emb)| (note.id.clone(), emb))
                    .collect();

                // Clone the index Arc while holding AppState lock briefly
                let index_arc = {
                    let state = app.state::<AppMutex>();
                    let s = state.lock().await;
                    s.search_index.clone()
                };

                // Take write lock on index — held for add_batch only (~50ms)
                if let Some(arc) = index_arc {
                    let mut index = arc.write().await;
                    let _ = index.add_batch(entries);
                    drop(index); // release write lock before re-acquiring AppState
                }

                // Update note_cache and scan timestamp.
                // Also clear any tombstone entries for restored/edited notes —
                // a note that's back in get_notes_since is live again.
                let max_ts = changed_notes.iter().map(|n| n.updated_time).max().unwrap_or(0);
                let state = app.state::<AppMutex>();
                let mut s = state.lock().await;
                for note in &changed_notes {
                    s.deleted_note_ids.remove(&note.id); // un-tombstone if restored
                    s.note_cache.insert(note.id.clone(), NoteMetadata {
                        id: note.id.clone(),
                        title: note.title.clone(),
                        updated_time: note.updated_time,
                    });
                }
                // Subtract 1ms so that a note whose updated_time exactly equals
                // the boundary is re-checked on the next cycle (off-by-one fix).
                s.last_scan_timestamp = s.last_scan_timestamp.max(max_ts.saturating_sub(1));
                s.index_status.indexed_notes = s.note_cache.len();
                s.index_status.total_notes = s.note_cache.len();
                let _ = app.emit("index-status", &s.index_status);
                drop(s);

                // Persist the updated index so new notes survive a restart.
                let index_path = index_file_path(&app);
                let index_arc2 = {
                    let state = app.state::<AppMutex>();
                    let guard = state.lock().await;
                    guard.search_index.clone()
                };
                if let Some(arc) = index_arc2 {
                    let idx = arc.read().await;
                    let _ = idx.save(&index_path);
                }
            }
        }
    }

    // 5. Schedule background full rebuild if 5 minutes have elapsed
    const REBUILD_INTERVAL: std::time::Duration = std::time::Duration::from_secs(300);
    if last_rebuild_ts.elapsed() >= REBUILD_INTERVAL {
        tauri::async_runtime::spawn(async move {
            run_full_indexing(app).await;
        });
    }
}

/// Ensure the embedding pipeline is loaded (downloads model if needed).
/// Uses is_pipeline_loading flag to prevent concurrent duplicate downloads.
async fn ensure_pipeline_loaded(app: tauri::AppHandle) {
    {
        let state = app.state::<AppMutex>();
        let mut s = state.lock().await;
        if s.embedding_pipeline.is_some() || s.is_pipeline_loading {
            return;
        }
        s.is_pipeline_loading = true;
    }

    let cache_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from(".fastembed_cache"));
    let cache_dir_owned = cache_dir.to_path_buf();

    let pipeline = tokio::task::spawn_blocking(move || {
        crate::embeddings::EmbeddingPipeline::new(&cache_dir_owned, false)
    })
    .await;

    let state = app.state::<AppMutex>();
    let mut s = state.lock().await;
    s.is_pipeline_loading = false;
    match pipeline {
        Ok(Ok(p)) => {
            s.embedding_pipeline = Some(Arc::new(p));
        }
        _ => {
            s.index_status.error =
                Some("Failed to load embedding model".to_string());
            let _ = app.emit("index-status", &s.index_status);
        }
    }
}

/// Validate that a note ID is a 32-character lowercase hex string (Joplin UUID format).
fn is_valid_joplin_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Open a note in the Joplin desktop app via its URL protocol handler.
/// Works on Linux (xdg-open), Windows (ShellExecute), macOS (open).
#[tauri::command]
pub async fn open_in_joplin(note_id: String) -> Result<(), String> {
    if !is_valid_joplin_id(&note_id) {
        return Err("invalid_note_id".to_string());
    }
    let url = format!("joplin://x-callback-url/openNote?id={}", note_id);
    open::that_detached(url).map_err(|e| e.to_string())
}

/// Path where the HNSW index binary is persisted.
pub fn index_file_path(app: &tauri::AppHandle) -> std::path::PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("joplin-smart-search")
        .join("index.bin")
}

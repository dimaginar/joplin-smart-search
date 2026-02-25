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

/// Installs a `.desktop` entry and a 128x128 icon for this app.
/// Runs on every launch (best-effort, all errors silently ignored):
///   - Always writes the icon + refreshes the GTK icon cache (fixes missing icons).
///   - Rewrites the .desktop entry if the binary path has changed (handles moves/updates).
#[cfg(target_os = "linux")]
fn install_desktop_entry() {
    // Resolve the real path of the running binary.
    let exe_path = match std::fs::read_link("/proc/self/exe") {
        Ok(p) => p,
        Err(_) => return,
    };

    let home = match std::env::var("HOME") {
        Ok(h) => h,
        Err(_) => return,
    };

    let desktop_dir = format!("{}/.local/share/applications", home);
    // Filename must match the Wayland app-id Tauri broadcasts: "joplin-smart-search"
    // (tao derives this from the binary name, not the bundle identifier).
    let desktop_file = format!("{}/joplin-smart-search.desktop", desktop_dir);
    // Remove any stale file written under the old (wrong) name.
    let _ = std::fs::remove_file(format!("{}/io.joplin.smart-search.desktop", desktop_dir));
    let icon_dir = format!("{}/.local/share/icons/hicolor/128x128/apps", home);
    let hicolor_dir = format!("{}/.local/share/icons/hicolor", home);

    // Create required directories.
    let _ = std::fs::create_dir_all(&desktop_dir);
    let _ = std::fs::create_dir_all(&icon_dir);

    // Always write the icon — cheap and ensures the icon cache can be refreshed.
    let icon_bytes = include_bytes!("../icons/128x128.png");
    let icon_path = format!("{}/joplin-smart-search.png", icon_dir);
    let _ = std::fs::write(&icon_path, icon_bytes);

    // Always refresh the GTK icon cache so the DE can find our icon.
    // Without this, icons added to hicolor/ are invisible until the cache is rebuilt.
    let _ = std::process::Command::new("gtk-update-icon-cache")
        .args(["-f", "-t", &hicolor_dir])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();

    // Write or update the .desktop entry if the binary path has changed.
    let exe_str = exe_path.to_string_lossy();
    let needs_update = std::fs::read_to_string(&desktop_file)
        .map(|c| !c.contains(exe_str.as_ref()))
        .unwrap_or(true); // missing → write it

    if needs_update {
        // Use the absolute icon path so KDE/GNOME don't need a cache lookup.
        // Icon theme name lookup ("joplin-smart-search") requires kbuildsycoca/GTK
        // cache to be current; absolute path works regardless of cache state.
        let desktop_contents = format!(
            "[Desktop Entry]\n\
             Name=Joplin Smart Search\n\
             Exec={exe_str}\n\
             Icon={icon_path}\n\
             Type=Application\n\
             Categories=Utility;\n\
             StartupWMClass=joplin-smart-search\n"
        );
        let _ = std::fs::write(&desktop_file, desktop_contents);

        // Refresh the desktop database.
        let _ = std::process::Command::new("update-desktop-database")
            .arg(&desktop_dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }

    // Rebuild KDE's sycoca cache so it picks up the new icon (KDE ignores
    // gtk-update-icon-cache). Try KDE 6 first, fall back to KDE 5.
    for cmd in &["kbuildsycoca6", "kbuildsycoca5"] {
        if std::process::Command::new(cmd)
            .arg("--noincremental")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .is_ok()
        {
            break;
        }
    }
}

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
            commands::open_external_url,
        ])
        .setup(|app| {
            // Set the window icon explicitly so the taskbar shows our icon on Linux.
            if let Some(window) = app.get_webview_window("main") {
                if let Some(icon) = app.default_window_icon() {
                    let _ = window.set_icon(icon.clone()).ok();
                }
            }
            #[cfg(target_os = "linux")]
            install_desktop_entry();

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                commands::startup_init(handle).await;
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

use std::time::{Duration, SystemTime};
use tauri::Manager;

use crate::AppMutex;

/// How often to check if the Joplin SQLite file has changed.
const POLL_INTERVAL: Duration = Duration::from_secs(10);

/// Minimum quiet period after a change before triggering an update.
/// Joplin may write to SQLite multiple times during a save; we debounce.
const DEBOUNCE: Duration = Duration::from_secs(5);

/// Start the background file watcher. Polls the Joplin SQLite file for
/// modifications and triggers an incremental index update when changes detected.
pub async fn start_watcher(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        watch_loop(app).await;
    });
}

async fn watch_loop(app: tauri::AppHandle) {
    let mut last_modified: Option<SystemTime> = None;
    let mut pending_since: Option<SystemTime> = None;

    loop {
        tokio::time::sleep(POLL_INTERVAL).await;

        let db_path = {
            let state = app.state::<AppMutex>();
            let x = state.lock().await.db_path.clone();
            x
        };

        let db_path = match db_path {
            Some(p) => p,
            None => continue, // no DB configured yet
        };

        // Check modification time of the main DB file AND the WAL file.
        // Joplin uses SQLite WAL mode: writes go to database.sqlite-wal first
        // and the main file's mtime only changes after a WAL checkpoint.
        // Watching only the main file misses most note saves.
        let mtime_main = std::fs::metadata(&db_path).and_then(|m| m.modified()).ok();
        let mtime_wal  = std::fs::metadata(format!("{}-wal", &db_path)).and_then(|m| m.modified()).ok();
        let current_mtime = match (mtime_main, mtime_wal) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b)             => a.or(b),
        };

        let changed = match (last_modified, current_mtime) {
            (None, Some(t)) => {
                last_modified = Some(t);
                false // first observation — don't treat as a change
            }
            (Some(prev), Some(cur)) if cur != prev => {
                last_modified = Some(cur);
                true
            }
            _ => false,
        };

        if changed {
            // Start (or reset) the debounce timer
            pending_since = Some(SystemTime::now());
        }

        // Fire update if we've been waiting long enough
        if let Some(since) = pending_since {
            if since.elapsed().unwrap_or_default() >= DEBOUNCE {
                pending_since = None;
                crate::commands::run_delta_update(app.clone(), db_path).await;
            }
        }
    }
}

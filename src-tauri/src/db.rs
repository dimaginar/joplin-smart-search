use anyhow::Result;
use rusqlite::Connection;
use std::path::PathBuf;

use crate::types::Note;

/// Auto-detect the Joplin SQLite database path.
/// Falls back to None if not found — caller should prompt user to browse.
pub fn detect_joplin_db_path() -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        if let Some(home) = std::env::var("HOME").ok() {
            let path = PathBuf::from(&home).join(".config/joplin-desktop/database.sqlite");
            if path.exists() {
                return Some(path);
            }
            let path = PathBuf::from(&home).join(".config/joplin/database.sqlite");
            if path.exists() {
                return Some(path);
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Standard Joplin Desktop install: %USERPROFILE%\.config\joplin-desktop\database.sqlite
        if let Some(profile) = std::env::var("USERPROFILE").ok() {
            let path = PathBuf::from(&profile).join(".config").join("joplin-desktop").join("database.sqlite");
            if path.exists() {
                return Some(path);
            }
        }
        // Fallback: older/portable Joplin: %APPDATA%\Joplin\database.sqlite
        if let Some(app_data) = std::env::var("APPDATA").ok() {
            let path = PathBuf::from(&app_data).join("Joplin").join("database.sqlite");
            if path.exists() {
                return Some(path);
            }
        }
    }

    None
}

/// Open the Joplin SQLite database in read-only mode.
/// WAL must be set before query_only — journal_mode writes a flag; query_only
/// blocks all writes including pragma writes, so the order matters.
pub fn open_joplin_db(path: &str) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA query_only = ON;
         PRAGMA busy_timeout = 5000;",
    )?;
    Ok(conn)
}

/// Fetch all non-conflict notes. Used for initial index build.
pub fn get_all_notes(conn: &Connection) -> Result<Vec<Note>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, body, updated_time
         FROM notes
         WHERE is_conflict = 0
           AND deleted_time = 0
           AND trim(body) != ''
         ORDER BY updated_time DESC",
    )?;

    let notes = stmt
        .query_map([], |row| {
            Ok(Note {
                id: row.get::<_, String>(0)?,
                title: row.get::<_, String>(1).unwrap_or_default(),
                body: row.get::<_, String>(2).unwrap_or_default(),
                updated_time: row.get::<_, i64>(3)?,
            })
        })?
        .filter_map(|r| {
            r.map_err(|e| tracing::warn!("Skipping malformed row: {e}"))
             .ok()
        })
        .filter(|n| !n.body.trim().is_empty())
        .collect();

    Ok(notes)
}

/// Fetch a single note by ID (including body). Returns None if not found.
pub fn get_note_by_id(conn: &Connection, id: &str) -> Result<Option<Note>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, body, updated_time
         FROM notes
         WHERE id = ?1
           AND is_conflict = 0
           AND deleted_time = 0",
    )?;
    let mut rows = stmt.query_map([id], |row| {
        Ok(Note {
            id: row.get::<_, String>(0)?,
            title: row.get::<_, String>(1).unwrap_or_default(),
            body: row.get::<_, String>(2).unwrap_or_default(),
            updated_time: row.get::<_, i64>(3)?,
        })
    })?;
    Ok(rows.next().transpose()?)
}

/// Cheaply check whether any notes have changed or been deleted since `since_ms`.
/// Used by the watcher before committing to a full re-embed.
pub fn has_notes_since(conn: &Connection, since_ms: i64) -> Result<bool> {
    let changed: i64 = conn.query_row(
        "SELECT COUNT(*) FROM notes
         WHERE is_conflict = 0
           AND deleted_time = 0
           AND updated_time > ?1",
        [since_ms],
        |row| row.get(0),
    )?;
    if changed > 0 {
        return Ok(true);
    }
    let deleted: i64 = conn.query_row(
        "SELECT COUNT(*) FROM notes
         WHERE is_conflict = 0
           AND deleted_time > ?1",
        [since_ms],
        |row| row.get(0),
    )?;
    Ok(deleted > 0)
}

/// Fetch IDs of notes soft-deleted after `since_ms`.
/// Joplin sets deleted_time to a non-zero Unix ms timestamp on soft-delete.
pub fn get_deleted_note_ids_since(conn: &Connection, since_ms: i64) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT id
         FROM notes
         WHERE is_conflict = 0
           AND deleted_time > ?1",
    )?;
    let ids = stmt
        .query_map([since_ms], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(ids)
}

/// Fetch only notes updated after `since_ms` (Unix ms timestamp).
/// Used by the delta update path to embed only changed notes.
pub fn get_notes_since(conn: &Connection, since_ms: i64) -> Result<Vec<Note>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, body, updated_time
         FROM notes
         WHERE is_conflict = 0
           AND deleted_time = 0
           AND trim(body) != ''
           AND updated_time > ?1
         ORDER BY updated_time DESC",
    )?;

    let notes = stmt
        .query_map([since_ms], |row| {
            Ok(Note {
                id: row.get::<_, String>(0)?,
                title: row.get::<_, String>(1).unwrap_or_default(),
                body: row.get::<_, String>(2).unwrap_or_default(),
                updated_time: row.get::<_, i64>(3)?,
            })
        })?
        .filter_map(|r| {
            r.map_err(|e| tracing::warn!("Skipping malformed row: {e}"))
             .ok()
        })
        .filter(|n| !n.body.trim().is_empty())
        .collect();

    Ok(notes)
}

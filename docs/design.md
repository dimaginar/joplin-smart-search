# Design Document — Joplin Smart Search

## Overview

Joplin Smart Search is a desktop application that adds semantic (concept-based) search to [Joplin](https://joplinapp.org/). It reads Joplin's local SQLite database, embeds all notes using a local ONNX model, stores the embeddings in an HNSW vector index, and serves a search UI that returns ranked results for any typed concept or idea.

The application is fully local and offline after first run. No data leaves the machine.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  [ search input (full width) ]          [index status]      │
├────────────┬────────────────────────────────────────────────┤
│ note title │                                                 │
│ 0.92       │   Note preview (rendered markdown)             │
│────────────│   links, headings, code blocks, tables         │
│ note title │                                                 │
│ 0.87       │                                                 │
│────────────│                                                 │
│ ...        │                        [ Open in Joplin ]      │
└────────────┴────────────────────────────────────────────────┘
  280px sidebar        detail panel
```

### Layers

| Layer | Technology |
|---|---|
| Shell | Tauri 2 |
| Backend | Rust |
| Frontend | React 19 + TypeScript |
| State | Zustand |
| Styling | Tailwind CSS 4 |
| Embeddings | fastembed 4 — bge-small-en-v1.5 via ONNX Runtime |
| Vector index | ruvector-core 2.x (HNSW) |
| Data source | Joplin local SQLite (read-only) |

---

## Key Design Decisions

### Read-only access to Joplin data
The app never writes to the Joplin SQLite database. It opens it in read-only mode. This is a hard constraint — the app cannot corrupt or modify any user data.

### Local embedding model
The [bge-small-en-v1.5](https://huggingface.co/BAAI/bge-small-en-v1.5) model (~33 MB) is downloaded from HuggingFace on first run and cached in `app_data_dir`. All subsequent runs are fully offline. No Python, no GPU required — inference runs via ONNX Runtime bundled in the `fastembed` crate.

### HNSW vector index via ruvector
[ruvector](https://github.com/ruvnet/ruvector) provides the HNSW (Hierarchical Navigable Small World) index — the core of the search engine. Approximate nearest-neighbour search over the embedding space returns ranked results in milliseconds even for thousands of notes. The index is persisted to `index.bin` in `app_data_dir` and loaded on startup.

### Delta indexing (incremental updates)
Rather than rebuilding the full index on every change, the app uses delta updates:
- A file watcher monitors both `database.sqlite` and `database.sqlite-wal` (Joplin uses WAL mode)
- On change, only notes newer than `last_scan_timestamp` are fetched and embedded
- New embeddings are inserted into the existing HNSW index
- Deleted notes are tracked in a tombstone set and filtered from results

HNSW does not support deletion, so duplicate nodes accumulate on trash/restore cycles. Search results are deduplicated by note ID to handle this.

### Arc-based lock-free inference
The embedding model (`Arc<EmbeddingPipeline>`) and index (`Arc<RwLock<SearchIndex>>`) are cloned out of the main `AppMutex` before use, so ML inference runs entirely outside the mutex lock. This prevents search queries from blocking indexing and vice versa. The `EmbeddingPipeline` uses an internal `Mutex<TextEmbedding>` to serialize ONNX inference calls, preventing heap corruption from concurrent session use.

### Atomic index persistence
The index is saved via a temp file + rename (`index.bin.tmp` → `index.bin`) to prevent corruption if the app is killed mid-write.

---

## Data Flow

### Startup
1. Load `index.bin` from disk (if exists)
2. Detect Joplin SQLite path (auto or user-provided)
3. Load embedding model (download if first run)
4. Run delta update to catch notes added while app was closed
5. Start file watcher

### Search
1. User types query
2. Frontend debounces and calls `search_notes` Tauri command
3. Rust embeds the query string using the ONNX model
4. HNSW nearest-neighbour search returns top-K candidates
5. Results are filtered against the tombstone set (deleted notes)
6. Deduplicated by note ID (handles HNSW duplicates)
7. Ranked by similarity score, returned to frontend

### Delta update (triggered by file watcher or user)
1. Query SQLite for notes with `updated_time > last_scan_timestamp`
2. Embed each note (`title + "\n\n" + body`)
3. Insert embeddings into HNSW index
4. Update `last_scan_timestamp`
5. Save index to disk

### Note preview
1. User clicks a search result
2. Frontend calls `get_note` to fetch full note body from SQLite
3. `renderMarkdown` transforms Joplin-specific link syntax and renders markdown to sanitized HTML
4. Internal note links (`:/note-id`) navigate within the app
5. External links open in the system browser

---

## File Structure

```
src-tauri/src/
  lib.rs          — AppState, Tauri builder setup, .desktop auto-install (Linux)
  commands.rs     — all Tauri commands, indexing logic, delta update
  db.rs           — SQLite queries (read-only)
  embeddings.rs   — fastembed pipeline wrapper
  index.rs        — HNSW index wrapper, atomic persistence
  types.rs        — shared types: Note, NoteMetadata, SearchResult, IndexStatus
  watcher.rs      — file watcher, WAL-aware mtime detection

src/
  App.tsx                    — root component, routing logic
  store.ts                   — Zustand store
  types.ts                   — TypeScript types (mirror of Rust types)
  lib/renderMarkdown.ts      — markdown → sanitized HTML pipeline
  components/
    ResultsList.tsx           — search result sidebar
    DetailPanel.tsx           — note preview with rendered markdown
```

---

## State Management

### Rust — AppState (behind tokio::sync::Mutex)

| Field | Purpose |
|---|---|
| `db_path` | Path to Joplin SQLite |
| `embedding_pipeline` | `Arc<EmbeddingPipeline>` — ONNX model |
| `search_index` | `Arc<RwLock<SearchIndex>>` — HNSW index |
| `note_cache` | `HashMap<id, NoteMetadata>` — in-memory title/timestamp cache |
| `last_scan_timestamp` | Unix ms of last indexed note |
| `deleted_note_ids` | Tombstone set for soft-deleted notes |
| `index_status` | Reported to frontend: total/indexed counts, errors, progress |
| `is_indexing` | Guard against concurrent full rebuilds |
| `is_pipeline_loading` | Guard against duplicate model downloads |
| `is_delta_updating` | Guard against overlapping delta passes |

### Frontend — Zustand store
Holds search query, results list, selected note, and index status polled from the backend.

---

## Distribution

| Artifact | Platform | Notes |
|---|---|---|
| AppImage | Linux (Ubuntu, Fedora, …) | Self-contained, bundles WebKit |
| tar.gz (raw binary) | Linux (Arch, CachyOS, …) | Uses system webkit2gtk-4.1 |
| NSIS installer (.exe) | Windows | Uses system WebView2 (built into Win 10/11) |

### Linux desktop integration
On first run the app auto-installs:
- `~/.local/share/icons/hicolor/128x128/apps/joplin-smart-search.png`
- `~/.local/share/applications/joplin-smart-search.desktop`

This gives the correct taskbar icon and app launcher entry without any user action. Subsequent launches skip this (idempotent check).

### Windows cross-compile
NSIS installer built locally from Linux — see [building-windows.md](building-windows.md). Upload the resulting `.exe` to the draft GitHub Release manually before publishing.

### CI/CD
GitHub Actions (`ubuntu-24.04`) builds Linux artifacts on tag push (attached to a draft GitHub Release) or manual dispatch (uploaded as private workflow artifacts, 7-day retention). Windows is built locally.

---

## Security

- **CSP** configured in `tauri.conf.json` — restricts script, style, image, and connect sources
- **Read-only DB access** — SQLite opened without write flags
- **DOMPurify** — all markdown rendered to HTML is sanitized with an explicit allowlist before `dangerouslySetInnerHTML`
- **URL scheme allowlist** — only `joplin-note://`, `http://`, and `https://` are permitted in rendered link `href` attributes
- **`open_external_url` guard** — rejects any URL that is not `http://` or `https://`

See [security-audit.md](security-audit.md) for dependency vulnerability status.

---

## Known Limitations

| Limitation | Notes |
|---|---|
| HNSW has no delete | Deleted notes accumulate as dead nodes; tombstone set + deduplication handles this at query time |
| Images in notes | Not rendered — deferred to a future phase (requires `joplin-resource://` URI scheme) |
| Encrypted notes | Encrypted note bodies cannot be indexed |
| Math / Mermaid | KaTeX and Mermaid diagrams not rendered — deferred |
| Windows code signing | No certificate; SmartScreen warning on first run |

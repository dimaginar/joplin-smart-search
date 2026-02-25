# Joplin Smart Search — Design Document

## 1. Overview

Joplin Smart Search is a desktop companion application for [Joplin](https://joplinapp.org/) that adds semantic (concept-based) search to a local note library. The stock Joplin search is keyword-only; this application embeds every note as a dense vector and uses approximate nearest-neighbor lookup to find notes that are conceptually related to a query, even when no exact keywords match.

**Key properties:**

- Fully local and offline after the first run. The embedding model is downloaded once from HuggingFace (~33 MB) and cached permanently.
- Read-only. The Joplin SQLite database is never modified.
- No Python, no GPU, no cloud service. The ONNX runtime runs natively inside the Rust process.
- Cross-platform. Targets Linux (AppImage) and Windows (.exe/.msi).

---

## 2. Architecture

### 2.1 Component diagram

```
  ┌──────────────────────────────────────────────────────────────┐
  │  Joplin desktop app                                          │
  │  ~/.config/joplin-desktop/database.sqlite  (read-only)      │
  └────────────────────┬─────────────────────────────────────────┘
                       │ rusqlite (WAL + query_only)
                       ▼
  ┌──────────────── db.rs ───────────────────────────────────────┐
  │  detect_joplin_db_path()   open_joplin_db()                 │
  │  get_all_notes()           get_note_by_id()                  │
  │  has_notes_since()         get_notes_since()                 │
  └────────────────────┬─────────────────────────────────────────┘
                       │ Vec<Note>  (id, title, body, updated_time)
                       ▼
  ┌──────────────── embeddings.rs ───────────────────────────────┐
  │  EmbeddingPipeline  (fastembed 4 / bge-small-en-v1.5)       │
  │  embed_one(text) → Vec<f32, 384>                             │
  │  embed_batch(texts) → Vec<Vec<f32, 384>>                     │
  │  Vectors are L2-normalised; cosine sim == dot product        │
  └────────────────────┬─────────────────────────────────────────┘
                       │ 384-dim f32 vectors
                       ▼
  ┌──────────────── index.rs ────────────────────────────────────┐
  │  SearchIndex  (ruvector-core 2.0.4 / HNSW)                  │
  │  add_batch([(note_id, embedding)])                           │
  │  search(query_vec, top_k=25) → Vec<IndexResult>             │
  │  save(path)  /  load(path)   (atomic temp+rename)           │
  └────────────────────┬─────────────────────────────────────────┘
                       │  IndexResult {note_id, score}
                       ▼
  ┌──────────────── commands.rs ─────────────────────────────────┐
  │  Tauri IPC commands                                          │
  │  startup_init → run_full_indexing → start_watcher           │
  │  search_notes   get_index_status   get_note   open_in_joplin│
  │  trigger_reindex   detect_db_path   set_joplin_db_path      │
  │                                                              │
  │  AppState (lib.rs)                                           │
  │  ├── db_path: Option<String>                                 │
  │  ├── embedding_pipeline: Option<Arc<EmbeddingPipeline>>     │
  │  ├── search_index: Option<Arc<SearchIndex>>                  │
  │  ├── note_cache: HashMap<uuid, NoteMetadata>                 │
  │  ├── last_scan_timestamp: i64                                │
  │  ├── index_status: IndexStatus                               │
  │  ├── is_indexing: bool                                       │
  │  └── is_pipeline_loading: bool                               │
  └────────────────────┬─────────────────────────────────────────┘
                       │ Tauri IPC (JSON serialisation)
                       ▼
  ┌──────────────── React frontend ──────────────────────────────┐
  │  Zustand store  (dbPath, indexStatus, results, selectedNote) │
  │                                                              │
  │  App.tsx ── routes to SetupScreen / IndexingScreen /         │
  │             MainLayout based on state                        │
  │                                                              │
  │  MainLayout                                                  │
  │  ├── SearchBar   — debounced invoke('search_notes')          │
  │  ├── ResultsList — invoke('get_note') on click               │
  │  ├── DetailPanel — renders body; invoke('open_in_joplin')    │
  │  └── StatusIndicator — reads indexStatus from store          │
  └──────────────────────────────────────────────────────────────┘
```

### 2.2 File watcher loop

```
  watcher.rs::start_watcher()
       │
       └─► tokio::spawn  watch_loop()
                │
                ├── sleep(10 s)
                ├── stat(db_path).modified()
                ├── mtime changed?
                │       yes → pending_since = now()
                │
                └── pending_since.elapsed() >= 30 s?
                        yes → run_delta_update(app, db_path)
                                  │
                                  ├── has_notes_since(last_scan_timestamp)?
                                  │       no  → return (no-op)
                                  │       yes → run_full_indexing(app)
                                  └── (updates last_scan_timestamp on completion)
```

The 30-second debounce absorbs the burst of writes that Joplin makes to SQLite during a note save.

---

## 3. Key Design Decisions

| Decision | Reason |
|---|---|
| `fastembed 4` instead of `ruvector-onnx-embeddings` | `ruvector-onnx-embeddings 0.1.0` fails to compile: it accesses `session.inputs` as a public struct field, but `ort 2.0.0-rc.9` changed it to a method. `fastembed` uses the same `ort` crate with the correct API. |
| BGE-small-EN-v1.5 as embedding model | 384-dimensional vectors, ~33 MB download, strong MTEB scores for retrieval at this size class. Smaller than MiniLM-L6-v2 with comparable English retrieval quality. |
| HNSW via `ruvector-core` | Approximate nearest-neighbor search with sub-linear query time and configurable recall/speed tradeoff (`ef_construction=200`, `ef_search=50`, `m=16`). Avoids a full linear scan over potentially thousands of embeddings on each keystroke. |
| Read-only SQLite access | `PRAGMA query_only = ON` is set after `journal_mode = WAL`. The WAL pragma must come first because `query_only` blocks all writes including pragma writes. This guarantees the Joplin database is never modified under any code path. |
| `Arc<EmbeddingPipeline>` and `Arc<SearchIndex>` | Both objects are placed behind `Arc` so that a clone of the pointer can be extracted while holding the `Mutex<AppState>` lock, and the lock can then be released before the expensive ONNX inference or HNSW search runs. This prevents the mutex from being held for hundreds of milliseconds per query. |
| Atomic index save (temp + rename) | The HNSW index is serialised to a `.bin.tmp` sibling file and then `rename`d over the target. `rename` is atomic on POSIX filesystems. A crash mid-write leaves the old valid index intact rather than a truncated file. |
| `NoteMetadata` cache vs full `Note` on demand | The in-memory `note_cache` stores only `{id, title, updated_time}` — no body. Holding every note body in RAM would be impractical for large libraries. The body is fetched from SQLite with a single-row query (`get_note_by_id`) only when the user selects a result. |
| `MIN_SCORE = 0.30` threshold | Cosine similarity below 0.30 on normalised BGE vectors is almost always noise. Filtering these results out prevents visually confusing low-relevance matches from appearing in the sidebar. |
| Double body whitespace filter (SQLite + Rust) | SQLite's `trim()` does not strip Unicode whitespace (e.g. non-breaking spaces). A second `.trim().is_empty()` guard in Rust catches notes that pass the SQL filter but contain only whitespace characters outside the ASCII range. |
| Model downloaded, not bundled | The ONNX Runtime pre-built binaries are fetched at compile time via `ort-download-binaries`. The model weights (~33 MB) are fetched at first runtime launch and cached in `app_data_dir()`. Bundling the model into the binary is unnecessary for MVP and inflates the download size. |
| `cargo-xwin` for Windows cross-compilation | The Tauri Windows build requires MSVC headers (`windows.h` etc.). `cargo-xwin` provides the MSVC toolchain on Linux without running a Windows VM, enabling a single Linux CI host to produce both Linux and Windows artifacts. |

---

## 4. Data Flows

### 4.1 Startup and full index build

```
1. run()  (lib.rs)
   └── tauri::Builder::setup spawns startup_init(app)

2. startup_init
   ├── detect_joplin_db_path()
   │     Linux:   $HOME/.config/joplin-desktop/database.sqlite
   │     Windows: %USERPROFILE%\.config\joplin-desktop\database.sqlite
   │              %APPDATA%\Joplin\database.sqlite  (fallback)
   │     → stores path in AppState.db_path
   ├── run_full_indexing(app)  [see below]
   └── start_watcher(app)

3. run_full_indexing
   ├── Guard: if is_indexing == true, return immediately
   ├── Set is_indexing = true

4. run_full_indexing_inner
   ├── Check if index.bin exists on disk
   │     yes → load SearchIndex from disk
   │           query all notes for metadata + max updated_time
   │           populate note_cache
   │           call ensure_pipeline_loaded (load/download ONNX model)
   │           set is_ready = true  (if pipeline loaded)
   │           emit "index-status"
   │           return  [fast path — no re-embedding]
   │
   │     no  → continue to full build:
   │
   ├── emit index-status { is_downloading_model: true }
   ├── ensure_pipeline_loaded
   │     ├── Guard: if pipeline already loaded or is_pipeline_loading, return
   │     ├── Set is_pipeline_loading = true
   │     ├── spawn_blocking → EmbeddingPipeline::new(app_data_dir, false)
   │     │     fastembed downloads BGE-small-EN-v1.5 from HuggingFace if not cached
   │     └── Store Arc<EmbeddingPipeline> in AppState  (or set error on failure)
   │
   ├── emit index-status { is_downloading_model: false }
   ├── open_joplin_db (WAL + query_only + busy_timeout=5000ms)
   ├── get_all_notes → Vec<Note>  (excludes conflicts, deleted, empty-body)
   ├── Create SearchIndex::new(max_elements)
   │     HNSW config: m=16, ef_construction=200, ef_search=50
   │
   ├── For each batch of 64 notes:
   │     ├── Build texts: "{title}\n\n{body}"  (ensures title-only notes embed well)
   │     ├── Clone Arc<EmbeddingPipeline> from AppState (releases lock)
   │     ├── pipeline.embed_batch(texts) → Vec<Vec<f32>>  [lock-free]
   │     ├── Filter out notes with invalid Joplin IDs (not 32-char lowercase hex)
   │     ├── search_index.add_batch([(id, embedding)])
   │     ├── Populate note_cache entry for each note
   │     └── emit index-status { indexed_notes, download_progress }
   │
   ├── search_index.save(app_data_dir/joplin-smart-search/index.bin)
   │     (atomic: write .bin.tmp then rename)
   └── Update AppState:
         search_index = Some(Arc::new(search_index))
         note_cache   = populated map
         last_scan_timestamp = max(updated_time across all notes)
         is_ready = true  (only if embedding_pipeline.is_some())
         emit "index-status"
```

### 4.2 Search query

```
1. User types in SearchBar
   └── debounce 350 ms

2. invoke('search_notes', { query })  →  commands::search_notes

3. Acquire AppMutex lock
   ├── Check is_ready; return Err("index_not_ready") if false
   ├── Clone Arc<EmbeddingPipeline>  (cheap pointer copy)
   ├── Clone Arc<SearchIndex>        (cheap pointer copy)
   └── Clone note_cache snapshot     (HashMap clone)
   → Release lock

4. pipeline.embed_one(query)
   └── ONNX inference  [runs outside the mutex lock]
   → query_embedding: Vec<f32, 384>  (L2-normalised)

5. index.search(query_embedding, top_k=25)
   └── HNSW ANN search  [runs outside the mutex lock]
   → Vec<IndexResult { note_id, cosine_distance }>
   → Convert distance to similarity: score = 1.0 - distance

6. Filter:
   ├── Look up note_id in note_cache snapshot
   └── Keep only results where score >= 0.30

7. Return Vec<SearchResult { note: NoteMetadata, score }>

8. Frontend receives results
   └── ResultsList renders title + "XX% match" for each result

9. User clicks a result
   └── invoke('get_note', { id })
         → open_joplin_db (fresh connection)
         → get_note_by_id (single-row query)
         → return Note { id, title, body, updated_time }
   └── DetailPanel renders note body

10. User clicks "Open in Joplin"
    └── invoke('open_in_joplin', { noteId })
          → validate 32-char hex ID
          → open::that_detached("joplin://x-callback-url/openNote?id=<id>")
             (xdg-open on Linux, ShellExecute on Windows)
```

---

## 5. IndexStatus State Machine

```
                    ┌─────────────────┐
    app launch ────►│  uninitialized  │
                    │  is_ready=false │
                    │  total_notes=0  │
                    └────────┬────────┘
                             │ startup_init finds DB
                             │ (or user sets path manually)
                             ▼
                    ┌─────────────────────┐
                    │  downloading_model  │
                    │  is_downloading_    │
                    │    model=true       │
                    └────────┬────────────┘
                             │ EmbeddingPipeline::new() succeeds
                             ▼
                    ┌─────────────────────┐
                    │     indexing        │
                    │  is_ready=false     │
                    │  indexed_notes      │
                    │    increments       │
                    └────────┬────────────┘
                             │ all notes embedded + index saved
                             ▼
                    ┌─────────────────────┐
                    │       ready         │◄──── watcher triggers
                    │  is_ready=true      │      rebuild (delta update)
                    │  total_notes=N      │      → transits back through
                    └─────────────────────┘        downloading_model/indexing
                                                    then back to ready

    Any step can transition to:
                    ┌─────────────────────┐
                    │       error         │
                    │  is_ready=false     │
                    │  error=Some(msg)    │
                    └─────────────────────┘

    Error causes:
    - DB open failed (file missing, permissions, corrupt)
    - EmbeddingPipeline::new() failed (network unavailable, disk full)
    - HNSW index creation failed
    - Index serialisation failed
```

The frontend uses `is_ready` as its primary gate. The route in `App.tsx` renders `IndexingScreen` for any non-ready state and `MainLayout` only when `is_ready = true`. `IndexingScreen` inspects `is_downloading_model`, `indexed_notes/total_notes`, and `error` to show the appropriate progress UI or error message.

---

## 6. Frontend Components

| File | Role |
|---|---|
| `App.tsx` | Root component. On mount, calls `detect_db_path` and `get_index_status`, then subscribes to the `index-status` Tauri event for live updates. Routes to `SetupScreen`, `IndexingScreen`, or `MainLayout` based on `dbPath` and `indexStatus.is_ready`. |
| `store.ts` | Single Zustand store. Holds `dbPath`, `indexStatus`, `results`, `selectedNote`, and `query`. All components read and write through this store; there is no prop drilling. |
| `SetupScreen.tsx` | Shown when no Joplin database was auto-detected. Provides a file-picker button (via `tauri-plugin-dialog`) that calls `set_joplin_db_path` and stores the chosen path. |
| `IndexingScreen.tsx` | Full-screen progress view shown while `is_ready = false`. Displays a `<progress>` bar and label for the two phases: model download and note embedding. Shows an error message if `indexStatus.error` is set. |
| `MainLayout.tsx` | The three-panel shell rendered when the index is ready. Composes `SearchBar`, `ResultsList`, `DetailPanel`, and `StatusIndicator`. Also exposes a "Refresh" button that calls `trigger_reindex`. |
| `SearchBar.tsx` | Controlled text input with a 350 ms debounce and a request-ID guard to discard stale responses from superseded queries. Calls `search_notes` via Tauri IPC. Silently ignores `index_not_ready` and `model_not_loaded` errors. |
| `ResultsList.tsx` | Renders the sidebar list of `SearchResult` items. Each row shows the note title and similarity score as a percentage. On click, calls `get_note` to fetch the full note body. Selected row receives a left accent bar. |
| `DetailPanel.tsx` | Renders the full note title and body. Validates the note ID client-side before enabling the "Open in Joplin" button. Calls `open_in_joplin` which invokes the `joplin://` URL protocol handler. |
| `StatusIndicator.tsx` | Small status pill in the sidebar footer. Shows a coloured dot (green/yellow/red) and a label derived from `indexStatus`. |

### Type mirroring

`src/types.ts` mirrors the Rust types in `src-tauri/src/types.rs`. Both must be kept in sync manually when the Tauri command signatures change.

| Rust (`types.rs`) | TypeScript (`types.ts`) |
|---|---|
| `Note` | `Note` |
| `NoteMetadata` | `NoteMetadata` |
| `SearchResult` | `SearchResult` |
| `IndexStatus` | `IndexStatus` |

---

## 7. Build and Distribution

### Development

```bash
# Install Rust, Node, and Tauri CLI, then:
npm install
npm run tauri dev
```

### Linux (AppImage)

```bash
npm run tauri build
# Output: src-tauri/target/release/bundle/appimage/joplin-smart-search_*.AppImage
```

AppImage bundles the Tauri WebView (WKWebView/WebKitGTK) and all Rust dependencies. The ONNX Runtime native library is linked statically via `ort-download-binaries` at compile time.

**FUSE2 requirement.** Running an AppImage requires `libfuse2`. On Ubuntu 22.04+ and Fedora 36+, FUSE2 is not installed by default. GitHub Actions Ubuntu runners lack it; the workaround in CI is to extract the AppImage with `--appimage-extract` and run the extracted binary directly, or to install `libfuse2` before the build step.

### Windows (NSIS installer)

Windows targets are cross-compiled from Linux using `cargo-xwin`, which downloads the MSVC CRT headers and import libraries automatically:

```bash
cargo install cargo-xwin
npm run tauri build -- --runner cargo-xwin --target x86_64-pc-windows-msvc
```

The NSIS installer script (`src-tauri/bundle/nsis/`) is invoked via `makensis`. This produces a standalone `.exe` installer that registers the `joplin://` URL protocol handler on the host machine (required for `open_in_joplin` to work).

### Runtime dependencies

| Dependency | Acquired at | Cached at |
|---|---|---|
| ONNX Runtime native library | Compile time (`ort-download-binaries`) | Linked into binary |
| BGE-small-EN-v1.5 ONNX weights | First launch (HuggingFace CDN) | `app_data_dir()` |
| HNSW index (`index.bin`) | After first full embed | `app_data_dir()/joplin-smart-search/` |

`app_data_dir()` resolves to:
- Linux: `~/.local/share/io.joplin.smart-search/`
- Windows: `%APPDATA%\io.joplin.smart-search\`

---

## 8. Known Limitations and Future Work

### Limitations

**No incremental HNSW update.** The HNSW graph does not support deletion or update of individual nodes. When the file watcher detects a change, `run_delta_update` performs a full rebuild rather than patching the graph. For libraries with thousands of notes this rebuild can take several seconds. The `get_notes_since` function in `db.rs` is already implemented for future incremental support but is currently unused.

**English-only model.** BGE-small-EN-v1.5 was trained on English text. Retrieval quality for non-English notes is poor. Multi-language support would require a multilingual model such as `paraphrase-multilingual-MiniLM-L12-v2`, which is larger (~120 MB).

**Empty-body notes excluded.** Notes with no body content are filtered out at the SQL layer. Title-only notes (Joplin task headers, stub notes) do not appear in search results even though the embed text includes the title. Relaxing this filter is a minor change but is out of scope for MVP.

**Single-window, no persistence of user preferences.** The DB path is not persisted between launches; auto-detection reruns on every startup. If auto-detection fails and the user browses to the path manually, they must do so again on the next launch.

**No fuzzy or keyword fallback.** The application performs only semantic search. There is no keyword search fallback for queries where exact term matching would be more appropriate (e.g. searching for a specific code snippet or a proper noun not in the embedding vocabulary).

### Potential future work

- Persist the detected DB path to `app_data_dir` so manual selection survives restarts.
- Replace full rebuild on change with a true incremental HNSW delete+insert when ruvector-core adds deletion support.
- Add a multilingual embedding model option (user-selectable in settings).
- Highlight matched concepts in the note body using the query embedding and token-level similarity.
- Expose search filters (by notebook, by tag, by date range) as pre-search SQL predicates.
- LLM-based answer synthesis over the top-K retrieved notes (out of scope for MVP per CLAUDE.md).

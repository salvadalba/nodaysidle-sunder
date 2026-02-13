# Technical Requirements Document

## 🧭 System Context
Sunder is a local-first, cross-platform desktop application built with Tauri 2. The Rust backend handles all data storage (SQLite with FTS5 and sqlite-vec extensions), embedding generation (quantized ONNX model via ort crate), and vector search. The frontend is a Vite 6 + React 19 + TypeScript SPA rendered in the Tauri webview, using Tailwind CSS 4 for glassmorphism styling. All communication flows through Tauri IPC commands and events. Zero network calls for core functionality. Single-user, single-machine deployment distributed as native installers per platform.

## 🔌 API Contracts
### create_note
- **Method:** tauri::command
- **Path:** invoke('create_note', { title, content, filePath? })
- **Auth:** none (local IPC)
- **Request:** { title: string, content: string, file_path: string | null }
- **Response:** { id: string, title: string, content: string, file_path: string | null, created_at: string, updated_at: string }
- **Errors:** NoteAlreadyExists: file_path conflicts with existing note, IoError: filesystem write failed, DbError: SQLite insert failed

### get_note
- **Method:** tauri::command
- **Path:** invoke('get_note', { id })
- **Auth:** none (local IPC)
- **Request:** { id: string }
- **Response:** { id: string, title: string, content: string, file_path: string | null, created_at: string, updated_at: string, word_count: number }
- **Errors:** NoteNotFound: no note with given id

### update_note
- **Method:** tauri::command
- **Path:** invoke('update_note', { id, title?, content? })
- **Auth:** none (local IPC)
- **Request:** { id: string, title: string | null, content: string | null }
- **Response:** { id: string, title: string, content: string, updated_at: string }
- **Errors:** NoteNotFound: no note with given id, DbError: SQLite update failed

### delete_note
- **Method:** tauri::command
- **Path:** invoke('delete_note', { id })
- **Auth:** none (local IPC)
- **Request:** { id: string }
- **Response:** { success: true }
- **Errors:** NoteNotFound: no note with given id, IoError: filesystem delete failed

### list_notes
- **Method:** tauri::command
- **Path:** invoke('list_notes', { offset?, limit?, sortBy? })
- **Auth:** none (local IPC)
- **Request:** { offset: number (default 0), limit: number (default 50), sort_by: 'updated_at' | 'created_at' | 'title' (default 'updated_at') }
- **Response:** { notes: Array<{ id: string, title: string, snippet: string, updated_at: string }>, total: number }
- **Errors:** DbError: SQLite query failed

### search_notes
- **Method:** tauri::command
- **Path:** invoke('search_notes', { query, mode?, limit? })
- **Auth:** none (local IPC)
- **Request:** { query: string, mode: 'hybrid' | 'fulltext' | 'semantic' (default 'hybrid'), limit: number (default 20) }
- **Response:** { results: Array<{ id: string, title: string, snippet: string, score: number, match_type: 'fulltext' | 'semantic' | 'both' }> }
- **Errors:** EmptyQuery: query string is blank, EmbeddingError: model inference failed, DbError: search query failed

### get_latent_links
- **Method:** tauri::command
- **Path:** invoke('get_latent_links', { content, threshold?, limit? })
- **Auth:** none (local IPC)
- **Request:** { content: string, threshold: number (default 0.65), limit: number (default 10) }
- **Response:** { links: Array<{ note_id: string, title: string, similarity: number, snippet: string }> }
- **Errors:** EmbeddingError: model inference failed, ContentTooShort: content under 20 characters

### get_graph_data
- **Method:** tauri::command
- **Path:** invoke('get_graph_data', { centerNoteId?, threshold? })
- **Auth:** none (local IPC)
- **Request:** { center_note_id: string | null, threshold: number (default 0.5) }
- **Response:** { nodes: Array<{ id: string, title: string, x: number, y: number, cluster: number }>, edges: Array<{ source: string, target: string, weight: number }> }
- **Errors:** DbError: graph query failed

### set_watch_directory
- **Method:** tauri::command
- **Path:** invoke('set_watch_directory', { path })
- **Auth:** none (local IPC)
- **Request:** { path: string }
- **Response:** { success: true, files_found: number }
- **Errors:** DirectoryNotFound: path does not exist, NotADirectory: path is a file, PermissionDenied: cannot read directory

### get_settings
- **Method:** tauri::command
- **Path:** invoke('get_settings')
- **Auth:** none (local IPC)
- **Request:** {}
- **Response:** { watch_directory: string | null, similarity_threshold: number, debounce_ms: number, theme: 'dark' | 'light' }
- **Errors:** DbError: settings read failed

### update_settings
- **Method:** tauri::command
- **Path:** invoke('update_settings', { settings })
- **Auth:** none (local IPC)
- **Request:** { similarity_threshold?: number, debounce_ms?: number, theme?: 'dark' | 'light' }
- **Response:** { success: true }
- **Errors:** InvalidValue: threshold out of range 0.0-1.0, DbError: settings write failed

### reindex_all
- **Method:** tauri::command
- **Path:** invoke('reindex_all')
- **Auth:** none (local IPC)
- **Request:** {}
- **Response:** { queued: number }
- **Errors:** AlreadyRunning: reindex job in progress

### event:latent_link_pulse
- **Method:** tauri::event (backend -> frontend)
- **Path:** listen('latent-link-pulse')
- **Auth:** none (local IPC)
- **Request:** Emitted by backend when typing content matches existing notes above threshold
- **Response:** { links: Array<{ note_id: string, title: string, similarity: number }> }

### event:indexing_progress
- **Method:** tauri::event (backend -> frontend)
- **Path:** listen('indexing-progress')
- **Auth:** none (local IPC)
- **Request:** Emitted by backend during bulk indexing operations
- **Response:** { processed: number, total: number, current_note_title: string }

### event:file_change
- **Method:** tauri::event (backend -> frontend)
- **Path:** listen('file-change')
- **Auth:** none (local IPC)
- **Request:** Emitted by backend when watched directory detects file changes
- **Response:** { event_type: 'created' | 'modified' | 'deleted', file_path: string, note_id: string | null }

## 🧱 Modules
### NoteService
- **Responsibilities:**
- CRUD operations on notes stored in SQLite notes table
- Parse markdown front matter for title extraction if no explicit title given
- Sync notes table with filesystem when file_path is set
- Maintain FTS5 virtual table in sync with notes table via triggers
- Generate UUIDv7 identifiers for new notes (time-sortable)
- Compute word count and snippet (first 200 chars stripped of markdown)
- **Interfaces:**
- fn create_note(title: String, content: String, file_path: Option<String>) -> Result<Note, SunderError>
- fn get_note(id: String) -> Result<Note, SunderError>
- fn update_note(id: String, title: Option<String>, content: Option<String>) -> Result<Note, SunderError>
- fn delete_note(id: String) -> Result<(), SunderError>
- fn list_notes(offset: u32, limit: u32, sort_by: SortField) -> Result<NoteList, SunderError>
- fn get_note_by_file_path(path: String) -> Result<Option<Note>, SunderError>
- **Dependencies:**
- DatabaseManager

### EmbeddingService
- **Responsibilities:**
- Load quantized ONNX all-MiniLM-L6-v2 model (~30MB) from sidecar assets at startup
- Tokenize text using included tokenizer.json (HuggingFace tokenizers crate)
- Generate 384-dimensional f32 embedding vectors from note content
- Chunk notes exceeding 512 tokens into overlapping segments (256 token overlap) and average embeddings
- Run inference on a dedicated tokio blocking thread pool (2 threads) to avoid blocking IPC
- Store embeddings in the embeddings table keyed by note_id
- Queue and process embedding jobs incrementally on note create/update
- **Interfaces:**
- fn embed_text(text: &str) -> Result<Vec<f32>, SunderError>
- fn index_note(note_id: &str, content: &str) -> Result<(), SunderError>
- fn remove_embedding(note_id: &str) -> Result<(), SunderError>
- fn reindex_all(progress_tx: Sender<IndexingProgress>) -> Result<u32, SunderError>
- **Dependencies:**
- DatabaseManager

### SearchService
- **Responsibilities:**
- Full-text search via FTS5 MATCH queries with BM25 ranking
- Semantic search via sqlite-vec cosine similarity on embedding vectors
- Hybrid search combining FTS5 and vector results using reciprocal rank fusion (k=60)
- Deduplicate results from both sources and merge scores
- Highlight matching snippets for fulltext results
- **Interfaces:**
- fn search(query: &str, mode: SearchMode, limit: u32) -> Result<Vec<SearchResult>, SunderError>
- fn fulltext_search(query: &str, limit: u32) -> Result<Vec<ScoredNote>, SunderError>
- fn semantic_search(query_embedding: &[f32], limit: u32) -> Result<Vec<ScoredNote>, SunderError>
- **Dependencies:**
- DatabaseManager
- EmbeddingService

### LinkService
- **Responsibilities:**
- Compute real-time latent links by embedding the current editor buffer and querying sqlite-vec
- Debounce incoming content at 300ms before triggering embedding generation
- Emit latent-link-pulse Tauri events to frontend when similarity exceeds threshold
- Maintain a small LRU cache (64 entries) of recent content hashes to avoid redundant embeddings
- Exclude the currently active note from latent link results
- **Interfaces:**
- fn compute_latent_links(content: &str, exclude_note_id: Option<&str>, threshold: f32, limit: u32) -> Result<Vec<LatentLink>, SunderError>
- fn start_live_linking(app_handle: AppHandle) -> Result<(), SunderError>
- **Dependencies:**
- EmbeddingService
- SearchService
- DatabaseManager

### GraphService
- **Responsibilities:**
- Build node/edge graph data from the similarity_cache table
- Precompute pairwise similarities above threshold and store in similarity_cache
- Invalidate and incrementally rebuild cache entries when notes are added, updated, or deleted
- Assign cluster IDs using simple single-linkage threshold clustering on similarity scores
- Return serialized graph data (nodes with initial layout positions, weighted edges) for frontend rendering
- **Interfaces:**
- fn get_graph(center_note_id: Option<&str>, threshold: f32) -> Result<GraphData, SunderError>
- fn rebuild_cache_for_note(note_id: &str) -> Result<(), SunderError>
- fn rebuild_full_cache() -> Result<(), SunderError>
- **Dependencies:**
- DatabaseManager
- EmbeddingService

### FileWatcherService
- **Responsibilities:**
- Watch a user-configured directory for .md file changes using the notify crate
- Detect file create, modify, delete events with debouncing (500ms) to batch rapid saves
- On create/modify: read file content, upsert note via NoteService, trigger re-embedding via EmbeddingService
- On delete: mark note as orphaned (soft delete) or hard delete based on user setting
- Emit file-change Tauri events to frontend for UI updates
- Handle initial directory scan on startup to import existing markdown files
- **Interfaces:**
- fn start_watching(directory: &str, app_handle: AppHandle) -> Result<(), SunderError>
- fn stop_watching() -> Result<(), SunderError>
- fn scan_directory(directory: &str) -> Result<Vec<String>, SunderError>
- **Dependencies:**
- NoteService
- EmbeddingService
- DatabaseManager

### DatabaseManager
- **Responsibilities:**
- Initialize SQLite connection with WAL journal mode for concurrent reads
- Compile in and load FTS5 and sqlite-vec extensions
- Run schema migrations on startup from embedded SQL strings
- Provide a connection pool (r2d2-sqlite, pool size 4) for concurrent read access
- Provide a single write connection with serialized access via a Mutex
- Store database file in platform app data directory (tauri::api::path::app_data_dir)
- **Interfaces:**
- fn initialize(app_data_dir: &Path) -> Result<DatabaseManager, SunderError>
- fn get_read_conn() -> Result<PooledConnection, SunderError>
- fn get_write_conn() -> Result<MutexGuard<Connection>, SunderError>
- fn run_migrations() -> Result<u32, SunderError>

### SettingsService
- **Responsibilities:**
- Store and retrieve user preferences from a settings table in SQLite
- Provide defaults for all settings (threshold: 0.65, debounce: 300ms, theme: dark)
- Validate setting values before persistence (threshold 0.0-1.0, debounce 100-2000ms)
- Emit settings-changed event when settings are updated so services can reconfigure
- **Interfaces:**
- fn get_settings() -> Result<Settings, SunderError>
- fn update_settings(patch: SettingsPatch) -> Result<(), SunderError>
- **Dependencies:**
- DatabaseManager

### Frontend: EditorView
- **Responsibilities:**
- Render CodeMirror 6 editor with markdown syntax highlighting and vim-optional keybindings
- Debounce content changes at 300ms and send to backend for latent link computation
- Auto-save note content on 1-second idle via update_note IPC command
- Display note title, word count, and last-updated timestamp in a minimal header bar
- **Interfaces:**
- React component: <EditorView noteId={string} />
- Hooks: useNote(id), useAutoSave(id, content), useLatentLinks(content)
- **Dependencies:**
- NoteService (via IPC)
- LinkService (via IPC events)

### Frontend: GraphCanvas
- **Responsibilities:**
- Render Latent Link graph as an SVG overlay using d3-force for physics-based layout
- Draw nodes as glass-morphic circles with title labels, edges as translucent lines with width proportional to similarity weight
- Animate pulse effect on nodes matching current latent link results (CSS keyframe glow)
- Support click-to-navigate (open note in editor), drag to reposition, scroll to zoom
- Request graph data from get_graph_data IPC command on mount and on note changes
- **Interfaces:**
- React component: <GraphCanvas centerNoteId={string | null} threshold={number} />
- Hooks: useGraphData(centerNoteId, threshold), useForceSimulation(nodes, edges)
- **Dependencies:**
- GraphService (via IPC)

### Frontend: Sidebar
- **Responsibilities:**
- Render collapsible sidebar with animated slide transition (200ms ease-out)
- Display flat note list sorted by recent, with search input at top
- Trigger search_notes IPC command on input with 200ms debounce
- Show search results with highlighted snippets and match type badges
- Support keyboard navigation (arrow keys, enter to open)
- **Interfaces:**
- React component: <Sidebar isOpen={boolean} onToggle={fn} onSelectNote={fn(id)} />
- Hooks: useNoteList(sort, offset, limit), useSearch(query)
- **Dependencies:**
- NoteService (via IPC)
- SearchService (via IPC)

### Frontend: AppShell
- **Responsibilities:**
- Top-level layout component: sidebar + main content area + graph overlay toggle
- Manage global state via useReducer: activeNoteId, sidebarOpen, graphVisible, theme
- Listen for Tauri events (latent-link-pulse, file-change, indexing-progress) and dispatch to state
- Apply glassmorphism theme via Tailwind classes: backdrop-blur-xl, bg-white/10, border-white/20
- Route between /editor, /graph, /settings views
- **Interfaces:**
- React component: <AppShell />
- Context: AppStateContext providing dispatch and state to all children
- **Dependencies:**
- EditorView
- GraphCanvas
- Sidebar
- SettingsService (via IPC)

## 🗃 Data Model Notes
- Table: notes (id TEXT PRIMARY KEY [UUIDv7], title TEXT NOT NULL, content TEXT NOT NULL, file_path TEXT UNIQUE, word_count INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL [ISO 8601], updated_at TEXT NOT NULL [ISO 8601])

- Table: notes_fts (FTS5 virtual table mirroring notes.id, notes.title, notes.content; content-sync via triggers on notes INSERT/UPDATE/DELETE; tokenizer = unicode61)

- Table: embeddings (note_id TEXT PRIMARY KEY REFERENCES notes(id) ON DELETE CASCADE, vector BLOB NOT NULL [384 x f32 = 1536 bytes], model_version TEXT NOT NULL DEFAULT 'minilm-v2-q8', updated_at TEXT NOT NULL)

- Virtual table: vec_embeddings (sqlite-vec virtual table over embeddings.vector column for ANN cosine similarity search; HNSW index with M=16 efConstruction=200)

- Table: similarity_cache (note_id_a TEXT NOT NULL, note_id_b TEXT NOT NULL, similarity REAL NOT NULL, updated_at TEXT NOT NULL, PRIMARY KEY (note_id_a, note_id_b), CHECK (note_id_a < note_id_b))

- Table: settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)

- Table: migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL)

- Index: idx_notes_updated_at ON notes(updated_at DESC) for list pagination

- Index: idx_notes_file_path ON notes(file_path) for file watcher lookups

- Index: idx_similarity_cache_a ON similarity_cache(note_id_a) for graph queries

- Index: idx_similarity_cache_b ON similarity_cache(note_id_b) for graph queries

- All TEXT timestamps stored as ISO 8601 UTC strings

- UUIDv7 chosen for note IDs to provide time-sortability without a separate sequence, and to avoid conflicts when importing notes from filesystem

## 🔐 Validation & Security
- All Tauri IPC commands validate input with serde deserialization; malformed payloads rejected before reaching service layer
- Note title: non-empty, max 500 characters, trimmed of leading/trailing whitespace
- Note content: max 2MB to prevent memory issues during embedding; reject with ContentTooLarge error
- File paths: resolved to canonical absolute paths and verified to be within the configured watch directory to prevent directory traversal
- Search query: non-empty, max 1000 characters, sanitized for FTS5 syntax injection (escape double quotes and special FTS operators)
- Similarity threshold: validated to be in range 0.0 to 1.0 inclusive
- Settings values: validated per-key with specific range/enum checks before write
- SQLite parameterized queries used exclusively; no string interpolation in SQL
- Tauri CSP configured to disallow eval, inline scripts, and external resource loading
- No network permissions requested in Tauri capability manifest; app has no internet access
- File watcher restricted to a single user-configured directory; symlink following disabled

## 🧯 Error Handling Strategy
All Rust services return Result<T, SunderError> where SunderError is an enum with variants: NotFound, AlreadyExists, ValidationError(String), DatabaseError(String), EmbeddingError(String), IoError(String), and Internal(String). SunderError implements serde::Serialize so Tauri can return it as a structured error object to the frontend. The frontend wraps all invoke() calls in a try/catch and maps SunderError variants to user-friendly toast notifications via a central error handler. Panics in the embedding thread are caught with std::panic::catch_unwind and converted to EmbeddingError. SQLite errors are wrapped with context about which operation failed. File I/O errors include the file path in the error message.

## 🔭 Observability
- **Logging:** Rust backend uses the tracing crate with a file appender writing to {app_data_dir}/logs/sunder.log. Log level configurable in settings (default: info). Logs rotate daily, kept for 7 days. Frontend errors logged to console and forwarded to backend via a log_frontend_error IPC command for unified log storage.
- **Tracing:** Rust tracing spans on every IPC command entry/exit with command name and duration. Span IDs propagated through service calls for debugging slow operations. No distributed tracing needed (single process).
- **Metrics:**
- embedding_generation_duration_ms: histogram per note, logged at debug level
- search_query_duration_ms: histogram per search, logged at info level
- note_count: gauge emitted at startup and on create/delete
- indexing_queue_length: gauge emitted during bulk operations
- similarity_cache_hit_rate: counter for graph queries served from cache vs recomputed

## ⚡ Performance Notes
- SQLite WAL mode enables concurrent reads from the connection pool while writes are serialized; eliminates reader-writer contention for typical usage patterns
- Embedding inference runs on tokio::task::spawn_blocking with a 2-thread pool; UI thread never blocks on model inference
- FTS5 content-sync triggers keep the fulltext index updated atomically with note writes; no separate indexing step for keyword search
- sqlite-vec HNSW index provides O(log n) approximate nearest neighbor search; configured with ef_search=50 for sub-100ms queries at 50k notes
- Similarity cache stores only above-threshold pairs (typically <5% of n^2 pairs); rebuilt incrementally by recomputing only rows involving the changed note_id
- Frontend debounces latent link requests at 300ms after typing stops; LRU cache in LinkService avoids re-embedding identical content
- d3-force simulation limited to 300 iterations on initial layout, then runs at 60fps for interactive dragging; nodes beyond viewport culled from SVG rendering
- Note list pagination via keyset pagination (WHERE updated_at < ? ORDER BY updated_at DESC LIMIT ?) avoids OFFSET performance degradation
- ONNX model loaded once at startup and held in memory (~50MB resident); inference reuses allocated tensors via a thread-local arena
- Bulk reindex (reindex_all) processes notes in batches of 50 with a 10ms yield between batches to keep IPC responsive

## 🧪 Testing Strategy
### Unit
- NoteService: test CRUD operations, title extraction from front matter, word count computation, UUIDv7 uniqueness
- EmbeddingService: test vector dimensionality (384), deterministic output for identical input, chunking behavior for long content, error on empty input
- SearchService: test FTS5 query construction, reciprocal rank fusion scoring, deduplication of hybrid results, empty query handling
- LinkService: test threshold filtering, exclusion of active note, LRU cache hit/miss behavior
- GraphService: test cluster assignment, edge weight computation, incremental cache invalidation
- DatabaseManager: test migration sequencing, idempotent re-run, WAL mode activation
- SettingsService: test default values, validation rejection of out-of-range values
- Frontend hooks: test useAutoSave debounce timing (vitest with fake timers), useSearch result mapping, useGraphData transform
### Integration
- NoteService + DatabaseManager: test full lifecycle (create, read, update, delete) with real SQLite in-memory database
- EmbeddingService + SearchService: test semantic search returns relevant results for known similar content pairs
- FileWatcherService + NoteService: test file create/modify/delete events trigger correct note operations (using temp directory)
- SearchService hybrid: test FTS5 + vector results merge correctly with reciprocal rank fusion against a seeded 100-note corpus
- GraphService + similarity_cache: test cache rebuild after note update produces correct graph edges
- Tauri IPC round-trip: test invoke('create_note') from a mock webview receives correct serialized response
### E2E
- Create a new note, type content, verify it appears in sidebar list and is retrievable
- Type content similar to an existing note, verify latent-link-pulse event fires within 600ms (300ms debounce + 300ms tolerance)
- Search for a note by keyword, verify FTS5 results appear; search by paraphrased description, verify semantic results appear
- Open graph view, verify nodes and edges render for a seeded 20-note corpus, click a node to navigate to editor
- Configure a watch directory with 5 markdown files, verify all are imported and indexed within 10 seconds
- Delete a note, verify it is removed from sidebar, search results, and graph view
- Cold start with 1000-note seeded database, verify app is interactive within 2 seconds

## 🚀 Rollout Plan
- Phase 1 - Foundation (weeks 1-2): Set up Tauri 2 project scaffold with Vite 6 + React 19 + TypeScript frontend. Implement DatabaseManager with migrations, notes table, FTS5 virtual table. Implement NoteService CRUD. Build AppShell, Sidebar (note list only), and EditorView with CodeMirror 6. Wire up IPC for note CRUD. Unit and integration tests for NoteService.

- Phase 2 - Search (weeks 3-4): Integrate sqlite-vec extension. Implement EmbeddingService with bundled ONNX model. Implement SearchService with hybrid FTS5 + vector search. Add search input to Sidebar. Implement background indexing queue. Unit tests for embedding determinism, integration tests for search relevance.

- Phase 3 - Latent Links (weeks 5-6): Implement LinkService with debounced live linking. Add latent-link-pulse event emission. Build pulse indicator UI in EditorView (subtle sidebar glow or inline badges). Implement LRU content cache. Integration tests for latent link accuracy and timing.

- Phase 4 - Graph View (weeks 7-8): Implement GraphService with similarity_cache and clustering. Build GraphCanvas component with d3-force layout. Add glassmorphism node styling, edge rendering, zoom/pan/drag interactions. Click-to-navigate from graph to editor. Performance test with 2000-node graph.

- Phase 5 - File Watcher & Polish (weeks 9-10): Implement FileWatcherService with notify crate. Add directory configuration in Settings view. Initial directory scan and import. Apply full glassmorphism theme (backdrop-blur, translucent panels, subtle borders). Keyboard navigation and accessibility pass.

- Phase 6 - Packaging & Release (weeks 11-12): Configure Tauri bundler for macOS (.dmg universal), Windows (.msi), Linux (.AppImage). Set up GitHub Actions CI/CD with cross-platform builds. Code signing for macOS and Windows. Tauri auto-updater configuration. Cold start and search performance benchmarks against NFR targets. Beta release to GitHub Releases.

## ❓ Open Questions
- Which specific quantized ONNX embedding model to bundle? all-MiniLM-L6-v2 quantized to INT8 is the assumed default (~30MB), but alternatives like bge-small-en-v1.5 may offer better quality at similar size.
- Should the similarity_cache be rebuilt fully on startup or only lazily as graph views are opened? Full rebuild ensures fresh data but adds to cold start time for large corpora.
- How should the app handle notes that exist both as filesystem files in the watched directory and as internal notes created via the editor? Conflict resolution strategy for concurrent edits from an external editor and Sunder simultaneously.
- Should the graph layout positions be persisted between sessions, or should the force simulation re-run each time the graph is opened? Persisting positions avoids layout jitter but requires storage and invalidation logic.
- What is the minimum content length required before latent link computation is triggered? Too short (e.g., <20 chars) produces noisy embeddings; too long (e.g., >100 chars) delays the pulse effect.
- Should the app support multiple watch directories or strictly one? Multiple directories add complexity but match users who organize research across project folders.
# Tasks Plan — Sunder

## 📌 Global Assumptions
- Solo developer building incrementally with a focus on macOS first, then cross-platform
- The all-MiniLM-L6-v2 INT8 quantized ONNX model is the chosen embedding model (~30MB)
- sqlite-vec Rust crate provides compile-time loadable extension support without dynamic library loading
- The ort crate supports the target ONNX model on macOS ARM64, macOS x86_64, Windows x64, and Linux x64
- The user has a valid Apple Developer ID certificate for macOS code signing
- Single watch directory is supported (not multiple directories)
- Content under 20 characters is too short for meaningful latent link computation
- Similarity cache is rebuilt incrementally on note changes, not lazily on graph open
- Graph layout positions are not persisted between sessions; d3-force re-runs each time

## ⚠️ Risks
- sqlite-vec Rust integration may have platform-specific compilation issues, especially on Windows; mitigate by testing early on all three platforms
- ONNX model inference via ort crate may have different performance characteristics on different CPU architectures; benchmark on each platform in Phase 2
- Bundling a 30MB ONNX model pushes the app bundle beyond the 10MB target; may need to explore smaller models or download-on-first-run
- d3-force with >500 nodes may cause frame drops in the SVG renderer; may need to switch to Canvas or WebGL rendering for large graphs
- CodeMirror 6 and Tauri webview keyboard event handling may conflict on some platforms; test early
- Pairwise similarity cache grows as O(n^2) in note count; at 10,000+ notes the cache rebuild time may become significant
- File watcher debouncing at 500ms may miss rapid successive changes from external editors that batch-save
- macOS notarization process may be slow and require Apple Developer Program enrollment

## 🧩 Epics
## Project Scaffold & Build Pipeline
**Goal:** Set up the Tauri 2 project with Vite 6, React 19, TypeScript, Tailwind CSS 4, and Rust backend so that a blank app compiles and launches on macOS, Windows, and Linux.

### User Stories
_None_

### Acceptance Criteria
_None_

### ✅ Initialize Tauri 2 project with Vite 6 + React 19 + TypeScript frontend (2h)

Run `npm create tauri-app@latest` selecting Vite 6 + React + TypeScript template. Verify the default window opens with hot-reload working. Pin Tauri CLI and Tauri API versions in Cargo.toml and package.json.

**Acceptance Criteria**
- `cargo tauri dev` launches a window with the React dev server
- Hot module reload works when editing a .tsx file
- Tauri CLI version is pinned in Cargo.toml
- Frontend runs on Vite 6 with React 19

**Dependencies**
_None_

### ✅ Configure Tailwind CSS 4 with glassmorphism base tokens (1h)

Install Tailwind CSS 4 and configure it with Vite. Define custom theme tokens for the glassmorphism aesthetic: backdrop-blur values, translucent background colors (bg-white/10, bg-white/5), border colors (border-white/20), and a dark color palette. Create a base CSS layer with these tokens.

**Acceptance Criteria**
- Tailwind CSS 4 is installed and integrated with Vite
- A test component using `backdrop-blur-xl bg-white/10 border border-white/20` renders correctly
- Custom theme tokens are defined in tailwind config
- Dark theme is the default

**Dependencies**
- Initialize Tauri 2 project with Vite 6 + React 19 + TypeScript frontend

### ✅ Set up Rust workspace structure with module stubs (2h)

Organize the Rust src-tauri/src directory into modules: db/, services/ (note, embedding, search, link, graph, file_watcher, settings), and error.rs. Create stub files with empty structs/functions for each module. Add rusqlite, serde, serde_json, uuid, tracing, and tokio as Cargo dependencies.

**Acceptance Criteria**
- Rust project compiles with all module stubs
- Cargo.toml includes rusqlite, serde, serde_json, uuid (v7 feature), tracing, tokio
- Module tree matches the TRD module list
- Each service module has a stub struct and placeholder impl block

**Dependencies**
- Initialize Tauri 2 project with Vite 6 + React 19 + TypeScript frontend

### ✅ Define SunderError enum and Tauri IPC error serialization (1h)

Create src-tauri/src/error.rs with a SunderError enum containing variants: NotFound, AlreadyExists, ValidationError(String), DatabaseError(String), EmbeddingError(String), IoError(String), Internal(String). Implement serde::Serialize and std::fmt::Display. Implement From<rusqlite::Error> and From<std::io::Error> conversions. Verify Tauri can serialize errors to the frontend.

**Acceptance Criteria**
- SunderError enum compiles with all variants
- Serde serialization produces structured JSON error objects
- From conversions work for rusqlite::Error and std::io::Error
- A test Tauri command returning Err(SunderError::NotFound) sends a parseable error to the frontend

**Dependencies**
- Set up Rust workspace structure with module stubs

### ✅ Configure ESLint, Prettier, and Clippy linting (1h)

Set up ESLint with TypeScript rules and Prettier for the frontend. Configure Clippy with reasonable defaults for the Rust backend. Add lint scripts to package.json. Ensure both lint cleanly on the scaffold.

**Acceptance Criteria**
- `npm run lint` passes with zero warnings on scaffold code
- `cargo clippy` passes with zero warnings on scaffold code
- Prettier formats all .ts/.tsx files consistently

**Dependencies**
- Initialize Tauri 2 project with Vite 6 + React 19 + TypeScript frontend

### ✅ Set up frontend TypeScript types for all IPC contracts (2h)

Create a src/types/ directory with TypeScript interfaces matching every IPC command request/response from the TRD: Note, NoteList, SearchResult, LatentLink, GraphData, GraphNode, GraphEdge, Settings, SunderError, and all event payloads. Create typed invoke() and listen() wrappers.

**Acceptance Criteria**
- All IPC request/response types are defined matching the TRD API contracts
- Typed wrapper functions exist for every invoke command
- Typed listener helpers exist for latent-link-pulse, indexing-progress, file-change events
- TypeScript compiles with strict mode and no errors

**Dependencies**
- Initialize Tauri 2 project with Vite 6 + React 19 + TypeScript frontend

## Database Layer
**Goal:** Implement DatabaseManager with SQLite, WAL mode, connection pooling, FTS5, migrations, and all schema tables so that services have a reliable persistence layer.

### User Stories
_None_

### Acceptance Criteria
_None_

### ✅ Implement DatabaseManager with SQLite connection and WAL mode (3h)

Create DatabaseManager struct that initializes a SQLite database file at the Tauri app_data_dir. Enable WAL journal mode. Set up r2d2-sqlite connection pool with 4 read connections and a Mutex-guarded single write connection. Add r2d2 and r2d2-sqlite to Cargo.toml.

**Acceptance Criteria**
- DatabaseManager::initialize() creates a .db file in app_data_dir
- WAL mode is active (verify with PRAGMA journal_mode)
- get_read_conn() returns a pooled connection
- get_write_conn() returns a Mutex-guarded connection
- Unit test: two concurrent read connections work simultaneously

**Dependencies**
- Define SunderError enum and Tauri IPC error serialization

### ✅ Implement migration system with embedded SQL (2h)

Create a migrations module that stores SQL migration strings as Rust constants. Create a migrations table to track applied versions. On startup, run_migrations() applies any unapplied migrations in order. Migrations are idempotent (IF NOT EXISTS). First migration creates the notes table and indexes.

**Acceptance Criteria**
- migrations table is created on first run
- run_migrations() applies all pending migrations in version order
- Re-running migrations on an up-to-date database is a no-op
- Unit test: fresh database gets all tables after migrations
- Unit test: migration version is tracked correctly

**Dependencies**
- Implement DatabaseManager with SQLite connection and WAL mode

### ✅ Create notes table schema and indexes (1h)

Write migration 001 that creates the notes table with columns: id TEXT PRIMARY KEY, title TEXT NOT NULL, content TEXT NOT NULL, file_path TEXT UNIQUE, word_count INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL, updated_at TEXT NOT NULL. Add indexes: idx_notes_updated_at on updated_at DESC, idx_notes_file_path on file_path.

**Acceptance Criteria**
- Migration 001 creates the notes table with all columns and correct types
- Both indexes exist after migration
- INSERT and SELECT operations work on the notes table
- file_path UNIQUE constraint rejects duplicates

**Dependencies**
- Implement migration system with embedded SQL

### ✅ Create FTS5 virtual table with content-sync triggers (2h)

Write migration 002 that creates the notes_fts FTS5 virtual table (content=notes, content_rowid=rowid) indexing title and content with unicode61 tokenizer. Create AFTER INSERT, AFTER UPDATE, and AFTER DELETE triggers on notes to keep notes_fts in sync.

**Acceptance Criteria**
- notes_fts virtual table is created
- Inserting a note into notes table automatically populates notes_fts
- Updating a note updates the FTS index
- Deleting a note removes the FTS entry
- FTS5 MATCH query returns results for inserted content

**Dependencies**
- Create notes table schema and indexes

### ✅ Create embeddings table schema (1h)

Write migration 003 that creates the embeddings table with columns: note_id TEXT PRIMARY KEY REFERENCES notes(id) ON DELETE CASCADE, vector BLOB NOT NULL, model_version TEXT NOT NULL DEFAULT 'minilm-v2-q8', updated_at TEXT NOT NULL.

**Acceptance Criteria**
- embeddings table is created with all columns
- Foreign key to notes(id) with ON DELETE CASCADE works
- Deleting a note cascades to delete its embedding
- BLOB column can store and retrieve 1536 bytes (384 x f32)

**Dependencies**
- Create notes table schema and indexes

### ✅ Integrate sqlite-vec extension and create vec_embeddings virtual table (4h)

Add sqlite-vec as a Rust dependency (compile-time loadable extension). Load the extension on DatabaseManager initialization. Write migration 004 that creates the vec_embeddings virtual table using sqlite-vec for ANN cosine similarity search over the embeddings table vectors with 384 dimensions.

**Acceptance Criteria**
- sqlite-vec extension loads without error at startup
- vec_embeddings virtual table is created
- Inserting a 384-dim vector and querying nearest neighbors returns results
- Cosine similarity query returns values between 0 and 1
- Integration test: insert 10 vectors and verify nearest neighbor ordering

**Dependencies**
- Create embeddings table schema

### ✅ Create similarity_cache and settings table schemas (1h)

Write migration 005 that creates: similarity_cache table (note_id_a TEXT, note_id_b TEXT, similarity REAL, updated_at TEXT, PRIMARY KEY(note_id_a, note_id_b), CHECK(note_id_a < note_id_b)) with indexes on note_id_a and note_id_b. settings table (key TEXT PRIMARY KEY, value TEXT NOT NULL). Seed default settings: similarity_threshold=0.65, debounce_ms=300, theme=dark.

**Acceptance Criteria**
- similarity_cache table created with composite PK and CHECK constraint
- Both similarity_cache indexes exist
- settings table created with default values seeded
- CHECK constraint rejects rows where note_id_a >= note_id_b

**Dependencies**
- Create notes table schema and indexes

## Note CRUD & NoteService
**Goal:** Implement full create, read, update, delete, and list operations for notes with UUIDv7 IDs, FTS sync, markdown front matter parsing, and Tauri IPC wiring.

### User Stories
_None_

### Acceptance Criteria
_None_

### ✅ Implement NoteService create_note with UUIDv7 and validation (3h)

Implement NoteService::create_note() that generates a UUIDv7 id, validates title (non-empty, max 500 chars, trimmed) and content (max 2MB), computes word_count, generates a snippet (first 200 chars stripped of markdown), sets created_at/updated_at to current UTC ISO 8601, and inserts into the notes table.

**Acceptance Criteria**
- create_note returns a Note with a valid UUIDv7 id
- Title is trimmed and validated (empty title rejected, >500 chars rejected)
- Content >2MB is rejected with ContentTooLarge error
- word_count is computed correctly
- created_at and updated_at are valid ISO 8601 UTC strings
- FTS5 table is populated via trigger
- Unit test: create and verify all fields

**Dependencies**
- Create FTS5 virtual table with content-sync triggers

### ✅ Implement NoteService get_note and get_note_by_file_path (1h)

Implement get_note(id) that retrieves a single note by ID with all fields including computed word_count. Implement get_note_by_file_path(path) that looks up a note by its file_path column. Return NoteNotFound error for missing notes.

**Acceptance Criteria**
- get_note returns full Note struct for existing note
- get_note returns NoteNotFound for non-existent ID
- get_note_by_file_path returns note when file_path matches
- get_note_by_file_path returns None for unknown path
- Unit test: create then get, verify round-trip

**Dependencies**
- Implement NoteService create_note with UUIDv7 and validation

### ✅ Implement NoteService update_note (2h)

Implement update_note(id, title?, content?) that updates only provided fields. Recompute word_count if content changes. Update updated_at timestamp. Return the updated Note. FTS5 index updates via trigger.

**Acceptance Criteria**
- Updating title only changes title and updated_at, not content
- Updating content recomputes word_count and snippet
- updated_at is refreshed on every update
- NoteNotFound returned for non-existent ID
- FTS5 index reflects the update
- Unit test: update title, update content, update both

**Dependencies**
- Implement NoteService get_note and get_note_by_file_path

### ✅ Implement NoteService delete_note (1h)

Implement delete_note(id) that deletes the note from the notes table. Verify the note exists first (return NoteNotFound if not). If file_path is set, delete the corresponding file from the filesystem. FTS5 and embeddings cascade via triggers/foreign keys.

**Acceptance Criteria**
- delete_note removes note from notes table
- FTS5 entry is removed via trigger
- Embedding is removed via CASCADE
- Associated file is deleted if file_path was set
- NoteNotFound returned for non-existent ID
- Unit test: create, delete, verify get returns NotFound

**Dependencies**
- Implement NoteService update_note

### ✅ Implement NoteService list_notes with pagination (2h)

Implement list_notes(offset, limit, sort_by) that returns a paginated list of notes with id, title, snippet, and updated_at. Support sorting by updated_at, created_at, or title. Use keyset pagination (WHERE updated_at < ? ORDER BY updated_at DESC LIMIT ?) for performance. Also return total count.

**Acceptance Criteria**
- Returns paginated notes with correct offset and limit
- Sorting works for all three sort fields
- Total count is accurate
- Empty database returns empty list with total=0
- Unit test: create 5 notes, page through them with limit=2

**Dependencies**
- Implement NoteService create_note with UUIDv7 and validation

### ✅ Implement markdown front matter title extraction (1h)

When create_note receives content with YAML front matter (--- delimited), extract the title field from the front matter if no explicit title was provided. Use a lightweight YAML parser (serde_yaml or manual parsing for just the title key).

**Acceptance Criteria**
- Content starting with --- front matter block has title extracted
- Explicit title parameter takes precedence over front matter title
- Malformed front matter does not crash; falls back to first heading or 'Untitled'
- Content without front matter uses provided title or 'Untitled'
- Unit test: front matter with title, without title, malformed

**Dependencies**
- Implement NoteService create_note with UUIDv7 and validation

### ✅ Wire up note CRUD Tauri IPC commands (2h)

Register Tauri commands for create_note, get_note, update_note, delete_note, and list_notes in main.rs. Each command deserializes input, calls NoteService, and returns the response or SunderError. Manage NoteService as Tauri state.

**Acceptance Criteria**
- All five IPC commands are registered and callable from frontend invoke()
- Input validation errors are returned as structured SunderError
- NoteService is initialized and injected as Tauri managed state
- Integration test: invoke create_note from a test and verify response shape

**Dependencies**
- Implement NoteService delete_note
- Implement NoteService list_notes with pagination

## Frontend Shell & Editor
**Goal:** Build the AppShell layout, collapsible Sidebar, and CodeMirror 6 EditorView so that a user can create, browse, and edit notes in a glassmorphism-styled interface.

### User Stories
_None_

### Acceptance Criteria
_None_

### ✅ Build AppShell layout with glassmorphism theme (3h)

Create the AppShell React component as the top-level layout. Use a useReducer for global state: activeNoteId, sidebarOpen, graphVisible, theme. Apply glassmorphism base styles: dark background gradient, backdrop-blur panels. Set up AppStateContext provider for child components.

**Acceptance Criteria**
- AppShell renders with a dark gradient background
- AppStateContext provides state and dispatch to children
- Sidebar area and main content area are laid out side by side
- Global state includes activeNoteId, sidebarOpen, graphVisible, theme
- Glassmorphism tokens (blur, translucency, borders) are visually applied

**Dependencies**
- Configure Tailwind CSS 4 with glassmorphism base tokens
- Set up frontend TypeScript types for all IPC contracts

### ✅ Build collapsible Sidebar component with note list (3h)

Create the Sidebar component with animated collapse/expand (200ms ease-out transition via Tailwind). Display a flat list of notes sorted by recent, loaded via the list_notes IPC command. Show title and snippet for each note. Include a toggle button. Clicking a note dispatches SET_ACTIVE_NOTE.

**Acceptance Criteria**
- Sidebar collapses and expands with smooth 200ms animation
- Note list loads via list_notes IPC on mount
- Each list item shows title and snippet preview
- Clicking a note sets activeNoteId in global state
- Toggle button is accessible and visually clear
- Empty state message shown when no notes exist

**Dependencies**
- Build AppShell layout with glassmorphism theme
- Wire up note CRUD Tauri IPC commands

### ✅ Implement useNoteList hook with pagination (2h)

Create a useNoteList(sortBy, offset, limit) hook that calls list_notes IPC and returns { notes, total, isLoading, error, loadMore }. Handle loading states and error display. Support infinite scroll via loadMore.

**Acceptance Criteria**
- Hook calls list_notes IPC with correct parameters
- Returns loading state while request is in flight
- Returns error state on IPC failure
- loadMore appends next page to existing notes
- Total count is available for display

**Dependencies**
- Set up frontend TypeScript types for all IPC contracts
- Wire up note CRUD Tauri IPC commands

### ✅ Integrate CodeMirror 6 with markdown syntax highlighting (3h)

Install @codemirror/lang-markdown, @codemirror/view, @codemirror/state, and theme packages. Create the EditorView component that renders a CodeMirror 6 instance with markdown syntax highlighting and a dark theme matching the glassmorphism aesthetic.

**Acceptance Criteria**
- CodeMirror 6 editor renders within the main content area
- Markdown syntax highlighting works (headings, bold, links, code blocks)
- Dark theme visually matches the glassmorphism aesthetic
- Editor is resizable and fills available space
- Text input and editing work correctly

**Dependencies**
- Build AppShell layout with glassmorphism theme

### ✅ Implement useNote hook and EditorView note loading (2h)

Create a useNote(id) hook that calls get_note IPC and returns the full note data. EditorView uses this hook to load note content into CodeMirror when activeNoteId changes. Display note title, word count, and last-updated timestamp in a minimal header bar above the editor.

**Acceptance Criteria**
- useNote hook fetches note data on id change
- EditorView displays loaded note content in CodeMirror
- Header bar shows title, word count, and updated_at
- Loading and error states are handled
- Switching notes replaces editor content

**Dependencies**
- Integrate CodeMirror 6 with markdown syntax highlighting
- Wire up note CRUD Tauri IPC commands

### ✅ Implement useAutoSave hook for note persistence (2h)

Create a useAutoSave(noteId, content) hook that debounces content changes at 1 second and calls update_note IPC. Show a subtle save indicator (saving... / saved). Handle save failures with a retry and toast notification.

**Acceptance Criteria**
- Content changes are debounced at 1 second before saving
- update_note IPC is called with current content
- Save indicator shows saving/saved state
- Save failures show a toast notification
- Unmounting cancels pending saves
- Unit test with fake timers: verify debounce timing

**Dependencies**
- Implement useNote hook and EditorView note loading

### ✅ Add create note and delete note UI actions (2h)

Add a 'New Note' button in the sidebar header that calls create_note IPC with a default title and empty content, then navigates to the new note. Add a delete button in the editor header that confirms with a dialog then calls delete_note IPC and navigates away.

**Acceptance Criteria**
- New Note button creates a note and opens it in the editor
- Delete button shows a confirmation dialog before deleting
- After deletion, editor shows empty state or previous note
- Sidebar list refreshes after create and delete
- Keyboard shortcut: Cmd/Ctrl+N for new note

**Dependencies**
- Build collapsible Sidebar component with note list
- Implement useNote hook and EditorView note loading

## Embedding Service
**Goal:** Bundle an ONNX embedding model, implement the EmbeddingService for on-device vector generation, and set up background indexing of notes.

### User Stories
_None_

### Acceptance Criteria
_None_

### ✅ Bundle quantized ONNX all-MiniLM-L6-v2 model as Tauri sidecar asset (2h)

Download the all-MiniLM-L6-v2 INT8 quantized ONNX model (~30MB) and its tokenizer.json. Configure Tauri to bundle these as sidecar resources accessible at runtime. Verify the files are included in the app bundle on all platforms.

**Acceptance Criteria**
- ONNX model file is bundled as a Tauri resource
- tokenizer.json is bundled alongside the model
- Files are accessible at runtime via tauri::api::path::resource_dir
- App bundle size increase is ~30MB
- Files are present in macOS .app, Windows installer, and Linux AppImage

**Dependencies**
- Set up Rust workspace structure with module stubs

### ✅ Implement ONNX model loading with ort crate (3h)

Add the ort crate to Cargo.toml. Implement EmbeddingService::new() that loads the ONNX model from the sidecar resource path into an ort::Session. Hold the session in memory for the app lifetime. Load the tokenizer from tokenizer.json using the tokenizers crate.

**Acceptance Criteria**
- ort::Session loads the ONNX model without error
- Tokenizer loads from tokenizer.json
- Model is loaded once at startup and reused
- Loading errors are wrapped in SunderError::EmbeddingError
- Integration test: model loads and is ready for inference

**Dependencies**
- Bundle quantized ONNX all-MiniLM-L6-v2 model as Tauri sidecar asset

### ✅ Implement embed_text() for single-pass vector generation (3h)

Implement EmbeddingService::embed_text(text) that tokenizes the input, runs ONNX inference, and returns a 384-dimensional f32 vector. Handle the mean pooling step to convert token embeddings to a single sentence embedding. Normalize the output vector to unit length for cosine similarity.

**Acceptance Criteria**
- embed_text returns a Vec<f32> of exactly 384 dimensions
- Output vector is L2-normalized (magnitude ~1.0)
- Identical inputs produce identical outputs (deterministic)
- Empty input returns EmbeddingError
- Unit test: embed a sentence and verify dimensionality and normalization

**Dependencies**
- Implement ONNX model loading with ort crate

### ✅ Implement text chunking for long content (2h)

For content exceeding 512 tokens, split into overlapping chunks of 512 tokens with 256 token overlap. Embed each chunk separately and average the resulting vectors. Re-normalize the averaged vector. Handle edge cases: content exactly at boundary, single chunk.

**Acceptance Criteria**
- Content <=512 tokens is embedded as a single chunk
- Content >512 tokens is split into overlapping chunks
- Chunk overlap is 256 tokens
- Averaged vector is re-normalized to unit length
- Unit test: long content produces same dimensionality as short content
- Unit test: chunk boundaries are correct

**Dependencies**
- Implement embed_text() for single-pass vector generation

### ✅ Implement index_note() and remove_embedding() with database storage (3h)

Implement index_note(note_id, content) that generates an embedding and stores it in the embeddings table and vec_embeddings virtual table. Implement remove_embedding(note_id) that deletes the embedding. Run inference on tokio::task::spawn_blocking to avoid blocking IPC.

**Acceptance Criteria**
- index_note stores the embedding vector in the embeddings table
- vec_embeddings virtual table is populated for ANN queries
- remove_embedding deletes from both tables
- Inference runs on a blocking thread pool, not the main async executor
- Integration test: index a note then query vec_embeddings for it

**Dependencies**
- Implement text chunking for long content
- Integrate sqlite-vec extension and create vec_embeddings virtual table

### ✅ Implement reindex_all with progress reporting (2h)

Implement reindex_all(progress_tx) that reads all notes, embeds each, and stores/updates embeddings. Process in batches of 50 with a 10ms yield between batches. Send IndexingProgress events via the progress_tx channel. Reject if a reindex is already running (AlreadyRunning error).

**Acceptance Criteria**
- All notes are re-embedded and stored
- Processing happens in batches of 50
- Progress events are emitted with processed/total/current_note_title
- Concurrent reindex requests return AlreadyRunning error
- 10ms yield between batches keeps IPC responsive
- Integration test: reindex 10 notes and verify all have embeddings

**Dependencies**
- Implement index_note() and remove_embedding() with database storage

### ✅ Wire up reindex_all Tauri IPC command and indexing-progress event (1h)

Register the reindex_all Tauri command. Emit indexing-progress events from the backend to the frontend during bulk indexing. Add a reindex button in the settings UI (placeholder for now).

**Acceptance Criteria**
- reindex_all IPC command is callable from frontend
- indexing-progress events are received by frontend listener
- AlreadyRunning error is returned if reindex is in progress
- Response includes queued count

**Dependencies**
- Implement reindex_all with progress reporting

### ✅ Auto-embed on note create and update (2h)

Hook into NoteService create_note and update_note to trigger asynchronous embedding generation via EmbeddingService::index_note(). On delete, call remove_embedding(). Embedding runs in the background; note CRUD returns immediately without waiting for embedding.

**Acceptance Criteria**
- Creating a note triggers background embedding
- Updating note content triggers re-embedding
- Deleting a note removes its embedding
- Note CRUD response is not delayed by embedding generation
- Integration test: create note, wait briefly, verify embedding exists

**Dependencies**
- Implement index_note() and remove_embedding() with database storage
- Wire up note CRUD Tauri IPC commands

## Search
**Goal:** Implement fulltext, semantic, and hybrid search so users can find notes by keyword or meaning.

### User Stories
_None_

### Acceptance Criteria
_None_

### ✅ Implement fulltext_search via FTS5 (3h)

Implement SearchService::fulltext_search(query, limit) that queries the notes_fts table using FTS5 MATCH with BM25 ranking. Sanitize the query string (escape double quotes and FTS5 special operators). Return results with id, title, highlighted snippet, and BM25 score.

**Acceptance Criteria**
- FTS5 MATCH query returns relevant results ranked by BM25
- Query string is sanitized against FTS5 injection
- Results include highlighted snippets using FTS5 snippet() function
- Empty query returns EmptyQuery error
- Query >1000 chars is rejected
- Unit test: insert notes, search by keyword, verify ranking

**Dependencies**
- Create FTS5 virtual table with content-sync triggers
- Implement NoteService create_note with UUIDv7 and validation

### ✅ Implement semantic_search via sqlite-vec (2h)

Implement SearchService::semantic_search(query_embedding, limit) that queries vec_embeddings for the nearest neighbors by cosine similarity. Return results with note_id, similarity score, title, and a content snippet.

**Acceptance Criteria**
- Cosine similarity query returns the most similar notes
- Results are ordered by descending similarity
- Each result includes title and snippet from the notes table
- Handles the case where no embeddings exist (empty results)
- Integration test: embed and store 5 notes, query with a similar embedding, verify top result

**Dependencies**
- Integrate sqlite-vec extension and create vec_embeddings virtual table
- Implement index_note() and remove_embedding() with database storage

### ✅ Implement hybrid search with reciprocal rank fusion (3h)

Implement SearchService::search(query, mode, limit) that runs both fulltext and semantic searches, then combines results using reciprocal rank fusion (k=60). Deduplicate results that appear in both. Each result gets a match_type: 'fulltext', 'semantic', or 'both'. Support mode parameter to run only one type if requested.

**Acceptance Criteria**
- Hybrid mode combines FTS5 and semantic results with RRF scoring
- Duplicate notes appearing in both result sets are merged with match_type 'both'
- RRF uses k=60 constant
- mode='fulltext' runs only FTS5 search
- mode='semantic' runs only vector search
- Unit test: verify RRF score computation for known rank inputs

**Dependencies**
- Implement fulltext_search via FTS5
- Implement semantic_search via sqlite-vec

### ✅ Wire up search_notes Tauri IPC command (1h)

Register the search_notes Tauri command that deserializes the request, calls SearchService::search(), and returns the response. Validate query is non-empty and under 1000 characters.

**Acceptance Criteria**
- search_notes IPC command is callable from frontend
- Returns SearchResult array matching the TRD contract
- Empty query returns structured EmptyQuery error
- mode parameter defaults to 'hybrid'
- limit parameter defaults to 20

**Dependencies**
- Implement hybrid search with reciprocal rank fusion

### ✅ Add search input to Sidebar with useSearch hook (3h)

Add a search text input at the top of the Sidebar. Create a useSearch(query) hook that debounces input at 200ms and calls search_notes IPC. Display search results with highlighted snippets and match type badges (fulltext/semantic/both). Replace the note list with search results when a query is active.

**Acceptance Criteria**
- Search input appears at the top of the sidebar
- Typing triggers search_notes with 200ms debounce
- Results replace the normal note list while query is active
- Each result shows title, snippet, and match type badge
- Clearing the search input restores the normal note list
- Keyboard: Enter on a result opens the note

**Dependencies**
- Build collapsible Sidebar component with note list
- Wire up search_notes Tauri IPC command

## Latent Links
**Goal:** Implement real-time latent link detection that pulses when the user types content similar to existing notes, with frontend visual indicators.

### User Stories
_None_

### Acceptance Criteria
_None_

### ✅ Implement LinkService compute_latent_links (2h)

Implement compute_latent_links(content, exclude_note_id, threshold, limit) that embeds the given content, queries sqlite-vec for similar notes above the threshold, excludes the current note, and returns LatentLink results with note_id, title, similarity, and snippet.

**Acceptance Criteria**
- Returns notes with similarity above the threshold
- Excludes the note matching exclude_note_id
- Content under 20 characters returns ContentTooShort error
- Results are ordered by descending similarity
- Integration test: create similar notes, verify latent links are found

**Dependencies**
- Implement semantic_search via sqlite-vec
- Implement embed_text() for single-pass vector generation

### ✅ Implement LRU content cache in LinkService (1h)

Add an LRU cache (64 entries) in LinkService that stores content hashes mapped to their computed latent links. Before embedding, hash the incoming content; if the hash is in the cache, return cached results. This avoids redundant embedding calls when the user pauses typing without changing content.

**Acceptance Criteria**
- Identical content on consecutive calls returns cached results without re-embedding
- Cache holds up to 64 entries
- Cache evicts least recently used entries when full
- Different content bypasses cache and computes fresh
- Unit test: verify cache hit and miss behavior

**Dependencies**
- Implement LinkService compute_latent_links

### ✅ Implement live linking with debounced Tauri events (3h)

Implement LinkService::start_live_linking() that listens for content from the frontend (via a Tauri command or channel), debounces at 300ms, computes latent links, and emits latent-link-pulse events back to the frontend. Wire up the get_latent_links Tauri IPC command for on-demand queries.

**Acceptance Criteria**
- Content is debounced at 300ms before triggering embedding
- latent-link-pulse events are emitted with matching notes
- get_latent_links IPC command works for on-demand queries
- Debounce resets when new content arrives within the window
- No events emitted if content is too short

**Dependencies**
- Implement LRU content cache in LinkService

### ✅ Implement useLatentLinks hook in frontend (2h)

Create a useLatentLinks(content, activeNoteId) hook that sends content to the backend on change (debounced at 300ms on the frontend side as well) and listens for latent-link-pulse events. Return the current set of latent links with loading state.

**Acceptance Criteria**
- Hook sends content to backend debounced at 300ms
- Listens for latent-link-pulse events and updates state
- Returns array of LatentLink objects
- Cleans up event listener on unmount
- Loading state indicates when computation is in progress

**Dependencies**
- Implement live linking with debounced Tauri events

### ✅ Build Latent Links panel UI in EditorView (3h)

Add a latent links indicator panel adjacent to the editor (right side or bottom panel). When latent links are active, display a list of related note titles with similarity percentages. Each link is clickable to open that note. Apply a subtle glow/pulse CSS animation when new links appear.

**Acceptance Criteria**
- Latent links panel appears when links are detected
- Each link shows note title and similarity percentage
- Clicking a link navigates to that note in the editor
- New links appearing trigger a pulse/glow animation
- Panel is hidden when no links are active
- Glassmorphism styling applied to the panel

**Dependencies**
- Implement useLatentLinks hook in frontend
- Implement useNote hook and EditorView note loading

## Graph View
**Goal:** Build the Latent Link graph visualization with d3-force layout, glassmorphism node styling, click-to-navigate, and drag interactions.

### User Stories
_None_

### Acceptance Criteria
_None_

### ✅ Implement GraphService with similarity_cache (3h)

Implement GraphService::get_graph(center_note_id, threshold) that reads from the similarity_cache table. Build a node/edge structure where nodes are notes and edges are similarity scores above threshold. Assign cluster IDs using single-linkage threshold clustering.

**Acceptance Criteria**
- get_graph returns nodes and edges from similarity_cache
- Nodes include id, title, and cluster assignment
- Edges include source, target, and weight (similarity score)
- Filtering by threshold excludes low-similarity edges
- center_note_id focuses the graph around that note if provided
- Unit test: verify clustering with known similarity data

**Dependencies**
- Create similarity_cache and settings table schemas

### ✅ Implement similarity_cache rebuild logic (3h)

Implement rebuild_cache_for_note(note_id) that recomputes pairwise similarities between the given note and all other notes, updating the similarity_cache. Implement rebuild_full_cache() for complete rebuild. Only store pairs above the configured threshold. Enforce note_id_a < note_id_b ordering.

**Acceptance Criteria**
- rebuild_cache_for_note computes similarity against all other notes
- Only pairs above threshold are stored in cache
- note_id_a < note_id_b ordering is enforced
- Old cache entries for the note are removed before inserting new ones
- rebuild_full_cache processes all notes
- Integration test: add note, rebuild, verify cache entries

**Dependencies**
- Implement GraphService with similarity_cache
- Implement index_note() and remove_embedding() with database storage

### ✅ Trigger incremental cache rebuild on note changes (2h)

After a note is created, updated, or deleted, trigger an incremental similarity_cache rebuild for that note. On delete, remove all cache entries involving that note. Run asynchronously in the background.

**Acceptance Criteria**
- Creating a note triggers cache rebuild for the new note
- Updating a note triggers cache rebuild for that note
- Deleting a note removes its cache entries
- Cache rebuilds run in the background without blocking IPC
- Integration test: create two similar notes, verify cache entry exists

**Dependencies**
- Implement similarity_cache rebuild logic
- Auto-embed on note create and update

### ✅ Wire up get_graph_data Tauri IPC command (1h)

Register the get_graph_data Tauri command. Deserialize request, call GraphService::get_graph(), and return the GraphData response with initial layout positions for nodes.

**Acceptance Criteria**
- get_graph_data IPC command is callable from frontend
- Returns nodes with id, title, x, y, cluster
- Returns edges with source, target, weight
- Threshold parameter defaults to 0.5
- center_note_id parameter is optional

**Dependencies**
- Implement GraphService with similarity_cache

### ✅ Build GraphCanvas component with d3-force layout (4h)

Create the GraphCanvas React component that fetches graph data via get_graph_data IPC and renders an SVG using d3-force simulation. Nodes are circles positioned by the force simulation, edges are lines between connected nodes. Limit simulation to 300 iterations on initial layout, then run at 60fps for interactivity.

**Acceptance Criteria**
- GraphCanvas renders an SVG with nodes and edges
- d3-force simulation positions nodes with physics-based layout
- Simulation runs 300 iterations initially then settles
- Nodes are rendered as circles with title labels
- Edges are rendered as lines with width proportional to weight
- Empty graph shows an informative message

**Dependencies**
- Wire up get_graph_data Tauri IPC command
- Build AppShell layout with glassmorphism theme

### ✅ Add glassmorphism styling to graph nodes and edges (2h)

Style graph nodes as glass-morphic circles: translucent fill, backdrop-blur effect (via SVG filter), subtle white border. Edges as semi-transparent lines with opacity proportional to weight. Color-code nodes by cluster. Apply the pulse glow CSS keyframe animation to nodes that match current latent links.

**Acceptance Criteria**
- Nodes have translucent glass-like appearance
- Edges have opacity proportional to similarity weight
- Nodes are color-coded by cluster ID
- Pulse animation plays on nodes matching active latent links
- Visual style is consistent with the overall glassmorphism theme

**Dependencies**
- Build GraphCanvas component with d3-force layout

### ✅ Add graph interactions: click, drag, zoom (3h)

Add click-to-navigate: clicking a node opens that note in the editor. Add drag to reposition: dragging a node pins it and updates the force simulation. Add scroll to zoom with pan support. Cull nodes beyond the viewport from SVG rendering for performance.

**Acceptance Criteria**
- Clicking a node navigates to that note in the editor
- Dragging a node repositions it and it stays pinned
- Scroll wheel zooms in and out
- Pan works via click-drag on the canvas background
- Nodes outside the visible viewport are not rendered
- Interactions feel smooth at 60fps

**Dependencies**
- Build GraphCanvas component with d3-force layout

### ✅ Add graph toggle button to AppShell (2h)

Add a toggle button in the AppShell to show/hide the GraphCanvas as an overlay or split view. When the graph is visible, it overlays the editor or takes the right half of the content area. Animate the transition.

**Acceptance Criteria**
- Toggle button shows/hides the graph view
- Graph renders as an overlay or split panel
- Transition is animated smoothly
- Graph state (graphVisible) is managed in AppShell reducer
- Keyboard shortcut: Cmd/Ctrl+G toggles graph

**Dependencies**
- Build GraphCanvas component with d3-force layout
- Build AppShell layout with glassmorphism theme

## File Watcher
**Goal:** Implement filesystem watching for a user-configured directory of markdown files, with auto-import, sync, and change detection.

### User Stories
_None_

### Acceptance Criteria
_None_

### ✅ Implement FileWatcherService with notify crate (3h)

Add the notify crate to Cargo.toml. Implement start_watching(directory, app_handle) that creates a filesystem watcher on the given directory for .md files. Debounce events at 500ms. Detect create, modify, and delete events. Implement stop_watching() to halt the watcher.

**Acceptance Criteria**
- Watcher detects .md file creation in the watched directory
- Watcher detects .md file modification
- Watcher detects .md file deletion
- Events are debounced at 500ms
- Non-.md files are ignored
- stop_watching() cleanly stops the watcher
- Integration test with temp directory: create file, verify event

**Dependencies**
- Set up Rust workspace structure with module stubs

### ✅ Handle file create/modify events: upsert notes (3h)

On file create or modify: read the file content, extract title from front matter or filename, call NoteService to create or update the note (upsert by file_path). Trigger EmbeddingService::index_note() for the new/updated content. Emit file-change Tauri event.

**Acceptance Criteria**
- New .md file creates a new note with file_path set
- Modified .md file updates the existing note
- Title is extracted from front matter or derived from filename
- Embedding is generated for the new/updated note
- file-change event is emitted with event_type and file_path
- Integration test: create file in temp dir, verify note exists

**Dependencies**
- Implement FileWatcherService with notify crate
- Implement NoteService create_note with UUIDv7 and validation
- Auto-embed on note create and update

### ✅ Handle file delete events: remove notes (1h)

On file delete: look up the note by file_path, delete it via NoteService. Embedding and FTS entries cascade. Emit file-change event with event_type 'deleted'.

**Acceptance Criteria**
- Deleted .md file removes the corresponding note
- Embedding and FTS entries are cleaned up
- file-change event is emitted with event_type 'deleted'
- Deleting a non-tracked file is a no-op
- Integration test: create file, delete it, verify note is gone

**Dependencies**
- Handle file create/modify events: upsert notes

### ✅ Implement initial directory scan on startup (2h)

Implement scan_directory(directory) that reads all .md files in the configured directory and upserts them as notes. Called on app startup if a watch directory is configured. Skip files that already have up-to-date notes (compare file modified time with note updated_at).

**Acceptance Criteria**
- All .md files in the directory are imported as notes
- Already up-to-date notes are skipped
- New or modified files are upserted
- Returns count of files found
- Integration test: create 5 files, scan, verify 5 notes exist

**Dependencies**
- Handle file create/modify events: upsert notes

### ✅ Wire up set_watch_directory Tauri IPC command (2h)

Register set_watch_directory IPC command. Validate the path (exists, is a directory, readable). Store the directory path in settings. Start the file watcher. Run initial directory scan. Return files_found count.

**Acceptance Criteria**
- set_watch_directory validates the path
- DirectoryNotFound error for non-existent path
- NotADirectory error for file paths
- PermissionDenied error for unreadable directories
- Watcher starts after successful configuration
- Initial scan runs and returns files_found

**Dependencies**
- Implement initial directory scan on startup
- Create similarity_cache and settings table schemas

### ✅ Validate file paths against watch directory (directory traversal prevention) (1h)

Before processing any file event, resolve the file path to a canonical absolute path and verify it is within the configured watch directory. Reject symlinks that point outside the directory. This prevents directory traversal attacks.

**Acceptance Criteria**
- File paths are canonicalized before processing
- Paths outside the watch directory are rejected
- Symlinks pointing outside the directory are rejected
- Unit test: verify traversal attempt is blocked

**Dependencies**
- Implement FileWatcherService with notify crate

## Settings & Configuration
**Goal:** Implement the SettingsService and a settings UI for managing preferences like similarity threshold, debounce timing, theme, and watch directory.

### User Stories
_None_

### Acceptance Criteria
_None_

### ✅ Implement SettingsService with validation (2h)

Implement SettingsService::get_settings() and update_settings(patch). Read/write from the settings table. Validate values: similarity_threshold 0.0-1.0, debounce_ms 100-2000, theme 'dark'|'light'. Return defaults for missing keys. Emit settings-changed event on update.

**Acceptance Criteria**
- get_settings returns current settings with defaults for missing keys
- update_settings validates and persists changes
- Out-of-range threshold is rejected with InvalidValue error
- Out-of-range debounce_ms is rejected
- Invalid theme value is rejected
- Unit test: update and retrieve settings round-trip

**Dependencies**
- Create similarity_cache and settings table schemas

### ✅ Wire up get_settings and update_settings Tauri IPC commands (1h)

Register both IPC commands. get_settings returns the full settings object. update_settings accepts a partial patch and returns success.

**Acceptance Criteria**
- get_settings IPC returns Settings object matching TRD contract
- update_settings IPC accepts partial updates
- Validation errors are returned as structured SunderError
- Settings changes persist across app restarts

**Dependencies**
- Implement SettingsService with validation

### ✅ Build Settings view UI (3h)

Create a Settings page/panel accessible from the AppShell. Include controls for: watch directory (path picker + set button), similarity threshold (slider 0-1), debounce timing (slider 100-2000ms), theme toggle (dark/light), and a reindex all button with progress indicator.

**Acceptance Criteria**
- Settings view is accessible from a menu or button in AppShell
- Watch directory can be selected via a native directory picker
- Similarity threshold slider adjusts between 0.0 and 1.0
- Debounce slider adjusts between 100 and 2000ms
- Theme toggle switches between dark and light
- Reindex button triggers reindex_all and shows progress
- All changes are saved via update_settings IPC

**Dependencies**
- Wire up get_settings and update_settings Tauri IPC commands
- Wire up reindex_all Tauri IPC command and indexing-progress event
- Wire up set_watch_directory Tauri IPC command

## Logging & Observability
**Goal:** Set up structured logging with tracing, file rotation, frontend error forwarding, and performance metrics.

### User Stories
_None_

### Acceptance Criteria
_None_

### ✅ Configure tracing crate with file appender and rotation (2h)

Set up the tracing crate with tracing-subscriber and tracing-appender. Write logs to {app_data_dir}/logs/sunder.log with daily rotation, keeping 7 days. Default log level: info. Add tracing spans on every IPC command entry/exit with command name and duration.

**Acceptance Criteria**
- Logs are written to the correct file path
- Daily rotation creates new log files
- Old logs beyond 7 days are cleaned up
- IPC commands have entry/exit spans with duration
- Log level is configurable (info default)
- Logs include timestamps and structured fields

**Dependencies**
- Implement DatabaseManager with SQLite connection and WAL mode

### ✅ Add performance metrics logging (1h)

Add tracing instrumentation for key operations: embedding_generation_duration_ms (debug level), search_query_duration_ms (info level), note_count gauge at startup and on create/delete. Log indexing_queue_length during bulk operations.

**Acceptance Criteria**
- Embedding duration is logged at debug level
- Search duration is logged at info level
- Note count is logged at startup
- Indexing queue length is logged during bulk operations
- Metrics are structured tracing fields, not string interpolation

**Dependencies**
- Configure tracing crate with file appender and rotation

### ✅ Implement log_frontend_error IPC command (2h)

Register a log_frontend_error Tauri command that receives error details from the frontend (message, stack, component) and writes them to the same log file via tracing. Add a central error handler in the frontend that catches unhandled errors and forwards them.

**Acceptance Criteria**
- Frontend errors are logged to the same sunder.log file
- Error details include message, stack trace, and component name
- Central frontend error handler catches React error boundaries and unhandled rejections
- Frontend error logs are distinguishable from backend logs

**Dependencies**
- Configure tracing crate with file appender and rotation
- Build AppShell layout with glassmorphism theme

## Keyboard Navigation & Accessibility
**Goal:** Ensure the app is fully navigable by keyboard and meets basic accessibility standards.

### User Stories
_None_

### Acceptance Criteria
_None_

### ✅ Add keyboard navigation to Sidebar (2h)

Support arrow key navigation through the note list and search results. Enter opens the selected note. Escape clears search and returns focus to the list. Tab moves focus between search input and list. Add visible focus indicators.

**Acceptance Criteria**
- Arrow up/down navigates through the note list
- Enter opens the focused note
- Escape clears search input
- Tab moves between search input and list
- Visible focus ring on the active list item
- Screen reader announces list item titles

**Dependencies**
- Add search input to Sidebar with useSearch hook

### ✅ Add global keyboard shortcuts (2h)

Implement global keyboard shortcuts: Cmd/Ctrl+N (new note), Cmd/Ctrl+G (toggle graph), Cmd/Ctrl+B (toggle sidebar), Cmd/Ctrl+K (focus search). Register shortcuts via the editor or window-level key handler. Ensure shortcuts don't conflict with CodeMirror bindings.

**Acceptance Criteria**
- Cmd/Ctrl+N creates a new note
- Cmd/Ctrl+G toggles the graph view
- Cmd/Ctrl+B toggles the sidebar
- Cmd/Ctrl+K focuses the search input
- Shortcuts work when focus is in the editor
- No conflicts with CodeMirror default bindings

**Dependencies**
- Add create note and delete note UI actions
- Add graph toggle button to AppShell

### ✅ Add ARIA labels and roles across the UI (2h)

Audit all components and add appropriate ARIA labels, roles, and live regions. The sidebar should be a navigation landmark. The editor should be labeled. Toast notifications should use aria-live. The graph should have an accessible description.

**Acceptance Criteria**
- Sidebar has role=navigation and aria-label
- Editor has role=main and aria-label
- Search input has aria-label and search results have aria-live
- Toast notifications use aria-live=polite
- Graph has an aria-label describing its purpose
- No accessibility violations from automated audit

**Dependencies**
- Build AppShell layout with glassmorphism theme

## Tauri CSP & Security Hardening
**Goal:** Lock down the Tauri security configuration to prevent code injection, unauthorized network access, and filesystem traversal.

### User Stories
_None_

### Acceptance Criteria
_None_

### ✅ Configure Tauri CSP and capability manifest (1h)

Set Content Security Policy in tauri.conf.json to disallow eval(), inline scripts, and external resource loading. Configure the Tauri capability manifest to request zero network permissions. Restrict IPC commands to only the defined set.

**Acceptance Criteria**
- CSP disallows eval and inline scripts
- CSP blocks external resource loading
- No network permissions in the capability manifest
- Only defined IPC commands are exposed
- Attempting to fetch an external URL from the frontend fails

**Dependencies**
- Initialize Tauri 2 project with Vite 6 + React 19 + TypeScript frontend

### ✅ Add input sanitization for FTS5 queries (1h)

Create a sanitize_fts_query() function that escapes or removes FTS5 special syntax characters (double quotes, OR, AND, NOT, NEAR, *, column filters) from user search input to prevent FTS5 injection. Apply this in SearchService before constructing FTS5 MATCH queries.

**Acceptance Criteria**
- Double quotes in queries are escaped
- FTS5 operators (OR, AND, NOT, NEAR) are escaped or quoted
- Wildcard * is removed or escaped
- Column filter syntax (column:) is neutralized
- Sanitized queries still return relevant results
- Unit test: verify each special character is handled

**Dependencies**
- Implement fulltext_search via FTS5

## Packaging & Distribution
**Goal:** Configure Tauri bundler for macOS, Windows, and Linux builds with code signing and auto-updater.

### User Stories
_None_

### Acceptance Criteria
_None_

### ✅ Configure Tauri bundler for macOS (.dmg universal binary) (2h)

Set up tauri.conf.json bundle configuration for macOS. Target universal binary (ARM64 + x86_64). Configure .dmg output with app icon and background. Set bundle identifier and version.

**Acceptance Criteria**
- cargo tauri build produces a .dmg file
- DMG contains a universal binary
- App icon is correctly set
- Bundle identifier is set (e.g., com.sunder.app)
- App launches correctly from the DMG on macOS

**Dependencies**
- Configure Tauri CSP and capability manifest

### ✅ Configure Tauri bundler for Windows (.msi) and Linux (.AppImage) (2h)

Configure tauri.conf.json for Windows MSI output and Linux AppImage output. Set Windows-specific metadata (publisher, icon). Set Linux desktop entry and icon.

**Acceptance Criteria**
- cargo tauri build on Windows produces a .msi installer
- cargo tauri build on Linux produces an .AppImage
- Windows installer includes proper metadata and icon
- Linux AppImage runs without external dependencies
- App bundle size is under 50MB (model included)

**Dependencies**
- Configure Tauri CSP and capability manifest

### ✅ Set up GitHub Actions CI/CD for cross-platform builds (4h)

Create a GitHub Actions workflow that builds the app on macOS, Windows, and Linux runners. Run Rust tests and Clippy on each platform. Build the frontend. Produce release artifacts. Upload artifacts to GitHub Releases on tag push.

**Acceptance Criteria**
- CI runs on push to main and on tag creation
- Builds succeed on macOS, Windows, and Linux runners
- Rust tests and Clippy run on each platform
- Release artifacts are uploaded to GitHub Releases on tag
- Build completes within 15 minutes per platform

**Dependencies**
- Configure Tauri bundler for macOS (.dmg universal binary)
- Configure Tauri bundler for Windows (.msi) and Linux (.AppImage)

### ✅ Configure Tauri auto-updater (3h)

Set up the Tauri auto-updater plugin with a GitHub Releases endpoint. Configure update check on app startup. Show a non-intrusive notification when an update is available with an install button.

**Acceptance Criteria**
- Auto-updater checks for updates on startup
- Update check uses GitHub Releases endpoint
- Notification appears when a new version is available
- User can choose to install or skip the update
- Update downloads and installs correctly

**Dependencies**
- Set up GitHub Actions CI/CD for cross-platform builds

### ✅ Configure macOS code signing (3h)

Set up macOS code signing with a Developer ID certificate in the CI pipeline. Configure notarization via Apple's notarytool. Ensure the .dmg and .app are signed and notarized for Gatekeeper approval.

**Acceptance Criteria**
- App is signed with a valid Developer ID certificate
- App passes notarization via notarytool
- Gatekeeper allows the app to run without warnings
- Signing works in the GitHub Actions CI pipeline

**Dependencies**
- Set up GitHub Actions CI/CD for cross-platform builds

## Testing
**Goal:** Write comprehensive unit, integration, and E2E tests to verify correctness and catch regressions.

### User Stories
_None_

### Acceptance Criteria
_None_

### ✅ Write NoteService unit tests (3h)

Test CRUD operations, title extraction from front matter, word count computation, UUIDv7 uniqueness, validation (empty title, max length, max content size), and snippet generation. Use an in-memory SQLite database.

**Acceptance Criteria**
- Tests cover create, get, update, delete, list operations
- Title extraction from YAML front matter tested
- Word count accuracy verified
- UUIDv7 IDs are unique across multiple creates
- Validation errors tested for edge cases
- All tests pass with in-memory database

**Dependencies**
- Wire up note CRUD Tauri IPC commands

### ✅ Write EmbeddingService unit tests (2h)

Test vector dimensionality (384), deterministic output for identical input, chunking behavior for long content, error on empty input, and vector normalization.

**Acceptance Criteria**
- Embedding output is exactly 384 dimensions
- Same input produces identical output across calls
- Long content chunking produces correct number of chunks
- Empty input returns error
- Output vectors are unit-normalized
- All tests pass

**Dependencies**
- Implement text chunking for long content

### ✅ Write SearchService unit and integration tests (3h)

Unit test: FTS5 query construction, RRF score computation, deduplication logic. Integration test: seed a 100-note corpus with known content, verify fulltext returns keyword matches, semantic returns paraphrased matches, and hybrid merges correctly.

**Acceptance Criteria**
- FTS5 query sanitization tested with special characters
- RRF score computation matches expected formula
- Hybrid deduplication correctly merges results
- Integration: fulltext finds keyword matches in 100-note corpus
- Integration: semantic finds paraphrased matches
- Integration: hybrid merges both result sets

**Dependencies**
- Implement hybrid search with reciprocal rank fusion

### ✅ Write LinkService and GraphService unit tests (2h)

LinkService: test threshold filtering, exclusion of active note, LRU cache hit/miss. GraphService: test cluster assignment, edge weight computation, incremental cache invalidation.

**Acceptance Criteria**
- Threshold filtering correctly excludes low-similarity results
- Active note is excluded from latent link results
- LRU cache returns cached results for identical content
- Cluster assignment groups connected notes correctly
- Cache invalidation removes stale entries
- All tests pass

**Dependencies**
- Implement LRU content cache in LinkService
- Implement similarity_cache rebuild logic

### ✅ Write FileWatcherService integration tests (2h)

Test file create/modify/delete events trigger correct note operations using a temporary directory. Verify initial scan imports all .md files. Verify symlink traversal prevention.

**Acceptance Criteria**
- File creation in temp dir creates a note
- File modification updates the note
- File deletion removes the note
- Initial scan imports all .md files
- Symlinks outside watch directory are rejected
- Non-.md files are ignored

**Dependencies**
- Implement initial directory scan on startup
- Validate file paths against watch directory (directory traversal prevention)

### ✅ Write frontend hook tests with vitest (3h)

Set up vitest with React Testing Library. Test useAutoSave debounce timing with fake timers. Test useSearch result mapping. Test useGraphData transform. Test useNoteList pagination.

**Acceptance Criteria**
- vitest is configured and running for the frontend
- useAutoSave fires after 1 second debounce (verified with fake timers)
- useSearch maps IPC results to the correct shape
- useGraphData transforms backend data for d3-force
- useNoteList handles pagination and loadMore correctly
- All frontend tests pass

**Dependencies**
- Implement useAutoSave hook for note persistence
- Add search input to Sidebar with useSearch hook

### ✅ Write E2E tests for core user workflows (6h)

Using Tauri's testing utilities or a similar E2E framework, test: create note and verify it appears in sidebar; type content and verify latent link pulse within 600ms; search by keyword and by paraphrase; open graph and click a node; configure watch directory and verify import; delete a note and verify removal.

**Acceptance Criteria**
- E2E: create note appears in sidebar list
- E2E: latent link pulse fires within 600ms of typing similar content
- E2E: keyword search returns FTS results; paraphrase search returns semantic results
- E2E: graph view renders nodes; clicking a node opens the editor
- E2E: setting a watch directory imports .md files
- E2E: deleting a note removes it from sidebar, search, and graph

**Dependencies**
- Build Latent Links panel UI in EditorView
- Add graph interactions: click, drag, zoom
- Wire up set_watch_directory Tauri IPC command
- Add search input to Sidebar with useSearch hook

### ✅ Performance benchmark: cold start and search at scale (3h)

Seed a database with 1000 notes. Measure and verify: cold start to interactive under 2 seconds, search query under 100ms, latent link computation under 300ms. Run on a representative machine and document results.

**Acceptance Criteria**
- Cold start with 1000 notes is interactive within 2 seconds
- Hybrid search completes in under 100ms
- Latent link embedding + query completes in under 300ms
- Graph view loads within 500ms for 1000-note corpus
- Results are documented and baseline numbers recorded

**Dependencies**
- Write E2E tests for core user workflows

## ❓ Open Questions
- Should the similarity_cache be rebuilt fully on startup or only incrementally? Full rebuild ensures freshness but adds cold start time. Current assumption: incremental only.
- How should concurrent edits from an external editor and Sunder be handled? Current assumption: last-write-wins based on file modification time.
- Should graph layout positions be persisted? Current assumption: no, re-run d3-force each time for simplicity.
- What is the minimum content length for latent link computation? Current assumption: 20 characters.
- Should multiple watch directories be supported? Current assumption: single directory for simplicity.
- Is all-MiniLM-L6-v2 INT8 the right model choice, or should bge-small-en-v1.5 be evaluated? Current assumption: all-MiniLM-L6-v2.
- How should the app handle the 10MB bundle size target given the 30MB ONNX model? Options: accept larger bundle, download model on first run, or use a smaller model.
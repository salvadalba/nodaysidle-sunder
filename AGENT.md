# Agent Prompts — Sunder

## Global Rules

### Do
- Use Tauri 2 with Rust backend commands and SQLite — no server, no network
- Use Vite 6 + React 19 + TypeScript + Tailwind CSS 4 for the frontend
- Use rusqlite with FTS5 and sqlite-vec for search and vector storage
- Use the ort crate for ONNX inference and tokenizers crate for tokenization
- Apply glassmorphism aesthetic: backdrop-blur, translucent panels, dark theme default

### Don't
- Do not introduce any backend server, REST API, or network-dependent service
- Do not substitute any technology in the stack (no Prisma, no Drizzle, no Electron)
- Do not use dynamic library loading for sqlite-vec — use compile-time linkage only
- Do not add unnecessary abstractions or over-engineer for hypothetical future needs
- Do not skip Tauri CSP hardening — disallow eval, inline scripts, and external resources

---

## Task Prompts
### Task 1: Project Scaffold, Database Layer & Rust Services

**Role:** Expert Tauri/Rust Systems Engineer
**Goal:** Scaffold the full Tauri 2 project with SQLite database layer, migrations, FTS5, sqlite-vec, and Rust service module stubs

**Context**
Initialize the Tauri 2 project with the full Rust backend: module structure, error handling, SQLite with WAL mode and connection pooling, migration system, all schema tables (notes, embeddings, vec_embeddings, similarity_cache, settings), FTS5 with sync triggers, and sqlite-vec integration. Also configure the frontend toolchain (Vite 6, React 19, TypeScript, Tailwind CSS 4, ESLint, Prettier, Clippy).

**Files to Create**
- src-tauri/src/main.rs
- src-tauri/src/error.rs
- src-tauri/src/db/mod.rs
- src-tauri/src/db/migrations.rs
- src-tauri/src/services/mod.rs
- src-tauri/src/services/note.rs
- src-tauri/src/services/settings.rs
- src/types/index.ts

**Files to Modify**
- src-tauri/Cargo.toml
- src-tauri/tauri.conf.json
- package.json
- tailwind.config.ts
- src/index.css

**Steps**
1. Run `npm create tauri-app@latest sunder -- --template react-ts` then pin Tauri CLI v2 in Cargo.toml. Add dependencies: rusqlite (with bundled, vtab features), serde, serde_json, uuid (v7), tracing, tracing-subscriber, tracing-appender, tokio (full), r2d2, r2d2-sqlite, notify, sqlite-vec. Install Tailwind CSS 4, ESLint with typescript-eslint, and Prettier. Configure Tailwind with glassmorphism tokens (backdrop-blur values, bg-white/10, border-white/20, dark palette).
2. Create src-tauri/src/error.rs with SunderError enum (NotFound, AlreadyExists, ValidationError, DatabaseError, EmbeddingError, IoError, Internal, ContentTooLarge, EmptyQuery, ContentTooShort, AlreadyRunning, DirectoryNotFound, NotADirectory, PermissionDenied, InvalidValue). Implement serde::Serialize, Display, From<rusqlite::Error>, From<std::io::Error>.
3. Create src-tauri/src/db/mod.rs with DatabaseManager struct: initialize() creates .db in app_data_dir, enables WAL mode and foreign keys, sets up r2d2 pool (4 read conns) and Mutex<Connection> for writes. Load sqlite-vec extension at init. Create db/migrations.rs with embedded SQL constants for 5 migrations: (001) notes table + indexes, (002) notes_fts FTS5 + sync triggers, (003) embeddings table with CASCADE, (004) vec_embeddings virtual table (384 dims), (005) similarity_cache + settings with defaults.
4. Create service module stubs in src-tauri/src/services/: note.rs (NoteService struct with CRUD stubs), settings.rs (SettingsService with get/update stubs), plus empty mod.rs files for embedding.rs, search.rs, link.rs, graph.rs, file_watcher.rs. Each has a struct and placeholder impl block. Wire DatabaseManager as Tauri managed state in main.rs.
5. Create src/types/index.ts with all IPC contract types: Note, NoteList, SearchResult, LatentLink, GraphData, GraphNode, GraphEdge, Settings, SunderError, IndexingProgress. Create typed invoke() and listen() wrappers using @tauri-apps/api. Verify with `cargo clippy`, `npm run lint`, and `npx tsc --noEmit`.

**Validation**
`cd sunder && cargo clippy -- -D warnings && npm run typecheck`

---

### Task 2: Note CRUD, Embedding Service & Search

**Role:** Expert Rust Backend Engineer
**Goal:** Implement note CRUD, ONNX embedding pipeline, and hybrid search with Tauri IPC wiring

**Context**
Implement full NoteService CRUD with UUIDv7, validation, front matter parsing, and FTS sync. Implement EmbeddingService with ONNX model loading (ort crate), text chunking, embed_text(), index_note(), remove_embedding(), and reindex_all. Implement SearchService with fulltext (FTS5 + BM25), semantic (sqlite-vec cosine), and hybrid search (reciprocal rank fusion). Wire all as Tauri IPC commands. Auto-embed on note create/update.

**Files to Create**
- src-tauri/src/services/embedding.rs
- src-tauri/src/services/search.rs
- src-tauri/resources/README.md

**Files to Modify**
- src-tauri/src/services/note.rs
- src-tauri/src/main.rs
- src-tauri/Cargo.toml
- src-tauri/tauri.conf.json

**Steps**
1. Implement NoteService in services/note.rs: create_note() generates UUIDv7, validates title (non-empty, max 500 chars, trimmed) and content (max 2MB), computes word_count, extracts title from YAML front matter if none provided, sets ISO 8601 timestamps. Implement get_note(id), get_note_by_file_path(path), update_note(id, title?, content?), delete_note(id) with file cleanup, and list_notes(offset, limit, sort_by) with keyset pagination. Add serde_yaml to Cargo.toml for front matter parsing.
2. Add ort and tokenizers crates to Cargo.toml. Configure Tauri to bundle the all-MiniLM-L6-v2 INT8 ONNX model and tokenizer.json as sidecar resources. Implement EmbeddingService::new() loading the model into ort::Session. Implement embed_text() with tokenization, inference, mean pooling, and L2 normalization to 384-dim Vec<f32>. For content >512 tokens, chunk with 256-token overlap, embed each chunk, average, and re-normalize.
3. Implement index_note(note_id, content) storing embeddings in both embeddings and vec_embeddings tables via spawn_blocking. Implement remove_embedding(). Implement reindex_all(progress_tx) processing in batches of 50 with 10ms yields and IndexingProgress events. Hook into NoteService: create_note and update_note trigger async index_note, delete_note calls remove_embedding.
4. Implement SearchService: fulltext_search() queries notes_fts with sanitized FTS5 MATCH and BM25 ranking. Create sanitize_fts_query() escaping quotes, OR/AND/NOT/NEAR operators, wildcards, and column filters. Implement semantic_search() querying vec_embeddings for cosine nearest neighbors. Implement hybrid search() combining both with reciprocal rank fusion (k=60), deduplicating with match_type 'fulltext'|'semantic'|'both'.
5. Register all Tauri IPC commands in main.rs: create_note, get_note, update_note, delete_note, list_notes, search_notes, reindex_all. Manage NoteService, EmbeddingService, SearchService as Tauri state. Emit indexing-progress events during reindex. Write unit tests for CRUD, embedding dimensionality, FTS5 sanitization, and RRF scoring. Validate: `cargo test && cargo clippy`.

**Validation**
`cd sunder && cargo test -- --nocapture && cargo clippy -- -D warnings`

---

### Task 3: Frontend Shell, Editor, Sidebar & Search UI

**Role:** Expert React/TypeScript Frontend Engineer
**Goal:** Build the glassmorphism AppShell, sidebar, CodeMirror editor, search UI, and all React hooks

**Context**
Build the React frontend: AppShell with glassmorphism theme, collapsible Sidebar with note list and search, CodeMirror 6 editor with markdown highlighting, auto-save, note creation/deletion, search UI with debounced hybrid search, and all custom hooks (useNoteList, useNote, useAutoSave, useSearch). Apply glassmorphism styling throughout.

**Files to Create**
- src/components/AppShell.tsx
- src/components/Sidebar.tsx
- src/components/EditorView.tsx
- src/components/SearchResults.tsx
- src/hooks/useNoteList.ts
- src/hooks/useNote.ts
- src/hooks/useAutoSave.ts
- src/hooks/useSearch.ts

**Files to Modify**
- src/App.tsx
- src/index.css
- package.json

**Steps**
1. Create AppShell.tsx as the top-level layout with useReducer managing global state: activeNoteId, sidebarOpen, graphVisible, theme. Provide AppStateContext. Apply dark gradient background (bg-gradient-to-br from-slate-950 to-slate-900), glassmorphism panels (backdrop-blur-xl bg-white/5 border border-white/10 rounded-2xl). Layout: sidebar (left, collapsible) + main content area (flex-1).
2. Create Sidebar.tsx with animated collapse/expand (w-72 to w-0, transition-all duration-200 ease-out). Add search input at top with magnifying glass icon. Create useNoteList(sortBy, offset, limit) hook calling list_notes IPC with loading/error states and loadMore for infinite scroll. Create useSearch(query) hook debouncing at 200ms, calling search_notes IPC. Display note list or search results with title, snippet, and match_type badges. Add 'New Note' button (Cmd/Ctrl+N shortcut).
3. Install @codemirror/lang-markdown, @codemirror/view, @codemirror/state, @codemirror/theme-one-dark. Create EditorView.tsx rendering CodeMirror 6 with markdown syntax highlighting and a custom dark theme matching glassmorphism. Create useNote(id) hook fetching via get_note IPC. Display title, word count, updated_at in a header bar. Add delete button with confirmation dialog.
4. Create useAutoSave(noteId, content) hook debouncing at 1s, calling update_note IPC. Show save indicator (saving.../saved). Handle failures with toast notification. Cancel pending saves on unmount. Add keyboard shortcuts: Cmd/Ctrl+N (new note), Cmd/Ctrl+B (toggle sidebar), Cmd/Ctrl+K (focus search). Ensure no conflicts with CodeMirror bindings.
5. Add ARIA labels and roles: sidebar as role=navigation, editor as role=main, search with aria-label, toast with aria-live=polite. Support arrow key navigation in note list, Enter to open, Escape to clear search. Set up vitest with React Testing Library. Write tests for useAutoSave debounce timing and useSearch result mapping. Validate: `npm run typecheck && npx vitest run`.

**Validation**
`cd sunder && npm run typecheck && npx vitest run`

---

### Task 4: Latent Links, Graph View & File Watcher

**Role:** Expert Full-Stack Tauri Engineer
**Goal:** Implement latent links, d3-force graph view, file watcher, and settings UI

**Context**
Implement LinkService for real-time latent link detection with LRU cache and debounced Tauri events. Build GraphService with similarity_cache and incremental rebuild. Create the d3-force graph visualization with glassmorphism nodes, click/drag/zoom interactions. Implement FileWatcherService with the notify crate for .md file monitoring, auto-import, directory traversal prevention, and initial scan. Build Settings UI.

**Files to Create**
- src-tauri/src/services/link.rs
- src-tauri/src/services/graph.rs
- src-tauri/src/services/file_watcher.rs
- src/components/GraphCanvas.tsx
- src/components/LatentLinksPanel.tsx
- src/components/SettingsView.tsx
- src/hooks/useLatentLinks.ts
- src/hooks/useGraphData.ts

**Files to Modify**
- src-tauri/src/main.rs
- src-tauri/src/services/mod.rs
- src/components/AppShell.tsx
- src/components/EditorView.tsx

**Steps**
1. Implement LinkService in link.rs: compute_latent_links(content, exclude_note_id, threshold, limit) embeds content, queries sqlite-vec, filters by threshold, excludes current note. Add 64-entry LRU cache keyed by content hash. Implement start_live_linking() debouncing at 300ms, emitting latent-link-pulse events. Implement GraphService in graph.rs: get_graph(center_note_id, threshold) reads similarity_cache, builds nodes/edges, assigns clusters via single-linkage. Implement rebuild_cache_for_note() and rebuild_full_cache() with incremental updates.
2. Implement FileWatcherService in file_watcher.rs using the notify crate. start_watching(dir) watches for .md create/modify/delete events debounced at 500ms. On create/modify: read file, extract title from front matter or filename, upsert via NoteService, trigger embedding. On delete: remove note. Implement scan_directory() for initial import. Add path canonicalization and directory traversal prevention (reject symlinks outside watch dir). Wire set_watch_directory IPC command with validation.
3. Hook note create/update/delete to trigger incremental similarity_cache rebuild via GraphService. Register Tauri IPC commands: get_latent_links, get_graph_data, set_watch_directory, get_settings, update_settings. Implement SettingsService with validation (threshold 0-1, debounce 100-2000ms, theme dark/light). Emit settings-changed events on update.
4. Create useLatentLinks(content, activeNoteId) hook sending content debounced at 300ms, listening for latent-link-pulse events. Build LatentLinksPanel.tsx: right-side panel showing related note titles with similarity %, clickable to navigate, with pulse/glow animation on new links. Install d3-force and d3-zoom. Create GraphCanvas.tsx: fetch via get_graph_data IPC, render SVG with d3-force simulation (300 initial iterations). Style nodes as glassmorphic circles (translucent fill, blur filter, white border), edges with opacity proportional to weight, color-coded by cluster.
5. Add graph interactions: click node to navigate, drag to pin/reposition, scroll to zoom, pan on background drag. Cull offscreen nodes. Add Cmd/Ctrl+G toggle for graph overlay/split view with animated transition. Build SettingsView.tsx with directory picker, threshold slider, debounce slider, theme toggle, and reindex button with progress bar. Write tests for LinkService LRU cache, GraphService clustering, and FileWatcher traversal prevention. Validate: `cargo test && npm run typecheck`.

**Validation**
`cd sunder && cargo test && npm run typecheck && npx vitest run`

---

### Task 5: Security, Logging, Packaging & CI/CD

**Role:** Expert DevOps & Security Engineer
**Goal:** Harden security, add logging, configure cross-platform packaging, and set up CI/CD with auto-updater

**Context**
Harden the app with Tauri CSP, FTS5 input sanitization, and zero network permissions. Configure structured logging with tracing, file rotation, and frontend error forwarding. Set up Tauri bundler for macOS (universal .dmg), Windows (.msi), Linux (.AppImage). Configure GitHub Actions CI/CD for cross-platform builds, code signing, notarization, and auto-updater.

**Files to Create**
- .github/workflows/ci.yml
- .github/workflows/release.yml
- src-tauri/src/logging.rs
- src-tauri/icons/icon.png

**Files to Modify**
- src-tauri/tauri.conf.json
- src-tauri/src/main.rs
- src-tauri/Cargo.toml
- src/components/AppShell.tsx

**Steps**
1. Configure Tauri CSP in tauri.conf.json: default-src 'self', script-src 'self', style-src 'self' 'unsafe-inline', img-src 'self' asset: https://asset.localhost, connect-src ipc: http://ipc.localhost. Set zero network permissions in capability manifest. Restrict IPC to only defined commands. Ensure sanitize_fts_query() from SearchService is applied to all FTS5 queries (already implemented in Task 2, verify coverage).
2. Create logging.rs: configure tracing with tracing-subscriber and tracing-appender. Write to {app_data_dir}/logs/sunder.log with daily rotation keeping 7 days. Default level: info. Add #[tracing::instrument] to all IPC command handlers for entry/exit spans with duration. Add performance metrics: embedding_duration_ms (debug), search_duration_ms (info), note_count (info at startup). Register log_frontend_error IPC command. Add window.onerror and unhandledrejection handler in AppShell forwarding to backend.
3. Configure Tauri bundler in tauri.conf.json: macOS — universal binary (aarch64 + x86_64), .dmg output, bundle identifier 'com.sunder.app', app icon. Windows — .msi with publisher metadata and icon. Linux — .AppImage with desktop entry. Add the @tauri-apps/plugin-updater for auto-update: check on startup, show non-intrusive notification, install on user action. Configure updater endpoint pointing to GitHub Releases.
4. Create .github/workflows/ci.yml: trigger on push to main and PRs. Matrix: macos-latest, windows-latest, ubuntu-latest. Steps: checkout, setup Rust toolchain, setup Node 20, npm ci, cargo clippy, cargo test, npm run typecheck, npx vitest run, cargo tauri build. Create .github/workflows/release.yml: trigger on tag v*. Same matrix + upload artifacts to GitHub Releases. Add macOS code signing with Developer ID certificate (secrets: APPLE_CERTIFICATE, APPLE_CERTIFICATE_PASSWORD, APPLE_ID, APPLE_TEAM_ID) and notarization via notarytool.
5. Write E2E test plan covering: create note → appears in sidebar, type content → latent link pulse within 600ms, keyword and paraphrase search, graph node click opens editor, set watch directory imports .md files, delete note removes from everywhere. Write performance benchmark test seeding 1000 notes verifying: cold start <2s, search <100ms, latent link <300ms. Validate entire project: `cargo clippy && cargo test && npm run typecheck && npx vitest run && cargo tauri build`.

**Validation**
`cd sunder && cargo clippy -- -D warnings && cargo test && npm run typecheck && npx vitest run`
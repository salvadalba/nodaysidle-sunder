# Architecture Requirements Document

## 🧱 System Overview
Sunder is a local-first, cross-platform desktop application built with Tauri 2 that provides a markdown-based research hub with on-device semantic search and automatic latent link discovery. The frontend is a Vite 6 + React + TypeScript SPA communicating with a Rust backend via Tauri IPC. All data lives in a local SQLite database with FTS5 and vector extensions. A bundled quantized ONNX embedding model runs in-process in Rust to generate note embeddings, enabling sub-100ms semantic search and real-time Latent Link graph updates without any network calls.

## 🏗 Architecture Style
Monolithic desktop application with a two-process architecture: a Rust core process (Tauri backend) handling data storage, indexing, embedding generation, and vector search, and a webview renderer process (Vite/React frontend) handling UI, canvas rendering, and user interaction. Communication flows through Tauri's typed IPC command/event bridge.

## 🎨 Frontend Architecture
- **Framework:** Vite 6 with React 19 and TypeScript. Single-page application rendered inside the Tauri webview. Component tree split into three zones: collapsible sidebar (file tree, search), central markdown editor (CodeMirror 6), and Latent Link graph canvas overlay.
- **State Management:** React context plus useReducer for global app state (active note, sidebar visibility, graph mode). Local component state for editor content and drag positions. Tauri event listeners push backend updates (new embeddings, link discoveries) into state via a lightweight event bus. No external state library needed given the single-user, single-window scope.
- **Routing:** Minimal client-side routing. React Router with three flat routes: /editor (default note editing view), /graph (full-screen Latent Link explorer), and /settings. Deep linking not required for a desktop app. Route state stored in memory, not URL hash.
- **Build Tooling:** Vite 6 as dev server and bundler. Tailwind CSS 4 for utility-first styling including glassmorphism effects (backdrop-blur, bg-opacity, border-opacity). PostCSS for Tailwind processing. TypeScript strict mode. Tauri CLI for building the final app bundle per platform.

## 🧠 Backend Architecture
- **Approach:** Rust binary compiled by Tauri 2 as the app backend. All logic runs as Tauri commands invoked from the frontend via IPC. No HTTP server, no REST API, no network listener. The Rust layer owns the SQLite database, embedding model runtime, file system watcher, and vector search. Commands are thin async functions that delegate to service modules.
- **API Style:** Tauri IPC commands (invoke) for request/response patterns and Tauri events (emit/listen) for real-time push from backend to frontend. Commands are typed with serde-serializable structs. No REST, no GraphQL, no HTTP.
- **Services:**
- NoteService: CRUD operations for markdown notes stored in SQLite. Handles import from filesystem, content parsing, and metadata extraction.
- EmbeddingService: Loads a quantized ONNX embedding model via the ort (ONNX Runtime) Rust crate. Generates 384-dimensional vectors from note content. Runs inference on a background thread pool to avoid blocking the UI.
- SearchService: Hybrid search combining FTS5 full-text queries with vector cosine similarity from the SQLite vector extension. Merges and ranks results using reciprocal rank fusion.
- LinkService: Computes semantic similarity between the current editor buffer and all stored embeddings. Emits Tauri events to the frontend when similarity exceeds a configurable threshold, powering the Latent Link pulse.
- FileWatcherService: Uses notify crate to watch a user-configured directory of markdown files. Detects creates, updates, and deletes. Triggers re-indexing through EmbeddingService.
- GraphService: Builds and caches the adjacency data for the Latent Link graph. Computes clusters using simple threshold-based grouping on the vector similarity matrix. Returns node and edge data optimized for frontend rendering.

## 🗄 Data Layer
- **Primary Store:** SQLite via rusqlite with two extensions compiled in: FTS5 for full-text search and sqlite-vec for vector similarity search. Single database file stored in the platform-appropriate app data directory. Notes table stores markdown content, metadata, and timestamps. Embeddings table stores vector blobs keyed by note ID. FTS5 virtual table mirrors note content for keyword search.
- **Relationships:** Notes are the single core entity. Relationships are computed at query time via vector similarity rather than stored as explicit foreign keys. A similarity_cache table stores precomputed pairwise similarities above a threshold to accelerate graph rendering. This cache is invalidated and rebuilt incrementally when notes are added or updated.
- **Migrations:** Schema migrations managed via a migrations table with version numbers. Rust code runs migrations on startup before any other database access. Migrations are embedded in the binary as SQL strings using include_str!. Forward-only migrations with no rollback support to keep complexity minimal.

## ☁️ Infrastructure
- **Hosting:** No hosting. Fully local desktop application distributed as platform-native installers: .dmg for macOS, .msi for Windows, .AppImage for Linux. Built via Tauri CLI bundler. Auto-updater uses Tauri's built-in updater plugin pointing to a static file host for update manifests.
- **Scaling Strategy:** Single-user, single-machine. Performance scaling handled through SQLite indexing, background thread pools for embedding generation, incremental re-indexing (only changed notes), and similarity cache precomputation. The vector search uses approximate nearest neighbor via sqlite-vec HNSW index for corpora exceeding 10,000 notes.
- **CI/CD:** GitHub Actions with three platform runners (macOS, Windows, Ubuntu). Build pipeline: lint (clippy + eslint), test (cargo test + vitest), build (tauri build per platform), sign (macOS codesign, Windows signtool), and publish (GitHub Releases with Tauri updater JSON manifest).

## ⚖️ Key Trade-offs
- Chose sqlite-vec over sqlite-vss because sqlite-vec is actively maintained, has a simpler build process, and compiles cleanly with rusqlite without requiring external C++ dependencies.
- Bundling a quantized ONNX model (~30MB) increases install size beyond the 10MB target for the app binary alone, but avoids any network dependency. The model is shipped as a sidecar asset, keeping the core binary under 10MB.
- Using React with a Canvas/SVG hybrid for the graph (via a lightweight library like d3-force for layout) instead of WebGL keeps complexity low and avoids shader debugging, at the cost of potential frame drops above ~2,000 visible nodes.
- Precomputing a similarity cache trades disk space and write-time computation for fast graph load times. This is acceptable because notes change infrequently relative to reads.
- Running the embedding model in-process in Rust (via ort crate) rather than as a sidecar process simplifies IPC and deployment but means a model crash could take down the app. Mitigated by running inference on a separate thread with panic catching.
- Chose FTS5 + vector hybrid search with reciprocal rank fusion over a single search modality. Adds implementation complexity but dramatically improves search quality for both keyword-exact and semantic-fuzzy queries.
- Watching an existing user directory of markdown files rather than managing internal storage reduces lock-in and aligns with the target audience's existing workflows, but requires handling external file mutations gracefully.

## 📐 Non-Functional Requirements
- All data must remain on the local filesystem. Zero network calls for any core functionality. The app must function fully offline.
- Semantic search must return results in under 100ms for a corpus of up to 50,000 notes.
- Latent Link graph must update within 300ms of the user pausing typing (debounced at 300ms).
- Cold startup must complete in under 2 seconds on an SSD-equipped machine.
- UI must render at 60fps during node dragging, graph animations, and sidebar transitions.
- App binary (excluding embedding model sidecar) must remain under 10MB.
- Must produce native builds for macOS (Apple Silicon and Intel universal binary), Windows 10+, and Ubuntu 22.04+.
- All Tauri IPC commands must complete within 50ms for non-search operations to keep the UI responsive.
- Embedding re-indexing must run incrementally in the background without blocking the editor or search.
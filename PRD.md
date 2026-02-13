# Sunder

## 🎯 Product Vision
A local-first, markdown-based research hub that uses an on-device vector database to surface hidden connections between your notes—without ever sending data to the cloud. Sunder replaces manual tagging with automatic semantic linking, showing you relationships you didn't know existed.

## ❓ Problem Statement
Researchers, writers, and knowledge workers accumulate thousands of markdown notes across months and years. Existing tools rely on manual tags, backlinks, or keyword search to connect ideas—all of which fail when the same concept is expressed with different words. Cloud-based AI solutions address this but require uploading private research data to third-party servers. There is no local-first tool that performs real-time semantic similarity detection across a personal knowledge base while keeping all data on-device.

## 🎯 Goals
- Provide a fully offline, local-first desktop app that never transmits user data over the network
- Automatically detect semantic relationships between notes without requiring manual tags or backlinks
- Surface latent connections in real-time as the user types via the Latent Link graph view
- Deliver sub-100ms semantic search across tens of thousands of markdown notes using on-device SQLite with vector extensions
- Ship a single codebase that produces native builds for macOS, Windows, and Linux under 10MB bundle size
- Present a polished glassmorphism UI with a collapsible sidebar and a fluid, non-linear node-dragging canvas

## 🚫 Non-Goals
- Cloud sync, server-side storage, or any network-dependent functionality
- Real-time collaboration or multi-user editing
- Mobile (iOS/Android) support in the initial release
- Plugin or extension system for third-party integrations
- Support for non-markdown file formats such as PDF, DOCX, or rich text
- AI-powered content generation or auto-completion of notes
- Web browser version of the application

## 👥 Target Users
- Independent researchers and academics who maintain large personal knowledge bases in markdown
- Writers and journalists who collect notes, sources, and drafts over long periods and need to rediscover buried connections
- Privacy-conscious knowledge workers who refuse to upload sensitive research to cloud services
- Zettelkasten and PKM practitioners looking for automatic link discovery beyond manual backlinks

## 🧩 Core Features
- [object Object]
- [object Object]
- [object Object]
- [object Object]
- [object Object]
- [object Object]
- [object Object]

## ⚙️ Non-Functional Requirements
- App bundle size must remain under 10MB excluding the bundled embedding model
- Semantic search queries must return results in under 100ms for a corpus of 50,000 notes
- Latent Link graph must update within 300ms of the user pausing typing
- Cold startup time must be under 2 seconds on a machine with an SSD
- All data must remain on the local filesystem at all times—zero network calls for core functionality
- The UI must render at 60fps during node dragging and graph animations
- Must build and run natively on macOS (Apple Silicon and Intel), Windows 10+, and Ubuntu 22.04+
- The app must function fully offline with no degradation of features

## 📊 Success Metrics
- Users discover at least one previously unknown connection between notes within their first 10 minutes of use
- Semantic search returns relevant results that keyword-only search misses in at least 30% of queries
- 90th percentile search latency stays under 100ms for corpora up to 50,000 notes
- App bundle size remains under 10MB (excluding embedding model)
- User retention: 40% of users who import notes return to the app within 7 days
- Latent Link graph interaction (clicking a pulsing node) occurs in at least 50% of editing sessions

## 📌 Assumptions
- A lightweight embedding model (e.g., quantized ONNX model under 50MB) can run on-device with acceptable quality for semantic similarity
- SQLite with vector extensions compiled via rusqlite provides sufficient vector search performance without a dedicated vector database
- Users already have their notes in markdown format or are willing to convert them
- The Tauri 2 framework provides stable cross-platform window management, file system access, and auto-update capabilities
- Vite 6 with React and Tailwind CSS 4 can deliver 60fps canvas interactions with a glassmorphism aesthetic
- FTS5 combined with vector similarity provides a good-enough hybrid search without a dedicated search engine
- Users are comfortable installing a native desktop application and do not require a browser-based alternative

## ❓ Open Questions
- Which on-device embedding model should be bundled—and should it be included in the installer or downloaded on first launch to keep bundle size under 10MB?
- Should the vector extension be sqlite-vss, sqlite-vec, or a custom Rust implementation using rusqlite UDFs?
- How should the Latent Link graph handle scaling when a user has thousands of semantically related notes—cluster, paginate, or threshold?
- What canvas library should power the node graph on the frontend—a React-based library like ReactFlow, or a lower-level WebGL/Canvas2D approach for performance?
- Should Sunder watch an existing directory of markdown files, or manage its own internal storage format with export capabilities?
- How should embedding re-indexing be handled when the bundled model is updated in a new app version—full re-index or versioned embeddings?
- What is the licensing model—open source, freemium, or one-time purchase?
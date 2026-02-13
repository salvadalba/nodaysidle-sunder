use crate::db::DatabaseManager;
use crate::error::SunderError;
use crate::services::embedding::EmbeddingService;
use crate::services::graph::GraphService;
use crate::services::note::NoteService;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::Emitter;

pub struct FileWatcherService {
    db: Arc<DatabaseManager>,
    embedding_service: Arc<EmbeddingService>,
    watcher: Mutex<Option<WatcherState>>,
}

struct WatcherState {
    _watcher: RecommendedWatcher,
    watch_dir: PathBuf,
}

impl FileWatcherService {
    pub fn new(db: Arc<DatabaseManager>, embedding_service: Arc<EmbeddingService>) -> Self {
        Self {
            db,
            embedding_service,
            watcher: Mutex::new(None),
        }
    }

    /// Start watching a directory for .md file changes.
    pub fn start_watching(
        &self,
        directory: &str,
        app_handle: tauri::AppHandle,
    ) -> Result<(), SunderError> {
        let dir = PathBuf::from(directory);

        // Validate directory
        if !dir.exists() {
            return Err(SunderError::DirectoryNotFound(directory.to_string()));
        }
        if !dir.is_dir() {
            return Err(SunderError::NotADirectory(directory.to_string()));
        }

        let canonical_dir = dir
            .canonicalize()
            .map_err(|e| SunderError::IoError(format!("Cannot canonicalize path: {e}")))?;

        // Stop existing watcher if any
        self.stop_watching()?;

        let db = Arc::clone(&self.db);
        let emb = Arc::clone(&self.embedding_service);
        let watch_dir = canonical_dir.clone();

        // Create a channel-based watcher with debounce
        let (tx, rx) = mpsc::channel::<Result<Event, notify::Error>>();

        let mut watcher = notify::recommended_watcher(tx)
            .map_err(|e| SunderError::Internal(format!("Failed to create watcher: {e}")))?;

        watcher
            .watch(&canonical_dir, RecursiveMode::Recursive)
            .map_err(|e| SunderError::Internal(format!("Failed to watch directory: {e}")))?;

        // Spawn event handler thread
        let handle = app_handle.clone();
        std::thread::spawn(move || {
            // Simple debounce: collect events for 500ms then process
            let mut pending_paths: std::collections::HashSet<PathBuf> =
                std::collections::HashSet::new();
            let mut last_event = std::time::Instant::now();

            loop {
                match rx.recv_timeout(Duration::from_millis(500)) {
                    Ok(Ok(event)) => {
                        if should_process_event(&event, &watch_dir) {
                            for path in &event.paths {
                                if is_markdown_file(path) && is_safe_path(path, &watch_dir) {
                                    pending_paths.insert(path.clone());
                                }
                            }
                            last_event = std::time::Instant::now();
                        }
                    }
                    Ok(Err(e)) => {
                        tracing::error!("Watch error: {e}");
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        // Process pending paths if enough time has passed
                        if !pending_paths.is_empty()
                            && last_event.elapsed() >= Duration::from_millis(500)
                        {
                            for path in pending_paths.drain() {
                                process_file_change(
                                    &path,
                                    &db,
                                    &emb,
                                    &handle,
                                );
                            }
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        tracing::info!("File watcher channel disconnected, stopping");
                        break;
                    }
                }
            }
        });

        // Store watcher state
        let mut state = self.watcher.lock().map_err(|e| {
            SunderError::Internal(format!("Watcher lock poisoned: {e}"))
        })?;
        *state = Some(WatcherState {
            _watcher: watcher,
            watch_dir: canonical_dir.clone(),
        });

        // Do initial scan
        self.scan_directory_internal(&canonical_dir, &app_handle)?;

        tracing::info!("Started watching: {}", canonical_dir.display());
        Ok(())
    }

    pub fn stop_watching(&self) -> Result<(), SunderError> {
        let mut state = self.watcher.lock().map_err(|e| {
            SunderError::Internal(format!("Watcher lock poisoned: {e}"))
        })?;

        if let Some(ws) = state.take() {
            tracing::info!("Stopped watching: {}", ws.watch_dir.display());
        }
        Ok(())
    }

    /// Scan a directory and import all .md files.
    pub fn scan_directory(
        &self,
        directory: &str,
        app_handle: &tauri::AppHandle,
    ) -> Result<u32, SunderError> {
        let dir = PathBuf::from(directory);
        if !dir.exists() {
            return Err(SunderError::DirectoryNotFound(directory.to_string()));
        }
        let canonical = dir
            .canonicalize()
            .map_err(|e| SunderError::IoError(format!("Cannot canonicalize: {e}")))?;

        self.scan_directory_internal(&canonical, app_handle)
    }

    fn scan_directory_internal(
        &self,
        dir: &Path,
        app_handle: &tauri::AppHandle,
    ) -> Result<u32, SunderError> {
        let note_service = NoteService::new(Arc::clone(&self.db));
        let mut imported = 0u32;

        let entries = walk_md_files(dir)?;
        let total = entries.len() as u32;

        for (idx, path) in entries.iter().enumerate() {
            match import_md_file(path, &note_service, &self.db) {
                Ok(true) => {
                    imported += 1;

                    // Index embedding for imported note
                    if let Ok(Some(note)) =
                        note_service.get_note_by_file_path(&path.to_string_lossy())
                    {
                        if note.content.split_whitespace().count() >= 3 {
                            let _ = self.embedding_service.index_note(&note.id, &note.content);
                            let graph_svc =
                                GraphService::new(Arc::clone(&self.db), Arc::clone(&self.embedding_service));
                            let _ = graph_svc.rebuild_cache_for_note(&note.id);
                        }
                    }
                }
                Ok(false) => {} // Skipped (already up-to-date)
                Err(e) => {
                    tracing::warn!("Failed to import {}: {e}", path.display());
                }
            }

            let _ = app_handle.emit(
                "file-change",
                serde_json::json!({
                    "type": "scan-progress",
                    "processed": idx + 1,
                    "total": total,
                }),
            );
        }

        tracing::info!("Scan complete: {imported}/{total} files imported from {}", dir.display());
        Ok(imported)
    }
}

/// Check if file event should be processed.
fn should_process_event(event: &Event, _watch_dir: &Path) -> bool {
    matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}

fn is_markdown_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext == "md" || ext == "markdown")
}

/// Ensure path is within watch directory and not a symlink escape.
fn is_safe_path(path: &Path, watch_dir: &Path) -> bool {
    match path.canonicalize() {
        Ok(canonical) => canonical.starts_with(watch_dir),
        Err(_) => {
            // File may have been deleted, check parent
            path.starts_with(watch_dir)
        }
    }
}

/// Process a single file change event.
fn process_file_change(
    path: &Path,
    db: &Arc<DatabaseManager>,
    emb: &Arc<EmbeddingService>,
    app_handle: &tauri::AppHandle,
) {
    let note_service = NoteService::new(Arc::clone(db));
    let path_str = path.to_string_lossy().to_string();

    if path.exists() {
        // Create or update
        match import_md_file(path, &note_service, db) {
            Ok(true) => {
                // Index embedding
                if let Ok(Some(note)) = note_service.get_note_by_file_path(&path_str) {
                    if note.content.split_whitespace().count() >= 3 {
                        let _ = emb.index_note(&note.id, &note.content);
                        let graph_svc = GraphService::new(Arc::clone(db), Arc::clone(emb));
                        let _ = graph_svc.rebuild_cache_for_note(&note.id);
                    }
                }

                let _ = app_handle.emit(
                    "file-change",
                    serde_json::json!({
                        "type": "updated",
                        "path": path_str,
                    }),
                );
            }
            Ok(false) => {} // No change needed
            Err(e) => tracing::warn!("Failed to import {}: {e}", path.display()),
        }
    } else {
        // File deleted — remove corresponding note
        if let Ok(Some(note)) = note_service.get_note_by_file_path(&path_str) {
            let _ = emb.remove_embedding(&note.id);
            // Don't call note_service.delete_note because that tries to delete the file too
            if let Ok(conn) = db.get_write_conn() {
                let _ = conn.execute("DELETE FROM notes WHERE id = ?1", [&note.id]);
            }

            let _ = app_handle.emit(
                "file-change",
                serde_json::json!({
                    "type": "deleted",
                    "path": path_str,
                    "note_id": note.id,
                }),
            );
        }
    }
}

/// Import a single .md file. Returns Ok(true) if imported, Ok(false) if skipped.
fn import_md_file(
    path: &Path,
    note_service: &NoteService,
    _db: &Arc<DatabaseManager>,
) -> Result<bool, SunderError> {
    let content = std::fs::read_to_string(path)?;
    let path_str = path.to_string_lossy().to_string();

    // Check if note already exists for this file
    let existing = note_service.get_note_by_file_path(&path_str)?;

    // Extract title from YAML front matter or filename
    let (title, body) = extract_front_matter(&content, path);

    if let Some(note) = existing {
        // Skip if content hasn't changed
        if note.content == body && note.title == title {
            return Ok(false);
        }

        // Update existing note
        note_service.update_note(
            &note.id,
            Some(title),
            Some(body),
        )?;
        Ok(true)
    } else {
        // Create new note
        note_service.create_note(title, body, Some(path_str))?;
        Ok(true)
    }
}

/// Extract title from YAML front matter, falling back to filename.
fn extract_front_matter(content: &str, path: &Path) -> (String, String) {
    let fallback_title = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Untitled".to_string());

    if !content.starts_with("---") {
        return (fallback_title, content.to_string());
    }

    // Find closing ---
    if let Some(end_idx) = content[3..].find("---") {
        let front_matter = &content[3..3 + end_idx];
        let body = content[3 + end_idx + 3..].trim_start().to_string();

        // Parse YAML front matter for title
        if let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(front_matter) {
            if let Some(title) = yaml.get("title").and_then(|v| v.as_str()) {
                let title = title.trim().to_string();
                if !title.is_empty() {
                    return (title, body);
                }
            }
        }

        (fallback_title, body)
    } else {
        (fallback_title, content.to_string())
    }
}

/// Recursively walk directory for .md files.
fn walk_md_files(dir: &Path) -> Result<Vec<PathBuf>, SunderError> {
    let mut files = Vec::new();
    walk_md_files_recursive(dir, &mut files)?;
    files.sort();
    Ok(files)
}

fn walk_md_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), SunderError> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        // Skip hidden files/dirs
        if path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with('.'))
        {
            continue;
        }

        if path.is_dir() {
            walk_md_files_recursive(&path, files)?;
        } else if is_markdown_file(&path) {
            files.push(path);
        }
    }
    Ok(())
}

pub mod db;
pub mod error;
pub mod services;

use db::DatabaseManager;
use error::SunderError;
use services::embedding::EmbeddingService;
use services::file_watcher::FileWatcherService;
use services::graph::{GraphData, GraphService};
use services::link::{LatentLink, LinkService};
use services::note::{Note, NoteList, NoteService};
use services::search::{SearchMode, SearchResult, SearchService};
use services::settings::{Settings, SettingsPatch, SettingsService};
use std::sync::Arc;
use tauri::Emitter;
use tauri::Manager;
use tauri::State;

// --- Tauri State Wrapper ---

pub struct AppState {
    pub note_service: NoteService,
    pub settings_service: SettingsService,
    pub embedding_service: Arc<EmbeddingService>,
    pub search_service: SearchService,
    pub link_service: LinkService,
    pub graph_service: GraphService,
    pub file_watcher_service: FileWatcherService,
    pub db: Arc<DatabaseManager>,
}

// --- Tauri IPC Commands ---

#[tauri::command]
fn create_note(
    state: State<'_, AppState>,
    title: String,
    content: String,
    file_path: Option<String>,
) -> Result<Note, SunderError> {
    let note = state.note_service.create_note(title, content, file_path)?;

    // Index embedding + rebuild graph cache in background
    if note.content.split_whitespace().count() >= 3 {
        let emb = Arc::clone(&state.embedding_service);
        let db = Arc::clone(&state.db);
        let note_id = note.id.clone();
        let note_content = note.content.clone();
        std::thread::spawn(move || {
            if let Err(e) = emb.index_note(&note_id, &note_content) {
                tracing::error!("Failed to index note {}: {}", note_id, e);
                return;
            }
            let graph_svc = GraphService::new(Arc::clone(&db), Arc::clone(&emb));
            if let Err(e) = graph_svc.rebuild_cache_for_note(&note_id) {
                tracing::error!("Failed to rebuild graph cache for {}: {}", note_id, e);
            }
        });
    }

    Ok(note)
}

#[tauri::command]
fn get_note(state: State<'_, AppState>, id: String) -> Result<Note, SunderError> {
    state.note_service.get_note(&id)
}

#[tauri::command]
fn update_note(
    state: State<'_, AppState>,
    id: String,
    title: Option<String>,
    content: Option<String>,
) -> Result<Note, SunderError> {
    let note = state.note_service.update_note(&id, title, content)?;

    // Re-index embedding + rebuild graph cache in background
    if note.content.split_whitespace().count() >= 3 {
        let emb = Arc::clone(&state.embedding_service);
        let db = Arc::clone(&state.db);
        let note_id = note.id.clone();
        let note_content = note.content.clone();
        std::thread::spawn(move || {
            if let Err(e) = emb.index_note(&note_id, &note_content) {
                tracing::error!("Failed to re-index note {}: {}", note_id, e);
                return;
            }
            let graph_svc = GraphService::new(Arc::clone(&db), Arc::clone(&emb));
            if let Err(e) = graph_svc.rebuild_cache_for_note(&note_id) {
                tracing::error!("Failed to rebuild graph cache for {}: {}", note_id, e);
            }
        });
    }

    Ok(note)
}

#[tauri::command]
fn delete_note(state: State<'_, AppState>, id: String) -> Result<(), SunderError> {
    // Remove embedding first
    let _ = state.embedding_service.remove_embedding(&id);
    state.note_service.delete_note(&id)
}

#[tauri::command]
fn list_notes(
    state: State<'_, AppState>,
    offset: Option<u32>,
    limit: Option<u32>,
    sort_by: Option<String>,
) -> Result<NoteList, SunderError> {
    state.note_service.list_notes(
        offset.unwrap_or(0),
        limit.unwrap_or(50),
        &sort_by.unwrap_or_else(|| "updated_at".to_string()),
    )
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Result<Settings, SunderError> {
    state.settings_service.get_settings()
}

#[tauri::command]
fn update_settings(
    state: State<'_, AppState>,
    settings: SettingsPatch,
) -> Result<(), SunderError> {
    state.settings_service.update_settings(settings)
}

#[tauri::command]
fn reindex_all(
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<serde_json::Value, SunderError> {
    let emb = Arc::clone(&state.embedding_service);
    let handle = app_handle.clone();

    std::thread::spawn(move || {
        let result = emb.reindex_all(|processed, total, title| {
            let _ = handle.emit(
                "indexing-progress",
                serde_json::json!({
                    "processed": processed,
                    "total": total,
                    "current_note_title": title,
                }),
            );
        });
        match result {
            Ok(count) => tracing::info!("Reindex complete: {count} notes indexed"),
            Err(e) => tracing::error!("Reindex failed: {e}"),
        }
    });

    // Return immediately — progress is reported via events
    Ok(serde_json::json!({ "queued": true }))
}

#[tauri::command]
fn search_notes(
    state: State<'_, AppState>,
    query: String,
    mode: Option<SearchMode>,
    limit: Option<u32>,
) -> Result<Vec<SearchResult>, SunderError> {
    state.search_service.search(
        &query,
        &mode.unwrap_or(SearchMode::Hybrid),
        limit.unwrap_or(20),
    )
}

#[tauri::command]
fn get_latent_links(
    state: State<'_, AppState>,
    content: String,
    exclude_note_id: Option<String>,
    threshold: Option<f64>,
    limit: Option<u32>,
) -> Result<Vec<LatentLink>, SunderError> {
    state.link_service.compute_latent_links(
        &content,
        exclude_note_id.as_deref(),
        threshold.unwrap_or(0.3),
        limit.unwrap_or(5),
    )
}

#[tauri::command]
fn get_graph_data(
    state: State<'_, AppState>,
    center_note_id: Option<String>,
    threshold: Option<f64>,
) -> Result<GraphData, SunderError> {
    state.graph_service.get_graph(
        center_note_id.as_deref(),
        threshold.unwrap_or(0.3),
    )
}

#[tauri::command]
fn rebuild_graph_cache(state: State<'_, AppState>) -> Result<u32, SunderError> {
    state.graph_service.rebuild_full_cache()
}

#[tauri::command]
fn set_watch_directory(
    state: State<'_, AppState>,
    directory: String,
    app_handle: tauri::AppHandle,
) -> Result<(), SunderError> {
    state
        .file_watcher_service
        .start_watching(&directory, app_handle)
}

#[tauri::command]
fn stop_watching(state: State<'_, AppState>) -> Result<(), SunderError> {
    state.file_watcher_service.stop_watching()
}

#[tauri::command]
fn scan_directory(
    state: State<'_, AppState>,
    directory: String,
    app_handle: tauri::AppHandle,
) -> Result<u32, SunderError> {
    state
        .file_watcher_service
        .scan_directory(&directory, &app_handle)
}

#[tauri::command]
fn log_frontend_error(level: String, message: String, context: Option<String>) {
    match level.as_str() {
        "error" => tracing::error!(context = context, "[frontend] {}", message),
        "warn" => tracing::warn!(context = context, "[frontend] {}", message),
        "info" => tracing::info!(context = context, "[frontend] {}", message),
        _ => tracing::debug!(context = context, "[frontend] {}", message),
    }
}

// --- App Builder ---

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to resolve app data directory");

            let db = Arc::new(
                DatabaseManager::initialize(&app_data_dir)
                    .expect("Failed to initialize database"),
            );

            // Resolve resource directory for ONNX model
            let resource_dir = app
                .path()
                .resource_dir()
                .expect("Failed to resolve resource directory")
                .join("resources");

            let note_service = NoteService::new(Arc::clone(&db));
            let settings_service = SettingsService::new(Arc::clone(&db));

            let embedding_service = Arc::new(
                EmbeddingService::new(&resource_dir, Arc::clone(&db))
                    .expect("Failed to initialize embedding service"),
            );

            let search_service =
                SearchService::new(Arc::clone(&db), Arc::clone(&embedding_service));
            let link_service =
                LinkService::new(Arc::clone(&db), Arc::clone(&embedding_service));
            let graph_service =
                GraphService::new(Arc::clone(&db), Arc::clone(&embedding_service));
            let file_watcher_service =
                FileWatcherService::new(Arc::clone(&db), Arc::clone(&embedding_service));

            app.manage(AppState {
                note_service,
                settings_service,
                embedding_service,
                search_service,
                link_service,
                graph_service,
                file_watcher_service,
                db,
            });

            tracing::info!("Sunder initialized successfully");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create_note,
            get_note,
            update_note,
            delete_note,
            list_notes,
            get_settings,
            update_settings,
            reindex_all,
            search_notes,
            get_latent_links,
            get_graph_data,
            rebuild_graph_cache,
            set_watch_directory,
            stop_watching,
            scan_directory,
            log_frontend_error,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

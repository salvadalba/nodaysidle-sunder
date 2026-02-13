import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// --- Data Types ---

export interface Note {
  id: string;
  title: string;
  content: string;
  file_path: string | null;
  word_count: number;
  created_at: string;
  updated_at: string;
}

export interface NoteListItem {
  id: string;
  title: string;
  snippet: string;
  updated_at: string;
}

export interface NoteList {
  notes: NoteListItem[];
  total: number;
}

export interface SearchResult {
  id: string;
  title: string;
  snippet: string;
  score: number;
  match_type: "fulltext" | "semantic" | "both";
}

export interface LatentLink {
  note_id: string;
  title: string;
  similarity: number;
  snippet: string;
}

export interface GraphNode {
  id: string;
  title: string;
  x: number;
  y: number;
  cluster: number;
}

export interface GraphEdge {
  source: string;
  target: string;
  weight: number;
}

export interface GraphData {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

export interface Settings {
  watch_directory: string | null;
  similarity_threshold: number;
  debounce_ms: number;
  theme: "dark" | "light";
}

export interface SettingsPatch {
  similarity_threshold?: number;
  debounce_ms?: number;
  theme?: "dark" | "light";
}

// --- Error Types ---

export interface SunderError {
  kind: string;
  message?: string;
}

// --- Event Payloads ---

export interface LatentLinkPulsePayload {
  links: Array<{
    note_id: string;
    title: string;
    similarity: number;
  }>;
}

export interface IndexingProgressPayload {
  processed: number;
  total: number;
  current_note_title: string;
}

export interface FileChangePayload {
  event_type: "created" | "modified" | "deleted";
  file_path: string;
  note_id: string | null;
}

// --- Typed IPC Wrappers ---

export const ipc = {
  createNote: (title: string, content: string, filePath?: string) =>
    invoke<Note>("create_note", { title, content, filePath }),

  getNote: (id: string) => invoke<Note>("get_note", { id }),

  updateNote: (id: string, title?: string, content?: string) =>
    invoke<Note>("update_note", { id, title, content }),

  deleteNote: (id: string) => invoke<void>("delete_note", { id }),

  listNotes: (offset?: number, limit?: number, sortBy?: string) =>
    invoke<NoteList>("list_notes", { offset, limit, sortBy }),

  searchNotes: (query: string, mode?: "hybrid" | "fulltext" | "semantic", limit?: number) =>
    invoke<SearchResult[]>("search_notes", { query, mode, limit }),

  getLatentLinks: (content: string, excludeNoteId?: string, threshold?: number, limit?: number) =>
    invoke<LatentLink[]>("get_latent_links", { content, excludeNoteId, threshold, limit }),

  getGraphData: (centerNoteId?: string, threshold?: number) =>
    invoke<GraphData>("get_graph_data", { centerNoteId, threshold }),

  rebuildGraphCache: () => invoke<number>("rebuild_graph_cache"),

  setWatchDirectory: (directory: string) =>
    invoke<void>("set_watch_directory", { directory }),

  stopWatching: () => invoke<void>("stop_watching"),

  scanDirectory: (directory: string) =>
    invoke<number>("scan_directory", { directory }),

  getSettings: () => invoke<Settings>("get_settings"),

  updateSettings: (settings: SettingsPatch) => invoke<void>("update_settings", { settings }),

  reindexAll: () => invoke<{ queued: boolean }>("reindex_all"),

  logFrontendError: (level: string, message: string, context?: string) =>
    invoke<void>("log_frontend_error", { level, message, context }),
};

// --- Typed Event Listeners ---

export const events = {
  onLatentLinkPulse: (callback: (payload: LatentLinkPulsePayload) => void): Promise<UnlistenFn> =>
    listen<LatentLinkPulsePayload>("latent-link-pulse", (event) => callback(event.payload)),

  onIndexingProgress: (
    callback: (payload: IndexingProgressPayload) => void,
  ): Promise<UnlistenFn> =>
    listen<IndexingProgressPayload>("indexing-progress", (event) => callback(event.payload)),

  onFileChange: (callback: (payload: FileChangePayload) => void): Promise<UnlistenFn> =>
    listen<FileChangePayload>("file-change", (event) => callback(event.payload)),
};

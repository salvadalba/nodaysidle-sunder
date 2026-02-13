import { useReducer, useEffect, useRef, createContext, useContext, type Dispatch } from "react";
import { Sidebar } from "./Sidebar";
import { EditorView } from "./EditorView";
import { LatentLinksPanel } from "./LatentLinksPanel";
import { GraphCanvas } from "./GraphCanvas";
import { SettingsView } from "./SettingsView";

// --- State ---

interface AppState {
  activeNoteId: string | null;
  sidebarOpen: boolean;
  graphVisible: boolean;
  settingsOpen: boolean;
  theme: "dark" | "light";
  editorContent: string;
}

type AppAction =
  | { type: "SET_ACTIVE_NOTE"; id: string | null }
  | { type: "TOGGLE_SIDEBAR" }
  | { type: "TOGGLE_GRAPH" }
  | { type: "TOGGLE_SETTINGS" }
  | { type: "SET_THEME"; theme: "dark" | "light" }
  | { type: "SET_EDITOR_CONTENT"; content: string };

const initialState: AppState = {
  activeNoteId: null,
  sidebarOpen: true,
  graphVisible: false,
  settingsOpen: false,
  theme: "dark",
  editorContent: "",
};

function appReducer(state: AppState, action: AppAction): AppState {
  switch (action.type) {
    case "SET_ACTIVE_NOTE":
      return { ...state, activeNoteId: action.id, editorContent: "", settingsOpen: false };
    case "TOGGLE_SIDEBAR":
      return { ...state, sidebarOpen: !state.sidebarOpen };
    case "TOGGLE_GRAPH":
      return { ...state, graphVisible: !state.graphVisible };
    case "TOGGLE_SETTINGS":
      return { ...state, settingsOpen: !state.settingsOpen };
    case "SET_THEME":
      return { ...state, theme: action.theme };
    case "SET_EDITOR_CONTENT":
      return { ...state, editorContent: action.content };
    default:
      return state;
  }
}

// --- Context ---

interface AppContextValue {
  state: AppState;
  dispatch: Dispatch<AppAction>;
}

const AppStateContext = createContext<AppContextValue | null>(null);

export function useAppState() {
  const ctx = useContext(AppStateContext);
  if (!ctx) throw new Error("useAppState must be used within AppShell");
  return ctx;
}

// --- Shell ---

export function AppShell() {
  const [state, dispatch] = useReducer(appReducer, initialState);
  const dispatchRef = useRef(dispatch);
  dispatchRef.current = dispatch;

  // Apply theme to document root
  useEffect(() => {
    document.documentElement.setAttribute("data-theme", state.theme);
  }, [state.theme]);

  // Global keyboard shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;
      if (!mod) return;

      if (e.key === "b") {
        e.preventDefault();
        dispatchRef.current({ type: "TOGGLE_SIDEBAR" });
      } else if (e.key === "g") {
        e.preventDefault();
        dispatchRef.current({ type: "TOGGLE_GRAPH" });
      } else if (e.key === "n") {
        e.preventDefault();
        dispatchRef.current({ type: "SET_ACTIVE_NOTE", id: "new" });
      } else if (e.key === "k") {
        e.preventDefault();
        const searchInput = document.querySelector<HTMLInputElement>('[aria-label="Search notes"]');
        searchInput?.focus();
      } else if (e.key === ",") {
        e.preventDefault();
        dispatchRef.current({ type: "TOGGLE_SETTINGS" });
      }
    };

    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  return (
    <AppStateContext.Provider value={{ state, dispatch }}>
      <div className="relative flex h-screen w-screen overflow-hidden" style={{ zIndex: 1 }}>
        {/* Sidebar */}
        <Sidebar />

        {/* Main Content Area */}
        <main
          role="main"
          aria-label="Editor"
          className="relative flex min-w-0 flex-1 flex-col"
        >
          {/* Top bar */}
          <TopBar />

          {/* Content area: Editor, Graph, or Settings */}
          <div className="flex min-h-0 flex-1">
            {state.settingsOpen ? (
              <div className="min-w-0 flex-1">
                <SettingsView />
              </div>
            ) : state.graphVisible ? (
              <>
                {/* Split view: editor left, graph right */}
                {state.activeNoteId ? (
                  <>
                    <div className="min-w-0 flex-1">
                      <EditorView noteId={state.activeNoteId} />
                    </div>
                    <div
                      className="w-[45%] shrink-0 border-l"
                      style={{ borderColor: "var(--color-border-subtle)" }}
                    >
                      <GraphCanvas />
                    </div>
                  </>
                ) : (
                  <div className="min-w-0 flex-1">
                    <GraphCanvas />
                  </div>
                )}
              </>
            ) : (
              <>
                <div className="min-w-0 flex-1">
                  {state.activeNoteId ? (
                    <EditorView noteId={state.activeNoteId} />
                  ) : (
                    <EmptyState />
                  )}
                </div>

                {/* Latent Links Panel */}
                <LatentLinksPanel />
              </>
            )}
          </div>
        </main>
      </div>
    </AppStateContext.Provider>
  );
}

// --- Top Bar ---

function TopBar() {
  const { state, dispatch } = useAppState();

  return (
    <header
      className="flex h-11 shrink-0 items-center gap-3 border-b px-3"
      style={{
        borderColor: "var(--color-border-subtle)",
        background: "oklch(0.1 0.015 270 / 0.6)",
        backdropFilter: "blur(12px)",
      }}
    >
      {/* Sidebar toggle */}
      <button
        onClick={() => dispatch({ type: "TOGGLE_SIDEBAR" })}
        className="focus-ring flex h-7 w-7 items-center justify-center rounded-md transition-colors hover:bg-[var(--color-surface-3)]"
        aria-label="Toggle sidebar"
        title="Toggle sidebar (Cmd+B)"
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
          <rect x="1" y="2" width="14" height="12" rx="2" stroke="var(--color-ink-tertiary)" strokeWidth="1.2" />
          <line x1="5.5" y1="2" x2="5.5" y2="14" stroke="var(--color-ink-tertiary)" strokeWidth="1.2" />
        </svg>
      </button>

      {/* Breadcrumb / note title */}
      <div className="flex min-w-0 flex-1 items-center gap-2">
        <span
          className="font-display text-[11px] font-600 tracking-wide uppercase"
          style={{ color: "var(--color-ink-ghost)" }}
        >
          Sunder
        </span>
        {state.settingsOpen ? (
          <>
            <span style={{ color: "var(--color-ink-ghost)" }}>/</span>
            <span className="text-[12px]" style={{ color: "var(--color-ink-secondary)" }}>
              Settings
            </span>
          </>
        ) : state.activeNoteId ? (
          <>
            <span style={{ color: "var(--color-ink-ghost)" }}>/</span>
            <span
              className="truncate text-[12px]"
              style={{ color: "var(--color-ink-secondary)" }}
            >
              {state.activeNoteId === "new" ? "New Note" : "Note"}
            </span>
          </>
        ) : null}
      </div>

      {/* Actions */}
      <div className="flex items-center gap-1">
        {/* Graph toggle */}
        <button
          onClick={() => dispatch({ type: "TOGGLE_GRAPH" })}
          className={`focus-ring flex h-7 items-center gap-1.5 rounded-md px-2 text-[11px] font-500 transition-all ${
            state.graphVisible
              ? "bg-[var(--color-violet-soft)] text-[var(--color-violet)]"
              : "text-[var(--color-ink-tertiary)] hover:bg-[var(--color-surface-3)] hover:text-[var(--color-ink-secondary)]"
          }`}
          title="Toggle graph (Cmd+G)"
        >
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
            <circle cx="4" cy="4" r="1.8" stroke="currentColor" strokeWidth="1.2" />
            <circle cx="10" cy="5" r="1.8" stroke="currentColor" strokeWidth="1.2" />
            <circle cx="6" cy="10" r="1.8" stroke="currentColor" strokeWidth="1.2" />
            <line x1="5.5" y1="5" x2="8.5" y2="4.5" stroke="currentColor" strokeWidth="0.8" opacity="0.5" />
            <line x1="5" y1="5.5" x2="5.5" y2="8.5" stroke="currentColor" strokeWidth="0.8" opacity="0.5" />
          </svg>
          Graph
        </button>

        {/* Settings toggle */}
        <button
          onClick={() => dispatch({ type: "TOGGLE_SETTINGS" })}
          className={`focus-ring flex h-7 w-7 items-center justify-center rounded-md transition-colors ${
            state.settingsOpen
              ? "bg-[var(--color-violet-soft)] text-[var(--color-violet)]"
              : "text-[var(--color-ink-tertiary)] hover:bg-[var(--color-surface-3)] hover:text-[var(--color-ink-secondary)]"
          }`}
          title="Settings (Cmd+,)"
          aria-label="Settings"
        >
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
            <circle cx="7" cy="7" r="2" stroke="currentColor" strokeWidth="1.2" />
            <path d="M7 1v2M7 11v2M1 7h2M11 7h2M2.8 2.8l1.4 1.4M9.8 9.8l1.4 1.4M2.8 11.2l1.4-1.4M9.8 4.2l1.4-1.4" stroke="currentColor" strokeWidth="1" strokeLinecap="round" />
          </svg>
        </button>
      </div>
    </header>
  );
}

// --- Empty State ---

function EmptyState() {
  const { dispatch } = useAppState();

  return (
    <div className="flex h-full items-center justify-center p-8">
      <div
        className="flex max-w-sm flex-col items-center text-center"
        style={{ animation: "fade-in 0.5s ease-out" }}
      >
        {/* Abstract mark */}
        <div className="relative mb-6 h-16 w-20">
          <div
            className="absolute left-0 top-0 h-16 w-16 rounded-full"
            style={{ border: "1.5px solid var(--color-ink-ghost)", opacity: 0.5 }}
          />
          <div
            className="absolute right-0 top-0 h-16 w-16 rounded-full"
            style={{ border: "1.5px solid var(--color-amber)", opacity: 0.35 }}
          />
          <div
            className="absolute left-1/2 top-1/2 h-4 w-4 -translate-x-1/2 -translate-y-1/2 rounded-full"
            style={{ background: "var(--color-amber)", opacity: 0.15, filter: "blur(6px)" }}
          />
        </div>

        <h2
          className="font-display text-lg font-600 tracking-tight"
          style={{ color: "var(--color-ink)" }}
        >
          No note selected
        </h2>
        <p
          className="mt-2 text-[13px] leading-relaxed"
          style={{ color: "var(--color-ink-tertiary)" }}
        >
          Select a note from the sidebar, or create a new one to begin mapping your research.
        </p>
        <button
          onClick={() => dispatch({ type: "SET_ACTIVE_NOTE", id: "new" })}
          className="focus-ring mt-5 flex h-8 items-center gap-2 rounded-lg px-4 text-[12px] font-500 transition-all hover:brightness-110"
          style={{
            background: "linear-gradient(135deg, var(--color-violet) 0%, oklch(0.58 0.2 280) 100%)",
            color: "#fff",
            boxShadow: "0 2px 8px oklch(0.5 0.18 290 / 0.25)",
          }}
        >
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
            <line x1="6" y1="1" x2="6" y2="11" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
            <line x1="1" y1="6" x2="11" y2="6" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
          </svg>
          New Note
        </button>
      </div>
    </div>
  );
}

import { useState, useRef, useEffect, useCallback } from "react";
import { useAppState } from "./AppShell";
import { useNoteList } from "../hooks/useNoteList";
import { useSearch } from "../hooks/useSearch";
import { ipc, events } from "../types";

export function Sidebar() {
  const { state, dispatch } = useAppState();
  const [searchQuery, setSearchQuery] = useState("");
  const [focusedIndex, setFocusedIndex] = useState(-1);
  const searchRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const { notes, total, isLoading, refresh } = useNoteList();
  const { results: searchResults, isSearching } = useSearch(searchQuery);

  const isOpen = state.sidebarOpen;
  const isSearchActive = searchQuery.trim().length > 0;

  // Listen for file-change events to refresh note list
  useEffect(() => {
    const unlisten = events.onFileChange(() => {
      refresh();
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [refresh]);

  // Displayed items: search results or note list
  const displayItems = isSearchActive
    ? searchResults.map((r) => ({
        id: r.id,
        title: r.title,
        snippet: r.snippet,
        updated_at: "",
        matchType: r.match_type,
      }))
    : notes.map((n) => ({
        id: n.id,
        title: n.title,
        snippet: n.snippet,
        updated_at: n.updated_at,
        matchType: undefined as string | undefined,
      }));

  const selectNote = useCallback(
    (id: string) => {
      dispatch({ type: "SET_ACTIVE_NOTE", id });
    },
    [dispatch],
  );

  const handleCreateNote = useCallback(async () => {
    try {
      const note = await ipc.createNote("Untitled", "");
      dispatch({ type: "SET_ACTIVE_NOTE", id: note.id });
      refresh();
    } catch {
      // Will be logged by frontend error handler
    }
  }, [dispatch, refresh]);

  // Keyboard navigation
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setFocusedIndex((i) => Math.min(i + 1, displayItems.length - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setFocusedIndex((i) => Math.max(i - 1, 0));
      } else if (e.key === "Enter" && focusedIndex >= 0) {
        e.preventDefault();
        selectNote(displayItems[focusedIndex].id);
      } else if (e.key === "Escape") {
        setSearchQuery("");
        setFocusedIndex(-1);
        searchRef.current?.blur();
      }
    },
    [focusedIndex, displayItems, selectNote],
  );

  // Scroll focused item into view
  useEffect(() => {
    if (focusedIndex >= 0 && listRef.current) {
      const items = listRef.current.querySelectorAll("[data-note-item]");
      items[focusedIndex]?.scrollIntoView({ block: "nearest" });
    }
  }, [focusedIndex]);

  // Relative time formatting
  const formatTime = (iso: string) => {
    if (!iso) return "";
    const diff = Date.now() - new Date(iso).getTime();
    const days = Math.floor(diff / 86400000);
    if (days === 0) return "Today";
    if (days === 1) return "Yesterday";
    if (days < 7) return `${days}d ago`;
    return `${Math.floor(days / 7)}w ago`;
  };

  return (
    <nav
      role="navigation"
      aria-label="Notes sidebar"
      className="relative flex shrink-0 flex-col overflow-hidden border-r transition-all duration-200 ease-out"
      style={{
        width: isOpen ? 272 : 0,
        borderColor: isOpen ? "var(--color-border-subtle)" : "transparent",
        background: "var(--color-surface-0)",
      }}
    >
      {/* Inner container prevents content squish */}
      <div className="flex w-[272px] flex-col" style={{ height: "100%" }}>
        {/* Header */}
        <div className="flex h-11 shrink-0 items-center justify-between border-b px-3" style={{ borderColor: "var(--color-border-subtle)" }}>
          <span
            className="font-display text-[13px] font-600 tracking-tight"
            style={{ color: "var(--color-ink-secondary)" }}
          >
            Notes
          </span>
          <button
            onClick={handleCreateNote}
            className="focus-ring flex h-6 w-6 items-center justify-center rounded-md transition-colors hover:bg-[var(--color-surface-3)]"
            aria-label="New note"
            title="New note (Cmd+N)"
          >
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
              <line x1="7" y1="2" x2="7" y2="12" stroke="var(--color-ink-tertiary)" strokeWidth="1.3" strokeLinecap="round" />
              <line x1="2" y1="7" x2="12" y2="7" stroke="var(--color-ink-tertiary)" strokeWidth="1.3" strokeLinecap="round" />
            </svg>
          </button>
        </div>

        {/* Search */}
        <div className="shrink-0 px-2.5 pt-2.5 pb-1">
          <div
            className="group flex h-8 items-center gap-2 rounded-lg px-2.5 transition-all"
            style={{
              background: "var(--color-surface-2)",
              border: "1px solid transparent",
            }}
          >
            <svg width="13" height="13" viewBox="0 0 13 13" fill="none" className="shrink-0">
              <circle cx="5.5" cy="5.5" r="4" stroke="var(--color-ink-ghost)" strokeWidth="1.2" />
              <line x1="8.5" y1="8.5" x2="11.5" y2="11.5" stroke="var(--color-ink-ghost)" strokeWidth="1.2" strokeLinecap="round" />
            </svg>
            <input
              ref={searchRef}
              type="text"
              value={searchQuery}
              onChange={(e) => {
                setSearchQuery(e.target.value);
                setFocusedIndex(-1);
              }}
              onKeyDown={handleKeyDown}
              placeholder="Search notes..."
              aria-label="Search notes"
              className="h-full min-w-0 flex-1 bg-transparent text-[12px] font-400 outline-none placeholder:text-[var(--color-ink-ghost)]"
              style={{ color: "var(--color-ink)" }}
            />
            {searchQuery && (
              <button
                onClick={() => {
                  setSearchQuery("");
                  searchRef.current?.focus();
                }}
                className="flex h-4 w-4 items-center justify-center rounded-sm hover:bg-[var(--color-surface-4)]"
              >
                <svg width="8" height="8" viewBox="0 0 8 8" fill="none">
                  <line x1="1" y1="1" x2="7" y2="7" stroke="var(--color-ink-ghost)" strokeWidth="1.2" strokeLinecap="round" />
                  <line x1="7" y1="1" x2="1" y2="7" stroke="var(--color-ink-ghost)" strokeWidth="1.2" strokeLinecap="round" />
                </svg>
              </button>
            )}
          </div>
        </div>

        {/* Note List */}
        <div
          ref={listRef}
          className="flex-1 overflow-y-auto px-2 py-1"
          onKeyDown={handleKeyDown}
          role="listbox"
          aria-label="Notes"
        >
          {isLoading && !isSearchActive ? (
            <div className="flex flex-col items-center py-12 text-center">
              <p className="text-[12px]" style={{ color: "var(--color-ink-ghost)" }}>
                Loading...
              </p>
            </div>
          ) : displayItems.length === 0 ? (
            <div className="flex flex-col items-center py-12 text-center">
              <p className="text-[12px]" style={{ color: "var(--color-ink-ghost)" }}>
                {isSearchActive
                  ? isSearching
                    ? "Searching..."
                    : "No matching notes"
                  : "No notes yet"}
              </p>
            </div>
          ) : (
            displayItems.map((item, idx) => {
              const isActive = state.activeNoteId === item.id;
              const isFocused = focusedIndex === idx;

              return (
                <button
                  key={item.id}
                  data-note-item
                  role="option"
                  aria-selected={isActive}
                  onClick={() => selectNote(item.id)}
                  className={`focus-ring group relative mb-0.5 flex w-full flex-col rounded-lg px-2.5 py-2 text-left transition-all duration-150 ${
                    isFocused && !isActive ? "ring-1 ring-[var(--color-violet)] ring-opacity-30" : ""
                  }`}
                  style={{
                    background: isActive
                      ? "var(--color-surface-3)"
                      : isFocused
                        ? "var(--color-surface-2)"
                        : "transparent",
                    animation: `fade-in 0.3s ease-out ${idx * 0.04}s both`,
                  }}
                  onMouseEnter={() => setFocusedIndex(idx)}
                >
                  {/* Active indicator */}
                  {isActive && (
                    <div
                      className="absolute left-0 top-2 bottom-2 w-[2px] rounded-full"
                      style={{
                        background: "var(--color-amber)",
                        boxShadow: "0 0 6px var(--color-amber-glow)",
                      }}
                    />
                  )}

                  {/* Title + match type badge */}
                  <div className="flex items-center gap-1.5">
                    <span
                      className="truncate text-[12.5px] font-500 leading-snug"
                      style={{
                        color: isActive ? "var(--color-ink)" : "var(--color-ink-secondary)",
                      }}
                    >
                      {item.title}
                    </span>
                    {item.matchType && (
                      <span
                        className="shrink-0 rounded-full px-1.5 py-px text-[8px] font-600 uppercase tracking-wider"
                        style={{
                          background:
                            item.matchType === "both"
                              ? "var(--color-amber-soft)"
                              : item.matchType === "semantic"
                                ? "var(--color-violet-soft)"
                                : "var(--color-surface-4)",
                          color:
                            item.matchType === "both"
                              ? "var(--color-amber)"
                              : item.matchType === "semantic"
                                ? "var(--color-violet)"
                                : "var(--color-ink-tertiary)",
                        }}
                      >
                        {item.matchType}
                      </span>
                    )}
                  </div>

                  {/* Meta row */}
                  <div className="mt-0.5 flex items-center gap-2">
                    <span
                      className="truncate text-[11px] font-300"
                      style={{ color: "var(--color-ink-ghost)", flex: 1 }}
                    >
                      {item.snippet}
                    </span>
                    {item.updated_at && (
                      <span
                        className="shrink-0 text-[10px] font-400 tabular-nums"
                        style={{ color: "var(--color-ink-ghost)" }}
                      >
                        {formatTime(item.updated_at)}
                      </span>
                    )}
                  </div>
                </button>
              );
            })
          )}
        </div>

        {/* Footer: note count */}
        <div
          className="flex h-8 shrink-0 items-center border-t px-3"
          style={{ borderColor: "var(--color-border-subtle)" }}
        >
          <span className="text-[10px] font-400 tabular-nums" style={{ color: "var(--color-ink-ghost)" }}>
            {isSearchActive
              ? `${displayItems.length} result${displayItems.length !== 1 ? "s" : ""}`
              : `${total} note${total !== 1 ? "s" : ""}`}
          </span>
        </div>
      </div>
    </nav>
  );
}

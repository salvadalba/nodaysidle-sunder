import { useState, useEffect, useRef, useCallback } from "react";
import { useAppState } from "./AppShell";
import { useNote } from "../hooks/useNote";
import { useAutoSave } from "../hooks/useAutoSave";
import { ipc } from "../types";
import { EditorState } from "@codemirror/state";
import { EditorView as CMEditorView, keymap, placeholder } from "@codemirror/view";
import { markdown } from "@codemirror/lang-markdown";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { syntaxHighlighting, HighlightStyle, bracketMatching } from "@codemirror/language";
import { tags } from "@lezer/highlight";

// --- CodeMirror theme matching Deep Cartography design system ---

const sunderTheme = CMEditorView.theme({
  "&": {
    backgroundColor: "transparent",
    color: "var(--color-ink)",
    height: "100%",
  },
  ".cm-scroller": {
    fontFamily: "var(--font-mono)",
    fontSize: "13.5px",
    lineHeight: "1.75",
    padding: "1.25rem 0 5rem",
    overflow: "auto",
  },
  ".cm-content": {
    padding: "0 2rem",
    maxWidth: "72ch",
    caretColor: "var(--color-amber)",
  },
  ".cm-cursor": {
    borderLeftColor: "var(--color-amber)",
    borderLeftWidth: "1.5px",
  },
  ".cm-activeLine": {
    backgroundColor: "oklch(0.2 0.02 270 / 0.15)",
  },
  ".cm-selectionBackground": {
    backgroundColor: "oklch(0.55 0.15 290 / 0.2) !important",
  },
  "&.cm-focused .cm-selectionBackground": {
    backgroundColor: "oklch(0.55 0.15 290 / 0.25) !important",
  },
  ".cm-gutters": {
    backgroundColor: "transparent",
    border: "none",
    color: "var(--color-ink-ghost)",
  },
  ".cm-activeLineGutter": {
    backgroundColor: "transparent",
    color: "var(--color-ink-tertiary)",
  },
  ".cm-line": {
    padding: "0",
  },
  "&.cm-focused": {
    outline: "none",
  },
  ".cm-placeholder": {
    color: "var(--color-ink-ghost)",
    fontStyle: "italic",
  },
});

const sunderHighlight = HighlightStyle.define([
  { tag: tags.heading1, color: "var(--color-ink)", fontWeight: "700", fontSize: "1.5em" },
  { tag: tags.heading2, color: "var(--color-ink)", fontWeight: "600", fontSize: "1.3em" },
  { tag: tags.heading3, color: "var(--color-ink)", fontWeight: "600", fontSize: "1.15em" },
  { tag: tags.heading4, color: "var(--color-ink-secondary)", fontWeight: "600" },
  { tag: tags.strong, color: "var(--color-ink)", fontWeight: "600" },
  { tag: tags.emphasis, color: "var(--color-ink-secondary)", fontStyle: "italic" },
  { tag: tags.link, color: "var(--color-violet)", textDecoration: "underline" },
  { tag: tags.url, color: "var(--color-violet)", opacity: "0.7" },
  { tag: tags.monospace, color: "var(--color-amber)", fontFamily: "var(--font-mono)" },
  { tag: tags.quote, color: "var(--color-ink-tertiary)", fontStyle: "italic" },
  { tag: tags.strikethrough, color: "var(--color-ink-ghost)", textDecoration: "line-through" },
  { tag: tags.meta, color: "var(--color-ink-ghost)" },
  { tag: tags.processingInstruction, color: "var(--color-teal)" },
]);

// --- Component ---

interface EditorViewProps {
  noteId: string;
}

export function EditorView({ noteId }: EditorViewProps) {
  const { dispatch } = useAppState();
  const { note, isLoading, error } = useNote(noteId === "new" ? null : noteId);
  const containerRef = useRef<HTMLDivElement>(null);
  const cmViewRef = useRef<CMEditorView | null>(null);
  const [content, setContent] = useState("");
  const [title, setTitle] = useState("");
  const [isNewNote, setIsNewNote] = useState(false);
  // Track whether we're programmatically updating CM to avoid feedback loop
  const isExternalUpdate = useRef(false);

  const saveState = useAutoSave(
    isNewNote ? null : noteId,
    title || undefined,
    content,
  );

  // Handle content changes from CodeMirror
  const handleChange = useCallback(
    (newContent: string) => {
      if (isExternalUpdate.current) return;
      setContent(newContent);
      dispatch({ type: "SET_EDITOR_CONTENT", content: newContent });
    },
    [dispatch],
  );

  // Create CodeMirror instance
  useEffect(() => {
    if (!containerRef.current) return;

    const updateListener = CMEditorView.updateListener.of((update) => {
      if (update.docChanged) {
        handleChange(update.state.doc.toString());
      }
    });

    const state = EditorState.create({
      doc: "",
      extensions: [
        sunderTheme,
        syntaxHighlighting(sunderHighlight),
        markdown(),
        history(),
        bracketMatching(),
        CMEditorView.lineWrapping,
        placeholder("Start writing..."),
        keymap.of([...defaultKeymap, ...historyKeymap]),
        updateListener,
      ],
    });

    const view = new CMEditorView({
      state,
      parent: containerRef.current,
    });

    cmViewRef.current = view;

    return () => {
      view.destroy();
      cmViewRef.current = null;
    };
  }, [handleChange]);

  // Load note content when note loads
  useEffect(() => {
    if (noteId === "new") {
      setContent("");
      setTitle("Untitled");
      setIsNewNote(true);
      // Clear CM content
      if (cmViewRef.current) {
        isExternalUpdate.current = true;
        const doc = cmViewRef.current.state.doc;
        cmViewRef.current.dispatch({
          changes: { from: 0, to: doc.length, insert: "" },
        });
        isExternalUpdate.current = false;
        // Focus editor for new notes
        setTimeout(() => cmViewRef.current?.focus(), 100);
      }
      return;
    }

    if (note) {
      setTitle(note.title);
      setIsNewNote(false);
      // Sync CM with loaded note content
      if (cmViewRef.current) {
        const currentDoc = cmViewRef.current.state.doc.toString();
        if (currentDoc !== note.content) {
          isExternalUpdate.current = true;
          cmViewRef.current.dispatch({
            changes: { from: 0, to: cmViewRef.current.state.doc.length, insert: note.content },
          });
          isExternalUpdate.current = false;
        }
      }
      setContent(note.content);
    }
  }, [noteId, note]);

  const handleDelete = useCallback(async () => {
    if (!noteId || noteId === "new") return;
    try {
      await ipc.deleteNote(noteId);
      dispatch({ type: "SET_ACTIVE_NOTE", id: null });
    } catch {
      // Error handled by IPC layer
    }
  }, [noteId, dispatch]);

  // Handle new note creation on first edit
  useEffect(() => {
    if (!isNewNote || content.split(/\s+/).filter(Boolean).length < 1) return;

    const createTimer = setTimeout(async () => {
      try {
        const newNote = await ipc.createNote(title, content);
        dispatch({ type: "SET_ACTIVE_NOTE", id: newNote.id });
        setIsNewNote(false);
      } catch {
        // Will retry on next edit
      }
    }, 500);

    return () => clearTimeout(createTimer);
  }, [isNewNote, content, title, dispatch]);

  if (isLoading) {
    return (
      <div className="flex h-full items-center justify-center">
        <p style={{ color: "var(--color-ink-ghost)" }}>Loading...</p>
      </div>
    );
  }

  if (error && noteId !== "new") {
    return (
      <div className="flex h-full items-center justify-center">
        <p style={{ color: "var(--color-ink-ghost)" }}>Note not found</p>
      </div>
    );
  }

  const wordCount = content.split(/\s+/).filter(Boolean).length;
  const updatedAt = note?.updated_at;

  return (
    <div className="flex h-full flex-col" style={{ animation: "fade-in 0.25s ease-out" }}>
      {/* Note Header */}
      <div className="flex shrink-0 items-center gap-3 px-8 pt-5 pb-3">
        <div className="min-w-0 flex-1">
          <input
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            className="w-full bg-transparent font-display text-[22px] font-700 leading-tight tracking-[-0.02em] outline-none"
            style={{ color: "var(--color-ink)" }}
            placeholder="Untitled"
          />
          <div className="mt-1.5 flex items-center gap-3">
            <span className="text-[11px] font-400 tabular-nums" style={{ color: "var(--color-ink-ghost)" }}>
              {wordCount} words
            </span>
            <span style={{ color: "var(--color-ink-ghost)", fontSize: "8px" }}>·</span>
            <span className="text-[11px] font-400" style={{ color: "var(--color-ink-ghost)" }}>
              {updatedAt
                ? new Date(updatedAt).toLocaleDateString("en-US", {
                    month: "short",
                    day: "numeric",
                    year: "numeric",
                  })
                : "Just now"}
            </span>
            {/* Save indicator */}
            <span
              className="ml-auto text-[10px] font-500 uppercase tracking-wider transition-opacity duration-300"
              style={{
                color:
                  saveState === "saving"
                    ? "var(--color-amber)"
                    : saveState === "saved"
                      ? "var(--color-teal)"
                      : saveState === "error"
                        ? "oklch(0.65 0.2 25)"
                        : "transparent",
                opacity: saveState === "idle" ? 0 : 1,
              }}
            >
              {saveState === "saving"
                ? "Saving..."
                : saveState === "saved"
                  ? "Saved"
                  : saveState === "error"
                    ? "Error"
                    : ""}
            </span>
          </div>
        </div>

        {/* Delete */}
        {noteId !== "new" && (
          <button
            onClick={handleDelete}
            className="focus-ring flex h-7 w-7 items-center justify-center rounded-md transition-all hover:bg-[oklch(0.4_0.1_10/0.15)]"
            style={{ opacity: 0.4 }}
            title="Delete note"
            aria-label="Delete note"
          >
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
              <path
                d="M3 4h8l-.5 7.5a1 1 0 01-1 .9H4.5a1 1 0 01-1-.9L3 4z"
                stroke="var(--color-ink-ghost)"
                strokeWidth="1.1"
              />
              <path d="M2 4h10" stroke="var(--color-ink-ghost)" strokeWidth="1.1" strokeLinecap="round" />
              <path d="M5.5 2h3" stroke="var(--color-ink-ghost)" strokeWidth="1.1" strokeLinecap="round" />
            </svg>
          </button>
        )}
      </div>

      {/* Divider with subtle gradient */}
      <div className="mx-8">
        <div
          className="h-px"
          style={{
            background: "linear-gradient(90deg, var(--color-border-subtle) 0%, var(--color-border) 50%, var(--color-border-subtle) 100%)",
          }}
        />
      </div>

      {/* CodeMirror Editor */}
      <div className="relative min-h-0 flex-1">
        <div ref={containerRef} className="h-full" />

        {/* Bottom fade for scroll */}
        <div
          className="pointer-events-none absolute bottom-0 left-0 right-0 h-16"
          style={{
            background: "linear-gradient(transparent, var(--color-void))",
          }}
        />
      </div>
    </div>
  );
}

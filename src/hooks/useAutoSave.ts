import { useEffect, useRef, useState } from "react";
import { ipc } from "../types";

type SaveState = "idle" | "saving" | "saved" | "error";

export function useAutoSave(
  noteId: string | null,
  title: string | undefined,
  content: string,
  debounceMs: number = 1000,
): SaveState {
  const [saveState, setSaveState] = useState<SaveState>("idle");
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const prevContentRef = useRef(content);
  const prevTitleRef = useRef(title);

  useEffect(() => {
    if (!noteId || noteId === "new") return;
    if (content === prevContentRef.current && title === prevTitleRef.current) return;

    prevContentRef.current = content;
    prevTitleRef.current = title;

    setSaveState("saving");

    if (timeoutRef.current) clearTimeout(timeoutRef.current);

    timeoutRef.current = setTimeout(async () => {
      try {
        await ipc.updateNote(noteId, title, content);
        setSaveState("saved");
        setTimeout(() => setSaveState("idle"), 2000);
      } catch {
        setSaveState("error");
      }
    }, debounceMs);

    return () => {
      if (timeoutRef.current) clearTimeout(timeoutRef.current);
    };
  }, [noteId, title, content, debounceMs]);

  return saveState;
}

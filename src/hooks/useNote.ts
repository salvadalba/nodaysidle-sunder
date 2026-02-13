import { useState, useEffect, useCallback } from "react";
import type { Note } from "../types";
import { ipc } from "../types";

interface UseNoteReturn {
  note: Note | null;
  isLoading: boolean;
  error: string | null;
  refresh: () => void;
}

export function useNote(id: string | null): UseNoteReturn {
  const [note, setNote] = useState<Note | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchNote = useCallback(async () => {
    if (!id || id === "new") {
      setNote(null);
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      const result = await ipc.getNote(id);
      setNote(result);
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      setError(msg);
      setNote(null);
    } finally {
      setIsLoading(false);
    }
  }, [id]);

  useEffect(() => {
    fetchNote();
  }, [fetchNote]);

  return { note, isLoading, error, refresh: fetchNote };
}

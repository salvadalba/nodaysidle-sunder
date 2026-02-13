import { useState, useEffect, useCallback } from "react";
import type { NoteListItem } from "../types";
import { ipc } from "../types";

interface UseNoteListReturn {
  notes: NoteListItem[];
  total: number;
  isLoading: boolean;
  error: string | null;
  refresh: () => void;
  loadMore: () => void;
}

export function useNoteList(
  sortBy: string = "updated_at",
  limit: number = 50,
): UseNoteListReturn {
  const [notes, setNotes] = useState<NoteListItem[]>([]);
  const [total, setTotal] = useState(0);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [offset, setOffset] = useState(0);

  const fetchNotes = useCallback(
    async (fetchOffset: number, append: boolean) => {
      setIsLoading(true);
      setError(null);
      try {
        const result = await ipc.listNotes(fetchOffset, limit, sortBy);
        if (append) {
          setNotes((prev) => [...prev, ...result.notes]);
        } else {
          setNotes(result.notes);
        }
        setTotal(result.total);
      } catch (e: unknown) {
        const msg = e instanceof Error ? e.message : String(e);
        setError(msg);
      } finally {
        setIsLoading(false);
      }
    },
    [limit, sortBy],
  );

  useEffect(() => {
    setOffset(0);
    fetchNotes(0, false);
  }, [fetchNotes]);

  const loadMore = useCallback(() => {
    if (offset + limit >= total) return;
    const newOffset = offset + limit;
    setOffset(newOffset);
    fetchNotes(newOffset, true);
  }, [offset, limit, total, fetchNotes]);

  const refresh = useCallback(() => {
    setOffset(0);
    fetchNotes(0, false);
  }, [fetchNotes]);

  return { notes, total, isLoading, error, refresh, loadMore };
}

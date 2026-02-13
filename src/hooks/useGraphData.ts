import { useState, useEffect, useCallback } from "react";
import type { GraphData } from "../types";
import { ipc } from "../types";

interface UseGraphDataReturn {
  data: GraphData | null;
  isLoading: boolean;
  error: string | null;
  refresh: () => void;
}

export function useGraphData(
  centerNoteId?: string,
  threshold: number = 0.3,
): UseGraphDataReturn {
  const [data, setData] = useState<GraphData | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchGraph = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const result = await ipc.getGraphData(centerNoteId, threshold);
      setData(result);
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      setError(msg);
    } finally {
      setIsLoading(false);
    }
  }, [centerNoteId, threshold]);

  useEffect(() => {
    fetchGraph();
  }, [fetchGraph]);

  return { data, isLoading, error, refresh: fetchGraph };
}

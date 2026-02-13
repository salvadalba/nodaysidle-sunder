import { useState, useEffect, useRef } from "react";
import type { LatentLink } from "../types";
import { ipc } from "../types";

interface UseLatentLinksReturn {
  links: LatentLink[];
  isLoading: boolean;
}

export function useLatentLinks(
  content: string,
  noteId: string | null,
  threshold: number = 0.3,
  limit: number = 5,
): UseLatentLinksReturn {
  const [links, setLinks] = useState<LatentLink[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (!content || content.split(/\s+/).length < 5) {
      setLinks([]);
      return;
    }

    if (timeoutRef.current) clearTimeout(timeoutRef.current);

    timeoutRef.current = setTimeout(async () => {
      setIsLoading(true);
      try {
        const result = await ipc.getLatentLinks(
          content,
          noteId ?? undefined,
          threshold,
          limit,
        );
        setLinks(result);
      } catch {
        // Silently fail — latent links are non-critical
      } finally {
        setIsLoading(false);
      }
    }, 300);

    return () => {
      if (timeoutRef.current) clearTimeout(timeoutRef.current);
    };
  }, [content, noteId, threshold, limit]);

  return { links, isLoading };
}

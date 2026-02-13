import { useAppState } from "./AppShell";
import { useLatentLinks } from "../hooks/useLatentLinks";
import type { LatentLink } from "../types";

export function LatentLinksPanel() {
  const { state, dispatch } = useAppState();

  const { links, isLoading } = useLatentLinks(
    state.editorContent,
    state.activeNoteId,
  );

  const isVisible =
    state.activeNoteId &&
    state.activeNoteId !== "new" &&
    (links.length > 0 || isLoading);

  if (!isVisible) return null;

  return (
    <aside
      role="complementary"
      aria-label="Related notes"
      className="flex w-56 shrink-0 flex-col border-l"
      style={{
        borderColor: "var(--color-border-subtle)",
        background: "oklch(0.09 0.015 270 / 0.5)",
        animation: "slide-in-right 0.3s ease-out",
      }}
    >
      {/* Header */}
      <div className="flex h-11 shrink-0 items-center gap-2 border-b px-3" style={{ borderColor: "var(--color-border-subtle)" }}>
        {/* Pulsing indicator dot */}
        <div className="relative flex h-4 w-4 items-center justify-center">
          <div
            className="absolute h-2 w-2 rounded-full"
            style={{
              background: "var(--color-amber)",
              boxShadow: "0 0 8px var(--color-amber-glow)",
              animation: "pulse-amber 2.5s ease-in-out infinite",
            }}
          />
          <div
            className="h-1.5 w-1.5 rounded-full"
            style={{ background: "var(--color-amber)" }}
          />
        </div>
        <span
          className="text-[11px] font-500 uppercase tracking-wider"
          style={{ color: "var(--color-amber)" }}
        >
          Latent Links
        </span>
        <span
          className="ml-auto flex h-4 min-w-4 items-center justify-center rounded-full px-1 text-[9px] font-600 tabular-nums"
          style={{
            background: "var(--color-amber-soft)",
            color: "var(--color-amber)",
          }}
        >
          {links.length}
        </span>
      </div>

      {/* Links list */}
      <div className="flex-1 overflow-y-auto p-2">
        {isLoading && links.length === 0 ? (
          <div className="flex items-center justify-center py-8">
            <span className="text-[11px]" style={{ color: "var(--color-ink-ghost)" }}>
              Finding connections...
            </span>
          </div>
        ) : (
          links.map((link: LatentLink, idx: number) => (
            <button
              key={link.note_id}
              onClick={() => dispatch({ type: "SET_ACTIVE_NOTE", id: link.note_id })}
              className="latent-glow group mb-1.5 flex w-full flex-col rounded-lg px-3 py-2.5 text-left transition-all duration-200 hover:bg-[var(--color-surface-2)]"
              style={{
                animation: `fade-in 0.3s ease-out ${idx * 0.08}s both`,
              }}
            >
              {/* Similarity score */}
              <div className="mb-1 flex items-center gap-2">
                <SimilarityArc value={link.similarity} />
                <span
                  className="text-[10px] font-500 tabular-nums"
                  style={{
                    color:
                      link.similarity > 0.8
                        ? "var(--color-amber)"
                        : link.similarity > 0.65
                          ? "var(--color-ink-secondary)"
                          : "var(--color-ink-tertiary)",
                  }}
                >
                  {Math.round(link.similarity * 100)}%
                </span>
              </div>

              {/* Title */}
              <span
                className="text-[12px] font-500 leading-snug transition-colors group-hover:text-[var(--color-ink)]"
                style={{ color: "var(--color-ink-secondary)" }}
              >
                {link.title}
              </span>

              {/* Snippet */}
              <span
                className="mt-0.5 line-clamp-2 text-[10.5px] font-300 leading-relaxed"
                style={{ color: "var(--color-ink-ghost)" }}
              >
                {link.snippet}
              </span>
            </button>
          ))
        )}
      </div>

      {/* Footer hint */}
      <div
        className="flex shrink-0 items-center justify-center border-t py-2"
        style={{ borderColor: "var(--color-border-subtle)" }}
      >
        <span className="text-[9.5px] font-300 italic" style={{ color: "var(--color-ink-ghost)" }}>
          connections surface as you type
        </span>
      </div>
    </aside>
  );
}

// Similarity Arc: a tiny radial gauge

function SimilarityArc({ value }: { value: number }) {
  const radius = 6;
  const circumference = 2 * Math.PI * radius;
  const filled = circumference * value;

  return (
    <svg width="16" height="16" viewBox="0 0 16 16" className="shrink-0">
      {/* Background track */}
      <circle
        cx="8"
        cy="8"
        r={radius}
        fill="none"
        stroke="var(--color-surface-4)"
        strokeWidth="1.5"
      />
      {/* Filled arc */}
      <circle
        cx="8"
        cy="8"
        r={radius}
        fill="none"
        stroke={value > 0.8 ? "var(--color-amber)" : value > 0.65 ? "var(--color-violet)" : "var(--color-ink-tertiary)"}
        strokeWidth="1.5"
        strokeDasharray={`${filled} ${circumference}`}
        strokeDashoffset={circumference * 0.25}
        strokeLinecap="round"
        style={{ transition: "stroke-dasharray 0.5s ease-out" }}
      />
    </svg>
  );
}

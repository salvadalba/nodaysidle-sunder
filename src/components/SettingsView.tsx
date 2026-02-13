import { useState, useEffect, useCallback } from "react";
import { useAppState } from "./AppShell";
import { ipc, events } from "../types";
import type { Settings, IndexingProgressPayload } from "../types";
import { open } from "@tauri-apps/plugin-dialog";

export function SettingsView() {
  const { state, dispatch } = useAppState();
  const [, setSettings] = useState<Settings | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [watchDir, setWatchDir] = useState<string | null>(null);
  const [threshold, setThreshold] = useState(0.4);
  const [debounceMs, setDebounceMs] = useState(1000);
  const [indexingProgress, setIndexingProgress] = useState<IndexingProgressPayload | null>(null);
  const [isReindexing, setIsReindexing] = useState(false);
  const [scanResult, setScanResult] = useState<string | null>(null);

  // Load settings
  useEffect(() => {
    (async () => {
      try {
        const s = await ipc.getSettings();
        setSettings(s);
        setWatchDir(s.watch_directory);
        setThreshold(s.similarity_threshold);
        setDebounceMs(s.debounce_ms);
      } catch {
        // Settings service unavailable
      } finally {
        setIsLoading(false);
      }
    })();
  }, []);

  // Listen for indexing progress
  useEffect(() => {
    const unlisten = events.onIndexingProgress((payload) => {
      setIndexingProgress(payload);
      if (payload.processed >= payload.total) {
        setTimeout(() => {
          setIsReindexing(false);
          setIndexingProgress(null);
        }, 1000);
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const handlePickDirectory = useCallback(async () => {
    try {
      const selected = await open({ directory: true, multiple: false });
      if (selected && typeof selected === "string") {
        setWatchDir(selected);
        await ipc.setWatchDirectory(selected);
        const count = await ipc.scanDirectory(selected);
        setScanResult(`Imported ${count} note${count !== 1 ? "s" : ""}`);
        setTimeout(() => setScanResult(null), 3000);
      }
    } catch {
      // User cancelled or error
    }
  }, []);

  const handleStopWatching = useCallback(async () => {
    try {
      await ipc.stopWatching();
      setWatchDir(null);
    } catch {
      // Error handled
    }
  }, []);

  const handleThresholdChange = useCallback(
    async (value: number) => {
      setThreshold(value);
      try {
        await ipc.updateSettings({ similarity_threshold: value });
      } catch {
        // Error handled
      }
    },
    [],
  );

  const handleDebounceChange = useCallback(
    async (value: number) => {
      setDebounceMs(value);
      try {
        await ipc.updateSettings({ debounce_ms: value });
      } catch {
        // Error handled
      }
    },
    [],
  );

  const handleThemeToggle = useCallback(
    async (theme: "dark" | "light") => {
      dispatch({ type: "SET_THEME", theme });
      try {
        await ipc.updateSettings({ theme });
      } catch {
        // Error handled
      }
    },
    [dispatch],
  );

  const handleReindex = useCallback(async () => {
    setIsReindexing(true);
    setIndexingProgress(null);
    try {
      await ipc.reindexAll();
    } catch {
      setIsReindexing(false);
    }
  }, []);

  if (isLoading) {
    return (
      <div className="flex h-full items-center justify-center">
        <p style={{ color: "var(--color-ink-ghost)" }}>Loading settings...</p>
      </div>
    );
  }

  return (
    <div
      className="flex h-full flex-col overflow-y-auto"
      style={{ animation: "fade-in 0.3s ease-out" }}
    >
      <div className="mx-auto w-full max-w-lg px-8 pt-8 pb-20">
        {/* Header */}
        <h1
          className="font-display text-[22px] font-700 tracking-tight"
          style={{ color: "var(--color-ink)" }}
        >
          Settings
        </h1>
        <p className="mt-1 text-[13px]" style={{ color: "var(--color-ink-tertiary)" }}>
          Configure Sunder to match your research workflow.
        </p>

        {/* Watch Directory */}
        <Section title="Watch Directory" description="Sunder will auto-import and sync .md files from this directory.">
          <div className="flex items-center gap-2">
            <div
              className="flex min-w-0 flex-1 items-center rounded-lg px-3 py-2"
              style={{
                background: "var(--color-surface-2)",
                border: "1px solid var(--color-border-subtle)",
              }}
            >
              <span
                className="truncate text-[12px] font-mono"
                style={{ color: watchDir ? "var(--color-ink-secondary)" : "var(--color-ink-ghost)" }}
              >
                {watchDir || "No directory selected"}
              </span>
            </div>
            <button
              onClick={handlePickDirectory}
              className="focus-ring shrink-0 rounded-lg px-3 py-2 text-[12px] font-500 transition-colors"
              style={{
                background: "var(--color-surface-3)",
                color: "var(--color-ink-secondary)",
                border: "1px solid var(--color-border-subtle)",
              }}
            >
              {watchDir ? "Change" : "Browse"}
            </button>
            {watchDir && (
              <button
                onClick={handleStopWatching}
                className="focus-ring shrink-0 rounded-lg px-3 py-2 text-[12px] font-500 transition-colors hover:bg-[oklch(0.4_0.1_10/0.15)]"
                style={{ color: "var(--color-rose)" }}
              >
                Stop
              </button>
            )}
          </div>
          {scanResult && (
            <p className="mt-2 text-[11px] font-500" style={{ color: "var(--color-teal)" }}>
              {scanResult}
            </p>
          )}
        </Section>

        {/* Similarity Threshold */}
        <Section
          title="Similarity Threshold"
          description="Minimum similarity score for latent links and graph edges. Higher values show fewer, stronger connections."
        >
          <div className="flex items-center gap-4">
            <input
              type="range"
              min={0}
              max={1}
              step={0.05}
              value={threshold}
              onChange={(e) => handleThresholdChange(parseFloat(e.target.value))}
              className="slider flex-1"
              style={{ accentColor: "var(--color-amber)" }}
            />
            <span
              className="w-12 text-right font-mono text-[13px] font-500 tabular-nums"
              style={{ color: "var(--color-amber)" }}
            >
              {threshold.toFixed(2)}
            </span>
          </div>
        </Section>

        {/* Debounce */}
        <Section
          title="Auto-save Debounce"
          description="Delay before auto-saving after you stop typing."
        >
          <div className="flex items-center gap-4">
            <input
              type="range"
              min={100}
              max={3000}
              step={100}
              value={debounceMs}
              onChange={(e) => handleDebounceChange(parseInt(e.target.value, 10))}
              className="slider flex-1"
              style={{ accentColor: "var(--color-violet)" }}
            />
            <span
              className="w-16 text-right font-mono text-[13px] font-500 tabular-nums"
              style={{ color: "var(--color-violet)" }}
            >
              {debounceMs}ms
            </span>
          </div>
        </Section>

        {/* Theme */}
        <Section title="Theme" description="Choose your preferred appearance.">
          <div className="flex gap-2">
            {(["dark", "light"] as const).map((t) => (
              <button
                key={t}
                onClick={() => handleThemeToggle(t)}
                className="focus-ring flex-1 rounded-lg px-4 py-2.5 text-[12px] font-500 capitalize transition-all"
                style={{
                  background:
                    state.theme === t
                      ? "var(--color-violet-soft)"
                      : "var(--color-surface-2)",
                  color:
                    state.theme === t
                      ? "var(--color-violet)"
                      : "var(--color-ink-tertiary)",
                  border: `1px solid ${
                    state.theme === t
                      ? "var(--color-violet)"
                      : "var(--color-border-subtle)"
                  }`,
                }}
              >
                {t}
              </button>
            ))}
          </div>
        </Section>

        {/* Reindex */}
        <Section
          title="Reindex All Notes"
          description="Regenerate embeddings for all notes. Useful after updating the model or if search seems off."
        >
          <button
            onClick={handleReindex}
            disabled={isReindexing}
            className="focus-ring rounded-lg px-4 py-2.5 text-[12px] font-500 transition-all disabled:opacity-50"
            style={{
              background: isReindexing
                ? "var(--color-surface-2)"
                : "linear-gradient(135deg, var(--color-amber) 0%, oklch(0.72 0.15 50) 100%)",
              color: isReindexing ? "var(--color-ink-tertiary)" : "#0d0e16",
              boxShadow: isReindexing ? "none" : "0 2px 8px oklch(0.72 0.12 70 / 0.2)",
            }}
          >
            {isReindexing ? "Reindexing..." : "Reindex All"}
          </button>

          {/* Progress bar */}
          {indexingProgress && (
            <div className="mt-3">
              <div className="flex items-center justify-between text-[10px]">
                <span style={{ color: "var(--color-ink-ghost)" }}>
                  {indexingProgress.current_note_title}
                </span>
                <span className="font-mono tabular-nums" style={{ color: "var(--color-amber)" }}>
                  {indexingProgress.processed}/{indexingProgress.total}
                </span>
              </div>
              <div
                className="mt-1.5 h-1.5 overflow-hidden rounded-full"
                style={{ background: "var(--color-surface-3)" }}
              >
                <div
                  className="h-full rounded-full transition-all duration-300"
                  style={{
                    width: `${(indexingProgress.processed / indexingProgress.total) * 100}%`,
                    background: "linear-gradient(90deg, var(--color-amber), oklch(0.72 0.15 50))",
                    boxShadow: "0 0 8px var(--color-amber-glow)",
                  }}
                />
              </div>
            </div>
          )}
        </Section>

        {/* Keyboard Shortcuts */}
        <Section title="Keyboard Shortcuts" description="">
          <div className="flex flex-col gap-1.5">
            {[
              ["Cmd+B", "Toggle sidebar"],
              ["Cmd+N", "New note"],
              ["Cmd+G", "Toggle graph"],
              ["Cmd+K", "Focus search"],
              ["Cmd+,", "Settings"],
            ].map(([key, desc]) => (
              <div key={key} className="flex items-center justify-between py-1">
                <span className="text-[12px]" style={{ color: "var(--color-ink-secondary)" }}>
                  {desc}
                </span>
                <kbd
                  className="rounded-md px-2 py-0.5 font-mono text-[10px] font-500"
                  style={{
                    background: "var(--color-surface-3)",
                    color: "var(--color-ink-tertiary)",
                    border: "1px solid var(--color-border-subtle)",
                  }}
                >
                  {key}
                </kbd>
              </div>
            ))}
          </div>
        </Section>
      </div>
    </div>
  );
}

// --- Section helper ---

function Section({
  title,
  description,
  children,
}: {
  title: string;
  description: string;
  children: React.ReactNode;
}) {
  return (
    <div className="mt-8">
      <h3
        className="font-display text-[14px] font-600 tracking-tight"
        style={{ color: "var(--color-ink)" }}
      >
        {title}
      </h3>
      {description && (
        <p className="mt-1 text-[12px] leading-relaxed" style={{ color: "var(--color-ink-ghost)" }}>
          {description}
        </p>
      )}
      <div className="mt-3">{children}</div>
    </div>
  );
}

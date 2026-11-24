import { useEffect, useRef, useState } from "react";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { ArrowUpRight, Check, Copy, Search, Trash2 } from "lucide-react";
import { formatLanguagePair } from "../../lib/languages";
import { useGlossaryHistoryStore } from "../../stores/useGlossaryHistoryStore";
import { useNavigationStore } from "../../stores/useNavigationStore";
import { useTextTranslatorStore } from "../../stores/useTextTranslatorStore";

export function HistoryTab() {
  const searchInputRef = useRef<HTMLInputElement>(null);
  const [copiedId, setCopiedId] = useState<string | null>(null);

  const history = useGlossaryHistoryStore((s) => s.history);
  const historyQuery = useGlossaryHistoryStore((s) => s.historyQuery);
  const setHistoryQuery = useGlossaryHistoryStore((s) => s.setHistoryQuery);
  const fetchHistory = useGlossaryHistoryStore((s) => s.fetchHistory);
  const deleteHistoryItem = useGlossaryHistoryStore(
    (s) => s.deleteHistoryItem
  );

  const loadFromHistory = useTextTranslatorStore((s) => s.loadFromHistory);
  const setActivePage = useNavigationStore((s) => s.setActivePage);

  useEffect(() => {
    void fetchHistory();
  }, [fetchHistory]);

  // Ctrl+F focuses search bar
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "f") {
        e.preventDefault();
        searchInputRef.current?.focus();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  const handleCopy = async (id: string, text: string) => {
    try {
      await writeText(text);
    } catch {
      await navigator.clipboard?.writeText(text);
    }
    setCopiedId(id);
    setTimeout(() => setCopiedId(null), 1500);
  };

  return (
    <div className="space-y-4">
      <div className="relative">
        <Search className="pointer-events-none absolute left-3.5 top-3 h-4 w-4 text-text-muted" />
        <input
          ref={searchInputRef}
          type="text"
          value={historyQuery}
          onChange={(e) => setHistoryQuery(e.target.value)}
          placeholder="Search translation history (Ctrl+F)..."
          className="w-full rounded-xl border border-border-default bg-surface-card pl-10 pr-4 py-2.5 text-sm text-text-primary placeholder:text-text-muted focus:border-primary-container focus:outline-none"
        />
      </div>

      {history.length === 0 ? (
        <div className="rounded-xl border border-border-default bg-surface-card p-8 text-center text-sm text-text-muted">
          No local text translation history found.
        </div>
      ) : (
        <div className="space-y-3">
          {history.map((item) => (
            <article
              key={item.id}
              className="rounded-xl border border-border-default bg-surface-card p-4 space-y-3"
            >
              <div className="flex flex-wrap items-center justify-between gap-2 text-xs">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="rounded-full border border-border-default bg-surface-container px-2.5 py-0.5 text-[11px] font-semibold text-primary">
                    {formatLanguagePair(item.source_lang, item.target_lang)}
                  </span>
                  <span className="rounded-full border border-border-default bg-surface-container px-2.5 py-0.5 text-[11px] font-medium text-primary capitalize">
                    {item.tone}
                  </span>
                  <span className="rounded-full border border-border-default bg-surface-container px-2.5 py-0.5 text-[11px] text-text-secondary">
                    {item.model_display_name}
                  </span>
                  <span className="text-text-muted tabular-nums">
                    {new Date(item.created_at).toLocaleString("en-US")}
                  </span>
                </div>

                <div className="flex items-center gap-1.5">
                  <button
                    type="button"
                    onClick={() => {
                      loadFromHistory(item);
                      setActivePage("text-translator");
                    }}
                    className="inline-flex items-center gap-1 rounded-lg border border-primary-container/30 bg-primary-container/15 px-2.5 py-1 text-xs font-medium text-primary hover:bg-primary-container/25 cursor-pointer"
                  >
                    <ArrowUpRight className="h-3.5 w-3.5" />
                    Open in Editor
                  </button>
                  <button
                    type="button"
                    onClick={() => void handleCopy(item.id, item.translated_text)}
                    aria-label="Copy translation"
                    className="rounded-lg border border-border-default bg-surface-elevated p-1.5 text-text-secondary hover:text-text-primary cursor-pointer"
                  >
                    {copiedId === item.id ? (
                      <Check className="h-3.5 w-3.5 text-primary" />
                    ) : (
                      <Copy className="h-3.5 w-3.5" />
                    )}
                  </button>
                  <button
                    type="button"
                    onClick={() => void deleteHistoryItem(item.id)}
                    aria-label="Delete history item"
                    className="rounded-lg border border-border-default bg-surface-elevated p-1.5 text-text-secondary hover:text-status-danger cursor-pointer"
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                  </button>
                </div>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-2 gap-3 text-xs">
                <div className="rounded-lg border border-border-subtle bg-surface-elevated/50 p-3 text-text-secondary line-clamp-4 whitespace-pre-wrap">
                  {item.source_text}
                </div>
                <div className="rounded-lg border border-border-subtle bg-surface-elevated/50 p-3 text-text-primary line-clamp-4 whitespace-pre-wrap">
                  {item.translated_text}
                </div>
              </div>
            </article>
          ))}
        </div>
      )}
    </div>
  );
}

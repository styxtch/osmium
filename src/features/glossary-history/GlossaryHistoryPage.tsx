import { useEffect } from "react";
import { AlertCircle, BookOpen, History, X } from "lucide-react";
import { useGlossaryHistoryStore } from "../../stores/useGlossaryHistoryStore";
import { useNavigationStore } from "../../stores/useNavigationStore";
import { GlossaryTab } from "./GlossaryTab";
import { HistoryTab } from "./HistoryTab";

export function GlossaryHistoryPage() {
  const tab = useNavigationStore((s) => s.glossaryHistoryTab);
  const setTab = useNavigationStore((s) => s.setGlossaryHistoryTab);

  const fetchGlossaries = useGlossaryHistoryStore((s) => s.fetchGlossaries);
  const error = useGlossaryHistoryStore((s) => s.error);
  const clearError = useGlossaryHistoryStore((s) => s.clearError);

  useEffect(() => {
    void fetchGlossaries();
  }, [fetchGlossaries]);

  return (
    <div className="w-full space-y-5">
      <header className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold leading-8 tracking-tight text-text-primary">
            Glossary & History
          </h1>
          <p className="mt-1 text-sm text-text-secondary">
            Manage consistent terminology collections for Subtitle Translator and browse local translation history.
          </p>
        </div>

        <div className="flex items-center gap-1 rounded-xl border border-border-default bg-surface-card p-1">
          <button
            type="button"
            onClick={() => setTab("glossary")}
            className={`inline-flex items-center gap-1.5 rounded-lg px-3.5 py-1.5 text-xs font-medium transition-colors cursor-pointer ${
              tab === "glossary"
                ? "bg-primary-container text-on-primary font-semibold"
                : "text-text-secondary hover:text-text-primary"
            }`}
          >
            <BookOpen className="h-3.5 w-3.5" />
            Subtitle Glossary
          </button>
          <button
            type="button"
            onClick={() => setTab("history")}
            className={`inline-flex items-center gap-1.5 rounded-lg px-3.5 py-1.5 text-xs font-medium transition-colors cursor-pointer ${
              tab === "history"
                ? "bg-primary-container text-on-primary font-semibold"
                : "text-text-secondary hover:text-text-primary"
            }`}
          >
            <History className="h-3.5 w-3.5" />
            Translation History
          </button>
        </div>
      </header>

      {error && (
        <div
          role="alert"
          className="flex items-center justify-between gap-3 rounded-xl border border-status-danger/40 bg-status-danger/10 px-4 py-3 text-sm text-status-danger"
        >
          <div className="flex items-center gap-2">
            <AlertCircle className="h-4 w-4 shrink-0" />
            <span>{error.message}</span>
          </div>
          <button
            type="button"
            onClick={clearError}
            aria-label="Dismiss error"
            className="rounded p-1 hover:bg-surface-hover cursor-pointer"
          >
            <X className="h-4 w-4" />
          </button>
        </div>
      )}

      {tab === "glossary" ? <GlossaryTab /> : <HistoryTab />}
    </div>
  );
}

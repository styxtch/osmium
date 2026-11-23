import { useEffect } from "react";
import { ArrowRight, History, X } from "lucide-react";
import type { TextHistoryItem } from "../../lib/ipc";
import { useGlossaryHistoryStore } from "../../stores/useGlossaryHistoryStore";
import { useNavigationStore } from "../../stores/useNavigationStore";

interface QuickHistoryPopoverProps {
  onSelect: (item: TextHistoryItem) => void;
  onClose: () => void;
}

export function QuickHistoryPopover({
  onSelect,
  onClose,
}: QuickHistoryPopoverProps) {
  const history = useGlossaryHistoryStore((s) => s.history);
  const fetchHistory = useGlossaryHistoryStore((s) => s.fetchHistory);
  const openHistoryPage = useNavigationStore((s) => s.openHistoryPage);

  useEffect(() => {
    void fetchHistory("");
  }, [fetchHistory]);

  const recent = history.slice(0, 10);

  return (
    <div
      role="dialog"
      aria-label="Quick Translation History"
      className="absolute right-0 top-full z-30 mt-2 w-80 rounded-xl border border-border-default bg-surface-card p-3 shadow-2xl"
    >
      <div className="flex items-center justify-between gap-2 pb-2 border-b border-border-subtle">
        <span className="inline-flex items-center gap-1.5 text-xs font-semibold text-text-primary">
          <History className="h-3.5 w-3.5 text-primary" />
          Quick History (Last 10)
        </span>
        <button
          type="button"
          onClick={onClose}
          aria-label="Close quick history"
          className="rounded-md p-1 text-text-secondary hover:bg-surface-hover hover:text-text-primary cursor-pointer"
        >
          <X className="h-3.5 w-3.5" />
        </button>
      </div>

      {recent.length === 0 ? (
        <div className="py-6 text-center text-xs text-text-muted">
          No saved translation history yet.
        </div>
      ) : (
        <ul className="mt-2 max-h-64 space-y-1.5 overflow-y-auto">
          {recent.map((item) => (
            <li key={item.id}>
              <button
                type="button"
                onClick={() => {
                  onSelect(item);
                  onClose();
                }}
                className="w-full rounded-lg border border-border-subtle bg-surface-elevated/70 p-2 text-left transition-colors hover:border-border-default hover:bg-surface-hover cursor-pointer"
              >
                <div className="truncate text-xs font-medium text-text-primary">
                  {item.source_text}
                </div>
                <div className="mt-0.5 truncate text-[11px] text-text-secondary">
                  {item.translated_text}
                </div>
              </button>
            </li>
          ))}
        </ul>
      )}

      <div className="mt-2 pt-2 border-t border-border-subtle">
        <button
          type="button"
          onClick={() => {
            onClose();
            openHistoryPage();
          }}
          className="flex w-full items-center justify-center gap-1.5 rounded-lg py-1.5 text-xs font-medium text-primary hover:bg-surface-elevated cursor-pointer"
        >
          View All History
          <ArrowRight className="h-3.5 w-3.5" />
        </button>
      </div>
    </div>
  );
}

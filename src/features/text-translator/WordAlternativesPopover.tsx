import { useEffect, useRef } from "react";
import { ArrowRight, Loader2, Sparkles, X } from "lucide-react";
import type { WordAlternativeItem } from "../../lib/ipc";

interface WordAlternativesPopoverProps {
  word: string;
  contextSummary: string;
  alternatives: WordAlternativeItem[];
  loading: boolean;
  error: string | null;
  anchorRect: { top: number; left: number; bottom: number };
  onSelect: (word: string) => void;
  onClose: () => void;
}

export function WordAlternativesPopover({
  word,
  contextSummary,
  alternatives,
  loading,
  error,
  anchorRect,
  onSelect,
  onClose,
}: WordAlternativesPopoverProps) {
  const popRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    const onMouseDown = (e: MouseEvent) => {
      if (popRef.current && !popRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("mousedown", onMouseDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("mousedown", onMouseDown);
    };
  }, [onClose]);

  const viewportW = typeof window !== "undefined" ? window.innerWidth : 1280;
  const viewportH = typeof window !== "undefined" ? window.innerHeight : 800;
  const left = Math.max(12, Math.min(anchorRect.left, viewportW - 336));
  const openAbove = anchorRect.bottom + 260 > viewportH && anchorRect.top > 240;
  const top = openAbove
    ? Math.max(12, anchorRect.top - 250)
    : Math.min(viewportH - 220, anchorRect.bottom + 8);

  return (
    <div
      ref={popRef}
      role="dialog"
      aria-label={`Alternatives for ${word}`}
      style={{ top, left }}
      className="fixed z-40 w-80 rounded-xl border border-border-default bg-surface-elevated p-3.5 shadow-[0_12px_32px_-4px_rgba(0,0,0,0.65),0_2px_6px_0_rgba(0,0,0,0.45)]"
    >
      <div className="flex items-start justify-between gap-2 pb-2.5 border-b border-border-subtle">
        <div className="min-w-0">
          <div className="text-xs font-semibold text-text-primary truncate">
            Alternatives for &lsquo;{word}&rsquo;
          </div>
          {contextSummary && (
            <div className="mt-0.5 text-[11px] text-text-secondary truncate">
              Context: &lsquo;{contextSummary}&rsquo;
            </div>
          )}
        </div>
        <button
          type="button"
          onClick={onClose}
          aria-label="Close word alternatives popover"
          className="rounded-md p-1 text-text-secondary hover:bg-surface-hover hover:text-text-primary cursor-pointer"
        >
          <X className="h-3.5 w-3.5" />
        </button>
      </div>

      <div className="mt-2.5">
        {loading ? (
          <div className="flex items-center justify-center gap-2 py-6 text-xs text-text-secondary">
            <Loader2 className="h-4 w-4 animate-spin text-primary" />
            Finding contextual alternatives...
          </div>
        ) : error ? (
          <div className="py-4 text-xs text-status-danger">{error}</div>
        ) : (
          <ul className="space-y-1.5">
            {alternatives.map((alt, idx) => (
              <li key={`${alt.word}-${idx}`}>
                <button
                  type="button"
                  onClick={() => onSelect(alt.word)}
                  className="group flex w-full items-start justify-between gap-2 rounded-lg border border-border-subtle bg-surface-card p-2.5 text-left transition-colors hover:border-primary-container/50 hover:bg-surface-hover cursor-pointer"
                >
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-1.5">
                      <span className="text-xs font-semibold text-text-primary">
                        {alt.word}
                      </span>
                      {alt.recommended && (
                        <span className="inline-flex items-center gap-1 rounded-full bg-primary-container/15 px-2 py-0.5 text-[10px] font-medium text-primary">
                          <Sparkles className="h-2.5 w-2.5" />
                          Recommended
                        </span>
                      )}
                    </div>
                    <p className="mt-1 text-[11px] leading-4 text-text-secondary">
                      {alt.nuance}
                    </p>
                  </div>

                  {alt.recommended ? (
                    <span className="shrink-0 rounded-md bg-primary-container px-2.5 py-1 text-[11px] font-semibold text-on-primary">
                      Replace
                    </span>
                  ) : (
                    <ArrowRight className="mt-0.5 h-3.5 w-3.5 shrink-0 text-text-muted group-hover:text-primary" />
                  )}
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>

      <div className="mt-2.5 pt-2 border-t border-border-subtle text-[11px] text-text-muted">
        Click to replace in-place • Ctrl+Z to Undo
      </div>
    </div>
  );
}

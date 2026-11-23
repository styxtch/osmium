import type { RefObject } from "react";
import { Search, X } from "lucide-react";
import type { SubtitleCueFilter } from "../../lib/ipc";

interface SubtitleSearchBarProps {
  inputRef: RefObject<HTMLInputElement | null>;
  searchQuery: string;
  statusFilter: SubtitleCueFilter;
  visibleCount: number;
  onSearchChange: (q: string) => void;
  onFilterChange: (filter: SubtitleCueFilter) => void;
}

const FILTERS: { value: SubtitleCueFilter; label: string }[] = [
  { value: "all", label: "All" },
  { value: "done", label: "Done" },
  { value: "processing", label: "Processing" },
  { value: "queued", label: "Queued" },
  { value: "warning", label: "Warnings" },
  { value: "skipped", label: "Comments / Drawings" },
];

export function SubtitleSearchBar({
  inputRef,
  searchQuery,
  statusFilter,
  visibleCount,
  onSearchChange,
  onFilterChange,
}: SubtitleSearchBarProps) {
  return (
    <div className="bg-surface-card border border-border-default rounded-xl px-4 py-2.5 flex flex-wrap items-center justify-between gap-3">
      <div className="relative flex-1 min-w-[220px] max-w-md">
        <Search className="w-3.5 h-3.5 text-text-muted absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none" />
        <input
          ref={inputRef}
          type="text"
          aria-label="Search subtitle cues"
          value={searchQuery}
          onChange={(e) => onSearchChange(e.target.value)}
          placeholder="Search dialogue, translation, cue #, or timestamp... (Ctrl+F)"
          className="w-full bg-surface-elevated border border-border-default rounded-lg pl-8 pr-8 py-1.5 text-xs text-text-primary placeholder:text-text-muted focus:outline-none focus:border-primary"
        />
        {searchQuery && (
          <button
            type="button"
            aria-label="Clear search"
            onClick={() => onSearchChange("")}
            className="absolute right-2.5 top-1/2 -translate-y-1/2 text-text-muted hover:text-text-primary"
          >
            <X className="w-3.5 h-3.5" />
          </button>
        )}
      </div>

      <div className="flex items-center gap-1.5 flex-wrap">
        {FILTERS.map((f) => {
          const active = statusFilter === f.value;
          return (
            <button
              key={f.value}
              type="button"
              onClick={() => onFilterChange(f.value)}
              className={`px-2.5 py-1 rounded-md text-[11px] font-medium transition-colors ${
                active
                  ? "bg-primary-container text-on-primary font-semibold"
                  : "bg-surface-elevated text-text-secondary hover:bg-surface-hover hover:text-text-primary"
              }`}
            >
              {f.label}
            </button>
          );
        })}

        <span className="ml-2 text-[11px] text-text-muted tabular-nums">
          {visibleCount} rows
        </span>
      </div>
    </div>
  );
}

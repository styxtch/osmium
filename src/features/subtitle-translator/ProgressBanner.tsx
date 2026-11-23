import { AlertTriangle, CheckCircle2, X, Zap } from "lucide-react";
import { formatEta, type SubtitleJobSummary } from "../../lib/ipc";

interface ProgressBannerProps {
  job: SubtitleJobSummary;
  batchTokPerSec: number | null;
  error: string | null;
  lastExportedPath: string | null;
  onFilterWarnings: () => void;
  onDismissError: () => void;
}

export function ProgressBanner({
  job,
  batchTokPerSec,
  error,
  lastExportedPath,
  onFilterWarnings,
  onDismissError,
}: ProgressBannerProps) {
  const pct =
    job.total_cues > 0
      ? Math.min(100, Math.round((job.completed_cues / job.total_cues) * 100))
      : 0;

  const remainingCues = Math.max(0, job.total_cues - job.completed_cues);
  // Conservative ETA estimate based on ~18 tokens per subtitle cue.
  const estSeconds =
    job.status === "running" &&
    batchTokPerSec &&
    batchTokPerSec > 0 &&
    remainingCues > 0
      ? Math.round((remainingCues * 18) / batchTokPerSec)
      : null;

  const statusText = (() => {
    switch (job.status) {
      case "running":
        return "Processing batches (SQLite WAL checkpoint saved after each batch)...";
      case "paused":
        return "Job paused cleanly. You can edit or re-translate individual rows or resume anytime.";
      case "completed":
        return "Translation 100% completed. Review warnings/edits if needed, then click Export.";
      case "cancelled":
        return "Job cancelled. Completed batches are preserved.";
      case "error":
        return "Engine stopped on error. Check banner below, then click Resume.";
      default:
        return "Ready to start offline batch translation.";
    }
  })();

  return (
    <div className="space-y-2">
      <div className="bg-surface-card border border-border-default rounded-xl px-4 py-3 space-y-2">
        <div className="flex flex-wrap items-center justify-between gap-2 text-xs">
          <span className="text-text-secondary">{statusText}</span>
          <div className="flex items-center gap-3 tabular-nums">
            {batchTokPerSec !== null && (
              <span className="inline-flex items-center gap-1 text-primary font-medium">
                <Zap className="w-3.5 h-3.5" />
                {batchTokPerSec.toFixed(1)} tok/s
              </span>
            )}
            {estSeconds !== null && (
              <span className="text-text-muted">
                ETA {formatEta(estSeconds)}
              </span>
            )}
            <span className="font-semibold text-text-primary">
              {job.completed_cues} / {job.total_cues} cues ({pct}%)
            </span>
            {job.self_healing_errors > 0 && (
              <button
                type="button"
                onClick={onFilterWarnings}
                className="inline-flex items-center gap-1 px-2 py-0.5 rounded bg-tag-optimize/15 hover:bg-tag-optimize/25 text-tag-optimize text-[11px] font-semibold transition-colors"
              >
                <AlertTriangle className="w-3 h-3" />
                {job.self_healing_errors} Warnings
              </button>
            )}
          </div>
        </div>

        {/* Determinate Progress Bar */}
        <div className="w-full h-1.5 rounded-full bg-surface-elevated overflow-hidden">
          <div
            className={`h-full rounded-full transition-all duration-300 ${
              job.status === "error"
                ? "bg-status-danger"
                : "bg-gradient-to-r from-primary-container to-primary"
            }`}
            style={{ width: `${pct}%` }}
          />
        </div>
      </div>

      {lastExportedPath && (
        <div className="flex items-center gap-2 px-4 py-2.5 rounded-xl bg-status-success/10 border border-status-success/30 text-xs text-status-success">
          <CheckCircle2 className="w-4 h-4 shrink-0" />
          <span className="truncate">
            Exported translated subtitle to:{" "}
            <strong className="font-semibold">{lastExportedPath}</strong>
          </span>
        </div>
      )}

      {error && (
        <div className="flex items-center justify-between gap-2 px-4 py-2.5 rounded-xl bg-status-danger/10 border border-status-danger/30 text-xs text-status-danger">
          <span>{error}</span>
          <button
            type="button"
            onClick={onDismissError}
            className="p-0.5 rounded hover:bg-status-danger/20"
            aria-label="Dismiss error"
          >
            <X className="w-3.5 h-3.5" />
          </button>
        </div>
      )}
    </div>
  );
}

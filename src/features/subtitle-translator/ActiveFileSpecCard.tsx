import {
  ArrowLeft,
  CheckCircle2,
  Download,
  Pause,
  Play,
  RotateCcw,
  ShieldCheck,
  Square,
} from "lucide-react";
import {
  formatBytes,
  type Glossary,
  type SubtitleGenre,
  type SubtitleJobSummary,
} from "../../lib/ipc";

interface ActiveFileSpecCardProps {
  job: SubtitleJobSummary;
  glossaries: Glossary[];
  activeModelName: string | null;
  hasActiveModel: boolean;
  isExporting: boolean;
  onBackToJobs: () => void;
  onUpdateOptions: (
    genre: SubtitleGenre,
    glossaryId: string | null,
    tagShieldEnabled: boolean
  ) => void;
  onStart: () => void;
  onPause: () => void;
  onResume: () => void;
  onCancel: () => void;
  onExport: () => void;
}

const GENRE_OPTIONS: { value: SubtitleGenre; label: string }[] = [
  { value: "general", label: "General" },
  { value: "anime", label: "Anime" },
  { value: "film", label: "Film & Series" },
  { value: "documentary", label: "Documentary" },
  { value: "gaming", label: "Gaming" },
];

export function ActiveFileSpecCard({
  job,
  glossaries,
  activeModelName,
  hasActiveModel,
  isExporting,
  onBackToJobs,
  onUpdateOptions,
  onStart,
  onPause,
  onResume,
  onCancel,
  onExport,
}: ActiveFileSpecCardProps) {
  const isRunning = job.status === "running";
  const isCompleted = job.status === "completed";
  const canResume =
    job.status === "paused" ||
    job.status === "error" ||
    job.status === "cancelled";

  return (
    <div className="bg-surface-card border border-border-default rounded-xl p-4 space-y-3">
      {/* Top Row: File metadata & Primary Action Buttons */}
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-3 min-w-0">
          <button
            type="button"
            disabled={isRunning}
            onClick={onBackToJobs}
            title={
              isRunning
                ? "Pause the job before returning to the jobs list"
                : "Back to all subtitle jobs"
            }
            className="p-2 rounded-lg bg-surface-elevated hover:bg-surface-hover border border-border-default text-text-secondary hover:text-text-primary transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
          >
            <ArrowLeft className="w-4 h-4" />
          </button>

          <div className="min-w-0">
            <div className="flex items-center gap-2 flex-wrap">
              <h1 className="text-sm font-semibold text-text-primary truncate max-w-md">
                {job.file_name}
              </h1>
              <span className="px-1.5 py-0.5 rounded bg-surface-elevated border border-border-default text-[10px] font-semibold uppercase text-tag-organize">
                .{job.format}
              </span>
              <span className="text-[11px] text-text-muted tabular-nums">
                {formatBytes(job.file_size_bytes)}
              </span>
              {isCompleted && (
                <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full bg-status-success/15 text-status-success text-[11px] font-semibold">
                  <CheckCircle2 className="w-3 h-3" />
                  100% Complete
                </span>
              )}
            </div>
            <p
              className="text-[11px] text-text-muted truncate max-w-xl"
              title={job.file_path}
            >
              {job.file_path}
            </p>
          </div>
        </div>

        {/* Action Controls */}
        <div className="flex items-center gap-2 flex-wrap">
          {job.status === "idle" && (
            <button
              type="button"
              disabled={!hasActiveModel}
              onClick={onStart}
              title={
                hasActiveModel
                  ? "Start batch subtitle translation"
                  : "Activate a model in Model Manager first"
              }
              className="inline-flex items-center gap-1.5 px-3.5 py-2 rounded-lg bg-primary-container hover:bg-primary text-on-primary text-xs font-semibold transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
            >
              <Play className="w-3.5 h-3.5 fill-current" />
              Start Translation
            </button>
          )}

          {isRunning && (
            <>
              <button
                type="button"
                onClick={onPause}
                className="inline-flex items-center gap-1.5 px-3 py-2 rounded-lg bg-tag-optimize/20 hover:bg-tag-optimize/30 border border-tag-optimize/40 text-tag-optimize text-xs font-semibold transition-colors"
              >
                <Pause className="w-3.5 h-3.5" />
                Pause
              </button>
              <button
                type="button"
                onClick={onCancel}
                className="inline-flex items-center gap-1.5 px-3 py-2 rounded-lg bg-surface-elevated hover:bg-status-danger/15 border border-border-default text-text-secondary hover:text-status-danger text-xs font-medium transition-colors"
              >
                <Square className="w-3.5 h-3.5" />
                Cancel
              </button>
            </>
          )}

          {canResume && (
            <>
              <button
                type="button"
                disabled={!hasActiveModel}
                onClick={onResume}
                className="inline-flex items-center gap-1.5 px-3.5 py-2 rounded-lg bg-primary-container hover:bg-primary text-on-primary text-xs font-semibold transition-colors disabled:opacity-40"
              >
                <RotateCcw className="w-3.5 h-3.5" />
                Resume
              </button>
              {job.status !== "cancelled" && (
                <button
                  type="button"
                  onClick={onCancel}
                  className="inline-flex items-center gap-1.5 px-3 py-2 rounded-lg bg-surface-elevated hover:bg-status-danger/15 border border-border-default text-text-secondary hover:text-status-danger text-xs font-medium transition-colors"
                >
                  <Square className="w-3.5 h-3.5" />
                  Cancel
                </button>
              )}
            </>
          )}

          {/* Export strictly locked until 100% completion */}
          <button
            type="button"
            disabled={!isCompleted || isExporting}
            onClick={onExport}
            title={
              isCompleted
                ? "Export translated subtitle (Ctrl+S)"
                : "Export unlocks once 100% of subtitle cues are completed"
            }
            className={`inline-flex items-center gap-1.5 px-3.5 py-2 rounded-lg text-xs font-semibold transition-colors ${
              isCompleted
                ? "bg-primary-container hover:bg-primary text-on-primary"
                : "bg-surface-elevated border border-border-default text-text-disabled cursor-not-allowed"
            }`}
          >
            <Download className="w-3.5 h-3.5" />
            {isExporting ? "Exporting..." : "Export..."}
          </button>
        </div>
      </div>

      {/* Bottom Options Strip (Genre, Glossary, Tag Shield, Model) */}
      <div className="pt-3 border-t border-border-subtle flex flex-wrap items-center justify-between gap-3">
        <div className="flex flex-wrap items-center gap-3">
          <label className="flex items-center gap-2 text-xs">
            <span className="text-text-muted">Genre:</span>
            <select
              aria-label="Job Genre"
              disabled={isRunning}
              value={job.genre}
              onChange={(e) =>
                onUpdateOptions(
                  e.target.value as SubtitleGenre,
                  job.glossary_id,
                  job.tag_shield_enabled
                )
              }
              className="bg-surface-elevated border border-border-default rounded-md px-2.5 py-1 text-xs text-text-primary focus:outline-none focus:border-primary disabled:opacity-50"
            >
              {GENRE_OPTIONS.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {opt.label}
                </option>
              ))}
            </select>
          </label>

          <label className="flex items-center gap-2 text-xs">
            <span className="text-text-muted">Glossary:</span>
            <select
              aria-label="Job Glossary"
              disabled={isRunning}
              value={job.glossary_id ?? ""}
              onChange={(e) =>
                onUpdateOptions(
                  job.genre as SubtitleGenre,
                  e.target.value || null,
                  job.tag_shield_enabled
                )
              }
              className="bg-surface-elevated border border-border-default rounded-md px-2.5 py-1 text-xs text-text-primary focus:outline-none focus:border-primary disabled:opacity-50"
            >
              <option value="">None</option>
              {glossaries.map((g) => (
                <option key={g.id} value={g.id}>
                  {g.name} ({g.entry_count})
                </option>
              ))}
            </select>
          </label>

          <button
            type="button"
            disabled={isRunning}
            onClick={() =>
              onUpdateOptions(
                job.genre as SubtitleGenre,
                job.glossary_id,
                !job.tag_shield_enabled
              )
            }
            className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md border text-xs transition-colors disabled:opacity-50 ${
              job.tag_shield_enabled
                ? "bg-primary/10 border-primary/30 text-text-primary"
                : "bg-surface-elevated border-border-default text-text-muted"
            }`}
          >
            <ShieldCheck
              className={`w-3.5 h-3.5 ${
                job.tag_shield_enabled ? "text-primary" : "text-text-muted"
              }`}
            />
            Tag Shield: {job.tag_shield_enabled ? "ON" : "OFF"}
          </button>
        </div>

        <div className="text-[11px] text-text-secondary">
          Model:{" "}
          <span className="text-text-primary font-medium">
            {activeModelName ?? job.model_display_name}
          </span>
        </div>
      </div>
    </div>
  );
}

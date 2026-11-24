import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import {
  AlertTriangle,
  Captions,
  CheckCircle2,
  FileUp,
  FolderOpen,
  PauseCircle,
  Play,
  ShieldCheck,
  Trash2,
  XCircle,
} from "lucide-react";
import {
  formatBytes,
  type Glossary,
  type SubtitleGenre,
  type SubtitleJobSummary,
} from "../../lib/ipc";
import {
  formatLanguagePair,
  SOURCE_LANGUAGES,
  TARGET_LANGUAGES,
} from "../../lib/languages";

interface DropzoneSavedJobsProps {
  jobs: SubtitleJobSummary[];
  glossaries: Glossary[];
  isLoadingJobs: boolean;
  isParsingFile: boolean;
  onImportFile: (
    filePath: string,
    sourceLang: string,
    targetLang: string,
    genre: SubtitleGenre,
    glossaryId: string | null,
    tagShieldEnabled: boolean
  ) => Promise<void>;
  onSelectJob: (jobId: string) => Promise<void>;
  onDeleteJob: (jobId: string) => Promise<void>;
}

const GENRE_OPTIONS: { value: SubtitleGenre; label: string }[] = [
  { value: "general", label: "General" },
  { value: "anime", label: "Anime / Animation" },
  { value: "film", label: "Film & TV Series" },
  { value: "documentary", label: "Documentary" },
  { value: "gaming", label: "Gaming & Lore" },
];

export function DropzoneSavedJobs({
  jobs,
  glossaries,
  isLoadingJobs,
  isParsingFile,
  onImportFile,
  onSelectJob,
  onDeleteJob,
}: DropzoneSavedJobsProps) {
  const [sourceLang, setSourceLang] = useState<string>("auto");
  const [targetLang, setTargetLang] = useState<string>("id");
  const [genre, setGenre] = useState<SubtitleGenre>("general");
  const [glossaryId, setGlossaryId] = useState<string>("");
  const [tagShieldEnabled, setTagShieldEnabled] = useState<boolean>(true);
  const [isDragHover, setIsDragHover] = useState<boolean>(false);

  const handleSourceLangChange = (nextSrc: string) => {
    if (nextSrc !== "auto" && nextSrc === targetLang) {
      setTargetLang(
        sourceLang !== "auto" ? sourceLang : nextSrc === "en" ? "id" : "en"
      );
    }
    setSourceLang(nextSrc);
  };

  const handleTargetLangChange = (nextDst: string) => {
    if (nextDst === "auto") return;
    if (sourceLang !== "auto" && nextDst === sourceLang) {
      setSourceLang(targetLang);
    }
    setTargetLang(nextDst);
  };

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    try {
      const win = getCurrentWebviewWindow();
      void win
        .onDragDropEvent((event) => {
          if (event.payload.type === "enter" || event.payload.type === "over") {
            setIsDragHover(true);
          } else if (event.payload.type === "leave") {
            setIsDragHover(false);
          } else if (event.payload.type === "drop") {
            setIsDragHover(false);
            const droppedPath = event.payload.paths?.[0];
            if (droppedPath) {
              void onImportFile(
                droppedPath,
                sourceLang,
                targetLang,
                genre,
                glossaryId || null,
                tagShieldEnabled
              );
            }
          }
        })
        .then((fn) => {
          unlisten = fn;
        })
        .catch(() => {
          // Ignore when running outside full Tauri webview (e.g. unit tests)
        });
    } catch {
      // Ignore in test environments without Tauri window runtime
    }
    return () => {
      unlisten?.();
    };
  }, [sourceLang, targetLang, genre, glossaryId, onImportFile, tagShieldEnabled]);

  const handleBrowseFile = async () => {
    const selected = await open({
      multiple: false,
      title: "Select Subtitle File",
      filters: [
        {
          name: "Subtitle Files (.srt, .vtt, .ass)",
          extensions: ["srt", "vtt", "ass", "ssa"],
        },
      ],
    });
    if (typeof selected === "string" && selected.trim()) {
      await onImportFile(
        selected,
        sourceLang,
        targetLang,
        genre,
        glossaryId || null,
        tagShieldEnabled
      );
    }
  };

  return (
    <div className="w-full flex-1 min-h-0 overflow-y-auto space-y-5">
      {/* Header */}
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h1 className="text-lg font-semibold text-text-primary flex items-center gap-2">
            <Captions className="w-5 h-5 text-primary" />
            Subtitle Translator
          </h1>
          <p className="text-xs text-text-secondary mt-0.5">
            Translate complete .SRT, .VTT, and .ASS files offline with Tag
            Shield protection, per-batch SQLite checkpoints, and glossary terms.
          </p>
        </div>

        <div className="flex items-center gap-2 text-[11px] font-medium text-text-secondary">
          <span className="px-2 py-0.5 rounded bg-surface-elevated border border-border-default">
            .SRT
          </span>
          <span className="px-2 py-0.5 rounded bg-surface-elevated border border-border-default">
            .VTT
          </span>
          <span className="px-2 py-0.5 rounded bg-surface-elevated border border-border-default">
            .ASS / .SSA
          </span>
        </div>
      </div>

      {/* Import Options + Dropzone Card */}
      <div className="bg-surface-card border border-border-default rounded-xl p-5 space-y-4">
        {/* Configuration before opening file */}
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-5 gap-3">
          <label className="flex flex-col gap-1.5 text-xs">
            <span className="text-text-secondary font-medium">
              Source Language
            </span>
            <select
              aria-label="Subtitle Source Language"
              value={sourceLang}
              onChange={(e) => handleSourceLangChange(e.target.value)}
              className="bg-surface-elevated border border-border-default rounded-lg px-3 py-2 text-xs text-text-primary focus:outline-none focus:border-primary"
            >
              {SOURCE_LANGUAGES.map((lang) => (
                <option key={lang.code} value={lang.code}>
                  {lang.name}
                </option>
              ))}
            </select>
          </label>

          <label className="flex flex-col gap-1.5 text-xs">
            <span className="text-text-secondary font-medium">
              Target Language
            </span>
            <select
              aria-label="Subtitle Target Language"
              value={targetLang}
              onChange={(e) => handleTargetLangChange(e.target.value)}
              className="bg-surface-elevated border border-border-default rounded-lg px-3 py-2 text-xs text-text-primary focus:outline-none focus:border-primary"
            >
              {TARGET_LANGUAGES.map((lang) => (
                <option key={lang.code} value={lang.code}>
                  {lang.name}
                </option>
              ))}
            </select>
          </label>

          <label className="flex flex-col gap-1.5 text-xs">
            <span className="text-text-secondary font-medium">
              Content Genre
            </span>
            <select
              aria-label="Content Genre"
              value={genre}
              onChange={(e) => setGenre(e.target.value as SubtitleGenre)}
              className="bg-surface-elevated border border-border-default rounded-lg px-3 py-2 text-xs text-text-primary focus:outline-none focus:border-primary"
            >
              {GENRE_OPTIONS.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {opt.label}
                </option>
              ))}
            </select>
          </label>

          <label className="flex flex-col gap-1.5 text-xs">
            <span className="text-text-secondary font-medium">
              Glossary (Optional)
            </span>
            <select
              aria-label="Glossary"
              value={glossaryId}
              onChange={(e) => setGlossaryId(e.target.value)}
              className="bg-surface-elevated border border-border-default rounded-lg px-3 py-2 text-xs text-text-primary focus:outline-none focus:border-primary"
            >
              <option value="">No Glossary</option>
              {glossaries.map((g) => (
                <option key={g.id} value={g.id}>
                  {g.name} ({g.entry_count} terms)
                </option>
              ))}
            </select>
          </label>

          <div className="flex flex-col justify-end">
            <button
              type="button"
              onClick={() => setTagShieldEnabled((prev) => !prev)}
              className={`flex items-center justify-between px-3 py-2 rounded-lg border text-xs transition-colors ${
                tagShieldEnabled
                  ? "bg-primary/10 border-primary/40 text-text-primary"
                  : "bg-surface-elevated border-border-default text-text-secondary"
              }`}
            >
              <span className="flex items-center gap-2 font-medium">
                <ShieldCheck
                  className={`w-4 h-4 ${
                    tagShieldEnabled ? "text-primary" : "text-text-muted"
                  }`}
                />
                Tag Shield Protection
              </span>
              <span
                className={`text-[11px] font-semibold px-2 py-0.5 rounded ${
                  tagShieldEnabled
                    ? "bg-primary-container text-on-primary"
                    : "bg-surface-hover text-text-muted"
                }`}
              >
                {tagShieldEnabled ? "ON" : "OFF"}
              </span>
            </button>
          </div>
        </div>

        {/* Dropzone Box */}
        <div
          onDragOver={(e) => {
            e.preventDefault();
            setIsDragHover(true);
          }}
          onDragLeave={() => setIsDragHover(false)}
          onDrop={(e) => {
            e.preventDefault();
            setIsDragHover(false);
          }}
          className={`border-2 border-dashed rounded-xl px-6 py-10 flex flex-col items-center justify-center text-center transition-colors ${
            isDragHover
              ? "border-primary bg-primary/5"
              : "border-border-default bg-surface-container hover:border-border-default/80"
          }`}
        >
          <div className="w-12 h-12 rounded-xl bg-surface-elevated border border-border-default flex items-center justify-center mb-3">
            <FileUp className="w-6 h-6 text-primary" />
          </div>
          <h2 className="text-sm font-semibold text-text-primary">
            {isParsingFile
              ? "Parsing subtitle cues..."
              : "Drop a subtitle file here or browse from disk"}
          </h2>
          <p className="text-xs text-text-secondary mt-1 max-w-md">
            Files are referenced directly from their original folder — zero
            duplicate copies are created in AppData.
          </p>
          <button
            type="button"
            disabled={isParsingFile}
            onClick={() => void handleBrowseFile()}
            className="mt-4 inline-flex items-center gap-2 px-4 py-2 rounded-lg bg-primary-container hover:bg-primary text-on-primary text-xs font-semibold transition-colors disabled:opacity-50"
          >
            <FolderOpen className="w-4 h-4" />
            {isParsingFile ? "Loading..." : "Choose Subtitle File"}
          </button>
        </div>
      </div>

      {/* Saved / Resumable Subtitle Jobs */}
      <div className="bg-surface-card border border-border-default rounded-xl p-5 space-y-3">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-semibold text-text-primary">
            Recent & Resumable Jobs
          </h2>
          <span className="text-xs text-text-muted tabular-nums">
            {jobs.length} {jobs.length === 1 ? "job" : "jobs"}
          </span>
        </div>

        {isLoadingJobs && jobs.length === 0 ? (
          <p className="text-xs text-text-muted py-6 text-center">
            Loading saved subtitle jobs...
          </p>
        ) : jobs.length === 0 ? (
          <p className="text-xs text-text-muted py-6 text-center">
            No subtitle jobs yet. Select a .srt, .vtt, or .ass file above to
            start.
          </p>
        ) : (
          <div className="divide-y divide-border-subtle">
            {jobs.map((job) => {
              const pct =
                job.total_cues > 0
                  ? Math.round((job.completed_cues / job.total_cues) * 100)
                  : 0;
              return (
                <div
                  key={job.id}
                  className="py-3 flex flex-wrap items-center justify-between gap-3"
                >
                  <div className="min-w-0 flex-1 space-y-1">
                    <div className="flex items-center gap-2 flex-wrap">
                      <span className="text-xs font-semibold text-text-primary truncate">
                        {job.file_name}
                      </span>
                      <span className="px-1.5 py-0.5 rounded bg-surface-elevated border border-border-default text-[10px] font-semibold uppercase text-tag-organize">
                        {job.format}
                      </span>
                      <span className="px-1.5 py-0.5 rounded bg-surface-elevated border border-border-default text-[10px] font-semibold text-primary">
                        {formatLanguagePair(job.source_lang, job.target_lang)}
                      </span>
                      <span className="px-1.5 py-0.5 rounded bg-surface-elevated text-[10px] font-medium capitalize text-text-secondary">
                        {job.genre}
                      </span>
                      <JobStatusPill status={job.status} />
                      {job.self_healing_errors > 0 && (
                        <span className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-tag-optimize/15 text-tag-optimize text-[10px] font-medium tabular-nums">
                          <AlertTriangle className="w-3 h-3" />
                          {job.self_healing_errors} warnings
                        </span>
                      )}
                    </div>

                    <div className="flex items-center gap-3 text-[11px] text-text-muted tabular-nums">
                      <span className="truncate max-w-md" title={job.file_path}>
                        {job.file_path}
                      </span>
                      <span>•</span>
                      <span>{formatBytes(job.file_size_bytes)}</span>
                      <span>•</span>
                      <span>
                        {job.completed_cues} / {job.total_cues} cues ({pct}%)
                      </span>
                    </div>
                  </div>

                  <div className="flex items-center gap-2">
                    <button
                      type="button"
                      onClick={() => void onSelectJob(job.id)}
                      className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-surface-elevated hover:bg-surface-hover border border-border-default text-xs font-medium text-text-primary transition-colors"
                    >
                      <Play className="w-3.5 h-3.5 text-primary" />
                      {job.status === "completed" ? "Inspect / Export" : "Open"}
                    </button>
                    <button
                      type="button"
                      disabled={job.status === "running"}
                      onClick={() => void onDeleteJob(job.id)}
                      title={
                        job.status === "running"
                          ? "Pause or cancel job before deleting"
                          : "Delete job from history"
                      }
                      className="p-1.5 rounded-lg bg-surface-elevated hover:bg-status-danger/15 border border-border-default text-text-secondary hover:text-status-danger transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}

function JobStatusPill({ status }: { status: string }) {
  switch (status) {
    case "completed":
      return (
        <span className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-status-success/15 text-status-success text-[10px] font-semibold">
          <CheckCircle2 className="w-3 h-3" />
          Completed
        </span>
      );
    case "paused":
      return (
        <span className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-tag-optimize/15 text-tag-optimize text-[10px] font-semibold">
          <PauseCircle className="w-3 h-3" />
          Paused
        </span>
      );
    case "error":
      return (
        <span className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-status-danger/15 text-status-danger text-[10px] font-semibold">
          <XCircle className="w-3 h-3" />
          Error
        </span>
      );
    case "running":
      return (
        <span className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-primary/15 text-primary text-[10px] font-semibold">
          Processing
        </span>
      );
    default:
      return (
        <span className="px-1.5 py-0.5 rounded bg-surface-elevated text-text-muted text-[10px] font-medium capitalize">
          {status}
        </span>
      );
  }
}

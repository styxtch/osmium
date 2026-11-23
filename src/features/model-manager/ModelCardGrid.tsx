import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  AlertTriangle,
  CheckCircle2,
  Download,
  FileUp,
  FolderSearch,
  Sparkles,
  Trash2,
} from "lucide-react";
import {
  formatBytes,
  formatVramGb,
  type CatalogModel,
  type DownloadProgressEvent,
  type DownloadTask,
  type HardwareProfile,
  type InstalledModelWithHealth,
} from "../../lib/ipc";

interface ModelCardGridProps {
  catalog: CatalogModel[];
  installed: InstalledModelWithHealth[];
  downloads: DownloadTask[];
  progressByModel: Record<string, DownloadProgressEvent>;
  hardware: HardwareProfile | null;
  actionBusy: boolean;
  onOpenConsent: (modelId: string) => void;
  onImportGguf: (filePath: string) => Promise<void>;
  onRelinkByom: (modelId: string, newPath: string) => Promise<void>;
  onActivate: (modelId: string) => Promise<void>;
  onDelete: (modelId: string) => Promise<void>;
}

export function ModelCardGrid({
  catalog,
  installed,
  downloads,
  progressByModel,
  hardware,
  actionBusy,
  onOpenConsent,
  onImportGguf,
  onRelinkByom,
  onActivate,
  onDelete,
}: ModelCardGridProps) {
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);

  const byomModels = installed.filter((m) => m.is_byom);

  const handlePickByom = async () => {
    const selected = await open({
      title: "Select Local .gguf Model File",
      multiple: false,
      directory: false,
      filters: [{ name: "GGUF Model Weights", extensions: ["gguf"] }],
    });
    if (typeof selected === "string" && selected.length > 0) {
      await onImportGguf(selected);
    }
  };

  const handleRelink = async (modelId: string) => {
    const selected = await open({
      title: "Locate .gguf Model File",
      multiple: false,
      directory: false,
      filters: [{ name: "GGUF Model Weights", extensions: ["gguf"] }],
    });
    if (typeof selected === "string" && selected.length > 0) {
      await onRelinkByom(modelId, selected);
    }
  };

  return (
    <section className="grid grid-cols-1 lg:grid-cols-3 gap-4">
      {catalog.map((cat) => {
        const installedModel = installed.find((m) => m.id === cat.id);
        const downloadTask = downloads.find((d) => d.model_id === cat.id);
        const liveProgress = progressByModel[cat.id];
        const isRecommended = hardware?.recommended_tier === cat.tier;
        const isOptimal = cat.tier === "optimal_8b";

        return (
          <article
            key={cat.id}
            className={`flex flex-col justify-between rounded-xl border bg-surface-card p-5 transition-colors ${
              installedModel?.is_active
                ? "border-primary-container/60"
                : "border-border-default"
            }`}
          >
            <div>
              {/* Badges with flex-wrap & gap to prevent typography overlap */}
              <div className="flex flex-wrap items-center gap-2 mb-3">
                <span
                  className={`inline-flex items-center gap-1 rounded-full border px-2.5 py-0.5 text-[11px] font-medium ${
                    isOptimal
                      ? "border-primary-container/30 bg-primary-container/12 text-primary"
                      : "border-tag-optimize/30 bg-tag-optimize/12 text-tag-optimize"
                  }`}
                >
                  {isRecommended && <Sparkles className="h-3 w-3" />}
                  {cat.badge_label}
                </span>
                <span className="rounded-full border border-border-default bg-surface-container px-2.5 py-0.5 text-[11px] font-medium text-text-secondary">
                  {cat.repo_id}
                </span>
              </div>

              <h3 className="text-base font-semibold leading-6 text-text-primary">
                {cat.display_name}
              </h3>
              <p className="mt-1.5 text-[13px] leading-5 text-text-secondary">
                {cat.description}
              </p>

              {/* Spec Table */}
              <dl className="mt-4 space-y-2 rounded-lg border border-border-subtle bg-surface-container p-3 text-xs">
                <div className="flex items-center justify-between gap-2">
                  <dt className="text-text-muted">Quantization</dt>
                  <dd className="font-medium text-text-primary">{cat.quantization}</dd>
                </div>
                <div className="flex items-center justify-between gap-2">
                  <dt className="text-text-muted">File Size</dt>
                  <dd className="font-medium text-text-primary tabular-nums">
                    {formatBytes(cat.file_size_bytes)}
                  </dd>
                </div>
                <div className="flex items-center justify-between gap-2">
                  <dt className="text-text-muted">Est. VRAM</dt>
                  <dd className="font-medium text-text-primary tabular-nums">
                    ~{formatVramGb(cat.estimated_vram_mb)}
                  </dd>
                </div>
              </dl>
            </div>

            {/* Card Footer Actions */}
            <div className="mt-5 pt-4 border-t border-border-subtle">
              {installedModel ? (
                <div className="space-y-2.5">
                  {!installedModel.file_exists && (
                    <div className="flex items-center gap-1.5 text-xs text-status-danger">
                      <AlertTriangle className="h-3.5 w-3.5 shrink-0" />
                      <span>Model file not found on disk</span>
                    </div>
                  )}
                  <div className="flex flex-wrap items-center justify-between gap-2">
                    {installedModel.is_active ? (
                      <span className="inline-flex items-center gap-1.5 rounded-lg border border-primary-container/30 bg-primary-container/15 px-3 py-1.5 text-xs font-semibold text-primary">
                        <CheckCircle2 className="h-3.5 w-3.5" />
                        Active Model
                      </span>
                    ) : (
                      <button
                        type="button"
                        disabled={actionBusy || !installedModel.file_exists}
                        onClick={() => void onActivate(installedModel.id)}
                        className="rounded-lg bg-primary-container px-3.5 py-1.5 text-xs font-semibold text-on-primary hover:bg-primary-container/90 disabled:opacity-50 cursor-pointer"
                      >
                        Activate Model
                      </button>
                    )}

                    {confirmDeleteId === installedModel.id ? (
                      <div className="flex items-center gap-1.5">
                        <button
                          type="button"
                          disabled={actionBusy}
                          onClick={() => {
                            setConfirmDeleteId(null);
                            void onDelete(installedModel.id);
                          }}
                          className="rounded-lg bg-status-danger px-2.5 py-1.5 text-xs font-semibold text-white hover:bg-status-danger/90 cursor-pointer"
                        >
                          Confirm Delete
                        </button>
                        <button
                          type="button"
                          onClick={() => setConfirmDeleteId(null)}
                          className="rounded-lg border border-border-default bg-surface-elevated px-2.5 py-1.5 text-xs text-text-secondary hover:text-text-primary cursor-pointer"
                        >
                          Cancel
                        </button>
                      </div>
                    ) : (
                      <button
                        type="button"
                        disabled={actionBusy}
                        onClick={() => setConfirmDeleteId(installedModel.id)}
                        className="inline-flex items-center gap-1 rounded-lg border border-border-default bg-surface-elevated px-2.5 py-1.5 text-xs text-text-secondary hover:border-status-danger/50 hover:text-status-danger cursor-pointer"
                      >
                        <Trash2 className="h-3.5 w-3.5" />
                        Delete
                      </button>
                    )}
                  </div>
                </div>
              ) : downloadTask || liveProgress?.status === "downloading" ? (
                <button
                  type="button"
                  onClick={() => onOpenConsent(cat.id)}
                  className="flex w-full items-center justify-center gap-2 rounded-lg border border-tag-optimize/40 bg-tag-optimize/12 px-4 py-2 text-xs font-semibold text-tag-optimize hover:bg-tag-optimize/20 cursor-pointer"
                >
                  <Download className="h-3.5 w-3.5" />
                  {liveProgress?.status === "downloading"
                    ? `Downloading (${liveProgress.percent.toFixed(1)}%)`
                    : `Resume Download (${formatBytes(downloadTask?.bytes_downloaded ?? 0)})`}
                </button>
              ) : (
                <button
                  type="button"
                  disabled={actionBusy}
                  onClick={() => onOpenConsent(cat.id)}
                  className="flex w-full items-center justify-center gap-2 rounded-lg bg-primary-container px-4 py-2 text-sm font-semibold text-on-primary hover:bg-primary-container/90 disabled:opacity-50 cursor-pointer"
                >
                  <Download className="h-4 w-4" />
                  Download Model ({formatBytes(cat.file_size_bytes)})
                </button>
              )}
            </div>
          </article>
        );
      })}

      {/* Card 3: Custom / Imported GGUF (BYOM) */}
      <article className="flex flex-col justify-between rounded-xl border border-border-default bg-surface-card p-5">
        <div>
          <div className="flex flex-wrap items-center gap-2 mb-3">
            <span className="inline-flex items-center rounded-full border border-tag-convert/30 bg-tag-convert/12 px-2.5 py-0.5 text-[11px] font-medium text-tag-convert">
              BYOM • Custom GGUF
            </span>
            <span className="rounded-full border border-border-default bg-surface-container px-2.5 py-0.5 text-[11px] font-medium text-text-secondary">
              Local Path Reference
            </span>
          </div>

          <h3 className="text-base font-semibold leading-6 text-text-primary">
            Custom / Imported GGUF
          </h3>
          <p className="mt-1.5 text-[13px] leading-5 text-text-secondary">
            Use your own local .gguf weights (Qwen, Gemma, Llama, Mistral) without copying files.
            GGUF magic bytes are validated automatically.
          </p>

          {/* Dropzone / Picker Trigger */}
          <button
            type="button"
            disabled={actionBusy}
            onClick={() => void handlePickByom()}
            className="mt-4 flex w-full flex-col items-center justify-center rounded-lg border border-dashed border-border-default bg-surface-elevated/60 px-4 py-5 text-center transition-colors hover:border-tag-convert/60 hover:bg-surface-elevated cursor-pointer"
          >
            <FileUp className="h-6 w-6 text-tag-convert mb-2" />
            <span className="text-xs font-semibold text-text-primary">
              Select Local .gguf File...
            </span>
            <span className="mt-1 text-[11px] text-text-muted">
              References the original file path without duplicating data
            </span>
          </button>

          {/* List of Linked BYOM Models */}
          {byomModels.length > 0 && (
            <div className="mt-4 space-y-2.5">
              {byomModels.map((byom) => (
                <div
                  key={byom.id}
                  className={`rounded-lg border p-3 text-xs ${
                    !byom.file_exists
                      ? "border-status-danger/40 bg-status-danger/8"
                      : byom.is_active
                        ? "border-primary-container/40 bg-surface-container"
                        : "border-border-subtle bg-surface-container"
                  }`}
                >
                  <div className="flex items-start justify-between gap-2">
                    <div className="min-w-0">
                      <div className="font-semibold text-text-primary truncate">
                        {byom.display_name}
                      </div>
                      <div
                        className="mt-0.5 truncate text-[11px] text-text-muted"
                        title={byom.file_path}
                      >
                        {byom.file_path}
                      </div>
                    </div>
                    <span className="shrink-0 text-[11px] text-text-secondary tabular-nums">
                      {formatBytes(byom.file_size_bytes)}
                    </span>
                  </div>

                  {!byom.file_exists && (
                    <div className="mt-2 flex items-center gap-1.5 text-[11px] text-status-danger">
                      <AlertTriangle className="h-3.5 w-3.5 shrink-0" />
                      <span>
                        Local model file was not found. It may have been moved or deleted.
                      </span>
                    </div>
                  )}

                  <div className="mt-2.5 flex flex-wrap items-center gap-2">
                    {!byom.file_exists ? (
                      <button
                        type="button"
                        disabled={actionBusy}
                        onClick={() => void handleRelink(byom.id)}
                        className="inline-flex items-center gap-1 rounded-md bg-tag-optimize px-2.5 py-1 text-[11px] font-semibold text-on-primary hover:bg-tag-optimize/90 cursor-pointer"
                      >
                        <FolderSearch className="h-3 w-3" />
                        Locate File
                      </button>
                    ) : byom.is_active ? (
                      <span className="inline-flex items-center gap-1 text-[11px] font-semibold text-primary">
                        <CheckCircle2 className="h-3.5 w-3.5" />
                        Active
                      </span>
                    ) : (
                      <button
                        type="button"
                        disabled={actionBusy}
                        onClick={() => void onActivate(byom.id)}
                        className="rounded-md bg-primary-container px-2.5 py-1 text-[11px] font-semibold text-on-primary hover:bg-primary-container/90 cursor-pointer"
                      >
                        Activate
                      </button>
                    )}

                    <button
                      type="button"
                      disabled={actionBusy}
                      onClick={() => void onDelete(byom.id)}
                      className="rounded-md border border-border-default bg-surface-elevated px-2.5 py-1 text-[11px] text-text-secondary hover:text-status-danger cursor-pointer"
                    >
                      Remove Reference
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </article>
    </section>
  );
}

import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  AlertTriangle,
  FolderOpen,
  HardDrive,
  Pause,
  Play,
  ShieldCheck,
  X,
} from "lucide-react";
import {
  formatBytes,
  formatEta,
  ipc,
  type CatalogModel,
  type DiskSpaceInfo,
  type DownloadProgressEvent,
  type DownloadTask,
} from "../../lib/ipc";

interface ConsentDownloadCardProps {
  model: CatalogModel;
  defaultModelDir: string;
  existingTask?: DownloadTask;
  liveProgress?: DownloadProgressEvent;
  onStartDownload: (modelId: string, targetDir: string) => Promise<void>;
  onPauseDownload: (modelId: string) => Promise<void>;
  onResumeDownload: (modelId: string) => Promise<void>;
  onCancelDownload: (modelId: string) => Promise<void>;
  onClose: () => void;
}

const SAFETY_MARGIN_BYTES = 512 * 1024 * 1024;

export function ConsentDownloadCard({
  model,
  defaultModelDir,
  existingTask,
  liveProgress,
  onStartDownload,
  onPauseDownload,
  onResumeDownload,
  onCancelDownload,
  onClose,
}: ConsentDownloadCardProps) {
  const initialDir = (() => {
    if (existingTask?.target_path) {
      const idx = Math.max(
        existingTask.target_path.lastIndexOf("\\"),
        existingTask.target_path.lastIndexOf("/")
      );
      if (idx > 0) return existingTask.target_path.slice(0, idx);
    }
    return defaultModelDir;
  })();

  const [targetDir, setTargetDir] = useState(initialDir);
  const [diskInfo, setDiskInfo] = useState<DiskSpaceInfo | null>(null);
  const [diskError, setDiskError] = useState<string | null>(null);

  useEffect(() => {
    setTargetDir(initialDir);
  }, [initialDir]);

  useEffect(() => {
    if (!targetDir) return;
    let active = true;
    setDiskError(null);
    void ipc
      .checkDiskSpace(targetDir)
      .then((info) => {
        if (active) setDiskInfo(info);
      })
      .catch((err) => {
        if (active) {
          setDiskInfo(null);
          setDiskError(err instanceof Error ? err.message : String(err));
        }
      });
    return () => {
      active = false;
    };
  }, [targetDir]);

  const status = liveProgress?.status ?? existingTask?.status ?? "awaiting_consent";
  const isDownloading = status === "downloading";
  const isVerifying = status === "verifying";
  const isPaused = status === "paused";
  const isFailed = status === "failed";

  const bytesDownloaded =
    liveProgress?.bytes_downloaded ?? existingTask?.bytes_downloaded ?? 0;
  const totalBytes = liveProgress?.total_bytes ?? model.file_size_bytes;
  const percent =
    liveProgress?.percent ??
    (totalBytes > 0 ? Math.min(100, (bytesDownloaded / totalBytes) * 100) : 0);

  const remainingBytes = Math.max(0, model.file_size_bytes - bytesDownloaded);
  const hasEnoughSpace =
    diskInfo != null &&
    diskInfo.writable &&
    diskInfo.free_bytes >= remainingBytes + SAFETY_MARGIN_BYTES;

  const handleChangeFolder = async () => {
    const picked = await open({
      title: "Select Local Model Storage Folder",
      directory: true,
      multiple: false,
      defaultPath: targetDir || undefined,
    });
    if (typeof picked === "string" && picked.length > 0) {
      setTargetDir(picked);
    }
  };

  return (
    <section
      aria-label="Explicit AI Model Download Confirmation"
      className="rounded-xl border border-primary-container/50 bg-surface-card p-5 space-y-4"
    >
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <div className="inline-flex items-center gap-1.5 rounded-full border border-tag-security/30 bg-tag-security/12 px-2.5 py-0.5 text-[11px] font-medium text-tag-security mb-2">
            <ShieldCheck className="h-3.5 w-3.5" />
            Consent-First Mandate • Explicit User Approval
          </div>
          <h3 className="text-base font-semibold leading-6 text-text-primary">
            Confirm AI Model Download — {model.display_name} ({model.quantization})
          </h3>
        </div>

        {!isDownloading && !isVerifying && (
          <button
            type="button"
            onClick={onClose}
            aria-label="Close download confirmation"
            className="rounded-lg p-1.5 text-text-secondary hover:bg-surface-hover hover:text-text-primary cursor-pointer"
          >
            <X className="h-4 w-4" />
          </button>
        )}
      </div>

      {/* 4 Transparent Info Blocks */}
      <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-3 text-xs">
        <div className="rounded-lg border border-border-default bg-surface-elevated p-3.5">
          <div className="text-text-muted">1. Selected Model & Source</div>
          <div className="mt-1.5 font-semibold text-text-primary">
            {model.display_name} ({model.quantization})
          </div>
          <div className="mt-1 text-text-secondary">
            Official HuggingFace: {model.repo_id}
          </div>
        </div>

        <div className="rounded-lg border border-border-default bg-surface-elevated p-3.5">
          <div className="text-text-muted">2. Transfer Size</div>
          <div className="mt-1.5 font-semibold text-text-primary tabular-nums">
            {formatBytes(model.file_size_bytes)}
          </div>
          <div className="mt-1 text-text-secondary">
            HTTP Range Resumable (pausable)
          </div>
        </div>

        <div className="rounded-lg border border-border-default bg-surface-elevated p-3.5">
          <div className="flex items-center justify-between gap-2 text-text-muted">
            <span>3. Local Save Location</span>
            {!isDownloading && !isPaused && !isVerifying && (
              <button
                type="button"
                onClick={() => void handleChangeFolder()}
                className="inline-flex items-center gap-1 text-[11px] font-medium text-primary hover:underline cursor-pointer"
              >
                <FolderOpen className="h-3 w-3" />
                Change Folder
              </button>
            )}
          </div>
          <div
            className="mt-1.5 truncate font-medium text-text-primary"
            title={targetDir}
          >
            {targetDir || "Loading default path..."}
          </div>
          <div className="mt-1 flex items-center gap-1.5 text-text-secondary tabular-nums">
            <HardDrive className="h-3 w-3 text-text-muted" />
            <span>
              Free Disk Space:{" "}
              {diskInfo ? formatBytes(diskInfo.free_bytes) : "Checking..."}
            </span>
          </div>
        </div>

        <div className="rounded-lg border border-border-default bg-surface-elevated p-3.5">
          <div className="text-text-muted">4. Integrity & Security</div>
          <div className="mt-1.5 font-semibold text-tag-security">
            Automatic SHA-256
          </div>
          <div className="mt-1 truncate text-text-secondary tabular-nums" title={model.sha256}>
            {model.sha256.slice(0, 16)}...
          </div>
        </div>
      </div>

      {/* Disk warning */}
      {((diskInfo && !hasEnoughSpace) || diskError) && (
        <div className="flex items-center gap-2 rounded-lg border border-status-danger/40 bg-status-danger/10 px-3.5 py-2.5 text-xs text-status-danger">
          <AlertTriangle className="h-4 w-4 shrink-0" />
          <span>
            {diskError ??
              (!diskInfo?.writable
                ? "Destination folder is not writable. Please choose another folder."
                : `Insufficient disk space. At least ${formatBytes(remainingBytes + SAFETY_MARGIN_BYTES)} required.`)}
          </span>
        </div>
      )}

      {/* Live Download Progress Area */}
      {(isDownloading || isPaused || isVerifying || isFailed) && (
        <div className="rounded-lg border border-border-default bg-surface-container p-4 space-y-2.5">
          <div className="flex flex-wrap items-center justify-between gap-2 text-xs">
            <span className="font-medium text-text-primary">
              {isVerifying
                ? "Verifying SHA-256 checksum integrity..."
                : isPaused
                  ? "Download Paused (progress safely saved on disk)"
                  : isFailed
                    ? "Download Failed"
                    : "Downloading model weights from HuggingFace..."}
            </span>
            <span className="text-text-secondary tabular-nums">
              {formatBytes(bytesDownloaded)} / {formatBytes(totalBytes)} ({percent.toFixed(1)}%)
              {isDownloading && liveProgress
                ? ` • ${liveProgress.speed_mb_per_sec.toFixed(1)} MB/s • ETA ${formatEta(liveProgress.eta_seconds)}`
                : ""}
            </span>
          </div>

          <div className="h-2 w-full overflow-hidden rounded-full bg-surface-elevated">
            <div
              className={`h-full rounded-full transition-all duration-200 ${
                isFailed
                  ? "bg-status-danger"
                  : isPaused
                    ? "bg-tag-optimize"
                    : "bg-primary-container"
              }`}
              style={{ width: `${percent}%` }}
            />
          </div>

          {(existingTask?.error_msg || liveProgress?.error_msg) && (
            <p className="text-xs text-tag-optimize">
              {liveProgress?.error_msg ?? existingTask?.error_msg}
            </p>
          )}
        </div>
      )}

      {/* Action Buttons */}
      <div className="flex flex-wrap items-center justify-end gap-2.5 pt-1">
        {isDownloading ? (
          <>
            <button
              type="button"
              onClick={() => void onCancelDownload(model.id)}
              className="rounded-lg border border-border-default bg-surface-elevated px-3.5 py-2 text-xs font-medium text-text-secondary hover:text-status-danger cursor-pointer"
            >
              Cancel
            </button>
            <button
              type="button"
              onClick={() => void onPauseDownload(model.id)}
              className="inline-flex items-center gap-1.5 rounded-lg border border-tag-optimize/40 bg-tag-optimize/15 px-4 py-2 text-xs font-semibold text-tag-optimize hover:bg-tag-optimize/25 cursor-pointer"
            >
              <Pause className="h-3.5 w-3.5" />
              Pause Download
            </button>
          </>
        ) : isPaused || isFailed ? (
          <>
            <button
              type="button"
              onClick={() => void onCancelDownload(model.id)}
              className="rounded-lg border border-border-default bg-surface-elevated px-3.5 py-2 text-xs font-medium text-text-secondary hover:text-status-danger cursor-pointer"
            >
              Cancel & Delete .part
            </button>
            <button
              type="button"
              onClick={() => void onResumeDownload(model.id)}
              className="inline-flex items-center gap-1.5 rounded-lg bg-primary-container px-4 py-2 text-xs font-semibold text-on-primary hover:bg-primary-container/90 cursor-pointer"
            >
              <Play className="h-3.5 w-3.5" />
              Resume Download
            </button>
          </>
        ) : isVerifying ? (
          <span className="text-xs font-medium text-tag-security">
            Verifying SHA-256...
          </span>
        ) : (
          <>
            <button
              type="button"
              onClick={onClose}
              className="rounded-lg border border-border-default bg-surface-elevated px-4 py-2 text-xs font-medium text-text-secondary hover:text-text-primary cursor-pointer"
            >
              Cancel
            </button>
            <button
              type="button"
              disabled={!targetDir || (diskInfo != null && !hasEnoughSpace)}
              onClick={() => void onStartDownload(model.id, targetDir)}
              className="rounded-lg bg-primary-container px-4 py-2 text-xs font-semibold text-on-primary hover:bg-primary-container/90 disabled:opacity-50 cursor-pointer"
            >
              Agree & Start Download ({formatBytes(model.file_size_bytes)})
            </button>
          </>
        )}
      </div>
    </section>
  );
}

import { useEffect } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { AlertCircle, FileUp, RefreshCw, ShieldCheck, X } from "lucide-react";
import { useEngineStore } from "../../stores/useEngineStore";
import { useModelsStore } from "../../stores/useModelsStore";
import { useSettingsStore } from "../../stores/useSettingsStore";
import { ConsentDownloadCard } from "./ConsentDownloadCard";
import { EngineFooter } from "./EngineFooter";
import { HardwareBanner } from "./HardwareBanner";
import { ModelCardGrid } from "./ModelCardGrid";

export function ModelManagerPage() {
  const hardware = useEngineStore((s) => s.hardware);
  const scanning = useEngineStore((s) => s.scanning);
  const scanHardware = useEngineStore((s) => s.scanHardware);
  const setDiagnosticsOpen = useEngineStore((s) => s.setDiagnosticsOpen);

  const settings = useSettingsStore((s) => s.settings);

  const catalog = useModelsStore((s) => s.catalog);
  const installed = useModelsStore((s) => s.installed);
  const downloads = useModelsStore((s) => s.downloads);
  const progressByModel = useModelsStore((s) => s.progressByModel);
  const consentModelId = useModelsStore((s) => s.consentModelId);
  const actionBusy = useModelsStore((s) => s.actionBusy);
  const actionError = useModelsStore((s) => s.actionError);
  const openConsent = useModelsStore((s) => s.openConsent);
  const closeConsent = useModelsStore((s) => s.closeConsent);
  const clearError = useModelsStore((s) => s.clearError);
  const startDownload = useModelsStore((s) => s.startDownload);
  const pauseDownload = useModelsStore((s) => s.pauseDownload);
  const resumeDownload = useModelsStore((s) => s.resumeDownload);
  const cancelDownload = useModelsStore((s) => s.cancelDownload);
  const importLocalGguf = useModelsStore((s) => s.importLocalGguf);
  const relinkByom = useModelsStore((s) => s.relinkByom);
  const activateModel = useModelsStore((s) => s.activateModel);
  const deleteModel = useModelsStore((s) => s.deleteModel);

  useEffect(() => {
    void setDiagnosticsOpen(true);
    return () => {
      void setDiagnosticsOpen(false);
    };
  }, [setDiagnosticsOpen]);

  const activeConsentModel = (() => {
    if (consentModelId) {
      return catalog.find((c) => c.id === consentModelId) ?? null;
    }
    const activeDl = downloads.find((d) => d.status === "downloading" || d.status === "verifying");
    if (activeDl) {
      return catalog.find((c) => c.id === activeDl.model_id) ?? null;
    }
    return null;
  })();

  const handleHeaderImportGguf = async () => {
    const selected = await open({
      title: "Import Local .gguf Model File",
      multiple: false,
      directory: false,
      filters: [{ name: "GGUF Model Weights", extensions: ["gguf"] }],
    });
    if (typeof selected === "string" && selected.length > 0) {
      await importLocalGguf(selected);
    }
  };

  return (
    <div className="w-full space-y-5">
      {/* Header & Quick Actions */}
      <header className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <div className="inline-flex items-center gap-1.5 rounded-full border border-tag-security/30 bg-tag-security/12 px-2.5 py-0.5 text-[11px] font-medium text-tag-security mb-2">
            <ShieldCheck className="h-3.5 w-3.5" />
            Zero Network Egress • Consent-First
          </div>
          <h1 className="text-2xl font-semibold leading-8 tracking-tight text-text-primary">
            Model Manager & Hardware Diagnostics
          </h1>
        </div>

        <div className="flex flex-wrap items-center gap-2.5">
          <button
            type="button"
            disabled={scanning}
            onClick={() => void scanHardware()}
            className="inline-flex items-center gap-2 rounded-lg border border-border-default bg-surface-elevated px-3.5 py-2 text-xs font-medium text-text-primary hover:bg-surface-hover disabled:opacity-50 cursor-pointer"
          >
            <RefreshCw className={`h-3.5 w-3.5 ${scanning ? "animate-spin" : ""}`} />
            Rescan Hardware
          </button>

          <button
            type="button"
            disabled={actionBusy}
            onClick={() => void handleHeaderImportGguf()}
            className="inline-flex items-center gap-2 rounded-lg border border-tag-convert/40 bg-tag-convert/15 px-3.5 py-2 text-xs font-medium text-text-primary hover:bg-tag-convert/25 disabled:opacity-50 cursor-pointer"
          >
            <FileUp className="h-3.5 w-3.5 text-tag-convert" />
            Import Local .gguf
          </button>
        </div>
      </header>

      {/* Action Error Banner */}
      {actionError && (
        <div
          role="alert"
          className="flex items-center justify-between gap-3 rounded-xl border border-status-danger/40 bg-status-danger/10 px-4 py-3 text-sm text-status-danger"
        >
          <div className="flex items-center gap-2">
            <AlertCircle className="h-4 w-4 shrink-0" />
            <span>{actionError.message}</span>
          </div>
          <button
            type="button"
            onClick={clearError}
            aria-label="Dismiss error message"
            className="rounded-lg p-1 hover:bg-surface-hover cursor-pointer"
          >
            <X className="h-4 w-4" />
          </button>
        </div>
      )}

      {/* Hardware Diagnostics */}
      <HardwareBanner hardware={hardware} scanning={scanning} />

      {/* Explicit Consent Download Card */}
      {activeConsentModel && (
        <ConsentDownloadCard
          model={activeConsentModel}
          defaultModelDir={settings?.default_model_dir ?? ""}
          existingTask={downloads.find((d) => d.model_id === activeConsentModel.id)}
          liveProgress={progressByModel[activeConsentModel.id]}
          onStartDownload={startDownload}
          onPauseDownload={pauseDownload}
          onResumeDownload={resumeDownload}
          onCancelDownload={cancelDownload}
          onClose={closeConsent}
        />
      )}

      {/* 3-Card Model Selection Grid */}
      <ModelCardGrid
        catalog={catalog}
        installed={installed}
        downloads={downloads}
        progressByModel={progressByModel}
        hardware={hardware}
        actionBusy={actionBusy}
        onOpenConsent={openConsent}
        onImportGguf={importLocalGguf}
        onRelinkByom={relinkByom}
        onActivate={activateModel}
        onDelete={deleteModel}
      />

      {/* Engine Lifecycle Footer */}
      <EngineFooter />
    </div>
  );
}

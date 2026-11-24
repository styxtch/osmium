import { useCallback, useEffect, useRef } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { useGlossaryHistoryStore } from "../../stores/useGlossaryHistoryStore";
import { useModelsStore } from "../../stores/useModelsStore";
import { useSettingsStore } from "../../stores/useSettingsStore";
import { useSubtitleStore } from "../../stores/useSubtitleStore";
import { ActiveFileSpecCard } from "./ActiveFileSpecCard";
import { DropzoneSavedJobs } from "./DropzoneSavedJobs";
import { ProgressBanner } from "./ProgressBanner";
import { SubtitleSearchBar } from "./SubtitleSearchBar";
import { SubtitleVirtualTable } from "./SubtitleVirtualTable";

function buildSuggestedExportPath(
  defaultExportDir: string | undefined,
  fileName: string,
  format: string,
  targetLang?: string
): string {
  const dotIdx = fileName.lastIndexOf(".");
  const stem = dotIdx > 0 ? fileName.slice(0, dotIdx) : fileName;
  const ext = format.toLowerCase();
  const langSuffix = (targetLang || "id").trim().toLowerCase() || "id";
  const outName = `${stem}.${langSuffix}.${ext}`;

  if (!defaultExportDir || !defaultExportDir.trim()) {
    return outName;
  }
  const normalizedDir = defaultExportDir.replace(/[\\/]+$/, "");
  return `${normalizedDir}\\${outName}`;
}

export function SubtitleTranslatorPage() {
  const searchInputRef = useRef<HTMLInputElement | null>(null);

  const jobs = useSubtitleStore((s) => s.jobs);
  const activeJob = useSubtitleStore((s) => s.activeJob);
  const cues = useSubtitleStore((s) => s.cues);
  const searchQuery = useSubtitleStore((s) => s.searchQuery);
  const statusFilter = useSubtitleStore((s) => s.statusFilter);
  const editingCueIndex = useSubtitleStore((s) => s.editingCueIndex);
  const retranslatingCueIndex = useSubtitleStore((s) => s.retranslatingCueIndex);
  const batchTokPerSec = useSubtitleStore((s) => s.batchTokPerSec);
  const isLoadingJobs = useSubtitleStore((s) => s.isLoadingJobs);
  const isLoadingCues = useSubtitleStore((s) => s.isLoadingCues);
  const isParsingFile = useSubtitleStore((s) => s.isParsingFile);
  const isExporting = useSubtitleStore((s) => s.isExporting);
  const lastExportedPath = useSubtitleStore((s) => s.lastExportedPath);
  const error = useSubtitleStore((s) => s.error);

  const refreshJobs = useSubtitleStore((s) => s.refreshJobs);
  const importSubtitleFile = useSubtitleStore((s) => s.importSubtitleFile);
  const selectJob = useSubtitleStore((s) => s.selectJob);
  const closeActiveJob = useSubtitleStore((s) => s.closeActiveJob);
  const deleteJob = useSubtitleStore((s) => s.deleteJob);
  const updateJobOptions = useSubtitleStore((s) => s.updateJobOptions);
  const setSearchQuery = useSubtitleStore((s) => s.setSearchQuery);
  const setStatusFilter = useSubtitleStore((s) => s.setStatusFilter);
  const startJob = useSubtitleStore((s) => s.startJob);
  const pauseJob = useSubtitleStore((s) => s.pauseJob);
  const resumeJob = useSubtitleStore((s) => s.resumeJob);
  const cancelJob = useSubtitleStore((s) => s.cancelJob);
  const setEditingCueIndex = useSubtitleStore((s) => s.setEditingCueIndex);
  const saveCueEdit = useSubtitleStore((s) => s.saveCueEdit);
  const retranslateRow = useSubtitleStore((s) => s.retranslateRow);
  const exportSubtitles = useSubtitleStore((s) => s.exportSubtitles);
  const clearError = useSubtitleStore((s) => s.clearError);

  const glossaries = useGlossaryHistoryStore((s) => s.glossaries);
  const fetchGlossaries = useGlossaryHistoryStore((s) => s.fetchGlossaries);
  const installedModels = useModelsStore((s) => s.installed);
  const activeModel =
    installedModels.find((m) => m.is_active && m.file_exists) ?? null;
  const settings = useSettingsStore((s) => s.settings);

  useEffect(() => {
    void refreshJobs();
    void fetchGlossaries();
  }, [fetchGlossaries, refreshJobs]);

  const handleExport = useCallback(async () => {
    if (!activeJob || activeJob.status !== "completed") return;

    const ext = activeJob.format.toLowerCase();
    const suggestedPath = buildSuggestedExportPath(
      settings?.default_export_dir,
      activeJob.file_name,
      ext,
      activeJob.target_lang
    );

    const destPath = await save({
      title: "Export Translated Subtitle",
      defaultPath: suggestedPath,
      filters: [
        {
          name: `${ext.toUpperCase()} Subtitle`,
          extensions: [ext],
        },
      ],
    });

    if (typeof destPath === "string" && destPath.trim()) {
      await exportSubtitles(destPath);
    }
  }, [activeJob, exportSubtitles, settings?.default_export_dir]);

  useEffect(() => {
    if (!activeJob) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "f") {
        e.preventDefault();
        searchInputRef.current?.focus();
        searchInputRef.current?.select();
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
        if (activeJob.status === "completed") {
          e.preventDefault();
          void handleExport();
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [activeJob, handleExport]);

  // State A: No active job selected -> Dropzone & Saved Resumable Jobs
  if (!activeJob) {
    return (
      <div className="w-full flex-1 min-h-0 flex flex-col">
        {error && (
          <div className="mb-4 flex items-center justify-between gap-2 px-4 py-2.5 rounded-xl bg-status-danger/10 border border-status-danger/30 text-xs text-status-danger">
            <span>{error}</span>
            <button
              type="button"
              onClick={clearError}
              className="text-xs underline hover:no-underline"
            >
              Dismiss
            </button>
          </div>
        )}
        <DropzoneSavedJobs
          jobs={jobs}
          glossaries={glossaries}
          isLoadingJobs={isLoadingJobs}
          isParsingFile={isParsingFile}
          onImportFile={async (
            filePath,
            sourceLang,
            targetLang,
            genre,
            glossaryId,
            tagShieldEnabled
          ) => {
            await importSubtitleFile(
              filePath,
              sourceLang,
              targetLang,
              genre,
              glossaryId,
              tagShieldEnabled
            );
          }}
          onSelectJob={selectJob}
          onDeleteJob={deleteJob}
        />
      </div>
    );
  }

  // State B: Active Subtitle Job Inspection, Translation & Review Workspace
  const hasActiveModel = Boolean(activeModel);

  return (
    <div className="w-full flex-1 min-h-0 flex flex-col gap-3">
      <ActiveFileSpecCard
        job={activeJob}
        glossaries={glossaries}
        activeModelName={activeModel?.display_name ?? null}
        hasActiveModel={hasActiveModel}
        isExporting={isExporting}
        onBackToJobs={closeActiveJob}
        onUpdateOptions={(
          sourceLang,
          targetLang,
          genre,
          glossaryId,
          tagShieldEnabled
        ) =>
          void updateJobOptions(
            sourceLang,
            targetLang,
            genre,
            glossaryId,
            tagShieldEnabled
          )
        }
        onStart={() => void startJob()}
        onPause={() => void pauseJob()}
        onResume={() => void resumeJob()}
        onCancel={() => void cancelJob()}
        onExport={() => void handleExport()}
      />

      <ProgressBanner
        job={activeJob}
        batchTokPerSec={batchTokPerSec}
        error={error}
        lastExportedPath={lastExportedPath}
        onFilterWarnings={() => setStatusFilter("warning")}
        onDismissError={clearError}
      />

      <SubtitleSearchBar
        inputRef={searchInputRef}
        searchQuery={searchQuery}
        statusFilter={statusFilter}
        visibleCount={cues.length}
        onSearchChange={setSearchQuery}
        onFilterChange={setStatusFilter}
      />

      <SubtitleVirtualTable
        cues={cues}
        targetLang={activeJob.target_lang || "id"}
        jobStatus={activeJob.status}
        tagShieldEnabled={activeJob.tag_shield_enabled}
        editingCueIndex={editingCueIndex}
        retranslatingCueIndex={retranslatingCueIndex}
        isLoadingCues={isLoadingCues}
        onStartEdit={setEditingCueIndex}
        onSaveEdit={saveCueEdit}
        onRetranslateRow={retranslateRow}
      />
    </div>
  );
}

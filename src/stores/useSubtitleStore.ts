import { create } from "zustand";
import {
  ipc,
  parseAppError,
  type SubtitleCueDto,
  type SubtitleCueFilter,
  type SubtitleGenre,
  type SubtitleJobSummary,
} from "../lib/ipc";

interface SubtitleState {
  jobs: SubtitleJobSummary[];
  activeJobId: string | null;
  activeJob: SubtitleJobSummary | null;
  cues: SubtitleCueDto[];
  searchQuery: string;
  statusFilter: SubtitleCueFilter;
  editingCueIndex: number | null;
  retranslatingCueIndex: number | null;
  batchTokPerSec: number | null;
  isLoadingJobs: boolean;
  isLoadingCues: boolean;
  isParsingFile: boolean;
  isExporting: boolean;
  lastExportedPath: string | null;
  error: string | null;

  initSubtitleListeners: () => Promise<() => void>;
  refreshJobs: () => Promise<SubtitleJobSummary[]>;
  importSubtitleFile: (
    filePath: string,
    genre?: SubtitleGenre,
    glossaryId?: string | null,
    tagShieldEnabled?: boolean
  ) => Promise<SubtitleJobSummary | null>;
  selectJob: (jobId: string) => Promise<void>;
  closeActiveJob: () => void;
  deleteJob: (jobId: string) => Promise<void>;
  updateJobOptions: (
    genre: SubtitleGenre,
    glossaryId: string | null,
    tagShieldEnabled: boolean
  ) => Promise<void>;
  refreshCues: () => Promise<void>;
  setSearchQuery: (query: string) => void;
  setStatusFilter: (filter: SubtitleCueFilter) => void;
  startJob: () => Promise<void>;
  pauseJob: () => Promise<void>;
  resumeJob: () => Promise<void>;
  cancelJob: () => Promise<void>;
  setEditingCueIndex: (cueIndex: number | null) => void;
  saveCueEdit: (cueIndex: number, editedText: string) => Promise<boolean>;
  retranslateRow: (cueIndex: number) => Promise<void>;
  exportSubtitles: (destPath: string) => Promise<string | null>;
  clearError: () => void;
}

const CUE_PAGE_LIMIT = 5000;

async function fetchFilteredCueList(
  jobId: string,
  searchQuery: string,
  statusFilter: SubtitleCueFilter
): Promise<SubtitleCueDto[]> {
  const all: SubtitleCueDto[] = [];
  let offset = 0;

  while (true) {
    const chunk = await ipc.getSubtitleCues({
      jobId,
      offset,
      limit: CUE_PAGE_LIMIT,
      searchQuery: searchQuery.trim() || undefined,
      statusFilter,
    });
    all.push(...chunk);
    if (chunk.length < CUE_PAGE_LIMIT) {
      break;
    }
    offset += CUE_PAGE_LIMIT;
  }

  return all;
}

export const useSubtitleStore = create<SubtitleState>((set, get) => ({
  jobs: [],
  activeJobId: null,
  activeJob: null,
  cues: [],
  searchQuery: "",
  statusFilter: "all",
  editingCueIndex: null,
  retranslatingCueIndex: null,
  batchTokPerSec: null,
  isLoadingJobs: false,
  isLoadingCues: false,
  isParsingFile: false,
  isExporting: false,
  lastExportedPath: null,
  error: null,

  initSubtitleListeners: async () => {
    const unlistenStarted = await ipc.onSubtitleBatchStarted((evt) => {
      const { activeJobId, cues } = get();
      if (activeJobId !== evt.job_id) return;

      const processingSet = new Set(evt.cue_indices);
      set({
        cues: cues.map((cue) =>
          processingSet.has(cue.cue_index)
            ? { ...cue, batch_number: evt.batch_number, status: "processing" }
            : cue
        ),
      });
    });

    const unlistenCompleted = await ipc.onSubtitleBatchCompleted((evt) => {
      const { activeJobId, activeJob, cues, jobs, searchQuery, statusFilter } =
        get();

      const updatedJobs = jobs.map((j) =>
        j.id === evt.job_id
          ? {
              ...j,
              completed_cues: evt.completed_cues,
              total_cues: evt.total_cues,
              self_healing_errors: evt.self_healing_errors,
              status: evt.status,
            }
          : j
      );

      if (activeJobId !== evt.job_id) {
        set({ jobs: updatedJobs });
        return;
      }

      const patchMap = new Map(evt.cues.map((c) => [c.cue_index, c]));
      const nextCues = cues.map((cue) => {
        const updated = patchMap.get(cue.cue_index);
        if (updated) return updated;
        if (
          (evt.status === "paused" ||
            evt.status === "cancelled" ||
            evt.status === "error") &&
          cue.status === "processing"
        ) {
          return { ...cue, status: "queued" as const };
        }
        return cue;
      });

      const nextActiveJob = activeJob
        ? {
            ...activeJob,
            completed_cues: evt.completed_cues,
            total_cues: evt.total_cues,
            self_healing_errors: evt.self_healing_errors,
            status: evt.status,
          }
        : null;

      set({
        jobs: updatedJobs,
        activeJob: nextActiveJob,
        cues: nextCues,
        batchTokPerSec:
          evt.tok_per_sec > 0 ? evt.tok_per_sec : get().batchTokPerSec,
      });

      if (statusFilter !== "all" || searchQuery.trim() !== "") {
        void get().refreshCues();
      }
    });

    const unlistenError = await ipc.onSubtitleJobError((evt) => {
      const { activeJobId, activeJob, jobs, cues } = get();
      const updatedJobs = jobs.map((j) =>
        j.id === evt.job_id ? { ...j, status: "error" as const } : j
      );
      if (activeJobId !== evt.job_id) {
        set({ jobs: updatedJobs });
        return;
      }

      set({
        jobs: updatedJobs,
        activeJob: activeJob ? { ...activeJob, status: "error" } : null,
        cues: cues.map((c) =>
          c.status === "processing" ? { ...c, status: "queued" } : c
        ),
        error: evt.message,
      });
    });

    return () => {
      unlistenStarted();
      unlistenCompleted();
      unlistenError();
    };
  },

  refreshJobs: async () => {
    set({ isLoadingJobs: true });
    try {
      const jobs = await ipc.listSubtitleJobs();
      const activeId = get().activeJobId;
      const syncedActive = activeId
        ? jobs.find((j) => j.id === activeId) ?? get().activeJob
        : null;
      set({ jobs, activeJob: syncedActive, isLoadingJobs: false });
      return jobs;
    } catch (err) {
      set({ isLoadingJobs: false, error: parseAppError(err).message });
      return [];
    }
  },

  importSubtitleFile: async (
    filePath,
    genre = "general",
    glossaryId = null,
    tagShieldEnabled = true
  ) => {
    set({ isParsingFile: true, error: null, lastExportedPath: null });
    try {
      const job = await ipc.parseAndCreateSubtitleJob({
        filePath,
        genre,
        glossaryId,
        tagShieldEnabled,
      });
      set({
        activeJobId: job.id,
        activeJob: job,
        searchQuery: "",
        statusFilter: "all",
        editingCueIndex: null,
        batchTokPerSec: null,
        isParsingFile: false,
      });
      await Promise.all([get().refreshJobs(), get().refreshCues()]);
      return job;
    } catch (err) {
      set({ isParsingFile: false, error: parseAppError(err).message });
      return null;
    }
  },

  selectJob: async (jobId) => {
    set({
      activeJobId: jobId,
      searchQuery: "",
      statusFilter: "all",
      editingCueIndex: null,
      lastExportedPath: null,
      batchTokPerSec: null,
      error: null,
      isLoadingCues: true,
    });
    try {
      const [job, cues] = await Promise.all([
        ipc.getSubtitleJob(jobId),
        fetchFilteredCueList(jobId, "", "all"),
      ]);
      set({ activeJob: job, cues, isLoadingCues: false });
    } catch (err) {
      set({ isLoadingCues: false, error: parseAppError(err).message });
    }
  },

  closeActiveJob: () => {
    set({
      activeJobId: null,
      activeJob: null,
      cues: [],
      searchQuery: "",
      statusFilter: "all",
      editingCueIndex: null,
      lastExportedPath: null,
      error: null,
    });
    void get().refreshJobs();
  },

  deleteJob: async (jobId) => {
    set({ error: null });
    try {
      await ipc.deleteSubtitleJob(jobId);
      if (get().activeJobId === jobId) {
        set({ activeJobId: null, activeJob: null, cues: [] });
      }
      await get().refreshJobs();
    } catch (err) {
      set({ error: parseAppError(err).message });
    }
  },

  updateJobOptions: async (genre, glossaryId, tagShieldEnabled) => {
    const jobId = get().activeJobId;
    if (!jobId) return;
    set({ error: null });
    try {
      const updated = await ipc.updateSubtitleJobOptions({
        jobId,
        genre,
        glossaryId,
        tagShieldEnabled,
      });
      set((s) => ({
        activeJob: updated,
        jobs: s.jobs.map((j) => (j.id === jobId ? updated : j)),
      }));
    } catch (err) {
      set({ error: parseAppError(err).message });
    }
  },

  refreshCues: async () => {
    const { activeJobId, searchQuery, statusFilter } = get();
    if (!activeJobId) return;
    set({ isLoadingCues: true });
    try {
      const cues = await fetchFilteredCueList(
        activeJobId,
        searchQuery,
        statusFilter
      );
      set({ cues, isLoadingCues: false });
    } catch (err) {
      set({ isLoadingCues: false, error: parseAppError(err).message });
    }
  },

  setSearchQuery: (query) => {
    set({ searchQuery: query });
    void get().refreshCues();
  },

  setStatusFilter: (filter) => {
    set({ statusFilter: filter });
    void get().refreshCues();
  },

  startJob: async () => {
    const jobId = get().activeJobId;
    if (!jobId) return;
    set({ error: null, editingCueIndex: null, lastExportedPath: null });
    try {
      const summary = await ipc.startSubtitleJob(jobId);
      set((s) => ({
        activeJob: summary,
        jobs: s.jobs.map((j) => (j.id === jobId ? summary : j)),
      }));
    } catch (err) {
      set({ error: parseAppError(err).message });
    }
  },

  pauseJob: async () => {
    const jobId = get().activeJobId;
    if (!jobId) return;
    set({ error: null });
    try {
      const summary = await ipc.pauseSubtitleJob(jobId);
      set((s) => ({
        activeJob: summary,
        jobs: s.jobs.map((j) => (j.id === jobId ? summary : j)),
        cues: s.cues.map((c) =>
          c.status === "processing" ? { ...c, status: "queued" } : c
        ),
      }));
    } catch (err) {
      set({ error: parseAppError(err).message });
    }
  },

  resumeJob: async () => {
    const jobId = get().activeJobId;
    if (!jobId) return;
    set({ error: null, editingCueIndex: null });
    try {
      const summary = await ipc.resumeSubtitleJob(jobId);
      set((s) => ({
        activeJob: summary,
        jobs: s.jobs.map((j) => (j.id === jobId ? summary : j)),
      }));
    } catch (err) {
      set({ error: parseAppError(err).message });
    }
  },

  cancelJob: async () => {
    const jobId = get().activeJobId;
    if (!jobId) return;
    set({ error: null });
    try {
      const summary = await ipc.cancelSubtitleJob(jobId);
      set((s) => ({
        activeJob: summary,
        jobs: s.jobs.map((j) => (j.id === jobId ? summary : j)),
        cues: s.cues.map((c) =>
          c.status === "processing" ? { ...c, status: "queued" } : c
        ),
      }));
    } catch (err) {
      set({ error: parseAppError(err).message });
    }
  },

  setEditingCueIndex: (cueIndex) => {
    set({ editingCueIndex: cueIndex });
  },

  saveCueEdit: async (cueIndex, editedText) => {
    const jobId = get().activeJobId;
    if (!jobId) return false;
    set({ error: null });
    try {
      const updatedCue = await ipc.updateCueTranslation({
        jobId,
        cueIndex,
        translatedText: editedText,
      });
      const updatedJob = await ipc.getSubtitleJob(jobId);
      set((s) => ({
        activeJob: updatedJob,
        jobs: s.jobs.map((j) => (j.id === jobId ? updatedJob : j)),
        cues: s.cues.map((c) =>
          c.cue_index === cueIndex ? updatedCue : c
        ),
        editingCueIndex: null,
      }));
      return true;
    } catch (err) {
      set({ error: parseAppError(err).message });
      return false;
    }
  },

  retranslateRow: async (cueIndex) => {
    const jobId = get().activeJobId;
    if (!jobId) return;
    set({ retranslatingCueIndex: cueIndex, error: null });
    try {
      const updatedCue = await ipc.retranslateSingleCue(jobId, cueIndex);
      const updatedJob = await ipc.getSubtitleJob(jobId);
      set((s) => ({
        retranslatingCueIndex: null,
        activeJob: updatedJob,
        jobs: s.jobs.map((j) => (j.id === jobId ? updatedJob : j)),
        cues: s.cues.map((c) =>
          c.cue_index === cueIndex ? updatedCue : c
        ),
      }));
    } catch (err) {
      set({
        retranslatingCueIndex: null,
        error: parseAppError(err).message,
      });
    }
  },

  exportSubtitles: async (destPath) => {
    const jobId = get().activeJobId;
    if (!jobId) return null;
    set({ isExporting: true, error: null });
    try {
      const savedPath = await ipc.exportSubtitleFile(jobId, destPath);
      set({ isExporting: false, lastExportedPath: savedPath });
      return savedPath;
    } catch (err) {
      set({ isExporting: false, error: parseAppError(err).message });
      return null;
    }
  },

  clearError: () => set({ error: null }),
}));

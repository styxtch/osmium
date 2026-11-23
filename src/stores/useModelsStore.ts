import { create } from "zustand";
import {
  ipc,
  parseAppError,
  type AppErrorPayload,
  type CatalogModel,
  type DownloadProgressEvent,
  type DownloadTask,
  type InstalledModelWithHealth,
} from "../lib/ipc";
import { useEngineStore } from "./useEngineStore";

interface ModelsState {
  catalog: CatalogModel[];
  installed: InstalledModelWithHealth[];
  downloads: DownloadTask[];
  progressByModel: Record<string, DownloadProgressEvent>;
  consentModelId: string | null;
  loading: boolean;
  actionBusy: boolean;
  actionError: AppErrorPayload | null;
  refreshModels: () => Promise<InstalledModelWithHealth[]>;
  openConsent: (modelId: string) => void;
  closeConsent: () => void;
  clearError: () => void;
  startDownload: (modelId: string, targetDir: string) => Promise<void>;
  pauseDownload: (modelId: string) => Promise<void>;
  resumeDownload: (modelId: string) => Promise<void>;
  cancelDownload: (modelId: string) => Promise<void>;
  importLocalGguf: (filePath: string) => Promise<void>;
  relinkByom: (modelId: string, newFilePath: string) => Promise<void>;
  activateModel: (modelId: string) => Promise<void>;
  deleteModel: (modelId: string) => Promise<void>;
}

export const useModelsStore = create<ModelsState>((set, get) => ({
  catalog: [],
  installed: [],
  downloads: [],
  progressByModel: {},
  consentModelId: null,
  loading: false,
  actionBusy: false,
  actionError: null,

  refreshModels: async () => {
    set({ loading: true });
    try {
      const overview = await ipc.listModels();
      set({
        catalog: overview.catalog,
        installed: overview.installed,
        downloads: overview.downloads,
        loading: false,
      });
      return overview.installed;
    } catch (err) {
      set({ actionError: parseAppError(err), loading: false });
      return get().installed;
    }
  },

  openConsent: (modelId) => set({ consentModelId: modelId, actionError: null }),
  closeConsent: () => set({ consentModelId: null }),
  clearError: () => set({ actionError: null }),

  startDownload: async (modelId, targetDir) => {
    set({ actionError: null });
    try {
      await ipc.confirmAndStartModelDownload(modelId, targetDir, (evt) => {
        set((state) => ({
          progressByModel: {
            ...state.progressByModel,
            [evt.model_id]: evt,
          },
        }));
      });
    } catch (err) {
      set({ actionError: parseAppError(err) });
    } finally {
      await get().refreshModels();
      await useEngineStore.getState().refreshTelemetry();
    }
  },

  pauseDownload: async (modelId) => {
    set({ actionError: null });
    try {
      await ipc.pauseModelDownload(modelId);
    } catch (err) {
      set({ actionError: parseAppError(err) });
    } finally {
      await get().refreshModels();
    }
  },

  resumeDownload: async (modelId) => {
    set({ actionError: null, consentModelId: modelId });
    try {
      await ipc.resumeModelDownload(modelId, (evt) => {
        set((state) => ({
          progressByModel: {
            ...state.progressByModel,
            [evt.model_id]: evt,
          },
        }));
      });
    } catch (err) {
      set({ actionError: parseAppError(err) });
    } finally {
      await get().refreshModels();
      await useEngineStore.getState().refreshTelemetry();
    }
  },

  cancelDownload: async (modelId) => {
    set({ actionError: null });
    try {
      await ipc.cancelModelDownload(modelId);
      set((state) => {
        const nextProgress = { ...state.progressByModel };
        delete nextProgress[modelId];
        return { progressByModel: nextProgress, consentModelId: null };
      });
    } catch (err) {
      set({ actionError: parseAppError(err) });
    } finally {
      await get().refreshModels();
    }
  },

  importLocalGguf: async (filePath) => {
    set({ actionBusy: true, actionError: null });
    try {
      await ipc.importLocalGguf(filePath);
      await get().refreshModels();
      await useEngineStore.getState().refreshTelemetry();
    } catch (err) {
      set({ actionError: parseAppError(err) });
    } finally {
      set({ actionBusy: false });
    }
  },

  relinkByom: async (modelId, newFilePath) => {
    set({ actionBusy: true, actionError: null });
    try {
      await ipc.relinkByomModel(modelId, newFilePath);
      await get().refreshModels();
      await useEngineStore.getState().refreshTelemetry();
    } catch (err) {
      set({ actionError: parseAppError(err) });
    } finally {
      set({ actionBusy: false });
    }
  },

  activateModel: async (modelId) => {
    set({ actionBusy: true, actionError: null });
    try {
      await ipc.activateModel(modelId);
      await get().refreshModels();
      await useEngineStore.getState().refreshTelemetry();
    } catch (err) {
      const parsed = parseAppError(err);
      set({ actionError: parsed });
      if (parsed.code === "BYOM_FILE_MISSING") {
        useEngineStore.getState().setEngineAlert(parsed);
      }
    } finally {
      set({ actionBusy: false });
    }
  },

  deleteModel: async (modelId) => {
    set({ actionBusy: true, actionError: null });
    try {
      await ipc.deleteModel(modelId);
      await get().refreshModels();
      await useEngineStore.getState().refreshTelemetry();
    } catch (err) {
      set({ actionError: parseAppError(err) });
    } finally {
      set({ actionBusy: false });
    }
  },
}));

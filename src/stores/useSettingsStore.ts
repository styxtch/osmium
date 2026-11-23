import { create } from "zustand";
import {
  ipc,
  parseAppError,
  type AppErrorPayload,
  type AppSettings,
} from "../lib/ipc";
import { useEngineStore } from "./useEngineStore";

interface SettingsState {
  settings: AppSettings | null;
  saving: boolean;
  error: AppErrorPayload | null;
  fetchSettings: () => Promise<void>;
  saveSettings: (next: AppSettings) => Promise<boolean>;
}

export const useSettingsStore = create<SettingsState>((set) => ({
  settings: null,
  saving: false,
  error: null,

  fetchSettings: async () => {
    try {
      const settings = await ipc.getSettings();
      set({ settings, error: null });
    } catch (err) {
      set({ error: parseAppError(err) });
    }
  },

  saveSettings: async (next) => {
    set({ saving: true, error: null });
    try {
      const saved = await ipc.updateSettings(next);
      set({ settings: saved, saving: false });
      await useEngineStore.getState().refreshTelemetry();
      return true;
    } catch (err) {
      set({ error: parseAppError(err), saving: false });
      return false;
    }
  },
}));

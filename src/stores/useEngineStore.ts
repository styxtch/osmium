import { create } from "zustand";
import {
  ipc,
  parseAppError,
  type AppErrorPayload,
  type EngineTelemetry,
  type HardwareProfile,
} from "../lib/ipc";

interface EngineState {
  hardware: HardwareProfile | null;
  telemetry: EngineTelemetry | null;
  vramUsedMb: number;
  vramTotalMb: number;
  scanning: boolean;
  error: AppErrorPayload | null;
  engineAlert: AppErrorPayload | null;
  scanHardware: () => Promise<void>;
  refreshTelemetry: () => Promise<void>;
  setDiagnosticsOpen: (open: boolean) => Promise<void>;
  setEngineAlert: (alert: AppErrorPayload | null) => void;
  dismissAlert: () => void;
  initEngineListeners: () => Promise<() => void>;
}

export const useEngineStore = create<EngineState>((set) => ({
  hardware: null,
  telemetry: null,
  vramUsedMb: 0,
  vramTotalMb: 0,
  scanning: false,
  error: null,
  engineAlert: null,

  scanHardware: async () => {
    set({ scanning: true, error: null });
    try {
      const hardware = await ipc.scanHardware();
      set((state) => ({
        hardware,
        vramUsedMb: hardware.current_vram_usage_mb,
        vramTotalMb: hardware.dedicated_vram_mb,
        scanning: false,
        engineAlert:
          !hardware.vulkan_supported && !state.engineAlert
            ? {
                code: "VULKAN_INIT_FAILED",
                message:
                  "GPU does not support Vulkan. Osmium is running in CPU mode (accurate, slightly slower).",
              }
            : state.engineAlert,
      }));
    } catch (err) {
      set({ error: parseAppError(err), scanning: false });
    }
  },

  refreshTelemetry: async () => {
    try {
      const telemetry = await ipc.getEngineTelemetry();
      set({
        telemetry,
        vramUsedMb: telemetry.vram_used_mb,
        vramTotalMb: telemetry.vram_total_mb,
      });
    } catch (err) {
      set({ error: parseAppError(err) });
    }
  },

  setDiagnosticsOpen: async (open) => {
    try {
      await ipc.setDiagnosticsTabOpen(open);
    } catch {
      // Non-fatal UI hint
    }
  },

  setEngineAlert: (alert) => set({ engineAlert: alert }),
  dismissAlert: () => set({ engineAlert: null }),

  initEngineListeners: async () => {
    const unlistenVram = await ipc.onVramTick((tick) => {
      set({
        vramUsedMb: tick.vram_used_mb,
        vramTotalMb: tick.vram_total_mb,
      });
    });
    const unlistenTel = await ipc.onEngineTelemetry((telemetry) => {
      set({
        telemetry,
        vramUsedMb: telemetry.vram_used_mb,
        vramTotalMb: telemetry.vram_total_mb,
      });
    });
    const unlistenErr = await ipc.onEngineError((payload) => {
      set({ engineAlert: payload });
    });

    return () => {
      unlistenVram();
      unlistenTel();
      unlistenErr();
    };
  },
}));

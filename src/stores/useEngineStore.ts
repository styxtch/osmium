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
  gpuTempC: number | null;
  gpuClockMhz: number | null;
  ramUsedGb: number;
  ramTotalGb: number;
  scanning: boolean;
  unloading: boolean;
  error: AppErrorPayload | null;
  engineAlert: AppErrorPayload | null;
  scanHardware: () => Promise<void>;
  refreshTelemetry: () => Promise<void>;
  unloadModel: () => Promise<void>;
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
  gpuTempC: null,
  gpuClockMhz: null,
  ramUsedGb: 0,
  ramTotalGb: 0,
  scanning: false,
  unloading: false,
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
        gpuTempC: hardware.thermal_c ?? state.gpuTempC,
        gpuClockMhz: hardware.clock_mhz ?? state.gpuClockMhz,
        ramTotalGb: hardware.system_ram_gb || state.ramTotalGb,
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

  unloadModel: async () => {
    set({ unloading: true, error: null });
    try {
      const telemetry = await ipc.unloadSidecarModel();
      set({
        telemetry,
        vramUsedMb: telemetry.vram_used_mb,
        vramTotalMb: telemetry.vram_total_mb,
        unloading: false,
      });
    } catch (err) {
      set({ error: parseAppError(err), unloading: false });
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
      set((state) => ({
        vramUsedMb: tick.vram_used_mb,
        vramTotalMb: tick.vram_total_mb,
        gpuTempC:
          tick.gpu_temp_c !== undefined ? tick.gpu_temp_c : state.gpuTempC,
        gpuClockMhz:
          tick.gpu_clock_mhz !== undefined
            ? tick.gpu_clock_mhz
            : state.gpuClockMhz,
        ramUsedGb: tick.ram_used_gb ?? state.ramUsedGb,
        ramTotalGb: tick.ram_total_gb ?? state.ramTotalGb,
      }));
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

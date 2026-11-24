import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const invokeMock = vi.fn();
const listenMock = vi.fn();
const openDialogMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => {
  class MockChannel<T = unknown> {
    id = 1;
    onmessage?: (response: T) => void;
    constructor(cb?: (response: T) => void) {
      this.onmessage = cb;
    }
  }
  return {
    invoke: (...args: unknown[]) => invokeMock(...args),
    Channel: MockChannel,
  };
});

vi.mock("@tauri-apps/api/event", () => ({
  listen: (...args: unknown[]) => listenMock(...args),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: (...args: unknown[]) => openDialogMock(...args),
}));

import App from "../../../App";
import { useEngineStore } from "../../../stores/useEngineStore";
import { useModelsStore } from "../../../stores/useModelsStore";
import { useNavigationStore } from "../../../stores/useNavigationStore";
import { useSettingsStore } from "../../../stores/useSettingsStore";

const BASE_CATALOG = [
  {
    id: "qwen3-8b-q4km",
    display_name: "Qwen 3 8B",
    tier: "optimal_8b",
    quantization: "Q4_K_M",
    repo_id: "Qwen/Qwen3-8B-GGUF",
    commit_sha: "7c41481",
    file_name: "Qwen3-8B-Q4_K_M.gguf",
    download_url: "https://huggingface.co/Qwen/Qwen3-8B-GGUF/resolve/main/Qwen3-8B-Q4_K_M.gguf",
    file_size_bytes: 5027783488,
    estimated_vram_mb: 6200,
    sha256: "d98cdcbd03e17ce47681435b5150e34c1417f50b5c0019dd560e4882c5745785",
    block_count: 36,
    description: "Optimal 8B",
    badge_label: "Recommended 8GB+ GPU",
  },
  {
    id: "qwen3-4b-q4km",
    display_name: "Qwen 3 4B",
    tier: "balanced_4b",
    quantization: "Q4_K_M",
    repo_id: "Qwen/Qwen3-4B-GGUF",
    commit_sha: "bc64014",
    file_name: "Qwen3-4B-Q4_K_M.gguf",
    download_url: "https://huggingface.co/Qwen/Qwen3-4B-GGUF/resolve/main/Qwen3-4B-Q4_K_M.gguf",
    file_size_bytes: 2497280256,
    estimated_vram_mb: 3600,
    sha256: "7485fe6f11af29433bc51cab58009521f205840f5b4ae3a32fa7f92e8534fdf5",
    block_count: 36,
    description: "Balanced 4B",
    badge_label: "VRAM Saver (4–6GB GPU)",
  },
];

const BASE_HARDWARE = {
  gpu_name: "NVIDIA GeForce RTX 3060",
  dedicated_vram_mb: 12288,
  shared_memory_mb: 8112,
  current_vram_usage_mb: 980,
  system_ram_gb: 15.85,
  vulkan_supported: true,
  vulkan_version: "1.4",
  vulkan_device_id: "Vulkan0",
  flash_attention_supported: true,
  recommended_tier: "optimal_8b",
  thermal_c: 45,
  clock_mhz: 1700,
};

function resetStores() {
  useNavigationStore.setState({
    activePage: "model-manager",
    glossaryHistoryTab: "glossary",
    hasInitializedRoute: false,
  });
  useEngineStore.setState({
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
  });
  useModelsStore.setState({
    catalog: [],
    installed: [],
    downloads: [],
    progressByModel: {},
    consentModelId: null,
    loading: false,
    actionBusy: false,
    actionError: null,
  });
  useSettingsStore.setState({
    settings: null,
    saving: false,
    error: null,
  });
}

describe("ModelManagerPage & Sidebar Flows", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resetStores();
    listenMock.mockResolvedValue(() => {});
    openDialogMock.mockResolvedValue(null);
  });

  afterEach(() => {
    cleanup();
  });

  it("enforces Consent-First download on first-run with 0 installed models", async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      switch (cmd) {
        case "get_settings":
          return {
            display_mode: "standard",
            default_model_dir: "C:\\Users\\Test\\Osmium\\models",
            default_export_dir: "C:\\Users\\Test\\Downloads",
            backend_override: "auto",
            ctx_size_override: "auto",
            gpu_layers_override: "auto",
            tts_provider: "native",
            tts_rate: 1.0,
            tts_voice_prefs_json: "{}",
            auto_idle_unload_minutes: 10,
          };
        case "list_models":
          return { catalog: BASE_CATALOG, installed: [], downloads: [] };
        case "scan_hardware":
          return BASE_HARDWARE;
        case "get_engine_telemetry":
          return {
            status: "unconfigured",
            pid: null,
            port: null,
            active_model_id: null,
            active_model_name: null,
            backend: "vulkan",
            ctx_size: 8192,
            gpu_layers: 99,
            vram_used_mb: 980,
            vram_total_mb: 12288,
            last_ttft_ms: null,
            last_tok_per_sec: null,
            db_wal_synced: true,
            busy_by: "none",
          };
        case "check_disk_space":
          return { free_bytes: 50 * 1024 * 1024 * 1024, writable: true };
        case "set_diagnostics_tab_open":
          return undefined;
        case "confirm_and_start_model_download":
          return {
            model_id: "qwen3-8b-q4km",
            download_url: BASE_CATALOG[0].download_url,
            target_path: "C:\\Users\\Test\\Osmium\\models\\Qwen3-8B-Q4_K_M.gguf",
            part_path: "C:\\Users\\Test\\Osmium\\models\\Qwen3-8B-Q4_K_M.gguf.part",
            expected_sha256: BASE_CATALOG[0].sha256,
            etag: null,
            total_bytes: BASE_CATALOG[0].file_size_bytes,
            bytes_downloaded: BASE_CATALOG[0].file_size_bytes,
            status: "completed",
            error_msg: null,
            updated_at: 1,
          };
        default:
          return undefined;
      }
    });

    render(<App />);

    await waitFor(() => {
      expect(screen.getByText("Model Manager & Hardware Diagnostics")).toBeTruthy();
    });

    expect(screen.getByText("SETUP")).toBeTruthy();
    expect(invokeMock).not.toHaveBeenCalledWith(
      "confirm_and_start_model_download",
      expect.anything()
    );

    const downloadBtns = screen.getAllByRole("button", { name: /Download Model/i });
    fireEvent.click(downloadBtns[0]);

    await waitFor(() => {
      expect(
        screen.getByText(/Confirm AI Model Download — Qwen 3 8B/i)
      ).toBeTruthy();
    });
    expect(invokeMock).not.toHaveBeenCalledWith(
      "confirm_and_start_model_download",
      expect.anything()
    );

    const consentBtn = await screen.findByRole("button", {
      name: /Agree & Start Download/i,
    });
    await waitFor(() => {
      expect((consentBtn as HTMLButtonElement).disabled).toBe(false);
    });
    fireEvent.click(consentBtn);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(
        "confirm_and_start_model_download",
        expect.objectContaining({
          modelId: "qwen3-8b-q4km",
          targetDir: "C:\\Users\\Test\\Osmium\\models",
        })
      );
    });
  });

  it("shows missing-file warning + relink button for broken BYOM reference and READY pill when active", async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      switch (cmd) {
        case "get_settings":
          return {
            display_mode: "standard",
            default_model_dir: "C:\\Users\\Test\\Osmium\\models",
            default_export_dir: "C:\\Users\\Test\\Downloads",
            backend_override: "auto",
            ctx_size_override: "auto",
            gpu_layers_override: "auto",
            tts_provider: "native",
            tts_rate: 1.0,
            tts_voice_prefs_json: "{}",
            auto_idle_unload_minutes: 10,
          };
        case "list_models":
          return {
            catalog: BASE_CATALOG,
            installed: [
              {
                id: "qwen3-4b-q4km",
                display_name: "Qwen 3 4B",
                tier: "balanced_4b",
                file_path: "C:\\Users\\Test\\Osmium\\models\\Qwen3-4B-Q4_K_M.gguf",
                file_size_bytes: 2497280256,
                sha256: BASE_CATALOG[1].sha256,
                quantization: "Q4_K_M",
                is_byom: false,
                block_count: 36,
                is_active: true,
                created_at: 1,
                file_exists: true,
              },
              {
                id: "byom-custom-1",
                display_name: "Custom Gemma 9B",
                tier: "custom_byom",
                file_path: "D:\\Moved\\custom-gemma.gguf",
                file_size_bytes: 4000000000,
                sha256: null,
                quantization: "Q4_K_M",
                is_byom: true,
                block_count: 42,
                is_active: false,
                created_at: 2,
                file_exists: false,
              },
            ],
            downloads: [],
          };
        case "scan_hardware":
          return BASE_HARDWARE;
        case "get_engine_telemetry":
          return {
            status: "standby",
            pid: null,
            port: null,
            active_model_id: "qwen3-4b-q4km",
            active_model_name: "Qwen 3 4B",
            backend: "vulkan",
            ctx_size: 4096,
            gpu_layers: 99,
            vram_used_mb: 980,
            vram_total_mb: 12288,
            last_ttft_ms: null,
            last_tok_per_sec: null,
            db_wal_synced: true,
            busy_by: "none",
          };
        default:
          return undefined;
      }
    });

    render(<App />);

    await waitFor(() => {
      expect(screen.getByText("READY")).toBeTruthy();
    });

    fireEvent.click(screen.getByRole("button", { name: /Model Manager/i }));

    await waitFor(() => {
      expect(screen.getByText("Custom Gemma 9B")).toBeTruthy();
    });

    expect(
      screen.getByText(/Local model file was not found\. It may have been moved or deleted\./i)
    ).toBeTruthy();
    expect(screen.getByRole("button", { name: /Locate File/i })).toBeTruthy();
  });

  it("invokes unload_sidecar_model when Unload Model button is clicked while engine is online", async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      switch (cmd) {
        case "get_settings":
          return {
            display_mode: "standard",
            default_model_dir: "C:\\Users\\Test\\Osmium\\models",
            default_export_dir: "C:\\Users\\Test\\Downloads",
            backend_override: "auto",
            ctx_size_override: "auto",
            gpu_layers_override: "auto",
            tts_provider: "native",
            tts_rate: 1.0,
            tts_voice_prefs_json: "{}",
            auto_idle_unload_minutes: 10,
          };
        case "list_models":
          return {
            catalog: BASE_CATALOG,
            installed: [
              {
                id: "qwen3-8b-q4km",
                display_name: "Qwen 3 8B",
                tier: "optimal_8b",
                file_path: "C:\\Models\\Qwen3-8B-Q4_K_M.gguf",
                file_size_bytes: 5027783488,
                sha256: BASE_CATALOG[0].sha256,
                quantization: "Q4_K_M",
                is_byom: false,
                block_count: 36,
                is_active: true,
                created_at: 1,
                file_exists: true,
              },
            ],
            downloads: [],
          };
        case "scan_hardware":
          return BASE_HARDWARE;
        case "get_engine_telemetry":
          return {
            status: "online",
            pid: 4321,
            port: 55100,
            active_model_id: "qwen3-8b-q4km",
            active_model_name: "Qwen 3 8B",
            backend: "vulkan",
            ctx_size: 8192,
            gpu_layers: 99,
            vram_used_mb: 6800,
            vram_total_mb: 12288,
            last_ttft_ms: 260,
            last_tok_per_sec: 42.5,
            db_wal_synced: true,
            busy_by: "none",
          };
        case "unload_sidecar_model":
          return {
            status: "standby",
            pid: null,
            port: null,
            active_model_id: "qwen3-8b-q4km",
            active_model_name: "Qwen 3 8B",
            backend: "vulkan",
            ctx_size: 8192,
            gpu_layers: 99,
            vram_used_mb: 980,
            vram_total_mb: 12288,
            last_ttft_ms: 260,
            last_tok_per_sec: 42.5,
            db_wal_synced: true,
            busy_by: "none",
          };
        default:
          return undefined;
      }
    });

    render(<App />);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Unload active model from VRAM/i })).toBeTruthy();
    });

    fireEvent.click(screen.getByRole("button", { name: /Unload active model from VRAM/i }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("unload_sidecar_model");
    });
  });
});

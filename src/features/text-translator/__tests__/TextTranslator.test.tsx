import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const invokeMock = vi.fn();
const listenMock = vi.fn();

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

vi.mock("@tauri-apps/plugin-clipboard-manager", () => ({
  readText: vi.fn().mockResolvedValue(""),
  writeText: vi.fn().mockResolvedValue(undefined),
}));

import { TextTranslatorPage } from "../TextTranslatorPage";
import { useEngineStore } from "../../../stores/useEngineStore";
import { useModelsStore } from "../../../stores/useModelsStore";
import { useSettingsStore } from "../../../stores/useSettingsStore";
import { useTextTranslatorStore } from "../../../stores/useTextTranslatorStore";

function seedActiveModelAndEngine(overrides?: {
  busyBy?: "none" | "subtitle";
  displayMode?: "standard" | "compact";
}) {
  useModelsStore.setState({
    installed: [
      {
        id: "qwen3-8b-q4km",
        display_name: "Qwen 3 8B",
        tier: "optimal_8b",
        file_path: "C:\\Models\\Qwen3-8B-Q4_K_M.gguf",
        file_size_bytes: 5027783488,
        sha256: "abc",
        quantization: "Q4_K_M",
        is_byom: false,
        block_count: 36,
        is_active: true,
        created_at: 1,
        file_exists: true,
      },
    ],
  });
  useEngineStore.setState({
    hardware: {
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
    },
    telemetry: {
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
      last_ttft_ms: 380,
      last_tok_per_sec: 47.9,
      db_wal_synced: true,
      busy_by: overrides?.busyBy ?? "none",
    },
    vramUsedMb: 980,
    vramTotalMb: 12288,
  });
  useSettingsStore.setState({
    settings: {
      display_mode: overrides?.displayMode ?? "standard",
      default_model_dir: "C:\\Models",
      default_export_dir: "C:\\Exports",
      backend_override: "auto",
      ctx_size_override: "auto",
      gpu_layers_override: "auto",
    },
  });
}

describe("TextTranslatorPage Flows", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    listenMock.mockResolvedValue(() => {});
    invokeMock.mockResolvedValue(undefined);
    useTextTranslatorStore.setState({
      sourceText: "",
      translatedText: "",
      tone: "natural",
      autoTranslate: false,
      isStreaming: false,
      statusText: "Ready",
      chunkInfo: null,
      lastTtftMs: null,
      lastTokPerSec: null,
      lastTotalTokens: null,
      lastModelId: null,
      lastModelName: null,
      undoStack: [],
      error: null,
      selectedWord: null,
      contextSummary: "",
      alternatives: [],
      loadingAlternatives: false,
      alternativesError: null,
    });
  });

  afterEach(() => {
    cleanup();
  });

  it("shows subtitle concurrency lock banner and disables input when busy_by is subtitle", () => {
    seedActiveModelAndEngine({ busyBy: "subtitle" });
    render(<TextTranslatorPage />);

    expect(
      screen.getByText(/Subtitle Translator is currently processing an active batch/i)
    ).toBeTruthy();
    const textarea = screen.getByLabelText(
      "English source text"
    ) as HTMLTextAreaElement;
    expect(textarea.disabled).toBe(true);
  });

  it("renders manual Translate button inside SourcePane footer alongside char counter when auto-translate is off", () => {
    seedActiveModelAndEngine();
    useTextTranslatorStore.setState({
      sourceText: "Hello world",
      autoTranslate: false,
    });

    render(<TextTranslatorPage />);

    const translateBtn = screen.getByRole("button", { name: /^Translate$/i });
    const statsSpan = screen.getByText(/25,000 chars/i);
    expect(translateBtn.closest("section")).toBe(statsSpan.closest("section"));
    expect((translateBtn as HTMLButtonElement).disabled).toBe(false);
  });

  it("warns above 25,000 characters and truncates via Trim to 25,000 button", () => {
    seedActiveModelAndEngine();
    useTextTranslatorStore.setState({
      sourceText: "a".repeat(25_120),
    });

    render(<TextTranslatorPage />);
    expect(screen.getByText(/25,000-character limit exceeded/i)).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: /Trim to 25,000/i }));
    expect(
      Array.from(useTextTranslatorStore.getState().sourceText).length
    ).toBe(25_000);
  });

  it("hides TelemetryStrip in compact display mode and shows it in standard mode", () => {
    seedActiveModelAndEngine({ displayMode: "standard" });
    const { rerender } = render(<TextTranslatorPage />);
    expect(screen.getByLabelText("Local performance telemetry strip")).toBeTruthy();

    act(() => {
      useSettingsStore.setState({
        settings: {
          display_mode: "compact",
          default_model_dir: "C:\\Models",
          default_export_dir: "C:\\Exports",
          backend_override: "auto",
          ctx_size_override: "auto",
          gpu_layers_override: "auto",
        },
      });
    });
    rerender(<TextTranslatorPage />);
    expect(screen.queryByLabelText("Local performance telemetry strip")).toBeNull();
  });

  it("applies scoped Ctrl+Z only when focus is outside textarea/input", async () => {
    seedActiveModelAndEngine();
    useTextTranslatorStore.setState({
      sourceText: "A bold move.",
      translatedText: "Langkah tegas.",
      undoStack: ["Langkah berani."],
    });

    render(<TextTranslatorPage />);

    const textarea = screen.getByLabelText(
      "English source text"
    ) as HTMLTextAreaElement;
    textarea.focus();

    fireEvent.keyDown(window, { key: "z", ctrlKey: true });
    expect(useTextTranslatorStore.getState().translatedText).toBe("Langkah tegas.");
    expect(useTextTranslatorStore.getState().undoStack).toHaveLength(1);

    textarea.blur();
    fireEvent.keyDown(window, { key: "z", ctrlKey: true });

    await waitFor(() => {
      expect(useTextTranslatorStore.getState().translatedText).toBe(
        "Langkah berani."
      );
    });
    expect(useTextTranslatorStore.getState().undoStack).toHaveLength(0);
  });
});

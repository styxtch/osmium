import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const invokeMock = vi.fn();
const listenMock = vi.fn();
const openDialogMock = vi.fn();
const saveDialogMock = vi.fn();

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

vi.mock("@tauri-apps/api/webviewWindow", () => ({
  getCurrentWebviewWindow: () => ({
    onDragDropEvent: vi.fn().mockResolvedValue(() => {}),
  }),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: (...args: unknown[]) => openDialogMock(...args),
  save: (...args: unknown[]) => saveDialogMock(...args),
}));

import { SubtitleTranslatorPage } from "../SubtitleTranslatorPage";
import { useGlossaryHistoryStore } from "../../../stores/useGlossaryHistoryStore";
import { useModelsStore } from "../../../stores/useModelsStore";
import { useSettingsStore } from "../../../stores/useSettingsStore";
import { useSubtitleStore } from "../../../stores/useSubtitleStore";
import type { SubtitleCueDto, SubtitleJobSummary } from "../../../lib/ipc";

function makeJob(
  status: SubtitleJobSummary["status"],
  overrides?: Partial<SubtitleJobSummary>
): SubtitleJobSummary {
  return {
    id: "job-1",
    file_name: "episode01.ass",
    file_path: "D:\\Videos\\episode01.ass",
    file_size_bytes: 20480,
    format: "ass",
    source_lang: "en",
    target_lang: "id",
    genre: "anime",
    glossary_id: null,
    tag_shield_enabled: true,
    model_id: "qwen3-8b-q4km",
    model_display_name: "Qwen 3 8B",
    status,
    total_cues: 2,
    completed_cues: status === "completed" ? 2 : 1,
    self_healing_errors: 0,
    created_at: 1000,
    updated_at: 2000,
    ...overrides,
  };
}

function makeCues(status: SubtitleJobSummary["status"]): SubtitleCueDto[] {
  return [
    {
      job_id: "job-1",
      cue_index: 1,
      cue_type: "dialogue",
      start_time: "0:00:01.00",
      end_time: "0:00:03.00",
      meta_json: JSON.stringify({ leading_tags: "{\\an8}", display_tags: ["{\\an8}"] }),
      glossary_matches_json: null,
      original_text: "{\\an8}Launch the EVA unit!",
      translated_text: "{\\an8}Luncurkan unit EVA!",
      batch_number: 1,
      status: "done",
      warning_msg: null,
    },
    {
      job_id: "job-1",
      cue_index: 2,
      cue_type: "dialogue",
      start_time: "0:00:04.00",
      end_time: "0:00:06.00",
      meta_json: null,
      glossary_matches_json: null,
      original_text: "Hold position!",
      translated_text: status === "completed" ? "Tahan posisi!" : null,
      batch_number: status === "completed" ? 1 : null,
      status: status === "completed" ? "done" : "processing",
      warning_msg: null,
    },
  ];
}

function seedStores(
  status: SubtitleJobSummary["status"],
  jobOverrides?: Partial<SubtitleJobSummary>
) {
  const job = makeJob(status, jobOverrides);
  const cues = makeCues(status);

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

  useSettingsStore.setState({
    settings: {
      default_model_dir: "C:\\Models",
      default_export_dir: "D:\\Exports",
      backend_override: "auto",
      ctx_size_override: "auto",
      gpu_layers_override: "auto",
      display_mode: "standard",
      tts_provider: "native",
      tts_rate: 1.0,
      tts_voice_prefs_json: "{}",
      auto_idle_unload_minutes: 10,
    },
  });

  useGlossaryHistoryStore.setState({ glossaries: [] });

  useSubtitleStore.setState({
    jobs: [job],
    activeJobId: job.id,
    activeJob: job,
    cues,
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
  });
}

describe("SubtitleTranslatorPage", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    listenMock.mockReset();
    openDialogMock.mockReset();
    saveDialogMock.mockReset();
    listenMock.mockResolvedValue(() => {});

    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(() => ({
      width: 1200,
      height: 640,
      top: 0,
      left: 0,
      bottom: 640,
      right: 1200,
      x: 0,
      y: 0,
      toJSON: () => ({}),
    }));
    Object.defineProperty(HTMLElement.prototype, "offsetHeight", {
      configurable: true,
      get: () => 640,
    });
    Object.defineProperty(HTMLElement.prototype, "offsetWidth", {
      configurable: true,
      get: () => 1200,
    });
    Object.defineProperty(HTMLElement.prototype, "clientHeight", {
      configurable: true,
      get: () => 640,
    });

    invokeMock.mockImplementation((cmd: string, args?: Record<string, unknown>) => {
      if (cmd === "list_subtitle_jobs") {
        return Promise.resolve(useSubtitleStore.getState().jobs);
      }
      if (cmd === "list_glossaries") {
        return Promise.resolve([]);
      }
      if (cmd === "get_subtitle_cues") {
        return Promise.resolve(makeCues(useSubtitleStore.getState().activeJob?.status ?? "paused"));
      }
      if (cmd === "get_subtitle_job") {
        return Promise.resolve(useSubtitleStore.getState().activeJob);
      }
      if (cmd === "update_cue_translation") {
        return Promise.resolve({
          ...makeCues("paused")[0],
          translated_text: `{\\an8}${String(args?.translatedText ?? "")}`,
          status: "done",
        });
      }
      if (cmd === "retranslate_single_cue") {
        return Promise.resolve({
          ...makeCues("paused")[0],
          translated_text: "{\\an8}Terjemahan ulang!",
          status: "done",
        });
      }
      if (cmd === "export_subtitle_file") {
        return Promise.resolve(String(args?.destPath ?? "D:\\Exports\\episode01.id.ass"));
      }
      return Promise.resolve(null);
    });
  });

  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  it("locks Export until job reaches 100% completion and opens Windows Save As dialog when completed", async () => {
    seedStores("running");
    let view!: ReturnType<typeof render>;
    await act(async () => {
      view = render(<SubtitleTranslatorPage />);
    });

    const exportRunning = screen.getByRole("button", { name: /export/i }) as HTMLButtonElement;
    expect(exportRunning.disabled).toBe(true);

    act(() => {
      seedStores("paused");
    });
    view.rerender(<SubtitleTranslatorPage />);
    const exportPaused = screen.getByRole("button", { name: /export/i }) as HTMLButtonElement;
    expect(exportPaused.disabled).toBe(true);

    act(() => {
      seedStores("completed");
    });
    view.rerender(<SubtitleTranslatorPage />);

    const exportCompleted = screen.getByRole("button", { name: /export/i }) as HTMLButtonElement;
    expect(exportCompleted.disabled).toBe(false);

    saveDialogMock.mockResolvedValueOnce("D:\\Exports\\episode01.id.ass");
    await act(async () => {
      fireEvent.click(exportCompleted);
    });

    expect(saveDialogMock).toHaveBeenCalledWith(
      expect.objectContaining({
        defaultPath: "D:\\Exports\\episode01.id.ass",
      })
    );
    expect(invokeMock).toHaveBeenCalledWith("export_subtitle_file", {
      jobId: "job-1",
      destPath: "D:\\Exports\\episode01.id.ass",
    });
  });

  it("disables row Edit and Re-translate while running and enables them on paused and completed", async () => {
    seedStores("running");
    let view!: ReturnType<typeof render>;
    await act(async () => {
      view = render(<SubtitleTranslatorPage />);
    });

    expect((screen.getByLabelText("Edit cue 1") as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByLabelText("Re-translate cue 1") as HTMLButtonElement).disabled).toBe(true);

    act(() => {
      seedStores("paused");
    });
    view.rerender(<SubtitleTranslatorPage />);

    const editBtn = screen.getByLabelText("Edit cue 1") as HTMLButtonElement;
    const retranslateBtn = screen.getByLabelText("Re-translate cue 1") as HTMLButtonElement;
    expect(editBtn.disabled).toBe(false);
    expect(retranslateBtn.disabled).toBe(false);

    fireEvent.click(editBtn);
    const editor = screen.getByLabelText("Edit translated dialogue") as HTMLTextAreaElement;
    // Leading ASS tag {\an8} is shown outside the textarea as protected pill
    expect(editor.value).toBe("Luncurkan unit EVA!");

    fireEvent.change(editor, { target: { value: "Kerahkan unit EVA sekarang!" } });
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /save/i }));
    });

    expect(invokeMock).toHaveBeenCalledWith("update_cue_translation", {
      jobId: "job-1",
      cueIndex: 1,
      translatedText: "Kerahkan unit EVA sekarang!",
    });

    await act(async () => {
      fireEvent.click(screen.getByLabelText("Re-translate cue 1"));
    });

    expect(invokeMock).toHaveBeenCalledWith("retranslate_single_cue", {
      jobId: "job-1",
      cueIndex: 1,
    });

    act(() => {
      seedStores("completed");
    });
    view.rerender(<SubtitleTranslatorPage />);
    expect((screen.getByLabelText("Edit cue 1") as HTMLButtonElement).disabled).toBe(false);
    expect((screen.getByLabelText("Re-translate cue 1") as HTMLButtonElement).disabled).toBe(false);
  });

  it("focuses search input on Ctrl+F and filters cues by query and warning status", async () => {
    seedStores("paused");
    await act(async () => {
      render(<SubtitleTranslatorPage />);
    });

    const searchInput = screen.getByLabelText("Search subtitle cues") as HTMLInputElement;
    expect(document.activeElement).not.toBe(searchInput);

    fireEvent.keyDown(window, { key: "f", ctrlKey: true });
    expect(document.activeElement).toBe(searchInput);

    fireEvent.change(searchInput, { target: { value: "EVA" } });
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("get_subtitle_cues", {
        jobId: "job-1",
        offset: 0,
        limit: 5000,
        searchQuery: "EVA",
        statusFilter: "all",
      });
    });

    fireEvent.click(screen.getByRole("button", { name: "Warnings" }));
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("get_subtitle_cues", {
        jobId: "job-1",
        offset: 0,
        limit: 5000,
        searchQuery: "EVA",
        statusFilter: "warning",
      });
    });
  });

  it("renders dynamic target language column header and export filename suffix", async () => {
    seedStores("completed", { source_lang: "ja", target_lang: "en" });
    await act(async () => {
      render(<SubtitleTranslatorPage />);
    });

    expect(screen.getByText("English Translation")).toBeTruthy();

    saveDialogMock.mockResolvedValueOnce("D:\\Exports\\episode01.en.ass");
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /export/i }));
    });

    expect(saveDialogMock).toHaveBeenCalledWith(
      expect.objectContaining({
        defaultPath: "D:\\Exports\\episode01.en.ass",
      })
    );
  });
});

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
import { useTtsStore } from "../../../stores/useTtsStore";

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
      tts_provider: "native",
      tts_rate: 1.0,
      tts_voice_prefs_json: "{}",
      auto_idle_unload_minutes: 10,
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
      sourceLang: "en",
      detectedSourceLang: null,
      targetLang: "id",
      tone: "natural",
      autoTranslate: false,
      lastTranslatedKey: null,
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
    useTtsStore.setState({
      speakingKey: null,
      voices: [],
      notice: null,
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

  it("renders manual Translate button and Auto-translate toggle inside SourcePane footer alongside char counter", () => {
    seedActiveModelAndEngine();
    useTextTranslatorStore.setState({
      sourceText: "Hello world",
      autoTranslate: false,
    });

    render(<TextTranslatorPage />);

    const translateBtn = screen.getByRole("button", { name: /^Translate$/i });
    const autoCheckbox = screen.getByRole("checkbox", {
      name: /Auto-translate \(600ms\)/i,
    });
    const statsSpan = screen.getByText(/25,000 chars/i);
    expect(translateBtn.closest("section")).toBe(statsSpan.closest("section"));
    expect(autoCheckbox.closest("section")).toBe(statsSpan.closest("section"));
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
          tts_provider: "native",
          tts_rate: 1.0,
          tts_voice_prefs_json: "{}",
          auto_idle_unload_minutes: 10,
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

  it("sends selected source and target languages to translate_text IPC", async () => {
    seedActiveModelAndEngine();
    invokeMock.mockResolvedValueOnce({
      translated_text: "Good morning",
      source_lang: "ja",
      target_lang: "en",
      tone: "natural",
      model_id: "qwen3-8b-q4km",
      model_display_name: "Qwen 3 8B",
      ttft_ms: 120,
      tok_per_sec: 45,
      total_tokens: 12,
      chunk_count: 1,
      cancelled: false,
    });

    render(<TextTranslatorPage />);

    fireEvent.change(screen.getByLabelText("Source language"), {
      target: { value: "ja" },
    });
    fireEvent.change(screen.getByLabelText("Target language"), {
      target: { value: "en" },
    });
    fireEvent.change(screen.getByLabelText("Japanese source text"), {
      target: { value: "おはようございます" },
    });

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /^Translate$/i }));
    });

    expect(invokeMock).toHaveBeenCalledWith(
      "translate_text",
      expect.objectContaining({
        sourceText: "おはようございます",
        sourceLang: "ja",
        targetLang: "en",
        tone: "natural",
      })
    );
  });

  it("swaps source/target languages and texts when clicking Swap languages", () => {
    seedActiveModelAndEngine();
    useTextTranslatorStore.setState({
      sourceLang: "en",
      targetLang: "id",
      sourceText: "Good evening",
      translatedText: "Selamat malam",
    });

    render(<TextTranslatorPage />);

    fireEvent.click(screen.getByRole("button", { name: "Swap languages" }));

    const state = useTextTranslatorStore.getState();
    expect(state.sourceLang).toBe("id");
    expect(state.targetLang).toBe("en");
    expect(state.sourceText).toBe("Selamat malam");
    expect(state.translatedText).toBe("Good evening");
    expect(screen.getByLabelText("Indonesian source text")).toBeTruthy();
    expect(screen.getByLabelText("English translation output")).toBeTruthy();
  });

  it("auto-detects Japanese source text, updates dropdown & header, and sends detected sourceLang", async () => {
    seedActiveModelAndEngine();
    useTextTranslatorStore.setState({
      sourceLang: "auto",
      detectedSourceLang: null,
      targetLang: "id",
    });
    invokeMock.mockResolvedValueOnce({
      translated_text: "Hei, mengapa kamu terus memaksakan diri?",
      source_lang: "ja",
      target_lang: "id",
      tone: "natural",
      model_id: "qwen3-8b-q4km",
      model_display_name: "Qwen 3 8B",
      ttft_ms: 150,
      tok_per_sec: 42,
      total_tokens: 18,
      chunk_count: 1,
      cancelled: false,
    });

    render(<TextTranslatorPage />);

    const sourceInput = screen.getByLabelText("Auto-detect source text");
    fireEvent.change(sourceInput, {
      target: { value: "「ねぇ、どうしてそんなに無茶ばかりするの？」" },
    });

    expect(screen.getByText("JAPANESE (DETECTED)")).toBeTruthy();
    expect(screen.getByText("Auto-detect (Japanese)")).toBeTruthy();
    expect(screen.getByLabelText("Japanese source text")).toBeTruthy();

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /^Translate$/i }));
    });

    expect(invokeMock).toHaveBeenCalledWith(
      "translate_text",
      expect.objectContaining({
        sourceText: "「ねぇ、どうしてそんなに無茶ばかりするの？」",
        sourceLang: "ja",
        targetLang: "id",
        tone: "natural",
      })
    );
  });

  it("auto-switches target language to English when Indonesian is detected while target is Indonesian", () => {
    seedActiveModelAndEngine();
    useTextTranslatorStore.setState({
      sourceLang: "auto",
      detectedSourceLang: null,
      targetLang: "id",
    });

    render(<TextTranslatorPage />);

    fireEvent.change(screen.getByLabelText("Auto-detect source text"), {
      target: { value: "Kamu tidak perlu menanggung semuanya sendirian." },
    });

    const state = useTextTranslatorStore.getState();
    expect(state.detectedSourceLang).toBe("id");
    expect(state.targetLang).toBe("en");
    expect(screen.getByText("INDONESIAN (DETECTED)")).toBeTruthy();
    expect(screen.getByLabelText("English translation output")).toBeTruthy();
  });

  it("swaps detected Japanese source and Indonesian target cleanly", () => {
    seedActiveModelAndEngine();
    useTextTranslatorStore.setState({
      sourceLang: "auto",
      detectedSourceLang: "ja",
      targetLang: "id",
      sourceText: "おはよう",
      translatedText: "Selamat pagi",
    });

    render(<TextTranslatorPage />);

    fireEvent.click(screen.getByRole("button", { name: "Swap languages" }));

    const state = useTextTranslatorStore.getState();
    expect(state.sourceLang).toBe("id");
    expect(state.detectedSourceLang).toBeNull();
    expect(state.targetLang).toBe("ja");
    expect(state.sourceText).toBe("Selamat pagi");
    expect(state.translatedText).toBe("おはよう");
  });

  it("does not re-trigger auto-translation when remounting after switching tabs", async () => {
    vi.useFakeTimers();
    try {
      seedActiveModelAndEngine();
      useTextTranslatorStore.setState({
        sourceLang: "auto",
        detectedSourceLang: "ja",
        targetLang: "id",
        tone: "natural",
        autoTranslate: true,
        sourceText: "おはようございます",
        translatedText: "",
        lastTranslatedKey: null,
      });

      invokeMock.mockResolvedValue({
        translated_text: "Selamat pagi",
        source_lang: "ja",
        target_lang: "id",
        tone: "natural",
        model_id: "qwen3-8b-q4km",
        model_display_name: "Qwen 3 8B",
        ttft_ms: 120,
        tok_per_sec: 45,
        total_tokens: 10,
        chunk_count: 1,
        cancelled: false,
      });

      const first = render(<TextTranslatorPage />);

      await act(async () => {
        await vi.advanceTimersByTimeAsync(700);
      });

      expect(
        invokeMock.mock.calls.filter(([cmd]) => cmd === "translate_text")
      ).toHaveLength(1);

      first.unmount();
      render(<TextTranslatorPage />);

      await act(async () => {
        await vi.advanceTimersByTimeAsync(1000);
      });

      expect(
        invokeMock.mock.calls.filter(([cmd]) => cmd === "translate_text")
      ).toHaveLength(1);
    } finally {
      vi.useRealTimers();
    }
  });

  it("merges tok/s and TTFT into a single status pill inside TargetPane header", () => {
    seedActiveModelAndEngine();
    useTextTranslatorStore.setState({
      sourceText: "Hello",
      translatedText: "Halo",
      statusText: "Done",
      lastTokPerSec: 40.7,
      lastTtftMs: 260,
    });

    render(<TextTranslatorPage />);

    expect(screen.getByText("Done • 40.7 tok/s • TTFT 0.26s")).toBeTruthy();
  });

  it("plays translation audio via window.speechSynthesis with target language and toggles to Stop", () => {
    const originalSynth = window.speechSynthesis;
    const originalUtterance = globalThis.SpeechSynthesisUtterance;
    const spoken: Array<{ text: string; lang: string }> = [];
    const speakMock = vi.fn((utt: { text: string; lang: string }) => {
      spoken.push({ text: utt.text, lang: utt.lang });
    });
    const cancelMock = vi.fn();

    class MockUtt {
      text: string;
      lang = "";
      rate = 1;
      voice: SpeechSynthesisVoice | null = null;
      onend: (() => void) | null = null;
      onerror: ((evt: { error?: string }) => void) | null = null;
      constructor(text: string) {
        this.text = text;
      }
    }

    Object.defineProperty(window, "speechSynthesis", {
      configurable: true,
      value: {
        getVoices: () => [
          {
            voiceURI: "ms-id",
            name: "Microsoft Gadis",
            lang: "id-ID",
            localService: true,
          },
        ],
        speak: speakMock,
        cancel: cancelMock,
      },
    });
    (globalThis as unknown as Record<string, unknown>).SpeechSynthesisUtterance =
      MockUtt;

    try {
      seedActiveModelAndEngine();
      useTextTranslatorStore.setState({
        sourceLang: "en",
        targetLang: "id",
        sourceText: "Good night",
        translatedText: "Selamat malam",
      });

      render(<TextTranslatorPage />);

      fireEvent.click(
        screen.getByRole("button", { name: "Listen to translation" })
      );

      expect(speakMock).toHaveBeenCalledTimes(1);
      expect(spoken[0]).toEqual({ text: "Selamat malam", lang: "id-ID" });
      expect(
        screen.getByRole("button", { name: "Stop translation audio" })
      ).toBeTruthy();

      fireEvent.click(
        screen.getByRole("button", { name: "Stop translation audio" })
      );
      expect(cancelMock).toHaveBeenCalled();
      expect(
        screen.getByRole("button", { name: "Listen to translation" })
      ).toBeTruthy();
    } finally {
      Object.defineProperty(window, "speechSynthesis", {
        configurable: true,
        value: originalSynth,
      });
      (
        globalThis as unknown as Record<string, unknown>
      ).SpeechSynthesisUtterance = originalUtterance;
    }
  });
});

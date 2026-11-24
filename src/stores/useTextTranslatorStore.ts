import { create } from "zustand";
import {
  ipc,
  parseAppError,
  type AppErrorPayload,
  type TextHistoryItem,
  type Tone,
  type WordAlternativeItem,
} from "../lib/ipc";
import { detectLanguageDetailed } from "../lib/languages";
import {
  extractSentenceContext,
  replaceTokenAtOffset,
  type TextToken,
} from "../features/text-translator/tokenizer";
import { useEngineStore } from "./useEngineStore";
import { useGlossaryHistoryStore } from "./useGlossaryHistoryStore";

export const MAX_TEXT_CHARS = 25_000;

export function buildTranslationKey(
  sourceText: string,
  sourceLang: string,
  detectedSourceLang: string | null,
  targetLang: string,
  tone: Tone
): string {
  const effectiveSource =
    sourceLang === "auto" ? (detectedSourceLang ?? "auto") : sourceLang;
  return `${effectiveSource}|${targetLang}|${tone}|${sourceText.trim()}`;
}

export interface SelectedWordState {
  token: TextToken;
  sentenceContext: string;
  anchorRect: { top: number; left: number; bottom: number };
}

interface TextTranslatorState {
  sourceText: string;
  translatedText: string;
  sourceLang: string;
  detectedSourceLang: string | null;
  targetLang: string;
  tone: Tone;
  autoTranslate: boolean;
  lastTranslatedKey: string | null;
  isStreaming: boolean;
  statusText: string;
  chunkInfo: { index: number; total: number } | null;
  lastTtftMs: number | null;
  lastTokPerSec: number | null;
  lastTotalTokens: number | null;
  lastModelId: string | null;
  lastModelName: string | null;
  undoStack: string[];
  error: AppErrorPayload | null;
  selectedWord: SelectedWordState | null;
  contextSummary: string;
  alternatives: WordAlternativeItem[];
  loadingAlternatives: boolean;
  alternativesError: string | null;

  setSourceText: (text: string) => void;
  setSourceLang: (lang: string) => void;
  setTargetLang: (lang: string) => void;
  swapLanguages: () => void;
  setTone: (tone: Tone) => void;
  setAutoTranslate: (enabled: boolean) => void;
  truncateToSafetyLimit: () => void;
  translateNow: () => Promise<void>;
  cancelTranslation: () => Promise<void>;
  clearSourceOnly: () => void;
  clearAll: () => void;
  selectWordForAlternatives: (
    token: TextToken,
    anchorRect: { top: number; left: number; bottom: number }
  ) => Promise<void>;
  closeAlternatives: () => void;
  applyAlternative: (replacementWord: string) => void;
  undoWordSwap: () => void;
  persistCurrentToHistory: () => Promise<void>;
  loadFromHistory: (item: TextHistoryItem) => void;
  clearError: () => void;
}

export const useTextTranslatorStore = create<TextTranslatorState>((set, get) => ({
  sourceText: "",
  translatedText: "",
  sourceLang: "auto",
  detectedSourceLang: null,
  targetLang: "id",
  tone: "natural",
  autoTranslate: true,
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

  setSourceText: (text) =>
    set((state) => {
      if (state.sourceLang !== "auto") {
        return {
          sourceText: text,
          detectedSourceLang: null,
          error: null,
          selectedWord: null,
        };
      }
      const detected = detectLanguageDetailed(text);
      const nextTarget =
        detected?.strong && detected.code === state.targetLang
          ? detected.code === "en"
            ? "id"
            : "en"
          : state.targetLang;
      return {
        sourceText: text,
        detectedSourceLang: detected?.code ?? null,
        targetLang: nextTarget,
        error: null,
        selectedWord: null,
      };
    }),

  setSourceLang: (lang) =>
    set((state) => {
      if (lang === "auto") {
        const detected = detectLanguageDetailed(state.sourceText);
        const nextTarget =
          detected?.strong && detected.code === state.targetLang
            ? detected.code === "en"
              ? "id"
              : "en"
            : state.targetLang;
        return {
          sourceLang: "auto",
          detectedSourceLang: detected?.code ?? null,
          targetLang: nextTarget,
          selectedWord: null,
          error: null,
        };
      }
      if (lang === state.targetLang) {
        const nextTarget =
          state.sourceLang !== "auto"
            ? state.sourceLang
            : lang === "en"
              ? "id"
              : "en";
        return {
          sourceLang: lang,
          detectedSourceLang: null,
          targetLang: nextTarget,
          selectedWord: null,
          error: null,
        };
      }
      return {
        sourceLang: lang,
        detectedSourceLang: null,
        selectedWord: null,
        error: null,
      };
    }),

  setTargetLang: (lang) =>
    set((state) => {
      if (lang === "auto") return state;
      if (state.sourceLang !== "auto" && lang === state.sourceLang) {
        return {
          sourceLang: state.targetLang,
          targetLang: lang,
          selectedWord: null,
          error: null,
        };
      }
      return {
        targetLang: lang,
        selectedWord: null,
        error: null,
      };
    }),

  swapLanguages: () =>
    set((state) => {
      if (state.isStreaming) return state;
      const effectiveSource =
        state.sourceLang === "auto"
          ? state.detectedSourceLang
          : state.sourceLang;
      const nextSource = state.targetLang;
      const nextTarget =
        effectiveSource && effectiveSource !== state.targetLang
          ? effectiveSource
          : state.targetLang === "en"
            ? "id"
            : "en";

      if (state.translatedText.trim().length > 0) {
        return {
          sourceLang: nextSource,
          detectedSourceLang: null,
          targetLang: nextTarget,
          sourceText: state.translatedText,
          translatedText: state.sourceText,
          lastTranslatedKey: null,
          undoStack: [],
          selectedWord: null,
          error: null,
          statusText: "Ready",
        };
      }

      return {
        sourceLang: nextSource,
        detectedSourceLang: null,
        targetLang: nextTarget,
        lastTranslatedKey: null,
        selectedWord: null,
        error: null,
      };
    }),

  setTone: (tone) =>
    set({
      tone,
      selectedWord: null,
    }),

  setAutoTranslate: (autoTranslate) => set({ autoTranslate }),

  truncateToSafetyLimit: () =>
    set((state) => ({
      sourceText: Array.from(state.sourceText).slice(0, MAX_TEXT_CHARS).join(""),
      error: null,
    })),

  translateNow: async () => {
    const {
      sourceText,
      sourceLang,
      detectedSourceLang,
      targetLang,
      tone,
    } = get();
    const effectiveSourceLang =
      sourceLang === "auto" ? (detectedSourceLang ?? "auto") : sourceLang;
    if (!sourceText.trim()) return;
    if (Array.from(sourceText).length > MAX_TEXT_CHARS) {
      set({
        error: {
          code: "INVALID_INPUT",
          message:
            "Text exceeds the 25,000-character safety limit per session. Split the text into smaller parts.",
        },
      });
      return;
    }

    const requestKey = buildTranslationKey(
      sourceText,
      sourceLang,
      detectedSourceLang,
      targetLang,
      tone
    );

    set({
      isStreaming: true,
      translatedText: "",
      lastTranslatedKey: requestKey,
      statusText: "Preparing model...",
      chunkInfo: null,
      error: null,
      selectedWord: null,
      undoStack: [],
    });

    try {
      const result = await ipc.translateText(
        sourceText,
        effectiveSourceLang,
        targetLang,
        tone,
        (evt) => {
        if (evt.event_type === "status") {
          set({
            statusText: evt.status_message ?? "Translating...",
            chunkInfo:
              evt.total_chunks > 1
                ? { index: evt.chunk_index, total: evt.total_chunks }
                : null,
          });
        } else if (evt.event_type === "chunk" && evt.delta) {
          set((state) => ({
            statusText: "Streaming...",
            translatedText: state.translatedText + evt.delta,
            chunkInfo:
              evt.total_chunks > 1
                ? { index: evt.chunk_index + 1, total: evt.total_chunks }
                : null,
          }));
        } else if (evt.event_type === "done") {
          set({
            isStreaming: false,
            statusText: "Done",
            lastTtftMs: evt.ttft_ms,
            lastTokPerSec: evt.tok_per_sec,
            lastTotalTokens: evt.total_tokens,
          });
        } else if (evt.event_type === "cancelled") {
          set({
            isStreaming: false,
            statusText: "Cancelled",
          });
        }
      });

      if (!result.cancelled) {
        set({
          translatedText: result.translated_text,
          isStreaming: false,
          statusText: "Done",
          lastTtftMs: result.ttft_ms,
          lastTokPerSec: result.tok_per_sec,
          lastTotalTokens: result.total_tokens,
          lastModelId: result.model_id,
          lastModelName: result.model_display_name,
        });
      }
      await useEngineStore.getState().refreshTelemetry();
    } catch (err) {
      const parsed = parseAppError(err);
      set({
        isStreaming: false,
        lastTranslatedKey: null,
        statusText: "Failed",
        error: parsed,
      });
      if (
        parsed.code === "OUT_OF_MEMORY" ||
        parsed.code === "VULKAN_INIT_FAILED" ||
        parsed.code === "BYOM_FILE_MISSING"
      ) {
        useEngineStore.getState().setEngineAlert(parsed);
      }
    }
  },

  cancelTranslation: async () => {
    try {
      await ipc.cancelTextTranslation();
    } catch {
      // Ignore cancel race
    }
  },

  clearSourceOnly: () => {
    if (get().isStreaming) {
      void get().cancelTranslation();
    }
    set({
      sourceText: "",
      detectedSourceLang: null,
      lastTranslatedKey: null,
      error: null,
      selectedWord: null,
    });
  },

  clearAll: () => {
    if (get().isStreaming) {
      void get().cancelTranslation();
    }
    set({
      sourceText: "",
      translatedText: "",
      detectedSourceLang: null,
      lastTranslatedKey: null,
      undoStack: [],
      error: null,
      selectedWord: null,
      statusText: "Ready",
      chunkInfo: null,
      lastTtftMs: null,
      lastTokPerSec: null,
      lastTotalTokens: null,
    });
  },

  selectWordForAlternatives: async (token, anchorRect) => {
    const { isStreaming, translatedText, sourceText, targetLang } = get();
    if (isStreaming || !token.isWord) return;

    const sentenceContext = extractSentenceContext(
      translatedText,
      token.start,
      token.end
    );

    set({
      selectedWord: { token, sentenceContext, anchorRect },
      alternatives: [],
      contextSummary: "",
      loadingAlternatives: true,
      alternativesError: null,
    });

    try {
      const res = await ipc.getWordAlternatives(
        token.text,
        sentenceContext,
        sourceText,
        targetLang
      );
      set({
        contextSummary: res.context_summary,
        alternatives: res.alternatives,
        loadingAlternatives: false,
      });
    } catch (err) {
      const parsed = parseAppError(err);
      set({
        loadingAlternatives: false,
        alternativesError: parsed.message,
      });
    }
  },

  closeAlternatives: () =>
    set({
      selectedWord: null,
      alternatives: [],
      alternativesError: null,
      loadingAlternatives: false,
    }),

  applyAlternative: (replacementWord) => {
    const { selectedWord, translatedText, undoStack } = get();
    if (!selectedWord) return;

    const updated = replaceTokenAtOffset(
      translatedText,
      selectedWord.token.start,
      selectedWord.token.end,
      replacementWord
    );

    set({
      undoStack: [...undoStack, translatedText],
      translatedText: updated,
      selectedWord: null,
      alternatives: [],
      alternativesError: null,
    });
  },

  undoWordSwap: () => {
    const { undoStack } = get();
    if (undoStack.length === 0) return;
    const prev = undoStack[undoStack.length - 1];
    set({
      translatedText: prev,
      undoStack: undoStack.slice(0, -1),
      selectedWord: null,
    });
  },

  persistCurrentToHistory: async () => {
    const {
      sourceText,
      translatedText,
      sourceLang,
      detectedSourceLang,
      targetLang,
      tone,
      isStreaming,
      lastModelId,
      lastModelName,
      lastTtftMs,
      lastTokPerSec,
    } = get();

    const effectiveSourceLang =
      sourceLang === "auto" ? (detectedSourceLang ?? "auto") : sourceLang;

    if (isStreaming || !sourceText.trim() || !translatedText.trim()) return;
    const telemetry = useEngineStore.getState().telemetry;
    const modelId = lastModelId ?? telemetry?.active_model_id ?? "local";
    const modelDisplayName =
      lastModelName ?? telemetry?.active_model_name ?? "Local Model";

    try {
      await ipc.saveTextHistory({
        sourceText,
        translatedText,
        sourceLang: effectiveSourceLang,
        targetLang,
        tone,
        modelId,
        modelDisplayName,
        ttftMs: lastTtftMs,
        tokPerSec: lastTokPerSec,
      });
      void useGlossaryHistoryStore.getState().fetchHistory();
    } catch {
      // Non-fatal history persistence
    }
  },

  loadFromHistory: (item) =>
    set({
      sourceText: item.source_text,
      translatedText: item.translated_text,
      sourceLang: item.source_lang || "auto",
      detectedSourceLang: null,
      targetLang: item.target_lang || "id",
      tone: item.tone,
      lastTranslatedKey: buildTranslationKey(
        item.source_text,
        item.source_lang || "auto",
        null,
        item.target_lang || "id",
        item.tone
      ),
      lastTtftMs: item.ttft_ms,
      lastTokPerSec: item.tok_per_sec,
      lastModelId: item.model_id,
      lastModelName: item.model_display_name,
      undoStack: [],
      selectedWord: null,
      statusText: "Loaded from history",
      error: null,
    }),

  clearError: () => set({ error: null }),
}));

import { useEffect, useRef } from "react";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { AlertCircle, Captions, Cpu, Lock, X } from "lucide-react";
import { useEngineStore } from "../../stores/useEngineStore";
import { useModelsStore } from "../../stores/useModelsStore";
import { useNavigationStore } from "../../stores/useNavigationStore";
import {
  MAX_TEXT_CHARS,
  useTextTranslatorStore,
} from "../../stores/useTextTranslatorStore";
import { SourcePane } from "./SourcePane";
import { SubToolbar } from "./SubToolbar";
import { TargetPane } from "./TargetPane";
import { TelemetryStrip } from "./TelemetryStrip";

export function TextTranslatorPage() {
  const installed = useModelsStore((s) => s.installed);
  const telemetry = useEngineStore((s) => s.telemetry);
  const setActivePage = useNavigationStore((s) => s.setActivePage);

  const sourceText = useTextTranslatorStore((s) => s.sourceText);
  const translatedText = useTextTranslatorStore((s) => s.translatedText);
  const tone = useTextTranslatorStore((s) => s.tone);
  const autoTranslate = useTextTranslatorStore((s) => s.autoTranslate);
  const isStreaming = useTextTranslatorStore((s) => s.isStreaming);
  const error = useTextTranslatorStore((s) => s.error);
  const translateNow = useTextTranslatorStore((s) => s.translateNow);
  const undoWordSwap = useTextTranslatorStore((s) => s.undoWordSwap);
  const persistCurrentToHistory = useTextTranslatorStore(
    (s) => s.persistCurrentToHistory
  );
  const clearError = useTextTranslatorStore((s) => s.clearError);

  const hasReadyModel = installed.some((m) => m.is_active && m.file_exists);
  const subtitleLocked = telemetry?.busy_by === "subtitle";
  const controlsDisabled = !hasReadyModel || subtitleLocked;

  const isFirstAutoRun = useRef(true);

  // 600ms debounce auto-translate when enabled
  useEffect(() => {
    if (isFirstAutoRun.current) {
      isFirstAutoRun.current = false;
      return;
    }
    if (!autoTranslate || controlsDisabled) return;
    if (!sourceText.trim()) return;
    if (Array.from(sourceText).length > MAX_TEXT_CHARS) return;

    const timer = window.setTimeout(() => {
      void translateNow();
    }, 600);

    return () => window.clearTimeout(timer);
  }, [sourceText, tone, autoTranslate, controlsDisabled, translateNow]);

  // 3s idle auto-save to text_history (anti-spam)
  useEffect(() => {
    if (isStreaming || !sourceText.trim() || !translatedText.trim()) return;
    const timer = window.setTimeout(() => {
      void persistCurrentToHistory();
    }, 3000);
    return () => window.clearTimeout(timer);
  }, [isStreaming, sourceText, translatedText, persistCurrentToHistory]);

  // Keyboard shortcuts: Ctrl+Enter, Ctrl+Shift+C, scoped Ctrl+Z
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (!e.ctrlKey && !e.metaKey) return;

      if (e.key === "Enter" && !controlsDisabled && !autoTranslate) {
        e.preventDefault();
        void translateNow();
        return;
      }

      if (e.shiftKey && e.key.toLowerCase() === "c" && translatedText.trim()) {
        e.preventDefault();
        void (async () => {
          try {
            await writeText(translatedText);
          } catch {
            await navigator.clipboard?.writeText(translatedText);
          }
          void persistCurrentToHistory();
        })();
        return;
      }

      if (!e.shiftKey && e.key.toLowerCase() === "z") {
        const tag = (document.activeElement?.tagName ?? "").toUpperCase();
        if (tag !== "TEXTAREA" && tag !== "INPUT") {
          e.preventDefault();
          undoWordSwap();
        }
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [
    autoTranslate,
    controlsDisabled,
    persistCurrentToHistory,
    translateNow,
    translatedText,
    undoWordSwap,
  ]);

  return (
    <div className="flex flex-1 flex-col min-h-0 w-full gap-4">
      {!hasReadyModel && (
        <div className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-tag-optimize/40 bg-tag-optimize/10 px-4 py-3 text-sm">
          <span className="text-text-primary">
            No active AI model selected. Download Qwen 3 or import a local .gguf file first.
          </span>
          <button
            type="button"
            onClick={() => setActivePage("model-manager")}
            className="inline-flex items-center gap-1.5 rounded-lg bg-primary-container px-3.5 py-1.5 text-xs font-semibold text-on-primary hover:bg-primary-container/90 cursor-pointer"
          >
            <Cpu className="h-3.5 w-3.5" />
            Open Model Manager
          </button>
        </div>
      )}

      {subtitleLocked && (
        <div
          role="status"
          className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-tag-optimize/40 bg-tag-optimize/12 px-4 py-3 text-sm"
        >
          <div className="flex items-center gap-2.5 text-text-primary">
            <Lock className="h-4 w-4 shrink-0 text-tag-optimize" />
            <span>
              Subtitle Translator is currently processing an active batch. Pause or wait until it finishes to use Text Translator.
            </span>
          </div>
          <button
            type="button"
            onClick={() => setActivePage("subtitle-translator")}
            className="inline-flex items-center gap-1.5 rounded-lg bg-tag-optimize px-3 py-1.5 text-xs font-semibold text-on-primary hover:bg-tag-optimize/90 cursor-pointer"
          >
            <Captions className="h-3.5 w-3.5" />
            View Subtitle Progress
          </button>
        </div>
      )}

      {error && (
        <div
          role="alert"
          className="flex items-center justify-between gap-3 rounded-xl border border-status-danger/40 bg-status-danger/10 px-4 py-3 text-sm text-status-danger"
        >
          <div className="flex items-center gap-2">
            <AlertCircle className="h-4 w-4 shrink-0" />
            <span>{error.message}</span>
          </div>
          <button
            type="button"
            onClick={clearError}
            aria-label="Dismiss error message"
            className="rounded-lg p-1 hover:bg-surface-hover cursor-pointer"
          >
            <X className="h-4 w-4" />
          </button>
        </div>
      )}

      <SubToolbar disabled={controlsDisabled} />

      <div className="grid flex-1 min-h-0 grid-cols-1 lg:grid-cols-2 gap-4">
        <SourcePane disabled={controlsDisabled} />
        <TargetPane />
      </div>

      <TelemetryStrip />
    </div>
  );
}

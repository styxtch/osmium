import { readText } from "@tauri-apps/plugin-clipboard-manager";
import {
  ClipboardPaste,
  Eraser,
  Layers,
  Play,
  Square,
  Volume2,
} from "lucide-react";
import { getLanguageLabel } from "../../lib/languages";
import { computeTextStats } from "./tokenizer";
import {
  MAX_TEXT_CHARS,
  useTextTranslatorStore,
} from "../../stores/useTextTranslatorStore";
import { useEngineStore } from "../../stores/useEngineStore";
import { useTtsStore } from "../../stores/useTtsStore";

interface SourcePaneProps {
  disabled: boolean;
}

export function SourcePane({ disabled }: SourcePaneProps) {
  const sourceText = useTextTranslatorStore((s) => s.sourceText);
  const sourceLang = useTextTranslatorStore((s) => s.sourceLang);
  const detectedSourceLang = useTextTranslatorStore(
    (s) => s.detectedSourceLang
  );
  const autoTranslate = useTextTranslatorStore((s) => s.autoTranslate);
  const isStreaming = useTextTranslatorStore((s) => s.isStreaming);
  const setSourceText = useTextTranslatorStore((s) => s.setSourceText);
  const setAutoTranslate = useTextTranslatorStore((s) => s.setAutoTranslate);
  const clearSourceOnly = useTextTranslatorStore((s) => s.clearSourceOnly);
  const truncateToSafetyLimit = useTextTranslatorStore(
    (s) => s.truncateToSafetyLimit
  );
  const translateNow = useTextTranslatorStore((s) => s.translateNow);
  const cancelTranslation = useTextTranslatorStore((s) => s.cancelTranslation);
  const telemetry = useEngineStore((s) => s.telemetry);
  const speakingKey = useTtsStore((s) => s.speakingKey);
  const speak = useTtsStore((s) => s.speak);
  const isSpeakingSource = speakingKey === "source";

  const effectiveLang =
    sourceLang === "auto" && detectedSourceLang ? detectedSourceLang : sourceLang;
  const effectiveLabel = getLanguageLabel(effectiveLang);
  const sourceHeader =
    sourceLang === "auto"
      ? detectedSourceLang
        ? `${effectiveLabel.toUpperCase()} (DETECTED)`
        : "AUTO-DETECT (SOURCE)"
      : `${effectiveLabel.toUpperCase()} (SOURCE)`;

  const stats = computeTextStats(sourceText);
  const chunkThreshold = (telemetry?.ctx_size ?? 4096) <= 2048 ? 2500 : 5000;
  const isOverSafetyLimit = stats.chars > MAX_TEXT_CHARS;
  const isChunkingActive = stats.chars > chunkThreshold && !isOverSafetyLimit;

  const handlePaste = async () => {
    if (disabled) return;
    try {
      const clip = await readText();
      if (clip) setSourceText(clip);
    } catch {
      const clip = await navigator.clipboard?.readText();
      if (clip) setSourceText(clip);
    }
  };

  return (
    <section className="flex flex-1 min-h-[360px] flex-col rounded-xl border border-border-default bg-surface-card">
      {/* Header */}
      <div className="flex items-center justify-between gap-2 border-b border-border-subtle px-4 py-3">
        <div className="flex items-center gap-2">
          <span className="h-2 w-2 rounded-full bg-tag-organize" />
          <span className="text-xs font-semibold tracking-wider text-text-secondary">
            {sourceHeader}
          </span>
        </div>

        <div className="flex items-center gap-1.5">
          <button
            type="button"
            disabled={!sourceText.trim()}
            onClick={() => speak("source", sourceText, effectiveLang)}
            title={isSpeakingSource ? "Stop speech" : "Listen to source text"}
            aria-label={
              isSpeakingSource ? "Stop source audio" : "Listen to source text"
            }
            className={`inline-flex items-center gap-1.5 rounded-lg border px-2.5 py-1 text-xs font-medium disabled:opacity-40 cursor-pointer ${
              isSpeakingSource
                ? "border-primary/40 bg-primary-container/15 text-primary"
                : "border-border-default bg-surface-elevated text-text-secondary hover:bg-surface-hover hover:text-text-primary"
            }`}
          >
            {isSpeakingSource ? (
              <>
                <Square className="h-3.5 w-3.5" />
                Stop
              </>
            ) : (
              <>
                <Volume2 className="h-3.5 w-3.5" />
                Listen
              </>
            )}
          </button>

          <button
            type="button"
            disabled={disabled}
            onClick={() => void handlePaste()}
            className="inline-flex items-center gap-1.5 rounded-lg border border-border-default bg-surface-elevated px-2.5 py-1 text-xs font-medium text-text-secondary hover:bg-surface-hover hover:text-text-primary disabled:opacity-40 cursor-pointer"
          >
            <ClipboardPaste className="h-3.5 w-3.5" />
            Paste
          </button>

          <button
            type="button"
            disabled={disabled || !sourceText}
            onClick={clearSourceOnly}
            className="inline-flex items-center gap-1.5 rounded-lg border border-border-default bg-surface-elevated px-2.5 py-1 text-xs font-medium text-text-secondary hover:bg-surface-hover hover:text-text-primary disabled:opacity-40 cursor-pointer"
          >
            <Eraser className="h-3.5 w-3.5" />
            Clear
          </button>
        </div>
      </div>

      {/* Body */}
      <div className="flex flex-1 flex-col min-h-0 p-3.5">
        <textarea
          value={sourceText}
          disabled={disabled}
          onChange={(e) => setSourceText(e.target.value)}
          placeholder={
            disabled
              ? "Editor is temporarily locked..."
              : sourceLang === "auto" && !detectedSourceLang
                ? "Type or paste text in any language..."
                : `Type or paste ${effectiveLabel} text here...`
          }
          aria-label={`${effectiveLabel} source text`}
          className="h-full min-h-[260px] w-full flex-1 resize-none rounded-lg border border-border-default bg-surface-elevated p-3.5 text-sm leading-6 text-text-primary placeholder:text-text-muted focus:border-primary-container focus:outline-none disabled:opacity-60"
        />
      </div>

      {/* Footer: Stats + Auto-Chunking / Safety Limit + Manual Translate Button */}
      <div className="flex flex-wrap items-center justify-between gap-2 border-t border-border-subtle px-4 py-2.5 text-xs">
        <span
          className={`tabular-nums ${
            isOverSafetyLimit
              ? "font-semibold text-status-danger"
              : stats.chars > MAX_TEXT_CHARS * 0.9
                ? "text-tag-optimize"
                : "text-text-muted"
          }`}
        >
          {stats.chars.toLocaleString("en-US")} / 25,000 chars •{" "}
          {stats.sentences} sentences • {stats.paragraphs} paragraphs
        </span>

        <div className="flex flex-wrap items-center gap-2.5">
          {isChunkingActive && (
            <span className="inline-flex items-center gap-1 rounded-full border border-tag-security/30 bg-tag-security/12 px-2.5 py-0.5 text-[11px] font-medium text-tag-security">
              <Layers className="h-3 w-3" />
              Auto-Chunking Active
            </span>
          )}

          {isOverSafetyLimit && (
            <div className="flex items-center gap-2">
              <span className="text-[11px] font-medium text-status-danger">
                25,000-character limit exceeded
              </span>
              <button
                type="button"
                onClick={truncateToSafetyLimit}
                className="rounded-md border border-status-danger/40 bg-status-danger/15 px-2 py-0.5 text-[11px] font-semibold text-status-danger hover:bg-status-danger/25 cursor-pointer"
              >
                Trim to 25,000
              </button>
            </div>
          )}

          <label className="inline-flex items-center gap-2 text-xs text-text-secondary cursor-pointer select-none">
            <input
              type="checkbox"
              checked={autoTranslate}
              disabled={disabled}
              onChange={(e) => setAutoTranslate(e.target.checked)}
              className="h-3.5 w-3.5 accent-[#10B981] cursor-pointer"
            />
            <span>Auto-translate (600ms)</span>
          </label>

          {isStreaming && (
            <button
              type="button"
              onClick={() => void cancelTranslation()}
              className="inline-flex items-center gap-1.5 rounded-lg border border-status-danger/40 bg-status-danger/12 px-3 py-1.5 text-xs font-medium text-status-danger hover:bg-status-danger/20 cursor-pointer"
            >
              <Square className="h-3 w-3" />
              Stop
            </button>
          )}

          {!autoTranslate && !isStreaming && (
            <button
              type="button"
              disabled={disabled || !sourceText.trim() || isOverSafetyLimit}
              onClick={() => void translateNow()}
              title="Ctrl + Enter"
              className="inline-flex items-center gap-1.5 rounded-lg bg-primary-container px-3.5 py-1.5 text-xs font-semibold text-on-primary hover:bg-primary-container/90 disabled:opacity-40 cursor-pointer"
            >
              <Play className="h-3.5 w-3.5" />
              Translate
            </button>
          )}
        </div>
      </div>
    </section>
  );
}

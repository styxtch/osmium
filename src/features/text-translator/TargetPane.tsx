import { useMemo, useState } from "react";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { Check, CheckCircle2, Copy, Loader2, RotateCcw } from "lucide-react";
import { tokenizeIndonesianText } from "./tokenizer";
import { WordAlternativesPopover } from "./WordAlternativesPopover";
import { useTextTranslatorStore } from "../../stores/useTextTranslatorStore";

export function TargetPane() {
  const [copied, setCopied] = useState(false);

  const translatedText = useTextTranslatorStore((s) => s.translatedText);
  const isStreaming = useTextTranslatorStore((s) => s.isStreaming);
  const statusText = useTextTranslatorStore((s) => s.statusText);
  const chunkInfo = useTextTranslatorStore((s) => s.chunkInfo);
  const lastTtftMs = useTextTranslatorStore((s) => s.lastTtftMs);
  const undoStack = useTextTranslatorStore((s) => s.undoStack);
  const selectedWord = useTextTranslatorStore((s) => s.selectedWord);
  const contextSummary = useTextTranslatorStore((s) => s.contextSummary);
  const alternatives = useTextTranslatorStore((s) => s.alternatives);
  const loadingAlternatives = useTextTranslatorStore(
    (s) => s.loadingAlternatives
  );
  const alternativesError = useTextTranslatorStore((s) => s.alternativesError);

  const selectWordForAlternatives = useTextTranslatorStore(
    (s) => s.selectWordForAlternatives
  );
  const closeAlternatives = useTextTranslatorStore((s) => s.closeAlternatives);
  const applyAlternative = useTextTranslatorStore((s) => s.applyAlternative);
  const undoWordSwap = useTextTranslatorStore((s) => s.undoWordSwap);
  const persistCurrentToHistory = useTextTranslatorStore(
    (s) => s.persistCurrentToHistory
  );

  const tokens = useMemo(
    () => tokenizeIndonesianText(translatedText),
    [translatedText]
  );

  const handleCopy = async () => {
    if (!translatedText.trim()) return;
    try {
      await writeText(translatedText);
    } catch {
      await navigator.clipboard?.writeText(translatedText);
    }
    setCopied(true);
    void persistCurrentToHistory();
    setTimeout(() => setCopied(false), 1800);
  };

  return (
    <section className="flex flex-1 min-h-[360px] flex-col rounded-xl border border-border-default bg-surface-card">
      {/* Header */}
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-border-subtle px-4 py-3">
        <div className="flex items-center gap-2">
          <span className="h-2 w-2 rounded-full bg-primary" />
          <span className="text-xs font-semibold tracking-wider text-text-secondary">
            BAHASA INDONESIA
          </span>
        </div>

        <div className="flex items-center gap-2 text-xs">
          {isStreaming ? (
            <span className="inline-flex items-center gap-1.5 rounded-full border border-tag-optimize/30 bg-tag-optimize/12 px-2.5 py-0.5 text-[11px] font-medium text-tag-optimize">
              <Loader2 className="h-3 w-3 animate-spin" />
              {statusText}
              {chunkInfo ? ` (${chunkInfo.index}/${chunkInfo.total})` : ""}
            </span>
          ) : translatedText ? (
            <span className="inline-flex items-center gap-1.5 rounded-full border border-primary-container/30 bg-primary-container/12 px-2.5 py-0.5 text-[11px] font-medium text-primary tabular-nums">
              <CheckCircle2 className="h-3 w-3" />
              {statusText}
              {lastTtftMs != null
                ? ` (TTFT: ${(lastTtftMs / 1000).toFixed(2)}s)`
                : ""}
            </span>
          ) : (
            <span className="text-[11px] text-text-muted">{statusText}</span>
          )}
        </div>
      </div>

      {/* Body */}
      <div className="relative flex flex-1 flex-col min-h-0 p-3.5">
        <div
          aria-label="Indonesian translation output"
          className="h-full min-h-[260px] w-full flex-1 overflow-y-auto rounded-lg border border-border-default bg-surface-elevated/60 p-3.5 text-sm leading-6 text-text-primary whitespace-pre-wrap break-words"
        >
          {!translatedText ? (
            <span className="text-text-muted">
              {isStreaming
                ? "Waiting for first token from local model..."
                : "Translation output will appear here. Click any word once finished to view contextual synonym alternatives."}
            </span>
          ) : (
            tokens.map((tok) => {
              if (!tok.isWord) {
                return <span key={tok.id}>{tok.text}</span>;
              }
              const isSelected =
                selectedWord?.token.start === tok.start &&
                selectedWord?.token.end === tok.end;

              return (
                <button
                  key={tok.id}
                  type="button"
                  disabled={isStreaming}
                  onClick={(e) => {
                    const rect = e.currentTarget.getBoundingClientRect();
                    void selectWordForAlternatives(tok, {
                      top: rect.top,
                      left: rect.left,
                      bottom: rect.bottom,
                    });
                  }}
                  className={`inline rounded px-0.5 text-left transition-colors ${
                    isStreaming
                      ? "cursor-default"
                      : isSelected
                        ? "bg-primary-container/25 text-primary ring-1 ring-primary font-medium cursor-pointer"
                        : "hover:bg-surface-hover hover:text-primary cursor-pointer"
                  }`}
                >
                  {tok.text}
                </button>
              );
            })
          )}
        </div>

        {selectedWord && (
          <WordAlternativesPopover
            word={selectedWord.token.text}
            contextSummary={contextSummary}
            alternatives={alternatives}
            loading={loadingAlternatives}
            error={alternativesError}
            anchorRect={selectedWord.anchorRect}
            onSelect={applyAlternative}
            onClose={closeAlternatives}
          />
        )}
      </div>

      {/* Footer */}
      <div className="flex flex-wrap items-center justify-between gap-2 border-t border-border-subtle px-4 py-2.5">
        <span className="text-[11px] text-text-muted">
          {!isStreaming && translatedText
            ? "Click any word for contextual synonym alternatives"
            : "100% processed locally on-device"}
        </span>

        <div className="flex items-center gap-2">
          <button
            type="button"
            disabled={undoStack.length === 0 || isStreaming}
            onClick={undoWordSwap}
            title="Ctrl + Z"
            className="inline-flex items-center gap-1.5 rounded-lg border border-border-default bg-surface-elevated px-2.5 py-1.5 text-xs font-medium text-text-secondary hover:bg-surface-hover hover:text-text-primary disabled:opacity-40 cursor-pointer"
          >
            <RotateCcw className="h-3.5 w-3.5" />
            Undo
            {undoStack.length > 0 && (
              <span className="rounded-full bg-tag-optimize/20 px-1.5 py-0.2 text-[10px] font-semibold text-tag-optimize tabular-nums">
                {undoStack.length}
              </span>
            )}
          </button>

          <button
            type="button"
            disabled={!translatedText.trim()}
            onClick={() => void handleCopy()}
            title="Ctrl + Shift + C"
            className="inline-flex items-center gap-1.5 rounded-lg bg-primary-container px-3 py-1.5 text-xs font-semibold text-on-primary hover:bg-primary-container/90 disabled:opacity-40 cursor-pointer"
          >
            {copied ? (
              <>
                <Check className="h-3.5 w-3.5" />
                Copied
              </>
            ) : (
              <>
                <Copy className="h-3.5 w-3.5" />
                Copy Translation
              </>
            )}
          </button>
        </div>
      </div>
    </section>
  );
}

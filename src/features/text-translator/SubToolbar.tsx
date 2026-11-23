import { useState } from "react";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import {
  ArrowLeftRight,
  Check,
  CopyCheck,
  History,
  Trash2,
  Zap,
} from "lucide-react";
import type { Tone } from "../../lib/ipc";
import { useEngineStore } from "../../stores/useEngineStore";
import { useModelsStore } from "../../stores/useModelsStore";
import { useTextTranslatorStore } from "../../stores/useTextTranslatorStore";
import { QuickHistoryPopover } from "./QuickHistoryPopover";

interface SubToolbarProps {
  disabled: boolean;
}

const TONES: { id: Tone; label: string }[] = [
  { id: "natural", label: "Natural" },
  { id: "formal", label: "Formal" },
  { id: "casual", label: "Casual" },
];

export function SubToolbar({ disabled }: SubToolbarProps) {
  const [showHistory, setShowHistory] = useState(false);
  const [copiedBoth, setCopiedBoth] = useState(false);

  const installed = useModelsStore((s) => s.installed);
  const telemetry = useEngineStore((s) => s.telemetry);

  const sourceText = useTextTranslatorStore((s) => s.sourceText);
  const translatedText = useTextTranslatorStore((s) => s.translatedText);
  const tone = useTextTranslatorStore((s) => s.tone);
  const autoTranslate = useTextTranslatorStore((s) => s.autoTranslate);
  const isStreaming = useTextTranslatorStore((s) => s.isStreaming);
  const lastTokPerSec = useTextTranslatorStore((s) => s.lastTokPerSec);

  const setTone = useTextTranslatorStore((s) => s.setTone);
  const setAutoTranslate = useTextTranslatorStore((s) => s.setAutoTranslate);
  const clearAll = useTextTranslatorStore((s) => s.clearAll);
  const loadFromHistory = useTextTranslatorStore((s) => s.loadFromHistory);
  const persistCurrentToHistory = useTextTranslatorStore(
    (s) => s.persistCurrentToHistory
  );

  const activeModelName =
    installed.find((m) => m.is_active)?.display_name ??
    telemetry?.active_model_name ??
    "No Model";

  const handleCopyBoth = async () => {
    if (!sourceText.trim() && !translatedText.trim()) return;
    const combined = `EN:\n${sourceText}\n\nID:\n${translatedText}`;
    try {
      await writeText(combined);
    } catch {
      await navigator.clipboard?.writeText(combined);
    }
    setCopiedBoth(true);
    void persistCurrentToHistory();
    setTimeout(() => setCopiedBoth(false), 1800);
  };

  return (
    <div className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-border-default bg-surface-card px-4 py-2.5">
      {/* Left: Language Pair Indicator */}
      <div className="flex flex-wrap items-center gap-2 rounded-full border border-border-default bg-surface-container px-3 py-1.5 text-xs font-medium">
        <span className="text-text-primary">English (Source)</span>
        <button
          type="button"
          disabled
          title="ID → EN direction coming in v1.1"
          aria-label="Swap languages (coming in v1.1)"
          className="rounded-full p-1 text-text-muted opacity-60 cursor-not-allowed"
        >
          <ArrowLeftRight className="h-3.5 w-3.5" />
        </button>
        <span className="inline-flex items-center gap-1.5 text-text-primary">
          <span className="h-1.5 w-1.5 rounded-full bg-primary" />
          Indonesian [{activeModelName}]
        </span>
      </div>

      {/* Center: Tone Segmented Pills */}
      <div
        role="group"
        aria-label="Select translation tone"
        className="flex items-center gap-1 rounded-lg border border-border-default bg-surface-container p-1"
      >
        {TONES.map((t) => {
          const active = tone === t.id;
          return (
            <button
              key={t.id}
              type="button"
              disabled={disabled}
              onClick={() => setTone(t.id)}
              className={`rounded-md px-3 py-1 text-xs font-medium transition-colors cursor-pointer disabled:opacity-50 ${
                active
                  ? "bg-primary-container text-on-primary font-semibold"
                  : "text-text-secondary hover:text-text-primary hover:bg-surface-hover"
              }`}
            >
              {t.label}
            </button>
          );
        })}
      </div>

      {/* Right: Auto-translate toggle, speed badge & 3 Quick Utilities */}
      <div className="relative flex flex-wrap items-center gap-2.5">
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

        {(isStreaming || lastTokPerSec != null) && (
          <span className="hidden sm:inline-flex items-center gap-1 rounded-full border border-primary-container/30 bg-primary-container/10 px-2.5 py-1 text-[11px] font-medium text-primary tabular-nums">
            <Zap className="h-3 w-3" />
            {isStreaming
              ? "Streaming active"
              : `${(lastTokPerSec ?? 0).toFixed(1)} tok/s`}
          </span>
        )}

        <div className="flex items-center gap-1 border-l border-border-subtle pl-2">
          <button
            type="button"
            disabled={disabled && !sourceText && !translatedText}
            onClick={clearAll}
            title="Clear All"
            aria-label="Clear All"
            className="rounded-lg p-1.5 text-text-secondary hover:bg-surface-hover hover:text-text-primary disabled:opacity-40 cursor-pointer"
          >
            <Trash2 className="h-4 w-4" />
          </button>

          <button
            type="button"
            disabled={!sourceText.trim() && !translatedText.trim()}
            onClick={() => void handleCopyBoth()}
            title="Copy Both (EN+ID)"
            aria-label="Copy Both (EN+ID)"
            className="rounded-lg p-1.5 text-text-secondary hover:bg-surface-hover hover:text-text-primary disabled:opacity-40 cursor-pointer"
          >
            {copiedBoth ? (
              <Check className="h-4 w-4 text-primary" />
            ) : (
              <CopyCheck className="h-4 w-4" />
            )}
          </button>

          <button
            type="button"
            onClick={() => setShowHistory((v) => !v)}
            title="Quick History"
            aria-label="Quick History"
            className={`rounded-lg p-1.5 transition-colors cursor-pointer ${
              showHistory
                ? "bg-surface-elevated text-primary"
                : "text-text-secondary hover:bg-surface-hover hover:text-text-primary"
            }`}
          >
            <History className="h-4 w-4" />
          </button>
        </div>

        {showHistory && (
          <QuickHistoryPopover
            onSelect={loadFromHistory}
            onClose={() => setShowHistory(false)}
          />
        )}
      </div>
    </div>
  );
}

import { useState } from "react";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { ArrowLeftRight, Check, CopyCheck, History, Trash2 } from "lucide-react";
import type { Tone } from "../../lib/ipc";
import {
  getLanguageLabel,
  getLanguageShortCode,
  SOURCE_LANGUAGES,
  TARGET_LANGUAGES,
} from "../../lib/languages";
import { useTextTranslatorStore } from "../../stores/useTextTranslatorStore";
import { QuickHistoryPopover } from "./QuickHistoryPopover";

interface SubToolbarProps {
  disabled: boolean;
}

const TONES: { id: Tone; label: string; description: string }[] = [
  {
    id: "natural",
    label: "Natural",
    description:
      "Auto-adapts like DeepL: keeps novel narrative fluent, matches dialogue voice, and preserves formal prose.",
  },
  {
    id: "formal",
    label: "Formal",
    description: "Explicit override: polite, structured, and standard formal register.",
  },
  {
    id: "casual",
    label: "Casual",
    description: "Explicit override: relaxed, conversational everyday register.",
  },
];

export function SubToolbar({ disabled }: SubToolbarProps) {
  const [showHistory, setShowHistory] = useState(false);
  const [copiedBoth, setCopiedBoth] = useState(false);

  const sourceText = useTextTranslatorStore((s) => s.sourceText);
  const translatedText = useTextTranslatorStore((s) => s.translatedText);
  const sourceLang = useTextTranslatorStore((s) => s.sourceLang);
  const detectedSourceLang = useTextTranslatorStore(
    (s) => s.detectedSourceLang
  );
  const targetLang = useTextTranslatorStore((s) => s.targetLang);
  const tone = useTextTranslatorStore((s) => s.tone);
  const isStreaming = useTextTranslatorStore((s) => s.isStreaming);

  const setSourceLang = useTextTranslatorStore((s) => s.setSourceLang);
  const setTargetLang = useTextTranslatorStore((s) => s.setTargetLang);
  const swapLanguages = useTextTranslatorStore((s) => s.swapLanguages);
  const setTone = useTextTranslatorStore((s) => s.setTone);
  const clearAll = useTextTranslatorStore((s) => s.clearAll);
  const loadFromHistory = useTextTranslatorStore((s) => s.loadFromHistory);
  const persistCurrentToHistory = useTextTranslatorStore(
    (s) => s.persistCurrentToHistory
  );

  const effectiveSourceLang =
    sourceLang === "auto" && detectedSourceLang ? detectedSourceLang : sourceLang;
  const srcShort = getLanguageShortCode(effectiveSourceLang);
  const dstShort = getLanguageShortCode(targetLang);

  const handleCopyBoth = async () => {
    if (!sourceText.trim() && !translatedText.trim()) return;
    const combined = `${srcShort}:\n${sourceText}\n\n${dstShort}:\n${translatedText}`;
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
    <div className="flex flex-wrap items-center justify-between gap-3 lg:grid lg:grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] lg:items-center rounded-xl border border-border-default bg-surface-card px-4 py-2.5">
      {/* Left: Interactive Multi-Language Pair */}
      <div className="flex flex-wrap items-center gap-2 lg:justify-self-start min-w-0 max-w-full rounded-xl border border-border-default bg-surface-container px-2.5 py-1.5 text-xs font-medium">
        <select
          aria-label="Source language"
          disabled={disabled || isStreaming}
          value={sourceLang}
          onChange={(e) => setSourceLang(e.target.value)}
          className="rounded-lg border border-border-subtle bg-surface-elevated px-2.5 py-1 text-xs font-medium text-text-primary focus:border-primary focus:outline-none disabled:opacity-50 cursor-pointer"
        >
          {SOURCE_LANGUAGES.map((lang) => (
            <option
              key={lang.code}
              value={lang.code}
              className="bg-surface-elevated text-text-primary"
            >
              {lang.code === "auto" && detectedSourceLang
                ? `Auto-detect (${getLanguageLabel(detectedSourceLang)})`
                : lang.name}
            </option>
          ))}
        </select>

        <button
          type="button"
          disabled={disabled || isStreaming}
          onClick={swapLanguages}
          title="Swap languages"
          aria-label="Swap languages"
          className="rounded-lg p-1.5 text-text-secondary hover:bg-surface-hover hover:text-primary transition-colors disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer"
        >
          <ArrowLeftRight className="h-3.5 w-3.5" />
        </button>

        <select
          aria-label="Target language"
          disabled={disabled || isStreaming}
          value={targetLang}
          onChange={(e) => setTargetLang(e.target.value)}
          className="rounded-lg border border-border-subtle bg-surface-elevated px-2.5 py-1 text-xs font-medium text-text-primary focus:border-primary focus:outline-none disabled:opacity-50 cursor-pointer"
        >
          {TARGET_LANGUAGES.map((lang) => (
            <option
              key={lang.code}
              value={lang.code}
              className="bg-surface-elevated text-text-primary"
            >
              {lang.name}
            </option>
          ))}
        </select>
      </div>

      {/* Center: Tone Segmented Pills */}
      <div
        role="group"
        aria-label="Select translation tone"
        className="flex items-center gap-1 lg:justify-self-center shrink-0 rounded-lg border border-border-default bg-surface-container p-1"
      >
        {TONES.map((t) => {
          const active = tone === t.id;
          return (
            <button
              key={t.id}
              type="button"
              disabled={disabled}
              title={t.description}
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

      {/* Right: 3 Quick Utilities */}
      <div className="relative flex flex-wrap items-center justify-end gap-1 lg:justify-self-end">
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
          title={`Copy Both (${srcShort}+${dstShort})`}
          aria-label={`Copy Both (${srcShort}+${dstShort})`}
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

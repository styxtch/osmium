import { useState } from "react";
import { Check, X } from "lucide-react";

interface InlineCueEditorProps {
  initialText: string;
  leadingTags?: string;
  trailingTags?: string;
  tagShieldEnabled: boolean;
  onSave: (editedText: string) => Promise<boolean>;
  onCancel: () => void;
}

function stripOuterTagsForEdit(
  fullText: string,
  leading?: string,
  trailing?: string,
  tagShieldEnabled?: boolean
): string {
  if (!tagShieldEnabled) return fullText;
  let inner = fullText;
  if (leading && inner.startsWith(leading)) {
    inner = inner.slice(leading.length);
  }
  if (trailing && inner.endsWith(trailing) && inner.length >= trailing.length) {
    inner = inner.slice(0, inner.length - trailing.length);
  }
  return inner;
}

export function InlineCueEditor({
  initialText,
  leadingTags,
  trailingTags,
  tagShieldEnabled,
  onSave,
  onCancel,
}: InlineCueEditorProps) {
  const [draft, setDraft] = useState(() =>
    stripOuterTagsForEdit(
      initialText,
      leadingTags,
      trailingTags,
      tagShieldEnabled
    )
  );
  const [isSaving, setIsSaving] = useState(false);

  const handleSave = async () => {
    if (isSaving) return;
    setIsSaving(true);
    await onSave(draft);
    setIsSaving(false);
  };

  return (
    <div className="space-y-2 py-1">
      {tagShieldEnabled && (leadingTags || trailingTags) && (
        <div className="flex flex-wrap items-center gap-1.5 text-[10px]">
          <span className="text-text-muted">Protected tags:</span>
          {leadingTags && (
            <code className="px-1.5 py-0.5 rounded bg-surface-elevated border border-border-default text-tag-security">
              {leadingTags}
            </code>
          )}
          {trailingTags && (
            <code className="px-1.5 py-0.5 rounded bg-surface-elevated border border-border-default text-tag-security">
              {trailingTags}
            </code>
          )}
        </div>
      )}

      <textarea
        aria-label="Edit translated dialogue"
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            e.preventDefault();
            onCancel();
          } else if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
            e.preventDefault();
            void handleSave();
          }
        }}
        rows={2}
        autoFocus
        className="w-full bg-bg-canvas border border-primary rounded-lg px-2.5 py-1.5 text-xs text-text-primary focus:outline-none resize-y"
      />

      <div className="flex items-center justify-end gap-2">
        <button
          type="button"
          onClick={onCancel}
          className="inline-flex items-center gap-1 px-2.5 py-1 rounded bg-surface-elevated hover:bg-surface-hover text-[11px] text-text-secondary transition-colors"
        >
          <X className="w-3 h-3" />
          Cancel
        </button>
        <button
          type="button"
          disabled={isSaving}
          onClick={() => void handleSave()}
          className="inline-flex items-center gap-1 px-2.5 py-1 rounded bg-primary-container hover:bg-primary text-[11px] font-semibold text-on-primary transition-colors disabled:opacity-50"
        >
          <Check className="w-3 h-3" />
          {isSaving ? "Saving..." : "Save (Ctrl+Enter)"}
        </button>
      </div>
    </div>
  );
}

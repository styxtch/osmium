import { useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import {
  AlertTriangle,
  CheckCircle2,
  Clock,
  Loader2,
  Pencil,
  RotateCw,
  Shield,
  Square,
  Tag,
  Volume2,
} from "lucide-react";
import {
  parseCueMeta,
  parseGlossaryMatches,
  type SubtitleCueDto,
  type SubtitleJobStatus,
} from "../../lib/ipc";
import { getLanguageLabel } from "../../lib/languages";
import { useTtsStore } from "../../stores/useTtsStore";
import { InlineCueEditor } from "./InlineCueEditor";

interface SubtitleVirtualTableProps {
  cues: SubtitleCueDto[];
  targetLang: string;
  jobStatus: SubtitleJobStatus;
  tagShieldEnabled: boolean;
  editingCueIndex: number | null;
  retranslatingCueIndex: number | null;
  isLoadingCues: boolean;
  onStartEdit: (cueIndex: number | null) => void;
  onSaveEdit: (cueIndex: number, editedText: string) => Promise<boolean>;
  onRetranslateRow: (cueIndex: number) => Promise<void>;
}

export function SubtitleVirtualTable({
  cues,
  targetLang,
  jobStatus,
  tagShieldEnabled,
  editingCueIndex,
  retranslatingCueIndex,
  isLoadingCues,
  onStartEdit,
  onSaveEdit,
  onRetranslateRow,
}: SubtitleVirtualTableProps) {
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const speakingKey = useTtsStore((s) => s.speakingKey);
  const speak = useTtsStore((s) => s.speak);

  const rowVirtualizer = useVirtualizer({
    count: cues.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => 88,
    overscan: 8,
    initialRect: { width: 1200, height: 640 },
  });

  const canModifyRows = jobStatus === "paused" || jobStatus === "completed";

  if (isLoadingCues && cues.length === 0) {
    return (
      <div className="flex-1 min-h-0 bg-surface-card border border-border-default rounded-xl flex items-center justify-center text-xs text-text-muted">
        <Loader2 className="w-4 h-4 animate-spin mr-2 text-primary" />
        Loading subtitle cues...
      </div>
    );
  }

  if (cues.length === 0) {
    return (
      <div className="flex-1 min-h-0 bg-surface-card border border-border-default rounded-xl flex items-center justify-center text-xs text-text-muted">
        No subtitle cues match the active filter.
      </div>
    );
  }

  const virtualItems = rowVirtualizer.getVirtualItems();

  return (
    <div className="flex-1 min-h-0 bg-surface-card border border-border-default rounded-xl flex flex-col overflow-hidden">
      {/* Table Column Header */}
      <div className="grid grid-cols-[64px_156px_minmax(0,1fr)_minmax(0,1fr)_132px] gap-3 px-4 py-2.5 border-b border-border-default bg-surface-container text-[11px] font-semibold uppercase tracking-wide text-text-muted shrink-0">
        <div>#</div>
        <div>Timestamp</div>
        <div>Source Dialogue</div>
        <div>{getLanguageLabel(targetLang)} Translation</div>
        <div className="text-right">Status / Actions</div>
      </div>

      {/* Virtualized Scroll Viewport */}
      <div ref={scrollRef} className="flex-1 min-h-0 overflow-auto">
        <div
          style={{
            height: `${rowVirtualizer.getTotalSize()}px`,
            width: "100%",
            position: "relative",
          }}
        >
          {virtualItems.map((virtualRow) => {
            const cue = cues[virtualRow.index];
            if (!cue) return null;

            const isDialogue = cue.cue_type === "dialogue";
            const isEditing = editingCueIndex === cue.cue_index;
            const isRetranslating = retranslatingCueIndex === cue.cue_index;
            const rowTtsKey = `subtitle-${cue.cue_index}`;
            const isSpeakingCue = speakingKey === rowTtsKey;
            const meta = parseCueMeta(cue.meta_json);
            const glossaryMatches = parseGlossaryMatches(
              cue.glossary_matches_json
            );
            const displayTags = meta.display_tags ?? [];

            return (
              <div
                key={cue.cue_index}
                data-index={virtualRow.index}
                ref={rowVirtualizer.measureElement}
                style={{
                  position: "absolute",
                  top: 0,
                  left: 0,
                  width: "100%",
                  transform: `translateY(${virtualRow.start}px)`,
                }}
                className={`grid grid-cols-[64px_156px_minmax(0,1fr)_minmax(0,1fr)_132px] gap-3 px-4 py-3 border-b border-border-subtle text-xs items-start ${
                  cue.status === "processing" || isRetranslating
                    ? "bg-primary/5"
                    : cue.status === "warning"
                      ? "bg-tag-optimize/5"
                      : !isDialogue
                        ? "bg-surface-container/60"
                        : "hover:bg-surface-elevated/40"
                }`}
              >
                {/* Cue Index + Non-dialogue badge */}
                <div className="space-y-1 tabular-nums">
                  <span className="font-mono text-[11px] font-semibold text-text-secondary">
                    {cue.cue_index}
                  </span>
                  {!isDialogue && (
                    <span className="block w-fit px-1.5 py-0.5 rounded bg-surface-elevated text-[10px] font-medium capitalize text-text-muted">
                      {cue.cue_type}
                    </span>
                  )}
                </div>

                {/* Timestamps */}
                <div className="text-[11px] font-mono text-text-muted tabular-nums leading-relaxed">
                  {cue.start_time || "--:--:--"}
                  <br />
                  {cue.end_time || "--:--:--"}
                </div>

                {/* Source Dialogue + Tag Pills + Glossary Pills */}
                <div className="min-w-0 space-y-1.5">
                  <p className="text-text-primary whitespace-pre-wrap break-words leading-relaxed">
                    {cue.original_text}
                  </p>

                  {(displayTags.length > 0 || glossaryMatches.length > 0) && (
                    <div className="flex flex-wrap items-center gap-1">
                      {displayTags.map((tag, idx) => (
                        <span
                          key={`${tag}-${idx}`}
                          className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-surface-elevated border border-border-default text-[10px] font-mono text-tag-organize"
                        >
                          <Shield className="w-2.5 h-2.5" />
                          {tag}
                        </span>
                      ))}
                      {glossaryMatches.map((m, idx) => (
                        <span
                          key={`${m.source_term}-${idx}`}
                          title={`${m.source_term} -> ${m.target_term}`}
                          className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-tag-security/15 text-[10px] font-medium text-tag-security"
                        >
                          <Tag className="w-2.5 h-2.5" />
                          {m.source_term} → {m.target_term}
                        </span>
                      ))}
                    </div>
                  )}
                </div>

                {/* Indonesian Translation / Processing / Inline Editor */}
                <div className="min-w-0 space-y-1.5">
                  {!isDialogue ? (
                    <p className="text-text-muted italic text-[11px]">
                      Preserved verbatim ({cue.cue_type})
                    </p>
                  ) : isEditing ? (
                    <InlineCueEditor
                      initialText={
                        cue.translated_text ?? cue.original_text
                      }
                      leadingTags={meta.leading_tags}
                      trailingTags={meta.trailing_tags}
                      tagShieldEnabled={tagShieldEnabled}
                      onSave={(edited) => onSaveEdit(cue.cue_index, edited)}
                      onCancel={() => onStartEdit(null)}
                    />
                  ) : cue.status === "processing" || isRetranslating ? (
                    <span className="inline-flex items-center gap-1.5 text-primary font-medium animate-pulse">
                      <Loader2 className="w-3.5 h-3.5 animate-spin" />
                      Processing...
                    </span>
                  ) : cue.status === "queued" ? (
                    <span className="text-text-disabled italic">Queued</span>
                  ) : (
                    <>
                      <p className="text-text-primary whitespace-pre-wrap break-words leading-relaxed">
                        {cue.translated_text ?? ""}
                      </p>
                      {cue.status === "warning" && cue.warning_msg && (
                        <p className="text-[11px] text-tag-optimize flex items-start gap-1">
                          <AlertTriangle className="w-3.5 h-3.5 shrink-0 mt-0.5" />
                          <span>{cue.warning_msg}</span>
                        </p>
                      )}
                    </>
                  )}
                </div>

                {/* Status Badge & Edit / Re-translate Actions */}
                <div className="flex flex-col items-end gap-1.5">
                  <CueStatusBadge
                    status={isRetranslating ? "processing" : cue.status}
                  />

                  {isDialogue && !isEditing && (
                    <div className="flex items-center gap-1">
                      <button
                        type="button"
                        aria-label={`Listen cue ${cue.cue_index}`}
                        disabled={!cue.translated_text?.trim()}
                        onClick={() =>
                          speak(
                            rowTtsKey,
                            cue.translated_text ?? "",
                            targetLang
                          )
                        }
                        title={
                          isSpeakingCue
                            ? "Stop speech"
                            : "Listen to translated dialogue"
                        }
                        className={`p-1.5 rounded border transition-colors disabled:opacity-35 disabled:cursor-not-allowed ${
                          isSpeakingCue
                            ? "bg-primary/15 border-primary/40 text-primary"
                            : "bg-surface-elevated hover:bg-surface-hover border-border-default text-text-secondary hover:text-primary"
                        }`}
                      >
                        {isSpeakingCue ? (
                          <Square className="w-3 h-3" />
                        ) : (
                          <Volume2 className="w-3 h-3" />
                        )}
                      </button>

                      <button
                        type="button"
                        aria-label={`Edit cue ${cue.cue_index}`}
                        disabled={!canModifyRows || retranslatingCueIndex !== null}
                        onClick={() => onStartEdit(cue.cue_index)}
                        title={
                          canModifyRows
                            ? "Edit translation manually"
                            : "Pause or complete the job before editing rows"
                        }
                        className="p-1.5 rounded bg-surface-elevated hover:bg-surface-hover border border-border-default text-text-secondary hover:text-text-primary transition-colors disabled:opacity-35 disabled:cursor-not-allowed"
                      >
                        <Pencil className="w-3 h-3" />
                      </button>

                      <button
                        type="button"
                        aria-label={`Re-translate cue ${cue.cue_index}`}
                        disabled={!canModifyRows || retranslatingCueIndex !== null}
                        onClick={() => void onRetranslateRow(cue.cue_index)}
                        title={
                          canModifyRows
                            ? "Re-translate 1 row"
                            : "Pause or complete the job before re-translating a row"
                        }
                        className="p-1.5 rounded bg-surface-elevated hover:bg-surface-hover border border-border-default text-text-secondary hover:text-primary transition-colors disabled:opacity-35 disabled:cursor-not-allowed"
                      >
                        <RotateCw
                          className={`w-3 h-3 ${
                            isRetranslating ? "animate-spin text-primary" : ""
                          }`}
                        />
                      </button>
                    </div>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}

function CueStatusBadge({ status }: { status: string }) {
  switch (status) {
    case "done":
      return (
        <span className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-status-success/15 text-status-success text-[10px] font-semibold">
          <CheckCircle2 className="w-3 h-3" />
          Done
        </span>
      );
    case "processing":
      return (
        <span className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-primary/15 text-primary text-[10px] font-semibold">
          <Loader2 className="w-3 h-3 animate-spin" />
          Processing
        </span>
      );
    case "warning":
      return (
        <span className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-tag-optimize/15 text-tag-optimize text-[10px] font-semibold">
          <AlertTriangle className="w-3 h-3" />
          Warning
        </span>
      );
    case "skipped":
      return (
        <span className="px-1.5 py-0.5 rounded bg-surface-elevated text-text-muted text-[10px] font-medium">
          Preserved
        </span>
      );
    default:
      return (
        <span className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-surface-elevated text-text-muted text-[10px] font-medium">
          <Clock className="w-3 h-3" />
          Queued
        </span>
      );
  }
}

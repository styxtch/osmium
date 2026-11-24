import { Activity, Cpu } from "lucide-react";
import { formatVramGb } from "../../lib/ipc";
import { useEngineStore } from "../../stores/useEngineStore";
import { useModelsStore } from "../../stores/useModelsStore";
import { useSettingsStore } from "../../stores/useSettingsStore";
import { useTextTranslatorStore } from "../../stores/useTextTranslatorStore";

export function TelemetryStrip() {
  const settings = useSettingsStore((s) => s.settings);
  const telemetry = useEngineStore((s) => s.telemetry);
  const hardware = useEngineStore((s) => s.hardware);
  const vramUsedMb = useEngineStore((s) => s.vramUsedMb);
  const vramTotalMb = useEngineStore((s) => s.vramTotalMb);
  const installed = useModelsStore((s) => s.installed);

  const lastTtftMs = useTextTranslatorStore((s) => s.lastTtftMs);
  const lastTokPerSec = useTextTranslatorStore((s) => s.lastTokPerSec);
  const lastTotalTokens = useTextTranslatorStore((s) => s.lastTotalTokens);

  if (settings?.display_mode === "compact") {
    return null;
  }

  const activeModel = installed.find((m) => m.is_active);
  const totalMb = vramTotalMb || hardware?.dedicated_vram_mb || 0;
  const backendLabel =
    telemetry?.backend === "cpu" ? "CPU Mode" : "Vulkan Native";

  const metricsParts = [
    lastTotalTokens != null ? `${lastTotalTokens} tokens` : null,
    lastTokPerSec != null ? `${lastTokPerSec.toFixed(1)} t/s` : null,
    lastTtftMs != null ? `TTFT ${(lastTtftMs / 1000).toFixed(2)}s` : null,
  ].filter((part): part is string => part !== null);

  return (
    <footer
      aria-label="Local performance telemetry strip"
      className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-border-default bg-surface-card px-4 py-2.5 text-xs text-text-secondary tabular-nums"
    >
      <div className="flex flex-wrap items-center gap-3">
        <span className="inline-flex items-center gap-1.5">
          <Cpu className="h-3.5 w-3.5 text-primary" />
          GPU VRAM: {formatVramGb(vramUsedMb)} / {formatVramGb(totalMb)}{" "}
          {telemetry?.backend !== "cpu" && "(Q8_0 KV Cache + Flash Attention)"}
        </span>

        <span className="text-border-default">•</span>

        <span className="inline-flex items-center gap-1.5">
          <Activity className="h-3.5 w-3.5 text-tag-security" />
          Token Delta:{" "}
          {metricsParts.length > 0 ? metricsParts.join(" • ") : "Idle"}
        </span>
      </div>

      <div className="flex items-center gap-2">
        <span className="rounded-full border border-border-default bg-surface-container px-2.5 py-0.5 text-[11px] font-medium text-text-primary">
          Model: {activeModel?.display_name ?? telemetry?.active_model_name ?? "Not Selected"}{" "}
          ({backendLabel})
        </span>
      </div>
    </footer>
  );
}

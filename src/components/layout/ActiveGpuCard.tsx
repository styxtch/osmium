import { Cpu } from "lucide-react";
import { formatVramGb } from "../../lib/ipc";
import { useEngineStore } from "../../stores/useEngineStore";
import { useModelsStore } from "../../stores/useModelsStore";
import { useNavigationStore } from "../../stores/useNavigationStore";

export function ActiveGpuCard() {
  const hardware = useEngineStore((s) => s.hardware);
  const telemetry = useEngineStore((s) => s.telemetry);
  const vramUsedMb = useEngineStore((s) => s.vramUsedMb);
  const vramTotalMb = useEngineStore((s) => s.vramTotalMb);
  const installed = useModelsStore((s) => s.installed);
  const setActivePage = useNavigationStore((s) => s.setActivePage);

  const hasActiveModel = installed.some((m) => m.is_active && m.file_exists);
  const status = telemetry?.status ?? (hasActiveModel ? "standby" : "unconfigured");

  const statusConfig = (() => {
    switch (status) {
      case "online":
        return { label: "Online", dot: "bg-status-success", text: "text-status-success" };
      case "loading":
        return { label: "Loading", dot: "bg-tag-optimize animate-pulse", text: "text-tag-optimize" };
      case "standby":
        return { label: "Standby", dot: "bg-primary", text: "text-primary" };
      case "error":
        return { label: "Error", dot: "bg-status-danger", text: "text-status-danger" };
      default:
        return { label: "Setup", dot: "bg-tag-optimize", text: "text-tag-optimize" };
    }
  })();

  const totalMb = vramTotalMb || hardware?.dedicated_vram_mb || 0;
  const usedPct = totalMb > 0 ? Math.min(100, Math.round((vramUsedMb / totalMb) * 100)) : 0;
  const ttftLabel =
    telemetry?.last_ttft_ms != null
      ? `${(telemetry.last_ttft_ms / 1000).toFixed(2)}s`
      : "Idle";

  return (
    <button
      type="button"
      onClick={() => setActivePage("model-manager")}
      title={hardware?.gpu_name ?? "Open Model Manager & Hardware Diagnostics"}
      className="w-full rounded-xl border border-border-default bg-surface-card p-2.5 text-left transition-colors hover:bg-surface-hover cursor-pointer"
    >
      <div className="flex items-center justify-center xl:justify-between gap-2">
        <span className="flex items-center gap-1.5 text-xs font-medium text-text-secondary">
          <Cpu className="h-3.5 w-3.5 shrink-0 text-primary" />
          <span className="hidden xl:inline">Active GPU</span>
        </span>
        <span className={`hidden xl:inline-flex items-center gap-1.5 text-[11px] font-medium ${statusConfig.text}`}>
          <span className={`h-1.5 w-1.5 rounded-full ${statusConfig.dot}`} />
          {statusConfig.label}
        </span>
      </div>

      <div className="hidden xl:block mt-1.5">
        <div className="truncate text-xs font-medium text-text-primary">
          {hardware?.gpu_name ?? "Scanning DXGI GPU..."}
        </div>
        <div className="mt-2 h-1.5 w-full overflow-hidden rounded-full bg-surface-elevated">
          <div
            className="h-full rounded-full bg-primary-container transition-all duration-300"
            style={{ width: `${usedPct}%` }}
          />
        </div>
        <div className="mt-1.5 flex items-center justify-between text-[11px] text-text-muted tabular-nums">
          <span>
            VRAM {formatVramGb(vramUsedMb)} / {formatVramGb(totalMb)}
          </span>
          <span>{ttftLabel}</span>
        </div>
      </div>
    </button>
  );
}

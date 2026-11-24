import { Cpu, Power } from "lucide-react";
import { useEngineStore } from "../../stores/useEngineStore";
import { useModelsStore } from "../../stores/useModelsStore";
import { useNavigationStore } from "../../stores/useNavigationStore";

export function ActiveGpuCard() {
  const hardware = useEngineStore((s) => s.hardware);
  const telemetry = useEngineStore((s) => s.telemetry);
  const vramUsedMb = useEngineStore((s) => s.vramUsedMb);
  const vramTotalMb = useEngineStore((s) => s.vramTotalMb);
  const gpuTempC = useEngineStore((s) => s.gpuTempC);
  const ramUsedGb = useEngineStore((s) => s.ramUsedGb);
  const ramTotalGb = useEngineStore((s) => s.ramTotalGb);
  const unloading = useEngineStore((s) => s.unloading);
  const unloadModel = useEngineStore((s) => s.unloadModel);
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
  const totalRam = ramTotalGb || hardware?.system_ram_gb || 0;
  const ramPct = totalRam > 0 ? Math.min(100, Math.round((ramUsedGb / totalRam) * 100)) : 0;
  const isCpuMode = telemetry?.backend === "cpu";
  const tempColorClass =
    gpuTempC == null
      ? ""
      : gpuTempC >= 85
        ? "text-status-danger"
        : gpuTempC >= 75
          ? "text-tag-optimize"
          : "text-text-secondary";

  return (
    <div
      role="button"
      tabIndex={0}
      onClick={() => setActivePage("model-manager")}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          setActivePage("model-manager");
        }
      }}
      title={hardware?.gpu_name ?? "Open Model Manager & Hardware Diagnostics"}
      className="w-full rounded-xl border border-border-default bg-surface-card p-2.5 text-left transition-colors hover:bg-surface-hover cursor-pointer"
    >
      <div className="flex items-center justify-center xl:justify-between gap-2">
        <span className="flex items-center gap-1.5 text-xs font-medium text-text-secondary">
          <Cpu className="h-3.5 w-3.5 shrink-0 text-primary" />
          <span className="hidden xl:inline">
            {isCpuMode ? "CPU Mode" : "Active GPU"}
          </span>
        </span>
        <div className="hidden xl:flex items-center gap-1.5">
          <span className={`inline-flex items-center gap-1.5 text-[11px] font-medium ${statusConfig.text}`}>
            <span className={`h-1.5 w-1.5 rounded-full ${statusConfig.dot}`} />
            {statusConfig.label}
          </span>
          {status === "online" && (
            <button
              type="button"
              disabled={telemetry?.busy_by !== "none" || unloading}
              onClick={(e) => {
                e.stopPropagation();
                void unloadModel();
              }}
              title={
                telemetry?.busy_by !== "none"
                  ? "Busy translating"
                  : "Unload active model from VRAM"
              }
              aria-label="Unload active model from VRAM"
              className="rounded p-1 text-text-secondary hover:bg-surface-elevated hover:text-tag-optimize disabled:opacity-40 cursor-pointer"
            >
              <Power className="h-3 w-3" />
            </button>
          )}
        </div>
      </div>

      <div className="hidden xl:block mt-1.5 space-y-2">
        <div className="flex items-center justify-between gap-2">
          <span className="truncate text-xs font-medium text-text-primary">
            {hardware?.gpu_name ?? "Scanning DXGI GPU..."}
          </span>
          {gpuTempC != null && (
            <span className={`shrink-0 text-[11px] font-medium tabular-nums ${tempColorClass}`}>
              {gpuTempC}°C
            </span>
          )}
        </div>

        <div>
          <div className="h-1.5 w-full overflow-hidden rounded-full bg-surface-elevated">
            <div
              className="h-full rounded-full bg-primary-container transition-all duration-300"
              style={{ width: `${usedPct}%` }}
            />
          </div>
          <div className="mt-1 flex items-center justify-between text-[11px] text-text-muted tabular-nums">
            <span>
              VRAM {(vramUsedMb / 1024).toFixed(1)} / {(totalMb / 1024).toFixed(1)} GB
            </span>
            <span>{usedPct}%</span>
          </div>
        </div>

        {totalRam > 0 && (
          <div>
            <div className="h-1.5 w-full overflow-hidden rounded-full bg-surface-elevated">
              <div
                className="h-full rounded-full bg-tag-organize transition-all duration-300"
                style={{ width: `${ramPct}%` }}
              />
            </div>
            <div className="mt-1 flex items-center justify-between text-[11px] text-text-muted tabular-nums">
              <span>
                RAM {ramUsedGb.toFixed(1)} / {totalRam.toFixed(1)} GB
              </span>
              <span>{ramPct}%</span>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

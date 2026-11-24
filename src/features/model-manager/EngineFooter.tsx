import { Database, Power, Shield, Terminal } from "lucide-react";
import { formatVramGb } from "../../lib/ipc";
import { useEngineStore } from "../../stores/useEngineStore";

export function EngineFooter() {
  const telemetry = useEngineStore((s) => s.telemetry);
  const vramUsedMb = useEngineStore((s) => s.vramUsedMb);
  const vramTotalMb = useEngineStore((s) => s.vramTotalMb);
  const unloading = useEngineStore((s) => s.unloading);
  const unloadModel = useEngineStore((s) => s.unloadModel);

  const statusText = (() => {
    switch (telemetry?.status) {
      case "online":
        return `llama-server Online (PID ${telemetry.pid ?? "--"}, Port ${telemetry.port ?? "--"}, ${telemetry.backend.toUpperCase()})`;
      case "loading":
        return "llama-server Loading Model into Memory...";
      case "standby":
        return `llama-server Ready (Standby — Lazy Load • ${telemetry.active_model_name ?? ""})`;
      case "error":
        return "llama-server Error — check model file or GPU memory";
      default:
        return "llama-server Unconfigured (Download or import a model)";
    }
  })();

  return (
    <footer className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-border-default bg-surface-card px-4 py-3 text-xs text-text-secondary">
      <div className="flex flex-wrap items-center gap-4">
        <span className="inline-flex items-center gap-2 font-medium text-text-primary">
          <Terminal className="h-3.5 w-3.5 text-primary" />
          {statusText}
        </span>

        <span className="inline-flex items-center gap-1.5 text-tag-security">
          <Shield className="h-3.5 w-3.5" />
          0 Zombie Process (Win32 JobObject)
        </span>
      </div>

      <div className="flex flex-wrap items-center gap-4 tabular-nums">
        {telemetry?.status === "online" && (
          <button
            type="button"
            disabled={telemetry.busy_by !== "none" || unloading}
            onClick={() => void unloadModel()}
            title={
              telemetry.busy_by !== "none"
                ? "Wait for the current translation to finish before unloading"
                : "Unload active model from GPU/RAM back to Standby"
            }
            className="inline-flex items-center gap-1.5 rounded-lg border border-border-default bg-surface-elevated px-2.5 py-1 text-[11px] font-medium text-text-secondary hover:bg-surface-hover hover:text-text-primary disabled:opacity-40 cursor-pointer"
          >
            <Power className="h-3 w-3 text-tag-optimize" />
            {unloading ? "Unloading..." : "Unload Model"}
          </button>
        )}

        <span>
          Allocated VRAM: {formatVramGb(vramUsedMb)} / {formatVramGb(vramTotalMb)}
        </span>
        <span className="inline-flex items-center gap-1.5 text-status-success">
          <Database className="h-3.5 w-3.5" />
          {telemetry?.db_wal_synced !== false
            ? "SQLite Checkpoint (WAL Synced — osmium.db)"
            : "SQLite Checkpoint (Busy)"}
        </span>
      </div>
    </footer>
  );
}

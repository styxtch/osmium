import type { ReactNode } from "react";
import { AlertTriangle, X } from "lucide-react";
import { useEngineStore } from "../../stores/useEngineStore";
import { useModelsStore } from "../../stores/useModelsStore";
import { useNavigationStore } from "../../stores/useNavigationStore";
import { Sidebar } from "./Sidebar";

interface AppShellProps {
  children: ReactNode;
}

export function AppShell({ children }: AppShellProps) {
  const engineAlert = useEngineStore((s) => s.engineAlert);
  const dismissAlert = useEngineStore((s) => s.dismissAlert);
  const installed = useModelsStore((s) => s.installed);
  const activateModel = useModelsStore((s) => s.activateModel);
  const setActivePage = useNavigationStore((s) => s.setActivePage);

  const qwen4bInstalled = installed.find((m) => m.id === "qwen3-4b-q4km" && m.file_exists);

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-bg-canvas text-text-primary">
      <Sidebar />

      <main className="flex flex-1 flex-col min-w-0 min-h-0 overflow-y-auto bg-bg-canvas">
        {engineAlert && (
          <div
            role="alert"
            className="mx-5 lg:mx-6 mt-4 flex flex-wrap items-center justify-between gap-3 rounded-xl border border-tag-optimize/40 bg-tag-optimize/10 px-4 py-3 text-sm"
          >
            <div className="flex items-center gap-2.5 min-w-0">
              <AlertTriangle className="h-4 w-4 shrink-0 text-tag-optimize" />
              <span className="text-text-primary">{engineAlert.message}</span>
            </div>

            <div className="flex items-center gap-2 shrink-0">
              {engineAlert.code === "OUT_OF_MEMORY" && qwen4bInstalled && (
                <button
                  type="button"
                  onClick={() => {
                    void activateModel(qwen4bInstalled.id);
                    dismissAlert();
                  }}
                  className="rounded-lg bg-primary-container px-3 py-1.5 text-xs font-semibold text-on-primary hover:bg-primary-container/90 cursor-pointer"
                >
                  Switch to Qwen 3 4B
                </button>
              )}
              {(engineAlert.code === "BYOM_FILE_MISSING" ||
                engineAlert.code === "OUT_OF_MEMORY") && (
                <button
                  type="button"
                  onClick={() => {
                    setActivePage("model-manager");
                    dismissAlert();
                  }}
                  className="rounded-lg border border-border-default bg-surface-elevated px-3 py-1.5 text-xs font-medium text-text-primary hover:bg-surface-hover cursor-pointer"
                >
                  Open Model Manager
                </button>
              )}
              <button
                type="button"
                onClick={dismissAlert}
                aria-label="Dismiss alert"
                className="rounded-lg p-1 text-text-secondary hover:bg-surface-hover hover:text-text-primary cursor-pointer"
              >
                <X className="h-4 w-4" />
              </button>
            </div>
          </div>
        )}

        <div className="flex flex-1 flex-col min-h-0 p-5 lg:p-6">{children}</div>
      </main>
    </div>
  );
}

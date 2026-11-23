import {
  BookOpen,
  Captions,
  Cpu,
  Languages,
  Lock,
  Settings,
} from "lucide-react";
import { useEngineStore } from "../../stores/useEngineStore";
import { useModelsStore } from "../../stores/useModelsStore";
import { useNavigationStore, type PageId } from "../../stores/useNavigationStore";
import { ActiveGpuCard } from "./ActiveGpuCard";

interface NavItem {
  id: PageId;
  label: string;
  icon: typeof Languages;
}

const NAV_ITEMS: NavItem[] = [
  { id: "text-translator", label: "Text Translator", icon: Languages },
  { id: "subtitle-translator", label: "Subtitle Translator", icon: Captions },
  { id: "model-manager", label: "Model Manager", icon: Cpu },
  { id: "glossary-history", label: "Glossary & History", icon: BookOpen },
  { id: "settings", label: "Settings", icon: Settings },
];

export function Sidebar() {
  const activePage = useNavigationStore((s) => s.activePage);
  const setActivePage = useNavigationStore((s) => s.setActivePage);
  const installed = useModelsStore((s) => s.installed);
  const telemetry = useEngineStore((s) => s.telemetry);

  const hasReadyActiveModel = installed.some((m) => m.is_active && m.file_exists);
  const subtitleBusy = telemetry?.busy_by === "subtitle";

  return (
    <aside className="flex w-16 xl:w-64 shrink-0 flex-col justify-between border-r border-border-subtle bg-bg-sidebar p-2.5 xl:p-3.5 transition-all">
      <div>
        {/* Brand Header */}
        <div className="flex items-center justify-center xl:justify-start gap-2.5 px-1.5 py-2 mb-4">
          <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-primary-container text-on-primary">
            <Languages className="h-4 w-4" />
          </div>
          <div className="hidden xl:block min-w-0">
            <div className="text-sm font-semibold leading-5 text-text-primary">
              Osmium
            </div>
            <div className="flex items-center gap-1.5 text-[11px] font-medium leading-3.5 text-primary">
              <span className="h-1.5 w-1.5 rounded-full bg-primary" />
              100% LOCAL
            </div>
          </div>
        </div>

        {/* Navigation Links */}
        <nav aria-label="Main navigation" className="space-y-1">
          {NAV_ITEMS.map((item) => {
            const Icon = item.icon;
            const isActive = activePage === item.id;
            const isTextLocked = item.id === "text-translator" && subtitleBusy;
            const isSubtitleRunning = item.id === "subtitle-translator" && subtitleBusy;
            const isModelMgr = item.id === "model-manager";

            return (
              <button
                key={item.id}
                type="button"
                onClick={() => setActivePage(item.id)}
                title={item.label}
                aria-current={isActive ? "page" : undefined}
                className={`group flex w-full items-center justify-center xl:justify-between gap-2 rounded-lg px-2.5 py-2 text-sm font-medium transition-colors cursor-pointer ${
                  isActive
                    ? "bg-surface-elevated text-text-primary border border-border-default"
                    : "text-text-secondary hover:bg-surface-hover/60 hover:text-text-primary border border-transparent"
                }`}
              >
                <span className="flex items-center gap-2.5 min-w-0">
                  <Icon
                    className={`h-[18px] w-[18px] shrink-0 ${
                      isActive ? "text-primary" : "text-text-secondary group-hover:text-text-primary"
                    }`}
                  />
                  <span className="hidden xl:inline whitespace-nowrap">{item.label}</span>
                </span>

                <span className="hidden xl:inline-flex items-center shrink-0">
                  {isTextLocked && (
                    <Lock className="h-3.5 w-3.5 text-tag-optimize" aria-label="Locked by active subtitle batch" />
                  )}
                  {isSubtitleRunning && (
                    <span className="h-2 w-2 rounded-full bg-tag-optimize animate-pulse" />
                  )}
                  {isModelMgr && (
                    <span
                      className={`rounded-full px-2 py-0.5 text-[10px] font-semibold tracking-wide ${
                        hasReadyActiveModel
                          ? "bg-primary-container/15 text-primary border border-primary-container/30"
                          : "bg-tag-optimize/15 text-tag-optimize border border-tag-optimize/30"
                      }`}
                    >
                      {hasReadyActiveModel ? "READY" : "SETUP"}
                    </span>
                  )}
                </span>
              </button>
            );
          })}
        </nav>
      </div>

      {/* Bottom Active GPU Card */}
      <div className="pt-3 border-t border-border-subtle">
        <ActiveGpuCard />
      </div>
    </aside>
  );
}

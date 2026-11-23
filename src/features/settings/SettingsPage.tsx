import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  AlertCircle,
  CheckCircle2,
  ChevronDown,
  FolderOpen,
  Lock,
  Save,
  Sliders,
} from "lucide-react";
import type { AppSettings } from "../../lib/ipc";
import { useEngineStore } from "../../stores/useEngineStore";
import { useSettingsStore } from "../../stores/useSettingsStore";

export function SettingsPage() {
  const settings = useSettingsStore((s) => s.settings);
  const saving = useSettingsStore((s) => s.saving);
  const error = useSettingsStore((s) => s.error);
  const saveSettings = useSettingsStore((s) => s.saveSettings);
  const telemetry = useEngineStore((s) => s.telemetry);

  const [draft, setDraft] = useState<AppSettings | null>(settings);
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [savedNotice, setSavedNotice] = useState(false);

  useEffect(() => {
    if (settings) {
      setDraft(settings);
    }
  }, [settings]);

  if (!draft) {
    return (
      <div className="w-full rounded-xl border border-border-default bg-surface-card p-6 text-sm text-text-secondary">
        Loading settings...
      </div>
    );
  }

  const subtitleBusy = telemetry?.busy_by === "subtitle";

  const pickDirectory = async (field: "default_model_dir" | "default_export_dir") => {
    const selected = await open({
      title:
        field === "default_model_dir"
          ? "Select Default AI Model Folder"
          : "Select Default Subtitle Export Folder",
      directory: true,
      multiple: false,
      defaultPath: draft[field] || undefined,
    });
    if (typeof selected === "string" && selected.length > 0) {
      setDraft({ ...draft, [field]: selected });
      setSavedNotice(false);
    }
  };

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    const ok = await saveSettings(draft);
    if (ok) {
      setSavedNotice(true);
      setTimeout(() => setSavedNotice(false), 2500);
    }
  };

  return (
    <form onSubmit={(e) => void handleSave(e)} className="w-full space-y-5">
      <header className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold leading-8 tracking-tight text-text-primary">
            Settings
          </h1>
          <p className="mt-1 text-sm text-text-secondary">
            Customize display mode, default storage directories, and advanced local engine parameters.
          </p>
        </div>

        <div className="flex items-center gap-3">
          {savedNotice && (
            <span className="inline-flex items-center gap-1.5 text-xs font-medium text-primary">
              <CheckCircle2 className="h-4 w-4" />
              Settings saved
            </span>
          )}
          <button
            type="submit"
            disabled={saving}
            className="inline-flex items-center gap-2 rounded-lg bg-primary-container px-4 py-2 text-xs font-semibold text-on-primary hover:bg-primary-container/90 disabled:opacity-50 cursor-pointer"
          >
            <Save className="h-3.5 w-3.5" />
            {saving ? "Saving..." : "Save Settings"}
          </button>
        </div>
      </header>

      {error && (
        <div
          role="alert"
          className="flex items-center gap-2 rounded-xl border border-status-danger/40 bg-status-danger/10 px-4 py-3 text-sm text-status-danger"
        >
          <AlertCircle className="h-4 w-4 shrink-0" />
          <span>{error.message}</span>
        </div>
      )}

      {/* Display Mode */}
      <section className="rounded-xl border border-border-default bg-surface-card p-5 space-y-3">
        <h2 className="text-base font-semibold text-text-primary">Display Mode</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
          <label
            className={`flex items-start gap-3 rounded-lg border p-3.5 cursor-pointer ${
              draft.display_mode === "standard"
                ? "border-primary-container/60 bg-surface-elevated"
                : "border-border-default bg-surface-container"
            }`}
          >
            <input
              type="radio"
              name="display_mode"
              checked={draft.display_mode === "standard"}
              onChange={() => {
                setDraft({ ...draft, display_mode: "standard" });
                setSavedNotice(false);
              }}
              className="mt-0.5 accent-[#10B981]"
            />
            <div>
              <div className="text-sm font-medium text-text-primary">
                Standard (With Telemetry)
              </div>
              <div className="mt-1 text-xs text-text-secondary">
                Displays GPU VRAM, Token Delta, and active model telemetry strip below Text Translator.
              </div>
            </div>
          </label>

          <label
            className={`flex items-start gap-3 rounded-lg border p-3.5 cursor-pointer ${
              draft.display_mode === "compact"
                ? "border-primary-container/60 bg-surface-elevated"
                : "border-border-default bg-surface-container"
            }`}
          >
            <input
              type="radio"
              name="display_mode"
              checked={draft.display_mode === "compact"}
              onChange={() => {
                setDraft({ ...draft, display_mode: "compact" });
                setSavedNotice(false);
              }}
              className="mt-0.5 accent-[#10B981]"
            />
            <div>
              <div className="text-sm font-medium text-text-primary">
                Compact Mode (Minimalist)
              </div>
              <div className="mt-1 text-xs text-text-secondary">
                Hides the bottom technical telemetry strip for a clean, distraction-free workspace.
              </div>
            </div>
          </label>
        </div>
      </section>

      {/* Default Directories */}
      <section className="rounded-xl border border-border-default bg-surface-card p-5 space-y-4">
        <h2 className="text-base font-semibold text-text-primary">
          Default Storage Locations
        </h2>

        <div className="space-y-3">
          <div>
            <label className="block text-xs font-medium text-text-secondary mb-1.5">
              Default AI Model Folder
            </label>
            <div className="flex gap-2">
              <input
                type="text"
                value={draft.default_model_dir}
                onChange={(e) => {
                  setDraft({ ...draft, default_model_dir: e.target.value });
                  setSavedNotice(false);
                }}
                className="flex-1 rounded-lg border border-border-default bg-surface-elevated px-3 py-2 text-xs text-text-primary focus:border-primary-container focus:outline-none"
              />
              <button
                type="button"
                onClick={() => void pickDirectory("default_model_dir")}
                className="inline-flex items-center gap-1.5 rounded-lg border border-border-default bg-surface-elevated px-3 py-2 text-xs font-medium text-text-primary hover:bg-surface-hover cursor-pointer"
              >
                <FolderOpen className="h-3.5 w-3.5" />
                Change Folder
              </button>
            </div>
          </div>

          <div>
            <label className="block text-xs font-medium text-text-secondary mb-1.5">
              Default Subtitle Export Folder (Save As Initial Path)
            </label>
            <div className="flex gap-2">
              <input
                type="text"
                value={draft.default_export_dir}
                onChange={(e) => {
                  setDraft({ ...draft, default_export_dir: e.target.value });
                  setSavedNotice(false);
                }}
                className="flex-1 rounded-lg border border-border-default bg-surface-elevated px-3 py-2 text-xs text-text-primary focus:border-primary-container focus:outline-none"
              />
              <button
                type="button"
                onClick={() => void pickDirectory("default_export_dir")}
                className="inline-flex items-center gap-1.5 rounded-lg border border-border-default bg-surface-elevated px-3 py-2 text-xs font-medium text-text-primary hover:bg-surface-hover cursor-pointer"
              >
                <FolderOpen className="h-3.5 w-3.5" />
                Change Folder
              </button>
            </div>
          </div>
        </div>
      </section>

      {/* Collapsible Advanced Accordion */}
      <section className="rounded-xl border border-border-default bg-surface-card overflow-hidden">
        <button
          type="button"
          onClick={() => setAdvancedOpen((v) => !v)}
          className="flex w-full items-center justify-between gap-3 px-5 py-4 text-left hover:bg-surface-hover/40 cursor-pointer"
        >
          <div className="flex items-center gap-2.5">
            <Sliders className="h-4 w-4 text-tag-optimize" />
            <div>
              <div className="text-sm font-semibold text-text-primary">
                Advanced Settings (Sidecar Engine)
              </div>
              <div className="text-xs text-text-muted">
                Vulkan/CPU backend, manual context window size, and GPU offload layers
              </div>
            </div>
          </div>
          <ChevronDown
            className={`h-4 w-4 text-text-secondary transition-transform ${
              advancedOpen ? "rotate-180" : ""
            }`}
          />
        </button>

        {advancedOpen && (
          <div className="border-t border-border-subtle p-5 space-y-4">
            {subtitleBusy && (
              <div className="flex items-center gap-2 rounded-lg border border-tag-optimize/40 bg-tag-optimize/10 px-3 py-2 text-xs text-tag-optimize">
                <Lock className="h-3.5 w-3.5 shrink-0" />
                <span>
                  Advanced settings are locked while a Subtitle Translator batch is running.
                </span>
              </div>
            )}

            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <div>
                <label className="block text-xs font-medium text-text-secondary mb-1.5">
                  Compute Backend
                </label>
                <select
                  disabled={subtitleBusy}
                  value={draft.backend_override}
                  onChange={(e) =>
                    setDraft({
                      ...draft,
                      backend_override: e.target.value as AppSettings["backend_override"],
                    })
                  }
                  className="w-full rounded-lg border border-border-default bg-surface-elevated px-3 py-2 text-xs text-text-primary focus:border-primary-container focus:outline-none disabled:opacity-50"
                >
                  <option value="auto">Automatic (Vulkan if supported)</option>
                  <option value="vulkan">Vulkan Native</option>
                  <option value="cpu">Force CPU Mode</option>
                </select>
              </div>

              <div>
                <label className="block text-xs font-medium text-text-secondary mb-1.5">
                  Context Size (`--ctx-size`)
                </label>
                <select
                  disabled={subtitleBusy}
                  value={draft.ctx_size_override}
                  onChange={(e) =>
                    setDraft({
                      ...draft,
                      ctx_size_override: e.target.value as AppSettings["ctx_size_override"],
                    })
                  }
                  className="w-full rounded-lg border border-border-default bg-surface-elevated px-3 py-2 text-xs text-text-primary focus:border-primary-container focus:outline-none disabled:opacity-50"
                >
                  <option value="auto">Automatic (Based on VRAM)</option>
                  <option value="2048">2048 tokens</option>
                  <option value="4096">4096 tokens</option>
                  <option value="8192">8192 tokens</option>
                </select>
              </div>

              <div>
                <label className="block text-xs font-medium text-text-secondary mb-1.5">
                  GPU Layers (`--n-gpu-layers`)
                </label>
                <input
                  type="text"
                  disabled={subtitleBusy}
                  value={draft.gpu_layers_override}
                  onChange={(e) =>
                    setDraft({ ...draft, gpu_layers_override: e.target.value })
                  }
                  placeholder="auto or integer (e.g. 99)"
                  className="w-full rounded-lg border border-border-default bg-surface-elevated px-3 py-2 text-xs text-text-primary focus:border-primary-container focus:outline-none disabled:opacity-50"
                />
              </div>
            </div>

            <p className="text-xs text-text-muted">
              Changing advanced settings resets the llama-server sidecar to Standby and reloads it on the next translation request.
            </p>
          </div>
        )}
      </section>
    </form>
  );
}

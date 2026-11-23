import { useEffect } from "react";
import { AppShell } from "./components/layout/AppShell";
import { GlossaryHistoryPage } from "./features/glossary-history/GlossaryHistoryPage";
import { ModelManagerPage } from "./features/model-manager/ModelManagerPage";
import { SettingsPage } from "./features/settings/SettingsPage";
import { SubtitleTranslatorPage } from "./features/subtitle-translator/SubtitleTranslatorPage";
import { TextTranslatorPage } from "./features/text-translator/TextTranslatorPage";
import { useEngineStore } from "./stores/useEngineStore";
import { useModelsStore } from "./stores/useModelsStore";
import { useNavigationStore } from "./stores/useNavigationStore";
import { useSettingsStore } from "./stores/useSettingsStore";
import { useSubtitleStore } from "./stores/useSubtitleStore";

export default function App() {
  const activePage = useNavigationStore((s) => s.activePage);
  const initializeDefaultRoute = useNavigationStore(
    (s) => s.initializeDefaultRoute
  );

  const fetchSettings = useSettingsStore((s) => s.fetchSettings);
  const refreshModels = useModelsStore((s) => s.refreshModels);
  const scanHardware = useEngineStore((s) => s.scanHardware);
  const refreshTelemetry = useEngineStore((s) => s.refreshTelemetry);
  const initEngineListeners = useEngineStore((s) => s.initEngineListeners);
  const initSubtitleListeners = useSubtitleStore(
    (s) => s.initSubtitleListeners
  );

  useEffect(() => {
    let cleanupEngineListeners: (() => void) | undefined;
    let cleanupSubtitleListeners: (() => void) | undefined;
    let mounted = true;

    async function bootstrap() {
      const [unsubEngine, unsubSubtitle] = await Promise.all([
        initEngineListeners(),
        initSubtitleListeners(),
      ]);
      cleanupEngineListeners = unsubEngine;
      cleanupSubtitleListeners = unsubSubtitle;

      const [, models] = await Promise.all([
        fetchSettings(),
        refreshModels(),
        scanHardware(),
        refreshTelemetry(),
      ]);
      if (mounted) {
        initializeDefaultRoute(models.some((m) => m.is_active && m.file_exists));
      }
    }

    void bootstrap();

    return () => {
      mounted = false;
      cleanupEngineListeners?.();
      cleanupSubtitleListeners?.();
    };
  }, [
    fetchSettings,
    initEngineListeners,
    initSubtitleListeners,
    initializeDefaultRoute,
    refreshModels,
    refreshTelemetry,
    scanHardware,
  ]);

  return (
    <AppShell>
      {activePage === "model-manager" && <ModelManagerPage />}
      {activePage === "text-translator" && <TextTranslatorPage />}
      {activePage === "subtitle-translator" && <SubtitleTranslatorPage />}
      {activePage === "glossary-history" && <GlossaryHistoryPage />}
      {activePage === "settings" && <SettingsPage />}
    </AppShell>
  );
}

import { create } from "zustand";

export type PageId =
  | "text-translator"
  | "subtitle-translator"
  | "model-manager"
  | "glossary-history"
  | "settings";

export type GlossaryHistoryTab = "glossary" | "history";

interface NavigationState {
  activePage: PageId;
  glossaryHistoryTab: GlossaryHistoryTab;
  hasInitializedRoute: boolean;
  setActivePage: (page: PageId) => void;
  openHistoryPage: () => void;
  setGlossaryHistoryTab: (tab: GlossaryHistoryTab) => void;
  initializeDefaultRoute: (hasInstalledModel: boolean) => void;
}

export const useNavigationStore = create<NavigationState>((set, get) => ({
  activePage: "model-manager",
  glossaryHistoryTab: "glossary",
  hasInitializedRoute: false,
  setActivePage: (page) => set({ activePage: page }),
  openHistoryPage: () =>
    set({ activePage: "glossary-history", glossaryHistoryTab: "history" }),
  setGlossaryHistoryTab: (tab) => set({ glossaryHistoryTab: tab }),
  initializeDefaultRoute: (hasInstalledModel) => {
    if (get().hasInitializedRoute) return;
    set({
      activePage: hasInstalledModel ? "text-translator" : "model-manager",
      hasInitializedRoute: true,
    });
  },
}));

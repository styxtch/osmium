import { create } from "zustand";
import {
  ipc,
  parseAppError,
  type AppErrorPayload,
  type Glossary,
  type GlossaryEntry,
  type TextHistoryItem,
} from "../lib/ipc";

interface GlossaryHistoryState {
  glossaries: Glossary[];
  selectedGlossaryId: string | null;
  entries: GlossaryEntry[];
  history: TextHistoryItem[];
  historyQuery: string;
  loading: boolean;
  notice: string | null;
  error: AppErrorPayload | null;

  fetchGlossaries: () => Promise<void>;
  selectGlossary: (id: string | null) => Promise<void>;
  createGlossary: (name: string) => Promise<void>;
  renameGlossary: (id: string, name: string) => Promise<void>;
  deleteGlossary: (id: string) => Promise<void>;
  upsertEntry: (payload: {
    id?: string;
    glossaryId: string;
    sourceTerm: string;
    targetTerm: string;
    caseSensitive: boolean;
  }) => Promise<boolean>;
  deleteEntry: (id: string, glossaryId: string) => Promise<void>;
  importGlossary: (
    filePath: string,
    targetGlossaryId?: string | null
  ) => Promise<void>;
  exportGlossary: (
    glossaryId: string,
    destPath: string,
    format?: "json" | "csv"
  ) => Promise<void>;

  fetchHistory: (query?: string) => Promise<void>;
  setHistoryQuery: (query: string) => void;
  deleteHistoryItem: (id: string) => Promise<void>;
  clearNotice: () => void;
  clearError: () => void;
}

export const useGlossaryHistoryStore = create<GlossaryHistoryState>(
  (set, get) => ({
    glossaries: [],
    selectedGlossaryId: null,
    entries: [],
    history: [],
    historyQuery: "",
    loading: false,
    notice: null,
    error: null,

    fetchGlossaries: async () => {
      set({ loading: true });
      try {
        const glossaries = await ipc.listGlossaries();
        const current = get().selectedGlossaryId;
        const nextSelected =
          current && glossaries.some((g) => g.id === current)
            ? current
            : (glossaries[0]?.id ?? null);
        set({ glossaries, selectedGlossaryId: nextSelected, loading: false });
        if (nextSelected) {
          const entries = await ipc.listGlossaryEntries(nextSelected);
          set({ entries });
        } else {
          set({ entries: [] });
        }
      } catch (err) {
        set({ error: parseAppError(err), loading: false });
      }
    },

    selectGlossary: async (id) => {
      set({ selectedGlossaryId: id, error: null });
      if (!id) {
        set({ entries: [] });
        return;
      }
      try {
        const entries = await ipc.listGlossaryEntries(id);
        set({ entries });
      } catch (err) {
        set({ error: parseAppError(err) });
      }
    },

    createGlossary: async (name) => {
      set({ error: null });
      try {
        const created = await ipc.createGlossary(name);
        await get().fetchGlossaries();
        await get().selectGlossary(created.id);
      } catch (err) {
        set({ error: parseAppError(err) });
      }
    },

    renameGlossary: async (id, name) => {
      set({ error: null });
      try {
        await ipc.updateGlossary(id, name);
        await get().fetchGlossaries();
      } catch (err) {
        set({ error: parseAppError(err) });
      }
    },

    deleteGlossary: async (id) => {
      set({ error: null });
      try {
        await ipc.deleteGlossary(id);
        await get().fetchGlossaries();
      } catch (err) {
        set({ error: parseAppError(err) });
      }
    },

    upsertEntry: async (payload) => {
      set({ error: null });
      try {
        await ipc.upsertGlossaryEntry(payload);
        const entries = await ipc.listGlossaryEntries(payload.glossaryId);
        const glossaries = await ipc.listGlossaries();
        set({ entries, glossaries });
        return true;
      } catch (err) {
        set({ error: parseAppError(err) });
        return false;
      }
    },

    deleteEntry: async (id, glossaryId) => {
      set({ error: null });
      try {
        await ipc.deleteGlossaryEntry(id, glossaryId);
        const entries = await ipc.listGlossaryEntries(glossaryId);
        const glossaries = await ipc.listGlossaries();
        set({ entries, glossaries });
      } catch (err) {
        set({ error: parseAppError(err) });
      }
    },

    importGlossary: async (filePath, targetGlossaryId) => {
      set({ error: null, notice: null });
      try {
        const result = await ipc.importGlossary({
          filePath,
          targetGlossaryId: targetGlossaryId ?? undefined,
        });
        await get().fetchGlossaries();
        await get().selectGlossary(result.glossary.id);
        const skippedNote =
          result.skipped_count > 0 ? ` (${result.skipped_count} skipped)` : "";
        set({
          notice: `Imported ${result.imported_count} terms${skippedNote} into "${result.glossary.name}".`,
        });
      } catch (err) {
        set({ error: parseAppError(err) });
      }
    },

    exportGlossary: async (glossaryId, destPath, format) => {
      set({ error: null, notice: null });
      try {
        const savedPath = await ipc.exportGlossary(
          glossaryId,
          destPath,
          format
        );
        set({ notice: `Exported glossary to ${savedPath}` });
      } catch (err) {
        set({ error: parseAppError(err) });
      }
    },

    fetchHistory: async (query) => {
      const q = query ?? get().historyQuery;
      try {
        const history = await ipc.listTextHistory(q || undefined, 100);
        set({ history });
      } catch (err) {
        set({ error: parseAppError(err) });
      }
    },

    setHistoryQuery: (historyQuery) => {
      set({ historyQuery });
      void get().fetchHistory(historyQuery);
    },

    deleteHistoryItem: async (id) => {
      try {
        await ipc.deleteTextHistory(id);
        set((state) => ({
          history: state.history.filter((h) => h.id !== id),
        }));
      } catch (err) {
        set({ error: parseAppError(err) });
      }
    },

    clearNotice: () => set({ notice: null }),
    clearError: () => set({ error: null }),
  })
);

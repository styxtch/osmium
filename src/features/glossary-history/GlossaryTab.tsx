import { useState } from "react";
import { Check, Edit2, Plus, Search, Trash2, X } from "lucide-react";
import type { GlossaryEntry } from "../../lib/ipc";
import { useGlossaryHistoryStore } from "../../stores/useGlossaryHistoryStore";

export function GlossaryTab() {
  const glossaries = useGlossaryHistoryStore((s) => s.glossaries);
  const selectedGlossaryId = useGlossaryHistoryStore(
    (s) => s.selectedGlossaryId
  );
  const entries = useGlossaryHistoryStore((s) => s.entries);

  const selectGlossary = useGlossaryHistoryStore((s) => s.selectGlossary);
  const createGlossary = useGlossaryHistoryStore((s) => s.createGlossary);
  const renameGlossary = useGlossaryHistoryStore((s) => s.renameGlossary);
  const deleteGlossary = useGlossaryHistoryStore((s) => s.deleteGlossary);
  const upsertEntry = useGlossaryHistoryStore((s) => s.upsertEntry);
  const deleteEntry = useGlossaryHistoryStore((s) => s.deleteEntry);

  const [newCollectionName, setNewCollectionName] = useState("");
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");

  const [entrySearch, setEntrySearch] = useState("");
  const [editingEntryId, setEditingEntryId] = useState<string | null>(null);
  const [sourceTerm, setSourceTerm] = useState("");
  const [targetTerm, setTargetTerm] = useState("");
  const [caseSensitive, setCaseSensitive] = useState(false);

  const handleCreateCollection = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newCollectionName.trim()) return;
    await createGlossary(newCollectionName.trim());
    setNewCollectionName("");
  };

  const handleSaveEntry = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!selectedGlossaryId || !sourceTerm.trim() || !targetTerm.trim()) return;
    const ok = await upsertEntry({
      id: editingEntryId ?? undefined,
      glossaryId: selectedGlossaryId,
      sourceTerm: sourceTerm.trim(),
      targetTerm: targetTerm.trim(),
      caseSensitive,
    });
    if (ok) {
      setEditingEntryId(null);
      setSourceTerm("");
      setTargetTerm("");
      setCaseSensitive(false);
    }
  };

  const startEditEntry = (entry: GlossaryEntry) => {
    setEditingEntryId(entry.id);
    setSourceTerm(entry.source_term);
    setTargetTerm(entry.target_term);
    setCaseSensitive(entry.case_sensitive);
  };

  const filteredEntries = entries.filter((item) => {
    const q = entrySearch.trim().toLowerCase();
    if (!q) return true;
    return (
      item.source_term.toLowerCase().includes(q) ||
      item.target_term.toLowerCase().includes(q)
    );
  });

  return (
    <div className="grid grid-cols-1 lg:grid-cols-12 gap-4">
      {/* Left: Glossary Collections */}
      <section className="lg:col-span-4 rounded-xl border border-border-default bg-surface-card p-4 space-y-4">
        <div>
          <h2 className="text-sm font-semibold text-text-primary">
            Subtitle Glossary Collections
          </h2>
          <p className="mt-1 text-xs text-text-muted">
            Linked to Subtitle Translator so character names, titles, and lore terms remain consistent across episodes.
          </p>
        </div>

        <form onSubmit={(e) => void handleCreateCollection(e)} className="flex gap-2">
          <input
            type="text"
            value={newCollectionName}
            onChange={(e) => setNewCollectionName(e.target.value)}
            placeholder="New collection name..."
            className="flex-1 rounded-lg border border-border-default bg-surface-elevated px-3 py-1.5 text-xs text-text-primary placeholder:text-text-muted focus:border-primary-container focus:outline-none"
          />
          <button
            type="submit"
            disabled={!newCollectionName.trim()}
            className="inline-flex items-center gap-1 rounded-lg bg-primary-container px-3 py-1.5 text-xs font-semibold text-on-primary hover:bg-primary-container/90 disabled:opacity-40 cursor-pointer"
          >
            <Plus className="h-3.5 w-3.5" />
            Create
          </button>
        </form>

        {glossaries.length === 0 ? (
          <div className="rounded-lg border border-border-subtle bg-surface-container p-4 text-center text-xs text-text-muted">
            No glossary collections yet.
          </div>
        ) : (
          <ul className="space-y-1.5">
            {glossaries.map((g) => {
              const active = g.id === selectedGlossaryId;
              const isRenaming = renamingId === g.id;
              return (
                <li
                  key={g.id}
                  className={`rounded-lg border p-2.5 transition-colors ${
                    active
                      ? "border-primary-container/50 bg-surface-elevated"
                      : "border-border-subtle bg-surface-container hover:bg-surface-hover/50"
                  }`}
                >
                  {isRenaming ? (
                    <div className="flex items-center gap-1.5">
                      <input
                        type="text"
                        value={renameValue}
                        onChange={(e) => setRenameValue(e.target.value)}
                        className="flex-1 rounded border border-border-default bg-surface-card px-2 py-1 text-xs text-text-primary focus:border-primary-container focus:outline-none"
                      />
                      <button
                        type="button"
                        onClick={() => {
                          if (renameValue.trim()) {
                            void renameGlossary(g.id, renameValue.trim());
                          }
                          setRenamingId(null);
                        }}
                        aria-label="Save collection name"
                        className="rounded p-1 text-primary hover:bg-surface-hover cursor-pointer"
                      >
                        <Check className="h-3.5 w-3.5" />
                      </button>
                      <button
                        type="button"
                        onClick={() => setRenamingId(null)}
                        aria-label="Cancel rename"
                        className="rounded p-1 text-text-muted hover:bg-surface-hover cursor-pointer"
                      >
                        <X className="h-3.5 w-3.5" />
                      </button>
                    </div>
                  ) : (
                    <div className="flex items-center justify-between gap-2">
                      <button
                        type="button"
                        onClick={() => void selectGlossary(g.id)}
                        className="flex-1 text-left min-w-0 cursor-pointer"
                      >
                        <div className="truncate text-xs font-semibold text-text-primary">
                          {g.name}
                        </div>
                        <div className="mt-0.5 text-[11px] text-tag-organize tabular-nums">
                          {g.entry_count} Terms
                        </div>
                      </button>

                      <div className="flex items-center gap-1 shrink-0">
                        <button
                          type="button"
                          onClick={() => {
                            setRenamingId(g.id);
                            setRenameValue(g.name);
                          }}
                          aria-label={`Rename ${g.name}`}
                          className="rounded p-1 text-text-secondary hover:bg-surface-hover hover:text-text-primary cursor-pointer"
                        >
                          <Edit2 className="h-3.5 w-3.5" />
                        </button>
                        <button
                          type="button"
                          onClick={() => void deleteGlossary(g.id)}
                          aria-label={`Delete ${g.name}`}
                          className="rounded p-1 text-text-secondary hover:bg-surface-hover hover:text-status-danger cursor-pointer"
                        >
                          <Trash2 className="h-3.5 w-3.5" />
                        </button>
                      </div>
                    </div>
                  )}
                </li>
              );
            })}
          </ul>
        )}
      </section>

      {/* Right: Entries Editor & Table */}
      <section className="lg:col-span-8 rounded-xl border border-border-default bg-surface-card p-4 space-y-4">
        {!selectedGlossaryId ? (
          <div className="py-12 text-center text-sm text-text-muted">
            Select or create a glossary collection on the left to start adding terms.
          </div>
        ) : (
          <>
            <form
              onSubmit={(e) => void handleSaveEntry(e)}
              className="grid grid-cols-1 md:grid-cols-12 gap-2.5 rounded-lg border border-border-subtle bg-surface-container p-3"
            >
              <input
                type="text"
                value={sourceTerm}
                onChange={(e) => setSourceTerm(e.target.value)}
                placeholder="EN Source Term (e.g. Sandevistan)"
                className="md:col-span-4 rounded-lg border border-border-default bg-surface-elevated px-3 py-1.5 text-xs text-text-primary placeholder:text-text-muted focus:border-primary-container focus:outline-none"
              />
              <input
                type="text"
                value={targetTerm}
                onChange={(e) => setTargetTerm(e.target.value)}
                placeholder="ID Target Term (e.g. Sandevistan)"
                className="md:col-span-4 rounded-lg border border-border-default bg-surface-elevated px-3 py-1.5 text-xs text-text-primary placeholder:text-text-muted focus:border-primary-container focus:outline-none"
              />
              <label className="md:col-span-2 flex items-center gap-1.5 text-xs text-text-secondary cursor-pointer select-none">
                <input
                  type="checkbox"
                  checked={caseSensitive}
                  onChange={(e) => setCaseSensitive(e.target.checked)}
                  className="h-3.5 w-3.5 accent-[#10B981]"
                />
                Case-Sensitive
              </label>
              <div className="md:col-span-2 flex items-center justify-end gap-1.5">
                {editingEntryId && (
                  <button
                    type="button"
                    onClick={() => {
                      setEditingEntryId(null);
                      setSourceTerm("");
                      setTargetTerm("");
                      setCaseSensitive(false);
                    }}
                    className="rounded-lg border border-border-default bg-surface-elevated px-2.5 py-1.5 text-xs text-text-secondary cursor-pointer"
                  >
                    Cancel
                  </button>
                )}
                <button
                  type="submit"
                  disabled={!sourceTerm.trim() || !targetTerm.trim()}
                  className="rounded-lg bg-primary-container px-3 py-1.5 text-xs font-semibold text-on-primary hover:bg-primary-container/90 disabled:opacity-40 cursor-pointer"
                >
                  {editingEntryId ? "Save" : "Add"}
                </button>
              </div>
            </form>

            <div className="relative">
              <Search className="pointer-events-none absolute left-3 top-2.5 h-3.5 w-3.5 text-text-muted" />
              <input
                type="text"
                value={entrySearch}
                onChange={(e) => setEntrySearch(e.target.value)}
                placeholder="Search terms in collection..."
                className="w-full rounded-lg border border-border-default bg-surface-elevated pl-8 pr-3 py-1.5 text-xs text-text-primary placeholder:text-text-muted focus:border-primary-container focus:outline-none"
              />
            </div>

            {filteredEntries.length === 0 ? (
              <div className="py-8 text-center text-xs text-text-muted">
                No matching terms found in this collection.
              </div>
            ) : (
              <div className="overflow-x-auto rounded-lg border border-border-subtle">
                <table className="w-full text-left text-xs">
                  <thead className="bg-surface-container text-text-secondary">
                    <tr>
                      <th className="px-3 py-2 font-medium">EN Term</th>
                      <th className="px-3 py-2 font-medium">ID Target</th>
                      <th className="px-3 py-2 font-medium">Matching</th>
                      <th className="px-3 py-2 text-right font-medium">Actions</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-border-subtle">
                    {filteredEntries.map((entry) => (
                      <tr key={entry.id} className="hover:bg-surface-elevated/40">
                        <td className="px-3 py-2 font-medium text-tag-organize">
                          {entry.source_term}
                        </td>
                        <td className="px-3 py-2 text-text-primary">
                          {entry.target_term}
                        </td>
                        <td className="px-3 py-2 text-text-secondary">
                          {entry.case_sensitive ? "Exact Case" : "Case-Insensitive"}
                        </td>
                        <td className="px-3 py-2 text-right">
                          <div className="inline-flex items-center gap-1">
                            <button
                              type="button"
                              onClick={() => startEditEntry(entry)}
                              aria-label={`Edit ${entry.source_term}`}
                              className="rounded p-1 text-text-secondary hover:bg-surface-hover hover:text-text-primary cursor-pointer"
                            >
                              <Edit2 className="h-3.5 w-3.5" />
                            </button>
                            <button
                              type="button"
                              onClick={() =>
                                void deleteEntry(entry.id, entry.glossary_id)
                              }
                              aria-label={`Delete ${entry.source_term}`}
                              className="rounded p-1 text-text-secondary hover:bg-surface-hover hover:text-status-danger cursor-pointer"
                            >
                              <Trash2 className="h-3.5 w-3.5" />
                            </button>
                          </div>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </>
        )}
      </section>
    </div>
  );
}

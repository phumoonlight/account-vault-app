import { useCallback, useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { open, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { api, type Entry, type EntryInput, type EntrySummary } from "./api";
import { EntryList } from "./components/EntryList";
import { EntryView } from "./components/EntryView";
import { EntryForm } from "./components/EntryForm";
import { PinDialog } from "./components/PinDialog";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { collectTags, sameTag } from "./tags";
import "./App.css";

type Pane =
  | { kind: "empty" }
  | { kind: "view"; entry: Entry }
  | { kind: "edit"; entry: Entry }
  | { kind: "new" };

type Dialog =
  | { kind: "export" }
  | { kind: "import"; path: string }
  /** Window close was blocked because something copied is still on the clipboard. */
  | { kind: "close"; secret: boolean };

const BACKUP_FILTER = { name: "Homemade Account Vault backup", extensions: ["avbackup"] };
const plural = (n: number) => `${n} ${n === 1 ? "entry" : "entries"}`;

function matches(e: EntrySummary, query: string, tag: string | null) {
  if (tag && !e.tags.some((t) => sameTag(t, tag))) return false;
  const q = query.trim().toLowerCase();
  return !q || [e.title, e.username, e.url, ...e.tags].some((s) => s.toLowerCase().includes(q));
}

export default function App() {
  const [entries, setEntries] = useState<EntrySummary[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [query, setQuery] = useState("");
  const [activeTag, setActiveTag] = useState<string | null>(null);
  const [pane, setPane] = useState<Pane>({ kind: "empty" });
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [dialog, setDialog] = useState<Dialog | null>(null);
  const closeDialog = useCallback(() => setDialog(null), []);

  /** Runs a backend call, showing its error message in the banner if it fails. */
  const run = useCallback(async <T,>(action: () => Promise<T>): Promise<T | undefined> => {
    try {
      setError(null);
      return await action();
    } catch (e) {
      setError(String(e));
      return undefined;
    }
  }, []);

  const refresh = useCallback(async () => {
    const list = await run(api.listEntries);
    if (list) setEntries(list);
    setLoaded(true);
  }, [run]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  // The backend blocks the window close and asks us to confirm it.
  useEffect(() => {
    const unlisten = listen<{ secret: boolean }>("close-requested", (e) =>
      setDialog({ kind: "close", secret: e.payload.secret }),
    );
    return () => {
      unlisten.then((stop) => stop());
    };
  }, []);

  const tags = useMemo(() => collectTags(entries), [entries]);

  // Drop the tag filter once no entry has that tag any more.
  useEffect(() => {
    if (activeTag && !tags.some((t) => sameTag(t.tag, activeTag))) setActiveTag(null);
  }, [tags, activeTag]);

  const visible = useMemo(
    () =>
      entries
        .filter((e) => matches(e, query, activeTag))
        .sort(
          (a, b) =>
            Number(b.favorite) - Number(a.favorite) ||
            a.title.localeCompare(b.title, undefined, { sensitivity: "base" }),
        ),
    [entries, query, activeTag],
  );

  async function select(id: string) {
    const entry = await run(() => api.getEntry(id));
    if (entry) setPane({ kind: "view", entry });
  }

  async function save(input: EntryInput) {
    const saved = await run(() =>
      pane.kind === "edit" ? api.updateEntry(pane.entry.id, input) : api.addEntry(input),
    );
    if (saved) {
      setPane({ kind: "view", entry: saved });
      await refresh();
    }
  }

  async function remove(id: string) {
    const ok = await run(async () => {
      await api.deleteEntry(id);
      return true;
    });
    if (ok) {
      setPane({ kind: "empty" });
      await refresh();
    }
  }

  async function exportBackup(pin: string) {
    const path = await saveDialog({
      defaultPath: `account-vault-${new Date().toISOString().slice(0, 10)}.avbackup`,
      filters: [BACKUP_FILTER],
    });
    if (!path) return; // Save dialog cancelled; keep the PIN dialog open.
    const count = await api.exportBackup(path, pin);
    setDialog(null);
    setNotice(`Exported ${plural(count)} to ${path}`);
  }

  async function chooseImportFile() {
    const path = await run(() =>
      open({ multiple: false, directory: false, filters: [BACKUP_FILTER] }),
    );
    if (path) setDialog({ kind: "import", path });
  }

  async function importBackup(path: string, pin: string) {
    const summary = await api.importBackup(path, pin);
    setDialog(null);
    setNotice(
      `Imported ${plural(summary.added)}` +
        (summary.skipped ? `, skipped ${summary.skipped} already in the vault.` : "."),
    );
    await refresh();
  }

  const selectedId = pane.kind === "view" || pane.kind === "edit" ? pane.entry.id : null;

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="sidebar-top">
          <input
            type="search"
            className="search"
            placeholder="Search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <button className="primary" onClick={() => setPane({ kind: "new" })}>
            + New
          </button>
        </div>
        {tags.length > 0 && (
          <div className="tag-filter" role="toolbar" aria-label="Filter by tag">
            <button
              className={`tag-chip${activeTag === null ? " active" : ""}`}
              onClick={() => setActiveTag(null)}
            >
              All
            </button>
            {tags.map(({ tag, count }) => {
              const active = activeTag !== null && sameTag(activeTag, tag);
              return (
                <button
                  key={tag}
                  className={`tag-chip${active ? " active" : ""}`}
                  aria-pressed={active}
                  onClick={() => setActiveTag(active ? null : tag)}
                >
                  {tag} <span className="tag-count">{count}</span>
                </button>
              );
            })}
          </div>
        )}
        {loaded && <EntryList entries={visible} selectedId={selectedId} onSelect={select} />}
        <footer className="sidebar-footer">
          <span>
            {entries.length} {entries.length === 1 ? "entry" : "entries"}
          </span>
          <span className="footer-actions">
            <button className="small" onClick={chooseImportFile}>
              Import
            </button>
            <button className="small" onClick={() => setDialog({ kind: "export" })}>
              Export
            </button>
          </span>
        </footer>
      </aside>

      <main className="main">
        {error && (
          <div className="error" role="alert">
            <span>{error}</span>
            <button className="small" onClick={() => setError(null)}>
              Dismiss
            </button>
          </div>
        )}
        {notice && (
          <div className="success" role="status">
            <span>{notice}</span>
            <button className="small" onClick={() => setNotice(null)}>
              Dismiss
            </button>
          </div>
        )}

        {pane.kind === "view" && (
          <EntryView
            key={pane.entry.id}
            entry={pane.entry}
            onEdit={() => setPane({ kind: "edit", entry: pane.entry })}
            onDelete={() => remove(pane.entry.id)}
          />
        )}
        {(pane.kind === "edit" || pane.kind === "new") && (
          <EntryForm
            key={pane.kind === "edit" ? pane.entry.id : "new"}
            initial={pane.kind === "edit" ? pane.entry : undefined}
            onSave={save}
            allTags={tags.map((t) => t.tag)}
            onCancel={() =>
              setPane(pane.kind === "edit" ? { kind: "view", entry: pane.entry } : { kind: "empty" })
            }
          />
        )}
        {pane.kind === "empty" && (
          <div className="placeholder">
            <p>{entries.length ? "Select an entry" : "Your vault is empty"}</p>
            <button className="primary" onClick={() => setPane({ kind: "new" })}>
              Add an account
            </button>
          </div>
        )}
      </main>

      {dialog?.kind === "export" && (
        <PinDialog
          title="Export backup"
          description="Choose a 4–6 digit PIN to protect the backup. You'll need it to import the backup later; if you forget it, the backup can't be opened."
          submitLabel="Export…"
          busyLabel="Exporting…"
          confirm
          onSubmit={exportBackup}
          onCancel={closeDialog}
        />
      )}
      {dialog?.kind === "close" && (
        <ConfirmDialog
          title="Close Homemade Account Vault?"
          message={
            dialog.secret
              ? "A password you copied is still on the clipboard. Closing the app clears it, so you won't be able to paste it anymore."
              : "Something you copied from this app is still on the clipboard. Closing the app clears it, so you won't be able to paste it anymore."
          }
          cancelLabel="Keep open"
          confirmLabel="Clear clipboard & close"
          onConfirm={() => run(api.closeApp)}
          onCancel={() => {
            setDialog(null);
            run(api.cancelClose);
          }}
        />
      )}
      {dialog?.kind === "import" && (
        <PinDialog
          title="Import backup"
          description={`Enter the PIN for ${dialog.path.split(/[\\/]/).pop()}.`}
          submitLabel="Import"
          busyLabel="Importing…"
          onSubmit={(pin) => importBackup(dialog.path, pin)}
          onCancel={closeDialog}
        />
      )}
    </div>
  );
}

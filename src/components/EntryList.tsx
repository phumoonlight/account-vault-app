import { useRef, useState, type DragEvent } from "react";
import type { EntrySummary, Folder } from "../api";
import { FolderIcon } from "./FolderIcon";

/** Drag data type, so entries can't be dropped into text inputs or other apps. */
const ENTRY_MIME = "application/x-vault-entry";
/** Drop-target key for "no folder" (folder ids are UUIDs, never empty). */
const NO_FOLDER = "";

interface Props {
  /** Visible entries, already filtered and sorted. */
  entries: EntrySummary[];
  /** All folders, sorted by name. */
  folders: Folder[];
  /** A search or tag filter is active: hide folders without matches, show the rest open. */
  filtering: boolean;
  selectedId: string | null;
  collapsed: Set<string>;
  onSelect: (id: string) => void;
  onToggleFolder: (id: string) => void;
  onMove: (entryId: string, folderId: string | null) => void;
  onRenameFolder: (folder: Folder) => void;
  onDeleteFolder: (folder: Folder) => void;
}

export function EntryList({
  entries,
  folders,
  filtering,
  selectedId,
  collapsed,
  onSelect,
  onToggleFolder,
  onMove,
  onRenameFolder,
  onDeleteFolder,
}: Props) {
  const [drag, setDrag] = useState<{ id: string; folderId: string | null } | null>(null);
  const [over, setOver] = useState<string | null>(null);
  const dragTimer = useRef<number | undefined>(undefined);

  const folderIds = new Set(folders.map((f) => f.id));
  const groups = folders
    .map((folder) => ({ folder, items: entries.filter((e) => e.folderId === folder.id) }))
    .filter((g) => !filtering || g.items.length > 0);
  const loose = entries.filter((e) => !e.folderId || !folderIds.has(e.folderId));

  if (groups.length === 0 && loose.length === 0) {
    return <p className="list-empty">No entries</p>;
  }

  function endDrag() {
    // A drag that ends before its start timer fires must not leave `drag` set.
    window.clearTimeout(dragTimer.current);
    setDrag(null);
    setOver(null);
  }

  /** Makes an element a drop target for `folderId` (null = out of any folder). */
  function dropTarget(folderId: string | null) {
    return {
      onDragOver(e: DragEvent) {
        if (!drag) return; // Not one of our entries, e.g. a file.
        // Stop here so a folder's own area never falls through to "no folder".
        e.stopPropagation();
        if (drag.folderId === folderId) {
          setOver(null);
          return;
        }
        e.preventDefault();
        e.dataTransfer.dropEffect = "move";
        setOver(folderId ?? NO_FOLDER);
      },
      onDrop(e: DragEvent) {
        if (!drag) return;
        e.preventDefault();
        e.stopPropagation();
        const moved = drag;
        endDrag();
        if (moved.folderId !== folderId) onMove(moved.id, folderId);
      },
    };
  }

  const item = (e: EntrySummary) => (
    <li key={e.id}>
      <button
        className={`entry-item${e.id === selectedId ? " selected" : ""}${
          drag?.id === e.id ? " dragging" : ""
        }`}
        onClick={() => onSelect(e.id)}
        draggable
        onDragStart={(ev) => {
          ev.dataTransfer.setData(ENTRY_MIME, e.id);
          ev.dataTransfer.effectAllowed = "move";
          const folderId = e.folderId && folderIds.has(e.folderId) ? e.folderId : null;
          // Changing the DOM during dragstart can cancel the drag in Chromium.
          dragTimer.current = window.setTimeout(() => setDrag({ id: e.id, folderId }));
        }}
        onDragEnd={endDrag}
      >
        {e.kind === "env" ? (
          <span className="entry-avatar env" aria-hidden>
            .env
          </span>
        ) : (
          <span className="entry-avatar" aria-hidden>
            {(e.title.trim()[0] ?? "?").toUpperCase()}
          </span>
        )}
        <span className="entry-text">
          <span className="entry-title">
            {e.title || "(untitled)"}
            {e.favorite && (
              <span className="star" aria-label="favorite">
                {" "}
                ★
              </span>
            )}
          </span>
          <span className="entry-sub">
            {e.kind === "env" ? "Environment variables" : e.username || e.url}
          </span>
        </span>
      </button>
    </li>
  );

  return (
    <ul
      className={`entry-list${over === NO_FOLDER ? " drop-over" : ""}`}
      {...dropTarget(null)}
      onDragLeave={(e) => {
        if (!e.currentTarget.contains(e.relatedTarget as Node | null)) setOver(null);
      }}
    >
      {groups.map(({ folder, items }) => {
        const open = filtering || !collapsed.has(folder.id);
        return (
          <li
            key={folder.id}
            className={`folder${over === folder.id ? " drop-over" : ""}`}
            {...dropTarget(folder.id)}
          >
            <div className="folder-header">
              <button
                className="folder-toggle"
                aria-expanded={open}
                onClick={() => !filtering && onToggleFolder(folder.id)}
              >
                <svg
                  className={`folder-chevron${open ? " open" : ""}`}
                  width="10"
                  height="10"
                  viewBox="0 0 10 10"
                  aria-hidden="true"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.6"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  <path d="M3.5 2 6.5 5 3.5 8" />
                </svg>
                <FolderIcon open={open} />
                <span className="folder-name">{folder.name}</span>
                <span className="folder-count">{items.length}</span>
              </button>
              <span className="folder-actions">
                <button
                  className="icon-only"
                  title="Rename folder"
                  aria-label={`Rename folder ${folder.name}`}
                  onClick={() => onRenameFolder(folder)}
                >
                  <svg
                    width="13"
                    height="13"
                    viewBox="0 0 14 14"
                    aria-hidden="true"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="1.4"
                    strokeLinejoin="round"
                  >
                    <path d="M9.5 2.5l2 2-7 7H2.5v-2z" />
                  </svg>
                </button>
                <button
                  className="icon-only"
                  title="Delete folder"
                  aria-label={`Delete folder ${folder.name}`}
                  onClick={() => onDeleteFolder(folder)}
                >
                  <svg
                    width="13"
                    height="13"
                    viewBox="0 0 14 14"
                    aria-hidden="true"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="1.4"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <path d="M2.5 4h9M5.5 4V2.5h3V4M3.5 4l.6 7.5h5.8l.6-7.5" />
                  </svg>
                </button>
              </span>
            </div>
            {open && (
              <ul className="folder-entries">
                {items.length ? items.map(item) : <li className="folder-empty">Drag entries here</li>}
              </ul>
            )}
          </li>
        );
      })}
      {loose.map(item)}
      {drag?.folderId && (
        <li className={`drop-out${over === NO_FOLDER ? " drop-over" : ""}`}>
          Drop here to take it out of the folder
        </li>
      )}
    </ul>
  );
}

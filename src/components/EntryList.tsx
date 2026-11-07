import type { EntrySummary } from "../api";

interface Props {
  entries: EntrySummary[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}

export function EntryList({ entries, selectedId, onSelect }: Props) {
  if (entries.length === 0) {
    return <p className="list-empty">No entries</p>;
  }
  return (
    <ul className="entry-list">
      {entries.map((e) => (
        <li key={e.id}>
          <button
            className={`entry-item${e.id === selectedId ? " selected" : ""}`}
            onClick={() => onSelect(e.id)}
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
      ))}
    </ul>
  );
}

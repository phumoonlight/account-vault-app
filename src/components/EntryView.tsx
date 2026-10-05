import { useEffect, useRef, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { Entry } from "../api";
import { HoldButton } from "./HoldButton";

interface Props {
  entry: Entry;
  onEdit: () => void;
  onDelete: () => void;
}

export function EntryView({ entry, onEdit, onDelete }: Props) {
  const [deleting, setDeleting] = useState(false);

  return (
    <div className="pane">
      <header className="pane-header">
        <h2>
          {entry.title || "(untitled)"}
          {entry.favorite && <span className="star"> ★</span>}
        </h2>
        <div className="actions">
          <button onClick={onEdit}>Edit</button>
        </div>
      </header>

      <dl className="fields">
        <Field label="Username" value={entry.username} />
        <Field label="Password" value={entry.password} secret />
        <Field label="Website" value={entry.url} link />
        {entry.customFields.map((f, i) => (
          <Field key={i} label={f.name || "(unnamed)"} value={f.value} secret={f.secret} />
        ))}
        {entry.notes && (
          <div className="field">
            <dt>Notes</dt>
            <dd className="notes">{entry.notes}</dd>
          </div>
        )}
      </dl>

      <p className="meta">
        Created {new Date(entry.createdAt).toLocaleString()} · Updated{" "}
        {new Date(entry.updatedAt).toLocaleString()}
      </p>

      {deleting ? (
        <section className="danger-zone" role="alert">
          <div>
            <h3>Delete "{entry.title || "(untitled)"}"?</h3>
            <p>This is a dangerous action. The entry is permanently removed and can't be recovered.</p>
          </div>
          <div className="actions">
            <button onClick={() => setDeleting(false)}>Cancel</button>
            <HoldButton className="danger" onConfirm={onDelete}>
              Hold to delete
            </HoldButton>
          </div>
        </section>
      ) : (
        <button className="text-danger" onClick={() => setDeleting(true)}>
          Delete entry
        </button>
      )}
    </div>
  );
}

interface FieldProps {
  label: string;
  value: string;
  secret?: boolean;
  link?: boolean;
}

function Field({ label, value, secret, link }: FieldProps) {
  const [revealed, setRevealed] = useState(false);
  const [copyState, setCopyState] = useState<"idle" | "copied" | "failed">("idle");
  const timer = useRef<number>(undefined);
  useEffect(() => () => window.clearTimeout(timer.current), []);

  if (!value) return null;

  async function copy() {
    let result: "copied" | "failed" = "copied";
    try {
      await navigator.clipboard.writeText(value);
    } catch {
      result = "failed";
    }
    setCopyState(result);
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => setCopyState("idle"), 1500);
  }

  const href = link ? (/^https?:\/\//i.test(value) ? value : `https://${value}`) : null;

  return (
    <div className="field">
      <dt>{label}</dt>
      <dd>
        <span className={`value${secret && !revealed ? " masked" : ""}`}>
          {secret && !revealed ? "••••••••••••" : value}
        </span>
        <span className="field-actions">
          {secret && (
            <button className="small" onClick={() => setRevealed((r) => !r)}>
              {revealed ? "Hide" : "Show"}
            </button>
          )}
          {href && (
            <button className="small" onClick={() => openUrl(href)}>
              Open
            </button>
          )}
          <button className="small" onClick={copy}>
            {copyState === "copied" ? "Copied" : copyState === "failed" ? "Failed" : "Copy"}
          </button>
        </span>
      </dd>
    </div>
  );
}

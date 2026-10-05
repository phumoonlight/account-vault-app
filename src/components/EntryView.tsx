import { useEffect, useId, useRef, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, type Entry } from "../api";
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

      {entry.tags.length > 0 && (
        <div className="tag-row">
          {entry.tags.map((t) => (
            <span className="tag" key={t}>
              {t}
            </span>
          ))}
        </div>
      )}

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

/** Fired on every copy so other fields drop their "Copied" countdown. */
const COPY_EVENT = "vault:clipboard-copy";

type CopyStatus =
  | { kind: "idle" | "copied" | "failed" }
  /** Secret on the clipboard; the backend clears it at `until` (ms epoch). */
  | { kind: "countdown"; until: number };

function Field({ label, value, secret, link }: FieldProps) {
  const id = useId();
  const [revealed, setRevealed] = useState(false);
  const [status, setStatus] = useState<CopyStatus>({ kind: "idle" });
  const [, setTick] = useState(0);
  const timer = useRef<number>(undefined);

  // Clears either a timeout or an interval (they share one id pool).
  const stopTimer = () => window.clearInterval(timer.current);

  useEffect(() => {
    // Another field copied, so whatever this one put on the clipboard is gone.
    const onCopy = (e: Event) => {
      if ((e as CustomEvent<string>).detail !== id) {
        stopTimer();
        setStatus({ kind: "idle" });
      }
    };
    window.addEventListener(COPY_EVENT, onCopy);
    return () => {
      window.removeEventListener(COPY_EVENT, onCopy);
      stopTimer();
    };
  }, [id]);

  if (!value) return null;

  async function copy() {
    stopTimer();
    window.dispatchEvent(new CustomEvent(COPY_EVENT, { detail: id }));
    try {
      const clearsIn = await api.copyToClipboard(value, !!secret);
      if (clearsIn) {
        const until = Date.now() + clearsIn * 1000;
        setStatus({ kind: "countdown", until });
        timer.current = window.setInterval(() => {
          if (Date.now() >= until) {
            stopTimer();
            setStatus({ kind: "idle" });
          } else {
            setTick((t) => t + 1);
          }
        }, 1000);
        return;
      }
      setStatus({ kind: "copied" });
    } catch {
      setStatus({ kind: "failed" });
    }
    timer.current = window.setTimeout(() => setStatus({ kind: "idle" }), 1500);
  }

  const copyLabel =
    status.kind === "countdown"
      ? `Copied · ${Math.max(1, Math.ceil((status.until - Date.now()) / 1000))}s`
      : status.kind === "copied"
        ? "Copied"
        : status.kind === "failed"
          ? "Failed"
          : "Copy";

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
          <button
            className="small"
            onClick={copy}
            title={status.kind === "countdown" ? "The clipboard will be cleared automatically" : undefined}
          >
            {copyLabel}
          </button>
        </span>
      </dd>
    </div>
  );
}

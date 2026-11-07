import { useEffect, useState, type FormEvent } from "react";
import { MAX_FOLDER_NAME_LEN } from "../api";

interface Props {
  title: string;
  submitLabel: string;
  initialName?: string;
  /** Throw to show the error in the dialog and let the user try again. */
  onSubmit: (name: string) => Promise<void>;
  onCancel: () => void;
}

/** Asks for a folder name, when creating or renaming a folder. */
export function FolderDialog({ title, submitLabel, initialName = "", onSubmit, onCancel }: Props) {
  const [name, setName] = useState(initialName);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && !busy && onCancel();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [busy, onCancel]);

  async function submit(e: FormEvent) {
    e.preventDefault();
    if (!name.trim() || busy) return;
    setBusy(true);
    setError(null);
    try {
      await onSubmit(name);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="dialog-backdrop" onMouseDown={() => !busy && onCancel()}>
      <form
        className="dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="folder-dialog-title"
        onSubmit={submit}
        onMouseDown={(e) => e.stopPropagation()}
      >
        <h2 id="folder-dialog-title">{title}</h2>
        <input
          autoFocus
          aria-label="Folder name"
          placeholder="Folder name, e.g. Work"
          maxLength={MAX_FOLDER_NAME_LEN}
          value={name}
          onChange={(e) => setName(e.target.value)}
          onFocus={(e) => e.target.select()}
          disabled={busy}
        />
        {error && <p className="hint-error">{error}</p>}
        <div className="dialog-actions">
          <button type="button" onClick={onCancel} disabled={busy}>
            Cancel
          </button>
          <button type="submit" className="primary" disabled={!name.trim() || busy}>
            {submitLabel}
          </button>
        </div>
      </form>
    </div>
  );
}

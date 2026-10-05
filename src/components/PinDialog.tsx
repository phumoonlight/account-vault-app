import { useEffect, useState, type FormEvent } from "react";

const PIN_PATTERN = /^\d{4,6}$/;

interface Props {
  title: string;
  description: string;
  submitLabel: string;
  busyLabel: string;
  /** Ask for the PIN twice (when creating a backup). */
  confirm?: boolean;
  /** Throw to show the error in the dialog and let the user try again. */
  onSubmit: (pin: string) => Promise<void>;
  onCancel: () => void;
}

export function PinDialog({
  title,
  description,
  submitLabel,
  busyLabel,
  confirm,
  onSubmit,
  onCancel,
}: Props) {
  const [pin, setPin] = useState("");
  const [pinAgain, setPinAgain] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && !busy && onCancel();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [busy, onCancel]);

  const valid = PIN_PATTERN.test(pin) && (!confirm || pin === pinAgain);
  const hint =
    pin && !PIN_PATTERN.test(pin)
      ? "PIN must be 4 to 6 digits."
      : confirm && pinAgain && pin !== pinAgain
        ? "PINs don't match."
        : null;

  async function submit(e: FormEvent) {
    e.preventDefault();
    if (!valid || busy) return;
    setBusy(true);
    setError(null);
    try {
      await onSubmit(pin);
    } catch (err) {
      setError(String(err));
      setPin("");
      setPinAgain("");
    } finally {
      setBusy(false);
    }
  }

  // Digits only, max 6.
  const clean = (value: string) => value.replace(/\D/g, "").slice(0, 6);

  return (
    <div className="dialog-backdrop" onMouseDown={() => !busy && onCancel()}>
      <form
        className="dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="pin-dialog-title"
        onSubmit={submit}
        onMouseDown={(e) => e.stopPropagation()}
      >
        <h2 id="pin-dialog-title">{title}</h2>
        <p className="dialog-desc">{description}</p>

        <input
          className="pin-input"
          type="password"
          inputMode="numeric"
          autoComplete="off"
          placeholder="PIN"
          autoFocus
          value={pin}
          onChange={(e) => setPin(clean(e.target.value))}
          disabled={busy}
        />
        {confirm && (
          <input
            className="pin-input"
            type="password"
            inputMode="numeric"
            autoComplete="off"
            placeholder="Confirm PIN"
            value={pinAgain}
            onChange={(e) => setPinAgain(clean(e.target.value))}
            disabled={busy}
          />
        )}

        {(error || hint) && <p className="hint-error">{error ?? hint}</p>}

        <div className="dialog-actions">
          <button type="button" onClick={onCancel} disabled={busy}>
            Cancel
          </button>
          <button type="submit" className="primary" disabled={!valid || busy}>
            {busy ? busyLabel : submitLabel}
          </button>
        </div>
      </form>
    </div>
  );
}

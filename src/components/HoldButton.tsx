import { useEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";

interface Props {
  onConfirm: () => void;
  /** How long the button must be held, in milliseconds. */
  holdMs?: number;
  className?: string;
  children: ReactNode;
}

/**
 * Button that only fires after being held down (mouse, touch, or Space/Enter)
 * for `holdMs`. A fill bar shows progress; releasing early cancels.
 */
export function HoldButton({ onConfirm, holdMs = 1500, className = "", children }: Props) {
  const [holding, setHolding] = useState(false);
  const timer = useRef<number>(undefined);

  useEffect(() => () => window.clearTimeout(timer.current), []);

  function start() {
    window.clearTimeout(timer.current);
    setHolding(true);
    timer.current = window.setTimeout(() => {
      setHolding(false);
      onConfirm();
    }, holdMs);
  }

  function cancel() {
    window.clearTimeout(timer.current);
    setHolding(false);
  }

  const isKey = (e: KeyboardEvent) => e.key === " " || e.key === "Enter";

  return (
    <button
      type="button"
      className={`hold-button ${holding ? "holding" : ""} ${className}`}
      style={{ "--hold-ms": `${holdMs}ms` } as React.CSSProperties}
      onPointerDown={(e) => e.button === 0 && start()}
      onPointerUp={cancel}
      onPointerLeave={cancel}
      onPointerCancel={cancel}
      onKeyDown={(e) => {
        if (isKey(e)) {
          e.preventDefault();
          if (!e.repeat) start();
        }
      }}
      onKeyUp={(e) => isKey(e) && cancel()}
      onBlur={cancel}
      onContextMenu={(e) => e.preventDefault()}
    >
      <span className="hold-fill" aria-hidden />
      <span className="hold-label">{children}</span>
    </button>
  );
}

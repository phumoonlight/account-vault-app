import { useEffect, useId, useRef, useState } from "react";
import { api } from "../api";

/** Fired on every copy so other fields drop their "Copied" countdown. */
const COPY_EVENT = "vault:clipboard-copy";

type CopyStatus =
  | { kind: "idle" | "copied" | "failed" }
  /** Secret on the clipboard; the backend clears it at `until` (ms epoch). */
  | { kind: "countdown"; until: number };

/** Copy through the backend, with a "Copied · 30s" countdown for secrets. */
export function useCopy() {
  const id = useId();
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

  async function copy(value: string, secret: boolean) {
    stopTimer();
    window.dispatchEvent(new CustomEvent(COPY_EVENT, { detail: id }));
    try {
      const clearsIn = await api.copyToClipboard(value, secret);
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

  const label =
    status.kind === "countdown"
      ? `Copied · ${Math.max(1, Math.ceil((status.until - Date.now()) / 1000))}s`
      : status.kind === "copied"
        ? "Copied"
        : status.kind === "failed"
          ? "Failed"
          : "Copy";
  const title =
    status.kind === "countdown" ? "The clipboard will be cleared automatically" : undefined;

  return { copy, label, title };
}

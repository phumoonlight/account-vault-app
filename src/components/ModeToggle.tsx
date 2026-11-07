import { useState } from "react";

export type EnvMode = "grid" | "raw";

const STORAGE_KEY = "env-view-mode";

/** Grid/Raw choice for env entries, remembered across entries and restarts. */
export function useEnvMode() {
  const [mode, setMode] = useState<EnvMode>(() => {
    try {
      return localStorage.getItem(STORAGE_KEY) === "raw" ? "raw" : "grid";
    } catch {
      return "grid";
    }
  });
  const set = (next: EnvMode) => {
    setMode(next);
    try {
      localStorage.setItem(STORAGE_KEY, next);
    } catch {
      // Not remembered; fine.
    }
  };
  return [mode, set] as const;
}

export function ModeToggle({ mode, onChange }: { mode: EnvMode; onChange: (m: EnvMode) => void }) {
  return (
    <div className="segmented" role="group" aria-label="View mode">
      {(["grid", "raw"] as const).map((m) => (
        <button
          key={m}
          type="button"
          className={mode === m ? "active" : undefined}
          aria-pressed={mode === m}
          onClick={() => onChange(m)}
        >
          {m === "grid" ? "Grid" : "Raw"}
        </button>
      ))}
    </div>
  );
}

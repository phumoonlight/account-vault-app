import { useMemo, useRef, useState, type ChangeEvent } from "react";
import { envPairs, mergeEnv, pair, parseEnv, serializeEnv, type EnvLine } from "../dotenv";
import { ModeToggle, useEnvMode } from "./ModeToggle";

/** .env files are small; anything bigger is almost certainly the wrong file. */
const MAX_IMPORT_BYTES = 1024 * 1024;

interface Props {
  value: string;
  onChange: (text: string) => void;
  /** Called with the file name after a successful .env import. */
  onImported?: (fileName: string) => void;
}

export function EnvEditor({ value, onChange, onImported }: Props) {
  const [mode, setMode] = useEnvMode();
  const [showValues, setShowValues] = useState(false);
  const [message, setMessage] = useState<{ error: boolean; text: string } | null>(null);
  const fileInput = useRef<HTMLInputElement>(null);

  const lines = useMemo(() => parseEnv(value), [value]);
  const duplicates = useMemo(() => {
    const seen = new Set<string>();
    const dup = new Set<string>();
    for (const { key } of envPairs(lines)) {
      if (key && seen.has(key)) dup.add(key);
      seen.add(key);
    }
    return dup;
  }, [lines]);

  const update = (next: EnvLine[]) => onChange(serializeEnv(next));
  const setPair = (index: number, key: string, val: string) =>
    update(lines.map((l, i) => (i === index ? pair(key, val) : l)));

  async function importFile(e: ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    e.target.value = ""; // Allow picking the same file again.
    if (!file) return;
    try {
      if (file.size > MAX_IMPORT_BYTES) throw new Error("That file is too large for a .env file.");
      const text = await file.text();
      if (text.includes("\0")) throw new Error("That doesn't look like a text file.");
      const found = envPairs(parseEnv(text)).filter((l) => l.key).length;
      if (!found) throw new Error(`No variables found in ${file.name}.`);
      const merged = mergeEnv(value, text);
      onChange(merged.text);
      onImported?.(file.name);
      const parts = [`${merged.added} added`];
      if (merged.updated) parts.push(`${merged.updated} updated`);
      setMessage({ error: false, text: `Imported ${file.name}: ${parts.join(", ")}.` });
    } catch (err) {
      setMessage({ error: true, text: err instanceof Error ? err.message : String(err) });
    }
  }

  return (
    <div className="env-editor">
      <div className="env-toolbar">
        <ModeToggle mode={mode} onChange={setMode} />
        <span className="env-toolbar-actions">
          {mode === "grid" && (
            <button type="button" className="small" onClick={() => setShowValues((s) => !s)}>
              {showValues ? "Hide values" : "Show values"}
            </button>
          )}
          <button type="button" className="small" onClick={() => fileInput.current?.click()}>
            Import .env…
          </button>
        </span>
        <input ref={fileInput} type="file" hidden onChange={importFile} />
      </div>

      {message && (
        <p className={message.error ? "hint-error" : "hint"} role="status">
          {message.text}
        </p>
      )}

      {mode === "raw" ? (
        <textarea
          className="env-raw"
          rows={12}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={"# One variable per line\nAPI_URL=https://example.com\nAPI_KEY=..."}
          spellCheck={false}
          autoComplete="off"
          wrap="off"
        />
      ) : (
        <div className="env-grid-edit">
          {lines.map((line, i) =>
            line.type === "pair" ? (
              <div className="env-edit-row" key={i}>
                <input
                  className={`env-key${duplicates.has(line.key) ? " dup" : ""}`}
                  value={line.key}
                  placeholder="KEY"
                  aria-label="Variable name"
                  title={duplicates.has(line.key) ? "This key appears more than once" : undefined}
                  spellCheck={false}
                  autoComplete="off"
                  onChange={(e) =>
                    setPair(i, e.target.value.replace(/[\s=]/g, "").replace(/^#+/, ""), line.value)
                  }
                />
                {line.value.includes("\n") ? (
                  <textarea
                    className="env-value"
                    rows={Math.min(6, line.value.split("\n").length)}
                    value={line.value}
                    aria-label={`Value of ${line.key || "variable"}`}
                    spellCheck={false}
                    onChange={(e) => setPair(i, line.key, e.target.value)}
                  />
                ) : (
                  <input
                    className="env-value"
                    type={showValues ? "text" : "password"}
                    value={line.value}
                    placeholder="value"
                    aria-label={`Value of ${line.key || "variable"}`}
                    spellCheck={false}
                    autoComplete="off"
                    onChange={(e) => setPair(i, line.key, e.target.value)}
                  />
                )}
                <button
                  type="button"
                  className="small"
                  aria-label={`Remove ${line.key || "variable"}`}
                  onClick={() => update(lines.filter((_, j) => j !== i))}
                >
                  Remove
                </button>
              </div>
            ) : null,
          )}
          {envPairs(lines).length === 0 && (
            <p className="hint">
              No variables yet. Add one, paste into Raw, or import a .env file.
            </p>
          )}
          <button type="button" className="small" onClick={() => update([...lines, pair("", "")])}>
            + Add variable
          </button>
          {lines.some((l) => l.type === "other" && l.raw.trim()) && (
            <p className="hint">Comments are kept; switch to Raw to edit them.</p>
          )}
        </div>
      )}
    </div>
  );
}

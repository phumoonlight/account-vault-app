import { useMemo, useState } from "react";
import { envPairs, parseEnv, type EnvLine } from "../dotenv";
import { ModeToggle, useEnvMode } from "./ModeToggle";
import { useCopy } from "./useCopy";

const MASK = "••••••••••";

/** Read-only env variables with Grid and Raw modes; values hidden until shown. */
export function EnvView({ text }: { text: string }) {
  const [mode, setMode] = useEnvMode();
  const [showAll, setShowAll] = useState(false);
  const copyAll = useCopy();
  const lines = useMemo(() => parseEnv(text), [text]);
  const pairs = envPairs(lines);

  return (
    <div className="env-view">
      <div className="env-toolbar">
        <ModeToggle mode={mode} onChange={setMode} />
        <span className="env-count">
          {pairs.length} {pairs.length === 1 ? "variable" : "variables"}
        </span>
        <span className="env-toolbar-actions">
          <button className="small" onClick={() => setShowAll((s) => !s)}>
            {showAll ? "Hide values" : "Show values"}
          </button>
          <button
            className="small"
            disabled={!text.trim()}
            onClick={() => copyAll.copy(text, true)}
            title={copyAll.title ?? "Copy the whole file in dotenv format"}
          >
            {copyAll.label === "Copy" ? "Copy all" : copyAll.label}
          </button>
        </span>
      </div>

      {!text.trim() ? (
        <p className="list-empty">No variables</p>
      ) : mode === "raw" ? (
        <pre className="env-raw-view">{showAll ? text : maskRaw(lines)}</pre>
      ) : (
        <div className="env-grid" role="table" aria-label="Environment variables">
          {pairs.map((p, i) => (
            <EnvRow key={i} name={p.key} value={p.value} showAll={showAll} />
          ))}
        </div>
      )}
    </div>
  );
}

function EnvRow({ name, value, showAll }: { name: string; value: string; showAll: boolean }) {
  const [revealed, setRevealed] = useState(false);
  const copier = useCopy();
  const visible = showAll || revealed;

  return (
    <div className="env-row" role="row">
      <span className="env-key" role="cell">
        {name || <em>(no name)</em>}
      </span>
      <span className={`env-value${visible ? "" : " masked"}`} role="cell">
        {!value ? <em className="env-empty">empty</em> : visible ? value : MASK}
      </span>
      <span className="field-actions" role="cell">
        {!showAll && value && (
          <button className="small" onClick={() => setRevealed((r) => !r)}>
            {revealed ? "Hide" : "Show"}
          </button>
        )}
        <button
          className="small"
          disabled={!value}
          onClick={() => copier.copy(value, true)}
          title={copier.title}
        >
          {copier.label}
        </button>
      </span>
    </div>
  );
}

/** Raw text with each value replaced by a mask; comments stay readable. */
function maskRaw(lines: EnvLine[]): string {
  return lines
    .map((l) => (l.type === "pair" ? `${l.key}=${l.value ? MASK : ""}` : l.raw))
    .join("\n");
}

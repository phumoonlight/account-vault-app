// Minimal dotenv parser/serializer for env entries. The stored text stays the
// source of truth: parsing keeps every line's original text, so comments,
// blank lines and untouched values survive edits made in the grid.

export type EnvLine =
  | { type: "pair"; key: string; value: string; raw: string }
  /** Comment, blank or unparseable line, kept as-is. */
  | { type: "other"; raw: string };

// Optional `export `, then a key (anything without spaces/`=`, not a comment;
// empty is allowed so a freshly added grid row stays a row), then `=`.
const PAIR = /^\s*(?:export\s+)?([^\s=#][^\s=]*|)\s*=\s?(.*)$/;

export function parseEnv(text: string): EnvLine[] {
  if (!text) return [];
  const lines = text.replace(/\r\n?/g, "\n").split("\n");
  // A trailing newline isn't an extra blank line.
  if (lines[lines.length - 1] === "") lines.pop();

  const out: EnvLine[] = [];
  for (let i = 0; i < lines.length; i++) {
    const m = PAIR.exec(lines[i]);
    if (!m) {
      out.push({ type: "other", raw: lines[i] });
      continue;
    }
    const key = m[1];
    let rest = m[2].trimStart();
    const quote = rest[0];

    if (quote === '"' || quote === "'" || quote === "`") {
      // Quoted values may span lines until the closing quote.
      let raw = lines[i];
      let body = rest.slice(1);
      let end = findClose(body, quote);
      let j = i;
      while (end < 0 && j + 1 < lines.length) {
        j++;
        raw += "\n" + lines[j];
        body += "\n" + lines[j];
        end = findClose(body, quote);
      }
      if (end < 0) {
        // Never closed: treat the line as a plain value rather than eating the file.
        out.push({ type: "pair", key, value: stripComment(rest), raw: lines[i] });
        continue;
      }
      i = j;
      const inner = body.slice(0, end);
      out.push({ type: "pair", key, value: quote === '"' ? unescape(inner) : inner, raw });
    } else {
      out.push({ type: "pair", key, value: stripComment(rest), raw: lines[i] });
    }
  }
  return out;
}

export function serializeEnv(lines: EnvLine[]): string {
  return lines.length ? lines.map((l) => l.raw).join("\n") + "\n" : "";
}

/** A `KEY=value` line, quoting the value only when it needs it. */
export function formatPair(key: string, value: string): string {
  if (/^[^\s"'`#\\]*$/.test(value)) return `${key}=${value}`;
  const escaped = value
    .replace(/\\/g, "\\\\")
    .replace(/"/g, '\\"')
    .replace(/\n/g, "\\n")
    .replace(/\r/g, "\\r")
    .replace(/\t/g, "\\t");
  return `${key}="${escaped}"`;
}

export function pair(key: string, value: string): EnvLine {
  return { type: "pair", key, value, raw: formatPair(key, value) };
}

/** Keys of the pairs, for counting and duplicate checks. */
export function envPairs(lines: EnvLine[]) {
  return lines.filter((l): l is Extract<EnvLine, { type: "pair" }> => l.type === "pair");
}

/**
 * Merges `incoming` into `current`: existing keys get the new value in place,
 * new keys are appended. Returns the merged text and what changed.
 */
export function mergeEnv(current: string, incoming: string) {
  const lines = parseEnv(current);
  const added: string[] = [];
  let updated = 0;
  for (const line of envPairs(parseEnv(incoming))) {
    if (!line.key) continue;
    const index = lines.findIndex((l) => l.type === "pair" && l.key === line.key);
    if (index < 0) {
      lines.push(pair(line.key, line.value));
      added.push(line.key);
    } else if ((lines[index] as { value: string }).value !== line.value) {
      lines[index] = pair(line.key, line.value);
      updated++;
    }
  }
  return { text: serializeEnv(lines), added: added.length, updated };
}

/** Index of the unescaped closing quote in `s`, or -1. */
function findClose(s: string, quote: string): number {
  for (let i = 0; i < s.length; i++) {
    if (s[i] === "\\" && quote === '"') i++;
    else if (s[i] === quote) return i;
  }
  return -1;
}

/** Unquoted values end at ` #` (an inline comment) and are trimmed. */
function stripComment(s: string): string {
  const m = /\s#/.exec(s);
  return (m ? s.slice(0, m.index) : s).trim();
}

function unescape(s: string): string {
  return s.replace(/\\(.)/g, (_, c: string) =>
    c === "n" ? "\n" : c === "r" ? "\r" : c === "t" ? "\t" : c,
  );
}

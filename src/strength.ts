// Lightweight password strength estimate (no dictionary dependency).
// Entropy from length × character pool, discounted for repeats, sequences
// and keyboard runs; common passwords and "common word + digits" are capped.

export type Score = 0 | 1 | 2 | 3 | 4;

export interface Strength {
  score: Score;
  label: string;
  /** Short advice, or null when there's nothing to improve. */
  tip: string | null;
}

const LABELS = ["Very weak", "Weak", "Fair", "Strong", "Very strong"] as const;

const COMMON = new Set([
  "password", "passw0rd", "123456", "12345678", "123456789", "1234567890", "qwerty",
  "qwertyuiop", "abc123", "111111", "000000", "123123", "letmein", "welcome", "admin",
  "administrator", "iloveyou", "monkey", "dragon", "football", "baseball", "sunshine",
  "princess", "master", "shadow", "superman", "batman", "trustno1", "login", "hello",
  "freedom", "whatever", "starwars", "secret", "pass", "test", "guest", "root",
  "changeme", "default", "asdfgh", "zxcvbn", "654321", "666666", "121212", "michael",
  "charlie", "jennifer", "computer", "internet",
]);

const KEYBOARD_ROWS = ["qwertyuiop", "asdfghjkl", "zxcvbnm", "1234567890"];

function hasKeyboardRun(lower: string, minLen = 4): boolean {
  for (const row of KEYBOARD_ROWS) {
    for (const r of [row, [...row].reverse().join("")]) {
      for (let i = 0; i + minLen <= r.length; i++) {
        if (lower.includes(r.slice(i, i + minLen))) return true;
      }
    }
  }
  return false;
}

export function passwordStrength(password: string): Strength | null {
  if (!password) return null;
  const chars = [...password];
  const lower = password.toLowerCase();

  // Common password, or common word wrapped in digits/symbols ("Password123!").
  const core = lower.replace(/^[^a-z]+|[^a-z]+$/g, "");
  if (COMMON.has(lower) || (core.length >= 4 && COMMON.has(core))) {
    return { score: 0, label: LABELS[0], tip: "This is a very common password." };
  }

  const classes = {
    lower: /[a-z]/.test(password),
    upper: /[A-Z]/.test(password),
    digit: /\d/.test(password),
    symbol: /[^A-Za-z0-9]/.test(password),
  };
  const classCount = Object.values(classes).filter(Boolean).length;
  let pool =
    (classes.lower ? 26 : 0) + (classes.upper ? 26 : 0) + (classes.digit ? 10 : 0) + (classes.symbol ? 33 : 0);
  if (chars.some((c) => c.codePointAt(0)! > 127)) pool += 100;

  // Characters that repeat or continue a sequence (aaa, abc, 321) add little.
  let effective = 0;
  let patterned = false;
  for (let i = 0; i < chars.length; i++) {
    const prev = i > 0 ? chars[i - 1].codePointAt(0)! : NaN;
    const cur = chars[i].codePointAt(0)!;
    if (cur === prev || Math.abs(cur - prev) === 1) {
      effective += 0.25;
      patterned = true;
    } else {
      effective += 1;
    }
  }
  if (hasKeyboardRun(lower)) {
    effective -= 2;
    patterned = true;
  }

  const bits = Math.max(0, effective) * Math.log2(Math.max(pool, 2));
  let score: Score = bits < 28 ? 0 : bits < 36 ? 1 : bits < 60 ? 2 : bits < 80 ? 3 : 4;
  // Without a dictionary, short or single-class passwords (often plain words)
  // look stronger than they are, so cap them.
  if (chars.length < 8 || (chars.length < 12 && classCount === 1)) score = Math.min(score, 1) as Score;
  else if (chars.length < 12) score = Math.min(score, 2) as Score;

  let tip: string | null = null;
  if (score < 4) {
    if (chars.length < 12) tip = "Use at least 12 characters.";
    else if (patterned) tip = "Avoid repeated characters and sequences like abc, 123 or qwerty.";
    else if (classCount < 3) tip = "Mix uppercase, lowercase, numbers and symbols.";
    else tip = "Make it longer for extra safety.";
  }
  return { score, label: LABELS[score], tip };
}

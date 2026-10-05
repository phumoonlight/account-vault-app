// Bumps the app version everywhere it's stored.
//
//   npm run bump            # patch: 0.1.0 -> 0.1.1 (fixes, small changes)
//   npm run bump minor      # minor: 0.1.1 -> 0.2.0 (new features)
//   npm run bump major      # major: 0.2.0 -> 1.0.0
//   npm run bump 1.4.2      # explicit version
//
// package.json is the source of truth; tauri.conf.json reads it directly
// ("version": "../package.json"). This script keeps package-lock.json,
// Cargo.toml and Cargo.lock in sync.

import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const path = (p) => join(root, p);

function update(file, transform) {
  const original = readFileSync(path(file), "utf8");
  const updated = transform(original);
  if (updated === original) throw new Error(`${file}: version not found`);
  writeFileSync(path(file), updated);
}

const pkg = JSON.parse(readFileSync(path("package.json"), "utf8"));
const current = pkg.version;
const arg = process.argv[2] ?? "patch";

const semver = /^(\d+)\.(\d+)\.(\d+)$/;
const match = current.match(semver);
if (!match) throw new Error(`package.json version "${current}" is not x.y.z`);
let [major, minor, patch] = match.slice(1).map(Number);

let next;
if (arg === "patch") next = `${major}.${minor}.${patch + 1}`;
else if (arg === "minor") next = `${major}.${minor + 1}.0`;
else if (arg === "major") next = `${major + 1}.0.0`;
else if (semver.test(arg)) next = arg;
else {
  console.error(`Unknown bump "${arg}". Use patch, minor, major, or x.y.z.`);
  process.exit(1);
}

const name = pkg.name;
const esc = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

// Replace only the first `"version": "..."` (the package's own version).
update("package.json", (s) => s.replace(/("version":\s*")[^"]+(")/, `$1${next}$2`));
// Top-level version and the root package entry ("").
update("package-lock.json", (s) =>
  s
    .replace(/("version":\s*")[^"]+(")/, `$1${next}$2`)
    .replace(/("":\s*\{\s*"name":\s*"[^"]*",\s*"version":\s*")[^"]+(")/, `$1${next}$2`),
);
update("src-tauri/Cargo.toml", (s) =>
  s.replace(/(\[package\][^[]*?\nversion\s*=\s*")[^"]+(")/, `$1${next}$2`),
);
update("src-tauri/Cargo.lock", (s) =>
  s.replace(new RegExp(`(name = "${esc(name)}"\\r?\\nversion = ")[^"]+(")`), `$1${next}$2`),
);

console.log(`${current} -> ${next}`);

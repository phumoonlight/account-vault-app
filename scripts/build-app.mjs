// Builds the release installers and copies them to release/ at the project root.
//
//   npm run build:app
//   npm run build:app -- --bundles nsis   # extra args are passed to `tauri build`
//
// Tauri always writes bundles to src-tauri/target/[<triple>/]<profile>/bundle/<format>/
// and has no option to change that, so this runs `tauri build` and then copies
// out the installers that build produced.

import { spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readdirSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, extname, join } from "node:path";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const outDir = join(root, "release");
const installerExts = new Set([".exe", ".msi", ".dmg", ".deb", ".rpm", ".AppImage"]);

const args = process.argv.slice(2);
const flagValue = (...names) => {
  const i = args.findIndex((a) => names.includes(a));
  return i >= 0 ? args[i + 1] : undefined;
};
const profile = args.includes("--debug") || args.includes("-d") ? "debug" : "release";
const triple = flagValue("--target", "-t");
const bundleDir = join(root, "src-tauri", "target", ...(triple ? [triple] : []), profile, "bundle");

// The bundle folder keeps installers from earlier builds; only copy files written by this one.
const startedAt = Date.now() - 1000;
const command = ["npx tauri build", ...args.map((a) => JSON.stringify(a))].join(" ");
const result = spawnSync(command, { cwd: root, stdio: "inherit", shell: true });
if (result.status !== 0) process.exit(result.status ?? 1);

if (!existsSync(bundleDir)) {
  console.error(`No bundle folder at ${bundleDir}`);
  process.exit(1);
}

mkdirSync(outDir, { recursive: true });
let copied = 0;
for (const format of readdirSync(bundleDir, { withFileTypes: true })) {
  if (!format.isDirectory()) continue;
  const dir = join(bundleDir, format.name);
  for (const file of readdirSync(dir, { withFileTypes: true })) {
    if (!file.isFile() || !installerExts.has(extname(file.name))) continue;
    const src = join(dir, file.name);
    if (statSync(src).mtimeMs < startedAt) continue;
    copyFileSync(src, join(outDir, file.name));
    console.log(`Copied ${file.name} -> release/`);
    copied++;
  }
}
if (copied === 0) {
  console.error("Build finished but no new installers were found to copy.");
  process.exit(1);
}

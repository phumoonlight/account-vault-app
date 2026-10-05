# CLAUDE.md

Homemade Account Vault: a desktop app that stores account credentials (title, username, password, URL, notes, custom fields) in an encrypted local vault, with PIN-protected backup export/import.

Stack: **Tauri 2** (Rust backend in `src-tauri/`) + **React 19 / TypeScript / Vite** (frontend in `src/`). Windows is the primary target.

## Commands

```sh
npm run dev:app            # run the app (= tauri dev; Vite on :1420 + Rust, hot reload both sides)
npm run build              # typecheck (tsc) + build frontend: the frontend "test"
npm run build:app          # release installer (= tauri build) -> src-tauri/target/release/bundle/
                           # (keep "dev"/"build" as vite: Tauri runs "npm run dev"/"npm run build"
                           #  itself, so pointing them at tauri would recurse forever)

cd src-tauri
cargo test                 # Rust unit tests (all logic tests live here)
cargo clippy --all-targets # should stay warning-free
cargo fmt                  # run before committing
```

If `cargo` isn't found in a terminal opened before Rust was installed, it's a stale PATH (`~/.cargo/bin`); restart VS Code.

## Versioning

- **Bump the version in every commit that changes the app**, as part of that commit. Run `npm run bump` (patch) for fixes and small changes, `npm run bump minor` for new features, and `npm run bump major` for breaking changes, such as a vault/backup format that older versions can't read.
- `package.json` is the single source of truth. `tauri.conf.json` reads it (`"version": "../package.json"`), and the script syncs `package-lock.json`, `Cargo.toml` and `Cargo.lock`. Don't edit versions by hand.
- Docs-only or tooling-only commits don't need a bump.

## Architecture

See [docs/architecture.md](docs/architecture.md) for the code layout, the **security model and its invariants**, and the vault/backup file formats. Read it before changing anything under `src-tauri/src/vault/`.

## Conventions and gotchas

- **Rust ↔ TS types:** Rust structs use `#[serde(rename_all = "camelCase")]`; keep `src/api.ts` in sync by hand. Tauri command args are camelCase on the JS side.
- **Command errors** are `Result<T, VaultError>`; the UI receives `String(err)` = the `#[error("...")]` text, so write those messages for end users.
- **Argon2 work belongs in `async` commands** (sync commands run on the main thread and would freeze the UI). Don't hold the vault mutex while deriving keys.
- **`ZeroizeOnDrop` adds a `Drop` impl**, so you can't move fields out of these structs or use `..Default::default()` / container-level `#[serde(default)]` on them. Use field-level `#[serde(default)]`, `std::mem::take`, or clone.
- **Crypto crate versions are new APIs:** `chacha20poly1305` 0.11 / `aead` 0.6 (`XNonce::generate()`, `Generate` trait, `new_from_slice`); `argon2` 0.6; `keyring` 4 with the `v1` API (`Entry::new`, `get_secret`/`set_secret`, `Error::NoEntry`). Check sources in `~/.cargo/registry` rather than older docs.
- **Legacy data:** `Entry` has a read-only `legacy_email` (serde `email`); `Entry::migrate()` merges it into `username` (or an "Email" custom field) on load/import. Use the same pattern for future field changes.
- Dev builds compile `argon2` and `chacha20poly1305` with `opt-level = 3` (see `Cargo.toml`); otherwise PIN derivation takes many seconds in dev.
- **Tests:** Rust tests use `MemoryKeyStore` and tiny Argon2 params (`TEST_KDF`) with `tempfile`; they never touch the real credential store. There are no frontend tests; `npm run build` is the typecheck.
- **UI style:** plain CSS in `src/App.css` with color tokens on `:root`, light/dark via `prefers-color-scheme`. No UI library. Destructive actions use the reveal-then-hold pattern (red text → danger section → `HoldButton`).

## Known gaps / ideas not yet done

- Copied secrets still land in Windows clipboard history (Win+V); excluding them needs the Windows clipboard-format flags.
- No unsaved-changes guard when leaving the edit form.
- CSV import/export was removed on purpose (the owner wants only PIN-protected backups).

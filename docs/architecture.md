# Architecture

Paths below are relative to the repository root.

The frontend never touches the key, the vault file, or the filesystem. It calls Tauri commands and gets entry data back.

```
src/                         React UI
  App.tsx                    layout, app state (Pane: empty|view|edit|new), Import/Export dialogs
  api.ts                     typed wrappers around invoke(); TS types mirror Rust model (camelCase)
  password.ts                password generator (crypto.getRandomValues, rejection sampling)
  strength.ts                password strength estimate (entropy + pattern/common-password checks, no deps)
  tags.ts                    tag normalization (mirrors Rust normalize_tags) and tag counts for the filter
  components/
    EntryList / EntryView / EntryForm
    TagInput.tsx             chip input with autocomplete from existing tags
    StrengthMeter.tsx        4-segment bar + tip under the password field
    HoldButton.tsx           press-and-hold confirm (pointer + Space/Enter); used for delete
    PinDialog.tsx            4-6 digit PIN modal; onSubmit throws -> error shown inline, retry
src-tauri/src/
  lib.rs                     plugin setup, builds AppState (vault path + key store), registers commands
  clipboard.rs               ClipboardGuard: copy, auto-clear secrets after 30 s, clear any copy on exit (only if still ours)
  commands.rs                #[tauri::command] fns; defines AppState { vault_path, keys, Mutex<Option<Vault>> }
  vault/
    mod.rs                   Vault: open/create, CRUD, import_entries, crash-safe save(); tests
    model.rs                 VaultData, Entry, EntryInput, EntrySummary, CustomField
    crypto.rs                vault file format (XChaCha20-Poly1305)
    keystore.rs              KeyStore trait; OsKeyStore = OS credential store via `keyring`
    backup.rs                .avbackup format (Argon2id PIN -> XChaCha20-Poly1305)
    error.rs                 VaultError (thiserror); serialized to the UI as a message string
```

## Security model (keep these invariants)

- **No master password.** The vault key is 32 random bytes stored in the OS credential store (Windows Credential Manager target `vault-key.com.phumoonlight.accountvault`). Anyone logged in as the OS user can open the app. This is a deliberate product decision.
- **The vault file only opens on this machine/account.** Portable backups are the `.avbackup` export.
- If the vault file exists but the key is missing → `KeyMissing` error. **Never** generate a new key over an existing vault (there's a test for this).
- The vault opens lazily on the first command (`AppState::with_vault`), so open errors reach the UI instead of crashing startup.
- Secrets in Rust are `Zeroizing`/`ZeroizeOnDrop`. `list_entries` returns `EntrySummary` (no password); passwords only cross to the UI via `get_entry`.
- `save()` writes a temp file, calls `sync_all()`, then renames. Don't drop the fsync; without it a power loss can leave an empty vault.
- Mutations go through `Vault::mutate`, which rolls back in-memory data if saving fails.
- Backup PIN: digits only, 4-6 (validated in the frontend **and** `backup::is_valid_pin`). Argon2id is 256 MiB / t=12 / p=4 (~1.1 s) because a PIN has ≤10^6 values. Don't lower it without reason. KDF params are stored in the file header and capped on read (`is_reasonable`) so crafted files can't exhaust memory.
- Copy goes through the `copy_to_clipboard` command (not `navigator.clipboard`). Secrets are cleared after `CLEAR_AFTER` (30 s); anything copied is cleared on `RunEvent::Exit`; both only if the clipboard still holds exactly what we copied. Closing the window while our copy is still on the clipboard is blocked (`CloseRequested` → `close-requested` event → `ConfirmDialog` → `close_app`/`cancel_close`); a second close while the warning is pending goes through, so a broken UI can't trap the window. Windows clipboard history (Win+V) is **not** excluded yet.
- CSP in `tauri.conf.json` is strict; capabilities are `core:default`, `opener:default`, `dialog:default`.
- Import refuses files over 50 MB and skips duplicates (`Entry::same_login`: title + username + password + url), assigning fresh ids.

## File formats (little-endian; the whole header is the AEAD's AAD)

- **Vault** `%APPDATA%\com.phumoonlight.accountvault\vault.avault`: `"AVLT"` | ver u8 | nonce[24] | ciphertext+tag. Plaintext is JSON `VaultData`.
- **Backup** `*.avbackup`: `"AVBK"` | ver u8 | m_cost u32 | t_cost u32 | p_cost u32 | salt[16] | nonce[24] | ciphertext+tag.

Changing a format means bumping its version byte and keeping old versions readable.

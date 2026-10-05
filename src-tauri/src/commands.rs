//! Tauri commands exposed to the frontend. The frontend never sees the key or
//! the vault file; it only gets entry data.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use tauri::State;
use zeroize::Zeroizing;

use crate::clipboard::ClipboardGuard;
use crate::vault::{
    backup,
    keystore::OsKeyStore,
    model::{Entry, EntryInput, EntrySummary, VaultData},
    ImportSummary, Result, Vault, VaultError,
};

/// Refuse to load absurdly large files into memory on import.
const MAX_IMPORT_BYTES: u64 = 50 * 1024 * 1024;

pub struct AppState {
    vault_path: PathBuf,
    keys: OsKeyStore,
    vault: Mutex<Option<Vault>>,
}

impl AppState {
    pub fn new(vault_path: PathBuf, keys: OsKeyStore) -> Self {
        Self {
            vault_path,
            keys,
            vault: Mutex::new(None),
        }
    }

    /// Opens the vault on first use, so an error (e.g. a missing key) reaches
    /// the UI as a command error instead of crashing the app at startup.
    fn with_vault<T>(&self, f: impl FnOnce(&mut Vault) -> Result<T>) -> Result<T> {
        let mut guard = self.vault.lock().unwrap();
        if guard.is_none() {
            *guard = Some(Vault::open(&self.vault_path, &self.keys)?);
        }
        f(guard.as_mut().unwrap())
    }
}

#[tauri::command]
pub fn list_entries(state: State<'_, AppState>) -> Result<Vec<EntrySummary>> {
    state.with_vault(|v| Ok(v.list_entries()))
}

#[tauri::command]
pub fn get_entry(id: String, state: State<'_, AppState>) -> Result<Entry> {
    state.with_vault(|v| v.get_entry(&id))
}

#[tauri::command]
pub fn add_entry(input: EntryInput, state: State<'_, AppState>) -> Result<Entry> {
    state.with_vault(|v| v.add_entry(&input))
}

#[tauri::command]
pub fn update_entry(id: String, input: EntryInput, state: State<'_, AppState>) -> Result<Entry> {
    state.with_vault(|v| v.update_entry(&id, &input))
}

#[tauri::command]
pub fn delete_entry(id: String, state: State<'_, AppState>) -> Result<()> {
    state.with_vault(|v| v.delete_entry(&id))
}

/// Copies `text`; for a secret, returns the seconds until it's cleared again.
#[tauri::command]
pub fn copy_to_clipboard(
    text: String,
    secret: bool,
    clipboard: State<'_, ClipboardGuard>,
) -> Result<Option<u64>> {
    let text = Zeroizing::new(text);
    clipboard
        .copy(&text, secret)
        .map(|delay| delay.map(|d| d.as_secs()))
        .map_err(VaultError::Clipboard)
}

/// True while the "close app?" warning is showing.
#[derive(Default)]
pub struct CloseWarning(pub std::sync::atomic::AtomicBool);

/// The user chose "Keep open": warn again on the next close.
#[tauri::command]
pub fn cancel_close(warning: State<'_, CloseWarning>) {
    warning.0.store(false, std::sync::atomic::Ordering::SeqCst);
}

/// Closes the window after the user confirmed the close warning. `destroy`
/// skips `CloseRequested`, and the exit handler then clears the clipboard.
#[tauri::command]
pub fn close_app(window: tauri::Window) -> std::result::Result<(), String> {
    window.destroy().map_err(|e| e.to_string())
}

// ---------- Export / import ----------
// Argon2 commands are async so the ~1s key derivation runs off the UI thread,
// and it runs outside the vault lock so other commands aren't blocked.

#[tauri::command]
pub async fn export_backup(
    path: PathBuf,
    pin: String,
    state: State<'_, AppState>,
) -> Result<usize> {
    let pin = Zeroizing::new(pin);
    if !backup::is_valid_pin(&pin) {
        return Err(VaultError::InvalidPin);
    }
    let data = state.with_vault(|v| {
        Ok(VaultData {
            entries: v.entries().to_vec(),
        })
    })?;
    let file = backup::encrypt(&data, &pin, backup::KdfParams::DEFAULT)?;
    fs::write(&path, file)?;
    Ok(data.entries.len())
}

#[tauri::command]
pub async fn import_backup(
    path: PathBuf,
    pin: String,
    state: State<'_, AppState>,
) -> Result<ImportSummary> {
    let pin = Zeroizing::new(pin);
    let mut data = backup::decrypt(&read_import_file(&path)?, &pin)?;
    let entries = std::mem::take(&mut data.entries);
    state.with_vault(|v| v.import_entries(entries))
}

fn read_import_file(path: &Path) -> Result<Vec<u8>> {
    if fs::metadata(path)?.len() > MAX_IMPORT_BYTES {
        return Err(VaultError::FileTooLarge);
    }
    Ok(fs::read(path)?)
}

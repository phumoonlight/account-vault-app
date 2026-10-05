mod clipboard;
mod commands;
mod vault;

use std::sync::atomic::Ordering;

use tauri::{Emitter, Manager, RunEvent, WindowEvent};

use clipboard::{ClipboardGuard, SystemClipboard};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let vault_path = app.path().app_data_dir()?.join("vault.avault");
            let keys = vault::keystore::OsKeyStore::new(&app.config().identifier, "vault-key");
            app.manage(commands::AppState::new(vault_path, keys));
            app.manage(ClipboardGuard::new(SystemClipboard, clipboard::CLEAR_AFTER));
            app.manage(commands::CloseWarning::default());
            Ok(())
        })
        .on_window_event(|window, event| {
            // Ask before closing if something copied from the app is still on
            // the clipboard, since closing clears it. The UI confirms via
            // `close_app`. A second close while the warning is pending goes
            // through, so a broken UI can never make the window unclosable.
            if let WindowEvent::CloseRequested { api, .. } = event {
                let pending = &window.state::<commands::CloseWarning>().0;
                if let Some(secret) = window.state::<ClipboardGuard>().still_on_clipboard() {
                    if !pending.swap(true, Ordering::SeqCst) {
                        api.prevent_close();
                        let _ =
                            window.emit("close-requested", serde_json::json!({ "secret": secret }));
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_entries,
            commands::get_entry,
            commands::add_entry,
            commands::update_entry,
            commands::delete_entry,
            commands::export_backup,
            commands::import_backup,
            commands::copy_to_clipboard,
            commands::close_app,
            commands::cancel_close,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // Don't leave anything copied from the app behind when it closes.
            if let RunEvent::Exit = event {
                app.state::<ClipboardGuard>().clear_if_ours();
            }
        });
}

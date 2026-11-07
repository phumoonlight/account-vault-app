import { invoke } from "@tauri-apps/api/core";

// Mirrors src-tauri/src/vault/model.rs (serde camelCase).

export interface CustomField {
  name: string;
  value: string;
  secret: boolean;
}

/** "env" entries hold dotenv text in `env` instead of login fields. */
export type EntryKind = "account" | "env";

export interface EntryInput {
  kind: EntryKind;
  title: string;
  username: string;
  password: string;
  url: string;
  notes: string;
  customFields: CustomField[];
  /** Dotenv text; only used when kind is "env". */
  env: string;
  favorite: boolean;
  tags: string[];
  /** The folder the entry is in, or null for none. */
  folderId: string | null;
}

export interface Entry extends EntryInput {
  id: string;
  createdAt: number;
  updatedAt: number;
}

export interface EntrySummary {
  id: string;
  kind: EntryKind;
  title: string;
  username: string;
  url: string;
  favorite: boolean;
  tags: string[];
  folderId: string | null;
  updatedAt: number;
}

export interface Folder {
  id: string;
  name: string;
  createdAt: number;
}

/** Mirrors MAX_FOLDER_NAME_LEN in model.rs. */
export const MAX_FOLDER_NAME_LEN = 64;

export interface ImportSummary {
  added: number;
  /** Identical to an entry already in the vault. */
  skipped: number;
}

// Rust command errors arrive as plain message strings.
export const api = {
  listEntries: () => invoke<EntrySummary[]>("list_entries"),
  getEntry: (id: string) => invoke<Entry>("get_entry", { id }),
  addEntry: (input: EntryInput) => invoke<Entry>("add_entry", { input }),
  updateEntry: (id: string, input: EntryInput) => invoke<Entry>("update_entry", { id, input }),
  deleteEntry: (id: string) => invoke<void>("delete_entry", { id }),
  /** Puts an entry in a folder, or in no folder when `folderId` is null. */
  moveEntry: (id: string, folderId: string | null) =>
    invoke<void>("move_entry", { id, folderId }),
  listFolders: () => invoke<Folder[]>("list_folders"),
  addFolder: (name: string) => invoke<Folder>("add_folder", { name }),
  renameFolder: (id: string, name: string) => invoke<Folder>("rename_folder", { id, name }),
  /** Deletes the folder only; its entries are kept, in no folder. */
  deleteFolder: (id: string) => invoke<void>("delete_folder", { id }),
  /** Returns the number of entries exported. */
  /** For a secret, returns the seconds until the clipboard is cleared again. */
  copyToClipboard: (text: string, secret: boolean) =>
    invoke<number | null>("copy_to_clipboard", { text, secret }),
  /** Closes the app after the user confirmed the close warning. */
  closeApp: () => invoke<void>("close_app"),
  /** The user dismissed the close warning; warn again next time. */
  cancelClose: () => invoke<void>("cancel_close"),
  exportBackup: (path: string, pin: string) => invoke<number>("export_backup", { path, pin }),
  importBackup: (path: string, pin: string) =>
    invoke<ImportSummary>("import_backup", { path, pin }),
};

export function emptyInput(kind: EntryKind = "account"): EntryInput {
  return {
    kind,
    title: "",
    username: "",
    password: "",
    url: "",
    notes: "",
    customFields: [],
    env: "",
    favorite: false,
    tags: [],
    folderId: null,
  };
}

import { invoke } from "@tauri-apps/api/core";

// Mirrors src-tauri/src/vault/model.rs (serde camelCase).

export interface CustomField {
  name: string;
  value: string;
  secret: boolean;
}

export interface EntryInput {
  title: string;
  username: string;
  password: string;
  url: string;
  notes: string;
  customFields: CustomField[];
  favorite: boolean;
  tags: string[];
}

export interface Entry extends EntryInput {
  id: string;
  createdAt: number;
  updatedAt: number;
}

export interface EntrySummary {
  id: string;
  title: string;
  username: string;
  url: string;
  favorite: boolean;
  tags: string[];
  updatedAt: number;
}

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
  /** Returns the number of entries exported. */
  exportBackup: (path: string, pin: string) => invoke<number>("export_backup", { path, pin }),
  importBackup: (path: string, pin: string) =>
    invoke<ImportSummary>("import_backup", { path, pin }),
};

export function emptyInput(): EntryInput {
  return {
    title: "",
    username: "",
    password: "",
    url: "",
    notes: "",
    customFields: [],
    favorite: false,
    tags: [],
  };
}

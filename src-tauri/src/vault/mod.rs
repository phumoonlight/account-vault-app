pub mod backup;
pub mod crypto;
pub mod error;
pub mod keystore;
pub mod model;

use serde::Serialize;
use std::{
    collections::HashMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use zeroize::Zeroizing;

use crypto::Key;
pub use error::{Result, VaultError};
use keystore::KeyStore;
use model::{normalize_folder_name, Entry, EntryInput, EntrySummary, Folder, VaultData};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub added: usize,
    /// Entries identical (title, username, password, url) to one already in the vault.
    pub skipped: usize,
}

/// An opened vault. The key and decrypted data are wiped from memory when
/// this is dropped.
pub struct Vault {
    path: PathBuf,
    key: Key,
    data: VaultData,
}

impl Vault {
    /// Opens the vault at `path` using the key from `keys`. If no vault file
    /// exists yet, creates an empty one with a new random key.
    pub fn open(path: &Path, keys: &dyn KeyStore) -> Result<Self> {
        let file = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Self::create(path, keys),
            Err(e) => return Err(e.into()),
        };
        let key = keys.load()?.ok_or(VaultError::KeyMissing)?;
        let plaintext = crypto::open(&key, &file)?;
        let mut data: VaultData = serde_json::from_slice(&plaintext)?;
        data.entries.iter_mut().for_each(Entry::migrate);
        drop_missing_folder_ids(&mut data);
        Ok(Self {
            path: path.to_path_buf(),
            key,
            data,
        })
    }

    fn create(path: &Path, keys: &dyn KeyStore) -> Result<Self> {
        let key = crypto::generate_key();
        keys.save(&key)?;
        let vault = Self {
            path: path.to_path_buf(),
            key,
            data: VaultData::default(),
        };
        vault.save()?;
        Ok(vault)
    }

    /// Re-encrypts and writes the vault. Writes and flushes a temp file, then
    /// renames it over the vault, so a crash or power loss mid-save leaves
    /// either the old or the new vault intact, never a half-written one.
    fn save(&self) -> Result<()> {
        let plaintext = Zeroizing::new(serde_json::to_vec(&self.data)?);
        let file = crypto::seal(&self.key, &plaintext)?;
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("tmp");
        {
            let mut out = fs::File::create(&tmp)?;
            out.write_all(&file)?;
            // Without this, the rename can reach the disk before the data does.
            out.sync_all()?;
        }
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    /// Applies `change` and saves; if saving fails, the in-memory data is rolled back.
    fn mutate<T>(&mut self, change: impl FnOnce(&mut VaultData) -> Result<T>) -> Result<T> {
        let backup = self.data.clone();
        let result = change(&mut self.data).and_then(|value| self.save().map(|()| value));
        if result.is_err() {
            self.data = backup;
        }
        result
    }

    pub fn entries(&self) -> &[Entry] {
        &self.data.entries
    }

    pub fn folders(&self) -> &[Folder] {
        &self.data.folders
    }

    /// Adds `entries` with fresh ids, skipping any that duplicate an existing
    /// entry. Each of `folders` is merged into the vault's folder with the
    /// same name (ignoring case), or added; entries keep their folder.
    pub fn import_entries(
        &mut self,
        entries: Vec<Entry>,
        folders: Vec<Folder>,
    ) -> Result<ImportSummary> {
        let now = now_ms();
        self.mutate(|data| {
            // Imported folder id -> folder id in this vault.
            let mut folder_ids: HashMap<String, String> = HashMap::new();
            for folder in &folders {
                let name = normalize_folder_name(&folder.name);
                if name.is_empty() {
                    continue;
                }
                let id = match data.folders.iter().find(|f| same_name(&f.name, &name)) {
                    Some(existing) => existing.id.clone(),
                    None => {
                        let created_at = if folder.created_at == 0 {
                            now
                        } else {
                            folder.created_at
                        };
                        let added = new_folder(name, created_at);
                        let id = added.id.clone();
                        data.folders.push(added);
                        id
                    }
                };
                folder_ids.insert(folder.id.clone(), id);
            }

            let mut summary = ImportSummary {
                added: 0,
                skipped: 0,
            };
            for mut entry in entries {
                entry.migrate();
                entry.folder_id = entry
                    .folder_id
                    .as_ref()
                    .and_then(|id| folder_ids.get(id).cloned());
                if data.entries.iter().any(|e| e.same_login(&entry)) {
                    summary.skipped += 1;
                    continue;
                }
                entry.id = uuid::Uuid::new_v4().to_string();
                if entry.created_at == 0 {
                    entry.created_at = now;
                }
                if entry.updated_at == 0 {
                    entry.updated_at = now;
                }
                data.entries.push(entry);
                summary.added += 1;
            }
            Ok(summary)
        })
    }

    pub fn list_entries(&self) -> Vec<EntrySummary> {
        self.data.entries.iter().map(Entry::summary).collect()
    }

    pub fn get_entry(&self, id: &str) -> Result<Entry> {
        self.data
            .entries
            .iter()
            .find(|e| e.id == id)
            .cloned()
            .ok_or(VaultError::EntryNotFound)
    }

    pub fn add_entry(&mut self, input: &EntryInput) -> Result<Entry> {
        let entry = Entry::new(uuid::Uuid::new_v4().to_string(), input, now_ms());
        self.mutate(|data| {
            check_folder(data, input.folder_id.as_deref())?;
            data.entries.push(entry.clone());
            Ok(entry)
        })
    }

    pub fn update_entry(&mut self, id: &str, input: &EntryInput) -> Result<Entry> {
        self.mutate(|data| {
            check_folder(data, input.folder_id.as_deref())?;
            let entry = data
                .entries
                .iter_mut()
                .find(|e| e.id == id)
                .ok_or(VaultError::EntryNotFound)?;
            entry.apply(input, now_ms());
            Ok(entry.clone())
        })
    }

    pub fn delete_entry(&mut self, id: &str) -> Result<()> {
        self.mutate(|data| {
            let index = data
                .entries
                .iter()
                .position(|e| e.id == id)
                .ok_or(VaultError::EntryNotFound)?;
            data.entries.remove(index);
            Ok(())
        })
    }

    /// Puts an entry in a folder, or in no folder. Doesn't change `updated_at`,
    /// since the entry's contents stay the same.
    pub fn move_entry(&mut self, id: &str, folder_id: Option<&str>) -> Result<()> {
        self.mutate(|data| {
            check_folder(data, folder_id)?;
            let entry = data
                .entries
                .iter_mut()
                .find(|e| e.id == id)
                .ok_or(VaultError::EntryNotFound)?;
            entry.folder_id = folder_id.map(str::to_owned);
            Ok(())
        })
    }

    pub fn add_folder(&mut self, name: &str) -> Result<Folder> {
        let name = normalize_folder_name(name);
        self.mutate(|data| {
            check_folder_name(data, &name, None)?;
            let folder = new_folder(name, now_ms());
            data.folders.push(folder.clone());
            Ok(folder)
        })
    }

    pub fn rename_folder(&mut self, id: &str, name: &str) -> Result<Folder> {
        let name = normalize_folder_name(name);
        self.mutate(|data| {
            check_folder_name(data, &name, Some(id))?;
            let folder = data
                .folders
                .iter_mut()
                .find(|f| f.id == id)
                .ok_or(VaultError::FolderNotFound)?;
            folder.name = name;
            Ok(folder.clone())
        })
    }

    /// Deletes the folder only; its entries move out of it and are kept.
    pub fn delete_folder(&mut self, id: &str) -> Result<()> {
        self.mutate(|data| {
            let index = data
                .folders
                .iter()
                .position(|f| f.id == id)
                .ok_or(VaultError::FolderNotFound)?;
            data.folders.remove(index);
            for entry in &mut data.entries {
                if entry.folder_id.as_deref() == Some(id) {
                    entry.folder_id = None;
                }
            }
            Ok(())
        })
    }
}

fn new_folder(name: String, created_at: u64) -> Folder {
    Folder {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        created_at,
    }
}

fn same_name(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

/// `None` (no folder) is always valid.
fn check_folder(data: &VaultData, folder_id: Option<&str>) -> Result<()> {
    match folder_id {
        Some(id) if !data.folders.iter().any(|f| f.id == id) => Err(VaultError::FolderNotFound),
        _ => Ok(()),
    }
}

/// `name` must already be normalized. `except` is the folder being renamed.
fn check_folder_name(data: &VaultData, name: &str, except: Option<&str>) -> Result<()> {
    if name.is_empty() {
        return Err(VaultError::FolderNameEmpty);
    }
    if data
        .folders
        .iter()
        .any(|f| Some(f.id.as_str()) != except && same_name(&f.name, name))
    {
        return Err(VaultError::FolderExists);
    }
    Ok(())
}

/// Takes entries out of folders that no longer exist.
fn drop_missing_folder_ids(data: &mut VaultData) {
    let folders = &data.folders;
    for entry in &mut data.entries {
        if let Some(id) = &entry.folder_id {
            if !folders.iter().any(|f| &f.id == id) {
                entry.folder_id = None;
            }
        }
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    #[derive(Default)]
    struct MemoryKeyStore(Mutex<Option<[u8; crypto::KEY_LEN]>>);

    impl KeyStore for MemoryKeyStore {
        fn load(&self) -> Result<Option<Key>> {
            Ok(self.0.lock().unwrap().map(Zeroizing::new))
        }
        fn save(&self, key: &Key) -> Result<()> {
            *self.0.lock().unwrap() = Some(**key);
            Ok(())
        }
    }

    fn sample_input() -> EntryInput {
        let mut input = EntryInput::default();
        input.title = "GitHub".into();
        input.username = "octocat".into();
        input.password = "hunter2".into();
        input
    }

    #[test]
    fn first_open_creates_vault_and_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.avault");
        let keys = MemoryKeyStore::default();

        let vault = Vault::open(&path, &keys).unwrap();
        assert!(path.exists());
        assert!(keys.load().unwrap().is_some());
        assert!(vault.list_entries().is_empty());
    }

    #[test]
    fn entries_persist_across_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.avault");
        let keys = MemoryKeyStore::default();

        let mut vault = Vault::open(&path, &keys).unwrap();
        let added = vault.add_entry(&sample_input()).unwrap();
        drop(vault);

        let vault = Vault::open(&path, &keys).unwrap();
        let entry = vault.get_entry(&added.id).unwrap();
        assert_eq!(entry.title, "GitHub");
        assert_eq!(entry.password, "hunter2");
        assert_eq!(vault.list_entries().len(), 1);
    }

    #[test]
    fn file_does_not_contain_plaintext() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.avault");
        let mut vault = Vault::open(&path, &MemoryKeyStore::default()).unwrap();
        vault.add_entry(&sample_input()).unwrap();

        let bytes = fs::read(&path).unwrap();
        let haystack = String::from_utf8_lossy(&bytes);
        assert!(!haystack.contains("hunter2"));
        assert!(!haystack.contains("octocat"));
    }

    #[test]
    fn different_key_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.avault");
        Vault::open(&path, &MemoryKeyStore::default()).unwrap();

        let other = MemoryKeyStore::default();
        other.save(&crypto::generate_key()).unwrap();
        assert!(matches!(
            Vault::open(&path, &other),
            Err(VaultError::Decrypt)
        ));
    }

    #[test]
    fn missing_key_is_reported_not_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.avault");
        Vault::open(&path, &MemoryKeyStore::default()).unwrap();

        let empty = MemoryKeyStore::default();
        assert!(matches!(
            Vault::open(&path, &empty),
            Err(VaultError::KeyMissing)
        ));
        assert!(empty.load().unwrap().is_none());
    }

    #[test]
    fn tampered_file_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.avault");
        let keys = MemoryKeyStore::default();
        let mut vault = Vault::open(&path, &keys).unwrap();
        vault.add_entry(&sample_input()).unwrap();

        let mut bytes = fs::read(&path).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 0x01;
        fs::write(&path, &bytes).unwrap();

        assert!(matches!(
            Vault::open(&path, &keys),
            Err(VaultError::Decrypt)
        ));
    }

    #[test]
    fn legacy_email_is_merged_into_username() {
        let json = r#"{"entries":[
            {"id":"a","title":"A","username":"","email":"a@x.com","createdAt":0,"updatedAt":0},
            {"id":"b","title":"B","username":"bob","email":"b@x.com","createdAt":0,"updatedAt":0}
        ]}"#;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.avault");
        let keys = MemoryKeyStore::default();
        let key = crypto::generate_key();
        keys.save(&key).unwrap();
        fs::write(&path, crypto::seal(&key, json.as_bytes()).unwrap()).unwrap();

        let vault = Vault::open(&path, &keys).unwrap();
        assert_eq!(vault.get_entry("a").unwrap().username, "a@x.com");
        let b = vault.get_entry("b").unwrap();
        assert_eq!(b.username, "bob");
        assert_eq!(b.custom_fields[0].name, "Email");
        assert_eq!(b.custom_fields[0].value, "b@x.com");
    }

    #[test]
    fn import_skips_duplicates_and_assigns_new_ids() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.avault");
        let mut vault = Vault::open(&path, &MemoryKeyStore::default()).unwrap();
        let existing = vault.add_entry(&sample_input()).unwrap();

        let mut other = sample_input();
        other.title = "Other".into();
        let incoming = vec![
            existing.clone(),
            Entry::new(String::new(), &other, 0),
            Entry::new(String::new(), &other, 0),
        ];
        let summary = vault.import_entries(incoming, Vec::new()).unwrap();
        assert_eq!((summary.added, summary.skipped), (1, 2));

        let ids: Vec<_> = vault.entries().iter().map(|e| e.id.clone()).collect();
        assert_eq!(ids.len(), 2);
        assert!(!ids[1].is_empty() && ids[1] != existing.id);
        assert!(vault.entries()[1].created_at > 0);
    }

    #[test]
    fn update_and_delete_entry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.avault");
        let mut vault = Vault::open(&path, &MemoryKeyStore::default()).unwrap();
        let id = vault.add_entry(&sample_input()).unwrap().id.clone();

        let mut changed = sample_input();
        changed.password = "correct horse".into();
        vault.update_entry(&id, &changed).unwrap();
        assert_eq!(vault.get_entry(&id).unwrap().password, "correct horse");

        vault.delete_entry(&id).unwrap();
        assert!(matches!(
            vault.get_entry(&id),
            Err(VaultError::EntryNotFound)
        ));
        assert!(matches!(
            vault.delete_entry(&id),
            Err(VaultError::EntryNotFound)
        ));
    }

    #[test]
    fn folders_rename_and_delete_keeps_entries() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.avault");
        let keys = MemoryKeyStore::default();
        let mut vault = Vault::open(&path, &keys).unwrap();

        let work = vault.add_folder("  Work ").unwrap();
        assert_eq!(work.name, "Work");
        assert!(matches!(
            vault.add_folder("work"),
            Err(VaultError::FolderExists)
        ));
        assert!(matches!(
            vault.add_folder("   "),
            Err(VaultError::FolderNameEmpty)
        ));
        // Renaming to its own name with different case is fine.
        assert_eq!(vault.rename_folder(&work.id, "WORK").unwrap().name, "WORK");

        let mut input = sample_input();
        input.folder_id = Some(work.id.clone());
        let id = vault.add_entry(&input).unwrap().id.clone();
        let updated_at = vault.get_entry(&id).unwrap().updated_at;
        drop(vault);

        let mut vault = Vault::open(&path, &keys).unwrap();
        assert_eq!(vault.folders().len(), 1);
        assert_eq!(vault.list_entries()[0].folder_id, Some(work.id.clone()));

        vault.move_entry(&id, None).unwrap();
        assert_eq!(vault.get_entry(&id).unwrap().folder_id, None);
        assert_eq!(vault.get_entry(&id).unwrap().updated_at, updated_at);
        vault.move_entry(&id, Some(&work.id)).unwrap();

        vault.delete_folder(&work.id).unwrap();
        assert!(vault.folders().is_empty());
        assert_eq!(vault.get_entry(&id).unwrap().folder_id, None);
    }

    #[test]
    fn unknown_folder_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.avault");
        let mut vault = Vault::open(&path, &MemoryKeyStore::default()).unwrap();
        let id = vault.add_entry(&sample_input()).unwrap().id.clone();

        assert!(matches!(
            vault.move_entry(&id, Some("nope")),
            Err(VaultError::FolderNotFound)
        ));
        let mut input = sample_input();
        input.folder_id = Some("nope".into());
        assert!(matches!(
            vault.add_entry(&input),
            Err(VaultError::FolderNotFound)
        ));
        assert_eq!(vault.entries().len(), 1);
    }

    #[test]
    fn missing_folder_ids_are_dropped_on_open() {
        let json = r#"{"entries":[
            {"id":"a","title":"A","folderId":"gone","createdAt":0,"updatedAt":0},
            {"id":"b","title":"B","folderId":"f","createdAt":0,"updatedAt":0}
        ],"folders":[{"id":"f","name":"Work"}]}"#;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.avault");
        let keys = MemoryKeyStore::default();
        let key = crypto::generate_key();
        keys.save(&key).unwrap();
        fs::write(&path, crypto::seal(&key, json.as_bytes()).unwrap()).unwrap();

        let vault = Vault::open(&path, &keys).unwrap();
        assert_eq!(vault.get_entry("a").unwrap().folder_id, None);
        assert_eq!(
            vault.get_entry("b").unwrap().folder_id.as_deref(),
            Some("f")
        );
    }

    #[test]
    fn import_merges_folders_by_name() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.avault");
        let mut vault = Vault::open(&path, &MemoryKeyStore::default()).unwrap();
        let work = vault.add_folder("Work").unwrap();

        let folders = vec![new_folder("work".into(), 5), new_folder("Home".into(), 0)];
        let mut a = Entry::new(String::new(), &sample_input(), 1);
        a.folder_id = Some(folders[0].id.clone());
        let mut other = sample_input();
        other.title = "Other".into();
        let mut b = Entry::new(String::new(), &other, 1);
        b.folder_id = Some(folders[1].id.clone());
        let mut c = Entry::new(String::new(), &other, 1);
        c.title = "Third".into();
        c.folder_id = Some("not-in-backup".into());

        let summary = vault.import_entries(vec![a, b, c], folders).unwrap();
        assert_eq!((summary.added, summary.skipped), (3, 0));
        assert_eq!(vault.folders().len(), 2);
        let home = vault.folders().iter().find(|f| f.name == "Home").unwrap();
        assert!(home.created_at > 0);

        let entries = vault.entries();
        assert_eq!(entries[0].folder_id.as_ref(), Some(&work.id));
        assert_eq!(entries[1].folder_id.as_ref(), Some(&home.id));
        assert_eq!(entries[2].folder_id, None);
    }
}

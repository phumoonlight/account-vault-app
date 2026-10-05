pub mod backup;
pub mod crypto;
pub mod error;
pub mod keystore;
pub mod model;

use serde::Serialize;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use zeroize::Zeroizing;

use crypto::Key;
pub use error::{Result, VaultError};
use keystore::KeyStore;
use model::{Entry, EntryInput, EntrySummary, VaultData};

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

    /// Adds `entries` with fresh ids, skipping any that duplicate an existing entry.
    pub fn import_entries(&mut self, entries: Vec<Entry>) -> Result<ImportSummary> {
        let now = now_ms();
        self.mutate(|data| {
            let mut summary = ImportSummary {
                added: 0,
                skipped: 0,
            };
            for mut entry in entries {
                entry.migrate();
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
            data.entries.push(entry.clone());
            Ok(entry)
        })
    }

    pub fn update_entry(&mut self, id: &str, input: &EntryInput) -> Result<Entry> {
        self.mutate(|data| {
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
        let summary = vault.import_entries(incoming).unwrap();
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
}

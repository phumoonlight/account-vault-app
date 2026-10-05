use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error("the vault file is damaged or was encrypted with a different key")]
    Decrypt,
    #[error("not a valid vault file")]
    Corrupt,
    #[error("unsupported vault file version {0}")]
    UnsupportedVersion(u8),
    #[error("the vault's encryption key is missing from the system credential store")]
    KeyMissing,
    #[error("the stored encryption key is invalid")]
    KeyInvalid,
    #[error("system credential store error: {0}")]
    Keyring(String),
    #[error("PIN must be 4 to 6 digits")]
    InvalidPin,
    #[error("not an Account Vault backup file")]
    NotABackup,
    #[error("incorrect PIN, or the backup file is damaged")]
    WrongPin,
    #[error("file is too large to import")]
    FileTooLarge,
    #[error("couldn't use the clipboard: {0}")]
    Clipboard(String),
    #[error("entry not found")]
    EntryNotFound,
    #[error("encryption failed")]
    Crypto,
    #[error("file error: {0}")]
    Io(#[from] std::io::Error),
    #[error("data error: {0}")]
    Serde(#[from] serde_json::Error),
}

// Tauri sends command errors to the frontend as JSON; a plain message string is enough.
impl Serialize for VaultError {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, VaultError>;

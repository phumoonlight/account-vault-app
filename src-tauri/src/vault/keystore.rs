//! Where the vault's encryption key lives. In the app this is the OS credential
//! store (Windows Credential Manager, macOS Keychain, Linux Secret Service),
//! which only the logged-in OS user can read.

use zeroize::Zeroizing;

use super::crypto::{Key, KEY_LEN};
use super::error::{Result, VaultError};

pub trait KeyStore: Send + Sync {
    /// Returns `None` if no key has been stored yet.
    fn load(&self) -> Result<Option<Key>>;
    fn save(&self, key: &Key) -> Result<()>;
}

pub struct OsKeyStore {
    service: String,
    user: String,
}

impl OsKeyStore {
    pub fn new(service: &str, user: &str) -> Self {
        Self {
            service: service.to_owned(),
            user: user.to_owned(),
        }
    }

    fn entry(&self) -> Result<keyring::Entry> {
        keyring::Entry::new(&self.service, &self.user).map_err(keyring_error)
    }
}

impl KeyStore for OsKeyStore {
    fn load(&self) -> Result<Option<Key>> {
        match self.entry()?.get_secret() {
            Ok(bytes) => {
                let bytes = Zeroizing::new(bytes);
                let key: [u8; KEY_LEN] =
                    bytes[..].try_into().map_err(|_| VaultError::KeyInvalid)?;
                Ok(Some(Zeroizing::new(key)))
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(keyring_error(e)),
        }
    }

    fn save(&self, key: &Key) -> Result<()> {
        self.entry()?.set_secret(&key[..]).map_err(keyring_error)
    }
}

fn keyring_error(e: keyring::Error) -> VaultError {
    VaultError::Keyring(e.to_string())
}

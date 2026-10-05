//! PIN-protected backup file (`.avbackup`), portable to any machine.
//!
//! ```text
//! offset  size  field
//! 0       4     magic "AVBK"
//! 4       1     format version
//! 5       4     argon2id m_cost (KiB, little-endian)
//! 9       4     argon2id t_cost
//! 13      4     argon2id p_cost
//! 17      16    salt
//! 33      24    XChaCha20-Poly1305 nonce
//! 57      ..    ciphertext + 16-byte tag (JSON of `VaultData`)
//! ```
//!
//! The header is passed as AAD, so tampering with any of it fails decryption.

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::{
    aead::{Aead, Generate, KeyInit, Payload},
    XChaCha20Poly1305, XNonce,
};
use zeroize::Zeroizing;

use super::error::{Result, VaultError};
use super::model::VaultData;

const MAGIC: &[u8; 4] = b"AVBK";
const FORMAT_VERSION: u8 = 1;
const SALT_LEN: usize = 16;
const HEADER_LEN: usize = 4 + 1 + 4 * 3 + SALT_LEN + 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KdfParams {
    pub m_cost_kib: u32,
    pub t_cost: u32,
    pub p_cost: u32,
}

impl KdfParams {
    /// Argon2id with 256 MiB memory, 12 passes, 4 lanes (~1 s per attempt).
    /// Deliberately expensive: a 4-6 digit PIN has at most a million values,
    /// so the cost per guess is what keeps offline brute-forcing slow.
    pub const DEFAULT: Self = Self {
        m_cost_kib: 256 * 1024,
        t_cost: 12,
        p_cost: 4,
    };

    /// Upper bounds accepted when reading, so a crafted file can't make
    /// importing allocate gigabytes or spin forever.
    fn is_reasonable(&self) -> bool {
        self.m_cost_kib <= 1024 * 1024 && self.t_cost <= 20 && self.p_cost <= 16
    }
}

fn derive_key(pin: &str, salt: &[u8], kdf: KdfParams) -> Result<Zeroizing<[u8; 32]>> {
    let params = Params::new(kdf.m_cost_kib, kdf.t_cost, kdf.p_cost, Some(32))
        .map_err(|_| VaultError::NotABackup)?;
    let mut key = Zeroizing::new([0u8; 32]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(pin.as_bytes(), salt, &mut key[..])
        .map_err(|_| VaultError::NotABackup)?;
    Ok(key)
}

pub fn is_valid_pin(pin: &str) -> bool {
    (4..=6).contains(&pin.len()) && pin.bytes().all(|b| b.is_ascii_digit())
}

pub fn encrypt(data: &VaultData, pin: &str, kdf: KdfParams) -> Result<Vec<u8>> {
    if !is_valid_pin(pin) {
        return Err(VaultError::InvalidPin);
    }
    let salt = <[u8; SALT_LEN]>::generate();
    let nonce = XNonce::generate();
    let key = derive_key(pin, &salt, kdf)?;

    let mut header = [0u8; HEADER_LEN];
    header[0..4].copy_from_slice(MAGIC);
    header[4] = FORMAT_VERSION;
    header[5..9].copy_from_slice(&kdf.m_cost_kib.to_le_bytes());
    header[9..13].copy_from_slice(&kdf.t_cost.to_le_bytes());
    header[13..17].copy_from_slice(&kdf.p_cost.to_le_bytes());
    header[17..33].copy_from_slice(&salt);
    header[33..57].copy_from_slice(&nonce);

    let plaintext = Zeroizing::new(serde_json::to_vec(data)?);
    let cipher = XChaCha20Poly1305::new_from_slice(&key[..]).map_err(|_| VaultError::Crypto)?;
    let ciphertext = cipher
        .encrypt(
            &nonce,
            Payload {
                msg: &plaintext,
                aad: &header,
            },
        )
        .map_err(|_| VaultError::Crypto)?;

    let mut out = Vec::with_capacity(HEADER_LEN + ciphertext.len());
    out.extend_from_slice(&header);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

pub fn decrypt(file: &[u8], pin: &str) -> Result<VaultData> {
    if file.len() < HEADER_LEN || &file[0..4] != MAGIC {
        return Err(VaultError::NotABackup);
    }
    if file[4] != FORMAT_VERSION {
        return Err(VaultError::UnsupportedVersion(file[4]));
    }
    let u32_at = |i: usize| u32::from_le_bytes(file[i..i + 4].try_into().unwrap());
    let kdf = KdfParams {
        m_cost_kib: u32_at(5),
        t_cost: u32_at(9),
        p_cost: u32_at(13),
    };
    if !kdf.is_reasonable() {
        return Err(VaultError::NotABackup);
    }
    let (header, ciphertext) = file.split_at(HEADER_LEN);
    let key = derive_key(pin, &header[17..33], kdf)?;
    let nonce = XNonce::try_from(&header[33..57]).map_err(|_| VaultError::NotABackup)?;
    let cipher = XChaCha20Poly1305::new_from_slice(&key[..]).map_err(|_| VaultError::Crypto)?;
    let plaintext = cipher
        .decrypt(
            &nonce,
            Payload {
                msg: ciphertext,
                aad: header,
            },
        )
        .map(Zeroizing::new)
        .map_err(|_| VaultError::WrongPin)?;
    Ok(serde_json::from_slice(&plaintext)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_KDF: KdfParams = KdfParams {
        m_cost_kib: 64,
        t_cost: 1,
        p_cost: 1,
    };

    #[test]
    fn roundtrip_and_wrong_pin() {
        let file = encrypt(&VaultData::default(), "1234", TEST_KDF).unwrap();
        assert!(decrypt(&file, "1234").unwrap().entries.is_empty());
        assert!(matches!(decrypt(&file, "4321"), Err(VaultError::WrongPin)));
    }

    #[test]
    fn pin_must_be_4_to_6_digits() {
        for bad in ["", "123", "1234567", "12a4", "12 34"] {
            assert!(matches!(
                encrypt(&VaultData::default(), bad, TEST_KDF),
                Err(VaultError::InvalidPin)
            ));
        }
        for good in ["1234", "12345", "000000"] {
            assert!(encrypt(&VaultData::default(), good, TEST_KDF).is_ok());
        }
    }

    #[test]
    fn rejects_non_backup_and_absurd_params() {
        assert!(matches!(
            decrypt(b"hello", "1234"),
            Err(VaultError::NotABackup)
        ));

        let mut file = encrypt(&VaultData::default(), "1234", TEST_KDF).unwrap();
        file[5..9].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(
            decrypt(&file, "1234"),
            Err(VaultError::NotABackup)
        ));
    }
}

//! Encrypted vault file format.
//!
//! ```text
//! offset  size  field
//! 0       4     magic "AVLT"
//! 4       1     format version
//! 5       24    XChaCha20-Poly1305 nonce
//! 29      ..    ciphertext + 16-byte tag
//! ```
//!
//! The whole header is passed as AAD, so tampering with it makes decryption fail.

use chacha20poly1305::{
    aead::{Aead, Generate, KeyInit, Payload},
    XChaCha20Poly1305, XNonce,
};
use zeroize::Zeroizing;

use super::error::{Result, VaultError};

const MAGIC: &[u8; 4] = b"AVLT";
const FORMAT_VERSION: u8 = 1;
const NONCE_LEN: usize = 24;
const HEADER_LEN: usize = 4 + 1 + NONCE_LEN;
pub const KEY_LEN: usize = 32;

pub type Key = Zeroizing<[u8; KEY_LEN]>;

pub fn generate_key() -> Key {
    Zeroizing::new(<[u8; KEY_LEN]>::generate())
}

fn encode_header(nonce: &XNonce) -> [u8; HEADER_LEN] {
    let mut out = [0u8; HEADER_LEN];
    out[0..4].copy_from_slice(MAGIC);
    out[4] = FORMAT_VERSION;
    out[5..].copy_from_slice(nonce);
    out
}

/// Encrypts `plaintext` with a fresh nonce and returns the complete file bytes.
pub fn seal(key: &Key, plaintext: &[u8]) -> Result<Vec<u8>> {
    let nonce = XNonce::generate();
    let header = encode_header(&nonce);
    let cipher = XChaCha20Poly1305::new_from_slice(&key[..]).map_err(|_| VaultError::Crypto)?;
    let ciphertext = cipher
        .encrypt(
            &nonce,
            Payload {
                msg: plaintext,
                aad: &header,
            },
        )
        .map_err(|_| VaultError::Crypto)?;

    let mut out = Vec::with_capacity(HEADER_LEN + ciphertext.len());
    out.extend_from_slice(&header);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Decrypts complete file bytes. Fails with `Decrypt` if the key is wrong or
/// any byte of the file was modified.
pub fn open(key: &Key, file: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    if file.len() < HEADER_LEN || &file[0..4] != MAGIC {
        return Err(VaultError::Corrupt);
    }
    if file[4] != FORMAT_VERSION {
        return Err(VaultError::UnsupportedVersion(file[4]));
    }
    let (header, ciphertext) = file.split_at(HEADER_LEN);
    let nonce = XNonce::try_from(&header[5..]).map_err(|_| VaultError::Corrupt)?;
    let cipher = XChaCha20Poly1305::new_from_slice(&key[..]).map_err(|_| VaultError::Crypto)?;
    cipher
        .decrypt(
            &nonce,
            Payload {
                msg: ciphertext,
                aad: header,
            },
        )
        .map(Zeroizing::new)
        .map_err(|_| VaultError::Decrypt)
}

use std::collections::BTreeMap;
use std::fmt::Write;

use aes_gcm::Aes256Gcm;
use aes_gcm::Key;
use aes_gcm::Nonce;
use aes_gcm::aead::{Aead, AeadCore, KeyInit, OsRng, Payload};
use base64::{Engine, engine::general_purpose::STANDARD};
use sha2::Digest;
use sha2::Sha256;
use zeroize::Zeroizing;

use crate::error::CryptoError;

const NONCE_LEN: usize = 12; // AES-GCM standard nonce size

pub fn pem_to_der(pem_str: &str) -> Result<Vec<u8>, pem::PemError> {
    let parsed = pem::parse(pem_str)?;
    Ok(parsed.contents().to_vec())
}

#[allow(clippy::expect_used)]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut s = String::with_capacity(digest.len() * 2);
    for b in digest {
        write!(&mut s, "{b:02x}").expect("writing to String never fails");
    }
    s
}

pub fn sha256_hex_pem(pem: &str) -> Option<String> {
    pem_to_der(pem).ok().map(|der| sha256_hex(&der))
}

pub fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        let hi = HEX.get((b >> 4) as usize).copied().unwrap_or(b'0');
        let lo = HEX.get((b & 0xf) as usize).copied().unwrap_or(b'0');
        out.push(hi as char);
        out.push(lo as char);
    }
    out
}

pub fn base64_decode(s: &str) -> Option<Vec<u8>> {
    STANDARD.decode(s).ok()
}

pub fn base64_encode(s: Vec<u8>) -> String {
    STANDARD.encode(s)
}

/// Encrypt a credential for storage.
/// Output blob layout: [12-byte nonce][ciphertext + 16-byte tag].
/// - `master_key`: exactly 32 bytes (from your unsealed key)
/// - `aad`: binds the ciphertext to context — pass `backend_name.as_bytes()`
pub fn encrypt_blob(
    master_key: &[u8],
    plaintext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let key = Key::<Aes256Gcm>::from_slice(master_key);
    let cipher = Aes256Gcm::new(key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // fresh random nonce every call
    let ct = cipher
        .encrypt(
            &nonce,
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| CryptoError::Encrypt)?;
    let mut out = Vec::with_capacity(NONCE_LEN + ct.len());
    out.extend_from_slice(nonce.as_slice());
    out.extend_from_slice(&ct);
    Ok(out)
}

/// Decrypt a blob produced by `encrypt_blob`.
/// `aad` must match what was used on encrypt (the same `backend_name`).
/// Returns zeroizing plaintext that wipes on drop.
pub fn decrypt_blob(
    master_key: &[u8],
    blob: &[u8],
    aad: &[u8],
) -> Result<Zeroizing<Vec<u8>>, CryptoError> {
    if blob.len() < NONCE_LEN {
        return Err(CryptoError::TooShort);
    }
    let (nonce_bytes, ct) = blob.split_at(NONCE_LEN);
    let key = Key::<Aes256Gcm>::from_slice(master_key);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(nonce_bytes);
    let pt = cipher
        .decrypt(nonce, Payload { msg: ct, aad })
        .map_err(|_| CryptoError::Decrypt)?;
    Ok(Zeroizing::new(pt))
}

/// Encrypt a whole credential map: serialize to JSON, then encrypt.
pub fn encrypt_map(
    master_key: &[u8],
    creds: &BTreeMap<String, Zeroizing<String>>,
    aad: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let json = Zeroizing::new(serde_json::to_vec(creds).map_err(|_| CryptoError::Serialize)?);
    encrypt_blob(master_key, &json, aad)
}

/// Decrypt back into the map: decrypt, then deserialize.
pub fn decrypt_map(
    master_key: &[u8],
    blob: &[u8],
    aad: &[u8],
) -> Result<BTreeMap<String, Zeroizing<String>>, CryptoError> {
    let json = decrypt_blob(master_key, blob, aad)?;
    serde_json::from_slice(&json).map_err(|_| CryptoError::Serialize)
}

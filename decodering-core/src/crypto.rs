use std::collections::BTreeMap;
use std::fmt::Write;

use aes_gcm::Aes256Gcm;
use aes_gcm::Nonce;
use aes_gcm::aead::{Aead, Generate, KeyInit, Payload};
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
    let cipher = Aes256Gcm::new_from_slice(master_key).map_err(|_| CryptoError::KeyLength)?;
    let nonce = Nonce::generate();
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
    out.extend_from_slice(&nonce);
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
    let cipher = Aes256Gcm::new_from_slice(master_key).map_err(|_| CryptoError::KeyLength)?;
    let (nonce_bytes, ct) = blob.split_at(NONCE_LEN);
    let nonce: &[u8; NONCE_LEN] = nonce_bytes.try_into().map_err(|_| CryptoError::TooShort)?;
    let pt = cipher
        .decrypt(nonce.into(), Payload { msg: ct, aad })
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use zeroize::Zeroizing;

    use super::CryptoError;
    use super::base64_decode;
    use super::base64_encode;
    use super::decrypt_blob;
    use super::decrypt_map;
    use super::encode_hex;
    use super::encrypt_blob;
    use super::encrypt_map;
    use super::sha256_hex;

    type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

    const KEY: &[u8] = &[7u8; 32];
    const AAD: &[u8] = b"vault-backend-1";
    const PLAINTEXT: &[u8] = b"super-secret-password";

    #[test]
    fn encrypt_decrypt_roundtrip() -> TestResult {
        let blob = encrypt_blob(KEY, PLAINTEXT, AAD)?;
        let recovered = decrypt_blob(KEY, &blob, AAD)?;
        assert_eq!(recovered.as_slice(), PLAINTEXT);
        Ok(())
    }

    #[test]
    fn blob_layout_is_nonce_plus_ciphertext_plus_tag() -> TestResult {
        let blob = encrypt_blob(KEY, PLAINTEXT, AAD)?;
        // [12-byte nonce][ciphertext (== plaintext len for GCM) + 16-byte tag]
        assert_eq!(blob.len(), 12 + PLAINTEXT.len() + 16);
        Ok(())
    }

    #[test]
    fn nonce_is_fresh_per_call() -> TestResult {
        // Same key + plaintext + aad must still produce different blobs,
        // because a random nonce is generated on every encryption.
        let a = encrypt_blob(KEY, PLAINTEXT, AAD)?;
        let b = encrypt_blob(KEY, PLAINTEXT, AAD)?;
        assert_ne!(a, b);
        Ok(())
    }

    #[test]
    fn decrypt_with_wrong_aad_fails() -> TestResult {
        // AAD binds the ciphertext to its backend context; a mismatch must fail.
        let blob = encrypt_blob(KEY, PLAINTEXT, AAD)?;
        let result = decrypt_blob(KEY, &blob, b"different-backend");
        assert!(matches!(result, Err(CryptoError::Decrypt)));
        Ok(())
    }

    #[test]
    fn decrypt_with_wrong_key_fails() -> TestResult {
        let blob = encrypt_blob(KEY, PLAINTEXT, AAD)?;
        let wrong_key: &[u8] = &[9u8; 32];
        let result = decrypt_blob(wrong_key, &blob, AAD);
        assert!(matches!(result, Err(CryptoError::Decrypt)));
        Ok(())
    }

    #[test]
    fn tampered_ciphertext_is_rejected() -> TestResult {
        let mut blob = encrypt_blob(KEY, PLAINTEXT, AAD)?;
        // Flip a bit in the trailing GCM tag; authentication must catch it.
        if let Some(last) = blob.last_mut() {
            *last ^= 0x01;
        }
        let result = decrypt_blob(KEY, &blob, AAD);
        assert!(matches!(result, Err(CryptoError::Decrypt)));
        Ok(())
    }

    #[test]
    fn wrong_key_length_is_rejected() {
        let short_key: &[u8] = &[0u8; 16];
        assert!(matches!(
            encrypt_blob(short_key, PLAINTEXT, AAD),
            Err(CryptoError::KeyLength)
        ));
        assert!(matches!(
            decrypt_blob(short_key, &[0u8; 40], AAD),
            Err(CryptoError::KeyLength)
        ));
    }

    #[test]
    fn blob_shorter_than_nonce_is_rejected() {
        assert!(matches!(
            decrypt_blob(KEY, &[0u8; 4], AAD),
            Err(CryptoError::TooShort)
        ));
    }

    #[test]
    fn map_roundtrip_preserves_entries() -> TestResult {
        let mut creds: BTreeMap<String, Zeroizing<String>> = BTreeMap::new();
        creds.insert("username".to_owned(), Zeroizing::new("app_user".to_owned()));
        creds.insert("password".to_owned(), Zeroizing::new("s3cr3t".to_owned()));

        let blob = encrypt_map(KEY, &creds, AAD)?;
        let recovered = decrypt_map(KEY, &blob, AAD)?;

        assert_eq!(recovered.len(), creds.len());
        for (name, value) in &creds {
            let got = recovered.get(name).ok_or("missing key after roundtrip")?;
            assert_eq!(got.as_str(), value.as_str());
        }
        Ok(())
    }

    #[test]
    fn sha256_hex_matches_known_vector() {
        // SHA-256("abc")
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn encode_hex_is_lowercase_and_zero_padded() {
        assert_eq!(encode_hex(&[0x00, 0x0f, 0xff]), "000fff");
    }

    #[test]
    fn base64_roundtrip() -> TestResult {
        let encoded = base64_encode(PLAINTEXT.to_vec());
        let decoded = base64_decode(&encoded).ok_or("base64 decode failed")?;
        assert_eq!(decoded.as_slice(), PLAINTEXT);
        Ok(())
    }
}

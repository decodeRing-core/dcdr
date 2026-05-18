use std::fmt::Write;

use rsa::RsaPublicKey;
use rsa::pkcs1v15;
use rsa::pkcs8::DecodePublicKey;
use rsa::pss;
use rsa::signature::Verifier;
use sha2::Digest;
use sha2::Sha256;

use p256::ecdsa::{Signature as EcdsaSignature, VerifyingKey as EcdsaVerifyingKey};

use crate::error::TpmVerifyError;

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

// TPM algorithm identifiers (TCG TPM 2.0 spec, Part 2, Table 9)
const TPM_ALG_RSASSA: u16 = 0x0014;
const TPM_ALG_RSAPSS: u16 = 0x0016;
const TPM_ALG_ECDSA: u16 = 0x0018;
const TPM_ALG_SHA256: u16 = 0x000B;

/// Verify the AK's signature over the quote bytes.
pub fn verify_quote_signature(
    quote: &[u8],
    sig: &[u8],
    ak_pubkey_pem: &str,
) -> Result<(), TpmVerifyError> {
    let parsed = parse_tpmt_signature(sig)?;

    if parsed.hash_alg != TPM_ALG_SHA256 {
        return Err(TpmVerifyError::UnsupportedSigAlg);
    }

    match parsed.sig_alg {
        TPM_ALG_RSASSA => verify_rsassa_sha256(quote, parsed.sig_bytes, ak_pubkey_pem),
        TPM_ALG_RSAPSS => verify_rsapss_sha256(quote, parsed.sig_bytes, ak_pubkey_pem),
        TPM_ALG_ECDSA => verify_ecdsa_p256_sha256(quote, parsed.sig_bytes, ak_pubkey_pem),
        _ => Err(TpmVerifyError::UnsupportedSigAlg),
    }
}

struct ParsedTpmtSignature<'a> {
    sig_alg: u16,
    hash_alg: u16,
    sig_bytes: &'a [u8],
}

/// Parse the `TPMT_SIGNATURE` wrapper written by `tpm2_quote`.
///
/// Layout for RSA: sigAlg(2) | hashAlg(2) | `sig_size(2)` | `sig_bytes`
/// Layout for ECDSA: sigAlg(2) | hashAlg(2) | `r_size(2)` | r | `s_size(2)` | s
fn parse_tpmt_signature(sig: &[u8]) -> Result<ParsedTpmtSignature<'_>, TpmVerifyError> {
    let sig_alg = read_u16_be(sig, 0)?;
    let hash_alg = read_u16_be(sig, 2)?;

    match sig_alg {
        TPM_ALG_RSASSA | TPM_ALG_RSAPSS => {
            let sig_size = read_u16_be(sig, 4)? as usize;
            let sig_bytes = sig
                .get(6..6 + sig_size)
                .ok_or(TpmVerifyError::InvalidSignature)?;
            Ok(ParsedTpmtSignature {
                sig_alg,
                hash_alg,
                sig_bytes,
            })
        }
        TPM_ALG_ECDSA => {
            let sig_bytes = sig.get(4..).ok_or(TpmVerifyError::InvalidSignature)?;
            Ok(ParsedTpmtSignature {
                sig_alg,
                hash_alg,
                sig_bytes,
            })
        }
        _ => Err(TpmVerifyError::UnsupportedSigAlg),
    }
}

fn verify_rsassa_sha256(
    quote: &[u8],
    sig_bytes: &[u8],
    ak_pubkey_pem: &str,
) -> Result<(), TpmVerifyError> {
    let ak_pubkey = RsaPublicKey::from_public_key_pem(ak_pubkey_pem)
        .map_err(|_| TpmVerifyError::InvalidAkPubkey)?;
    let signature =
        pkcs1v15::Signature::try_from(sig_bytes).map_err(|_| TpmVerifyError::InvalidSignature)?;
    let verifying_key = pkcs1v15::VerifyingKey::<Sha256>::new(ak_pubkey);
    verifying_key
        .verify(quote, &signature)
        .map_err(|_| TpmVerifyError::SignatureVerificationFailed)
}

fn verify_rsapss_sha256(
    quote: &[u8],
    sig_bytes: &[u8],
    ak_pubkey_pem: &str,
) -> Result<(), TpmVerifyError> {
    let ak_pubkey = RsaPublicKey::from_public_key_pem(ak_pubkey_pem)
        .map_err(|_| TpmVerifyError::InvalidAkPubkey)?;
    let signature =
        pss::Signature::try_from(sig_bytes).map_err(|_| TpmVerifyError::InvalidSignature)?;
    let verifying_key = pss::VerifyingKey::<Sha256>::new(ak_pubkey);
    verifying_key
        .verify(quote, &signature)
        .map_err(|_| TpmVerifyError::SignatureVerificationFailed)
}

fn verify_ecdsa_p256_sha256(
    quote: &[u8],
    rs_blob: &[u8],
    ak_pubkey_pem: &str,
) -> Result<(), TpmVerifyError> {
    // TPM serialized ECDSA: r_size(2) | r | s_size(2) | s
    // p256::ecdsa::Signature wants 64 bytes: r(32) || s(32), left-padded if needed.
    let (r, rest) = read_length_prefixed(rs_blob).ok_or(TpmVerifyError::InvalidSignature)?;
    let (s, _) = read_length_prefixed(rest).ok_or(TpmVerifyError::InvalidSignature)?;

    let mut fixed = [0u8; 64];
    let (r_dst, s_dst) = fixed.split_at_mut(32);
    pad_left(r_dst, r).ok_or(TpmVerifyError::InvalidSignature)?;
    pad_left(s_dst, s).ok_or(TpmVerifyError::InvalidSignature)?;

    let signature =
        EcdsaSignature::from_slice(&fixed).map_err(|_| TpmVerifyError::InvalidSignature)?;
    let verifying_key = EcdsaVerifyingKey::from_public_key_pem(ak_pubkey_pem)
        .map_err(|_| TpmVerifyError::InvalidAkPubkey)?;

    verifying_key
        .verify(quote, &signature)
        .map_err(|_| TpmVerifyError::SignatureVerificationFailed)
}

fn read_u16_be(input: &[u8], offset: usize) -> Result<u16, TpmVerifyError> {
    let bytes: &[u8; 2] = input
        .get(offset..offset + 2)
        .and_then(|s| s.try_into().ok())
        .ok_or(TpmVerifyError::InvalidSignature)?;
    Ok(u16::from_be_bytes(*bytes))
}

fn read_length_prefixed(input: &[u8]) -> Option<(&[u8], &[u8])> {
    let len_bytes: &[u8; 2] = input.get(0..2).and_then(|s| s.try_into().ok())?;
    let len = u16::from_be_bytes(*len_bytes) as usize;
    let payload = input.get(2..2 + len)?;
    let rest = input.get(2 + len..)?;
    Some((payload, rest))
}

fn pad_left(dest: &mut [u8], src: &[u8]) -> Option<()> {
    let start = dest.len().checked_sub(src.len())?;
    let (pad, tail) = dest.split_at_mut(start);
    pad.fill(0);
    tail.copy_from_slice(src);
    Some(())
}

#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
#[cfg(test)]
mod tests {
    use super::*;
    use p256::ecdsa::{SigningKey as EcdsaSigningKey, signature::Signer as _};
    use rsa::pkcs1v15::SigningKey as Pkcs1v15SigningKey;
    use rsa::pkcs8::{EncodePublicKey, LineEnding};
    use rsa::pss::SigningKey as PssSigningKey;
    use rsa::rand_core::OsRng;
    use rsa::signature::{RandomizedSigner, SignatureEncoding};
    use rsa::{RsaPrivateKey, RsaPublicKey};
    use sha2::Sha256;

    fn make_rsa_keypair() -> (RsaPrivateKey, String) {
        let mut rng = OsRng;
        let priv_key = RsaPrivateKey::new(&mut rng, 2048).unwrap();
        let pub_key = RsaPublicKey::from(&priv_key);
        let pem = pub_key.to_public_key_pem(LineEnding::default()).unwrap();
        (priv_key, pem)
    }

    fn make_ecdsa_keypair() -> (EcdsaSigningKey, String) {
        let signing_key = EcdsaSigningKey::random(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let pem = verifying_key
            .to_public_key_pem(LineEnding::default())
            .unwrap();
        (signing_key, pem)
    }

    fn build_rsa_tpmt_sig(alg: u16, hash_alg: u16, sig_bytes: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&alg.to_be_bytes());
        out.extend_from_slice(&hash_alg.to_be_bytes());
        out.extend_from_slice(&(sig_bytes.len() as u16).to_be_bytes());
        out.extend_from_slice(sig_bytes);
        out
    }

    fn build_ecdsa_tpmt_sig(hash_alg: u16, r: &[u8], s: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&TPM_ALG_ECDSA.to_be_bytes());
        out.extend_from_slice(&hash_alg.to_be_bytes());
        out.extend_from_slice(&(r.len() as u16).to_be_bytes());
        out.extend_from_slice(r);
        out.extend_from_slice(&(s.len() as u16).to_be_bytes());
        out.extend_from_slice(s);
        out
    }

    #[test]
    fn pem_to_der_roundtrip() {
        let pem = "-----BEGIN TEST-----\nSGVsbG8gV29ybGQ=\n-----END TEST-----\n";
        let der = pem_to_der(pem).unwrap();
        assert_eq!(der, b"Hello World");
    }

    #[test]
    fn pem_to_der_rejects_invalid() {
        assert!(pem_to_der("not a pem").is_err());
        assert!(pem_to_der("").is_err());
    }

    #[test]
    fn sha256_hex_known_value() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn sha256_hex_pem_roundtrip() {
        let pem = "-----BEGIN TEST-----\nSGVsbG8=\n-----END TEST-----\n";
        let expected = sha256_hex(b"Hello");
        assert_eq!(sha256_hex_pem(pem), Some(expected));
    }

    #[test]
    fn sha256_hex_pem_invalid_returns_none() {
        assert_eq!(sha256_hex_pem("garbage"), None);
    }

    #[test]
    fn encode_hex_basic() {
        assert_eq!(encode_hex(&[]), "");
        assert_eq!(encode_hex(&[0x00]), "00");
        assert_eq!(encode_hex(&[0xff]), "ff");
        assert_eq!(encode_hex(&[0xde, 0xad, 0xbe, 0xef]), "deadbeef");
        assert_eq!(
            encode_hex(&[0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]),
            "0123456789abcdef"
        );
    }

    #[test]
    fn encode_hex_matches_sha256_hex() {
        let bytes = b"some test data";
        let digest = Sha256::digest(bytes);
        assert_eq!(encode_hex(&digest), sha256_hex(bytes));
    }

    #[test]
    fn read_u16_be_valid() {
        assert_eq!(read_u16_be(&[0x12, 0x34], 0).unwrap(), 0x1234);
        assert_eq!(read_u16_be(&[0x00, 0x12, 0x34, 0x56], 1).unwrap(), 0x1234);
    }

    #[test]
    fn read_u16_be_out_of_bounds() {
        assert!(read_u16_be(&[0x12], 0).is_err());
        assert!(read_u16_be(&[0x12, 0x34], 1).is_err());
        assert!(read_u16_be(&[], 0).is_err());
    }

    #[test]
    fn read_length_prefixed_valid() {
        let input = &[0x00, 0x03, 0xAA, 0xBB, 0xCC, 0xDD];
        let (payload, rest) = read_length_prefixed(input).unwrap();
        assert_eq!(payload, &[0xAA, 0xBB, 0xCC]);
        assert_eq!(rest, &[0xDD]);
    }

    #[test]
    fn read_length_prefixed_zero_length() {
        let input = &[0x00, 0x00, 0xFF];
        let (payload, rest) = read_length_prefixed(input).unwrap();
        assert!(payload.is_empty());
        assert_eq!(rest, &[0xFF]);
    }

    #[test]
    fn read_length_prefixed_truncated() {
        assert!(read_length_prefixed(&[0x00]).is_none());
        assert!(read_length_prefixed(&[0x00, 0x05, 0xAA]).is_none());
    }

    #[test]
    fn pad_left_exact() {
        let mut dest = [0xFFu8; 4];
        pad_left(&mut dest, &[1, 2, 3, 4]).unwrap();
        assert_eq!(dest, [1, 2, 3, 4]);
    }

    #[test]
    fn pad_left_shorter() {
        let mut dest = [0xFFu8; 4];
        pad_left(&mut dest, &[1, 2]).unwrap();
        assert_eq!(dest, [0, 0, 1, 2]);
    }

    #[test]
    fn pad_left_empty_src() {
        let mut dest = [0xFFu8; 3];
        pad_left(&mut dest, &[]).unwrap();
        assert_eq!(dest, [0, 0, 0]);
    }

    #[test]
    fn pad_left_src_too_long() {
        let mut dest = [0u8; 2];
        assert!(pad_left(&mut dest, &[1, 2, 3]).is_none());
    }

    #[test]
    fn parse_tpmt_signature_rsassa() {
        let sig = build_rsa_tpmt_sig(TPM_ALG_RSASSA, TPM_ALG_SHA256, &[0xAA; 256]);
        let parsed = parse_tpmt_signature(&sig).unwrap();
        assert_eq!(parsed.sig_alg, TPM_ALG_RSASSA);
        assert_eq!(parsed.hash_alg, TPM_ALG_SHA256);
        assert_eq!(parsed.sig_bytes.len(), 256);
    }

    #[test]
    fn parse_tpmt_signature_rsapss() {
        let sig = build_rsa_tpmt_sig(TPM_ALG_RSAPSS, TPM_ALG_SHA256, &[0xBB; 128]);
        let parsed = parse_tpmt_signature(&sig).unwrap();
        assert_eq!(parsed.sig_alg, TPM_ALG_RSAPSS);
        assert_eq!(parsed.sig_bytes.len(), 128);
    }

    #[test]
    fn parse_tpmt_signature_ecdsa() {
        let sig = build_ecdsa_tpmt_sig(TPM_ALG_SHA256, &[0x11; 32], &[0x22; 32]);
        let parsed = parse_tpmt_signature(&sig).unwrap();
        assert_eq!(parsed.sig_alg, TPM_ALG_ECDSA);
        assert_eq!(parsed.hash_alg, TPM_ALG_SHA256);
        assert_eq!(parsed.sig_bytes.len(), 2 + 32 + 2 + 32);
    }

    #[test]
    fn parse_tpmt_signature_rejects_unsupported_alg() {
        let sig = vec![0x00, 0x10, 0x00, 0x0B, 0x00, 0x00];
        let result = parse_tpmt_signature(&sig);
        assert!(matches!(result, Err(TpmVerifyError::UnsupportedSigAlg)));
    }

    #[test]
    fn parse_tpmt_signature_rejects_truncated_header() {
        assert!(matches!(
            parse_tpmt_signature(&[0x00]),
            Err(TpmVerifyError::InvalidSignature)
        ));
        assert!(matches!(
            parse_tpmt_signature(&[0x00, 0x14, 0x00]),
            Err(TpmVerifyError::InvalidSignature)
        ));
    }

    #[test]
    fn parse_tpmt_signature_rejects_rsa_truncated_payload() {
        let mut sig = Vec::new();
        sig.extend_from_slice(&TPM_ALG_RSASSA.to_be_bytes());
        sig.extend_from_slice(&TPM_ALG_SHA256.to_be_bytes());
        sig.extend_from_slice(&100u16.to_be_bytes());
        sig.extend_from_slice(&[0xAA; 10]);
        assert!(matches!(
            parse_tpmt_signature(&sig),
            Err(TpmVerifyError::InvalidSignature)
        ));
    }

    #[test]
    fn verify_quote_signature_rsassa_valid() {
        let (priv_key, pem) = make_rsa_keypair();
        let quote = b"test quote data";
        let signing_key = Pkcs1v15SigningKey::<Sha256>::new(priv_key);
        let signature = signing_key.sign(quote);
        let sig_bytes = signature.to_bytes();
        let tpmt_sig = build_rsa_tpmt_sig(TPM_ALG_RSASSA, TPM_ALG_SHA256, &sig_bytes);

        assert!(verify_quote_signature(quote, &tpmt_sig, &pem).is_ok());
    }

    #[test]
    fn verify_quote_signature_rsassa_rejects_wrong_message() {
        let (priv_key, pem) = make_rsa_keypair();
        let signing_key = Pkcs1v15SigningKey::<Sha256>::new(priv_key);
        let signature = signing_key.sign(b"original message");
        let sig_bytes = signature.to_bytes();
        let tpmt_sig = build_rsa_tpmt_sig(TPM_ALG_RSASSA, TPM_ALG_SHA256, &sig_bytes);

        let result = verify_quote_signature(b"tampered message", &tpmt_sig, &pem);
        assert!(matches!(
            result,
            Err(TpmVerifyError::SignatureVerificationFailed)
        ));
    }

    #[test]
    fn verify_quote_signature_rsapss_valid() {
        let (priv_key, pem) = make_rsa_keypair();
        let quote = b"test quote for pss";
        let signing_key = PssSigningKey::<Sha256>::new(priv_key);
        let signature = signing_key.sign_with_rng(&mut OsRng, quote);
        let sig_bytes = signature.to_bytes();
        let tpmt_sig = build_rsa_tpmt_sig(TPM_ALG_RSAPSS, TPM_ALG_SHA256, &sig_bytes);

        assert!(verify_quote_signature(quote, &tpmt_sig, &pem).is_ok());
    }

    #[test]
    fn verify_quote_signature_rsapss_rejects_tampered() {
        let (priv_key, pem) = make_rsa_keypair();
        let signing_key = PssSigningKey::<Sha256>::new(priv_key);
        let signature = signing_key.sign_with_rng(&mut OsRng, b"signed");
        let sig_bytes = signature.to_bytes();
        let tpmt_sig = build_rsa_tpmt_sig(TPM_ALG_RSAPSS, TPM_ALG_SHA256, &sig_bytes);

        let result = verify_quote_signature(b"not signed", &tpmt_sig, &pem);
        assert!(matches!(
            result,
            Err(TpmVerifyError::SignatureVerificationFailed)
        ));
    }

    #[test]
    fn verify_quote_signature_ecdsa_valid() {
        let (signing_key, pem) = make_ecdsa_keypair();
        let quote = b"ecdsa test quote";
        let signature: EcdsaSignature = signing_key.sign(quote);
        let bytes = signature.to_bytes();
        let r = &bytes[..32];
        let s = &bytes[32..];
        let tpmt_sig = build_ecdsa_tpmt_sig(TPM_ALG_SHA256, r, s);

        assert!(verify_quote_signature(quote, &tpmt_sig, &pem).is_ok());
    }

    #[test]
    fn verify_quote_signature_ecdsa_rejects_wrong_message() {
        let (signing_key, pem) = make_ecdsa_keypair();
        let signature: EcdsaSignature = signing_key.sign(b"good");
        let bytes = signature.to_bytes();
        let tpmt_sig = build_ecdsa_tpmt_sig(TPM_ALG_SHA256, &bytes[..32], &bytes[32..]);

        let result = verify_quote_signature(b"bad", &tpmt_sig, &pem);
        assert!(matches!(
            result,
            Err(TpmVerifyError::SignatureVerificationFailed)
        ));
    }

    #[test]
    fn verify_quote_signature_ecdsa_with_short_r_and_s() {
        let (signing_key, pem) = make_ecdsa_keypair();
        let quote = b"short r and s test";
        let signature: EcdsaSignature = signing_key.sign(quote);
        let bytes = signature.to_bytes();

        let r_full = &bytes[..32];
        let s_full = &bytes[32..];
        let r_trimmed = if r_full[0] == 0 { &r_full[1..] } else { r_full };
        let s_trimmed = if s_full[0] == 0 { &s_full[1..] } else { s_full };

        let tpmt_sig = build_ecdsa_tpmt_sig(TPM_ALG_SHA256, r_trimmed, s_trimmed);
        assert!(verify_quote_signature(quote, &tpmt_sig, &pem).is_ok());
    }

    #[test]
    fn verify_quote_signature_rejects_non_sha256_hash() {
        let (_, pem) = make_rsa_keypair();
        let tpmt_sig = build_rsa_tpmt_sig(TPM_ALG_RSASSA, 0x000C, &[0xAA; 256]);
        let result = verify_quote_signature(b"quote", &tpmt_sig, &pem);
        assert!(matches!(result, Err(TpmVerifyError::UnsupportedSigAlg)));
    }

    #[test]
    fn verify_quote_signature_rejects_unsupported_sig_alg() {
        let (_, pem) = make_rsa_keypair();
        let sig = vec![0x00, 0x10, 0x00, 0x0B, 0x00, 0x00];
        let result = verify_quote_signature(b"quote", &sig, &pem);
        assert!(matches!(result, Err(TpmVerifyError::UnsupportedSigAlg)));
    }

    #[test]
    fn verify_quote_signature_rejects_invalid_pem() {
        let tpmt_sig = build_rsa_tpmt_sig(TPM_ALG_RSASSA, TPM_ALG_SHA256, &[0xAA; 256]);
        let result = verify_quote_signature(b"quote", &tpmt_sig, "not a pem");
        assert!(matches!(result, Err(TpmVerifyError::InvalidAkPubkey)));
    }

    #[test]
    fn verify_quote_signature_ecdsa_rejects_truncated_blob() {
        let (_, pem) = make_ecdsa_keypair();
        let mut sig = Vec::new();
        sig.extend_from_slice(&TPM_ALG_ECDSA.to_be_bytes());
        sig.extend_from_slice(&TPM_ALG_SHA256.to_be_bytes());
        sig.extend_from_slice(&32u16.to_be_bytes());
        sig.extend_from_slice(&[0x11; 32]);
        let result = verify_quote_signature(b"quote", &sig, &pem);
        assert!(matches!(result, Err(TpmVerifyError::InvalidSignature)));
    }

    #[test]
    fn verify_quote_signature_rsa_wrong_key() {
        let (priv_key, _) = make_rsa_keypair();
        let (_, other_pem) = make_rsa_keypair();
        let signing_key = Pkcs1v15SigningKey::<Sha256>::new(priv_key);
        let signature = signing_key.sign(b"quote");
        let sig_bytes = signature.to_bytes();
        let tpmt_sig = build_rsa_tpmt_sig(TPM_ALG_RSASSA, TPM_ALG_SHA256, &sig_bytes);

        let result = verify_quote_signature(b"quote", &tpmt_sig, &other_pem);
        assert!(matches!(
            result,
            Err(TpmVerifyError::SignatureVerificationFailed)
        ));
    }
}

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

use std::collections::HashMap;

use aes::Aes128;
use aes::cipher::{Array, KeyIvInit};
use hmac::{Hmac, Mac};
use p256::ecdsa::{Signature as EcdsaSignature, VerifyingKey as EcdsaVerifyingKey};
use rand_08::RngCore;
use rsa::BigUint;
use rsa::Oaep;
use rsa::RsaPublicKey;
use rsa::pkcs1v15;
use rsa::pkcs8::DecodePublicKey;
use rsa::pkcs8::EncodePublicKey;
use rsa::pkcs8::LineEnding;
use rsa::pss;
use rsa::signature::Verifier;
use serde::Deserialize;
use sha2::Digest;
use sha2::Sha256;

use crate::tpm::error::TpmVerifyError;

#[derive(Deserialize, Debug)]
pub struct TpmMaterial {
    pub ek_pubkey_pem: String,
    #[serde(default)]
    pub ek_cert_pem: Option<String>,
    #[serde(default)]
    pub expected_pcrs: Option<HashMap<u8, String>>,
    #[serde(default)]
    pub require_ek_cert: bool,
    #[serde(default)]
    pub ak_pubkey_pem: String,
    #[serde(default)]
    pub ak_name_hex: String,
    #[serde(default)]
    pub activation_secret_hash: String,
}

const TPM_GENERATED_VALUE: u32 = 0xFF54_4347;
const TPM_ST_ATTEST_QUOTE: u16 = 0x8018;

/// Parsed `TPMS_ATTEST` structure (quote variant).
#[derive(Debug)]
pub struct ParsedQuote {
    /// extraData field — the nonce the server issued.
    pub extra_data: Vec<u8>,
    /// PCR digest from `TPMS_QUOTE_INFO` — SHA-256 of the concatenated PCR values
    /// that were selected (in selection order).
    pub pcr_digest: Vec<u8>,
    /// PCR selection (hash algorithm, PCR indices). Needed to verify the
    /// pcrs blob the client sent matches what was actually quoted.
    pub pcr_selections: Vec<PcrSelection>,
}

#[derive(Debug, Clone)]
pub struct PcrSelection {
    pub hash_alg: u16,
    /// PCR indices that were quoted, sorted ascending.
    pub indices: Vec<u8>,
}

// TPM algorithm identifiers (TCG TPM 2.0 spec, Part 2, Table 9)
const TPM_ALG_RSASSA: u16 = 0x0014;
const TPM_ALG_RSAPSS: u16 = 0x0016;
const TPM_ALG_ECDSA: u16 = 0x0018;
const TPM_ALG_SHA256: u16 = 0x000B;

// TCG Algorithm Registry IDs
const TPM_ALG_RSA: u16 = 0x0001;
const TPM_ALG_NULL: u16 = 0x0010;

#[derive(Debug)]
pub struct AkPublic {
    /// TPM object Name: UINT16(nameAlg) || `H_nameAlg(publicArea)`.
    pub name: Vec<u8>,
    /// SPKI PEM of the RSA public key, for `verify_quote_signature`.
    pub pubkey_pem: String,
}

impl AkPublic {
    /// Parse a marshaled `TPM2B_PUBLIC` (size-prefixed `TPMT_PUBLIC`) for an
    /// RSA attestation key: compute its Name and extract the public key.
    pub fn parse(tpm2b_public: &[u8]) -> Result<Self, TpmVerifyError> {
        let mut outer = Cursor::new(tpm2b_public);
        let pa_len = outer.read_u16()? as usize;
        let public_area = outer.read_bytes(pa_len)?; // the exact bytes we hash for Name

        let mut r = Cursor::new(public_area);

        let key_type = r.read_u16()?;
        if key_type != TPM_ALG_RSA {
            return Err(TpmVerifyError::UnsupportedSigAlg); // ECC AK not handled here
        }

        let name_alg = r.read_u16()?;
        let _object_attributes = r.read_u32()?;
        let _auth_policy = r.read_sized_2b()?; // TPM2B_DIGEST

        // TPMS_RSA_PARMS.symmetric : TPMT_SYM_DEF_OBJECT
        let sym_alg = r.read_u16()?;
        if sym_alg != TPM_ALG_NULL {
            // keyBits(UINT16) + mode(UINT16) for AES/SM4/Camellia.
            // An AK normally has symmetric == NULL, so this branch is unused.
            r.skip(4)?;
        }

        // TPMS_RSA_PARMS.scheme : TPMT_RSA_SCHEME
        let scheme = r.read_u16()?;
        if scheme != TPM_ALG_NULL {
            r.skip(2)?; // details = single hashAlg(UINT16) for RSASSA/RSAPSS/OAEP
        }

        let _key_bits = r.read_u16()?; // e.g. 2048
        let exponent_raw = r.read_u32()?; // 0 => default 65537

        // unique : TPM2B_PUBLIC_KEY_RSA -> modulus, big-endian
        let modulus = r.read_sized_2b()?;

        // Name
        if name_alg != TPM_ALG_SHA256 {
            return Err(TpmVerifyError::UnsupportedSigAlg); // extend for other nameAlgs
        }
        let mut name = Vec::with_capacity(2 + 32);
        name.extend_from_slice(&name_alg.to_be_bytes());
        name.extend_from_slice(&Sha256::digest(public_area));

        // public key
        let exponent = if exponent_raw == 0 {
            65_537
        } else {
            exponent_raw
        };
        let pubkey = RsaPublicKey::new(BigUint::from_bytes_be(modulus), BigUint::from(exponent))
            .map_err(|_| TpmVerifyError::InvalidAkPubkey)?;
        let pubkey_pem = pubkey
            .to_public_key_pem(LineEnding::LF)
            .map_err(|_| TpmVerifyError::InvalidAkPubkey)?;

        Ok(Self { name, pubkey_pem })
    }
}

type HmacSha256 = Hmac<Sha256>;
type Aes128CfbEnc = cfb_mode::Encryptor<Aes128>;

const SEED_LEN: usize = 32; // SHA-256 digest size = EK nameAlg digest size

pub struct MakeCredentialOutput {
    pub credential_blob: Vec<u8>, // TPM2B_ID_OBJECT, size-prefixed
    pub secret: Vec<u8>,          // TPM2B_ENCRYPTED_SECRET, size-prefixed
}

/// Software `TPM2_MakeCredential` for an RSA EK.
///
/// Uses the default TCG EK template (nameAlg SHA-256, symmetric AES-128-CFB).
/// `ak_name` is the full TPM Name (UINT16(nameAlg) || H(publicArea)) from `AkPublic::parse`.
pub fn make_credential_rsa(
    ek_pubkey_pem: &str,
    ak_name: &[u8],
    secret: &[u8], // credential payload; must be <= SEED_LEN
) -> Result<MakeCredentialOutput, TpmVerifyError> {
    if secret.len() > SEED_LEN {
        return Err(TpmVerifyError::InvalidQuote);
    }
    let ek = RsaPublicKey::from_public_key_pem(ek_pubkey_pem)
        .map_err(|_| TpmVerifyError::InvalidAkPubkey)?;

    let mut rng = rand_08::thread_rng();
    // Random seed, digest-sized.
    let mut seed = [0u8; SEED_LEN];
    rng.fill_bytes(&mut seed);
    // 2. secret blob = RSA-OAEP(EK_pub, seed); SHA-256; L = b"IDENTITY\0".
    let padding = Oaep::new_with_label::<Sha256, _>("IDENTITY\0");
    let enc_secret = ek
        .encrypt(&mut rng, padding, &seed)
        .map_err(|_| TpmVerifyError::InvalidAkPubkey)?;

    // 3. symKey = KDFa(SHA256, seed, "STORAGE", ak_name, <empty>, 128)
    let sym_key = kdfa_sha256(&seed, "STORAGE", ak_name, &[], 128)?;

    // 4. encIdentity = AES-128-CFB(symKey, IV=0) over TPM2B(secret)
    let mut inner = Vec::with_capacity(2 + secret.len());
    let secret_len = u16::try_from(secret.len()).map_err(|_| TpmVerifyError::InvalidSize)?;
    inner.extend_from_slice(&secret_len.to_be_bytes());
    //inner.extend_from_slice(&(secret.len() as u16).to_be_bytes());
    inner.extend_from_slice(secret);
    let mut enc_identity = inner;
    let key: [u8; 16] = sym_key
        .as_slice()
        .try_into()
        .map_err(|_| TpmVerifyError::InvalidAkPubkey)?;
    Aes128CfbEnc::new((&key).into(), &Array::default()).encrypt(&mut enc_identity);

    // 5. HMACkey = KDFa(SHA256, seed, "INTEGRITY", <empty>, <empty>, 256)
    let hmac_key = kdfa_sha256(&seed, "INTEGRITY", &[], &[], 256)?;

    // 6. outerHMAC = HMAC-SHA256(HMACkey, encIdentity || ak_name)
    let mut mac =
        <HmacSha256 as Mac>::new_from_slice(&hmac_key).map_err(|_| TpmVerifyError::InvalidQuote)?;
    mac.update(&enc_identity);
    mac.update(ak_name);
    let outer_hmac = mac.finalize().into_bytes();

    // 7. idObject = TPM2B_DIGEST(outerHMAC) || encIdentity   (encIdentity raw)
    let mut id_object = Vec::new();
    let outer_hmac_len =
        u16::try_from(outer_hmac.len()).map_err(|_| TpmVerifyError::InvalidSize)?;
    id_object.extend_from_slice(&outer_hmac_len.to_be_bytes());
    id_object.extend_from_slice(&outer_hmac);
    id_object.extend_from_slice(&enc_identity);

    Ok(MakeCredentialOutput {
        credential_blob: prepend_u16_size(&id_object)?,
        secret: prepend_u16_size(&enc_secret)?,
    })
}

fn prepend_u16_size(b: &[u8]) -> Result<Vec<u8>, TpmVerifyError> {
    let len = u16::try_from(b.len()).map_err(|_| TpmVerifyError::InvalidSize)?;
    let mut v = Vec::with_capacity(2 + b.len());
    v.extend_from_slice(&len.to_be_bytes());
    v.extend_from_slice(b);
    Ok(v)
}

/// `KDFa`, TPM 2.0 Part 1 §11.4.10 — SP800-108 counter mode, HMAC-SHA256.
/// Per iteration: `counter_be(4)` || label || 0x00 || contextU || contextV || `bits_be(4)`
fn kdfa_sha256(
    key: &[u8],
    label: &str,
    context_u: &[u8],
    context_v: &[u8],
    bits: u32,
) -> Result<Vec<u8>, TpmVerifyError> {
    let out_len = (bits as usize).div_ceil(8);
    let mut out = Vec::new();
    let mut counter: u32 = 0;
    while out.len() < out_len {
        counter += 1;
        let mut mac =
            <HmacSha256 as Mac>::new_from_slice(key).map_err(|_| TpmVerifyError::InvalidQuote)?;
        mac.update(&counter.to_be_bytes());
        mac.update(label.as_bytes());
        mac.update(&[0u8]); // label terminator
        mac.update(context_u);
        mac.update(context_v);
        mac.update(&bits.to_be_bytes());
        out.extend_from_slice(&mac.finalize().into_bytes());
    }
    out.truncate(out_len);
    Ok(out)
}

/// Parse a `TPMS_ATTEST` blob produced by `tpm2_quote` -m.
pub fn parse_tpms_attest(quote: &[u8]) -> Result<ParsedQuote, TpmVerifyError> {
    let mut c = Cursor::new(quote);

    let magic = c.read_u32()?;
    if magic != TPM_GENERATED_VALUE {
        return Err(TpmVerifyError::InvalidQuote);
    }

    // Type — must be a quote attestation
    let ty = c.read_u16()?;
    if ty != TPM_ST_ATTEST_QUOTE {
        return Err(TpmVerifyError::InvalidQuote);
    }

    // qualifiedSigner: TPM2B_NAME (skip — not needed here, AK match is done elsewhere)
    let _qualified_signer = c.read_sized_2b()?;

    // extraData: TPM2B_DATA — this is the nonce
    let extra_data = c.read_sized_2b()?.to_vec();

    // clockInfo: 8 + 4 + 4 + 1 = 17 bytes
    c.skip(17)?;

    // firmwareVersion: UINT64
    c.skip(8)?;

    // attested (TPMS_QUOTE_INFO)
    //   pcrSelect: TPML_PCR_SELECTION
    //   pcrDigest: TPM2B_DIGEST
    let pcr_selections = parse_pcr_selection_list(&mut c)?;
    let pcr_digest = c.read_sized_2b()?.to_vec();

    Ok(ParsedQuote {
        extra_data,
        pcr_digest,
        pcr_selections,
    })
}

/// `TPML_PCR_SELECTION`: UINT32 count, then `count` `TPMS_PCR_SELECTION` entries.
/// Each `TPMS_PCR_SELECTION`: hashAlg UINT16, sizeOfSelect UINT8, pcrSelect bytes (bitmap).
fn parse_pcr_selection_list(c: &mut Cursor<'_>) -> Result<Vec<PcrSelection>, TpmVerifyError> {
    let count = c.read_u32()? as usize;
    let mut selections = Vec::with_capacity(count);

    for _ in 0..count {
        let hash_alg = c.read_u16()?;
        let size_of_select = c.read_u8()? as usize;
        let bitmap = c.read_bytes(size_of_select)?;

        // Decode the bitmap. Byte i, bit j means PCR (i*8 + j) is selected.
        let mut indices = Vec::new();
        for (byte_idx, byte) in bitmap.iter().enumerate() {
            for bit in 0..8u8 {
                if byte & (1 << bit) != 0 {
                    let idx = u8::try_from(byte_idx * 8)
                        .map_err(|_| TpmVerifyError::InvalidPcrSelection)?
                        + bit;
                    indices.push(idx);
                }
            }
        }

        selections.push(PcrSelection { hash_alg, indices });
    }

    Ok(selections)
}

/// Verify the PCR values the client sent against the quote and the
/// credential policy.
///
/// Two checks:
///   1. Hashing the concatenated PCR values in selection order produces
///      the same digest as the quote's pcrDigest. This proves the PCRs
///      sent are the same ones the TPM signed over.
///   2. Each PCR pinned by the credential policy matches the expected value.
pub fn verify_pcrs(
    pcrs: &HashMap<u8, String>,
    expected_digest: &[u8],
    expected_pcrs: &HashMap<u8, String>,
    pcr_selections: &[PcrSelection],
) -> Result<(), TpmVerifyError> {
    const SHA256_LEN: usize = 32;
    const TPM_ALG_SHA256: u16 = 0x000B;

    // Flatten the selections into an ordered list of PCR indices.
    // The TPM concatenates PCR digests in this same order to compute
    // the quote's pcrDigest.
    let mut ordered_indices = Vec::new();
    for sel in pcr_selections {
        if sel.hash_alg != TPM_ALG_SHA256 {
            tracing::error!(
                hash_alg = sel.hash_alg,
                "unsupported PCR hash algorithm (only SHA-256 is supported)"
            );
            return Err(TpmVerifyError::PcrMismatch);
        }
        ordered_indices.extend_from_slice(&sel.indices);
    }

    // Build the concatenated digest blob in selection order, decoding
    // each hex value from the client-supplied map.
    let mut concatenated = Vec::with_capacity(ordered_indices.len() * SHA256_LEN);
    for &pcr_index in &ordered_indices {
        let hex = pcrs.get(&pcr_index).ok_or_else(|| {
            tracing::error!(pcr_index, "client did not send a value for this quoted PCR");
            TpmVerifyError::PcrMismatch
        })?;

        let bytes = hex_decode(hex).ok_or_else(|| {
            tracing::error!(pcr_index, "PCR value is not valid hex");
            TpmVerifyError::PcrMismatch
        })?;

        if bytes.len() != SHA256_LEN {
            tracing::error!(
                pcr_index,
                actual_len = bytes.len(),
                expected_len = SHA256_LEN,
                "PCR digest has wrong length"
            );
            return Err(TpmVerifyError::PcrMismatch);
        }

        concatenated.extend_from_slice(&bytes);
    }

    // Recomputed digest matches what the TPM signed.
    let recomputed_digest = Sha256::digest(&concatenated);
    if recomputed_digest.as_slice() != expected_digest {
        tracing::error!(
            "recomputed PCR digest does not match quote's pcrDigest \
             (PCR values sent do not match what the TPM signed over)"
        );
        return Err(TpmVerifyError::PcrMismatch);
    }

    // Each PCR pinned by the policy matches the sent value.
    for (&pcr_index, expected_hex) in expected_pcrs {
        let actual_hex = pcrs.get(&pcr_index).ok_or_else(|| {
            tracing::error!(
                pcr_index,
                "credential policy requires this PCR but client did not send it"
            );
            TpmVerifyError::PcrMismatch
        })?;

        if !actual_hex.eq_ignore_ascii_case(expected_hex) {
            tracing::error!(
                pcr_index,
                expected = %expected_hex,
                actual = %actual_hex,
                "PCR value does not match credential policy"
            );
            return Err(TpmVerifyError::PcrMismatch);
        }
    }

    Ok(())
}

struct Cursor<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn read_u8(&mut self) -> Result<u8, TpmVerifyError> {
        let b = *self.buf.get(self.pos).ok_or(TpmVerifyError::InvalidQuote)?;
        self.pos += 1;
        Ok(b)
    }

    fn read_u16(&mut self) -> Result<u16, TpmVerifyError> {
        let slice = self.read_bytes(2)?;
        let arr: [u8; 2] = slice.try_into().map_err(|_| TpmVerifyError::InvalidQuote)?;
        Ok(u16::from_be_bytes(arr))
    }

    fn read_u32(&mut self) -> Result<u32, TpmVerifyError> {
        let slice = self.read_bytes(4)?;
        let arr: [u8; 4] = slice.try_into().map_err(|_| TpmVerifyError::InvalidQuote)?;
        Ok(u32::from_be_bytes(arr))
    }

    fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], TpmVerifyError> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or(TpmVerifyError::InvalidQuote)?;
        let slice = self
            .buf
            .get(self.pos..end)
            .ok_or(TpmVerifyError::InvalidQuote)?;
        self.pos = end;
        Ok(slice)
    }

    fn skip(&mut self, n: usize) -> Result<(), TpmVerifyError> {
        self.read_bytes(n).map(|_| ())
    }

    /// Read a TPM2B (UINT16 size prefix, then bytes).
    fn read_sized_2b(&mut self) -> Result<&'a [u8], TpmVerifyError> {
        let size = self.read_u16()? as usize;
        self.read_bytes(size)
    }
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok())
        .collect()
}

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
    use crate::tpm::error::TpmVerifyError;
    use decodering_core::crypto::{encode_hex, pem_to_der, sha256_hex, sha256_hex_pem};
    use p256::ecdsa::{SigningKey as EcdsaSigningKey, signature::Signer as _};
    use rsa::pkcs1v15::SigningKey as Pkcs1v15SigningKey;
    use rsa::pkcs8::{EncodePublicKey, LineEnding};
    use rsa::pss::SigningKey as PssSigningKey;
    use rsa::rand_core::OsRng;
    use rsa::signature::{RandomizedSigner, SignatureEncoding};
    use rsa::{RsaPrivateKey, RsaPublicKey};
    use sha2::Sha256;

    use sha2::Digest;
    use std::collections::HashMap;

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

    fn build_tpms_attest(
        magic: u32,
        ty: u16,
        qualified_signer: &[u8],
        extra_data: &[u8],
        pcr_selections: &[(u16, &[u8], &[u8])],
        pcr_digest: &[u8],
    ) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&magic.to_be_bytes());
        buf.extend_from_slice(&ty.to_be_bytes());
        buf.extend_from_slice(&(qualified_signer.len() as u16).to_be_bytes());
        buf.extend_from_slice(qualified_signer);
        buf.extend_from_slice(&(extra_data.len() as u16).to_be_bytes());
        buf.extend_from_slice(extra_data);
        buf.extend_from_slice(&[0u8; 17]);
        buf.extend_from_slice(&[0u8; 8]);
        buf.extend_from_slice(&(pcr_selections.len() as u32).to_be_bytes());
        for (hash_alg, size_of_select, bitmap) in pcr_selections {
            buf.extend_from_slice(&hash_alg.to_be_bytes());
            buf.push(size_of_select[0]);
            buf.extend_from_slice(bitmap);
        }
        buf.extend_from_slice(&(pcr_digest.len() as u16).to_be_bytes());
        buf.extend_from_slice(pcr_digest);
        buf
    }

    fn pcr_bitmap_for(indices: &[u8]) -> (Vec<u8>, u8) {
        let max = indices.iter().copied().max().unwrap_or(0);
        let size = (max / 8 + 1) as usize;
        let mut bitmap = vec![0u8; size];
        for &idx in indices {
            bitmap[(idx / 8) as usize] |= 1 << (idx % 8);
        }
        (bitmap, size as u8)
    }

    #[test]
    fn parse_tpms_attest_valid_quote() {
        let (bitmap, size) = pcr_bitmap_for(&[0, 7]);
        let quote = build_tpms_attest(
            TPM_GENERATED_VALUE,
            TPM_ST_ATTEST_QUOTE,
            b"signer-name",
            b"nonce-123",
            &[(0x000B, &[size], &bitmap)],
            &[0xAB; 32],
        );
        let parsed = parse_tpms_attest(&quote).unwrap();
        assert_eq!(parsed.extra_data, b"nonce-123");
        assert_eq!(parsed.pcr_digest, vec![0xAB; 32]);
        assert_eq!(parsed.pcr_selections.len(), 1);
        assert_eq!(parsed.pcr_selections[0].hash_alg, 0x000B);
        assert_eq!(parsed.pcr_selections[0].indices, vec![0, 7]);
    }

    #[test]
    fn parse_tpms_attest_multiple_selections() {
        let (bitmap_a, size_a) = pcr_bitmap_for(&[0, 1, 2]);
        let (bitmap_b, size_b) = pcr_bitmap_for(&[10, 15]);
        let quote = build_tpms_attest(
            TPM_GENERATED_VALUE,
            TPM_ST_ATTEST_QUOTE,
            b"signer",
            b"nonce",
            &[
                (0x000B, &[size_a], &bitmap_a),
                (0x000B, &[size_b], &bitmap_b),
            ],
            &[0x11; 32],
        );
        let parsed = parse_tpms_attest(&quote).unwrap();
        assert_eq!(parsed.pcr_selections.len(), 2);
        assert_eq!(parsed.pcr_selections[0].indices, vec![0, 1, 2]);
        assert_eq!(parsed.pcr_selections[1].indices, vec![10, 15]);
    }

    #[test]
    fn parse_tpms_attest_empty_extra_data() {
        let (bitmap, size) = pcr_bitmap_for(&[0]);
        let quote = build_tpms_attest(
            TPM_GENERATED_VALUE,
            TPM_ST_ATTEST_QUOTE,
            b"signer",
            b"",
            &[(0x000B, &[size], &bitmap)],
            &[0x00; 32],
        );
        let parsed = parse_tpms_attest(&quote).unwrap();
        assert!(parsed.extra_data.is_empty());
    }

    #[test]
    fn parse_tpms_attest_no_selections() {
        let quote = build_tpms_attest(
            TPM_GENERATED_VALUE,
            TPM_ST_ATTEST_QUOTE,
            b"signer",
            b"nonce",
            &[],
            &[0x00; 32],
        );
        let parsed = parse_tpms_attest(&quote).unwrap();
        assert!(parsed.pcr_selections.is_empty());
    }

    #[test]
    fn parse_tpms_attest_rejects_bad_magic() {
        let (bitmap, size) = pcr_bitmap_for(&[0]);
        let quote = build_tpms_attest(
            0xDEAD_BEEF,
            TPM_ST_ATTEST_QUOTE,
            b"signer",
            b"nonce",
            &[(0x000B, &[size], &bitmap)],
            &[0x00; 32],
        );
        assert!(matches!(
            parse_tpms_attest(&quote),
            Err(TpmVerifyError::InvalidQuote)
        ));
    }

    #[test]
    fn parse_tpms_attest_rejects_non_quote_type() {
        let (bitmap, size) = pcr_bitmap_for(&[0]);
        let quote = build_tpms_attest(
            TPM_GENERATED_VALUE,
            0x8017,
            b"signer",
            b"nonce",
            &[(0x000B, &[size], &bitmap)],
            &[0x00; 32],
        );
        assert!(matches!(
            parse_tpms_attest(&quote),
            Err(TpmVerifyError::InvalidQuote)
        ));
    }

    #[test]
    fn parse_tpms_attest_rejects_truncated() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&TPM_GENERATED_VALUE.to_be_bytes());
        buf.extend_from_slice(&TPM_ST_ATTEST_QUOTE.to_be_bytes());
        assert!(matches!(
            parse_tpms_attest(&buf),
            Err(TpmVerifyError::InvalidQuote)
        ));
    }

    #[test]
    fn parse_tpms_attest_rejects_empty() {
        assert!(matches!(
            parse_tpms_attest(&[]),
            Err(TpmVerifyError::InvalidQuote)
        ));
    }

    #[test]
    fn parse_tpms_attest_rejects_truncated_extra_data() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&TPM_GENERATED_VALUE.to_be_bytes());
        buf.extend_from_slice(&TPM_ST_ATTEST_QUOTE.to_be_bytes());
        buf.extend_from_slice(&0u16.to_be_bytes());
        buf.extend_from_slice(&100u16.to_be_bytes());
        buf.extend_from_slice(b"short");
        assert!(matches!(
            parse_tpms_attest(&buf),
            Err(TpmVerifyError::InvalidQuote)
        ));
    }

    #[test]
    fn parse_tpms_attest_bitmap_decodes_high_pcr_index() {
        let (bitmap, size) = pcr_bitmap_for(&[23]);
        let quote = build_tpms_attest(
            TPM_GENERATED_VALUE,
            TPM_ST_ATTEST_QUOTE,
            b"signer",
            b"nonce",
            &[(0x000B, &[size], &bitmap)],
            &[0x00; 32],
        );
        let parsed = parse_tpms_attest(&quote).unwrap();
        assert_eq!(parsed.pcr_selections[0].indices, vec![23]);
    }

    #[test]
    fn verify_pcrs_succeeds_with_valid_digest() {
        let pcr0 = [0xAAu8; 32];
        let pcr7 = [0xBBu8; 32];
        let mut hasher = Sha256::new();
        hasher.update(pcr0);
        hasher.update(pcr7);
        let digest = hasher.finalize();

        let mut pcrs = HashMap::new();
        pcrs.insert(0, "aa".repeat(32));
        pcrs.insert(7, "bb".repeat(32));

        let selections = vec![PcrSelection {
            hash_alg: 0x000B,
            indices: vec![0, 7],
        }];

        let expected_pcrs = HashMap::new();
        assert!(verify_pcrs(&pcrs, &digest, &expected_pcrs, &selections).is_ok());
    }

    #[test]
    fn verify_pcrs_succeeds_with_policy_match() {
        let pcr0 = [0xAAu8; 32];
        let digest = Sha256::digest(pcr0);

        let mut pcrs = HashMap::new();
        pcrs.insert(0, "aa".repeat(32));

        let mut expected = HashMap::new();
        expected.insert(0, "aa".repeat(32));

        let selections = vec![PcrSelection {
            hash_alg: 0x000B,
            indices: vec![0],
        }];

        assert!(verify_pcrs(&pcrs, &digest, &expected, &selections).is_ok());
    }

    #[test]
    fn verify_pcrs_policy_match_case_insensitive() {
        let pcr0 = [0xAAu8; 32];
        let digest = Sha256::digest(pcr0);

        let mut pcrs = HashMap::new();
        pcrs.insert(0, "AA".repeat(32));

        let mut expected = HashMap::new();
        expected.insert(0, "aa".repeat(32));

        let selections = vec![PcrSelection {
            hash_alg: 0x000B,
            indices: vec![0],
        }];

        assert!(verify_pcrs(&pcrs, &digest, &expected, &selections).is_ok());
    }

    #[test]
    fn verify_pcrs_rejects_wrong_digest() {
        let mut pcrs = HashMap::new();
        pcrs.insert(0, "aa".repeat(32));

        let selections = vec![PcrSelection {
            hash_alg: 0x000B,
            indices: vec![0],
        }];

        let wrong_digest = [0u8; 32];
        let result = verify_pcrs(&pcrs, &wrong_digest, &HashMap::new(), &selections);
        assert!(matches!(result, Err(TpmVerifyError::PcrMismatch)));
    }

    #[test]
    fn verify_pcrs_rejects_unsupported_hash_alg() {
        let pcrs = HashMap::new();
        let selections = vec![PcrSelection {
            hash_alg: 0x0004,
            indices: vec![0],
        }];
        let result = verify_pcrs(&pcrs, &[0u8; 32], &HashMap::new(), &selections);
        assert!(matches!(result, Err(TpmVerifyError::PcrMismatch)));
    }

    #[test]
    fn verify_pcrs_rejects_missing_quoted_pcr() {
        let pcrs = HashMap::new();
        let selections = vec![PcrSelection {
            hash_alg: 0x000B,
            indices: vec![0],
        }];
        let result = verify_pcrs(&pcrs, &[0u8; 32], &HashMap::new(), &selections);
        assert!(matches!(result, Err(TpmVerifyError::PcrMismatch)));
    }

    #[test]
    fn verify_pcrs_rejects_invalid_hex() {
        let mut pcrs = HashMap::new();
        pcrs.insert(0, "zz".repeat(32));

        let selections = vec![PcrSelection {
            hash_alg: 0x000B,
            indices: vec![0],
        }];
        let result = verify_pcrs(&pcrs, &[0u8; 32], &HashMap::new(), &selections);
        assert!(matches!(result, Err(TpmVerifyError::PcrMismatch)));
    }

    #[test]
    fn verify_pcrs_rejects_wrong_length() {
        let mut pcrs = HashMap::new();
        pcrs.insert(0, "aabb".to_owned());

        let selections = vec![PcrSelection {
            hash_alg: 0x000B,
            indices: vec![0],
        }];
        let result = verify_pcrs(&pcrs, &[0u8; 32], &HashMap::new(), &selections);
        assert!(matches!(result, Err(TpmVerifyError::PcrMismatch)));
    }

    #[test]
    fn verify_pcrs_rejects_policy_mismatch() {
        let pcr0 = [0xAAu8; 32];
        let digest = Sha256::digest(pcr0);

        let mut pcrs = HashMap::new();
        pcrs.insert(0, "aa".repeat(32));

        let mut expected = HashMap::new();
        expected.insert(0, "bb".repeat(32));

        let selections = vec![PcrSelection {
            hash_alg: 0x000B,
            indices: vec![0],
        }];

        let result = verify_pcrs(&pcrs, &digest, &expected, &selections);
        assert!(matches!(result, Err(TpmVerifyError::PcrMismatch)));
    }

    #[test]
    fn verify_pcrs_rejects_policy_pcr_not_sent() {
        let pcr0 = [0xAAu8; 32];
        let digest = Sha256::digest(pcr0);

        let mut pcrs = HashMap::new();
        pcrs.insert(0, "aa".repeat(32));

        let mut expected = HashMap::new();
        expected.insert(7, "bb".repeat(32));

        let selections = vec![PcrSelection {
            hash_alg: 0x000B,
            indices: vec![0],
        }];

        let result = verify_pcrs(&pcrs, &digest, &expected, &selections);
        assert!(matches!(result, Err(TpmVerifyError::PcrMismatch)));
    }

    #[test]
    fn verify_pcrs_ordering_matters() {
        let pcr0 = [0xAAu8; 32];
        let pcr7 = [0xBBu8; 32];
        let mut hasher = Sha256::new();
        hasher.update(pcr7);
        hasher.update(pcr0);
        let digest_reversed = hasher.finalize();

        let mut pcrs = HashMap::new();
        pcrs.insert(0, "aa".repeat(32));
        pcrs.insert(7, "bb".repeat(32));

        let selections = vec![PcrSelection {
            hash_alg: 0x000B,
            indices: vec![0, 7],
        }];

        let result = verify_pcrs(&pcrs, &digest_reversed, &HashMap::new(), &selections);
        assert!(matches!(result, Err(TpmVerifyError::PcrMismatch)));
    }

    #[test]
    fn verify_pcrs_concatenates_across_selections() {
        let pcr0 = [0xAAu8; 32];
        let pcr10 = [0xCCu8; 32];
        let mut hasher = Sha256::new();
        hasher.update(pcr0);
        hasher.update(pcr10);
        let digest = hasher.finalize();

        let mut pcrs = HashMap::new();
        pcrs.insert(0, "aa".repeat(32));
        pcrs.insert(10, "cc".repeat(32));

        let selections = vec![
            PcrSelection {
                hash_alg: 0x000B,
                indices: vec![0],
            },
            PcrSelection {
                hash_alg: 0x000B,
                indices: vec![10],
            },
        ];

        assert!(verify_pcrs(&pcrs, &digest, &HashMap::new(), &selections).is_ok());
    }

    #[test]
    fn verify_pcrs_empty_selections_with_empty_digest() {
        let pcrs = HashMap::new();
        let digest = Sha256::digest([]);
        let result = verify_pcrs(&pcrs, &digest, &HashMap::new(), &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn hex_decode_valid() {
        assert_eq!(hex_decode("00").unwrap(), vec![0x00]);
        assert_eq!(hex_decode("ff").unwrap(), vec![0xff]);
        assert_eq!(hex_decode("FF").unwrap(), vec![0xff]);
        assert_eq!(hex_decode("dead").unwrap(), vec![0xde, 0xad]);
        assert_eq!(hex_decode("").unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn hex_decode_rejects_odd_length() {
        assert!(hex_decode("a").is_none());
        assert!(hex_decode("abc").is_none());
    }

    #[test]
    fn hex_decode_rejects_non_hex() {
        assert!(hex_decode("zz").is_none());
        assert!(hex_decode("0g").is_none());
    }

    #[test]
    fn cursor_read_u8() {
        let mut c = Cursor::new(&[0x42, 0x43]);
        assert_eq!(c.read_u8().unwrap(), 0x42);
        assert_eq!(c.read_u8().unwrap(), 0x43);
        assert!(c.read_u8().is_err());
    }

    #[test]
    fn cursor_read_u16_be() {
        let mut c = Cursor::new(&[0x12, 0x34]);
        assert_eq!(c.read_u16().unwrap(), 0x1234);
    }

    #[test]
    fn cursor_read_u32_be() {
        let mut c = Cursor::new(&[0x12, 0x34, 0x56, 0x78]);
        assert_eq!(c.read_u32().unwrap(), 0x1234_5678);
    }

    #[test]
    fn cursor_read_u16_truncated() {
        let mut c = Cursor::new(&[0x12]);
        assert!(c.read_u16().is_err());
    }

    #[test]
    fn cursor_read_u32_truncated() {
        let mut c = Cursor::new(&[0x12, 0x34, 0x56]);
        assert!(c.read_u32().is_err());
    }

    #[test]
    fn cursor_read_bytes() {
        let mut c = Cursor::new(&[1, 2, 3, 4, 5]);
        assert_eq!(c.read_bytes(3).unwrap(), &[1, 2, 3]);
        assert_eq!(c.read_bytes(2).unwrap(), &[4, 5]);
        assert!(c.read_bytes(1).is_err());
    }

    #[test]
    fn cursor_read_bytes_zero() {
        let mut c = Cursor::new(&[1, 2, 3]);
        assert_eq!(c.read_bytes(0).unwrap(), &[] as &[u8]);
    }

    #[test]
    fn cursor_skip() {
        let mut c = Cursor::new(&[1, 2, 3, 4, 5]);
        c.skip(2).unwrap();
        assert_eq!(c.read_u8().unwrap(), 3);
    }

    #[test]
    fn cursor_skip_too_far() {
        let mut c = Cursor::new(&[1, 2]);
        assert!(c.skip(5).is_err());
    }

    #[test]
    fn cursor_read_sized_2b() {
        let mut c = Cursor::new(&[0x00, 0x03, b'a', b'b', b'c']);
        assert_eq!(c.read_sized_2b().unwrap(), b"abc");
    }

    #[test]
    fn cursor_read_sized_2b_empty() {
        let mut c = Cursor::new(&[0x00, 0x00]);
        assert_eq!(c.read_sized_2b().unwrap(), b"");
    }

    #[test]
    fn cursor_read_sized_2b_truncated_payload() {
        let mut c = Cursor::new(&[0x00, 0x05, b'a', b'b']);
        assert!(c.read_sized_2b().is_err());
    }
}

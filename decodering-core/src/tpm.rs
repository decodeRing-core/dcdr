use aes::Aes128;
use aes::cipher::{Array, KeyIvInit};
use rand_08::RngCore;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

use crate::error::TpmVerifyError;

use hmac::{Hmac, Mac};
use rsa::pkcs8::{DecodePublicKey, EncodePublicKey, LineEnding};
use rsa::{BigUint, Oaep, RsaPublicKey};

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

// TCG Algorithm Registry IDs
const TPM_ALG_RSA: u16 = 0x0001;
const TPM_ALG_NULL: u16 = 0x0010;
const TPM_ALG_SHA256: u16 = 0x000B;

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
        HmacSha256::new_from_slice(&hmac_key).map_err(|_| TpmVerifyError::InvalidQuote)?;
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
        let mut mac = HmacSha256::new_from_slice(key).map_err(|_| TpmVerifyError::InvalidQuote)?;
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

#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::collections::HashMap;

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

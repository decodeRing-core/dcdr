use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

use crate::{crypto::encode_hex, error::TpmVerifyError};

#[derive(Deserialize, Debug)]
pub struct TpmMaterial {
    pub ek_pubkey_pem: String,
    #[serde(default)]
    pub ek_cert_pem: Option<String>,
    #[serde(default)]
    pub expected_pcrs: Option<HashMap<u8, String>>,
    #[serde(default)]
    pub require_ek_cert: bool,
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
    // Only SHA-256 is supported here; extend if you need SHA-1 or SHA-384.
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

    // Check 1: recomputed digest matches what the TPM signed.
    let recomputed_digest = Sha256::digest(&concatenated);
    if recomputed_digest.as_slice() != expected_digest {
        tracing::error!(
            "recomputed PCR digest does not match quote's pcrDigest \
             (PCR values sent do not match what the TPM signed over)"
        );
        return Err(TpmVerifyError::PcrMismatch);
    }

    // Check 2: each PCR pinned by the policy matches the sent value.
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

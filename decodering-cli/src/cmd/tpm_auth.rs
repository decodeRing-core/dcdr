//! tpm-auth subcommand
//!
//! Produces a TPM 2.0 quote attestation payload, signed by the PERSISTENT AK
//! pinned at enrollment (created by `tpm-params`, activated by `tpm-activate`).
//! Does NOT create keys — the server pinned this exact AK, so the quote must be
//! signed by it. Writes JSON to --out FILE, or stdout if omitted. Pass
//! `--debug`/`-d` for progress messages.
//!
//! Output JSON shape:
//!   {
//!     "challenge_id":  "<uuid>",
//!     "ek_pubkey_pem": "<PEM>",
//!     "ak_pubkey_pem": "<PEM>",
//!     "quote":         "<base64 TPMS_ATTEST>",
//!     "signature":     "<base64 TPMT_SIGNATURE>",
//!     "pcrs":          { "0": "<hex>", ... }
//!   }
//!
//! Env vars:
//!   EK_HANDLE       persistent EK handle   (default 0x81010001)
//!   AK_HANDLE       persistent AK handle   (default 0x81010002)
//!   TPM2TOOLS_TCTI  TCTI string            (falls back to /dev/tpmrm0, /dev/tpm0)
//!
//! ============================ ASSUMPTIONS ============================
//!  1. `tpm-params` has run against this TPM, so a compatible EK and the
//!     enrolled AK are persisted at EK_HANDLE / AK_HANDLE.
//!  2. EK and AK are RSA keys; the AK signs with RSASSA over SHA-256, matching
//!     how `tpm-params` created it (`tpm2_createak -g sha256 -s rsassa`).
//!  3. The quote covers the SHA-256 bank, indices 0..=7.
//!  4. The AK has empty auth (no password), satisfied with a null-auth session.
//!  5. The nonce is the verifier's qualifying data: hex is decoded, anything
//!     else is taken as raw UTF-8 bytes. Max 64 bytes for a SHA-256 quote.
//!  6. One-shot process: sessions are released when the Context drops.
//! ====================================================================

use std::env;
use std::error::Error;
use std::fmt;
use std::path::Path;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;
use rsa::pkcs8::{EncodePublicKey, LineEnding};
use rsa::{BigUint, RsaPublicKey};
use serde::Serialize;
use serde_json::{Map, Value};

use tss_esapi::handles::{KeyHandle, ObjectHandle, PersistentTpmHandle, TpmHandle};
use tss_esapi::interface_types::algorithm::HashingAlgorithm;
use tss_esapi::structures::{
    Attest, Data, HashScheme, PcrSelectionListBuilder, PcrSlot, Public, Signature, SignatureScheme,
};
use tss_esapi::tcti_ldr::DeviceConfig;
use tss_esapi::traits::Marshall;
use tss_esapi::{Context, TctiNameConf};

type Result<T> = core::result::Result<T, Box<dyn Error>>;

trait ErrCtx<T> {
    fn context<S: fmt::Display>(self, msg: S) -> Result<T>;
    fn with_context<S, F>(self, f: F) -> Result<T>
    where
        S: fmt::Display,
        F: FnOnce() -> S;
}

impl<T, E> ErrCtx<T> for core::result::Result<T, E>
where
    E: Error + 'static,
{
    fn context<S: fmt::Display>(self, msg: S) -> Result<T> {
        self.map_err(|e| -> Box<dyn Error> { format!("{msg}: {e}").into() })
    }
    fn with_context<S, F>(self, f: F) -> Result<T>
    where
        S: fmt::Display,
        F: FnOnce() -> S,
    {
        self.map_err(|e| -> Box<dyn Error> { format!("{}: {e}", f()).into() })
    }
}

static DEBUG: AtomicBool = AtomicBool::new(false);

fn step(msg: impl fmt::Display) {
    if DEBUG.load(Ordering::Relaxed) {
        let _ = cliclack::log::info(msg);
    }
}

const PCR_BANK: HashingAlgorithm = HashingAlgorithm::Sha256;
const PCR_SLOTS: &[(u32, PcrSlot)] = &[
    (0, PcrSlot::Slot0),
    (1, PcrSlot::Slot1),
    (2, PcrSlot::Slot2),
    (3, PcrSlot::Slot3),
    (4, PcrSlot::Slot4),
    (5, PcrSlot::Slot5),
    (6, PcrSlot::Slot6),
    (7, PcrSlot::Slot7),
];

#[derive(Serialize)]
struct AuthPayload {
    challenge_id: String,
    ek_pubkey_pem: String,
    ak_pubkey_pem: String,
    quote: String,
    signature: String,
    pcrs: Value,
}

pub fn run(nonce: &str, challenge_id: &str, out: &Path, debug: bool) -> Result<()> {
    if debug {
        DEBUG.store(true, Ordering::Relaxed);
    }
    let payload = generate(nonce, challenge_id)?;
    let json = serde_json::to_string_pretty(&payload)?;
    std::fs::write(out, &json).with_context(|| format!("failed to write {}", out.display()))?;
    let _ = cliclack::log::success(format!("Auth payload (written to {})", out.display()));
    Ok(())
}

/// Build the attestation payload: read EK/AK pubkeys, quote PCRs with the nonce
/// as qualifying data, and read the raw PCR values.
pub fn generate(nonce: &str, challenge_id: &str) -> Result<AuthPayload> {
    let ek_handle_raw = parse_handle("EK_HANDLE", 0x8101_0001)?;
    let ak_handle_raw = parse_handle("AK_HANDLE", 0x8101_0002)?;

    let qualifying_data =
        Data::try_from(normalize_nonce(nonce)?).context("nonce rejected as qualifying data")?;

    let mut ctx = Context::new(tcti()?).context("failed to open TPM context")?;

    step("Reading persistent EK and AK");
    let ek_handle = load_key(&mut ctx, ek_handle_raw, "EK")?;
    let ak_handle = load_key(&mut ctx, ak_handle_raw, "AK")?;

    let ek_pubkey_pem = read_pubkey_pem(&mut ctx, ek_handle, "EK")?;
    let ak_pubkey_pem = read_pubkey_pem(&mut ctx, ak_handle, "AK")?;

    step("Quoting PCRs (sha256:0..=7) with the pinned AK");
    let slots: Vec<PcrSlot> = PCR_SLOTS.iter().map(|(_, s)| *s).collect();
    let selection = PcrSelectionListBuilder::new()
        .with_selection(PCR_BANK, &slots)
        .build()?;
    let scheme = SignatureScheme::RsaSsa {
        hash_scheme: HashScheme::new(HashingAlgorithm::Sha256),
    };

    let (attest, signature): (Attest, Signature) = ctx
        .execute_with_nullauth_session(|ctx| {
            ctx.quote(ak_handle, qualifying_data, scheme, selection)
        })
        .context("tpm2_quote failed")?;

    let quote = B64.encode(
        attest
            .marshall()
            .context("marshal quote (TPMS_ATTEST) failed")?,
    );
    let signature = B64.encode(
        signature
            .marshall()
            .context("marshal signature (TPMT_SIGNATURE) failed")?,
    );

    step("Reading PCRs as hex map");
    let pcrs = read_pcrs(&mut ctx)?;

    Ok(AuthPayload {
        challenge_id: challenge_id.to_owned(),
        ek_pubkey_pem,
        ak_pubkey_pem,
        quote,
        signature,
        pcrs,
    })
}

/// Normalize the nonce: hex -> decoded bytes, otherwise raw UTF-8. Bounded to
/// 64 bytes, the limit for a SHA-256 quote's qualifying data.
fn normalize_nonce(nonce: &str) -> Result<Vec<u8>> {
    let is_hex =
        !nonce.is_empty() && nonce.len() % 2 == 0 && nonce.chars().all(|c| c.is_ascii_hexdigit());
    let bytes = if is_hex {
        step(format!(
            "Nonce looks like hex ({} chars), decoding",
            nonce.len()
        ));
        hex::decode(nonce).context("nonce hex decode failed")?
    } else {
        step(format!(
            "Nonce treated as raw string ({} bytes)",
            nonce.len()
        ));
        nonce.as_bytes().to_vec()
    };
    if bytes.len() > 64 {
        return Err(format!("nonce is {} bytes; max 64 for SHA-256 quotes", bytes.len()).into());
    }
    Ok(bytes)
}

fn read_pubkey_pem(ctx: &mut Context, handle: KeyHandle, label: &str) -> Result<String> {
    let (public, _, _) = ctx
        .read_public(handle)
        .with_context(|| format!("tpm2_readpublic on {label} failed"))?;
    rsa_public_to_pem(&public)
        .map_err(|e| -> Box<dyn Error> { format!("{label} -> PEM conversion failed: {e}").into() })
}

fn load_key(ctx: &mut Context, raw: u32, label: &str) -> Result<KeyHandle> {
    let persistent = PersistentTpmHandle::new(raw)?;
    let object = lookup_persistent(ctx, persistent).ok_or_else(|| -> Box<dyn Error> {
        format!("no {label} at {raw:#010x}; run `tpm-params` first").into()
    })?;
    Ok(KeyHandle::from(object))
}

fn lookup_persistent(ctx: &mut Context, handle: PersistentTpmHandle) -> Option<ObjectHandle> {
    ctx.execute_without_session(|ctx| ctx.tr_from_tpm_public(TpmHandle::Persistent(handle)))
        .ok()
}

fn rsa_public_to_pem(public: &Public) -> Result<String> {
    let (modulus, exp): (&[u8], u32) = match public {
        Public::Rsa {
            unique, parameters, ..
        } => (unique.value(), parameters.exponent().value()),
        _ => return Err("key is not an RSA key".into()),
    };
    let n = BigUint::from_bytes_be(modulus);
    let e = if exp == 0 {
        BigUint::from(65_537u32)
    } else {
        BigUint::from(exp)
    };
    let key = RsaPublicKey::new(n, e).context("invalid RSA public parameters")?;
    Ok(key.to_public_key_pem(LineEnding::LF)?)
}

fn read_pcrs(ctx: &mut Context) -> Result<Value> {
    let slots: Vec<PcrSlot> = PCR_SLOTS.iter().map(|(_, s)| *s).collect();
    let selection = PcrSelectionListBuilder::new()
        .with_selection(PCR_BANK, &slots)
        .build()?;
    let (_counter, _read, digests) = ctx.pcr_read(selection)?;
    let digests = digests.value();
    if digests.len() != PCR_SLOTS.len() {
        return Err(format!(
            "expected {} PCRs, TPM returned {}",
            PCR_SLOTS.len(),
            digests.len()
        )
        .into());
    }
    let mut map = Map::new();
    for ((idx, _slot), digest) in PCR_SLOTS.iter().zip(digests) {
        map.insert(idx.to_string(), Value::String(hex::encode(digest.value())));
    }
    Ok(Value::Object(map))
}

fn tcti() -> Result<TctiNameConf> {
    if let Ok(t) = TctiNameConf::from_environment_variable() {
        return Ok(t);
    }
    for dev in ["/dev/tpmrm0", "/dev/tpm0"] {
        if Path::new(dev).exists() {
            return Ok(TctiNameConf::Device(DeviceConfig::from_str(dev)?));
        }
    }
    Err(
        "no TPM device (/dev/tpm0, /dev/tpmrm0) and TPM2TOOLS_TCTI unset.\n\
         Point TPM2TOOLS_TCTI at a simulator or map a host TPM."
            .into(),
    )
}

fn parse_handle(var: &str, default: u32) -> Result<u32> {
    match env::var(var) {
        Ok(s) => {
            let s = s.trim();
            let s = s
                .strip_prefix("0x")
                .or_else(|| s.strip_prefix("0X"))
                .unwrap_or(s);
            u32::from_str_radix(s, 16).with_context(|| format!("invalid {var}: {s}"))
        }
        Err(_) => Ok(default),
    }
}

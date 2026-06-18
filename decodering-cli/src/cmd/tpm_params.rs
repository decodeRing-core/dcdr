//! tpm-params subcommand
//!
//! Talks to the local TPM 2.0 device directly through the TSS ESAPI (via the
//! `tss-esapi` crate) and prints a JSON params block to stdout (and nothing
//! else). Pass `--debug`/`-d` for progress messages on stderr.
//!
//! Output JSON shape:
//!   {
//!     "tpm": {
//!       "ek_pubkey_pem":       "<PEM>",
//!       "ek_cert_pem":         "<PEM>" | null,
//!       "ak_public_tpm2b_b64": "<base64 of marshaled TPM2B_PUBLIC>",
//!       "expected_pcrs":       { "0": "<hex>", ... },
//!       "require_ek_cert":     true | false
//!     }
//!   }
//!
//! Env vars:
//!   EK_HANDLE       persistent EK handle   (default 0x81010001)
//!   AK_HANDLE       persistent AK handle   (default 0x81010002)
//!   TPM2TOOLS_TCTI  TCTI string            (falls back to /dev/tpmrm0, /dev/tpm0)
//!
//! ============================ ASSUMPTIONS ============================
//!  1. A TPM 2.0 device is reachable, either via TPM2TOOLS_TCTI or one of the
//!     standard device nodes (/dev/tpmrm0 preferred, then /dev/tpm0).
//!  2. EK and AK are RSA keys. The asymmetric algorithm is hardcoded to RSA;
//!     ECC TPMs are not handled.
//!  3. The relevant PCR bank is SHA-256 and only indices 0..=7 matter. All 8
//!     are read in a single pcr_read call and returned in ascending order.
//!  4. The owner and endorsement hierarchies use the empty (null) auth value,
//!     and the EK/AK are created with no auth value. evict_control and the EK
//!     policy are satisfied with null-auth sessions accordingly.
//!  5. The EK uses the standard EK auth policy, i.e. loading a child requires a
//!     single TPM2_PolicySecret against the endorsement hierarchy.
//!  6. If a persistent object already exists at EK_HANDLE it is a compatible EK
//!     and is reused as-is (its template is not re-verified). Any object at
//!     AK_HANDLE is treated as a stale AK and evicted before persisting a new one.
//!  7. Absence of an EK certificate means a virtual TPM, so require_ek_cert is
//!     set to false; presence of a cert sets it to true.
//!  8. The marshaled AK public matches `tpm2_createak -u`: a TPM2B_PUBLIC, i.e.
//!     a 2-byte big-endian size prefix followed by the marshaled TPMT_PUBLIC.
//!  9. This is a one-shot process: outstanding handles (incl. the policy
//!     session) are released when the Context is dropped, so no explicit
//!     flush_context is performed.
//! 10. The byte accessor on tss-esapi buffer types is `.value()` (it is
//!     `.as_bytes()` on some versions — adjust if the crate version differs).
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

use tss_esapi::abstraction::{ak, ek};
use tss_esapi::constants::SessionType;
use tss_esapi::handles::{AuthHandle, KeyHandle, ObjectHandle, PersistentTpmHandle, TpmHandle};
use tss_esapi::interface_types::algorithm::{
    AsymmetricAlgorithm, HashingAlgorithm, SignatureSchemeAlgorithm,
};
use tss_esapi::interface_types::dynamic_handles::Persistent;
use tss_esapi::interface_types::resource_handles::Provision;
use tss_esapi::interface_types::session_handles::PolicySession;
use tss_esapi::structures::{
    CreateKeyResult, Digest, Nonce, PcrSelectionListBuilder, PcrSlot, Public, SymmetricDefinition,
};
use tss_esapi::tcti_ldr::DeviceConfig;
use tss_esapi::traits::Marshall;
use tss_esapi::{Context, TctiNameConf};

/// Local result alias so signatures stay short (replaces `anyhow::Result`).
type Result<T> = core::result::Result<T, Box<dyn Error>>;

/// Adds `.context()` / `.with_context()` to `Result`, mirroring anyhow's
/// `Context` trait. Named `ErrCtx` to avoid clashing with `tss_esapi::Context`.
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

/// Whether to emit progress messages to stderr (off unless `--debug`/`-d`).
static DEBUG: AtomicBool = AtomicBool::new(false);

/// eprintln! that only fires when --debug is set. Keeps stdout JSON-only.
macro_rules! dlog {
    ($($arg:tt)*) => {
        if DEBUG.load(Ordering::Relaxed) {
            eprintln!($($arg)*);
        }
    };
}

// PCRs read: sha256 bank, indices 0..=7.
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
struct Output {
    tpm: TpmParams,
}

#[derive(Serialize)]
struct TpmParams {
    ek_pubkey_pem: String,
    ek_cert_pem: Option<String>,
    ak_public_tpm2b_b64: String,
    expected_pcrs: Value,
    require_ek_cert: bool,
}

pub fn run(debug: bool) -> Result<()> {
    if debug {
        DEBUG.store(true, Ordering::Relaxed);
    }

    let ek_handle_raw = parse_handle("EK_HANDLE", 0x8101_0001)?;
    let ak_handle_raw = parse_handle("AK_HANDLE", 0x8101_0002)?;

    let mut ctx = Context::new(tcti()?).context("failed to open TPM context")?;

    // 1. EK: reuse if already persisted, else create + persist.
    dlog!("[*] Creating EK and extracting public key...");
    let ek_persistent = PersistentTpmHandle::new(ek_handle_raw)?;
    let ek_handle = get_or_create_ek(&mut ctx, ek_persistent)?;

    let (ek_public, _, _) = ctx
        .read_public(ek_handle)
        .context("tpm2_readpublic on EK failed")?;

    let ek_pubkey_pem = rsa_public_to_pem(&ek_public)
        .map_err(|e| std::io::Error::other(e.to_string()))
        .context("EK -> PEM conversion failed")?;

    // EK certificate: present on a physical TPM, absent on a vTPM.
    let ek_cert_pem = match ek::retrieve_ek_pubcert(&mut ctx, AsymmetricAlgorithm::Rsa) {
        Ok(der) if !der.is_empty() => Some(der_cert_to_pem(&der)?),
        _ => None,
    };
    let require_ek_cert = ek_cert_pem.is_some();

    // 2. AK under EK, persisted at AK_HANDLE.
    dlog!(
        "[*] Creating AK under EK and persisting at {:#010x}...",
        ak_handle_raw
    );
    let ak = ak::create_ak(
        &mut ctx,
        ek_handle,
        HashingAlgorithm::Sha256,
        SignatureSchemeAlgorithm::RsaSsa,
        None, // no AK auth value
        None, // no key customization
    )
    .context("tpm2_createak failed")?;

    let ak_persistent = PersistentTpmHandle::new(ak_handle_raw)?;
    persist_ak(&mut ctx, ek_handle, ak_persistent, &ak)?;

    // Marshaled TPM2B_PUBLIC (2-byte BE size prefix + TPMT_PUBLIC), then base64.
    let tpmt = ak
        .out_public
        .marshall()
        .context("marshal AK public failed")?;
    let mut tpm2b = (tpmt.len() as u16).to_be_bytes().to_vec();
    tpm2b.extend_from_slice(&tpmt);
    let ak_public_tpm2b_b64 = B64.encode(&tpm2b);

    // 3. Expected PCRs.
    dlog!("[*] Reading PCRs (sha256:0,1,2,3,4,5,6,7)...");
    let expected_pcrs = read_pcrs(&mut ctx)?;

    // 4. Assemble JSON and write it to stdout (and nothing else).
    dlog!("[*] Building JSON output...");
    let output = Output {
        tpm: TpmParams {
            ek_pubkey_pem,
            ek_cert_pem,
            ak_public_tpm2b_b64,
            expected_pcrs,
            require_ek_cert,
        },
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Pick a TCTI: honor TPM2TOOLS_TCTI, else fall back to a device node.
fn tcti() -> Result<TctiNameConf> {
    if let Ok(t) = TctiNameConf::from_environment_variable() {
        return Ok(t);
    }
    for dev in ["/dev/tpmrm0", "/dev/tpm0"] {
        if Path::new(dev).exists() {
            return Ok(TctiNameConf::Device(DeviceConfig::from_str(dev)?));
        }
    }
    Err(format!(
        "no TPM device (/dev/tpm0, /dev/tpmrm0) and TPM2TOOLS_TCTI unset.\n\
         Map a host TPM in (--device /dev/tpmrm0) or point TPM2TOOLS_TCTI at a simulator."
    )
    .into())
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

/// Reuse the EK if it is already persisted at `handle`, otherwise create a
/// fresh EK and evict it to that persistent handle.
fn get_or_create_ek(ctx: &mut Context, handle: PersistentTpmHandle) -> Result<KeyHandle> {
    if let Some(existing) = lookup_persistent(ctx, handle) {
        dlog!("    EK already present at {handle:?}, reusing.");
        return Ok(KeyHandle::from(existing));
    }

    // Transient EK from the standard RSA EK template.
    let transient = ek::create_ek_object(ctx, AsymmetricAlgorithm::Rsa, None)
        .context("tpm2_createek failed")?;

    // Persist it (needs owner authorization -> null-auth session).
    let persisted = ctx.execute_with_nullauth_session(|ctx| {
        ctx.evict_control(
            Provision::Owner,
            transient.into(),
            Persistent::Persistent(handle),
        )
    })?;
    Ok(KeyHandle::from(persisted))
}

/// Load the freshly created AK under the EK and persist it at `handle`,
/// dropping any stale object already sitting there.
fn persist_ak(
    ctx: &mut Context,
    ek_handle: KeyHandle,
    handle: PersistentTpmHandle,
    ak: &CreateKeyResult,
) -> Result<()> {
    // Drop an AK left over from a previous run.
    if let Some(stale) = lookup_persistent(ctx, handle) {
        ctx.execute_with_nullauth_session(|ctx| {
            ctx.evict_control(Provision::Owner, stale, Persistent::Persistent(handle))
        })?;
    }

    // Loading a child of the EK requires satisfying the EK's auth policy
    // (TPM2_PolicySecret against the endorsement hierarchy).
    let session = ctx
        .start_auth_session(
            None,
            None,
            None,
            SessionType::Policy,
            SymmetricDefinition::AES_128_CFB,
            HashingAlgorithm::Sha256,
        )?
        .ok_or_else(|| "TPM returned empty policy session handle".to_string())?;
    let policy_session = PolicySession::try_from(session)?;

    ctx.execute_with_nullauth_session(|ctx| {
        ctx.policy_secret(
            policy_session,
            AuthHandle::Endorsement,
            Nonce::default(),
            Digest::default(),
            Nonce::default(),
            None,
        )
    })?;

    let ak_handle = ctx.execute_with_session(Some(session), |ctx| {
        ctx.load(ek_handle, ak.out_private.clone(), ak.out_public.clone())
    })?;

    // The Context closes outstanding handles (incl. this policy session) on drop.
    ctx.execute_with_nullauth_session(|ctx| {
        ctx.evict_control(
            Provision::Owner,
            ak_handle.into(),
            Persistent::Persistent(handle),
        )
    })?;
    Ok(())
}

/// Return the ObjectHandle for a persistent handle if it exists, else None.
fn lookup_persistent(ctx: &mut Context, handle: PersistentTpmHandle) -> Option<ObjectHandle> {
    ctx.execute_without_session(|ctx| ctx.tr_from_tpm_public(TpmHandle::Persistent(handle)))
        .ok()
}

/// Build an SPKI ("BEGIN PUBLIC KEY") PEM from an RSA TPM public area,
/// matching `tpm2_readpublic -f pem`.
fn rsa_public_to_pem(public: &Public) -> Result<String> {
    let (modulus, exp): (&[u8], u32) = match public {
        Public::Rsa {
            unique, parameters, ..
        } => (unique.value(), parameters.exponent().value()),
        _ => return Err("EK is not an RSA key".to_string().into()),
    };

    let n = BigUint::from_bytes_be(modulus);
    let e = if exp == 0 {
        BigUint::from(65_537u32) // TPM convention: 0 means the default 2^16 + 1
    } else {
        BigUint::from(exp)
    };

    let key = RsaPublicKey::new(n, e).context("invalid RSA EK parameters")?;
    Ok(key.to_public_key_pem(LineEnding::LF)?)
}

/// DER X.509 certificate -> PEM, matching `openssl x509 -inform der`.
fn der_cert_to_pem(der: &[u8]) -> Result<String> {
    let b64 = B64.encode(der);
    let body = b64
        .as_bytes()
        .chunks(64)
        .map(|c| std::str::from_utf8(c).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    Ok(format!(
        "-----BEGIN CERTIFICATE-----\n{body}\n-----END CERTIFICATE-----\n"
    ))
}

/// Read sha256 PCRs 0..=7 into a JSON object {"0":"<hex>", ...}, lowercase hex,
/// numeric key order preserved (serde_json preserve_order feature).
fn read_pcrs(ctx: &mut Context) -> Result<Value> {
    let slots: Vec<PcrSlot> = PCR_SLOTS.iter().map(|(_, s)| *s).collect();
    let selection = PcrSelectionListBuilder::new()
        .with_selection(PCR_BANK, &slots)
        .build()?;

    let (_counter, _read, digests) = ctx.pcr_read(selection)?;
    let digests = digests.value(); // &[Digest]
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

//! tpm-activate subcommand
//!
//! Recovers the activation secret from a create_app_user (makecredential)
//! response by running TPM2_ActivateCredential against the persistent EK/AK
//! created by `tpm-params`. Pass `--debug`/`-d` for progress messages.
//!
//! This base64 value is the `recovered_secret` posted to
//! /app/user/auth/activate.
//!
//! Env vars:
//!   EK_HANDLE       persistent EK handle   (default 0x81010001)
//!   AK_HANDLE       persistent AK handle   (default 0x81010002)
//!   TPM2TOOLS_TCTI  TCTI string            (falls back to /dev/tpmrm0, /dev/tpm0)
//!
//! ============================ ASSUMPTIONS ============================
//!  1. `tpm-params` has already run against this TPM, so a compatible EK and
//!     AK are persisted at EK_HANDLE / AK_HANDLE.
//!  2. `credential_blob` and `secret` are base64 of the size-prefixed
//!     TPM2B_ID_OBJECT and TPM2B_ENCRYPTED_SECRET respectively, i.e. their
//!     marshalled wire form — so they unmarshall directly. (If a server ever
//!     sends the bare buffer with no 2-byte prefix, switch unmarshall() for
//!     the matching TryFrom and prepend the length.)
//!  3. The EK has the standard EK auth policy (PolicySecret against the
//!     endorsement hierarchy); the AK has empty auth, satisfied with a
//!     password session.
//!  4. One-shot process: the policy session is released when the Context is
//!     dropped, so no explicit flush_context is performed.
//! ====================================================================

use std::env;
use std::error::Error;
use std::fmt;
use std::path::Path;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;

use tss_esapi::constants::SessionType;
use tss_esapi::handles::{AuthHandle, KeyHandle, ObjectHandle, PersistentTpmHandle, TpmHandle};
use tss_esapi::interface_types::algorithm::HashingAlgorithm;
use tss_esapi::interface_types::session_handles::{AuthSession, PolicySession};
use tss_esapi::structures::{Digest, EncryptedSecret, IdObject, Nonce, SymmetricDefinition};
use tss_esapi::tcti_ldr::DeviceConfig;
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

pub fn run(credential_blob_b64: &str, secret_b64: &str, debug: bool) -> Result<()> {
    if debug {
        DEBUG.store(true, Ordering::Relaxed);
    }
    let recovered = recover(credential_blob_b64, secret_b64)?;
    let _ = cliclack::log::success(format!("Recovered secret {recovered}"));
    Ok(())
}

/// Run activatecredential and return the recovered secret as base64.
pub fn recover(credential_blob_b64: &str, secret_b64: &str) -> Result<String> {
    let ek_handle_raw = parse_handle("EK_HANDLE", 0x8101_0001)?;
    let ak_handle_raw = parse_handle("AK_HANDLE", 0x8101_0002)?;

    let credential_blob = IdObject::try_from(strip_tpm2b(
        decode_b64(credential_blob_b64).context("bad credential_blob")?,
    )?)
    .context("credential_blob is not a valid TPM2B_ID_OBJECT")?;

    let secret =
        EncryptedSecret::try_from(strip_tpm2b(decode_b64(secret_b64).context("bad secret")?)?)
            .context("secret is not a valid TPM2B_ENCRYPTED_SECRET")?;

    let mut ctx = Context::new(tcti()?).context("failed to open TPM context")?;

    step("Loading persistent EK and AK");
    let ek_handle = load_key(&mut ctx, ek_handle_raw, "EK")?;
    let ak_handle = load_key(&mut ctx, ak_handle_raw, "AK")?;

    step("Building EK policy session (PolicySecret against endorsement)");
    let policy_session = ctx
        .start_auth_session(
            None,
            None,
            None,
            SessionType::Policy,
            SymmetricDefinition::AES_128_CFB,
            HashingAlgorithm::Sha256,
        )?
        .ok_or("TPM returned empty policy session handle")?;
    let policy = PolicySession::try_from(policy_session)?;
    ctx.execute_with_nullauth_session(|ctx| {
        ctx.policy_secret(
            policy,
            AuthHandle::Endorsement,
            Nonce::default(),
            Digest::default(),
            Nonce::default(),
            None,
        )
    })?;

    step("Activating credential");
    // Session 1 authorizes the AK (empty auth -> password); session 2
    // authorizes the EK (the policy session just built).
    let recovered: Digest = ctx
        .execute_with_sessions(
            (Some(AuthSession::Password), Some(policy_session), None),
            |ctx| ctx.activate_credential(ak_handle, ek_handle, credential_blob, secret),
        )
        .context("tpm2_activatecredential failed")?;

    Ok(B64.encode(recovered.value()))
}

fn decode_b64(s: &str) -> core::result::Result<Vec<u8>, base64::DecodeError> {
    let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    B64.decode(cleaned)
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

fn strip_tpm2b(buf: Vec<u8>) -> Result<Vec<u8>> {
    let len = buf.get(..2).ok_or("TPM2B too short")?;
    let n = u16::from_be_bytes([len[0], len[1]]) as usize;
    let body = buf.get(2..2 + n).ok_or("TPM2B length exceeds buffer")?;
    Ok(body.to_vec())
}

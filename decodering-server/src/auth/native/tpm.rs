// decodering-server/src/auth/native/tpm.rs
use async_trait::async_trait;

use decodering_core::auth::method::{AuthMethod, Capabilities};
use decodering_core::auth::types::{
    ActivateRequest, ActivateResponse, AuthError, AuthRequest, AuthResponse, ChallengeRequest,
    ChallengeResponse, EnrollRequest, EnrollResponse, ResolveRequest,
};
use decodering_core::cert::{TpmTrustStore, verify_ek_cert_chain};
use decodering_core::crypto::{
    base64_decode, base64_encode, encode_hex, pem_to_der, sha256_hex, verify_quote_signature,
};
use decodering_core::domain::PrincipalStatus;
use decodering_core::tpm::{AkPublic, make_credential_rsa, parse_tpms_attest, verify_pcrs};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub struct TpmMethod;

impl TpmMethod {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TpmMethod {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
struct TpmEnrollData {
    ek_pubkey_pem: String,
    #[serde(default)]
    ek_cert_pem: Option<String>,
    ak_public_tpm2b_b64: String,
    #[serde(default)]
    expected_pcrs: Option<HashMap<u8, String>>,
    #[serde(default)]
    require_ek_cert: bool,
}

#[derive(Serialize, Deserialize)]
struct TpmCredentialMaterial {
    ek_pubkey_pem: String,
    ek_cert_pem: Option<String>,
    require_ek_cert: bool,
    expected_pcrs: Option<HashMap<u8, String>>,
    ak_pubkey_pem: String,
    ak_name_hex: String,
    activation_secret_hash: String,
}

/// `AuthRequest.proof` / `ResolveRequest.proof` — the /auth/tpm body.
#[derive(Deserialize)]
struct TpmAuthProof {
    challenge_id: String,
    ek_pubkey_pem: String,
    ak_pubkey_pem: String,
    quote: String,
    signature: String,
    #[serde(default)]
    pcrs: Option<HashMap<u8, String>>,
}

/// `ActivateRequest.proof` — the /tpm/activate body's `recovered_secret`.
#[derive(Deserialize)]
struct TpmActivateProof {
    recovered_secret: String, // base64
}

#[async_trait]
impl AuthMethod for TpmMethod {
    fn kind(&self) -> String {
        "trustedPlatformModule".to_owned()
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            kind: self.kind(),
            requires_activation: true, // MakeCredential / ActivateCredential
            requires_challenge: true,  // nonce before auth
            requires_resolve: true,    // need stored EK material to verify
        }
    }

    async fn enroll(&self, req: EnrollRequest) -> Result<EnrollResponse, AuthError> {
        let data: TpmEnrollData = serde_json::from_value(req.data)
            .map_err(|e| AuthError::InvalidEnrollmentData(e.to_string()))?;

        // lookup_key = sha256(EK DER)
        let ek_der = pem_to_der(&data.ek_pubkey_pem)
            .map_err(|e| AuthError::InvalidEnrollmentData(format!("bad EK: {e}")))?;
        let lookup_key = sha256_hex(&ek_der);

        // Optional EK cert chain check (deferred path; matches your require_ek_cert)
        if data.require_ek_cert {
            let cert = data.ek_cert_pem.as_ref().ok_or_else(|| {
                AuthError::InvalidEnrollmentData("require_ek_cert set but no cert".to_owned())
            })?;
            let tpm_trust_dir = req
                .config
                .get("tpm_trust_dir")
                .and_then(|v| v.as_str())
                .map(String::from)
                .unwrap_or_default();
            let trust_store = TpmTrustStore::from_directory(&tpm_trust_dir)
                .map_err(|_| AuthError::Internal("trust store load failed".to_owned()))?;
            verify_ek_cert_chain(cert, &data.ek_pubkey_pem, &trust_store)
                .map_err(|e| AuthError::VerificationFailed(format!("EK cert: {e:#?}")))?;
        }

        // Parse AK from the TPM2B, derive its name
        let ak_pub_2b = base64_decode(&data.ak_public_tpm2b_b64)
            .ok_or_else(|| AuthError::InvalidEnrollmentData("AK b64".to_owned()))?;
        let ak = AkPublic::parse(&ak_pub_2b)
            .map_err(|e| AuthError::InvalidEnrollmentData(format!("parse AK: {e}")))?;

        // MakeCredential: random secret sealed to EK, bound to AK name
        let mut secret = [0u8; 32];
        rand::rng().fill_bytes(&mut secret);
        let challenge = make_credential_rsa(&data.ek_pubkey_pem, &ak.name, &secret)
            .map_err(|e| AuthError::Internal(format!("make_credential: {e}")))?;

        let material = TpmCredentialMaterial {
            ek_pubkey_pem: data.ek_pubkey_pem,
            ek_cert_pem: data.ek_cert_pem,
            require_ek_cert: data.require_ek_cert,
            expected_pcrs: data.expected_pcrs,
            ak_pubkey_pem: ak.pubkey_pem,
            ak_name_hex: encode_hex(&ak.name),
            activation_secret_hash: sha256_hex(&secret),
        };

        Ok(EnrollResponse {
            lookup_key,
            secret_material: serde_json::to_value(material)
                .map_err(|e| AuthError::Internal(e.to_string()))?,
            status: PrincipalStatus::Pending,
            client_payload: Some(serde_json::json!({
                "credential_blob": base64_encode(challenge.credential_blob),
                "secret": base64_encode(challenge.secret),
            })),
        })
    }

    async fn activate(&self, req: ActivateRequest) -> Result<ActivateResponse, AuthError> {
        // Host supplies the credential's secret_material here.
        let material: TpmCredentialMaterial = serde_json::from_value(req.credential_material)
            .map_err(|e| AuthError::Internal(format!("bad material: {e}")))?;

        let proof: TpmActivateProof = serde_json::from_value(req.proof)
            .map_err(|e| AuthError::InvalidProof(e.to_string()))?;

        let recovered = base64_decode(&proof.recovered_secret)
            .ok_or_else(|| AuthError::InvalidProof("recovered_secret b64".to_owned()))?;

        // Compare hashes — we only stored the hash of the secret.
        if sha256_hex(&recovered) != material.activation_secret_hash {
            return Err(AuthError::ActivationFailed("secret mismatch".to_owned()));
        }
        Ok(ActivateResponse { activated: true })
    }

    async fn challenge(&self, req: ChallengeRequest) -> Result<ChallengeResponse, AuthError> {
        // The nonce is just host-supplied entropy. The TPM will embed it in
        // the quote's extraData and sign over it.
        let nonce = req.entropy;
        let nonce_hex = encode_hex(&nonce);

        Ok(ChallengeResponse {
            client_payload: serde_json::json!({ "nonce": nonce_hex }),
            challenge_state: nonce,
        })
    }

    /// RESOLVE: extract the `lookup_key` (sha256 of EK pubkey) from the proof
    /// WITHOUT verifying, so the host can fetch the stored credential
    /// material before we verify the quote.
    async fn resolve(&self, req: ResolveRequest) -> Result<String, AuthError> {
        let proof: TpmAuthProof = serde_json::from_value(req.proof)
            .map_err(|e| AuthError::InvalidProof(e.to_string()))?;

        let ek_der = pem_to_der(&proof.ek_pubkey_pem)
            .map_err(|e| AuthError::InvalidProof(format!("bad EK pubkey: {e}")))?;
        Ok(sha256_hex(&ek_der))
    }

    /// AUTHENTICATE: verify the quote.
    ///   1. nonce in the quote == the challenge nonce (replay protection)
    ///   2. quote signature is valid under the AK
    ///   3. the AK presented matches the EK-bound credential (via material)
    ///   4. PCRs match the quote digest and the pinned policy
    async fn authenticate(&self, req: AuthRequest) -> Result<AuthResponse, AuthError> {
        let proof: TpmAuthProof = serde_json::from_value(req.proof)
            .map_err(|e| AuthError::InvalidProof(e.to_string()))?;

        // Host supplied the stored material after resolve().
        let material_val = req
            .credential_material
            .ok_or_else(|| AuthError::Internal("missing credential_material".to_owned()))?;
        let material: TpmCredentialMaterial = serde_json::from_value(material_val)
            .map_err(|e| AuthError::Internal(format!("bad credential_material: {e}")))?;

        // The EK in the proof must match the enrolled EK (defense in depth —
        // resolve already keyed on it, but re-check against stored material).
        if proof.ek_pubkey_pem.trim() != material.ek_pubkey_pem.trim() {
            return Err(AuthError::VerificationFailed(
                "presented EK does not match enrolled EK".to_owned(),
            ));
        }

        let challenge_nonce = req.challenge_state.ok_or_else(|| {
            AuthError::Internal("missing challenge_state for TPM auth".to_owned())
        })?;

        let quote_bytes = base64_decode(&proof.quote)
            .ok_or_else(|| AuthError::InvalidProof("quote not base64".to_owned()))?;
        let sig_bytes = base64_decode(&proof.signature)
            .ok_or_else(|| AuthError::InvalidProof("signature not base64".to_owned()))?;

        // Parse TPMS_ATTEST → extract nonce (extraData), pcr digest, selections.
        let attest = parse_tpms_attest(&quote_bytes)
            .map_err(|e| AuthError::VerificationFailed(format!("bad quote: {e}")))?;

        // 1. Nonce check — the TPM signed over the challenge nonce.
        if attest.extra_data != challenge_nonce {
            return Err(AuthError::VerificationFailed(
                "quote nonce does not match challenge".to_owned(),
            ));
        }

        // 2. Signature: the AK signed the quote.
        verify_quote_signature(&quote_bytes, &sig_bytes, &proof.ak_pubkey_pem)
            .map_err(|e| AuthError::VerificationFailed(format!("signature: {e}")))?;

        // 4. PCRs: recompute the digest from the sent PCR map, compare to the
        // quote's pcrDigest, then check against the pinned policy (if any).
        if let Some(expected) = &material.expected_pcrs {
            let pcrs = proof.pcrs.as_ref().ok_or_else(|| {
                AuthError::VerificationFailed("credential pins PCRs but none sent".to_owned())
            })?;
            verify_pcrs(pcrs, &attest.pcr_digest, expected, &attest.pcr_selections)
                .map_err(|e| AuthError::VerificationFailed(format!("pcr: {e}")))?;
        }

        // lookup_key = sha256(EK). Host re-validates the credential row is
        // active/not-revoked and the principal has grants for the app.
        let ek_der = pem_to_der(&proof.ek_pubkey_pem)
            .map_err(|e| AuthError::InvalidProof(format!("bad EK pubkey: {e}")))?;

        Ok(AuthResponse {
            lookup_key: sha256_hex(&ek_der),
            audit_metadata: serde_json::json!({
                "ek_sha256_prefix": &sha256_hex(&ek_der)[..16],
            }),
        })
    }
}

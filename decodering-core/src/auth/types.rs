use serde::{Deserialize, Serialize};

use crate::domain::PrincipalStatus;

#[derive(Serialize, Deserialize)]
pub struct EnrollRequest {
    pub principal_id: String,
    /// Method-specific enrollment data, exactly the shapes from your
    /// /app/user/create payloads:
    ///   apiKey:               null / {}
    ///   trustedPlatformModule {`ek_pubkey_pem`, `ek_cert_pem`, `ak_public_tpm2b_b64`,
    ///                          `expected_pcrs`, `require_ek_cert`}
    ///   awsIdentity           {`role_arn`}
    pub data: serde_json::Value,
    pub now: i64,
    /// Method config supplied at registration (trust roots, STS endpoint).
    pub config: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EnrollResponse {
    /// `principal_credentials.lookup_key`
    ///   apiKey:  `sha256(raw_key)`
    ///   tpm:     `sha256(ek_pubkey_der)`
    ///   aws:     normalized role ARN
    pub lookup_key: String,
    /// `principal_credentials.secret_material` (opaque to host)
    pub secret_material: serde_json::Value,
    /// Active immediately, or `Pending` (TPM).
    pub status: PrincipalStatus,
    /// Returned to the caller once. Matches your `data.tpm` /
    /// `data.token` create responses:
    ///   apiKey:  {"key": "pk_..."}
    ///   tpm:     {"`credential_blob"`: "...", "secret": "..."}
    ///   aws:     null
    pub client_payload: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
pub struct ActivateRequest {
    pub principal_id: String,
    pub credential_id: String,
    pub credential_material: serde_json::Value,
    pub proof: serde_json::Value,
    pub now: i64,
    pub config: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
pub struct ActivateResponse {
    pub activated: bool,
}

#[derive(Serialize, Deserialize)]
pub struct ChallengeRequest {
    /// Optional hint about which credential (e.g. ek hash). Your challenge
    /// endpoint takes {} today; this leaves room.
    pub hint: Option<serde_json::Value>,
    pub entropy: Vec<u8>, // becomes the nonce
    pub now: i64,
    pub config: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
pub struct ChallengeResponse {
    /// Returned to client. Your challenge response's `nonce` (+ expiry the
    /// host adds).
    pub client_payload: serde_json::Value,
    /// Opaque server-side state the host persists (TTL + single-use) and
    /// hands to `authenticate()` — the nonce, essentially.
    pub challenge_state: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct ResolveRequest {
    /// The auth proof, same as AuthRequest.proof. For TPM, the method
    /// extracts the EK pubkey → sha256 → `lookup_key`, without verifying.
    pub proof: serde_json::Value,
    pub config: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
pub struct AuthRequest {
    /// The client's proof, exactly your /auth/* bodies:
    ///   apiKey: {"key": "pk_..."}
    ///   tpm:    {`challenge_id`, `ek_pubkey_pem`, `ak_pubkey_pem`, quote,
    ///            signature, pcrs}
    ///   aws:    {body, headers, method, url}
    pub proof: serde_json::Value,
    /// `challenge_state` from `challenge()`, if the method uses one (TPM).
    pub challenge_state: Option<Vec<u8>>,
    /// Stored `secret_material`, supplied by the host after `resolve()` for
    /// methods that need it (TPM's EK pubkey + `expected_pcrs`). None for
    /// self-contained methods.
    pub credential_material: Option<serde_json::Value>,
    pub now: i64,
    pub config: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthResponse {
    /// The credential `lookup_key` the host uses to find the principal.
    pub lookup_key: String,
    /// For the audit log (which role, which EK, source IP later).
    pub audit_metadata: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum AuthError {
    UnsupportedOperation,
    InvalidEnrollmentData(String),
    InvalidProof(String),
    VerificationFailed(String),
    ActivationFailed(String),
    ChallengeExpired,
    CredentialNotFound,
    Internal(String),
}

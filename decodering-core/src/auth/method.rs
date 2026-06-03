use crate::auth::types::{
    ActivateRequest, ActivateResponse, AuthError, AuthRequest, AuthResponse, ChallengeRequest,
    ChallengeResponse, EnrollRequest, EnrollResponse, ResolveRequest,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// An identity method.
///
/// Same interface whether in-process (native) or out-of-process (RPC). Everything is owned
/// and serializable; the method never calls back into the host — all needed inputs are passed in.
#[async_trait]
pub trait AuthMethod: Send + Sync {
    /// Stable identifier matching `credential_kind`: "apiKey",
    /// "trustedPlatformModule", "awsIdentity".
    fn kind(&self) -> String;

    /// Declares which optional steps this method uses, so the host knows
    /// the flow without probing. Fetched once at registration (over RPC too).
    fn capabilities(&self) -> Capabilities;

    /// ENROLL — admin creates a credential.
    async fn enroll(&self, req: EnrollRequest) -> Result<EnrollResponse, AuthError>;

    /// ACTIVATE — confirm a pending credential (TPM only).
    async fn activate(&self, req: ActivateRequest) -> Result<ActivateResponse, AuthError> {
        Err(AuthError::UnsupportedOperation)
    }

    /// CHALLENGE — issue a nonce/challenge before authentication (TPM only).
    async fn challenge(&self, req: ChallengeRequest) -> Result<ChallengeResponse, AuthError> {
        Err(AuthError::UnsupportedOperation)
    }

    /// RESOLVE — extract the credential `lookup_key` from a proof WITHOUT verifying,
    /// so the host can fetch stored credential material before `verify()`.
    async fn resolve(&self, req: ResolveRequest) -> Result<String, AuthError> {
        Err(AuthError::UnsupportedOperation)
    }

    /// AUTHENTICATE — verify a proof, resolve to a credential `lookup_key`.
    async fn authenticate(&self, req: AuthRequest) -> Result<AuthResponse, AuthError>;
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Capabilities {
    pub kind: String,
    /// TPM = true (needs activate step). API key / AWS = false.
    pub requires_activation: bool,
    /// TPM = true (needs challenge before auth). API key / AWS = false.
    pub requires_challenge: bool,
    /// TPM = true (verify needs stored EK material). API key / AWS = false.
    pub requires_resolve: bool,
}

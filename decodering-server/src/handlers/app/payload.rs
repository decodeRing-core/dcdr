use std::collections::HashMap;

use decodering_core::domain::{PrincipalCredentialKind, PrincipalKind};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize, Debug)]
pub struct CreateAppUserData {
    pub name: String,
    pub kind: PrincipalKind,
    pub credential_kind: PrincipalCredentialKind,
    pub data: Option<Value>,
    // pub tpm: Option<TrustedPlatformModuleData>,
    // pub aws: Option<AwsData>,
    pub expires_at: Option<i64>,
    pub apps: Option<Vec<String>>,
}

#[derive(Deserialize, Debug)]
pub struct TrustedPlatformModuleData {
    pub ek_pubkey_pem: String,       // the public key from the TPM
    pub ek_cert_pem: Option<String>, // optional EK certificate
    pub expected_pcrs: Option<HashMap<u8, String>>, // optional boot-state pinning
    pub require_ek_cert: bool,       // policy
    pub ak_public_tpm2b_b64: String, // marshaled TPM2B_PUBLIC of the AK
}

#[derive(Deserialize, Debug)]
pub struct AwsData {
    pub role_arn: String,
}

#[derive(Deserialize, Debug)]
pub struct CreateAppData {
    pub app_name: String,
}

#[derive(Deserialize, Debug)]
pub struct AuthUserData {
    pub kind: String,
    pub proof: Value,
    //pub key: String,
}

#[derive(Deserialize, Debug)]
pub struct AuthTpmUserData {
    pub challenge_id: String,
    pub ek_pubkey_pem: String,
    pub ak_pubkey_pem: String,
    pub quote: String,
    pub signature: String,

    /// PCR index → hex SHA-256 digest. Optional; required only if the
    /// credential policy pins PCR values.
    #[serde(default)]
    pub pcrs: Option<HashMap<u8, String>>,
}

#[derive(Deserialize, Debug)]
pub struct AuthTpmActivationData {
    pub principal_id: String,
    pub credential_id: String,
    pub recovered_secret: String,
}

#[derive(Deserialize, Debug)]
pub struct AuthTpmChallengeData {
    pub ek_pubkey_hash: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct AuthAwsUserData {
    /// The HTTP method the client used to sign (always "POST" for `GetCallerIdentity`).
    pub method: String,
    /// The URL the request was signed for. MUST be sts.amazonaws.com (or a
    /// pinned regional STS endpoint). Rejected otherwise.
    pub url: String,
    /// The signed request body, typically:
    /// "Action=GetCallerIdentity&Version=2011-06-15"
    pub body: String,
    /// All headers from the signed request, including Authorization,
    /// X-Amz-Date, Host, X-Amz-Security-Token (if temp creds), etc.
    pub headers: HashMap<String, String>,
}

#[derive(Deserialize, Debug)]
pub struct AppGrantData {
    pub apps: Vec<String>,
    pub principal_id: String,
}

#[derive(Deserialize, Debug)]
pub struct RevokeAppData {
    pub app_id: String,
    pub principal_id: String,
}

#[derive(Deserialize, Debug)]
pub struct ListAppsData {
    pub principal_id: String,
    pub after_app_id: Option<String>,
}

use std::collections::HashMap;

use decodering_core::domain::{PrincipalCredentialKind, PrincipalKind};
use serde::Deserialize;
use serde_with::base64::Base64;
use serde_with::serde_as;

#[derive(Deserialize, Debug)]
pub struct CreateAppUserData {
    pub name: String,
    pub kind: PrincipalKind,
    pub credential_kind: PrincipalCredentialKind,
    pub tpm: Option<TrustedPlatformModuleData>,
    pub expires_at: Option<i64>,
    pub apps: Option<Vec<String>>,
}

#[derive(Deserialize, Debug)]
pub struct TrustedPlatformModuleData {
    pub ek_pubkey_pem: String,       // the public key from the TPM
    pub ek_cert_pem: Option<String>, // optional EK certificate
    pub expected_pcrs: Option<HashMap<u8, String>>, // optional boot-state pinning
    pub require_ek_cert: bool,       // policy
}

#[serde_as]
#[derive(Deserialize, Debug)]
pub struct UnlockData {
    #[serde_as(as = "Vec<Base64>")]
    pub shards: Vec<Vec<u8>>,
}

#[derive(Deserialize, Debug)]
pub struct CreateAppData {
    pub app_name: String,
}

#[derive(Deserialize, Debug)]
pub struct AuthUserData {
    pub app_id: String,
    pub key: String,
}

#[derive(Deserialize, Debug)]
pub struct AuthTpmData {
    pub ek_pubkey_hash: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct AppGrantData {
    pub apps: Vec<String>,
    pub principal_id: String,
}

use decodering_core::domain::{PrincipalCredentialKind, PrincipalKind};
use serde::Deserialize;
use serde_with::{base64::Base64, serde_as};

#[derive(Deserialize, Debug)]
pub struct CreateAppUserData {
    pub app_id: String,
    pub name: String,
    pub kind: PrincipalKind,
    pub credential_kind: PrincipalCredentialKind,
    pub expires_at: Option<i64>,
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

use std::collections::HashMap;

use serde::Deserialize;

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

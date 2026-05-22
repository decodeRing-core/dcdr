use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct InitSystemRequestData {
    pub total_shares: Option<u8>,
    pub threshold: Option<u8>,
    pub plugins_credentials: BTreeMap<String, BTreeMap<String, String>>,
}

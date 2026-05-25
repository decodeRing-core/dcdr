use std::collections::BTreeMap;

use serde::Deserialize;
use serde_with::base64::Base64;
use serde_with::serde_as;

use zeroize::Zeroizing;

#[derive(Deserialize)]
pub struct InitSystemData {
    pub total_shares: Option<u8>,
    pub threshold: Option<u8>,
    pub plugins_credentials: BTreeMap<String, BTreeMap<String, Zeroizing<String>>>,
}

#[serde_as]
#[derive(Deserialize, Debug)]
pub struct UnlockData {
    #[serde_as(as = "Vec<Base64>")]
    pub shards: Vec<Vec<u8>>,
}

#[derive(Deserialize, Debug)]
pub struct PluginConfigData {
    pub plugins_credentials: BTreeMap<String, BTreeMap<String, Zeroizing<String>>>,
}

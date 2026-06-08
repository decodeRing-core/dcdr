use std::collections::BTreeMap;

use serde::Deserialize;
use serde_with::base64::Base64;
use serde_with::serde_as;

use utoipa::ToSchema;
use zeroize::Zeroizing;

#[derive(Deserialize, ToSchema)]
pub struct InitSystemData {
    pub total_shares: Option<u8>,
    pub threshold: Option<u8>,
    #[schema(value_type = std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>>)]
    pub plugins_credentials: BTreeMap<String, BTreeMap<String, Zeroizing<String>>>,
}

#[serde_as]
#[derive(Deserialize, Debug, ToSchema)]
pub struct UnlockData {
    #[serde_as(as = "Vec<Base64>")]
    pub shards: Vec<Vec<u8>>,
}

#[derive(Deserialize, Debug, ToSchema)]
pub struct PluginConfigData {
    #[schema(value_type = std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>>)]
    pub plugins_credentials: BTreeMap<String, BTreeMap<String, Zeroizing<String>>>,
}

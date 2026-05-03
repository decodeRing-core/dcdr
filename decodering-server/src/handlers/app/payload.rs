use serde::Deserialize;
use serde_with::{base64::Base64, serde_as};

#[derive(Deserialize, Debug)]
pub(crate) struct CreateAppUserData {
    pub app_id: String,
    pub username: String,
    pub email: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct GetAppUserData {
    pub app_id: String,
    pub username: String,
}

#[serde_as]
#[derive(Deserialize, Debug)]
pub(crate) struct UnlockData {
    #[serde_as(as = "Vec<Base64>")]
    pub shards: Vec<Vec<u8>>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct CreateAppData {
    pub app_name: String,
}

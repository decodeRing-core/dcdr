use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize, Debug)]
pub(crate) struct PutSecretRequestData {
    pub app_id: String,
    pub secret_name: String,
    pub store: Store,
    pub data: Value,
    pub options: Options,
}

#[derive(Deserialize, Debug)]
pub(crate) struct Store {
    pub backend_ref: String,
    pub store_path: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct Options {
    pub create_only: bool,
}

#[derive(Deserialize, Debug)]
pub(crate) struct GetSecretRequestData {
    pub app_id: String,
    pub secret_name: String,
    pub version: u64,
}

use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize, Debug)]
pub struct PutSecretRequestData {
    pub app_id: String,
    pub secret_name: String,
    pub store: Store,
    pub data: Value,
    pub options: Options,
}

#[derive(Deserialize, Debug)]
pub struct Store {
    pub backend_ref: String,
    pub store_path: String,
}

#[derive(Deserialize, Debug)]
pub struct Options {
    pub create_only: bool,
}

#[derive(Deserialize, Debug)]
pub struct GetSecretRequestData {
    pub app_id: String,
    pub secret_name: String,
    pub version: u64,
}

#[derive(Deserialize, Debug)]
pub struct DeleteSecretRequestData {
    pub app_id: String,
    pub secret_name: String,
}

#[derive(Deserialize, Debug)]
pub struct DestroySecretRequestData {
    pub app_id: String,
    pub secret_name: String,
}

#[derive(Deserialize, Debug)]
pub struct RestoreSecretRequestData {
    pub app_id: String,
    pub secret_name: String,
}

#[derive(Deserialize, Debug)]
pub struct ListSecretRequestData {
    pub app_id: String,
    pub after_secret: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct TaintSecretRequestData {
    pub app_id: String,
    pub secret_name: String,
}

#[derive(Deserialize, Debug)]
pub struct UntaintSecretRequestData {
    pub app_id: String,
    pub secret_name: String,
}

#[derive(Deserialize, Debug)]
pub struct IsTaintedSecretRequestData {
    pub app_id: String,
    pub secret_name: String,
}

#[derive(Deserialize, Debug)]
pub struct DescribeSecretRequestData {
    pub app_id: String,
    pub secret_name: String,
}

#[derive(Deserialize, Debug)]
pub struct ListAppsData {
    pub after_app_id: Option<String>,
}

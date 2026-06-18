use std::error::Error;

use serde::{Deserialize, Serialize};

use crate::api::{Resp, get_auth, get_auth_body, post_auth};

#[derive(Serialize, Deserialize)]
pub struct PutRequest {
    pub app_id: String,
    pub secret_name: String,
    pub store: SecretStore,
    pub data: serde_json::Map<String, serde_json::Value>,
    pub options: PutOptions,
}

#[derive(Serialize, Deserialize)]
pub struct SecretStore {
    pub backend_ref: String,
    pub store_path: String,
}

#[derive(Serialize, Deserialize)]
pub struct PutOptions {
    pub create_only: bool,
}

#[derive(Serialize, Deserialize)]
pub struct GetSecretRequest {
    pub app_id: String,
    pub secret_name: String,
    pub version: String,
}

#[derive(Serialize, Deserialize)]
pub struct SecretRef {
    pub app_id: String,
    pub secret_name: String,
}

#[derive(Serialize, Deserialize)]
pub struct AppRef {
    pub app_id: String,
}

pub async fn osl_secrets_put(
    addr: &str,
    token: &str,
    req: PutRequest,
) -> Result<Resp, Box<dyn Error>> {
    post_auth(addr, "/osl/v1/secrets/put", token, &req).await
}
pub async fn osl_secrets_get(
    addr: &str,
    token: &str,
    req: GetSecretRequest,
) -> Result<Resp, Box<dyn Error>> {
    post_auth(addr, "/osl/v1/secrets/get", token, &req).await
}
pub async fn osl_secrets_list(
    addr: &str,
    token: &str,
    req: AppRef,
) -> Result<Resp, Box<dyn Error>> {
    post_auth(addr, "/osl/v1/secrets/list", token, &req).await
}
pub async fn osl_secrets_taint(
    addr: &str,
    token: &str,
    req: SecretRef,
) -> Result<Resp, Box<dyn Error>> {
    post_auth(addr, "/osl/v1/secrets/taint", token, &req).await
}
pub async fn osl_secrets_untaint(
    addr: &str,
    token: &str,
    req: SecretRef,
) -> Result<Resp, Box<dyn Error>> {
    post_auth(addr, "/osl/v1/secrets/untaint", token, &req).await
}
pub async fn osl_secrets_is_tainted(
    addr: &str,
    token: &str,
    req: SecretRef,
) -> Result<Resp, Box<dyn Error>> {
    post_auth(addr, "/osl/v1/secrets/is-tainted", token, &req).await
}
pub async fn osl_secrets_describe(
    addr: &str,
    token: &str,
    req: SecretRef,
) -> Result<Resp, Box<dyn Error>> {
    post_auth(addr, "/osl/v1/secrets/describe", token, &req).await
}
pub async fn osl_secrets_restore(
    addr: &str,
    token: &str,
    req: SecretRef,
) -> Result<Resp, Box<dyn Error>> {
    post_auth(addr, "/osl/v1/secrets/restore", token, &req).await
}
pub async fn osl_secrets_destroy(
    addr: &str,
    token: &str,
    req: SecretRef,
) -> Result<Resp, Box<dyn Error>> {
    post_auth(addr, "/osl/v1/secrets/destroy", token, &req).await
}
pub async fn osl_secrets_delete(
    addr: &str,
    token: &str,
    req: SecretRef,
) -> Result<Resp, Box<dyn Error>> {
    post_auth(addr, "/osl/v1/secrets/delete", token, &req).await
}
pub async fn osl_capabilities_get(addr: &str, token: &str) -> Result<Resp, Box<dyn Error>> {
    get_auth(addr, "/osl/v1/capabilities/get", token).await
}
pub async fn osl_apps_list(addr: &str, token: &str) -> Result<Resp, Box<dyn Error>> {
    get_auth_body(addr, "/osl/v1/apps/list", token, &serde_json::Map::new()).await
}
pub async fn osl_backends_list(addr: &str, token: &str) -> Result<Resp, Box<dyn Error>> {
    get_auth_body(
        addr,
        "/osl/v1/backends/list",
        token,
        &serde_json::Map::new(),
    )
    .await
}

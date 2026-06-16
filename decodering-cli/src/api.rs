use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::error::Error;
use std::fmt;

pub type PluginsCredentials = HashMap<String, serde_json::Value>;

#[derive(Serialize, Deserialize)]
pub struct Node {
    pub addr: String,
}

#[derive(Serialize, Deserialize)]
pub enum ChangeMembers {
    AddVoterIds(BTreeSet<u64>),
    AddVoters(BTreeMap<u64, Node>),
    RemoveVoters(BTreeSet<u64>),
    ReplaceAllVoters(BTreeSet<u64>),
    AddNodes(BTreeMap<u64, Node>),
    SetNodes(BTreeMap<u64, Node>),
    RemoveNodes(BTreeSet<u64>),
    ReplaceAllNodes(BTreeMap<u64, Node>),
    Batch(Vec<Self>),
}

#[derive(Serialize)]
pub struct InitRequest {
    pub total_shares: u8,
    pub threshold: u8,
    pub plugins_credentials: PluginsCredentials,
}

#[derive(Serialize, Deserialize)]
pub struct UnlockRequest {
    pub shards: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct RaftInitRequest {
    pub raft_init: Vec<(u64, String)>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateAppRequest {
    pub app_name: String,
}

#[derive(Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub kind: String,
    pub credential_kind: String,
}

#[derive(Serialize, Deserialize)]
pub struct GrantRequest {
    pub principal_id: String,
    pub apps: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct RevokeRequest {
    pub principal_id: String,
    pub app_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct ListUsersRequest {
    pub principal_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct AuthRequest {
    pub credential_kind: String,
    pub proof: serde_json::Value,
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

#[derive(Serialize, Deserialize)]
pub struct GetSecretRequest {
    pub app_id: String,
    pub secret_name: String,
    pub version: String,
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
pub struct PutRequest {
    pub app_id: String,
    pub secret_name: String,
    pub store: SecretStore,
    pub data: serde_json::Map<String, serde_json::Value>,
    pub options: PutOptions,
}

#[derive(Deserialize)]
pub struct ApiResponse<T> {
    #[serde(default)]
    pub _status: String,
    #[serde(default)]
    pub message: String,
    pub data: Option<T>,
}

#[derive(Deserialize)]
struct ErrorEnvelope {
    error: ApiError,
}

#[derive(Deserialize, Debug)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub detail: Option<String>,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let primary = self.detail.as_deref().unwrap_or(&self.message);
        write!(f, "{primary} [{}]", self.code)
    }
}

impl Error for ApiError {}

type Resp = ApiResponse<serde_json::Value>;

#[derive(Deserialize)]
pub struct InitResponse {
    pub shards: Vec<String>,
    pub root_token: String,
}

pub async fn init(addr: &str, req: InitRequest) -> Result<InitResponse, Box<dyn Error>> {
    let url = format!("{}/system/init", addr.trim_end_matches('/'));
    let res = reqwest::Client::new().post(url).json(&req).send().await?;
    let resp: ApiResponse<InitResponse> = handle(res).await?;
    resp.data
        .ok_or_else(|| Box::<dyn Error>::from("response missing data"))
}

pub async fn unlock(
    addr: &str,
    req: UnlockRequest,
) -> Result<ApiResponse<serde_json::Value>, Box<dyn Error>> {
    let url = format!("{}/system/unlock", addr.trim_end_matches('/'));
    let res = reqwest::Client::new().post(url).json(&req).send().await?;
    handle(res).await
}

pub async fn status(addr: &str) -> Result<ApiResponse<serde_json::Value>, Box<dyn Error>> {
    let url = format!("{}/system/status", addr.trim_end_matches('/'));
    let res = reqwest::Client::new().get(url).send().await?;
    handle(res).await
}

async fn handle<T: DeserializeOwned>(
    res: reqwest::Response,
) -> Result<ApiResponse<T>, Box<dyn Error>> {
    let status = res.status();
    let body = res.text().await?;

    if status.is_success() {
        return serde_json::from_str(&body)
            .map_err(|e| format!("unexpected response ({status}): {e}: {body}").into());
    }

    match serde_json::from_str::<ErrorEnvelope>(&body) {
        Ok(env) => Err(Box::new(env.error)),
        Err(_) => Err(format!("server returned {status}: {body}").into()),
    }
}

pub async fn raft_init(
    addr: &str,
    req: RaftInitRequest,
) -> Result<ApiResponse<serde_json::Value>, Box<dyn Error>> {
    let url = format!("{}/raft/init", addr.trim_end_matches('/'));
    let res = reqwest::Client::new().post(url).json(&req).send().await?;
    handle(res).await
}

pub async fn raft_shutdown(addr: &str) -> Result<ApiResponse<serde_json::Value>, Box<dyn Error>> {
    let url = format!("{}/raft/shutdown", addr.trim_end_matches('/'));
    let res = reqwest::Client::new().post(url).send().await?;
    handle(res).await
}

pub async fn raft_add_learner(
    addr: &str,
    node: (u64, String),
) -> Result<ApiResponse<serde_json::Value>, Box<dyn Error>> {
    let url = format!("{}/raft/add-learner", addr.trim_end_matches('/'));
    let res = reqwest::Client::new().post(url).json(&node).send().await?;
    handle(res).await
}

async fn post_auth<B: Serialize, T: DeserializeOwned>(
    addr: &str,
    path: &str,
    token: &str,
    body: &B,
) -> Result<ApiResponse<T>, Box<dyn Error>> {
    let url = format!("{}{}", addr.trim_end_matches('/'), path);
    let res = reqwest::Client::new()
        .post(url)
        .bearer_auth(token)
        .json(body)
        .send()
        .await?;
    handle(res).await
}

pub async fn app_create(
    addr: &str,
    token: &str,
    req: CreateAppRequest,
) -> Result<ApiResponse<serde_json::Value>, Box<dyn Error>> {
    post_auth(addr, "/app/create", token, &req).await
}

pub async fn app_user_create(
    addr: &str,
    token: &str,
    req: CreateUserRequest,
) -> Result<ApiResponse<serde_json::Value>, Box<dyn Error>> {
    post_auth(addr, "/app/user/create", token, &req).await
}

pub async fn app_user_grant(
    addr: &str,
    token: &str,
    req: GrantRequest,
) -> Result<ApiResponse<serde_json::Value>, Box<dyn Error>> {
    post_auth(addr, "/app/user/grant", token, &req).await
}

pub async fn app_user_revoke(
    addr: &str,
    token: &str,
    req: RevokeRequest,
) -> Result<ApiResponse<serde_json::Value>, Box<dyn Error>> {
    post_auth(addr, "/app/user/revoke", token, &req).await
}

pub async fn app_user_list(
    addr: &str,
    token: &str,
    req: ListUsersRequest,
) -> Result<ApiResponse<serde_json::Value>, Box<dyn Error>> {
    post_auth(addr, "/app/user/list", token, &req).await
}

pub async fn app_user_auth(
    addr: &str,
    req: AuthRequest,
) -> Result<ApiResponse<serde_json::Value>, Box<dyn Error>> {
    let url = format!("{}/app/user/auth", addr.trim_end_matches('/'));
    let res = reqwest::Client::new().post(url).json(&req).send().await?;
    handle(res).await
}

async fn get_auth<T: DeserializeOwned>(
    addr: &str,
    path: &str,
    token: &str,
) -> Result<ApiResponse<T>, Box<dyn Error>> {
    let url = format!("{}{path}", addr.trim_end_matches('/'));
    let res = reqwest::Client::new()
        .get(url)
        .bearer_auth(token)
        .send()
        .await?;
    handle(res).await
}

async fn get_auth_body<B: Serialize, T: DeserializeOwned>(
    addr: &str,
    path: &str,
    token: &str,
    body: &B,
) -> Result<ApiResponse<T>, Box<dyn Error>> {
    let url = format!("{}{path}", addr.trim_end_matches('/'));
    let res = reqwest::Client::new()
        .get(url)
        .bearer_auth(token)
        .json(body)
        .send()
        .await?;
    handle(res).await
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

pub async fn raft_metrics(addr: &str) -> Result<ApiResponse<serde_json::Value>, Box<dyn Error>> {
    let url = format!("{}/raft/metrics", addr.trim_end_matches('/'));
    let res = reqwest::Client::new().post(url).send().await?;
    handle(res).await
}

pub async fn raft_change_membership(
    addr: &str,
    change: ChangeMembers,
) -> Result<ApiResponse<serde_json::Value>, Box<dyn Error>> {
    let url = format!("{}/raft/change-membership", addr.trim_end_matches('/'));
    let res = reqwest::Client::new()
        .post(url)
        .json(&change)
        .send()
        .await?;
    handle(res).await
}

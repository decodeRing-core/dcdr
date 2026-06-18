use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;

pub mod app;
pub mod osl;
pub mod raft;
pub mod system;

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

use std::error::Error;

use serde::{Deserialize, Serialize};

use crate::api::ApiResponse;
use crate::api::{handle, post_auth};

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

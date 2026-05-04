use extism_pdk::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Deserialize)]
struct ReadSecretInput {
    secret_name: String,
    version: Option<String>,
}

#[derive(Deserialize)]
pub struct WriteSecretInput {
    pub path: String,
    pub data: serde_json::Value,
}

#[derive(Serialize)]
pub struct WriteSecretOutput {
    pub version: String,
}

#[derive(Serialize)]
pub struct ReadSecretResponse {
    pub version: String,
    pub data: Value,
}

#[plugin_fn]
pub fn get_secret(Json(input): Json<ReadSecretInput>) -> FnResult<Json<ReadSecretResponse>> {
    let addr = config::get("vault_addr")?
        .ok_or_else(|| WithReturnCode::from(Error::msg("missing vault_addr config")))?;
    let token = config::get("vault_token")?
        .ok_or_else(|| WithReturnCode::from(Error::msg("missing vault_token config")))?;
    let mount = config::get("kv_mount")?.unwrap_or_else(|| "secret".to_string());

    let url = match input.version {
        Some(v) => format!(
            "{}/v1/{}/data/{}?version={}",
            addr.trim_end_matches('/'),
            mount,
            input.secret_name,
            v
        ),
        None => format!(
            "{}/v1/{}/data/{}",
            addr.trim_end_matches('/'),
            mount,
            input.secret_name
        ),
    };

    let req = HttpRequest::new(&url)
        .with_method("GET")
        .with_header("X-Vault-Token", token);

    let res = http::request::<()>(&req, None)?;

    if res.status_code() >= 300 {
        return Err(WithReturnCode::from(Error::msg(format!(
            "openbao returned {}: {}",
            res.status_code(),
            String::from_utf8_lossy(&res.body())
        ))));
    }

    let parsed: serde_json::Value = serde_json::from_slice(&res.body())?;
    let version = parsed
        .pointer("/data/metadata/version")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let data = parsed
        .pointer("/data/data")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    Ok(Json(ReadSecretResponse {
        version: version.to_string(),
        data,
    }))
}

#[plugin_fn]
pub fn put_secret(Json(input): Json<WriteSecretInput>) -> FnResult<Json<WriteSecretOutput>> {
    let addr = config::get("vault_addr")?
        .ok_or_else(|| WithReturnCode::from(Error::msg("missing vault_addr config")))?;
    let token = config::get("vault_token")?
        .ok_or_else(|| WithReturnCode::from(Error::msg("missing vault_token config")))?;
    let mount = config::get("kv_mount")?.unwrap_or_else(|| "secret".to_string());

    let body = json!({ "data": input.data });

    let url = format!(
        "{}/v1/{}/data/{}",
        addr.trim_end_matches('/'),
        mount,
        input.path
    );

    let req = HttpRequest::new(&url)
        .with_method("POST")
        .with_header("X-Vault-Token", token)
        .with_header("Content-Type", "application/json");

    let res = http::request(&req, Some(body.to_string().as_bytes()))?;

    if res.status_code() >= 300 {
        return Err(WithReturnCode::from(Error::msg(format!(
            "openbao returned {}: {}",
            res.status_code(),
            String::from_utf8_lossy(&res.body())
        ))));
    }

    let parsed: serde_json::Value = serde_json::from_slice(&res.body())?;
    let version = parsed
        .pointer("/data/version")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    Ok(Json(WriteSecretOutput {
        version: version.to_string(),
    }))
}

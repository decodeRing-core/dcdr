#![allow(clippy::unnecessary_wraps)]
use extism_pdk::{Error, FnResult, HttpRequest, Json, WithReturnCode, config, http, plugin_fn};
use serde_json::json;

use crate::contract::Capability;
use crate::contract::DeleteInput;
use crate::contract::DeleteOutput;
use crate::contract::DestroyInput;
use crate::contract::DestroyOutput;
use crate::contract::ReadInput;
use crate::contract::ReadResponse;
use crate::contract::RestoreInput;
use crate::contract::RestoreOutput;
use crate::contract::WriteInput;
use crate::contract::WriteOutput;

mod contract;

#[derive(serde::Deserialize)]
struct MetadataData {
    current_version: u64,
}

#[derive(serde::Serialize)]
struct UndeleteBody {
    versions: Vec<u64>,
}

#[derive(serde::Deserialize)]
struct MetadataResponse {
    data: MetadataData,
}

#[plugin_fn]
pub fn capabilities(_: ()) -> FnResult<Json<Vec<Capability>>> {
    Ok(Json(vec![
        Capability::KvRead,
        Capability::KvDelete,
        Capability::KvWrite,
        Capability::KvVersioning,
    ]))
}

#[plugin_fn]
pub fn restore_secret(Json(input): Json<RestoreInput>) -> FnResult<Json<RestoreOutput>> {
    let addr = config::get("vault_addr")?
        .ok_or_else(|| WithReturnCode::from(Error::msg("missing vault_addr config")))?;
    let token = config::get("vault_token")?
        .ok_or_else(|| WithReturnCode::from(Error::msg("missing vault_token config")))?;
    let mount = config::get("kv_mount")?.unwrap_or_else(|| "secret".to_owned());

    let meta_url = format!(
        "{}/v1/{}/metadata/{}",
        addr.trim_end_matches('/'),
        mount,
        input.path
    );
    let meta_req = HttpRequest::new(&meta_url)
        .with_method("GET")
        .with_header("X-Vault-Token", &token);
    let meta_res = http::request::<()>(&meta_req, None)?;
    if meta_res.status_code() >= 300 {
        return Err(WithReturnCode::from(Error::msg(format!(
            "openbao returned {}: {}",
            meta_res.status_code(),
            String::from_utf8_lossy(&meta_res.body())
        ))));
    }
    let meta: MetadataResponse = serde_json::from_slice(&meta_res.body())
        .map_err(|e| WithReturnCode::from(Error::msg(format!("failed to parse metadata: {e}"))))?;
    let latest = meta.data.current_version;

    let url = format!(
        "{}/v1/{}/undelete/{}",
        addr.trim_end_matches('/'),
        mount,
        input.path
    );
    let body = serde_json::to_vec(&UndeleteBody {
        versions: vec![latest],
    })
    .map_err(|e| WithReturnCode::from(Error::msg(format!("failed to serialize body: {e}"))))?;
    let req = HttpRequest::new(&url)
        .with_method("POST")
        .with_header("X-Vault-Token", &token)
        .with_header("Content-Type", "application/json");
    let res = http::request(&req, Some(&body))?;
    if res.status_code() >= 300 {
        return Err(WithReturnCode::from(Error::msg(format!(
            "openbao returned {}: {}",
            res.status_code(),
            String::from_utf8_lossy(&res.body())
        ))));
    }

    Ok(Json(RestoreOutput { restored: true }))
}

#[plugin_fn]
pub fn delete_secret(Json(input): Json<DeleteInput>) -> FnResult<Json<DeleteOutput>> {
    let addr = config::get("vault_addr")?
        .ok_or_else(|| WithReturnCode::from(Error::msg("missing vault_addr config")))?;
    let token = config::get("vault_token")?
        .ok_or_else(|| WithReturnCode::from(Error::msg("missing vault_token config")))?;
    let mount = config::get("kv_mount")?.unwrap_or_else(|| "secret".to_owned());
    let url = format!(
        "{}/v1/{}/data/{}",
        addr.trim_end_matches('/'),
        mount,
        input.path
    );
    let req = HttpRequest::new(&url)
        .with_method("DELETE")
        .with_header("X-Vault-Token", token);
    let res = http::request::<()>(&req, None)?;
    if res.status_code() >= 300 {
        return Err(WithReturnCode::from(Error::msg(format!(
            "openbao returned {}: {}",
            res.status_code(),
            String::from_utf8_lossy(&res.body())
        ))));
    }
    Ok(Json(DeleteOutput { deleted: true }))
}

#[plugin_fn]
pub fn destroy_secret(Json(input): Json<DestroyInput>) -> FnResult<Json<DestroyOutput>> {
    let addr = config::get("vault_addr")?
        .ok_or_else(|| WithReturnCode::from(Error::msg("missing vault_addr config")))?;
    let token = config::get("vault_token")?
        .ok_or_else(|| WithReturnCode::from(Error::msg("missing vault_token config")))?;
    let mount = config::get("kv_mount")?.unwrap_or_else(|| "secret".to_owned());

    let url = format!(
        "{}/v1/{}/metadata/{}",
        addr.trim_end_matches('/'),
        mount,
        input.path
    );

    let req = HttpRequest::new(&url)
        .with_method("DELETE")
        .with_header("X-Vault-Token", token);

    let res = http::request::<()>(&req, None)?;

    if res.status_code() >= 300 {
        return Err(WithReturnCode::from(Error::msg(format!(
            "openbao returned {}: {}",
            res.status_code(),
            String::from_utf8_lossy(&res.body())
        ))));
    }

    Ok(Json(DestroyOutput { destroyed: true }))
}

#[plugin_fn]
pub fn get_secret(Json(input): Json<ReadInput>) -> FnResult<Json<ReadResponse>> {
    let addr = config::get("vault_addr")?
        .ok_or_else(|| WithReturnCode::from(Error::msg("missing vault_addr config")))?;
    let token = config::get("vault_token")?
        .ok_or_else(|| WithReturnCode::from(Error::msg("missing vault_token config")))?;
    let mount = config::get("kv_mount")?.unwrap_or_else(|| "secret".to_owned());

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
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);

    let data = parsed
        .pointer("/data/data")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    Ok(Json(ReadResponse {
        version: version.to_string(),
        data: Some(data),
    }))
}

#[plugin_fn]
pub fn put_secret(Json(input): Json<WriteInput>) -> FnResult<Json<WriteOutput>> {
    let addr = config::get("vault_addr")?
        .ok_or_else(|| WithReturnCode::from(Error::msg("missing vault_addr config")))?;
    let token = config::get("vault_token")?
        .ok_or_else(|| WithReturnCode::from(Error::msg("missing vault_token config")))?;
    let mount = config::get("kv_mount")?.unwrap_or_else(|| "secret".to_owned());

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
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);

    Ok(Json(WriteOutput {
        version: version.to_string(),
    }))
}

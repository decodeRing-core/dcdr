#![allow(clippy::unnecessary_wraps)]
use extism_pdk::{Error, FnResult, HttpRequest, Json, WithReturnCode, config, http, plugin_fn};
use serde_json::Value;
use serde_json::json;

use crate::contract::Capability;
use crate::contract::DeleteInput;
use crate::contract::DeleteOutput;
use crate::contract::DescribeInput;
use crate::contract::DescribeOutput;
use crate::contract::DestroyInput;
use crate::contract::DestroyOutput;
use crate::contract::ReadInput;
use crate::contract::ReadOutput;
use crate::contract::RestoreInput;
use crate::contract::RestoreOutput;
use crate::contract::SecretStatus;
use crate::contract::VersionInfo;
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
        Capability::KvSoftDelete,
        Capability::KvDestroy,
        Capability::KvWrite,
        Capability::KvVersioning,
        Capability::KvRestore,
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
pub fn get_secret(Json(input): Json<ReadInput>) -> FnResult<Json<ReadOutput>> {
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
    let status = res.status_code();
    let body = res.body();
    if status == 404 {
        // Body may be empty (true not-found) or carry version metadata.
        if let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(&body) {
            let meta = parsed.pointer("/data/metadata");
            let version = meta
                .and_then(|m| m.get("version"))
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let destroyed = meta
                .and_then(|m| m.get("destroyed"))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            let deletion_time = meta
                .and_then(|m| m.get("deletion_time"))
                .and_then(serde_json::Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_owned);

            if destroyed {
                return Ok(Json(ReadOutput {
                    status: SecretStatus::Destroyed,
                    version: version.to_string(),
                    data: None,
                    deletion_time: None,
                }));
            }
            if deletion_time.is_some() {
                return Ok(Json(ReadOutput {
                    status: SecretStatus::SoftDeleted,
                    version: version.to_string(),
                    data: None,
                    deletion_time,
                }));
            }
        }
        // No usable metadata: the path/version genuinely doesn't exist.
        return Ok(Json(ReadOutput {
            status: SecretStatus::NotFound,
            version: "0".to_owned(),
            data: None,
            deletion_time: None,
        }));
    }

    if status >= 300 {
        return Err(WithReturnCode::from(Error::msg(format!(
            "openbao returned {}: {}",
            status,
            String::from_utf8_lossy(&body)
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

    Ok(Json(ReadOutput {
        status: SecretStatus::Present,
        version: version.to_string(),
        data: Some(data),
        deletion_time: None,
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

#[plugin_fn]
pub fn describe(Json(input): Json<DescribeInput>) -> FnResult<Json<DescribeOutput>> {
    let addr = config::get("vault_addr")?
        .ok_or_else(|| WithReturnCode::from(Error::msg("missing vault_addr config")))?;
    let token = config::get("vault_token")?
        .ok_or_else(|| WithReturnCode::from(Error::msg("missing vault_token config")))?;
    let mount = config::get("kv_mount")?.unwrap_or_else(|| "secret".to_owned());

    let base = addr.trim_end_matches('/');
    let url = format!("{base}/v1/{}/metadata/{}", mount, input.path);

    let req = HttpRequest::new(&url)
        .with_method("GET")
        .with_header("X-Vault-Token", token);
    let res = http::request::<()>(&req, None)?;
    let status = res.status_code();
    let body = res.body();

    if status == 404 {
        return Err(WithReturnCode::from(Error::msg(format!(
            "openbao returned {}: {}",
            res.status_code(),
            String::from_utf8_lossy(&res.body())
        ))));
    }

    if status >= 300 {
        return Err(WithReturnCode::from(Error::msg(format!(
            "openbao returned {status}: {}",
            String::from_utf8_lossy(&body)
        ))));
    }

    let parsed: Value = serde_json::from_slice(&body)?;
    let meta = parsed.pointer("/data").unwrap_or(&Value::Null);

    let current_version = meta
        .pointer("/current_version")
        .and_then(Value::as_u64)
        .map(|v| v.to_string());

    // Build the version array from the metadata "versions" map.
    let mut versions: Vec<VersionInfo> = Vec::new();
    if let Some(map) = meta.pointer("/versions").and_then(Value::as_object) {
        for (vnum, vmeta) in map {
            let destroyed = vmeta
                .get("destroyed")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let deletion_time = vmeta
                .get("deletion_time")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_owned);

            let vstatus = if destroyed {
                SecretStatus::Destroyed
            } else if deletion_time.is_some() {
                SecretStatus::SoftDeleted
            } else {
                SecretStatus::Present
            };

            versions.push(VersionInfo {
                id: vnum.clone(),
                status: vstatus,
                created_time: vmeta
                    .get("created_time")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                deletion_time,
            });
        }
        // Object iteration order isn't guaranteed; sort numerically.
        versions.sort_by_key(|v| v.id.parse::<u64>().unwrap_or(0));
    }

    // Current status = status of the current version, defaulting to Present.
    let current_status = current_version
        .as_deref()
        .and_then(|cv| meta.pointer(&format!("/versions/{cv}")))
        .map_or(SecretStatus::Present, status_from_meta);

    let provider_hints = serde_json::json!({
        "engine": "kv",
        "engine_version": "2",
        "mount": mount,
        "path": format!("{}/data/{}", mount, input.path),
        "metadata_path": format!("{}/metadata/{}", mount, input.path),
        "max_versions": meta.pointer("/max_versions").and_then(Value::as_u64),
        "cas_required": meta.pointer("/cas_required").and_then(Value::as_bool),
        "delete_version_after": meta
            .pointer("/delete_version_after")
            .and_then(Value::as_str),
    });

    Ok(Json(DescribeOutput {
        secret_name: input.path,
        provider: "openbao".to_owned(),
        exists: true,
        current_version,
        current_status,
        created_time: meta
            .pointer("/created_time")
            .and_then(Value::as_str)
            .map(str::to_owned),
        updated_time: meta
            .pointer("/updated_time")
            .and_then(Value::as_str)
            .map(str::to_owned),
        versions,
        custom_metadata: meta.pointer("/custom_metadata").cloned(),
        provider_hints: Some(provider_hints),
    }))
}

fn status_from_meta(vmeta: &Value) -> SecretStatus {
    let destroyed = vmeta
        .get("destroyed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let deleted = vmeta
        .get("deletion_time")
        .and_then(Value::as_str)
        .is_some_and(|s| !s.is_empty());
    if destroyed {
        SecretStatus::Destroyed
    } else if deleted {
        SecretStatus::SoftDeleted
    } else {
        SecretStatus::Present
    }
}

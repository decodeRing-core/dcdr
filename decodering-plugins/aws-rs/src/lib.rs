use std::time::SystemTime;

use extism_pdk::{Error, FnResult, HttpRequest, Json, WithReturnCode, config, http, plugin_fn};
use serde_json::{Value, json};

use aws_credential_types::Credentials;
use aws_sigv4::http_request::{SignableBody, SignableRequest, SigningSettings, sign};
use aws_sigv4::sign::v4;

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

struct AwsConfig {
    access_key: String,
    secret_key: String,
    session_token: Option<String>,
    region: String,
}

fn load_config() -> Result<AwsConfig, WithReturnCode<Error>> {
    let get = |k: &str| -> Result<String, WithReturnCode<Error>> {
        config::get(k)?
            .ok_or_else(|| WithReturnCode::from(Error::msg(format!("missing {k} config"))))
    };
    Ok(AwsConfig {
        access_key: get("aws_access_key_id")?,
        secret_key: get("aws_secret_access_key")?,
        session_token: config::get("aws_session_token")?,
        region: get("region")?,
    })
}

fn call(
    cfg: &AwsConfig,
    target: &str,
    body: &[u8],
) -> Result<(u16, Vec<u8>), WithReturnCode<Error>> {
    let host = format!("secretsmanager.{}.amazonaws.com", cfg.region);
    let url = format!("https://{host}/");

    let identity = Credentials::new(
        &cfg.access_key,
        &cfg.secret_key,
        cfg.session_token.clone(),
        None,
        "extism-config",
    )
    .into();

    let signing_params = v4::SigningParams::builder()
        .identity(&identity)
        .region(&cfg.region)
        .name("secretsmanager")
        .time(SystemTime::now())
        .settings(SigningSettings::default())
        .build()
        .map_err(|e| WithReturnCode::from(Error::msg(format!("signing params: {e}"))))?
        .into();

    let headers = [
        ("host", host.as_str()),
        ("content-type", "application/x-amz-json-1.1"),
        ("x-amz-target", target),
    ];

    let signable = SignableRequest::new(
        "POST",
        &url,
        headers.iter().copied(),
        SignableBody::Bytes(body),
    )
    .map_err(|e| WithReturnCode::from(Error::msg(format!("signable: {e}"))))?;

    let (instructions, _sig) = sign(signable, &signing_params)
        .map_err(|e| WithReturnCode::from(Error::msg(format!("sign: {e}"))))?
        .into_parts();

    let mut req = HttpRequest::new(&url).with_method("POST");
    for (k, v) in headers {
        req = req.with_header(k, v);
    }
    for (name, value) in instructions.headers() {
        req = req.with_header(name, value);
    }

    let res = http::request(&req, Some(body))?;
    Ok((res.status_code(), res.body()))
}

fn is_exception(status: u16, body: &[u8], needle: &str) -> bool {
    status == 400
        && serde_json::from_slice::<Value>(body)
            .ok()
            .and_then(|v| {
                v.get("__type")
                    .and_then(Value::as_str)
                    .map(|t| t.contains(needle))
            })
            .unwrap_or(false)
}

fn err_response(status: u16, body: &[u8]) -> WithReturnCode<Error> {
    WithReturnCode::from(Error::msg(format!(
        "secretsmanager returned {status}: {}",
        String::from_utf8_lossy(body)
    )))
}

#[plugin_fn]
pub fn capabilities(_: ()) -> FnResult<Json<Vec<Capability>>> {
    Ok(Json(vec![
        Capability::Read,
        Capability::SoftDelete,
        Capability::Destroy,
        Capability::Write,
        Capability::Versioning,
        Capability::Restore,
    ]))
}

#[plugin_fn]
pub fn get_secret(Json(input): Json<ReadInput>) -> FnResult<Json<ReadOutput>> {
    let cfg = load_config()?;

    let mut b = json!({ "SecretId": input.secret_name });
    if let Some(v) = &input.version {
        b["VersionId"] = json!(v);
    }
    let (status, body) = call(
        &cfg,
        "secretsmanager.GetSecretValue",
        &serde_json::to_vec(&b)?,
    )?;

    if is_exception(status, &body, "ResourceNotFound") {
        return Ok(Json(ReadOutput {
            status: SecretStatus::NotFound,
            version: "".to_string(),
            data: None,
            deletion_time: None,
        }));
    }
    if is_exception(status, &body, "InvalidRequestException") {
        let (dstatus, dbody) = call(
            &cfg,
            "secretsmanager.DescribeSecret",
            &serde_json::to_vec(&json!({ "SecretId": input.secret_name }))?,
        )?;
        if dstatus < 300 {
            let parsed: Value = serde_json::from_slice(&dbody)?;
            let deletion_time = parsed
                .get("DeletedDate")
                .and_then(Value::as_str)
                .map(str::to_owned);
            if deletion_time.is_some() {
                return Ok(Json(ReadOutput {
                    status: SecretStatus::SoftDeleted,
                    version: "".to_string(),
                    data: None,
                    deletion_time,
                }));
            }
        }
    }
    if status >= 300 {
        return Err(err_response(status, &body));
    }

    let parsed: Value = serde_json::from_slice(&body)?;
    let version = parsed
        .get("VersionId")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();

    let data = parsed.get("SecretString").map(|s| match s.as_str() {
        Some(raw) => serde_json::from_str(raw).unwrap_or_else(|_| json!(raw)),
        None => s.clone(),
    });

    Ok(Json(ReadOutput {
        status: SecretStatus::Present,
        version,
        data,
        deletion_time: None,
    }))
}

#[plugin_fn]
pub fn put_secret(Json(input): Json<WriteInput>) -> FnResult<Json<WriteOutput>> {
    let cfg = load_config()?;

    let secret_string = match &input.data {
        Some(Value::String(s)) => s.clone(),
        Some(v) => v.to_string(),
        None => String::new(),
    };

    let put_body = json!({
        "SecretId": input.path,
        "SecretString": secret_string,
        "ClientRequestToken": input.idempotency_token,
    });
    let (status, body) = call(
        &cfg,
        "secretsmanager.PutSecretValue",
        &serde_json::to_vec(&put_body)?,
    )?;

    let (status, body) = if is_exception(status, &body, "ResourceNotFound") {
        let create_body = json!({
            "Name": input.path,
            "SecretString": secret_string,
            "ClientRequestToken": input.idempotency_token,
        });
        call(
            &cfg,
            "secretsmanager.CreateSecret",
            &serde_json::to_vec(&create_body)?,
        )?
    } else {
        (status, body)
    };

    if status >= 300 {
        return Err(err_response(status, &body));
    }

    let parsed: Value = serde_json::from_slice(&body)?;
    let version = parsed
        .get("VersionId")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();

    Ok(Json(WriteOutput { version }))
}

#[plugin_fn]
pub fn delete_secret(Json(input): Json<DeleteInput>) -> FnResult<Json<DeleteOutput>> {
    let cfg = load_config()?;
    let body = json!({ "SecretId": input.path });
    let (status, body) = call(
        &cfg,
        "secretsmanager.DeleteSecret",
        &serde_json::to_vec(&body)?,
    )?;
    if status >= 300 {
        return Err(err_response(status, &body));
    }
    Ok(Json(DeleteOutput { deleted: true }))
}

#[plugin_fn]
pub fn destroy_secret(Json(input): Json<DestroyInput>) -> FnResult<Json<DestroyOutput>> {
    let cfg = load_config()?;
    let body = json!({ "SecretId": input.path, "ForceDeleteWithoutRecovery": true });
    let (status, body) = call(
        &cfg,
        "secretsmanager.DeleteSecret",
        &serde_json::to_vec(&body)?,
    )?;
    if status >= 300 {
        return Err(err_response(status, &body));
    }
    Ok(Json(DestroyOutput { destroyed: true }))
}

#[plugin_fn]
pub fn restore_secret(Json(input): Json<RestoreInput>) -> FnResult<Json<RestoreOutput>> {
    let cfg = load_config()?;
    let body = json!({ "SecretId": input.path });
    let (status, body) = call(
        &cfg,
        "secretsmanager.RestoreSecret",
        &serde_json::to_vec(&body)?,
    )?;
    if status >= 300 {
        return Err(err_response(status, &body));
    }
    Ok(Json(RestoreOutput { restored: true }))
}

#[plugin_fn]
pub fn describe(Json(input): Json<DescribeInput>) -> FnResult<Json<DescribeOutput>> {
    let cfg = load_config()?;

    let (status, body) = call(
        &cfg,
        "secretsmanager.DescribeSecret",
        &serde_json::to_vec(&json!({ "SecretId": input.path }))?,
    )?;

    if status >= 300 {
        return Err(err_response(status, &body));
    }

    let meta: Value = serde_json::from_slice(&body)?;

    let deletion_time = meta
        .get("DeletedDate")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let current_status = if deletion_time.is_some() {
        SecretStatus::SoftDeleted
    } else {
        SecretStatus::Present
    };

    let mut versions: Vec<VersionInfo> = Vec::new();
    let mut current_version: Option<String> = None;
    let (vstatus, vbody) = call(
        &cfg,
        "secretsmanager.ListSecretVersionIds",
        &serde_json::to_vec(&json!({ "SecretId": input.path, "IncludeDeprecated": true }))?,
    )?;
    if vstatus < 300 {
        let parsed: Value = serde_json::from_slice(&vbody)?;
        if let Some(list) = parsed.get("Versions").and_then(Value::as_array) {
            for v in list {
                let id = v
                    .get("VersionId")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned();
                let is_current = v
                    .get("VersionStages")
                    .and_then(Value::as_array)
                    .map(|a| {
                        a.iter()
                            .filter_map(Value::as_str)
                            .any(|s| s == "AWSCURRENT")
                    })
                    .unwrap_or(false);
                if is_current {
                    current_version = Some(id.clone());
                }
                versions.push(VersionInfo {
                    id,
                    status: SecretStatus::Present,
                    created_time: v
                        .get("CreatedDate")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    deletion_time: None,
                });
            }
        }
    }

    let provider_hints = json!({
        "engine": "secretsmanager",
        "arn": meta.get("ARN"),
        "rotation_enabled": meta.get("RotationEnabled"),
        "kms_key_id": meta.get("KmsKeyId"),
    });

    Ok(Json(DescribeOutput {
        secret_name: input.path,
        provider: "aws".to_owned(),
        exists: true,
        current_version,
        current_status,
        created_time: meta
            .get("CreatedDate")
            .and_then(Value::as_str)
            .map(str::to_owned),
        updated_time: meta
            .get("LastChangedDate")
            .and_then(Value::as_str)
            .map(str::to_owned),
        versions,
        custom_metadata: meta.get("Tags").cloned(),
        provider_hints: Some(provider_hints),
    }))
}

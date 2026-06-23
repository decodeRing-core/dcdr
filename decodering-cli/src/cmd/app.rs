use std::error::Error;

use clap::{Args, Subcommand};

use crate::api::app::ActivateRequest;
use crate::api::app::AuthRequest;
use crate::api::app::ChallengeRequest;
use crate::api::app::GrantRequest;
use crate::api::app::ListUsersRequest;
use crate::api::app::RevokeRequest;
use crate::api::app::app_create;
use crate::api::app::app_user_auth;
use crate::api::app::app_user_auth_activate;
use crate::api::app::app_user_auth_challenge;
use crate::api::app::app_user_create;
use crate::api::app::app_user_grant;
use crate::api::app::app_user_list;
use crate::api::app::app_user_revoke;
use crate::api::app::{CreateAppRequest, CreateUserRequest};
use crate::output;
use crate::prompt;
use crate::source::ValueSource;
use crate::state;
use crate::token_store;

#[derive(Subcommand)]
pub enum AppCommand {
    /// Create an app
    Create(CreateInput),
    /// Manage app users
    #[command(subcommand)]
    User(UserCommand),
}

#[derive(Subcommand)]
pub enum UserCommand {
    /// Create a user and issue a credential
    Create(UserCreateInput),
    /// Grant a user access to apps
    Grant(GrantInput),
    /// Revoke a user's access to an app
    Revoke(RevokeInput),
    /// List the apps a user can access
    List(ListInput),
    /// Authenticate with a credential and obtain a session token
    Auth(AuthInput),
    /// Activate credential
    Activate(ActivateInput),
    /// Request an authentication challenge for a credential kind
    Challenge(ChallengeInput),
}

#[derive(Args)]
pub struct AuthInput {
    /// Params as JSON (inline or `@file`). When omitted, you'll be prompted.
    #[arg(long, value_name = "SOURCE")]
    params: Option<ValueSource>,
}

#[derive(Args)]
pub struct CreateInput {
    /// Params as JSON (inline or `@file`). When omitted, you'll be prompted.
    #[arg(long, value_name = "SOURCE")]
    params: Option<ValueSource>,
}

#[derive(Args)]
pub struct UserCreateInput {
    #[arg(long, value_name = "SOURCE")]
    params: Option<ValueSource>,
    /// Credential data as JSON (inline or `@file`, e.g. `@tpm.json`).
    /// When omitted for a non-apiKey kind, you'll be prompted field by field.
    #[arg(long, value_name = "SOURCE")]
    data: Option<ValueSource>,
}

#[derive(Args)]
pub struct GrantInput {
    #[arg(long, value_name = "SOURCE")]
    params: Option<ValueSource>,
}

#[derive(Args)]
pub struct RevokeInput {
    #[arg(long, value_name = "SOURCE")]
    params: Option<ValueSource>,
}

#[derive(Args)]
pub struct ListInput {
    #[arg(long, value_name = "SOURCE")]
    params: Option<ValueSource>,
}

#[derive(Args)]
pub struct ChallengeInput {
    #[arg(long, value_name = "SOURCE")]
    params: Option<ValueSource>,
    #[arg(long)]
    credential_kind: Option<String>,
}

#[derive(Args)]
pub struct ActivateInput {
    /// Whole request as JSON (inline or `@file`); skips prompts and proof generation.
    #[arg(long, value_name = "SOURCE")]
    params: Option<ValueSource>,
    /// Proof as JSON (inline or `@file`); use when you already have it.
    #[arg(long, value_name = "SOURCE")]
    proof: Option<ValueSource>,
    #[arg(long)]
    credential_kind: Option<String>,
    #[arg(long)]
    principal_id: Option<String>,
    #[arg(long)]
    credential_id: Option<String>,
}

pub async fn run(cmd: AppCommand, addr: &str) -> Result<(), Box<dyn Error>> {
    match cmd {
        AppCommand::Create(i) => create(i, addr, &token()?).await,
        AppCommand::User(UserCommand::Create(i)) => user_create(i, addr, &token()?).await,
        AppCommand::User(UserCommand::Auth(i)) => user_auth(i, addr).await,
        AppCommand::User(UserCommand::Grant(i)) => user_grant(i, addr, &token()?).await,
        AppCommand::User(UserCommand::Revoke(i)) => user_revoke(i, addr, &token()?).await,
        AppCommand::User(UserCommand::List(i)) => user_list(i, addr, &token()?).await,
        AppCommand::User(UserCommand::Activate(i)) => user_activate(i, addr).await,
        AppCommand::User(UserCommand::Challenge(i)) => user_challenge(i, addr).await,
    }
}

fn prompt_principal() -> Result<String, Box<dyn Error>> {
    let id = match state::last_principal() {
        Some(prev) => prompt::with_default("Principal id", &prev)?,
        None => prompt::required("Principal id")?,
    };
    state::set_last_principal(&id)?;
    Ok(id)
}

fn token() -> Result<String, Box<dyn Error>> {
    token_store::load()?
        .ok_or_else(|| Box::<dyn Error>::from("no root token found; run `system init` first"))
}

async fn create(input: CreateInput, addr: &str, token: &str) -> Result<(), Box<dyn Error>> {
    let req: CreateAppRequest = match &input.params {
        Some(src) => serde_json::from_str(&src.read()?)?,
        None => CreateAppRequest {
            app_name: required("App name: ")?,
        },
    };
    let resp = app_create(addr, token, req).await?;
    output::report(&resp);
    Ok(())
}

async fn user_create(
    input: UserCreateInput,
    addr: &str,
    token: &str,
) -> Result<(), Box<dyn Error>> {
    let req: CreateUserRequest = if let Some(src) = &input.params {
        serde_json::from_str(&src.read()?)?
    } else {
        let name = prompt::required("Name")?;
        let kind = prompt::with_default("Kind", "human")?;
        let credential_kind = prompt::with_default("Credential kind", "apiKey")?;
        let data = credential_data(&credential_kind, input.data.as_ref())?;
        CreateUserRequest {
            name,
            kind,
            credential_kind,
            data,
        }
    };
    let resp = app_user_create(addr, token, req).await?;

    if let Some(id) = resp
        .data
        .as_ref()
        .and_then(|d| d.get("principal_id"))
        .and_then(|v| v.as_str())
    {
        let _ = state::set_last_principal(id);
    }
    output::report(&resp);
    Ok(())
}

fn normalize_pem_fields(mut v: serde_json::Value) -> serde_json::Value {
    if let Some(obj) = v.as_object_mut() {
        for key in ["ek_pubkey_pem", "ek_cert_pem"] {
            if let Some(serde_json::Value::String(s)) = obj.get_mut(key)
                && s.contains("\\n")
            {
                *s = s.replace("\\n", "\n");
            }
        }
    }
    v
}

fn credential_data(
    credential_kind: &str,
    data_src: Option<&ValueSource>,
) -> Result<Option<serde_json::Value>, Box<dyn Error>> {
    if let Some(src) = data_src {
        let value: serde_json::Value = serde_json::from_str(&src.read()?)?;
        return Ok(Some(normalize_pem_fields(unwrap_tpm(value))));
    }
    match credential_kind {
        "apiKey" => Ok(None),
        "trustedPlatformModule" => Ok(Some(prompt_tpm_data()?)),
        "awsIdentity" => Ok(Some(prompt_aws_data()?)),
        other => Err(format!("unknown credential kind: {other}").into()),
    }
}

fn unwrap_tpm(v: serde_json::Value) -> serde_json::Value {
    match &v {
        serde_json::Value::Object(m) if m.len() == 1 && m.contains_key("tpm") => {
            m.get("tpm").cloned().unwrap_or(v)
        }
        _ => v,
    }
}

fn unescape_pem(s: &str) -> String {
    s.replace("\\r\\n", "\n")
        .replace("\\n", "\n")
        .replace("\\r", "\n")
}

fn prompt_tpm_data() -> Result<serde_json::Value, Box<dyn Error>> {
    let ek_pubkey_pem = unescape_pem(&required("EK public key PEM: ")?);
    let ek_cert = prompt::line("EK cert PEM [none]: ")?;
    let ek_cert_pem = if ek_cert.trim().is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::Value::String(unescape_pem(&ek_cert))
    };
    let ak_public_tpm2b_b64 = required("AK public TPM2B (base64): ")?;
    let require_ek_cert =
        or_default(prompt::line("Require EK cert [false]: ")?, "false").trim() == "true";

    let mut pcrs = serde_json::Map::new();
    for i in 0..=7 {
        let hex = required(&format!("PCR {i} (sha256 hex): "))?;
        pcrs.insert(i.to_string(), serde_json::Value::String(hex));
    }

    Ok(serde_json::json!({
        "ek_pubkey_pem": ek_pubkey_pem,
        "ek_cert_pem": ek_cert_pem,
        "ak_public_tpm2b_b64": ak_public_tpm2b_b64,
        "expected_pcrs": serde_json::Value::Object(pcrs),
        "require_ek_cert": require_ek_cert,
    }))
}

fn prompt_aws_data() -> Result<serde_json::Value, Box<dyn Error>> {
    let role_arn = required("Role ARN: ")?;
    Ok(serde_json::json!({ "role_arn": role_arn }))
}

async fn user_grant(input: GrantInput, addr: &str, token: &str) -> Result<(), Box<dyn Error>> {
    let req: GrantRequest = match &input.params {
        Some(src) => serde_json::from_str(&src.read()?)?,
        None => GrantRequest {
            principal_id: prompt_principal()?,
            apps: prompt_list("App id")?,
        },
    };
    let resp = app_user_grant(addr, token, req).await?;
    output::report(&resp);
    Ok(())
}

async fn user_revoke(input: RevokeInput, addr: &str, token: &str) -> Result<(), Box<dyn Error>> {
    let req: RevokeRequest = match &input.params {
        Some(src) => serde_json::from_str(&src.read()?)?,
        None => RevokeRequest {
            principal_id: prompt_principal()?,
            app_id: required("App id: ")?,
        },
    };
    let resp = app_user_revoke(addr, token, req).await?;
    output::report(&resp);
    Ok(())
}

async fn user_list(input: ListInput, addr: &str, token: &str) -> Result<(), Box<dyn Error>> {
    let req: ListUsersRequest = match &input.params {
        Some(src) => serde_json::from_str(&src.read()?)?,
        None => ListUsersRequest {
            principal_id: prompt_principal()?,
        },
    };
    let resp = app_user_list(addr, token, req).await?;
    output::report(&resp);
    Ok(())
}

fn required(label: &str) -> Result<String, Box<dyn Error>> {
    let value = prompt::line(label)?;
    if value.is_empty() {
        return Err(format!("{} is required", label.trim_end_matches([':', ' '])).into());
    }
    Ok(value)
}

fn or_default(value: String, default: &str) -> String {
    if value.is_empty() {
        return default.to_owned();
    }
    value
}

fn prompt_list(label: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let mut items = Vec::new();
    loop {
        let line = prompt::line(&format!("{label} {} (empty to finish)", items.len() + 1))?;
        if line.is_empty() {
            break;
        }
        items.push(line);
    }
    if items.is_empty() {
        return Err("at least one app is required".into());
    }
    Ok(items)
}

async fn user_auth(input: AuthInput, addr: &str) -> Result<(), Box<dyn Error>> {
    let req: AuthRequest = if let Some(src) = &input.params {
        serde_json::from_str(&src.read()?)?
    } else {
        let credential_kind = or_default(prompt::line("Credential kind [apiKey]: ")?, "apiKey");
        let proof = match credential_kind.as_str() {
            "apiKey" => {
                let key = prompt::password("API key")?;
                if key.is_empty() {
                    return Err("api key is required".into());
                }
                serde_json::json!({ "key": key })
            }
            other => {
                return Err(format!(
                    "interactive auth supports apiKey only; use --params for `{other}`"
                )
                .into());
            }
        };
        AuthRequest {
            credential_kind,
            proof,
        }
    };

    let resp = app_user_auth(addr, req).await?;

    if let Some(data) = &resp.data
        && let (Some(token), Some(expires_at)) = (
            data.get("token").and_then(|v| v.as_str()),
            data.get("expires_at").and_then(serde_json::Value::as_i64),
        )
    {
        let _ = token_store::save_session(&token_store::Session {
            token: token.to_owned(),
            expires_at,
        });
    }

    output::report(&resp);
    Ok(())
}

async fn user_activate(input: ActivateInput, addr: &str) -> Result<(), Box<dyn Error>> {
    let req: ActivateRequest = match &input.params {
        Some(src) => serde_json::from_str(&src.read()?)?,
        None => {
            let credential_kind = match &input.credential_kind {
                Some(k) => k.clone(),
                None => or_default(prompt::line("Credential kind [apiKey]: ")?, "apiKey"),
            };
            let principal_id = match &input.principal_id {
                Some(p) => p.clone(),
                None => required("Principal id: ")?,
            };
            let credential_id = match &input.credential_id {
                Some(c) => c.clone(),
                None => required("Credential id: ")?,
            };
            let proof = build_proof(&credential_kind, input.proof.as_ref())?;
            ActivateRequest {
                credential_kind,
                principal_id,
                credential_id,
                proof,
            }
        }
    };
    let resp = app_user_auth_activate(addr, req).await?;
    output::report(&resp);
    Ok(())
}

fn build_proof(
    credential_kind: &str,
    proof_src: Option<&ValueSource>,
) -> Result<serde_json::Value, Box<dyn Error>> {
    if let Some(src) = proof_src {
        return Ok(serde_json::from_str(&src.read()?)?);
    }
    match credential_kind {
        "trustedPlatformModule" => {
            let recovered_secret = required("Recovered secret: ")?;
            Ok(serde_json::json!({ "recovered_secret": recovered_secret }))
        }
        other => Err(format!(
            "no proof builder for credential kind `{other}`; \
             supply it directly with --proof @proof.json"
        )
        .into()),
    }
}

async fn user_challenge(input: ChallengeInput, addr: &str) -> Result<(), Box<dyn Error>> {
    let req: ChallengeRequest = if let Some(src) = &input.params {
        serde_json::from_str(&src.read()?)?
    } else {
        let credential_kind = match input.credential_kind.clone() {
            Some(k) => k,
            None => or_default(prompt::line("Credential kind [apiKey]: ")?, "apiKey"),
        };
        ChallengeRequest { credential_kind }
    };
    let resp = app_user_auth_challenge(addr, req).await?;
    output::report(&resp);
    Ok(())
}

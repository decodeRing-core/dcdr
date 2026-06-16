use std::error::Error;

use clap::{Args, Subcommand};

use crate::{api, output, prompt, source::ValueSource, state, token_store};

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

pub async fn run(cmd: AppCommand, addr: &str) -> Result<(), Box<dyn Error>> {
    match cmd {
        AppCommand::Create(i) => create(i, addr, &token()?).await,
        AppCommand::User(UserCommand::Create(i)) => user_create(i, addr, &token()?).await,
        AppCommand::User(UserCommand::Auth(i)) => user_auth(i, addr).await,
        AppCommand::User(UserCommand::Grant(i)) => user_grant(i, addr, &token()?).await,
        AppCommand::User(UserCommand::Revoke(i)) => user_revoke(i, addr, &token()?).await,
        AppCommand::User(UserCommand::List(i)) => user_list(i, addr, &token()?).await,
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
    let req: api::CreateAppRequest = match &input.params {
        Some(src) => serde_json::from_str(&src.read()?)?,
        None => api::CreateAppRequest {
            app_name: required("App name: ")?,
        },
    };
    let resp = api::app_create(addr, token, req).await?;
    output::report(&resp);
    Ok(())
}

async fn user_create(
    input: UserCreateInput,
    addr: &str,
    token: &str,
) -> Result<(), Box<dyn Error>> {
    let req: api::CreateUserRequest = match &input.params {
        Some(src) => serde_json::from_str(&src.read()?)?,
        None => api::CreateUserRequest {
            name: prompt::required("Name")?,
            kind: prompt::with_default("Kind", "human")?,
            credential_kind: prompt::with_default("Credential kind", "apiKey")?,
        },
    };
    let resp = api::app_user_create(addr, token, req).await?;

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

async fn user_grant(input: GrantInput, addr: &str, token: &str) -> Result<(), Box<dyn Error>> {
    let req: api::GrantRequest = match &input.params {
        Some(src) => serde_json::from_str(&src.read()?)?,
        None => api::GrantRequest {
            principal_id: prompt_principal()?,
            apps: prompt_list("App id")?,
        },
    };
    let resp = api::app_user_grant(addr, token, req).await?;
    output::report(&resp);
    Ok(())
}

async fn user_revoke(input: RevokeInput, addr: &str, token: &str) -> Result<(), Box<dyn Error>> {
    let req: api::RevokeRequest = match &input.params {
        Some(src) => serde_json::from_str(&src.read()?)?,
        None => api::RevokeRequest {
            principal_id: prompt_principal()?,
            app_id: required("App id: ")?,
        },
    };
    let resp = api::app_user_revoke(addr, token, req).await?;
    output::report(&resp);
    Ok(())
}

async fn user_list(input: ListInput, addr: &str, token: &str) -> Result<(), Box<dyn Error>> {
    let req: api::ListUsersRequest = match &input.params {
        Some(src) => serde_json::from_str(&src.read()?)?,
        None => api::ListUsersRequest {
            principal_id: prompt_principal()?,
        },
    };
    let resp = api::app_user_list(addr, token, req).await?;
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
    let req: api::AuthRequest = if let Some(src) = &input.params {
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
        api::AuthRequest {
            credential_kind,
            proof,
        }
    };

    let resp = api::app_user_auth(addr, req).await?;

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

use std::error::Error;

use clap::{Args, Subcommand};

use crate::{api, output, prompt, source::ValueSource, state, token_store};

#[derive(Subcommand)]
pub enum OslCommand {
    #[command(subcommand)]
    Secrets(SecretsCommand),
    #[command(subcommand)]
    Capabilities(CapabilitiesCommand),
    #[command(subcommand)]
    Apps(AppsCommand),
    #[command(subcommand)]
    Backends(BackendsCommand),
}

#[derive(Subcommand)]
pub enum SecretsCommand {
    /// Create or update a secret
    Put(PutInput),
    /// Read a secret
    Get(GetInput),
    /// List secrets for an app
    List(AppRefInput),
    /// Mark a secret tainted
    Taint(SecretRefInput),
    /// Clear the tainted mark
    Untaint(SecretRefInput),
    /// Check whether a secret is tainted
    IsTainted(SecretRefInput),
    /// Describe a secret's metadata
    Describe(SecretRefInput),
    /// Restore a deleted secret
    Restore(SecretRefInput),
    /// Permanently destroy a secret
    Destroy(SecretRefInput),
    /// Soft-delete a secret
    Delete(SecretRefInput),
}

#[derive(Subcommand)]
pub enum CapabilitiesCommand {
    /// Report capabilities
    Get,
}

#[derive(Subcommand)]
pub enum AppsCommand {
    /// List apps
    List,
}

#[derive(Subcommand)]
pub enum BackendsCommand {
    /// List backends
    List,
}

#[derive(Args)]
pub struct PutInput {
    #[arg(long, value_name = "SOURCE")]
    params: Option<ValueSource>,
}

#[derive(Args)]
pub struct GetInput {
    #[arg(long, value_name = "SOURCE")]
    params: Option<ValueSource>,
}

#[derive(Args)]
pub struct AppRefInput {
    #[arg(long, value_name = "SOURCE")]
    params: Option<ValueSource>,
}

#[derive(Args)]
pub struct SecretRefInput {
    #[arg(long, value_name = "SOURCE")]
    params: Option<ValueSource>,
}

pub async fn run(cmd: OslCommand, addr: &str) -> Result<(), Box<dyn Error>> {
    let token = token_store::resolve_token()?;
    let resp = match cmd {
        OslCommand::Secrets(c) => return secrets(c, addr, &token).await,
        OslCommand::Capabilities(CapabilitiesCommand::Get) => {
            api::osl_capabilities_get(addr, &token).await?
        }
        OslCommand::Apps(AppsCommand::List) => api::osl_apps_list(addr, &token).await?,
        OslCommand::Backends(BackendsCommand::List) => api::osl_backends_list(addr, &token).await?,
    };
    output::report(&resp);
    Ok(())
}

async fn secrets(cmd: SecretsCommand, addr: &str, token: &str) -> Result<(), Box<dyn Error>> {
    use SecretsCommand as S;
    let resp = match cmd {
        S::Put(i) => api::osl_secrets_put(addr, token, put_req(i)?).await?,
        S::Get(i) => api::osl_secrets_get(addr, token, get_req(i)?).await?,
        S::List(i) => api::osl_secrets_list(addr, token, app_ref_req(i)?).await?,
        S::Taint(i) => api::osl_secrets_taint(addr, token, secret_ref_req(i)?).await?,
        S::Untaint(i) => api::osl_secrets_untaint(addr, token, secret_ref_req(i)?).await?,
        S::IsTainted(i) => api::osl_secrets_is_tainted(addr, token, secret_ref_req(i)?).await?,
        S::Describe(i) => api::osl_secrets_describe(addr, token, secret_ref_req(i)?).await?,
        S::Restore(i) => api::osl_secrets_restore(addr, token, secret_ref_req(i)?).await?,
        S::Destroy(i) => api::osl_secrets_destroy(addr, token, secret_ref_req(i)?).await?,
        S::Delete(i) => api::osl_secrets_delete(addr, token, secret_ref_req(i)?).await?,
    };
    output::report(&resp);
    Ok(())
}

fn secret_ref_req(input: SecretRefInput) -> Result<api::SecretRef, Box<dyn Error>> {
    match input.params {
        Some(src) => Ok(serde_json::from_str(&src.read()?)?),
        None => Ok(api::SecretRef {
            app_id: prompt_app_id()?,
            secret_name: prompt::required("Secret name: ")?,
        }),
    }
}

fn app_ref_req(input: AppRefInput) -> Result<api::AppRef, Box<dyn Error>> {
    match input.params {
        Some(src) => Ok(serde_json::from_str(&src.read()?)?),
        None => Ok(api::AppRef {
            app_id: prompt_app_id()?,
        }),
    }
}

fn get_req(input: GetInput) -> Result<api::GetSecretRequest, Box<dyn Error>> {
    match input.params {
        Some(src) => Ok(serde_json::from_str(&src.read()?)?),
        None => Ok(api::GetSecretRequest {
            app_id: prompt_app_id()?,
            secret_name: prompt::required("Secret name: ")?,
            version: prompt::or_default(prompt::line("Version [0]: ")?, "0"),
        }),
    }
}

fn put_req(input: PutInput) -> Result<api::PutRequest, Box<dyn Error>> {
    match input.params {
        Some(src) => Ok(serde_json::from_str(&src.read()?)?),
        None => Ok(api::PutRequest {
            app_id: prompt_app_id()?,
            secret_name: prompt::required("Secret name: ")?,
            store: api::SecretStore {
                backend_ref: prompt::required("Backend ref: ")?,
                store_path: prompt::required("Store path: ")?,
            },
            data: prompt_secret_data()?,
            options: api::PutOptions {
                create_only: prompt_bool("Create only? [y/N]: ")?,
            },
        }),
    }
}

fn prompt_app_id() -> Result<String, Box<dyn Error>> {
    let last = state::last_app();
    let label = last
        .as_ref()
        .map_or_else(|| "App id: ".to_owned(), |id| format!("App id [{id}]: "));
    let input = prompt::line(&label)?;
    let id = if input.is_empty() {
        last.ok_or("app id is required")?
    } else {
        input
    };
    state::set_last_app(&id)?;
    Ok(id)
}

fn prompt_secret_data() -> Result<serde_json::Map<String, serde_json::Value>, Box<dyn Error>> {
    let mut data = serde_json::Map::new();
    loop {
        let key = prompt::line("Data key (empty to finish): ")?;
        if key.is_empty() {
            break;
        }
        let value = rpassword::prompt_password("  value: ")?.trim().to_owned();
        data.insert(key, serde_json::Value::String(value));
    }
    if data.is_empty() {
        return Err("at least one data entry is required".into());
    }
    Ok(data)
}

fn prompt_bool(label: &str) -> Result<bool, Box<dyn Error>> {
    let value = prompt::line(label)?;
    Ok(matches!(
        value.to_lowercase().as_str(),
        "y" | "yes" | "true"
    ))
}

// app-core/src/plugin/wasm.rs
use extism::convert::Json;
use extism::{Manifest, Plugin};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::error::PluginError;
use super::secret_backend::{ReadResponse, SecretBackend};

#[derive(Serialize)]
struct ReadInput<'a> {
    secret_name: &'a str,
    version: Option<String>,
}

#[derive(Serialize)]
struct WriteInput<'a> {
    path: &'a str,
    data: &'a Value,
}

#[derive(Deserialize)]
struct WriteOutput {
    version: String,
}

#[derive(Serialize)]
struct DeleteInput<'a> {
    path: &'a str,
}

#[derive(Deserialize)]
pub struct DeleteSecretOutput {
    pub deleted: bool,
}

pub struct WasmSecretBackend {
    manifest: Manifest,
}

impl WasmSecretBackend {
    pub fn new(manifest: Manifest) -> Self {
        Self { manifest }
    }

    /// Intentional: per-call isolation is a security requirement, not a performance oversight.
    fn instantiate(&self) -> Result<Plugin, PluginError> {
        Plugin::new(&self.manifest, [], true).map_err(|e| PluginError::Instantiation(e.to_string()))
    }
}

impl SecretBackend for WasmSecretBackend {
    fn get(&self, secret_name: &str, version: Option<String>) -> Result<ReadResponse, PluginError> {
        let mut plugin = self.instantiate()?;
        let input = ReadInput {
            secret_name,
            version,
        };
        plugin
            .call::<Json<ReadInput>, Json<ReadResponse>>("get_secret", Json(input))
            .map(|out| out.0)
            .map_err(|e| PluginError::Call {
                function: "get_secret".into(),
                message: e.to_string(),
            })
    }

    fn put(&self, path: &str, data: &Value) -> Result<String, PluginError> {
        let mut plugin = self.instantiate()?;
        let input = WriteInput { path, data };
        plugin
            .call::<Json<WriteInput>, Json<WriteOutput>>("put_secret", Json(input))
            .map(|out| out.0.version)
            .map_err(|e| PluginError::Call {
                function: "put_secret".into(),
                message: e.to_string(),
            })
    }

    fn destroy(&self, path: &str) -> Result<bool, PluginError> {
        let mut plugin = self.instantiate()?;
        let input = DeleteInput { path };
        plugin
            .call::<Json<DeleteInput>, Json<DeleteSecretOutput>>("destroy_secret", Json(input))
            .map(|out| out.0.deleted)
            .map_err(|e| PluginError::Call {
                function: "destroy_secret".into(),
                message: e.to_string(),
            })
    }
}

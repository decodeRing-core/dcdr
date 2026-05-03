// app-core/src/plugin/wasm.rs
use extism::convert::Json;
use extism::{Manifest, Plugin};
use serde::Serialize;
use serde_json::Value;

use super::error::PluginError;
use super::secret_backend::{ReadResponse, SecretBackend};

#[derive(Serialize)]
struct ReadInput<'a> {
    secret_name: &'a str,
    version: Option<u64>,
}

#[derive(Serialize)]
struct WriteInput<'a> {
    path: &'a str,
    data: &'a Value,
}

#[derive(serde::Deserialize)]
struct WriteOutput {
    version: u64,
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
    fn get(&self, secret_name: &str, version: Option<u64>) -> Result<ReadResponse, PluginError> {
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

    fn put(&self, path: &str, data: &Value) -> Result<u64, PluginError> {
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
}

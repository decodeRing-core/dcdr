use extism::convert::Json;
use extism::{Manifest, Plugin};
use serde_json::Value;

use crate::plugin::osl_contract::{
    DeleteInput, DeleteOutput, DestroyInput, DestroyOutput, ReadInput, ReadResponse, RestoreInput,
    RestoreOutput, WriteInput, WriteOutput,
};
use crate::plugin::secret_backend::Capability;

use super::error::PluginError;
use super::secret_backend::SecretBackend;

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
            secret_name: secret_name.to_owned(),
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
        let input = WriteInput {
            path: path.to_owned(),
            data: data.to_owned(),
        };
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
        let input = DestroyInput {
            path: path.to_owned(),
        };
        plugin
            .call::<Json<DestroyInput>, Json<DestroyOutput>>("destroy_secret", Json(input))
            .map(|out| out.0.destroyed)
            .map_err(|e| PluginError::Call {
                function: "destroy_secret".into(),
                message: e.to_string(),
            })
    }

    fn delete(&self, path: &str) -> Result<bool, PluginError> {
        let mut plugin = self.instantiate()?;
        let input = DeleteInput {
            path: path.to_owned(),
        };
        plugin
            .call::<Json<DeleteInput>, Json<DeleteOutput>>("delete_secret", Json(input))
            .map(|out| out.0.deleted)
            .map_err(|e| PluginError::Call {
                function: "delete_secret".into(),
                message: e.to_string(),
            })
    }

    fn restore(&self, path: &str) -> Result<bool, PluginError> {
        let mut plugin = self.instantiate()?;
        let input = RestoreInput {
            path: path.to_owned(),
        };
        plugin
            .call::<Json<RestoreInput>, Json<RestoreOutput>>("restore_secret", Json(input))
            .map(|out| out.0.restored)
            .map_err(|e| PluginError::Call {
                function: "restore_secret".into(),
                message: e.to_string(),
            })
    }
    fn capabilities(&self) -> Result<Vec<Capability>, PluginError> {
        let mut plugin = self.instantiate()?;
        plugin
            .call::<(), Json<Vec<Capability>>>("capabilities", ())
            .map(|out| out.0)
            .map_err(|e| PluginError::Call {
                function: "capabilities".into(),
                message: e.to_string(),
            })
    }
}

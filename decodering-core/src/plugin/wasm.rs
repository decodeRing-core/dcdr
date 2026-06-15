use std::collections::BTreeMap;
use std::sync::Arc;

use extism::convert::Json;
use extism::{Manifest, Plugin};
use serde_json::Value;
use zeroize::Zeroizing;

use crate::metrics::Metrics;
use crate::metrics::plugin_invocation::PluginInvocation;
use crate::plugin::osl_contract::{
    Capability, DeleteInput, DeleteOutput, DescribeInput, DescribeOutput, DestroyInput,
    DestroyOutput, ReadInput, ReadOutput, RestoreInput, RestoreOutput, WriteInput, WriteOutput,
};

use super::error::PluginError;
use super::secret_backend::SecretBackend;

pub struct WasmSecretBackend {
    backend_type: String,
    manifest: Manifest,
    metrics: Arc<dyn Metrics>,
}

impl WasmSecretBackend {
    pub fn new(manifest: Manifest, backend_type: String, metrics: Arc<dyn Metrics>) -> Self {
        Self {
            backend_type,
            manifest,
            metrics,
        }
    }

    /// Intentional: per-call isolation is a security requirement
    fn instantiate(
        &self,
        credential: &BTreeMap<String, Zeroizing<String>>,
    ) -> Result<Plugin, PluginError> {
        let mut m = self.manifest.clone();
        for (k, v) in credential {
            m.config.insert(k.clone(), v.to_string());
        }
        Plugin::new(&m, [], true).map_err(|e| PluginError::Instantiation(e.to_string()))
    }
}

impl SecretBackend for WasmSecretBackend {
    fn get(
        &self,
        secret_name: &str,
        version: Option<String>,
        credential: &BTreeMap<String, Zeroizing<String>>,
    ) -> Result<ReadOutput, PluginError> {
        let mut metric = PluginInvocation::start(self.metrics.clone(), self.backend_type.clone());
        let mut plugin = self.instantiate(credential)?;
        let input = ReadInput {
            secret_name: secret_name.to_owned(),
            version,
        };
        plugin
            .call::<Json<ReadInput>, Json<ReadOutput>>("get_secret", Json(input))
            .map(|out| {
                metric.ok();
                out.0
            })
            .map_err(|e| PluginError::Call {
                function: "get_secret".into(),
                message: e.to_string(),
            })
    }

    fn put(
        &self,
        path: &str,
        data: &Value,
        idempotency_token: &str,
        credential: &BTreeMap<String, Zeroizing<String>>,
    ) -> Result<String, PluginError> {
        let mut metric = PluginInvocation::start(self.metrics.clone(), self.backend_type.clone());
        let mut plugin = self.instantiate(credential)?;
        let input = WriteInput {
            path: path.to_owned(),
            data: data.to_owned(),
            idempotency_token: idempotency_token.to_owned(),
        };
        plugin
            .call::<Json<WriteInput>, Json<WriteOutput>>("put_secret", Json(input))
            .map(|out| {
                metric.ok();
                out.0.version
            })
            .map_err(|e| PluginError::Call {
                function: "put_secret".into(),
                message: e.to_string(),
            })
    }

    fn destroy(
        &self,
        path: &str,
        credential: &BTreeMap<String, Zeroizing<String>>,
    ) -> Result<bool, PluginError> {
        let mut metric = PluginInvocation::start(self.metrics.clone(), self.backend_type.clone());
        let mut plugin = self.instantiate(credential)?;
        let input = DestroyInput {
            path: path.to_owned(),
        };
        plugin
            .call::<Json<DestroyInput>, Json<DestroyOutput>>("destroy_secret", Json(input))
            .map(|out| {
                metric.ok();
                out.0.destroyed
            })
            .map_err(|e| PluginError::Call {
                function: "destroy_secret".into(),
                message: e.to_string(),
            })
    }

    fn delete(
        &self,
        path: &str,
        credential: &BTreeMap<String, Zeroizing<String>>,
    ) -> Result<bool, PluginError> {
        let mut metric = PluginInvocation::start(self.metrics.clone(), self.backend_type.clone());
        let mut plugin = self.instantiate(credential)?;
        let input = DeleteInput {
            path: path.to_owned(),
        };
        plugin
            .call::<Json<DeleteInput>, Json<DeleteOutput>>("delete_secret", Json(input))
            .map(|out| {
                metric.ok();
                out.0.deleted
            })
            .map_err(|e| PluginError::Call {
                function: "delete_secret".into(),
                message: e.to_string(),
            })
    }

    fn restore(
        &self,
        path: &str,
        credential: &BTreeMap<String, Zeroizing<String>>,
    ) -> Result<bool, PluginError> {
        let mut metric = PluginInvocation::start(self.metrics.clone(), self.backend_type.clone());
        let mut plugin = self.instantiate(credential)?;
        let input = RestoreInput {
            path: path.to_owned(),
        };
        plugin
            .call::<Json<RestoreInput>, Json<RestoreOutput>>("restore_secret", Json(input))
            .map(|out| {
                metric.ok();
                out.0.restored
            })
            .map_err(|e| PluginError::Call {
                function: "restore_secret".into(),
                message: e.to_string(),
            })
    }
    fn capabilities(
        &self,
        credential: &BTreeMap<String, Zeroizing<String>>,
    ) -> Result<Vec<Capability>, PluginError> {
        let mut metric = PluginInvocation::start(self.metrics.clone(), self.backend_type.clone());
        let mut plugin = self.instantiate(credential)?;

        plugin
            .call::<(), Json<Vec<Capability>>>("capabilities", ())
            .map(|out| {
                metric.ok();
                out.0
            })
            .map_err(|e| PluginError::Call {
                function: "capabilities".into(),
                message: e.to_string(),
            })
    }

    fn describe(
        &self,
        path: &str,
        credential: &BTreeMap<String, Zeroizing<String>>,
    ) -> Result<DescribeOutput, PluginError> {
        let mut metric = PluginInvocation::start(self.metrics.clone(), self.backend_type.clone());
        let mut plugin = self.instantiate(credential)?;
        let input = DescribeInput {
            path: path.to_owned(),
        };
        plugin
            .call::<Json<DescribeInput>, Json<DescribeOutput>>("describe", Json(input))
            .map(|out| {
                metric.ok();
                out.0
            })
            .map_err(|e| PluginError::Call {
                function: "describe".into(),
                message: e.to_string(),
            })
    }
}

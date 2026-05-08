use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use extism::Manifest;

use super::error::PluginError;
use super::secret_backend::SecretBackend;
use super::wasm::WasmSecretBackend;

#[derive(Clone)]
pub struct Orchestrator {
    backends: HashMap<String, Arc<dyn SecretBackend>>,
}

impl Orchestrator {
    pub fn new() -> Self {
        Self {
            backends: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: String, backend: Arc<dyn SecretBackend>) {
        self.backends.insert(name, backend);
    }

    pub fn get_backend(&self, name: &str) -> Result<&Arc<dyn SecretBackend>, PluginError> {
        self.backends
            .get(name)
            .ok_or_else(|| PluginError::BackendNotFound(name.into()))
    }

    pub fn load_wasm_plugins_from_dir(&mut self, plugins_root: &str) -> Result<(), PluginError> {
        let manifests_dir = Path::new(plugins_root).join("manifests");

        let entries = fs::read_dir(&manifests_dir)
            .map_err(|e| PluginError::Io(format!("read_dir {}: {e}", manifests_dir.display())))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
                continue;
            }

            let Some(name) = path.file_stem().and_then(|s| s.to_str()) else {
                tracing::warn!(?path, "invalid plugin filename");
                continue;
            };

            let yaml = match fs::read_to_string(&path) {
                Ok(d) => d,
                Err(e) => {
                    tracing::warn!(?path, error = %e, "read failed");
                    continue;
                }
            };

            let manifest: Manifest = match serde_yaml::from_str(&yaml) {
                Ok(m) => m,
                Err(e) => {
                    tracing::warn!(?path, error = %e, "yaml parse failed");
                    continue;
                }
            };

            tracing::info!(backend = name, "loaded plugin manifest");
            self.register(name.to_owned(), Arc::new(WasmSecretBackend::new(manifest)));
        }
        Ok(())
    }
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self::new()
    }
}

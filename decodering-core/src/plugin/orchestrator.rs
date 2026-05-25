use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;

use extism::Manifest;
use serde::Serialize;
use zeroize::Zeroizing;

use crate::plugin::osl_contract::Capability;

use super::error::PluginError;
use super::secret_backend::SecretBackend;
use super::wasm::WasmSecretBackend;

const SERVER_CAPABILITIES: [Capability; 1] = [Capability::KvTaint];

#[derive(Serialize)]
pub struct BackendCapabilities {
    pub backend_ref: String,
    pub backend_type: String,
    pub capabilities: Vec<Capability>,
}

#[derive(Clone)]
pub struct Orchestrator {
    backends: HashMap<String, BackendEntry>,
}

#[derive(Clone)]
pub struct BackendEntry {
    pub backend_type: String,
    pub backend: Arc<dyn SecretBackend>,
}

impl Orchestrator {
    pub fn new() -> Self {
        Self {
            backends: HashMap::new(),
        }
    }

    pub fn get_backends(&self) -> HashMap<String, BackendEntry> {
        self.backends.clone()
    }

    pub fn get_backend_capabilities(
        &self,
        credentials: &BTreeMap<String, BTreeMap<String, Zeroizing<String>>>,
    ) -> Vec<BackendCapabilities> {
        self.backends
            .iter()
            .filter_map(|(backend_ref, backend_entry)| {
                let credential = credentials.get(backend_ref)?;
                backend_entry
                    .backend
                    .capabilities(credential)
                    .ok()
                    .map(|capabilities| BackendCapabilities {
                        backend_ref: backend_ref.clone(),
                        backend_type: backend_entry.backend_type.clone(),
                        capabilities,
                    })
            })
            .collect()
    }

    pub fn get_server_capabilities(
        &self,
        credentials: &BTreeMap<String, BTreeMap<String, Zeroizing<String>>>,
    ) -> Vec<Capability> {
        self.backends
            .iter()
            .filter_map(|(key, backend_entry)| {
                if let Some(credential) = credentials.get(key) {
                    return backend_entry.backend.capabilities(credential).ok();
                }
                Some(vec![])
            })
            .flatten()
            .chain(SERVER_CAPABILITIES)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn register(&mut self, name: String, backend: BackendEntry) {
        self.backends.insert(name, backend);
    }

    pub fn get_backend(&self, name: &str) -> Result<&BackendEntry, PluginError> {
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

            let manifest: Manifest = match serde_norway::from_str(&yaml) {
                Ok(m) => m,
                Err(e) => {
                    tracing::warn!(?path, error = %e, "yaml parse failed");
                    continue;
                }
            };

            let Some(backend_type) = manifest.config.get("type").cloned() else {
                tracing::warn!(?path, "manifest missing config.type, skipping");
                continue;
            };

            let backend_entry = BackendEntry {
                backend_type,
                backend: Arc::new(WasmSecretBackend::new(manifest)),
            };
            tracing::info!(backend = name, "loaded plugin manifest");
            self.register(name.to_owned(), backend_entry);
        }
        Ok(())
    }
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self::new()
    }
}

use crate::plugin::osl_contract::{Capability, DescribeOutput, ReadOutput};

use super::error::PluginError;
use serde_json::Value;

pub trait SecretBackend: Send + Sync {
    fn get(&self, secret_name: &str, version: Option<String>) -> Result<ReadOutput, PluginError>;
    fn put(&self, path: &str, data: &Value) -> Result<String, PluginError>;
    /// Permently delete secret
    fn destroy(&self, path: &str) -> Result<bool, PluginError>;
    /// Soft delete secret
    fn delete(&self, path: &str) -> Result<bool, PluginError>;
    /// Restore soft deleted secret
    fn restore(&self, path: &str) -> Result<bool, PluginError>;
    fn capabilities(&self) -> Result<Vec<Capability>, PluginError>;
    fn describe(&self, path: &str) -> Result<DescribeOutput, PluginError>;
}

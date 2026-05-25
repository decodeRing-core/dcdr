use std::collections::BTreeMap;

use crate::plugin::osl_contract::{Capability, DescribeOutput, ReadOutput};

use super::error::PluginError;
use serde_json::Value;
use zeroize::Zeroizing;

pub trait SecretBackend: Send + Sync {
    fn get(
        &self,
        secret_name: &str,
        version: Option<String>,
        credential: &BTreeMap<String, Zeroizing<String>>,
    ) -> Result<ReadOutput, PluginError>;
    fn put(
        &self,
        path: &str,
        data: &Value,
        credential: &BTreeMap<String, Zeroizing<String>>,
    ) -> Result<String, PluginError>;
    /// Permently delete secret
    fn destroy(
        &self,
        path: &str,
        credential: &BTreeMap<String, Zeroizing<String>>,
    ) -> Result<bool, PluginError>;
    /// Soft delete secret
    fn delete(
        &self,
        path: &str,
        credential: &BTreeMap<String, Zeroizing<String>>,
    ) -> Result<bool, PluginError>;
    /// Restore soft deleted secret
    fn restore(
        &self,
        path: &str,
        credential: &BTreeMap<String, Zeroizing<String>>,
    ) -> Result<bool, PluginError>;
    fn capabilities(
        &self,
        credential: &BTreeMap<String, Zeroizing<String>>,
    ) -> Result<Vec<Capability>, PluginError>;
    fn describe(
        &self,
        path: &str,
        credential: &BTreeMap<String, Zeroizing<String>>,
    ) -> Result<DescribeOutput, PluginError>;
}

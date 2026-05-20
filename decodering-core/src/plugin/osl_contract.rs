use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Hash)]
pub enum Capability {
    #[serde(rename = "kv.read")]
    KvRead,
    #[serde(rename = "kv.write")]
    KvWrite,
    #[serde(rename = "kv.destroy")]
    KvDestroy,
    #[serde(rename = "kv.soft.delete")]
    KvSoftDelete,
    #[serde(rename = "kv.taint")]
    KvTaint,
    #[serde(rename = "kv.versioning")]
    KvVersioning,
    #[serde(rename = "kv.restore")]
    KvRestore,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadInput {
    pub secret_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadResponse {
    pub version: String,
    pub data: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WriteInput {
    pub path: String,
    /// Arbitrary secret payload (any JSON value)
    #[schemars(with = "serde_json::Value")]
    pub data: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WriteOutput {
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DeleteInput {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DeleteOutput {
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DestroyInput {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DestroyOutput {
    pub destroyed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RestoreInput {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RestoreOutput {
    pub restored: bool,
}

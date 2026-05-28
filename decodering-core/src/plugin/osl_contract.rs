use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Hash)]
pub enum Capability {
    Read,
    Write,
    Destroy,
    SoftDelete,
    Taint,
    Versioning,
    Restore,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[schemars(title = "SecretStatus")]
pub enum SecretStatus {
    Present,
    SoftDeleted,
    Destroyed,
    Disabled,
    NotFound,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadInput {
    pub secret_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadOutput {
    pub status: SecretStatus,
    pub version: String,
    pub data: Option<serde_json::Value>,
    /// RFC3339 deletion timestamp, present only when soft-deleted.
    pub deletion_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WriteInput {
    pub path: String,
    /// Arbitrary secret payload (any JSON value)
    #[schemars(with = "serde_json::Value")]
    pub data: Value,
    pub idempotency_token: String,
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(title = "VersionInfo")]
pub struct VersionInfo {
    /// Opaque, provider-defined version identifier. Vault: "3".
    /// AWS: a `VersionId` UUID. Azure: the version GUID in the id URL.
    pub id: String,
    pub status: SecretStatus,
    /// RFC3339. Null when the provider doesn't expose it for this version.
    pub created_time: Option<String>,
    pub deletion_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DescribeInput {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DescribeOutput {
    pub secret_name: String,
    pub provider: String,
    pub exists: bool,
    /// Opaque identifier of the current/active version.
    pub current_version: Option<String>,
    pub current_status: SecretStatus,
    pub created_time: Option<String>,
    pub updated_time: Option<String>,
    pub versions: Vec<VersionInfo>,
    /// User-defined tags/metadata, when the provider supports them.
    pub custom_metadata: Option<Value>,
    /// Opaque, provider-native details. Callers must not assume any schema.
    pub provider_hints: Value,
}

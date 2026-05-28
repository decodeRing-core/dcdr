// Example code that deserializes and serializes the model.
// extern crate serde;
// #[macro_use]
// extern crate serde_derive;
// extern crate serde_json;
//
// use generated_module::Capability;
//
// fn main() {
//     let json = r#"{"answer": 42}"#;
//     let model: Capability = serde_json::from_str(&json).unwrap();
// }

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub enum Capability {
    Destroy,

    Read,

    Restore,

    #[serde(rename = "SoftDelete")]
    SoftDelete,

    Taint,

    Versioning,

    Write,
}

#[derive(Serialize, Deserialize)]
pub struct DeleteInput {
    pub path: String,
}

#[derive(Serialize, Deserialize)]
pub struct DeleteOutput {
    pub deleted: bool,
}

#[derive(Serialize, Deserialize)]
pub struct DescribeInput {
    pub path: String,
}

#[derive(Serialize, Deserialize)]
pub struct DescribeOutput {
    pub created_time: Option<String>,

    pub current_status: SecretStatus,

    /// Opaque identifier of the current/active version.
    pub current_version: Option<String>,

    /// User-defined tags/metadata, when the provider supports them.
    pub custom_metadata: Option<serde_json::Value>,

    pub exists: bool,

    pub provider: String,

    /// Opaque, provider-native details. Callers must not assume any schema.
    pub provider_hints: Option<serde_json::Value>,

    pub secret_name: String,

    pub updated_time: Option<String>,

    pub versions: Vec<VersionInfo>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretStatus {
    Destroyed,

    Disabled,

    #[serde(rename = "not_found")]
    NotFound,

    Present,

    #[serde(rename = "soft_deleted")]
    SoftDeleted,
}

#[derive(Serialize, Deserialize)]
pub struct VersionInfo {
    /// RFC3339. Null when the provider doesn't expose it for this version.
    pub created_time: Option<String>,

    pub deletion_time: Option<String>,

    /// Opaque, provider-defined version identifier. Vault: "3".
    /// AWS: a `VersionId` UUID. Azure: the version GUID in the id URL.
    pub id: String,

    pub status: SecretStatus,
}

#[derive(Serialize, Deserialize)]
pub struct DestroyInput {
    pub path: String,
}

#[derive(Serialize, Deserialize)]
pub struct DestroyOutput {
    pub destroyed: bool,
}

#[derive(Serialize, Deserialize)]
pub struct ReadInput {
    pub secret_name: String,

    pub version: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ReadOutput {
    pub data: Option<serde_json::Value>,

    /// RFC3339 deletion timestamp, present only when soft-deleted.
    pub deletion_time: Option<String>,

    pub status: SecretStatus,

    pub version: String,
}

#[derive(Serialize, Deserialize)]
pub struct RestoreInput {
    pub path: String,
}

#[derive(Serialize, Deserialize)]
pub struct RestoreOutput {
    pub restored: bool,
}

#[derive(Serialize, Deserialize)]
pub struct WriteInput {
    /// Arbitrary secret payload (any JSON value)
    pub data: Option<serde_json::Value>,

    pub idempotency_token: String,

    pub path: String,
}

#[derive(Serialize, Deserialize)]
pub struct WriteOutput {
    pub version: String,
}

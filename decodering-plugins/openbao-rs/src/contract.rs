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
    #[serde(rename = "kv.destroy")]
    KvDestroy,

    #[serde(rename = "kv.read")]
    KvRead,

    #[serde(rename = "kv.restore")]
    KvRestore,

    #[serde(rename = "kv.soft.delete")]
    KvSoftDelete,

    #[serde(rename = "kv.taint")]
    KvTaint,

    #[serde(rename = "kv.versioning")]
    KvVersioning,

    #[serde(rename = "kv.write")]
    KvWrite,
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
pub struct ReadResponse {
    pub data: Option<serde_json::Value>,

    /// RFC3339 deletion timestamp, present only when soft-deleted.
    pub deletion_time: Option<String>,

    pub status: Status,

    pub version: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Destroyed,

    #[serde(rename = "not_found")]
    NotFound,

    Present,

    #[serde(rename = "soft_deleted")]
    SoftDeleted,
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

    pub path: String,
}

#[derive(Serialize, Deserialize)]
pub struct WriteOutput {
    pub version: String,
}

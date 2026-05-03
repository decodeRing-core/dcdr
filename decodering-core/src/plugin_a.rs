// use extism::convert::Json;
// use extism::{Manifest, Plugin};
// use serde::{Deserialize, Serialize};
// use serde_json::Value;
// use std::collections::HashMap;
// use std::fs;
// use std::path::Path;

// #[derive(Serialize)]
// struct ReadSecretInput<'a> {
//     secret_name: &'a str,
//     version: Option<u64>,
// }

// #[derive(Serialize)]
// pub struct WriteSecretInput<'a> {
//     pub path: &'a str,
//     pub data: &'a serde_json::Value,
// }

// #[derive(Deserialize, Debug)]
// pub struct Response {
//     pub version: u64,
//     pub data: Value,
// }

// #[derive(Clone)]
// pub struct Orchestrator {
//     manifests: HashMap<String, Manifest>,
// }

// #[derive(Deserialize)]
// pub struct WriteSecretOutput {
//     pub version: u64,
// }

// impl Orchestrator {
//     pub fn new() -> Self {
//         Orchestrator {
//             manifests: HashMap::new(),
//         }
//     }

//     pub fn load_plugins_from_dir(
//         &mut self,
//         plugins_root: &str,
//     ) -> Result<(), Box<dyn std::error::Error>> {
//         let manifests_dir = Path::new(plugins_root).join("manifests");

//         let entries = fs::read_dir(manifests_dir)?;

//         for entry_result in entries {
//             let entry = match entry_result {
//                 Ok(e) => e,
//                 Err(e) => {
//                     eprintln!("Warning: Failed to read a directory entry: {}", e);
//                     continue;
//                 }
//             };

//             let path = entry.path();

//             if !path.is_file() {
//                 continue;
//             }

//             let ext = match path.extension() {
//                 Some(e) => e,
//                 None => continue,
//             };

//             if ext != "yaml" {
//                 continue;
//             }

//             let stem = match path.file_stem() {
//                 Some(s) => s,
//                 None => {
//                     eprintln!("Warning: Could not extract file stem for {:?}", path);
//                     continue;
//                 }
//             };

//             let backend_name = match stem.to_str() {
//                 Some(s) => s.to_string(),
//                 None => {
//                     eprintln!("Warning: Invalid UTF-8 characters in filename {:?}", path);
//                     continue;
//                 }
//             };

//             tracing::info!("Loading Plugin Manifest: {}", backend_name.to_uppercase());

//             let yaml_data = match fs::read_to_string(&path) {
//                 Ok(data) => data,
//                 Err(e) => {
//                     eprintln!("Error reading manifest {:?}: {}", path, e);
//                     continue;
//                 }
//             };

//             let manifest: Manifest = match serde_yaml::from_str(&yaml_data) {
//                 Ok(m) => m,
//                 Err(e) => {
//                     eprintln!("Invalid YAML format in {:?}: {}", path, e);
//                     continue;
//                 }
//             };

//             self.manifests.insert(backend_name, manifest);
//         }
//         Ok(())
//     }

//     pub fn get_secret(
//         &self,
//         backend: &str,
//         secret_name: &str,
//         version: Option<u64>,
//     ) -> Result<Response, Box<dyn std::error::Error>> {
//         let manifest = self
//             .manifests
//             .get(backend)
//             .ok_or_else(|| -> Box<dyn std::error::Error> { "Invalid backend".into() })?;

//         let mut plugin = match Plugin::new(manifest, [], true) {
//             Ok(p) => p,
//             Err(e) => {
//                 eprintln!("Failed to instantiate Wasm plugin: {}", e);
//                 return Err(format!("Failed to instantiate ephemeral Wasm sandbox: {}", e).into());
//             }
//         };

//         let request_payload = ReadSecretInput {
//             secret_name,
//             version,
//         };

//         match plugin
//             .call::<Json<ReadSecretInput>, Json<Response>>("get_secret", Json(request_payload))
//         {
//             Ok(res) => Ok(res.0),
//             Err(e) => Err(e.to_string().into()),
//         }
//     }

//     pub fn put_secret(
//         &self,
//         backend: &str,
//         path: &str,
//         data: &Value,
//     ) -> Result<String, Box<dyn std::error::Error>> {
//         let manifest = self
//             .manifests
//             .get(backend)
//             .ok_or_else(|| -> Box<dyn std::error::Error> { "Invalid backend".into() })?;

//         let mut plugin = match Plugin::new(manifest, [], true) {
//             Ok(p) => p,
//             Err(e) => {
//                 eprintln!("Failed to instantiate Wasm plugin: {}", e);
//                 return Err(format!("Failed to instantiate ephemeral Wasm sandbox: {}", e).into());
//             }
//         };

//         tracing::debug!("Preparing to call WASM plugin...");
//         let request_payload = WriteSecretInput {
//             path: path,
//             data: data,
//         };

//         match plugin.call::<Json<WriteSecretInput>, Json<WriteSecretOutput>>(
//             "put_secret",
//             Json(request_payload),
//         ) {
//             Ok(res) => Ok(res.0.version.to_string()),
//             Err(e) => Err(e.to_string().into()),
//         }
//     }
// }

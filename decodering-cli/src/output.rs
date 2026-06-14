#![allow(clippy::print_stdout)]
use crate::api::ApiResponse;

pub fn report(resp: &ApiResponse<serde_json::Value>) {
    if !resp.message.is_empty() {
        println!("{}", resp.message);
    }
    if let Some(data) = &resp.data {
        match serde_json::to_string_pretty(data) {
            Ok(s) => println!("{s}"),
            Err(_) => println!("{data}"),
        }
    }
}

use std::time::{SystemTime, UNIX_EPOCH};

pub mod action;
pub mod actions;
pub mod audit;
pub mod error;
pub mod plugin;
pub mod request;
pub mod response;
pub mod runner;

pub fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

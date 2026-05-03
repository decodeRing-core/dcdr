use std::time::{SystemTime, UNIX_EPOCH};

pub mod action;
pub mod actions;
pub mod audit;
pub mod domain;
pub mod error;
pub mod plugin;
pub mod repository;
pub mod request;
pub mod response;
pub mod runner;
pub mod tx;

pub fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

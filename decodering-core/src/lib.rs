use std::fmt::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

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

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut s = String::with_capacity(digest.len() * 2);
    for b in digest {
        write!(&mut s, "{:02x}", b).unwrap();
    }
    s
}

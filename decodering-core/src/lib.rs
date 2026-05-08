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

#[allow(clippy::expect_used)]
pub fn now_ts_plus(secs: i64) -> i64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System clock is before UNIX_EPOCH; refusing to operate with broken clock")
        .as_secs();
    i64::try_from(now).expect("System time exceeds i64::MAX seconds since epoch") + secs
}

pub fn now_ts() -> i64 {
    now_ts_plus(0)
}

#[allow(clippy::expect_used)]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut s = String::with_capacity(digest.len() * 2);
    for b in digest {
        write!(&mut s, "{b:02x}").expect("writing to String never fails");
    }
    s
}

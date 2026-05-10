use std::time::{SystemTime, UNIX_EPOCH};

pub const CHALLENGE_TTL_SECS: i64 = 60;

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

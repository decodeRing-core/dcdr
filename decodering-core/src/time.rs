use std::time::{SystemTime, UNIX_EPOCH};

pub const CHALLENGE_TTL_SECS: i64 = 300;

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

#[cfg(test)]
mod tests {
    use crate::time::{CHALLENGE_TTL_SECS, now_ts, now_ts_plus};

    #[test]
    fn now_ts_plus_offsets_correctly() {
        let base = now_ts();
        let plus = now_ts_plus(CHALLENGE_TTL_SECS);
        let diff = plus - base;
        assert!((CHALLENGE_TTL_SECS..=CHALLENGE_TTL_SECS + 2).contains(&diff));
    }

    #[test]
    fn now_ts_plus_accepts_negative() {
        let base = now_ts();
        let past = now_ts_plus(-60);
        assert!(past <= base);
    }
}

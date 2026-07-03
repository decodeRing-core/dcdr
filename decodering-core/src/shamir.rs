use std::fmt;

use blahaj::{Share, Sharks};
use rand::{Rng, rngs::ThreadRng};
use sha2::{Digest, Sha256};

pub struct ShamirInit {
    pub shards: Vec<Share>,
    pub master_key: Vec<u8>,
    _threshold: u8,
    pub hash: Vec<u8>,
}

pub fn initialize_shamir(n: u8, k: u8) -> Result<ShamirInit, Box<dyn std::error::Error>> {
    if n > 10 {
        return Err(format!("Invalid number of shards: {n}").into());
    }

    if k > n || k < 2 {
        return Err(format!("Invalid treshold: {k}").into());
    }

    let mut secret = vec![0u8; 32];
    ThreadRng::default().fill_bytes(&mut secret);

    let shards: Vec<Share> = {
        let sharks = Sharks(k);
        sharks.dealer(&secret).take(n as usize).collect()
    };

    let mut hasher = Sha256::new();
    hasher.update(&secret);
    let hash = hasher.finalize().to_vec();

    Ok(ShamirInit {
        shards,
        master_key: secret,
        _threshold: k,
        hash,
    })
}

#[derive(Debug)]
pub enum LockError {
    RecoveryFailed(String),
    HashMismatch,
    InsufficientShards { got: usize, need: u8 },
}

impl fmt::Display for LockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RecoveryFailed(s) => write!(f, "recovery failed: {s}"),
            Self::HashMismatch => {
                write!(f, "reconstructed key does not match expected hash")
            }
            Self::InsufficientShards { got, need } => {
                write!(f, "need {need} shards, got {got}")
            }
        }
    }
}

impl std::error::Error for LockError {}

pub fn unlock(
    threshold: u8,
    expected_hash: &[u8],
    shards: &[Vec<u8>],
) -> Result<Vec<u8>, LockError> {
    let shares: Option<Vec<Share>> = shards
        .iter()
        .map(|b| Share::try_from(b.as_slice()))
        .collect::<Result<_, _>>()
        .ok();

    let Some(shares) = shares else {
        return Err(LockError::RecoveryFailed("Invalid shards".to_owned()));
    };

    if shares.len() < threshold as usize {
        return Err(LockError::InsufficientShards {
            got: shares.len(),
            need: threshold,
        });
    }

    let secret = Sharks(threshold)
        .recover(shares.as_slice())
        .map_err(|e| LockError::RecoveryFailed(e.to_owned()))?;

    if &Sha256::digest(&secret)[..] != expected_hash {
        return Err(LockError::HashMismatch);
    }

    Ok(secret)
}

#[cfg(test)]
mod tests {
    use super::{LockError, ShamirInit, initialize_shamir, unlock};

    type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // Serialize each dealer `Share` the same way the server does before handing
    // shards to operators, so `unlock` can parse them back from raw bytes.
    fn shard_bytes(init: &ShamirInit) -> Vec<Vec<u8>> {
        init.shards.iter().map(Vec::from).collect()
    }

    #[test]
    fn unlock_with_exactly_threshold_shares_recovers_key() -> TestResult {
        let init = initialize_shamir(5, 3).map_err(|_| "init failed")?;
        let shards: Vec<Vec<u8>> = shard_bytes(&init).into_iter().take(3).collect();
        let recovered = unlock(3, &init.hash, &shards).map_err(|_| "unlock failed")?;
        assert_eq!(recovered, init.master_key);
        Ok(())
    }

    #[test]
    fn unlock_with_more_than_threshold_shares_recovers_key() -> TestResult {
        let init = initialize_shamir(5, 3).map_err(|_| "init failed")?;
        let shards: Vec<Vec<u8>> = shard_bytes(&init).into_iter().take(4).collect();
        let recovered = unlock(3, &init.hash, &shards).map_err(|_| "unlock failed")?;
        assert_eq!(recovered, init.master_key);
        Ok(())
    }

    #[test]
    fn unlock_with_too_few_shares_is_rejected() -> TestResult {
        let init = initialize_shamir(5, 3).map_err(|_| "init failed")?;
        let shards: Vec<Vec<u8>> = shard_bytes(&init).into_iter().take(2).collect();
        let result = unlock(3, &init.hash, &shards);
        assert!(matches!(
            result,
            Err(LockError::InsufficientShards { got: 2, need: 3 })
        ));
        Ok(())
    }

    #[test]
    fn unlock_with_wrong_hash_is_rejected() -> TestResult {
        let init = initialize_shamir(5, 3).map_err(|_| "init failed")?;
        let shards: Vec<Vec<u8>> = shard_bytes(&init).into_iter().take(3).collect();
        let result = unlock(3, &[0u8; 32], &shards);
        assert!(matches!(result, Err(LockError::HashMismatch)));
        Ok(())
    }

    #[test]
    fn unlock_with_unparseable_shares_is_rejected() {
        // Byte slices too short to be valid shares must fail cleanly, not panic.
        let garbage = vec![Vec::new(), Vec::new()];
        let result = unlock(2, &[0u8; 32], &garbage);
        assert!(matches!(result, Err(LockError::RecoveryFailed(_))));
    }

    #[test]
    fn initialize_rejects_invalid_parameters() {
        assert!(initialize_shamir(11, 3).is_err(), "n > 10 must be rejected");
        assert!(initialize_shamir(5, 1).is_err(), "k < 2 must be rejected");
        assert!(initialize_shamir(3, 5).is_err(), "k > n must be rejected");
    }
}

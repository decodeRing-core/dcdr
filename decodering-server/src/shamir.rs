use std::fmt;

use blahaj::{Share, Sharks};
use rand::{Rng, rngs::ThreadRng};
use sha2::{Digest, Sha256};

pub struct ShamirInit {
    pub shards: Vec<Share>,
    pub _master_key: Vec<u8>,
    pub _threshold: u8,
    pub hash: Vec<u8>,
}

pub fn initialize_shamir(n: u8, k: u8) -> Result<ShamirInit, Box<dyn std::error::Error>> {
    if n > 10 {
        return Err(format!("Invalid number of shards: {}", n).into());
    }

    if k > n || k < 2 {
        return Err(format!("Invalid treshold: {}", k).into());
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
        _master_key: secret,
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
            LockError::RecoveryFailed(s) => write!(f, "recovery failed: {}", s),
            LockError::HashMismatch => {
                write!(f, "reconstructed key does not match expected hash")
            }
            LockError::InsufficientShards { got, need } => {
                write!(f, "need {} shards, got {}", need, got)
            }
        }
    }
}

impl std::error::Error for LockError {}

pub fn unlock(
    threshold: u8,
    expected_hash: &[u8],
    shards: Vec<Share>,
) -> Result<Vec<u8>, LockError> {
    if shards.len() < threshold as usize {
        return Err(LockError::InsufficientShards {
            got: shards.len(),
            need: threshold,
        });
    }

    let secret = Sharks(threshold)
        .recover(shards.as_slice())
        .map_err(|e| LockError::RecoveryFailed(e.to_string()))?;

    if Sha256::digest(&secret).as_slice() != expected_hash {
        return Err(LockError::HashMismatch);
    }

    Ok(secret)
}

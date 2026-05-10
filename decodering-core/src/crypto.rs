use std::fmt::Write;

use sha2::{Digest, Sha256};

pub fn pem_to_der(pem_str: &str) -> Result<Vec<u8>, pem::PemError> {
    let parsed = pem::parse(pem_str)?;
    Ok(parsed.contents().to_vec())
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

pub fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        let hi = HEX.get((b >> 4) as usize).copied().unwrap_or(b'0');
        let lo = HEX.get((b & 0xf) as usize).copied().unwrap_or(b'0');
        out.push(hi as char);
        out.push(lo as char);
    }
    out
}

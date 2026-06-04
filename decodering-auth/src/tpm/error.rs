#[derive(Debug)]
pub enum TpmVerifyError {
    InvalidAkPubkey,
    InvalidSignature,
    SignatureVerificationFailed,
    UnsupportedSigAlg,
    InvalidQuote,
    NonceMismatch,
    PcrMismatch,
    InvalidPcrSelection,
    InvalidSize,
}

impl std::fmt::Display for TpmVerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidAkPubkey => write!(f, "Invalid AK public key"),
            Self::InvalidSignature => write!(f, "Invalid signature encoding"),
            Self::SignatureVerificationFailed => write!(f, "Signature verification failed"),
            Self::UnsupportedSigAlg => write!(f, "Unsupported signature algorithm"),
            Self::InvalidQuote => write!(f, "Invalid quote structure"),
            Self::NonceMismatch => write!(f, "Nonce mismatch"),
            Self::PcrMismatch => write!(f, "PCR mismatch"),
            Self::InvalidPcrSelection => write!(f, "PCR bitmap exceeds 32 bytes"),
            Self::InvalidSize => write!(f, "Unexpect size"),
        }
    }
}

impl std::error::Error for TpmVerifyError {}

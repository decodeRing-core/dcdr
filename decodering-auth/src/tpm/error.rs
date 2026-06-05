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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_messages() {
        assert_eq!(
            TpmVerifyError::InvalidAkPubkey.to_string(),
            "Invalid AK public key"
        );
        assert_eq!(
            TpmVerifyError::InvalidSignature.to_string(),
            "Invalid signature encoding"
        );
        assert_eq!(
            TpmVerifyError::SignatureVerificationFailed.to_string(),
            "Signature verification failed"
        );
        assert_eq!(
            TpmVerifyError::UnsupportedSigAlg.to_string(),
            "Unsupported signature algorithm"
        );
        assert_eq!(
            TpmVerifyError::InvalidQuote.to_string(),
            "Invalid quote structure"
        );
        assert_eq!(TpmVerifyError::NonceMismatch.to_string(), "Nonce mismatch");
        assert_eq!(TpmVerifyError::PcrMismatch.to_string(), "PCR mismatch");
        assert_eq!(
            TpmVerifyError::InvalidPcrSelection.to_string(),
            "PCR bitmap exceeds 32 bytes"
        );
        assert_eq!(TpmVerifyError::InvalidSize.to_string(), "Unexpect size");
    }

    #[test]
    fn debug_format() {
        assert_eq!(format!("{:?}", TpmVerifyError::PcrMismatch), "PcrMismatch");
    }

    #[test]
    fn is_std_error() {
        let err = TpmVerifyError::InvalidQuote;
        let dyn_err: &dyn std::error::Error = &err;
        assert!(dyn_err.source().is_none());
        assert_eq!(dyn_err.to_string(), "Invalid quote structure");
    }
}

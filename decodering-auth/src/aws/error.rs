#[derive(Debug)]
pub enum AwsError {
    InvalidStsUrl,
    InvalidStsAction,
    InvalidInput,
    StsUnreachable,
    StsRejected,
    StsUnavailableClient,
}

impl std::fmt::Display for AwsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidStsUrl => write!(f, "Invalid STS url"),
            Self::InvalidStsAction => write!(f, "Invalid STS action"),
            Self::InvalidInput => write!(f, "Invalid input"),
            Self::StsUnreachable => write!(f, "STS unreachable"),
            Self::StsRejected => write!(f, "STS rejected"),
            Self::StsUnavailableClient => write!(f, "Unavailable client"),
        }
    }
}

impl std::error::Error for AwsError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_messages() {
        assert_eq!(AwsError::InvalidStsUrl.to_string(), "Invalid STS url");
        assert_eq!(AwsError::InvalidStsAction.to_string(), "Invalid STS action");
        assert_eq!(AwsError::InvalidInput.to_string(), "Invalid input");
        assert_eq!(AwsError::StsUnreachable.to_string(), "STS unreachable");
        assert_eq!(AwsError::StsRejected.to_string(), "STS rejected");
        assert_eq!(
            AwsError::StsUnavailableClient.to_string(),
            "Unavailable client"
        );
    }

    #[test]
    fn debug_format() {
        assert_eq!(format!("{:?}", AwsError::InvalidStsUrl), "InvalidStsUrl");
    }

    #[test]
    fn is_std_error() {
        // Confirms the Error impl is wired up (and source() defaults to None).
        let err = AwsError::StsRejected;
        let dyn_err: &dyn std::error::Error = &err;
        assert!(dyn_err.source().is_none());
        assert_eq!(dyn_err.to_string(), "STS rejected");
    }
}

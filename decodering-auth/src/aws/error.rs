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

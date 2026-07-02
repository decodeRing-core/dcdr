use std::str::FromStr;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug)]
pub struct UnknownPrincipalStatus(pub String);

impl std::fmt::Display for UnknownPrincipalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown principal status: {}", self.0)
    }
}

#[derive(Debug)]
pub struct UnknownPrincipalKind(pub String);

impl std::fmt::Display for UnknownPrincipalKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown principal kind: {}", self.0)
    }
}

#[derive(Debug)]
pub struct UnknownPrincipalCredentialKind(pub String);

impl std::fmt::Display for UnknownPrincipalCredentialKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown principal credential kind: {}", self.0)
    }
}

impl std::error::Error for UnknownPrincipalKind {}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum PrincipalKind {
    Human,
    Machine,
    Service,
}

impl FromStr for PrincipalKind {
    type Err = UnknownPrincipalKind;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "human" => Self::Human,
            "machine" => Self::Machine,
            "service" => Self::Service,
            other => return Err(UnknownPrincipalKind(other.to_owned())),
        })
    }
}

impl PrincipalKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Machine => "machine",
            Self::Service => "service",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PrincipalStatus {
    Active,
    Disabled,
    Deleted,
    Pending,
}

impl FromStr for PrincipalStatus {
    type Err = UnknownPrincipalStatus;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "active" => Self::Active,
            "disabled" => Self::Disabled,
            "deleted" => Self::Deleted,
            "pending" => Self::Pending,
            other => return Err(UnknownPrincipalStatus(other.to_owned())),
        })
    }
}

impl PrincipalStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Disabled => "disabled",
            Self::Deleted => "deleted",
            Self::Pending => "pending",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum PrincipalCredentialKind {
    ApiKey,
    TrustedPlatformModule,
    AwsIdentity,
}

impl FromStr for PrincipalCredentialKind {
    type Err = UnknownPrincipalCredentialKind;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "apiKey" => Self::ApiKey,
            "trustedPlatformModule" => Self::TrustedPlatformModule,
            "awsIdentity" => Self::AwsIdentity,
            other => return Err(UnknownPrincipalCredentialKind(other.to_owned())),
        })
    }
}

impl PrincipalCredentialKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ApiKey => "apiKey",
            Self::TrustedPlatformModule => "trustedPlatformModule",
            Self::AwsIdentity => "awsIdentity",
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub enum AuditOutcome {
    Allowed,
    Denied,
    Error,
}

impl AuditOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Allowed => "allowed",
            Self::Denied => "denied",
            Self::Error => "error",
        }
    }
}

impl FromStr for AuditOutcome {
    type Err = UnknownPrincipalStatus;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "allowed" => Self::Allowed,
            "denied" => Self::Denied,
            "error" => Self::Error,
            other => return Err(UnknownPrincipalStatus(other.to_owned())),
        })
    }
}

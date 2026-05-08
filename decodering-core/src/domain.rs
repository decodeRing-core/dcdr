use std::str::FromStr;

use serde::{Deserialize, Serialize};

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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
            "human" => PrincipalKind::Human,
            "machine" => PrincipalKind::Machine,
            "service" => PrincipalKind::Service,
            other => return Err(UnknownPrincipalKind(other.to_owned())),
        })
    }
}

impl PrincipalKind {
    pub fn as_str(self) -> &'static str {
        match self {
            PrincipalKind::Human => "human",
            PrincipalKind::Machine => "machine",
            PrincipalKind::Service => "service",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PrincipalStatus {
    Active,
    Disabled,
    Deleted,
}

impl FromStr for PrincipalStatus {
    type Err = UnknownPrincipalStatus;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "active" => PrincipalStatus::Active,
            "disabled" => PrincipalStatus::Disabled,
            "deleted" => PrincipalStatus::Deleted,
            other => return Err(UnknownPrincipalStatus(other.to_owned())),
        })
    }
}

impl PrincipalStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            PrincipalStatus::Active => "active",
            PrincipalStatus::Disabled => "disabled",
            PrincipalStatus::Deleted => "deleted",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PrincipalCredentialKind {
    ApiKey,
    VirtualTrustedPlatformModule,
    AwsIdentity,
}

impl FromStr for PrincipalCredentialKind {
    type Err = UnknownPrincipalCredentialKind;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "api_key" => PrincipalCredentialKind::ApiKey,
            "vtpm" => PrincipalCredentialKind::VirtualTrustedPlatformModule,
            "aws_iam" => PrincipalCredentialKind::AwsIdentity,
            other => return Err(UnknownPrincipalCredentialKind(other.to_owned())),
        })
    }
}

impl PrincipalCredentialKind {
    pub fn as_str(self) -> &'static str {
        match self {
            PrincipalCredentialKind::ApiKey => "api_key",
            PrincipalCredentialKind::VirtualTrustedPlatformModule => "vtpm",
            PrincipalCredentialKind::AwsIdentity => "aws_iam",
        }
    }
}

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

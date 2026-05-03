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

impl std::error::Error for UnknownPrincipalKind {}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PrincipalKind {
    Human,
    Machine,
    Service,
}

impl PrincipalKind {
    pub fn from_str(s: &str) -> Result<Self, UnknownPrincipalKind> {
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PrincipalStatus {
    Active,
    Disabled,
    Deleted,
}

impl PrincipalStatus {
    pub fn from_str(s: &str) -> Result<Self, UnknownPrincipalStatus> {
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

use sqlx::error::BoxDynError;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PrincipalKind {
    Human,
    Machine,
    Service,
}

impl PrincipalKind {
    pub fn as_str(self) -> &'static str {
        match self {
            PrincipalKind::Human => "human",
            PrincipalKind::Machine => "machine",
            PrincipalKind::Service => "service",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, BoxDynError> {
        Ok(match s {
            "human" => PrincipalKind::Human,
            "machine" => PrincipalKind::Machine,
            "service" => PrincipalKind::Service,
            other => return Err(format!("unknown principal kind: {other}").into()),
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PrincipalStatus {
    Active,
    Disabled,
    Deleted,
}

impl PrincipalStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            PrincipalStatus::Active => "active",
            PrincipalStatus::Disabled => "disabled",
            PrincipalStatus::Deleted => "deleted",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, BoxDynError> {
        Ok(match s {
            "active" => PrincipalStatus::Active,
            "disabled" => PrincipalStatus::Disabled,
            "deleted" => PrincipalStatus::Deleted,
            other => return Err(format!("unknown principal status: {other}").into()),
        })
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

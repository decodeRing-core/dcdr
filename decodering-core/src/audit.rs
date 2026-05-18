use serde::{Deserialize, Serialize};

use crate::domain::AuditOutcome;
use crate::error::{DenyReason, ExecutionError};
use crate::repository::AuditEntry;

pub enum Actor {
    User { user_id: i64 },
    Principal { principal_id: String },
    None,
}

#[derive(Clone, Debug)]
pub enum Target {
    App(String),
    User(i64),
    ApiKey(i64),
    SecretMapping(String, String),
    ShamirConfiguration(i64),
    Principal(String),
    PrincipalCredential(String),
    PrincipalAppGrant(Option<String>),
    PrincipalToken(String),
    TpmChallenge(String),
    AuditEntry(i64),
}

impl Target {
    pub fn kind_str(&self) -> &'static str {
        match self {
            Self::App(_) => "app",
            Self::User(_) => "user",
            Self::ApiKey(_) => "api_key",
            Self::SecretMapping(_, _) => "secret_mapping",
            Self::ShamirConfiguration(_) => "shamir_configuration",
            Self::AuditEntry(_) => "audit_entry",
            Self::Principal(_) => "principal",
            Self::PrincipalCredential(_) => "principal_credential",
            Self::PrincipalToken(_) => "principal_token",
            Self::TpmChallenge(_) => "tpm_challenge",
            Self::PrincipalAppGrant(_) => "principal_app_grant",
        }
    }

    pub fn id_str(&self) -> String {
        match self {
            Self::App(id) => id.clone(),
            Self::User(user_id) => user_id.to_string(),
            Self::ApiKey(api_key_id) => api_key_id.to_string(),
            Self::SecretMapping(app_id, name) => format!("{app_id}:{name}"),
            Self::ShamirConfiguration(shamir_id) => shamir_id.to_string(),
            Self::AuditEntry(id) => id.to_string(),
            Self::Principal(principal_id) => principal_id.to_owned(),
            Self::PrincipalCredential(principal_credential_id) => {
                principal_credential_id.to_owned()
            }
            Self::PrincipalToken(principal_token_id) => principal_token_id.to_owned(),
            Self::TpmChallenge(tpm_challenge_id) => tpm_challenge_id.to_owned(),
            Self::PrincipalAppGrant(principal_app_grant) => principal_app_grant
                .as_deref()
                .unwrap_or_default()
                .to_owned(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ActionKind {
    AppCreate,
    UserCreate,
    ApiKeyCreate,
    PrincipalCreate,
    PrincipalCredentialCreate,
    PrincipalCredentialLastUsedUpdate,
    PrincipalTokenCreate,
    PrincipalAppGrantCreate,
    PrincipalAppGrantDelete,
    SecretMappingCreate,
    SecretMappingDelete,
    SecretMappingGet,
    ShamirConfigurationCreate,
    AppUserCreate,
    TpmChallengeCreate,
    TpmChallengeConsume,
    SystemInit,
}

impl ActionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AppCreate => "app.create",
            Self::UserCreate => "user.create",
            Self::ApiKeyCreate => "api_key.create",
            Self::PrincipalCreate => "principal.create",
            Self::SecretMappingCreate => "secret_mapping.create",
            Self::SecretMappingDelete => "secret_mapping.delete",
            Self::ShamirConfigurationCreate => "shamir_configuration.create",
            Self::SystemInit => "system.init",
            Self::SecretMappingGet => "secret_mapping.get",
            Self::PrincipalCredentialCreate => "principal_credential.create",
            Self::PrincipalTokenCreate => "principal_token.create",
            Self::AppUserCreate => "app_user.create",
            Self::TpmChallengeCreate => "tpm_challenge.create",
            Self::TpmChallengeConsume => "tpm_challenge.consume",
            Self::PrincipalAppGrantCreate => "principal_app_grant.create",
            Self::PrincipalAppGrantDelete => "principal_app_grant.delete",
            Self::PrincipalCredentialLastUsedUpdate => "principal_credential.update",
        }
    }
}

pub struct AuditDescriptor {
    pub actor: Actor,
    pub action_kind: ActionKind,
    pub revertible: bool,
    pub undoes: Option<i64>,
    pub metadata: Option<serde_json::Value>,
}

impl AuditDescriptor {
    fn metadata_json(&self) -> Option<String> {
        self.metadata.as_ref().map(ToString::to_string)
    }
}

pub fn audit_allowed<O: AuditCapture>(
    descriptor: &AuditDescriptor,
    raft_index: i64,
    output: &O,
    timestamp: i64,
) -> AuditEntry {
    let (user_id, principal_id) = split_actor(&descriptor.actor);
    let (target_type, target_id) = output.target().map_or((None, None), |t| {
        (Some(t.kind_str().to_owned()), Some(t.id_str()))
    });
    AuditEntry {
        raft_index,
        timestamp,
        user_id,
        principal_id,
        action_type: descriptor.action_kind.as_str().to_owned(),
        target_type,
        target_id,
        outcome: AuditOutcome::Allowed,
        reason: None,
        before_state: output.before_state(),
        after_state: output.after_state(),
        metadata: descriptor.metadata_json(),
        revertible: descriptor.revertible,
        undoes: descriptor.undoes,
    }
}

pub fn audit_denied(
    descriptor: &AuditDescriptor,
    raft_index: i64,
    reason: &DenyReason,
    timestamp: i64,
) -> AuditEntry {
    let (user_id, principal_id) = split_actor(&descriptor.actor);

    AuditEntry {
        raft_index,
        timestamp,
        user_id,
        principal_id,
        action_type: descriptor.action_kind.as_str().to_owned(),
        target_type: None,
        target_id: None,
        outcome: AuditOutcome::Denied,
        reason: Some(reason.to_string()),
        before_state: None,
        after_state: None,
        metadata: descriptor.metadata_json(),
        revertible: false, // denied actions are never revertible
        undoes: None,
    }
}

pub fn audit_errored(
    descriptor: &AuditDescriptor,
    raft_index: i64,
    err: &ExecutionError,
    timestamp: i64,
) -> AuditEntry {
    let (user_id, principal_id) = split_actor(&descriptor.actor);

    AuditEntry {
        raft_index,
        timestamp,
        user_id,
        principal_id,
        action_type: descriptor.action_kind.as_str().to_owned(),
        target_type: None,
        target_id: None,
        outcome: AuditOutcome::Error,
        reason: Some(err.to_string()),
        before_state: None,
        after_state: None,
        metadata: descriptor.metadata_json(),
        revertible: false, // errored actions are never revertible
        undoes: None,
    }
}

fn split_actor(actor: &Actor) -> (Option<i64>, Option<String>) {
    match actor {
        Actor::User { user_id } => (Some(*user_id), None),
        Actor::Principal { principal_id } => (None, Some(principal_id.clone())),
        Actor::None => (None, None),
    }
}

pub trait AuditCapture {
    fn before_state(&self) -> Option<String>;
    fn after_state(&self) -> Option<String>;
    fn target(&self) -> Option<Target>;
}

pub struct ActionOutput<R> {
    pub response: R,
    pub before_state: Option<serde_json::Value>,
    pub after_state: Option<serde_json::Value>,
    pub target: Option<Target>,
}

impl<R> AuditCapture for ActionOutput<R> {
    fn before_state(&self) -> Option<String> {
        self.before_state.as_ref().map(ToString::to_string)
    }
    fn after_state(&self) -> Option<String> {
        self.after_state.as_ref().map(ToString::to_string)
    }
    fn target(&self) -> Option<Target> {
        self.target.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct StubCapture {
        before: Option<String>,
        after: Option<String>,
        target: Option<Target>,
    }

    impl AuditCapture for StubCapture {
        fn before_state(&self) -> Option<String> {
            self.before.clone()
        }
        fn after_state(&self) -> Option<String> {
            self.after.clone()
        }
        fn target(&self) -> Option<Target> {
            self.target.clone()
        }
    }

    fn descriptor(actor: Actor) -> AuditDescriptor {
        AuditDescriptor {
            actor,
            action_kind: ActionKind::AppCreate,
            revertible: true,
            undoes: Some(42),
            metadata: Some(json!({"k": "v"})),
        }
    }

    #[test]
    fn id_str_secret_mapping_uses_colon_separator() {
        let t = Target::SecretMapping("app-1".to_owned(), "DB_PASSWORD".to_owned());
        assert_eq!(t.id_str(), "app-1:DB_PASSWORD");
    }

    #[test]
    fn id_str_principal_app_grant_some() {
        let t = Target::PrincipalAppGrant(Some("grant-xyz".to_owned()));
        assert_eq!(t.id_str(), "grant-xyz");
    }

    #[test]
    fn id_str_principal_app_grant_none_is_empty() {
        let t = Target::PrincipalAppGrant(None);
        assert_eq!(t.id_str(), "");
    }

    #[test]
    fn audit_allowed_populates_target_and_state_from_capture() {
        let desc = descriptor(Actor::User { user_id: 7 });
        let capture = StubCapture {
            before: Some("{\"x\":1}".to_owned()),
            after: Some("{\"x\":2}".to_owned()),
            target: Some(Target::App("app-1".to_owned())),
        };

        let entry = audit_allowed(&desc, 100, &capture, 1_700_000_000);

        assert_eq!(entry.raft_index, 100);
        assert_eq!(entry.timestamp, 1_700_000_000);
        assert_eq!(entry.user_id, Some(7));
        assert_eq!(entry.principal_id, None);
        assert_eq!(entry.action_type, "app.create");
        assert_eq!(entry.target_type.as_deref(), Some("app"));
        assert_eq!(entry.target_id.as_deref(), Some("app-1"));
        assert!(matches!(entry.outcome, AuditOutcome::Allowed));
        assert_eq!(entry.reason, None);
        assert_eq!(entry.before_state.as_deref(), Some("{\"x\":1}"));
        assert_eq!(entry.after_state.as_deref(), Some("{\"x\":2}"));
        assert!(entry.revertible);
        assert_eq!(entry.undoes, Some(42));
    }

    #[test]
    fn audit_allowed_with_no_target_yields_none_target_fields() {
        let desc = descriptor(Actor::Principal {
            principal_id: "p-1".to_owned(),
        });
        let capture = StubCapture {
            before: None,
            after: None,
            target: None,
        };

        let entry = audit_allowed(&desc, 1, &capture, 0);

        assert_eq!(entry.user_id, None);
        assert_eq!(entry.principal_id.as_deref(), Some("p-1"));
        assert_eq!(entry.target_type, None);
        assert_eq!(entry.target_id, None);
        assert_eq!(entry.before_state, None);
        assert_eq!(entry.after_state, None);
    }

    #[test]
    fn audit_denied_forces_non_revertible_and_strips_target_and_state() {
        let desc = AuditDescriptor {
            actor: Actor::User { user_id: 3 },
            action_kind: ActionKind::SecretMappingGet,
            revertible: true,
            undoes: Some(99),
            metadata: Some(json!({"reason": "policy"})),
        };
        let reason = DenyReason("denied".to_owned());

        let entry = audit_denied(&desc, 10, &reason, 1);

        assert!(matches!(entry.outcome, AuditOutcome::Denied));
        assert_eq!(entry.target_type, None);
        assert_eq!(entry.target_id, None);
        assert_eq!(entry.before_state, None);
        assert_eq!(entry.after_state, None);
        assert!(!entry.revertible);
        assert_eq!(entry.undoes, None);
        assert_eq!(entry.reason, Some(reason.to_string()));
        assert!(entry.metadata.is_some());
    }

    #[test]
    fn audit_errored_forces_non_revertible_and_strips_target_and_state() {
        let desc = AuditDescriptor {
            actor: Actor::None,
            action_kind: ActionKind::SystemInit,
            revertible: true,
            undoes: Some(99),
            metadata: None,
        };
        let err = ExecutionError::Other("other".to_owned());

        let entry = audit_errored(&desc, 5, &err, 2);

        assert!(matches!(entry.outcome, AuditOutcome::Error));
        assert_eq!(entry.user_id, None);
        assert_eq!(entry.principal_id, None);
        assert_eq!(entry.target_type, None);
        assert_eq!(entry.target_id, None);
        assert_eq!(entry.before_state, None);
        assert_eq!(entry.after_state, None);
        assert!(!entry.revertible);
        assert_eq!(entry.undoes, None);
        assert_eq!(entry.reason, Some(err.to_string()));
    }
}

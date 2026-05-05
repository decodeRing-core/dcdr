use serde::{Deserialize, Serialize};

use crate::{
    domain::AuditOutcome,
    error::{DenyReason, ExecutionError},
    repository::AuditEntry,
};

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
    PrincipalToken(String),
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
        }
    }

    pub fn id_str(&self) -> String {
        match self {
            Self::App(id) => id.clone(),
            Self::User(id) => id.to_string(),
            Self::ApiKey(id) => id.to_string(),
            Self::SecretMapping(app_id, name) => format!("{}:{}", app_id, name),
            Self::ShamirConfiguration(id) => id.to_string(),
            Self::AuditEntry(id) => id.to_string(),
            Self::Principal(id) => id.to_string(),
            Self::PrincipalCredential(id) => id.to_string(),
            Self::PrincipalToken(id) => id.to_string(),
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
    PrincipalTokenCreate,
    SecretMappingCreate,
    SecretMappingGet,
    ShamirConfigurationCreate,
    CreateAppUser,
    SystemInit,
}

impl ActionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActionKind::AppCreate => "app.create",
            ActionKind::UserCreate => "user.create",
            ActionKind::ApiKeyCreate => "api_key.create",
            ActionKind::PrincipalCreate => "principal.create",
            ActionKind::SecretMappingCreate => "secret_mapping.create",
            ActionKind::ShamirConfigurationCreate => "shamir_configuration.create",
            ActionKind::SystemInit => "system.init",
            ActionKind::SecretMappingGet => "secret_mapping.get",
            ActionKind::PrincipalCredentialCreate => "principal_credential.create",
            ActionKind::PrincipalTokenCreate => "principal_token.create",
            ActionKind::CreateAppUser => "app_user.create",
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
        self.metadata.as_ref().map(|v| v.to_string())
    }
}

pub fn audit_allowed<O: AuditCapture>(
    descriptor: &AuditDescriptor,
    raft_index: i64,
    output: &O,
    timestamp: i64,
) -> AuditEntry {
    let (user_id, principal_id) = split_actor(&descriptor.actor);
    let (target_type, target_id) = match output.target() {
        Some(t) => (Some(t.kind_str().to_string()), Some(t.id_str())),
        None => (None, None),
    };
    AuditEntry {
        raft_index,
        timestamp,
        user_id,
        principal_id,
        action_type: descriptor.action_kind.as_str().to_string(),
        target_type: target_type,
        target_id: target_id,
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
    reason: DenyReason,
    timestamp: i64,
) -> AuditEntry {
    let (user_id, principal_id) = split_actor(&descriptor.actor);

    AuditEntry {
        raft_index,
        timestamp,
        user_id,
        principal_id,
        action_type: descriptor.action_kind.as_str().to_string(),
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
        action_type: descriptor.action_kind.as_str().to_string(),
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
        self.before_state.as_ref().map(|v| v.to_string())
    }
    fn after_state(&self) -> Option<String> {
        self.after_state.as_ref().map(|v| v.to_string())
    }
    fn target(&self) -> Option<Target> {
        self.target.clone()
    }
}

use decodering_core::audit::{
    ActionKind, ActionOutput, Actor, AuditDescriptor, Target, audit_allowed, audit_errored,
};
use decodering_core::error::ExecutionError;
use decodering_core::repository::AuditRepository;
use decodering_core::time::now_ts;
use decodering_core::tx::{Database, Tx};

pub async fn unlock_audit_allowed<D: Database>(
    ip: Option<String>,
    mut db: <D as Database>::Tx<'_>,
) {
    let audit_descriptor = AuditDescriptor {
        actor: Actor::unauthenticated(ip),
        action_kind: ActionKind::SystemUnlock,
        revertible: true,
        undoes: None,
        metadata: None,
    };
    let output = ActionOutput {
        response: (),
        before_state: None,
        after_state: None,
        target: Some(Target::System),
    };
    let allowed = audit_allowed(&audit_descriptor, None, &output, now_ts());
    let _ = db.audit().insert(&allowed).await;
    let _ = db.commit().await;
}

pub async fn unlock_audit_errored<D: Database>(
    ip: Option<String>,
    mut db: <D as Database>::Tx<'_>,
    err_msg: String,
) {
    let audit_descriptor = AuditDescriptor {
        actor: Actor::unauthenticated(ip),
        action_kind: ActionKind::SystemUnlock,
        revertible: false,
        undoes: None,
        metadata: None,
    };
    let err = ExecutionError::Other(err_msg);
    let allowed = audit_errored(&audit_descriptor, None, &err, now_ts());
    let _ = db.audit().insert(&allowed).await;
    let _ = db.commit().await;
}

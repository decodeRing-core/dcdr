use crate::action::Action;
use crate::audit::{audit_allowed, audit_denied, audit_errored};
use crate::error::ActionError;
use crate::repository::AuditRepository;
use crate::time::now_ts;
use crate::tx::{Database, Tx};

pub async fn run_action_direct<D, A>(db: &D, action: A) -> Result<A::Output, ActionError>
where
    D: Database,
    A: Action,
{
    let descriptor = action.audit_descriptor();

    if let Some(reason) = action.policy_check().await.map_err(ActionError::Db)? {
        let mut tx = db.begin().await.map_err(ActionError::Db)?;
        let entry = audit_denied(&descriptor, None, &reason, now_ts());
        tx.audit().insert(&entry).await.map_err(ActionError::Db)?;
        tx.commit().await.map_err(ActionError::Db)?;
        return Err(ActionError::Denied(reason));
    }

    let mut data_tx = db.begin().await.map_err(ActionError::Db)?;
    let exec_result = action.execute(&mut data_tx).await;

    match exec_result {
        Ok(output) => {
            let entry = audit_allowed(&descriptor, None, &output, now_ts());
            data_tx
                .audit()
                .insert(&entry)
                .await
                .map_err(ActionError::Db)?;
            data_tx.commit().await.map_err(ActionError::Db)?;
            Ok(output)
        }
        Err(exec_err) => {
            // Roll back data changes, write audit on a fresh tx.
            data_tx.rollback().await.map_err(ActionError::Db)?;

            let mut audit_tx = db.begin().await.map_err(ActionError::Db)?;
            let entry = audit_errored(&descriptor, None, &exec_err, now_ts());
            audit_tx
                .audit()
                .insert(&entry)
                .await
                .map_err(ActionError::Db)?;
            audit_tx.commit().await.map_err(ActionError::Db)?;

            Err(ActionError::Execution(exec_err))
        }
    }
}

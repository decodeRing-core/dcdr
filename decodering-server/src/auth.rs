use actix_web::dev::ConnectionInfo;
use actix_web::web::Data;
use decodering_core::actions::update_principal_credential_last_used::UpdatePrincipalCredentialLastUsed;
use decodering_core::repository::PrincipalAppGrant;
use decodering_core::repository::PrincipalAppGrantRepository;
use decodering_core::request::AppRequest;
use decodering_core::response::AppResponse;
use decodering_core::time::now_ts;
use decodering_core::tx::Database;
use decodering_core::tx::Tx;

use crate::app_data::AppData;
use crate::error::AppError::Action;
use crate::error::AppError::Raft;
use crate::error::ErrorReason;
use crate::extractor::AuthOSLMiddleware;
use crate::handlers::response::ErrorStatus;

pub async fn require_app_grant_for_principal<D: Database>(
    conn: &ConnectionInfo,
    db: &mut <D as Database>::Tx<'_>,
    app: &Data<AppData<D>>,
    auth: &AuthOSLMiddleware<D>,
    app_id: &str,
) -> Result<Option<PrincipalAppGrant>, ErrorStatus> {
    if let Some(p) = &auth.principal {
        match db
            .principal_app_grant()
            .get_by_app_id_and_principal_id(app_id, &p.principal_id)
            .await
        {
            Ok(Some(principal_app_grant)) => {
                let last_used = UpdatePrincipalCredentialLastUsed {
                    actor: auth.actor(conn),
                    credential_id: p.clone().credential_id.unwrap_or_default(),
                    principal_id: p.principal_id.clone(),
                    last_used_at: now_ts(),
                };
                let request = AppRequest::UpdatePrincipalCredentialLastUsed(last_used);
                match app.submit(request).await {
                    Ok(resp) => match resp {
                        AppResponse::UpdatePrincipalCredentialLastUsed(_) => {
                            return Ok(Some(principal_app_grant));
                        }
                        AppResponse::Error(e) => {
                            tracing::error!(%e, "Failed to update credential last used timestamp");
                            return Err(ErrorStatus::OperationFailed(ErrorReason::Message(
                                "Failed to update credential".into(),
                            )));
                        }
                        other_api_response => {
                            tracing::error!(?other_api_response, "Unexpected AppResponse variant");
                            return Err(ErrorStatus::OperationFailed(ErrorReason::Unexpected));
                        }
                    },
                    Err(e) => {
                        tracing::error!(?e);
                        match e {
                            Action(action_error) => {
                                return Err(ErrorStatus::OperationFailed(ErrorReason::Message(
                                    action_error.to_string().into(),
                                )));
                            }
                            Raft(raft_error) => {
                                return Err(ErrorStatus::OperationFailed(ErrorReason::Message(
                                    raft_error.to_string().into(),
                                )));
                            }
                        }
                    }
                }
            }
            Ok(None) => {
                tracing::error!(
                    "Principal doesn't have permission to access app id {}",
                    app_id
                );
                return Err(ErrorStatus::OperationFailed(ErrorReason::Unauthorized));
            }
            Err(e) => {
                tracing::error!(err=?e, "Failed to query database");
                return Err(ErrorStatus::OperationFailed(ErrorReason::Database));
            }
        };
    }
    Ok(None)
}

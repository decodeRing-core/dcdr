use actix_web::Responder;
use actix_web::web::{Data, Json};
use decodering_core::tx::Database;
use decodering_raft::raft_types::{ClientWriteError, Node, RaftError};
use decodering_raft::{ChangeMembers, NodeId};

use crate::app_data::AppData;
use crate::error::ErrorReason;
use crate::handlers::raft::payload::InitRaftRequestData;
use crate::handlers::response::{ApiResponse, ErrorStatus, SuccessStatus};

pub async fn init_raft<D: Database + 'static>(
    app: Data<AppData<D>>,
    req: Json<InitRaftRequestData>,
) -> impl Responder {
    let req = req.into_inner();
    let Some(raft_bits) = &app.raft else {
        tracing::error!("RaftBits not available");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::RaftNotAvailable));
    };

    let is_initialized = raft_bits.raft.is_initialized().await;
    if matches!(is_initialized, Ok(true)) {
        return ApiResponse::error(ErrorStatus::OperationFailed(
            ErrorReason::RaftAlreadyInitialized,
        ));
    }

    match raft_bits.init(app.addr.clone(), req.raft_init).await {
        Ok(()) => ApiResponse::empty(SuccessStatus::RaftInitialized.into()),
        Err(e) => {
            tracing::error!(err=%e, "Raft error");
            ApiResponse::<()>::error(ErrorStatus::OperationFailed(ErrorReason::Raft))
        }
    }
}

pub async fn metrics_raft<D: Database + 'static>(app: Data<AppData<D>>) -> impl Responder {
    let Some(raft_bits) = &app.raft else {
        tracing::error!("RaftBits not available");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::RaftNotAvailable));
    };

    let result = raft_bits.metrics();
    ApiResponse::new(SuccessStatus::RaftMetrics.into(), Some(result))
}

pub async fn add_learner_raft<D: Database + 'static>(
    app: Data<AppData<D>>,
    req: Json<(NodeId, String)>,
) -> impl Responder {
    let Some(raft_bits) = &app.raft else {
        tracing::error!("RaftBits not available");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::RaftNotAvailable));
    };

    match raft_bits.add_learner(req.0).await {
        Ok(resp) => ApiResponse::new(SuccessStatus::RaftAddLearner.into(), Some(resp)),
        Err(RaftError::APIError(ClientWriteError::ForwardToLeader(fwd))) => {
            tracing::warn!(err=%fwd, "forward to leader");
            ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::RaftNotLeader))
        }
        Err(e) => {
            tracing::error!(err=%e, "Raft add learner error");
            ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Raft))
        }
    }
}

pub async fn change_membership_raft<D: Database + 'static>(
    app: Data<AppData<D>>,
    req: Json<ChangeMembers<NodeId, Node>>,
) -> impl Responder {
    let Some(raft_bits) = &app.raft else {
        tracing::error!("RaftBits not available");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::RaftNotAvailable));
    };

    match raft_bits.change_membership(req.0).await {
        Ok(resp) => ApiResponse::new(SuccessStatus::RaftMembership.into(), Some(resp)),
        Err(RaftError::APIError(ClientWriteError::ForwardToLeader(fwd))) => {
            tracing::warn!(err=%fwd, "forward to leader");
            ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::RaftNotLeader))
        }
        Err(e) => {
            tracing::error!(err=%e, "Raft change membership error");
            ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Raft))
        }
    }
}

pub async fn shutdown_raft<D: Database + 'static>(app: Data<AppData<D>>) -> impl Responder {
    let Some(raft_bits) = &app.raft else {
        tracing::error!("RaftBits not available");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::RaftNotAvailable));
    };

    match raft_bits.shutdown().await {
        Ok(()) => ApiResponse::empty(SuccessStatus::RaftShutdown.into()),
        Err(e) => {
            tracing::error!(err=%e, "Raft error");
            ApiResponse::<()>::error(ErrorStatus::OperationFailed(ErrorReason::Raft))
        }
    }
}

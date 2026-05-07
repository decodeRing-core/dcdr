use actix_web::Responder;
use actix_web::web::{Data, Json};
use decodering_core::tx::Database;
use decodering_raft::NodeId;
use std::collections::BTreeSet;

use crate::app_data::AppData;
use crate::handlers::raft::payload::InitRaftRequestData;
use crate::handlers::response::{ApiResponse, ErrorStatus, SuccessStatus};

pub(crate) async fn init_raft<D: Database + 'static>(
    app: Data<AppData<D>>,
    req: Json<InitRaftRequestData>,
) -> impl Responder {
    let req = req.into_inner();
    match &app.raft {
        Some(raft_bits) => {
            let is_initialized = raft_bits.raft.is_initialized().await;
            if matches!(is_initialized, Ok(true)) {
                return ApiResponse::error(ErrorStatus::AlreadyInitialized.into());
            }
            let result = raft_bits.init(app.addr.clone(), req.raft_init).await;
            let Ok(_) = result else {
                let err = result.unwrap_err();
                tracing::error!(e=%err, "Raft error");
                return ApiResponse::<()>::error(ErrorStatus::Internal.into());
            };
            return ApiResponse::empty(SuccessStatus::RaftInitialized.into());
        }
        _ => {
            tracing::error!("RaftBits not available");
            return ApiResponse::error(ErrorStatus::Internal.into());
        }
    }
}

pub(crate) async fn metrics_raft<D: Database + 'static>(
    app: Data<AppData<D>>,
    _req: Json<Vec<(NodeId, String)>>,
) -> impl Responder {
    match &app.raft {
        Some(raft_bits) => {
            let result = raft_bits.metrics().await;
            return ApiResponse::new(SuccessStatus::RaftMetrics.into(), Some(result));
        }
        _ => {
            tracing::error!("RaftBits not available");
            return ApiResponse::error(ErrorStatus::Internal.into());
        }
    }
}

pub(crate) async fn add_learner_raft<D: Database + 'static>(
    app: Data<AppData<D>>,
    req: Json<(NodeId, String)>,
) -> impl Responder {
    match &app.raft {
        Some(raft_bits) => {
            let result = raft_bits.add_learner(req.0).await;
            return ApiResponse::new(SuccessStatus::RaftAddLearner.into(), Some(result));
        }
        _ => {
            tracing::error!("RaftBits not available");
            return ApiResponse::error(ErrorStatus::Internal.into());
        }
    }
}

pub(crate) async fn change_membership_raft<D: Database + 'static>(
    app: Data<AppData<D>>,
    req: Json<BTreeSet<NodeId>>,
) -> impl Responder {
    match &app.raft {
        Some(raft_bits) => {
            let result = raft_bits.change_membership(req.0).await;
            return ApiResponse::new(SuccessStatus::RaftMembership.into(), Some(result));
        }
        _ => {
            tracing::error!("RaftBits not available");
            return ApiResponse::error(ErrorStatus::Internal.into());
        }
    }
}

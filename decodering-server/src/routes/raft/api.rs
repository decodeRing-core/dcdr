use actix_web::web;
use decodering_core::tx::Database;

use crate::handlers::raft::api::append_raft;
use crate::handlers::raft::api::snapshot_raft;
use crate::handlers::raft::api::vote_raft;
use crate::middleware::RaftBackendOnly;

pub fn raft_api_routes<D: Database + 'static>(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/vote")
            .wrap(RaftBackendOnly::<D>::new())
            .route(web::post().to(vote_raft::<D>)),
    )
    .service(
        web::resource("/append")
            .wrap(RaftBackendOnly::<D>::new())
            .route(web::post().to(append_raft::<D>)),
    )
    .service(
        web::resource("/snapshot")
            .wrap(RaftBackendOnly::<D>::new())
            .route(web::post().to(snapshot_raft::<D>)),
    );
}

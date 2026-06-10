use actix_web::middleware::from_fn;
use actix_web::web;
use decodering_core::tx::Database;

use crate::handlers::raft::api::append_raft;
use crate::handlers::raft::api::snapshot_raft;
use crate::handlers::raft::api::vote_raft;
use crate::middleware::require_raft_backend;

pub fn raft_api_routes<D: Database + 'static>(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/vote")
            .wrap(from_fn(require_raft_backend))
            .route(web::post().to(vote_raft::<D>)),
    )
    .service(
        web::resource("/append")
            .wrap(from_fn(require_raft_backend))
            .route(web::post().to(append_raft::<D>)),
    )
    .service(
        web::resource("/snapshot")
            .wrap(from_fn(require_raft_backend))
            .route(web::post().to(snapshot_raft::<D>)),
    );
}

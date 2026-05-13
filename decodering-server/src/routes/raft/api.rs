use actix_web::dev::HttpServiceFactory;
use actix_web::web;
use decodering_core::tx::Database;

use crate::handlers::raft::api::append_raft;
use crate::handlers::raft::api::snapshot_raft;
use crate::handlers::raft::api::vote_raft;
use crate::middleware::RaftInitializedHelper;

pub fn raft_api_routes<D: Database + 'static>() -> impl HttpServiceFactory {
    web::scope(r"")
        .wrap(RaftInitializedHelper::<D>::new())
        .route("/vote", web::post().to(vote_raft::<D>))
        .route("/append", web::post().to(append_raft::<D>))
        .route("/snapshot", web::post().to(snapshot_raft::<D>))
}

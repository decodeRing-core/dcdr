use actix_web::Scope;
use actix_web::web;
use decodering_core::tx::Database;

use crate::handlers::raft::api::append_raft;
use crate::handlers::raft::api::snapshot_raft;
use crate::handlers::raft::api::vote_raft;

pub fn raft_api_routes<D: Database + 'static>() -> Scope {
    web::scope(r"")
        .service(web::resource("/vote").route(web::post().to(vote_raft::<D>)))
        .service(web::resource("/append").route(web::post().to(append_raft::<D>)))
        .service(web::resource("/snapshot").route(web::post().to(snapshot_raft::<D>)))
}

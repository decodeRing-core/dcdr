use actix_web::dev::HttpServiceFactory;
use actix_web::middleware::from_fn;
use actix_web::web;
use decodering_core::tx::Database;

use crate::handlers::raft::management::add_learner_raft;
use crate::handlers::raft::management::change_membership_raft;
use crate::handlers::raft::management::init_raft;
use crate::handlers::raft::management::metrics_raft;
use crate::handlers::raft::management::shutdown_raft;
use crate::middleware::require_raft_backend;
use crate::middleware::require_raft_initialized;
use crate::middleware::require_raft_leader;

pub fn raft_management_routes<D: Database + 'static>() -> impl HttpServiceFactory {
    web::scope("/raft")
        .wrap(from_fn(require_raft_backend))
        .route("/init", web::post().to(init_raft::<D>))
        .route("/metrics", web::post().to(metrics_raft::<D>))
        .route(
            "/shutdown",
            web::post()
                .to(shutdown_raft::<D>)
                .wrap(from_fn(require_raft_initialized::<D, _>)),
        )
        .service(
            web::scope("")
                .wrap(from_fn(require_raft_leader::<D, _>))
                .wrap(from_fn(require_raft_initialized::<D, _>))
                .route("/add-learner", web::post().to(add_learner_raft::<D>))
                .route(
                    "/change-membership",
                    web::post().to(change_membership_raft::<D>),
                ),
        )
}

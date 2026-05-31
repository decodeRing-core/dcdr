use actix_web::dev::HttpServiceFactory;
use actix_web::web;
use decodering_core::tx::Database;

use crate::handlers::raft::management::add_learner_raft;
use crate::handlers::raft::management::change_membership_raft;
use crate::handlers::raft::management::init_raft;
use crate::handlers::raft::management::metrics_raft;
use crate::handlers::raft::management::shutdown_raft;
use crate::middleware::RaftInitializedHelper;
use crate::middleware::RaftLeaderHelper;

pub fn raft_management_routes<D: Database + 'static>() -> impl HttpServiceFactory {
    web::scope("/raft")
        .route("/init", web::post().to(init_raft::<D>))
        .route("/metrics", web::post().to(metrics_raft::<D>))
        .route("/shutdown", web::post().to(shutdown_raft::<D>))
        .service(
            web::scope("")
                .wrap(RaftLeaderHelper::<D>::new())
                .wrap(RaftInitializedHelper::<D>::new())
                .route("/add-learner", web::post().to(add_learner_raft::<D>))
                .route(
                    "/change-membership",
                    web::post().to(change_membership_raft::<D>),
                ),
        )
}

use actix_web::Scope;
use actix_web::web;
use decodering_db::Database;

use crate::handlers::raft::management::add_learner_raft;
use crate::handlers::raft::management::change_membership_raft;
use crate::handlers::raft::management::init_raft;
use crate::handlers::raft::management::metrics_raft;

pub fn raft_management_routes<D: Database + 'static>() -> Scope {
    web::scope(r"/raft")
        .service(web::resource("/init").route(web::post().to(init_raft::<D>)))
        .service(web::resource("/metrics").route(web::post().to(metrics_raft::<D>)))
        .service(web::resource("/add-learner").route(web::post().to(add_learner_raft::<D>)))
        .service(
            web::resource("/change-membership").route(web::post().to(change_membership_raft::<D>)),
        )
}

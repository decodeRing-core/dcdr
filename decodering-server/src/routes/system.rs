use actix_web::web;
use decodering_core::tx::Database;

use crate::handlers::system::management::system_init;
use crate::handlers::system::management::system_plugin_config;
use crate::handlers::system::management::system_status;
use crate::handlers::system::management::system_unlock;
use crate::middleware::LockState;
use crate::middleware::RaftInitializedHelper;
use crate::middleware::RaftLeaderHelper;

pub fn app_system_routes<D: Database + 'static>(cfg: &mut web::ServiceConfig) {
    cfg.route(
        "/init",
        web::post()
            .to(system_init::<D>)
            .wrap(RaftLeaderHelper::<D>::new())
            .wrap(RaftInitializedHelper::<D>::new()),
    )
    .route(
        "/plugin/config",
        web::post()
            .to(system_plugin_config::<D>)
            .wrap(RaftLeaderHelper::<D>::new())
            .wrap(RaftInitializedHelper::<D>::new())
            .wrap(LockState::<D>::new()),
    )
    .route("/unlock", web::post().to(system_unlock::<D>))
    .route("/status", web::get().to(system_status::<D>));
}

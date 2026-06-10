use actix_web::middleware::from_fn;
use actix_web::web;
use decodering_core::tx::Database;

use crate::handlers::system::management::system_init;
use crate::handlers::system::management::system_plugin_config;
use crate::handlers::system::management::system_status;
use crate::handlers::system::management::system_unlock;
use crate::middleware::require_raft_initialized;
use crate::middleware::require_raft_leader;
use crate::middleware::require_unlocked;

pub fn app_system_routes<D: Database + 'static>(cfg: &mut web::ServiceConfig) {
    cfg.route(
        "/init",
        web::post()
            .to(system_init::<D>)
            .wrap(from_fn(require_raft_leader::<D, _>))
            .wrap(from_fn(require_raft_initialized::<D, _>)),
    )
    .route(
        "/plugin/config",
        web::post()
            .to(system_plugin_config::<D>)
            .wrap(from_fn(require_raft_leader::<D, _>))
            .wrap(from_fn(require_raft_initialized::<D, _>))
            .wrap(from_fn(require_unlocked::<D, _>)),
    )
    .route("/unlock", web::post().to(system_unlock::<D>))
    .route("/status", web::get().to(system_status::<D>));
}

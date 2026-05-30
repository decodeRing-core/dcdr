use actix_web::web;
use decodering_core::tx::Database;

use crate::middleware::{LockState, RaftBackendOnly, RaftInitializedHelper};
use crate::routes::app::management::app_management_routes;
use crate::routes::osl::api::{read_osl_routes, write_osl_routes};
use crate::routes::raft::api::raft_api_routes;
use crate::routes::raft::management::raft_management_routes;
use crate::routes::system::app_system_routes;

pub fn config_app<D: Database + 'static>(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/osl/v1")
            .wrap(LockState::<D>::new())
            .wrap(RaftInitializedHelper::<D>::new())
            .configure(read_osl_routes::<D>)
            .configure(write_osl_routes::<D>),
    )
    .service(app_management_routes::<D>())
    .service(web::scope("/system").configure(app_system_routes::<D>))
    .service(
        web::scope("")
            .wrap(RaftBackendOnly::<D>::new())
            .service(raft_management_routes::<D>())
            .service(raft_api_routes::<D>()),
    );
}

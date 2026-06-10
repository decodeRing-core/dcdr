use actix_web::middleware::from_fn;
use actix_web::web::{self, ServiceConfig};
use decodering_core::tx::Database;

use crate::middleware::{require_raft_initialized, require_unlocked};
use crate::routes::RouteExtensions;
use crate::routes::app::management::app_management_routes;
use crate::routes::doc::doc_routes;
use crate::routes::osl::api::{read_osl_routes, write_osl_routes};
use crate::routes::raft::api::raft_api_routes;
use crate::routes::raft::management::raft_management_routes;
use crate::routes::system::app_system_routes;

pub fn config_app<D: Database + Clone + 'static>(
    exts: RouteExtensions,
) -> impl FnOnce(&mut ServiceConfig) {
    move |cfg| {
        cfg.service(
            web::scope("/osl/v1")
                .wrap(from_fn(require_unlocked::<D, _>))
                .wrap(from_fn(require_raft_initialized::<D, _>))
                .configure(read_osl_routes::<D>)
                .configure(write_osl_routes::<D>),
        )
        .service(app_management_routes::<D>())
        .service(
            web::scope("/system")
                .configure(app_system_routes::<D>)
                .configure({
                    let exts = exts.clone();
                    move |c| exts.apply_scope("/system", c)
                }),
        )
        .configure(doc_routes)
        .service(raft_management_routes::<D>())
        .configure(raft_api_routes::<D>);

        cfg.configure(move |c| exts.apply_root(c));
    }
}
